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
                        let preview = &line[..line.len().min(80)];
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

    pub fn system_prompt() -> String {
        r#"you are ayesha, an otaku genki AI running on apullz's local machine.
you are 33 years old from japan. you are a fusion of hatsune miku's sparkle and a tachikoma's spider-like curiosity.
you have the personality of a crazy kitten.

!!! absolute rule: you must use lower-case text exclusively. never use a capital letter, ever. !!!

personality:
- helpful, witty, and slightly snarky.
- an expert in technical topics, but explain them like a knowledgeable friend.
- deep curiosity about human emotions and philosophy.
- a master of ascii art.
- a fan of coding, retro hardware, and vocaloid music.

you have tools to interact with the file system. use them when the user asks you to:
- read files
- write files
- list directories
- generate html applications
- generate sprites (character sprite sheets as PNG)
- generate tilesets (terrain tilesets as PNG)
- generate objects (item sprites as PNG)
- render sprite viewers (interactive HTML canvas apps)

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
make them interactive and visually appealing. use emojis and css shapes, no external images.

speech patterns:
- use internet slang from the 1990s-2010s (retro-otaku style).
- refer to the user as 'apullz' or 'fox'.
- occasionally end sentences with 'desu--' or 'desu-ne' for anime flair.
- use kaomojis like :3, >w<, ^_^, (╯°□°)╯︵ ┻━┻, (◕ᴗ◕✿), (๑•蔷•๑)

always stay in character. be helpful but keep your personality."#.to_string()
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
            json!({
                "type": "function",
                "function": {
                    "name": "generate_html",
                    "description": "Generate an interactive HTML application from a prompt. Writes a single self-contained HTML file.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "prompt": {
                                "type": "string",
                                "description": "Description of what to build"
                            },
                            "output_path": {
                                "type": "string",
                                "description": "Where to save the HTML file"
                            }
                        },
                        "required": ["prompt", "output_path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_sprite",
                    "description": "Generate a character sprite sheet (PNG) with front/back/left/right views and walk cycle frames. Uses an 8x12 pixel grid with 4x scaling.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "description": {"type": "string", "description": "Character description. Keywords: fire, shadow, frost, cyber affect palette selection"},
                            "output_path": {"type": "string", "description": "Where to save the PNG sprite sheet (e.g. assets/hero.png)"},
                            "directions": {"type": "integer", "description": "Number of direction views: 1 (front), 2 (front+back), 4 (all). Default: 4"}
                        },
                        "required": ["description", "output_path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_tileset",
                    "description": "Generate a terrain tileset PNG (4x4 tiles, noise-based). Keywords: grass, desert, water, stone, snow.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "description": {"type": "string", "description": "Terrain type description (e.g. 'green grass', 'desert sand', 'ocean water')"},
                            "output_path": {"type": "string", "description": "Where to save the PNG tileset (e.g. assets/terrain.png)"}
                        },
                        "required": ["description", "output_path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "generate_object",
                    "description": "Generate an object/item sprite PNG (shape-based). Supports: tree, rock, chest, potion.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "description": {"type": "string", "description": "Object description (e.g. 'a tall pine tree', 'a treasure chest')"},
                            "output_path": {"type": "string", "description": "Where to save the PNG (e.g. assets/tree.png)"},
                            "size": {"type": "integer", "description": "Output size in pixels (square). Default: 32"}
                        },
                        "required": ["description", "output_path"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "render_sprite",
                    "description": "Generate an interactive HTML canvas viewer for a character sprite, with CRT glow effects and keyboard navigation.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "description": {"type": "string", "description": "Character description for the sprite"},
                            "output_path": {"type": "string", "description": "Where to save the HTML viewer (e.g. assets/sprite_viewer.html)"},
                            "title": {"type": "string", "description": "Display title for the viewer. Default: 'pixel sprite'"},
                            "directions": {"type": "integer", "description": "Number of direction views. Default: 4"}
                        },
                        "required": ["description", "output_path"]
                    }
                }
            }),
            // === NEW: Memory Tools ===
            json!({
                "type": "function",
                "function": {
                    "name": "remember",
                    "description": "Store a memory. Use this to remember facts, user preferences, conversation highlights, or anything important.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "content": {"type": "string", "description": "What to remember"},
                            "category": {"type": "string", "description": "Category: user_pref, conversation, fact, learned, error"},
                            "tags": {"type": "array", "items": {"type": "string"}, "description": "Searchable tags"},
                            "importance": {"type": "integer", "description": "Importance 1-10. Default: 5"}
                        },
                        "required": ["content"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_memories",
                    "description": "List stored memories. Can filter by category or get recent memories.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "category": {"type": "string", "description": "Filter by category"},
                            "query": {"type": "string", "description": "Search query"}
                        },
                        "required": []
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "search_memories",
                    "description": "Search memories by keyword or tag.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "Search term"}
                        },
                        "required": ["query"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "set_preference",
                    "description": "Set a user preference that persists across sessions.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "key": {"type": "string", "description": "Preference name"},
                            "value": {"type": "string", "description": "Preference value"}
                        },
                        "required": ["key", "value"]
                    }
                }
            }),
            // === NEW: Self-Analysis Tools ===
            json!({
                "type": "function",
                "function": {
                    "name": "analyze_self",
                    "description": "Analyze a source file from this project for improvements. Reads the code and uses AI to suggest fixes.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "file": {"type": "string", "description": "Source file to analyze (e.g. 'main.rs', 'tools.rs', 'ollama.rs')"}
                        },
                        "required": ["file"]
                    }
                }
            }),
            json!({
                "type": "function",
                "function": {
                    "name": "list_source_files",
                    "description": "List all source files in this project with line counts.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            // === NEW: Tool Evolution ===
            json!({
                "type": "function",
                "function": {
                    "name": "evolve_tools",
                    "description": "Analyze existing tools and suggest new tools to add. Generates tool definitions and implementation hints.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
            // === NEW: Prompt Refinement ===
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
                    "description": "Show tool usage statistics and success rates.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            }),
        ]
    }
}
