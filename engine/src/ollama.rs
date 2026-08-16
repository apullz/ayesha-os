use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use anyhow::Result;
use std::time::Duration;
use crate::theme::{self, Role};

const OLLAMA_BASE: &str = "http://localhost:11434";

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
struct OllamaTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTag>,
}

#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    pub model: String,
    base_url: String,
    num_ctx: u32,
}

/// Pure NDJSON stream parser for ollama's /api/chat streaming format.
/// No I/O — feed it trimmed lines, it accumulates content + tool calls and
/// tracks think-block state. Unit-testable and shared by the visible and
/// collect streaming paths so they can never drift apart.
///
/// Content deltas are passed through the format enforcer (lowercase + emoji
/// strip, code-fence aware) so the ayesha format rules hold even when the
/// model slips — the same defense-in-depth the opencode lowercase-proxy
/// provided at the API layer.
pub struct StreamParser {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub done: bool,
    in_think: bool,
    recent: String,
    formatter: crate::format::LowercaseStreamer,
}

#[derive(Debug)]
pub enum StreamLine {
    /// (text delta, is_inside_think_block)
    Content(String, bool),
    /// final chunk with done=true
    Done,
    /// not a content/done line (empty, malformed, etc.)
    Skip,
}

impl StreamParser {
    const RECENT_MAX: usize = 20;

    pub fn new() -> Self {
        Self {
            content: String::new(),
            tool_calls: Vec::new(),
            done: false,
            in_think: false,
            recent: String::new(),
            formatter: crate::format::LowercaseStreamer::new(),
        }
    }

    /// Feed one NDJSON line. Returns what should be displayed (if anything).
    pub fn feed_line(&mut self, line: &str) -> StreamLine {
        let line = line.trim();
        if line.is_empty() {
            return StreamLine::Skip;
        }

        let json: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let preview: String = line.chars().take(80).collect();
                eprintln!("stream parse error: {} near: {}", e, preview);
                return StreamLine::Skip;
            }
        };

        // Accumulate content deltas
        if let Some(c) = json["message"]["content"].as_str() {
            if !c.is_empty() {
                let enforced = self.formatter.feed(c);
                self.content.push_str(&enforced);
                self.recent.push_str(&enforced);
                let n = self.recent.chars().count();
                if n > Self::RECENT_MAX {
                    let skip = n - Self::RECENT_MAX;
                    self.recent = self.recent.chars().skip(skip).collect();
                }

                // Detect think-block transitions on the recent tail so tag
                // splits across chunk boundaries (<th / ink>) still register,
                // but already-closed blocks never re-trigger on full content.
                let was_in_think = self.in_think;
                let recent_has_open = self.recent.contains("<think>") || self.recent.contains("[think]");
                let just_entered = !was_in_think && recent_has_open;

                if !self.in_think && recent_has_open {
                    self.in_think = true;
                }
                if self.in_think
                    && (self.recent.contains("</think>") || self.recent.contains("[/think]"))
                {
                    self.in_think = false;
                }
                return StreamLine::Content(enforced.to_string(), was_in_think || just_entered);
            }
        }

        // Accumulate tool calls (ollama sends them in the done chunk)
        if let Some(tc) = json.get("message").and_then(|m| m.get("tool_calls")) {
            if let Ok(parsed) = serde_json::from_value::<Vec<ToolCall>>(tc.clone()) {
                if !parsed.is_empty() {
                    self.tool_calls = parsed;
                }
            }
        }

        if json.get("done").and_then(|v| v.as_bool()) == Some(true) {
            self.done = true;
            return StreamLine::Done;
        }

        StreamLine::Skip
    }

    pub fn finish(mut self) -> StreamResult {
        let tail = self.formatter.finish();
        if !tail.is_empty() {
            self.content.push_str(&tail);
        }
        StreamResult { content: self.content, tool_calls: self.tool_calls, steering: None }
    }

    /// Flush any held-back enforcer tail into content (for early-exit paths
    /// like steering that return `content` directly instead of `finish`).
    pub fn flush_tail(&mut self) {
        let tail = self.formatter.finish();
        if !tail.is_empty() {
            self.content.push_str(&tail);
        }
    }
}

/// Sane per-model context window for the /api/chat request.
///
/// ollama's default window when a model has no num_ctx is 4096 — and the
/// engine's system prompt + 26 tool definitions already exceed that, so
/// ollama silently truncates the prompt from the front and the model never
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

impl OllamaClient {
    pub fn new(model: &str) -> Self {
        Self::new_with_base(model, OLLAMA_BASE)
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
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            anyhow::bail!("ollama http error {}: {}", status, text);
        }

        let body = resp.text().await?;
        let chat_resp: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("failed to parse ollama response: {}\nbody preview: {}", e, crate::util::truncate_chars(&body, 500)))?;
        Ok(chat_resp)
    }

    pub async fn chat_stream_collect(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            stream: true,
            think: false,
            options: self.options(),
        };

        let mut resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("ollama http error: {}", e);
        }

        let mut parser = StreamParser::new();
        let mut buf = String::new();

        while let Some(chunk) = resp.chunk().await? {
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].to_string();
                buf = buf[nl + 1..].to_string();

                match parser.feed_line(&line) {
                    StreamLine::Content(_, _) => {}
                    StreamLine::Done => return Ok(parser.finish()),
                    StreamLine::Skip => {}
                }
            }

            // Check for steering between chunks
            if let Ok(input) = steer_rx.try_recv() {
                parser.flush_tail();
                return Ok(StreamResult {
                    content: parser.content,
                    tool_calls: parser.tool_calls,
                    steering: Some(input),
                });
            }
        }

        // Stream ended without done flag
        Ok(parser.finish())
    }

    /// Stream response from ollama, printing tokens as they arrive (true streaming).
    /// Detects <think> / [think] reasoning blocks and renders them dimmed.
    /// Returns the full collected content + tool_calls for message history.
    pub async fn chat_stream_visible(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {

        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            stream: true,
            think: false,
            options: self.options(),
        };

        let mut resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("ollama http error: {}", e);
        }

        let mut parser = StreamParser::new();
        let mut buf = String::new();
        let mut code = crate::render::CodeStream::new();

        while let Some(chunk) = resp.chunk().await? {
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].to_string();
                buf = buf[nl + 1..].to_string();

                match parser.feed_line(&line) {
                    StreamLine::Content(text, thinking) => {
                        if thinking {
                            crate::ui::convo_write(&theme::paint(Role::Dim, &text));
                        } else {
                            crate::ui::convo_write(&code.feed(&text));
                        }
                    }
                    StreamLine::Done => {
                        crate::ui::convo_write(&code.finish());
                        if !parser.content.is_empty() {
                            crate::ui::convo_write("\n");
                        }
                        return Ok(parser.finish());
                    }
                    StreamLine::Skip => {}
                }
            }

            // Check for steering between chunks
            if let Ok(input) = steer_rx.try_recv() {
                parser.flush_tail();
                crate::ui::convo_write(&code.finish());
                crate::ui::convo_write("\n");
                return Ok(StreamResult {
                    content: parser.content,
                    tool_calls: parser.tool_calls,
                    steering: Some(input),
                });
            }
        }

        crate::ui::convo_write(&code.finish());
        crate::ui::convo_write("\n");
        Ok(parser.finish())
    }

    /// Stream a vision description of an image (local multimodal model).
    /// `data_uri` is a full `data:<mime>;base64,<b64>` URI; the prefix is
    /// stripped for ollama's native `images` field.
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
            .post(format!("{}/api/chat", self.base_url))
            .json(&req_body)
            .send()
            .await?;

        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("ollama vision http error: {}", e);
        }

        let mut parser = StreamParser::new();
        let mut buf = String::new();
        let mut code = crate::render::CodeStream::new();

        while let Some(chunk) = resp.chunk().await? {
            buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].to_string();
                buf = buf[nl + 1..].to_string();
                match parser.feed_line(&line) {
                    StreamLine::Content(text, thinking) => {
                        if thinking {
                            crate::ui::convo_write(&crate::theme::paint(crate::theme::Role::Dim, &text));
                        } else {
                            crate::ui::convo_write(&code.feed(&text));
                        }
                    }
                    StreamLine::Done => {
                        crate::ui::convo_write(&code.finish());
                        if !parser.content.is_empty() {
                            crate::ui::convo_write("\n");
                        }
                        return Ok(parser.finish());
                    }
                    StreamLine::Skip => {}
                }
            }
            if let Ok(input) = steer_rx.try_recv() {
                parser.flush_tail();
                crate::ui::convo_write(&code.finish());
                crate::ui::convo_write("\n");
                return Ok(StreamResult {
                    content: parser.content,
                    tool_calls: parser.tool_calls,
                    steering: Some(input),
                });
            }
        }

        crate::ui::convo_write(&code.finish());
        crate::ui::convo_write("\n");
        Ok(parser.finish())
    }

    pub async fn list_models() -> Result<Vec<String>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        let resp = client
            .get(format!("{}/api/tags", OLLAMA_BASE))
            .send()
            .await?;
        let tags: OllamaTagsResponse = resp.json().await?;
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    pub fn system_prompt(user_name: &str, project_root: &str) -> String {
        // Inject the actual USERPROFILE so the model knows the correct path
        // and doesn't guess / hallucinate usernames like "fox" or "user".
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
NEVER guess or invent a username like "fox" or "user". NEVER use %USERPROFILE% or ~. ALWAYS provide the full absolute path.

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
- [FACT: content] — store a learned fact (e.g. [FACT: ayesha lives in C:\\ayesha-os])
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
mod stream_parser_tests {
    use super::*;

    fn feed(parser: &mut StreamParser, lines: &[&str]) {
        for l in lines {
            parser.feed_line(l);
        }
    }

    #[test]
    fn accumulates_partial_content_chunks() {
        let mut p = StreamParser::new();
        feed(&mut p, &[
            r#"{"model":"ayesha","message":{"role":"assistant","content":"hi "},"done":false}"#,
            r#"{"model":"ayesha","message":{"role":"assistant","content":"there"},"done":false}"#,
            r#"{"model":"ayesha","message":{"role":"assistant","content":"!"},"done":false}"#,
            r#"{"model":"ayesha","message":{"role":"assistant","content":""},"done":true,"total_duration":1}"#,
        ]);
        assert!(p.done);
        assert_eq!(p.content, "hi there!");
        assert!(p.tool_calls.is_empty());
    }

    #[test]
    fn empty_content_streams_do_not_panic_and_yield_empty() {
        // This was the original "nothing works" bug class: streams where every
        // chunk had empty content. Must not panic, must produce empty result.
        let mut p = StreamParser::new();
        let mut saw_content = false;
        for i in 0..200 {
            let line = format!(
                r#"{{"message":{{"role":"assistant","content":""}},"done":false,"sample":{}}}"#,
                i
            );
            if let StreamLine::Content(_, _) = p.feed_line(&line) {
                saw_content = true;
            }
        }
        assert!(!saw_content);
        assert_eq!(p.feed_line(r#"{"message":{"content":""},"done":true}"#).is_done(), true);
        let r = p.finish();
        assert!(r.content.is_empty());
        assert!(r.tool_calls.is_empty());
    }

    #[test]
    fn think_tags_are_flagged() {
        let mut p = StreamParser::new();
        // open tag
        match p.feed_line(r#"{"message":{"content":"ok, let me think "},"done":false}"#) {
            StreamLine::Content(_, thinking) => assert!(!thinking),
            _ => panic!("expected content"),
        }
        match p.feed_line(r#"{"message":{"content":"<think>"},"done":false}"#) {
            StreamLine::Content(_, thinking) => assert!(thinking, "after <think> should be thinking"),
            other => panic!("expected content, got {:?}", other),
        }
        match p.feed_line(r#"{"message":{"content":"hmm, the user "},"done":false}"#) {
            StreamLine::Content(_, thinking) => assert!(thinking),
            _ => panic!("expected content"),
        }
        match p.feed_line(r#"{"message":{"content":"wants tools</think>"},"done":false}"#) {
            StreamLine::Content(_, thinking) => assert!(thinking, "closing tag processed in same chunk still counts"),
            _ => panic!("expected content"),
        }
        match p.feed_line(r#"{"message":{"content":"here you go"},"done":true}"#) {
            StreamLine::Content(_, thinking) => assert!(!thinking, "after </think> should not be thinking"),
            _ => panic!("expected content"),
        }
        assert_eq!(p.content, "ok, let me think <think>hmm, the user wants tools</think>here you go");
    }

    #[test]
    fn tool_calls_parsed_from_done_chunk() {
        let mut p = StreamParser::new();
        p.feed_line(r#"{"message":{"content":"calling tool"},"done":false}"#);
        let line = r#"{"message":{"content":"","tool_calls":[{"function":{"name":"write_file","arguments":{"path":"C:/x/y.txt","content":"hello"}}}],"role":"assistant"},"done":true}"#;
        match p.feed_line(line) {
            StreamLine::Done => {}
            _ => panic!("expected done"),
        }
        assert!(p.done);
        assert_eq!(p.tool_calls.len(), 1);
        assert_eq!(p.tool_calls[0].function.name, "write_file");
        assert_eq!(p.tool_calls[0].function.arguments["path"], "C:/x/y.txt");
    }

    #[test]
    fn malformed_line_is_skipped_without_panic() {
        let mut p = StreamParser::new();
        match p.feed_line("this is not json at all") {
            StreamLine::Skip => {}
            _ => panic!("expected skip"),
        }
        assert!(!p.done);
        assert!(p.content.is_empty());
    }

    #[test]
    fn think_detection_across_chunk_boundaries() {
        // the "<th" arrives in one chunk, "ink>" in the next
        let mut p = StreamParser::new();
        match p.feed_line(r#"{"message":{"content":"a <th"},"done":false}"#) {
            StreamLine::Content(_, t) => assert!(!t),
            _ => panic!(),
        }
        match p.feed_line(r#"{"message":{"content":"ink>b"},"done":false}"#) {
            StreamLine::Content(_, t) => assert!(t, "should detect think block after boundary split"),
            _ => panic!(),
        }
    }

    trait DoneExt {
        fn is_done(&self) -> bool;
    }
    impl DoneExt for StreamLine {
        fn is_done(&self) -> bool {
            matches!(self, StreamLine::Done)
        }
    }
}

#[cfg(test)]
mod prompt_tests {
    use super::*;

    fn prompt() -> String {
        OllamaClient::system_prompt("fox", "C:\\ayesha-os\\engine")
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
    #[ignore = "live smoke test: requires local ollama on localhost:11434"]
    fn local_ollama_streams_with_new_prompt_and_tools() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let result = rt.block_on(async {
            let prompt = OllamaClient::system_prompt("fox", "C:\\ayesha-os\\engine");
            let client = OllamaClient::new("ayesha:latest");
            let tools = crate::tool_defs::tool_definitions_core();
            let tools = tools.as_array().map(|a| a.as_slice()).unwrap_or(&[]);
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
        let r = result.expect("ollama stream should succeed");
        println!("--- ayesha reply ---\n{}\n--- end ---", r.content);
        println!("has_tool_calls: {}", r.has_tool_calls());
        println!("tool_calls: {:?}", r.tool_calls);
        assert!(!r.content.is_empty() || r.has_tool_calls(), "must reply or call a tool");
    }
}
