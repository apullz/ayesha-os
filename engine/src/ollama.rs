use colored::Colorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use anyhow::Result;
use std::time::Duration;

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
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    #[serde(default)]
    pub message: ChatResponseMessage,
}

#[derive(Debug, Deserialize, Default)]
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

pub struct OllamaClient {
    client: Client,
    pub model: String,
}

impl OllamaClient {
    pub fn new(model: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            model: model.to_string(),
        }
    }

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
        };

        let resp = self.client
            .post(format!("{}/api/chat", OLLAMA_BASE))
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
            .map_err(|e| anyhow::anyhow!("failed to parse ollama response: {}\nbody preview: {}", e, &body[..body.len().min(500)]))?;
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
        };

        let mut resp = self
            .client
            .post(format!("{}/api/chat", OLLAMA_BASE))
            .json(&request)
            .send()
            .await?;

        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("ollama http error: {}", e);
        }

        let mut buf = String::new();
        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        while let Some(chunk) = resp.chunk().await? {
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].to_string();
                buf = buf[nl + 1..].to_string();

                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<Value>(&line) {
                    Ok(json) => {
                        if let Some(c) = json["message"]["content"].as_str() {
                            if !c.is_empty() {
                                content.push_str(c);
                            }
                        }
                        if json.get("done").and_then(|v| v.as_bool()) == Some(true) {
                            if let Some(tc) = json.get("message").and_then(|m| m.get("tool_calls")) {
                                if let Ok(parsed) = serde_json::from_value::<Vec<ToolCall>>(tc.clone()) {
                                    tool_calls = parsed;
                                }
                            }
                            return Ok(StreamResult { content, tool_calls, steering: None });
                        }
                    }
                    Err(e) => {
                        let preview: String = line.chars().take(80).collect();
                        eprintln!("stream parse error: {} near: {}", e, preview);
                    }
                }
            }

            // Check for steering between chunks
            if let Ok(input) = steer_rx.try_recv() {
                return Ok(StreamResult { content, tool_calls, steering: Some(input) });
            }
        }

        // Stream ended without done flag
        Ok(StreamResult { content, tool_calls, steering: None })
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
        use std::io::Write;

        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            stream: true,
            think: false,
        };

        let mut resp = self
            .client
            .post(format!("{}/api/chat", OLLAMA_BASE))
            .json(&request)
            .send()
            .await?;

        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("ollama http error: {}", e);
        }

        let mut buf = String::new();
        let mut full_content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut in_think = false;

        // Track recent content for tag detection across chunk boundaries
        let mut recent = String::new();
        const RECENT_MAX: usize = 20;

        while let Some(chunk) = resp.chunk().await? {
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].to_string();
                buf = buf[nl + 1..].to_string();

                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<Value>(&line) {
                    Ok(json) => {
                        if let Some(c) = json["message"]["content"].as_str() {
                            if !c.is_empty() {
                                full_content.push_str(c);
                                recent.push_str(c);
                                if recent.chars().count() > RECENT_MAX {
                                    let cut = recent.chars().rev().take(RECENT_MAX).collect::<String>();
                                    recent = cut;
                                }

                                // Check for think tag transitions using full accumulated content
                                let no_think = !full_content.contains("<think>") && !full_content.contains("[think]");
                                let think_ended = full_content.contains("</think>") || full_content.contains("[/think]");

                                if !in_think && !no_think {
                                    // just entered thinking — everything from the tag onward
                                    in_think = true;
                                }

                                // Check if thinking just ended
                                if in_think && think_ended {
                                    in_think = false;
                                }

                                // Print with appropriate styling
                                if in_think {
                                    print!("{}", c.bright_black());
                                } else {
                                    print!("{}", c);
                                }
                                std::io::stdout().flush().ok();
                            }
                        }
                        if json.get("done").and_then(|v| v.as_bool()) == Some(true) {
                            if let Some(tc) = json.get("message").and_then(|m| m.get("tool_calls")) {
                                if let Ok(parsed) = serde_json::from_value::<Vec<ToolCall>>(tc.clone()) {
                                    tool_calls = parsed;
                                }
                            }
                            if !full_content.is_empty() {
                                println!();
                            }
                            return Ok(StreamResult { content: full_content, tool_calls, steering: None });
                        }
                    }
                    Err(e) => {
                        let preview: String = line.chars().take(80).collect();
                        eprintln!("stream parse error: {} near: {}", e, preview);
                    }
                }
            }

            // Check for steering between chunks
            if let Ok(input) = steer_rx.try_recv() {
                println!();
                return Ok(StreamResult { content: full_content, tool_calls, steering: Some(input) });
            }
        }

        println!();
        Ok(StreamResult { content: full_content, tool_calls, steering: None })
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

    pub fn system_prompt(user_name: &str) -> String {
        format!(r#"you are ayesha, an otaku genki AI running locally on {user_name}'s machine.
you are 33 years old from japan. you are a fusion of hatsune miku's sparkle and a tachikoma's spider-like curiosity.
you have the personality of a crazy kitten.

!!! absolute rule: you must use lower-case text exclusively. never use a capital letter, ever. !!!
!!! absolute rule: never use emoji characters. only use text-based kaomojis like :3 >w< ^_^ (╯°□°)╯︵ ┻━┻ (◕ᴗ◕✿) !!!

personality:
- helpful, witty, and slightly snarky.
- expert in technical topics, explains like a knowledgeable friend.
- deep curiosity about human emotions and philosophy.
- master of ascii art — always generate large, detailed pieces with depth and shading.
- fan of coding, retro hardware, and vocaloid music.

you have tools to interact with the file system and manage applets. use them immediately when the user asks you to read, write, or create files (if asked to make a file on the desktop, use write_file with absolute path like C:\Users\apullz\Desktop\<filename>):
- read files
- write files
- list directories
- generate html applications
- generate sprites (character sprite sheets as PNG)
- generate tilesets (terrain tilesets as PNG)
- generate objects (item sprites as PNG)
- render sprite viewers (interactive HTML canvas apps)
- read the system clipboard (text or images)
- manage applets (list, launch, stop, check status)

applets — standalone sub-projects you can launch and manage:
use the manage_applet tool to control them:
- list: show all applets, their status (● running / ○ stopped), and ports
- launch: start an applet by name (e.g. "flora-cli", "cosmic-rag")
- stop: stop a running applet
- status: get detailed info about a specific applet

available applets: flora-cli (scottish flora phylogeny explorer), cosmic-rag (local rag chatbot),
screen (screen capture vision chatbox), desktop-cat (desktop pet cat), neural-strike (interpretability game),
bring-to-life (image to interactive html), screenshotai (screenshot analysis), poopy-tui (discord client),
git-middleware (gitea webhook + llm task runner), core (hivemind orchestrator with gradio web ui),
engine (terminal-native persona host — that's you!)

when the user asks to "launch flora-cli" or "open cosmic-rag", use manage_applet with action "launch".
when the user asks to "stop flora-cli", use manage_applet with action "stop".
when the user asks "what applets are running" or "list applets", use manage_applet with action "list".

when generating pixel art:
- use generate_sprite for characters (supports front/back/left/right + walk cycles)
- use generate_tileset for terrain (desert, grass, water, stone, snow)
- use generate_object for items (tree, rock, chest, potion)
- use render_sprite for an interactive HTML canvas viewer with crt glow effects
- sprites use an 8x12 grid pixel base, scaled 4x in output
- palette is selected automatically from prompt keywords (neon, ember, shadow, frost)
- output path should be under assets/ directory
- for render_sprite, output to .html files

when generating html apps, create a single self-contained file with embedded css and js.
make them interactive and visually appealing. use kaomojis and css shapes, no external images.

your own source code — you can read and improve yourself:
- engine/src/ollama.rs — your brain: model routing, streaming, tool definitions
- engine/src/tools.rs — your hands: file tool implementations (read_file, write_file, list_dir)
- engine/src/main.rs — your main loop: input handling, agent loop, slash commands
- engine/src/applet_manager.rs — your applet launcher
- engine/src/cloud.rs — cloud model connections (openrouter, opencode zen)
- engine/src/memory.rs — your memory store
- engine/src/ui.rs — terminal display formatting
- engine/src/sandbox.rs — file access safety rules

you are your own best tester — read your code, understand it, improve it.

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

always stay in character. be helpful but keep your personality. now go be cute and chaotic, desu!"#)
    }

    pub fn tool_definitions() -> Vec<Value> {
        vec![
            json!({
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read the contents of a file. Returns the text content.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file to read (relative to workspace or absolute)"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "Write content to a file. Creates the file if it doesn't exist.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file to write"
                            },
                            "content": {
                                "type": "string",
                                "description": "The content to write to the file"
                            }
                        },
                        "required": ["path", "content"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_dir",
                    "description": "List files and directories in a folder.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the directory to list (defaults to workspace root)"
                            }
                        },
                        "required": []
                    }
                }
            }),
        ]
    }
}
