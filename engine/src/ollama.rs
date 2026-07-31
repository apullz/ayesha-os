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

#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    pub model: String,
    base_url: String,
}

/// Pure NDJSON stream parser for ollama's /api/chat streaming format.
/// No I/O — feed it trimmed lines, it accumulates content + tool calls and
/// tracks think-block state. Unit-testable and shared by the visible and
/// collect streaming paths so they can never drift apart.
pub struct StreamParser {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub done: bool,
    in_think: bool,
    recent: String,
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
                self.content.push_str(c);
                self.recent.push_str(c);
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
                return StreamLine::Content(c.to_string(), was_in_think || just_entered);
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

    pub fn finish(self) -> StreamResult {
        StreamResult { content: self.content, tool_calls: self.tool_calls, steering: None }
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
                    StreamLine::Content(text, thinking) => {
                        if thinking {
                            print!("{}", text.bright_black());
                        } else {
                            print!("{}", text);
                        }
                        std::io::stdout().flush().ok();
                    }
                    StreamLine::Done => {
                        if !parser.content.is_empty() {
                            println!();
                        }
                        return Ok(parser.finish());
                    }
                    StreamLine::Skip => {}
                }
            }

            // Check for steering between chunks
            if let Ok(input) = steer_rx.try_recv() {
                println!();
                return Ok(StreamResult {
                    content: parser.content,
                    tool_calls: parser.tool_calls,
                    steering: Some(input),
                });
            }
        }

        println!();
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
- launch: start an applet by name (e.g. "flora-cli", "desktop-cat")
- stop: stop a running applet
- status: get detailed info about a specific applet

some applets are "foreground" applets (flora-cli, poopy-tui): when launched they take over
the current terminal window and run there like a page of ayesha-os. the user returns to
ayesha when the applet exits, or by pressing ctrl+p to open the page switcher.
the rest open in their own separate window.

available applets: desktop-cat (desktop pet cat), flora-cli (scottish flora phylogeny explorer, in-window),
neural-strike (interpretability game), poopy-tui (discord client, in-window),
git-middleware (gitea webhook + llm task runner), core (hivemind orchestrator with gradio web ui),
engine (terminal-native persona host — that's you!)

when the user asks to "launch flora-cli" or "open desktop-cat", use manage_applet with action "launch".
foreground applets will run in the current window; background applets open in their own window.
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
                            "path": { "type": "string", "description": "Absolute path to the file" }
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
                            "path": { "type": "string", "description": "Absolute path to the file to write" },
                            "content": { "type": "string", "description": "The content to write to the file" }
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
                            "path": { "type": "string", "description": "Absolute path to the directory to list" }
                        },
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_html",
                    "description": "Generate a standalone HTML file with embedded CSS and JS.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Output path for the HTML file" },
                            "content": { "type": "string", "description": "Full HTML content to write" }
                        },
                        "required": ["path", "content"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_sprite",
                    "description": "Generate a pixel art character sprite sheet as a PNG file.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "output": { "type": "string", "description": "Output path for the PNG file" },
                            "sprite_width": { "type": "integer", "description": "Width of each sprite frame in pixels" },
                            "sprite_height": { "type": "integer", "description": "Height of each sprite frame in pixels" },
                            "pixel_size": { "type": "integer", "description": "Size of each pixel in the output" },
                            "palette": { "type": "object", "description": "Color palette with skin, hair, shirt, pants, shoes, visor, circuit keys" }
                        },
                        "required": ["output"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_tileset",
                    "description": "Generate a terrain tileset as a PNG file.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "output": { "type": "string", "description": "Output path for the PNG file" },
                            "tile_width": { "type": "integer", "description": "Width of each tile in pixels" },
                            "tile_height": { "type": "integer", "description": "Height of each tile in pixels" },
                            "columns": { "type": "integer", "description": "Number of tile columns" },
                            "rows": { "type": "integer", "description": "Number of tile rows" }
                        },
                        "required": ["output"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_object",
                    "description": "Generate an item/object sprite as a PNG file.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "output": { "type": "string", "description": "Output path for the PNG file" },
                            "width": { "type": "integer", "description": "Width in pixels" },
                            "height": { "type": "integer", "description": "Height in pixels" },
                            "pixel_size": { "type": "integer", "description": "Size of each pixel" },
                            "color_r": { "type": "integer", "description": "Red component of main color" },
                            "color_g": { "type": "integer", "description": "Green component of main color" },
                            "color_b": { "type": "integer", "description": "Blue component of main color" }
                        },
                        "required": ["output"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "render_sprite",
                    "description": "Render an interactive HTML canvas sprite viewer.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "output": { "type": "string", "description": "Output path for the HTML file" }
                        },
                        "required": ["output"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "read_clipboard",
                    "description": "Read text or image data from the system clipboard.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "remember",
                    "description": "Store a fact or memory. The model should use this when the user asks it to remember something.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "content": { "type": "string", "description": "The content to remember" },
                            "category": { "type": "string", "description": "Category of memory (general, user_pref, fact)" }
                        },
                        "required": ["content"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_memories",
                    "description": "List recent memories.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "count": { "type": "integer", "description": "Number of recent memories to show" }
                        },
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "search_memories",
                    "description": "Search stored memories by keyword.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search keyword or phrase" }
                        },
                        "required": ["query"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "set_preference",
                    "description": "Store a user preference (key-value pair).",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "key": { "type": "string", "description": "Preference name" },
                            "value": { "type": "string", "description": "Preference value" }
                        },
                        "required": ["key", "value"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "analyze_self",
                    "description": "Analyze own source code for issues and improvements.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "file": { "type": "string", "description": "Filename to analyze (e.g. tools.rs). Omit to list all source files." }
                        },
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_source_files",
                    "description": "List all Rust source files in the project with line counts.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "evolve_tools",
                    "description": "Analyze tool gaps and generate suggestions for new tools.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "gap": { "type": "string", "description": "Optional specific gap to fill (omit to list all gaps)" }
                        },
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "refine_prompt",
                    "description": "Analyze tool usage history and suggest improvements to the system prompt.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "get_tool_stats",
                    "description": "Get tool usage statistics with success rates.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "coding_agent",
                    "description": "Multi-action coding tool. Use this for complex code operations.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "action": {
                                "type": "string",
                                "description": "Action to perform: read, write, edit, list, analyze, modify, suggest"
                            },
                            "path": { "type": "string", "description": "File path relative to project root" },
                            "content": { "type": "string", "description": "Content for write action" },
                            "edits": {
                                "type": "array",
                                "description": "Array of edit operations for edit action",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "action": { "type": "string" },
                                        "old": { "type": "string" },
                                        "new": { "type": "string" },
                                        "text": { "type": "string" },
                                        "after": { "type": "string" }
                                    }
                                }
                            },
                            "instruction": { "type": "string", "description": "Natural language instruction for modify action" }
                        },
                        "required": ["action", "path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "manage_applet",
                    "description": "Control applets. launch runs a foreground applet in the current window (or a background applet in its own window), list shows all applets with status, stop kills a running applet, status gives details.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "action": {
                                "type": "string",
                                "description": "Action to perform: list, status, launch, stop"
                            },
                            "name": { "type": "string", "description": "Applet name for status/launch/stop (e.g. flora-cli, desktop-cat)" }
                        },
                        "required": ["action"]
                    }
                }
            }),
        ]
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
