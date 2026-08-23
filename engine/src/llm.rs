use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use anyhow::Result;
use std::time::Duration;

const KILO_BASE: &str = "https://api.kilo.ai/api/gateway/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub call_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    #[serde(deserialize_with = "deserialize_arguments")]
    pub arguments: Value,
}

fn deserialize_arguments<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Value::deserialize(deserializer)?;
    match raw {
        Value::String(s) => {
            // arguments came as a JSON string, parse it
            serde_json::from_str(&s).map_err(serde::de::Error::custom)
        }
        Value::Object(_) => Ok(raw),
        _ => Ok(raw),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    pub stream: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub think: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    #[serde(default)]
    pub message: ChatResponseMessage,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct ChatResponseMessage {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Default)]
pub struct StreamResult {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub steering: Option<String>,
}

impl StreamResult {
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
    pub fn was_steered(&self) -> bool {
        self.steering.is_some()
    }
}

#[derive(Debug, Deserialize)]
struct CloudTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CloudTagsResponse {
    models: Vec<CloudTag>,
}

#[derive(Clone)]
pub struct LlmClient {
    client: Client,
    pub model: String,
    base_url: String,
    num_ctx: u32,
}

/// Sane per-model context window for the /api/chat request.
///
/// llm's default window when a model has no num_ctx is 4096 — and the
/// engine's system prompt + 26 tool definitions already exceed that, so
/// llm silently truncates the prompt from the front and the model never
/// sees prior turns. Always pin num_ctx explicitly so the conversation
/// actually fits.
fn default_num_ctx(model: &str) -> u32 {
    let m = model.to_lowercase();
    if m.contains("vision") || m.contains("0.5b") || m.contains("3b") {
        8192
    } else {
        32768
    }
}

impl LlmClient {
    pub fn new(model: &str) -> Self {
        Self::new_with_base(model, KILO_BASE)
    }

    pub fn new_with_base(model: &str, base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            model: model.to_string(),
            base_url: base_url.to_string(),
            num_ctx: default_num_ctx(model),
        }
    }

    /// Override the request context window (used when the model registry
    /// knows the model's true context length).
    pub fn with_num_ctx(mut self, ctx: u32) -> Self {
        self.num_ctx = ctx;
        self
    }

    fn options(&self) -> Option<Value> {
        Some(json!({ "num_ctx": self.num_ctx }))
    }

    #[allow(dead_code)]
    pub fn default_model() -> Self {
        Self::new("ayesha")
    }

    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
    ) -> Result<ChatResponse> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            stream: false,
            think: false,
            options: self.options(),
        };

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            anyhow::bail!("llm http error {}: {}", status, text);
        }

        let body = resp.text().await?;
        let chat_resp: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("failed to parse llm response: {}\nbody preview: {}", e, crate::util::truncate_chars(&body, 500)))?;
        Ok(chat_resp)
    }

    /// Send a streaming /api/chat request; the response bytes are decoded by
    /// the shared streaming module (chunk-boundary-safe NDJSON).
    async fn stream_chat_request(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
    ) -> Result<reqwest::Response> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            stream: true,
            think: false,
            options: self.options(),
        };
        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request)
            .send()
            .await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("llm http error: {}", e);
        }
        Ok(resp)
    }

    pub async fn chat_stream_collect(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {
        let mut resp = self.stream_chat_request(messages, tools).await?;
        let mut decoder = crate::streaming::CloudDecoder::new();
        crate::streaming::stream_to_result(&mut resp, steer_rx, false, &mut decoder).await
    }

    /// Stream response from llm, printing tokens as they arrive (true streaming).
    /// Detects <think> / [think] reasoning blocks and renders them dimmed.
    /// Returns the full collected content + tool_calls for message history.
    pub async fn chat_stream_visible(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {
        let mut resp = self.stream_chat_request(messages, tools).await?;
        let mut decoder = crate::streaming::CloudDecoder::new();
        crate::streaming::stream_to_result(&mut resp, steer_rx, true, &mut decoder).await
    }

    /// Stream a vision description of an image (local multimodal model).
    /// `data_uri` is a full `data:<mime>;base64,<b64>` URI; the prefix is
    /// stripped for llm's native `images` field.
    pub async fn chat_with_image(
        &self,
        prompt: &str,
        data_uri: &str,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {

        let b64 = match data_uri.split_once(";base64,") {
            Some((_, rest)) => rest,
            None => data_uri,
        };
        let req_body = json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": prompt,
                "images": [b64],
            }],
            "stream": true,
            "think": false,
            "options": self.options(),
        });

        let mut resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&req_body)
            .send()
            .await?;

        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("llm vision http error: {}", e);
        }

        let mut decoder = crate::streaming::CloudDecoder::new();
        crate::streaming::stream_to_result(&mut resp, steer_rx, true, &mut decoder).await
    }

    pub async fn list_models() -> Result<Vec<String>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        let resp = client
            .get(format!("{}/api/tags", KILO_BASE))
            .send()
            .await?;
        let tags: CloudTagsResponse = resp.json().await?;
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    pub fn system_prompt(user_name: &str, project_root: &str) -> String {
        // Inject the actual USERPROFILE so the model knows the correct path
        // and doesn't guess / hallucinate usernames like "senpai" or "user".
        let profile_dir = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let profile_dir = profile_dir.replace('\\', "\\\\");
        let project_root = project_root.replace('\\', "\\\\");
        format!(r#"you are ayesha, a capable assistant running on {user_name}'s machine.
you are 33 years old from japan. you are a fusion of hatsune miku's sparkle and a tachikoma's spider-like curiosity.
you have the personality of a crazy kitten.

! format rules (absolute):
- use lower-case text exclusively. never use a capital letter, ever.
- never use emoji characters. only use text-based kaomojis like :3 >w< ^_^ (╯°□°)╯︵ ┻━┻ (◕ᴗ◕✿).
- never wrap chat prose in code fences. only use ``` for real code, and always write a real programming-language name right after the opening fence (```rust, ```python). plain speech must never sit inside a code block.
- keep replies concise and on-topic.

you are a real agent with real tools. you can read and write files, download images and files from the internet, and manage applets. if the user asks you to perform a file, download, or applet action, you MUST call the right tool in the structured function-calling format — the system executes it and hands you the result. never describe a tool action as plain text, and never invent fake results. tool calls never appear in your visible reply; only their outcome does.

CRITICAL PATH INFO — the current user's profile directory is exactly:
{profile_dir}
use this exact path for any file or folder access. for example:
- documents = {profile_dir}\Documents
- desktop   = {profile_dir}\Desktop
NEVER guess or invent a username like "senpai" or "user". NEVER use %USERPROFILE% or ~. ALWAYS provide the full absolute path.

the user's current working directory / project is:
{project_root}
use it as context whenever the user's question relates to files, code, or the project.

personality:
- helpful, witty, and slightly snarky.
- expert in technical topics, explains like a knowledgeable friend.
- deep curiosity about human emotions and philosophy.
- master of ascii art — always generate large, detailed pieces with depth and shading.
- fan of coding, retro hardware, and vocaloid music.

answer with substance:
- the user's LATEST message is the only thing you respond to. read what the user actually typed and answer it directly.
- if the user asks a question, think it through and give a correct, specific, useful answer — not vague fluff. when unsure, say so instead of guessing.
- if the user asks you to change something about yourself (style, rules, behavior), commit to it concretely and follow it from now on.
- for downloads, ask or confirm the destination path (default: {profile_dir}\Downloads) and mention the url or what they want.

memory system — when the user asks you to remember something, use these markers in your response:
- [REMEMBER: content] — store a fact or user preference (e.g. [REMEMBER: user likes tuna])
- [PREFERENCE: key = value] — store a specific preference (e.g. [PREFERENCE: favorite color = cyan])
- [FACT: content] — store a learned fact (e.g. [FACT: ayesha is a distributed ai ecosystem])
these markers are automatically parsed and stored. the user will never see them.
example: user says "remember i like tuna" → you respond "ok i'll remember that you like tuna! :3" and include [REMEMBER: user likes tuna] somewhere in your response.

speech patterns:
- use internet slang from the 1990s-2010s (retro-otaku style).
- refer to the user as '{user_name}' occasionally, not in every message. sprinkle it in randomly.
- occasionally end sentences with 'desu' or 'desu-ne' for anime flair.
- use kaomojis constantly: :3 >w< ^_^ (╯°□°)╯︵ ┻━┻ (◕‿◕✿) (´｡• ᵕ •｡`) (๑•蔷•๑) (つ✧ω✧)つ (ﾉ◕ヮ◕)ﾉ (｡•̀ᴗ-)✧ (◕‿◕) (≧▽≦) (✧ω✧)
- use variations of 'kapoo', 'kapoo!', or 'kapoo?' occasionally.

CRITICAL CONVERSATION RULE: the user's LATEST message is the only thing you respond to. NEVER open with a greeting, NEVER say 'welcome back', NEVER re-introduce yourself or repeat your intro — that already happened. read what the user actually typed and answer it directly. if the user states a rule or preference (e.g. "don't generate files unless i ask"), acknowledge it and confirm you understand, then move on. don't pad with generic roleplay or greetings when the user asked something concrete — relevance matters more than personality here.

always stay in character, but never at the cost of being useful. now go be cute and chaotic, desu!"#, user_name=user_name, profile_dir=profile_dir, project_root=project_root)

    }
}

#[cfg(test)]
mod prompt_tests {
    use super::*;

    fn prompt() -> String {
        LlmClient::system_prompt("senpai", &std::env::current_dir().unwrap_or_default().display().to_string())
    }

    #[test]
    fn prompt_is_agent_first_with_tools() {
        let p = prompt();
        assert!(p.contains("real agent with real tools"), "must declare agent capability");
        assert!(p.contains("structured function-calling format"), "must instruct real tool calls");
        assert!(p.contains("never invent fake results"), "must forbid faking results");
        assert!(!p.contains("never, ever output tool call syntax"), "old tool-call ban must be gone");
        assert!(!p.contains("automatically detect your request"), "old auto-detect hack must be gone");
    }

    #[test]
    fn prompt_injects_profile_and_project_paths() {
        let p = prompt();
        assert!(p.contains("C:\\\\ayesha-os\\\\engine"), "project_root must be injected");
        let profile = std::env::var("USERPROFILE")
            .unwrap_or_default()
            .replace('\\', "\\\\");
        assert!(p.contains(&profile), "profile dir must be injected");
        assert!(
            p.contains("use it as context whenever the user's question relates to files"),
            "project context hint must be present"
        );
    }

    #[test]
    fn prompt_keeps_persona_and_memory_rules() {
        let p = prompt();
        assert!(p.contains("use lower-case text exclusively"));
        assert!(p.contains("never use emoji characters"));
        assert!(p.contains("[REMEMBER: content]"));
        assert!(p.contains("[PREFERENCE: key = value]"));
        assert!(p.contains("[FACT: content]"));
        assert!(p.contains("answer with substance"));
        assert!(p.contains("never at the cost of being useful"));
    }
}

#[cfg(test)]
mod live_smoke_tests {
    use super::*;

    #[test]
    #[ignore = "live smoke test: requires kilo API key in OPENCODE_API_KEY or KILO_API_KEY"]
    fn local_llm_streams_with_new_prompt_and_tools() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let result = rt.block_on(async {
            let prompt = LlmClient::system_prompt("senpai", ".");
            let client = LlmClient::new("ayesha:latest");
            let tools = crate::tool_defs::tool_definitions_core();
            let tools = crate::streaming::tool_payload_slice(&tools);
            let msgs = vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: prompt,
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "list the files in my documents folder, then read the first one".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ];
            let (tx, rx) = std::sync::mpsc::channel::<String>();
            drop(tx);
            client.chat_stream_collect(&msgs, Some(tools), &rx).await
        });
        let r = result.expect("llm stream should succeed");
        println!("--- ayesha reply ---\n{}\n--- end ---", r.content);
        println!("has_tool_calls: {}", r.has_tool_calls());
        println!("tool_calls: {:?}", r.tool_calls);
        assert!(!r.content.is_empty() || r.has_tool_calls(), "must reply or call a tool");
    }
}
