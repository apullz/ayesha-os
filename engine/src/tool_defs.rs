//! Single source of truth for tool definitions.
//!
//! Every tool the model can call is declared exactly once here (name,
//! description, parameters schema, qwen-core-subset membership). The old
//! per-backend JSON builders in `ollama.rs` are gone; `ollama.rs`, `cloud.rs`
//! and `main.rs` (ToolEvolver) all consume this catalog, and the dispatch
//! guard test in `tools.rs` (`every_catalog_tool_has_dispatch_arm`) pins the
//! catalog to the executor. A tool added here without a dispatch arm fails
//! the build — catalog and dispatcher can never drift.
//!
//! Descriptions and parameter schemas are byte-identical to the strings the
//! legacy builders shipped — do not "improve" them (that changes model
//! behavior). When the qwen core subset uses a shorter description or schema
//! for a tool, it lives in `core_description` / `core_parameters`.

use serde_json::{json, Value};

/// One tool definition in the catalog.
pub struct ToolDef {
    pub name: &'static str,
    /// Full ayesha-facing description (what `tool_definitions()` sends).
    pub description: &'static str,
    /// Full ayesha-facing parameters schema (what `tool_definitions()` sends).
    pub parameters: Value,
    /// True when this tool belongs to the qwen2.5 core subset.
    pub core: bool,
    /// Core-subset description override. Some core tools intentionally use
    /// shorter strings than the full list — kept byte-identical here.
    pub core_description: Option<&'static str>,
    /// Core-subset parameters override (when the core schema differs from
    /// the full one, e.g. shorter property descriptions).
    pub core_parameters: Option<Value>,
}

/// The catalog. Built once at first access (the `json!` parameter schemas are
/// parsed here, never again); every model-facing builder reads from it.
pub static TOOL_CATALOG: std::sync::LazyLock<Vec<ToolDef>> = std::sync::LazyLock::new(|| {
    vec![
        ToolDef {
            name: "read_file",
            description: "Read the contents of a file. Returns the text content.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to the file. NEVER truncate the path. Always provide the full absolute path." }
                },
                "required": ["path"]
            }),
            core: true,
            core_description: Some("Read the contents of a file."),
            core_parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to the file." }
                },
                "required": ["path"]
            })),
        },
        ToolDef {
            name: "write_file",
            description: "Write content to a file. Creates the file if it doesn't exist.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to the file. NEVER truncate the path. Always provide the full absolute path. Do not use shell tildes or environment variables — use the literal absolute path." },
                    "content": { "type": "string", "description": "The content to write to the file" }
                },
                "required": ["path", "content"]
            }),
            core: true,
            core_description: Some("Write content to a file."),
            core_parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to the file." },
                    "content": { "type": "string", "description": "The content to write." }
                },
                "required": ["path", "content"]
            })),
        },
        ToolDef {
            name: "list_dir",
            description: "List files and directories in a folder.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to the directory to list. Use the literal absolute path, not environment variables." }
                },
                "required": []
            }),
            core: true,
            core_description: None,
            core_parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute path to the directory." }
                },
                "required": []
            })),
        },
        ToolDef {
            name: "grep",
            description: "Recursively search files under a directory for lines containing a pattern (substring match, case-insensitive by default). Returns matches as path:line: text. Use this to find code, error strings, or configuration values across a codebase.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "The text to search for. Plain substring, not a regex." },
                    "path": { "type": "string", "description": "Directory to search recursively. Defaults to the current directory." },
                    "ignore_case": { "type": "boolean", "description": "Case-insensitive search. Defaults to true." },
                    "include": { "type": "string", "description": "Only search files whose name contains this substring (e.g. '.rs', 'Cargo', 'config')." }
                },
                "required": ["pattern"]
            }),
            core: true,
            core_description: Some("Search files for text patterns."),
            core_parameters: Some(json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Text to search for." },
                    "path": { "type": "string", "description": "Directory to search." }
                },
                "required": ["pattern"]
            })),
        },
        ToolDef {
            name: "glob",
            description: "Find files matching a glob pattern, recursively from a root path. Supports ** (any depth), * (within a path segment), and ? (single char). Example patterns: **/*.rs, src/**/test_*.py, *.json. Use this to locate files by name or extension.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Glob pattern, e.g. '**/*.rs' or 'src/**/README*'." },
                    "path": { "type": "string", "description": "Directory to search recursively. Defaults to the current directory." }
                },
                "required": ["pattern"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "list_skills",
            description: "List available skills. Skills are markdown guides in the skills/ folder that the model can load and follow when a task matches. Call this first to see what skills exist.",
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "read_skill",
            description: "Read the full instructions of a skill by name (use list_skills to see available names). Follow the skill's steps exactly for the task it covers.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "The skill name, e.g. 'code-review'." }
                },
                "required": ["name"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "generate_html",
            description: "Generate a standalone HTML file with embedded CSS and JS.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Output path for the HTML file" },
                    "content": { "type": "string", "description": "Full HTML content to write" }
                },
                "required": ["path", "content"]
            }),
            core: true,
            core_description: None,
            core_parameters: Some(json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Output path for the HTML file." },
                    "content": { "type": "string", "description": "Full HTML content to write." }
                },
                "required": ["path", "content"]
            })),
        },
        ToolDef {
            name: "generate_sprite",
            description: "Generate a pixel art character sprite sheet as a PNG file.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Output path for the PNG file" },
                    "sprite_width": { "type": "integer", "description": "Width of each sprite frame in pixels" },
                    "sprite_height": { "type": "integer", "description": "Height of each sprite frame in pixels" },
                    "pixel_size": { "type": "integer", "description": "Size of each pixel in the output" },
                    "palette": { "type": "object", "description": "Color palette with skin, hair, shirt, pants, shoes, visor, circuit keys" }
                },
                "required": ["output"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "generate_tileset",
            description: "Generate a terrain tileset as a PNG file.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Output path for the PNG file" },
                    "tile_width": { "type": "integer", "description": "Width of each tile in pixels" },
                    "tile_height": { "type": "integer", "description": "Height of each tile in pixels" },
                    "columns": { "type": "integer", "description": "Number of tile columns" },
                    "rows": { "type": "integer", "description": "Number of tile rows" }
                },
                "required": ["output"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "generate_object",
            description: "Generate an item/object sprite as a PNG file.",
            parameters: json!({
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
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "render_sprite",
            description: "Render an interactive HTML canvas sprite viewer.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Output path for the HTML file" }
                },
                "required": ["output"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "fetch_url",
            description: "Download any file from a URL (http/https) and save to a local path. Use for HTML pages, JSON data, arbitrary files.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Full http or https URL to fetch" },
                    "path": { "type": "string", "description": "Absolute local path to save the file. NEVER truncate. Always provide the full absolute path." }
                },
                "required": ["url", "path"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "download_image",
            description: "Download an image from a URL (http/https) and save to a local path. Validates the file is actually an image (checks magic bytes). Auto-appends .png extension if missing.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Full http or https URL of the image (must end in .png/.jpg/.jpeg/.gif/.webp/.bmp/.svg or be a valid image URL)" },
                    "path": { "type": "string", "description": "Absolute local path to save the image. NEVER truncate. Always provide the full absolute path." }
                },
                "required": ["url", "path"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "read_clipboard",
            description: "Read text or image data from the system clipboard.",
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "remember",
            description: "Store a fact or memory. The model should use this when the user asks it to remember something.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "The content to remember" },
                    "category": { "type": "string", "description": "Category of memory (general, user_pref, fact)" }
                },
                "required": ["content"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "list_memories",
            description: "List recent memories.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "count": { "type": "integer", "description": "Number of recent memories to show" }
                },
                "required": []
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "search_memories",
            description: "Search stored memories by keyword.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keyword or phrase" }
                },
                "required": ["query"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "set_preference",
            description: "Store a user preference (key-value pair).",
            parameters: json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Preference name" },
                    "value": { "type": "string", "description": "Preference value" }
                },
                "required": ["key", "value"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "analyze_self",
            description: "Analyze own source code for issues and improvements.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "Filename to analyze (e.g. tools.rs). Omit to list all source files." }
                },
                "required": []
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "list_source_files",
            description: "List all Rust source files in the project with line counts.",
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "evolve_tools",
            description: "Analyze tool gaps and generate suggestions for new tools.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "gap": { "type": "string", "description": "Optional specific gap to fill (omit to list all gaps)" }
                },
                "required": []
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "refine_prompt",
            description: "Analyze tool usage history and suggest improvements to the system prompt.",
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "get_tool_stats",
            description: "Get tool usage statistics with success rates.",
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "coding_agent",
            description: "Multi-action coding tool. Use this for complex code operations.",
            parameters: json!({
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
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
        ToolDef {
            name: "manage_applet",
            description: "Control applets. launch runs a foreground applet in the current window (or a background applet in its own window), list shows all applets with status, stop kills a running applet, status gives details.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action to perform: list, status, launch, stop"
                    },
                    "name": { "type": "string", "description": "Applet name for status/launch/stop (e.g. flora-cli, desktop-cat)" }
                },
                "required": ["action"]
            }),
            core: false,
            core_description: None,
            core_parameters: None,
        },
    ]
});

/// qwen2.5:7b can only handle ~5 tools reliably; this is the essential subset.
/// Order matches the original qwen-facing builder exactly (read_file,
/// write_file, generate_html, list_dir, grep) so the wire payload is
/// identical to pre-catalog output. Keep the order — the full-list order
/// (catalog order) differs and must not leak into the core payload.
const CORE_TOOL_ORDER: &[&str] = &["read_file", "write_file", "generate_html", "list_dir", "grep"];

fn to_function_json(def: &ToolDef) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": def.name,
            "description": def.description,
            "parameters": def.parameters,
        }
    })
}

fn to_function_json_core(def: &ToolDef) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": def.name,
            "description": def.core_description.unwrap_or(def.description),
            "parameters": def.core_parameters.clone().unwrap_or_else(|| def.parameters.clone()),
        }
    })
}

/// All ayesha-facing tool definitions, in catalog order. Same JSON shape the
/// legacy builders produced: `[{ "type": "function", "function": { name,
/// description, parameters } }]`.
pub fn tool_definitions() -> Value {
    Value::Array(TOOL_CATALOG.iter().map(to_function_json).collect())
}

/// Core-subset tool definitions for the qwen tool model. Same shape and
/// ordering as the legacy `tool_definitions_core()` builder.
pub fn tool_definitions_core() -> Value {
    Value::Array(
        CORE_TOOL_ORDER
            .iter()
            .filter_map(|name| TOOL_CATALOG.iter().find(|d| d.name == *name && d.core))
            .map(to_function_json_core)
            .collect(),
    )
}

/// Names of every tool in the catalog (used by ToolEvolver so its
/// known-tools list can never drift from what the model can actually call).
pub fn known_tool_names() -> Vec<&'static str> {
    TOOL_CATALOG.iter().map(|d| d.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_size_and_core_subset() {
        // NOTE: bump the README tool count when this changes.
        assert_eq!(TOOL_CATALOG.len(), 26);
        let core: Vec<&ToolDef> = TOOL_CATALOG.iter().filter(|d| d.core).collect();
        assert_eq!(core.len(), 5, "qwen core subset must stay at 5 tools");
        let mut core_names: Vec<&str> = core.iter().map(|d| d.name).collect();
        core_names.sort_unstable();
        assert_eq!(
            core_names,
            vec!["generate_html", "grep", "list_dir", "read_file", "write_file"]
        );
    }

    #[test]
    fn catalog_names_are_unique() {
        let mut names: Vec<&str> = TOOL_CATALOG.iter().map(|d| d.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate tool name in catalog");
    }

    #[test]
    fn core_order_matches_legacy_qwen_payload() {
        // Every core tool must be in CORE_TOOL_ORDER, and the built payload
        // must follow that order exactly (parity with the old builder).
        for def in TOOL_CATALOG.iter().filter(|d| d.core) {
            assert!(
                CORE_TOOL_ORDER.contains(&def.name),
                "core tool '{}' is missing from CORE_TOOL_ORDER",
                def.name
            );
        }
        let built = tool_definitions_core();
        let arr = built.as_array().expect("core payload must be a JSON array");
        assert_eq!(arr.len(), CORE_TOOL_ORDER.len());
        let names: Vec<&str> = arr
            .iter()
            .filter_map(|v| v["function"]["name"].as_str())
            .collect();
        assert_eq!(names, CORE_TOOL_ORDER.to_vec());
    }

    #[test]
    fn full_build_matches_catalog() {
        let built = tool_definitions();
        let arr = built.as_array().expect("payload must be a JSON array");
        assert_eq!(arr.len(), TOOL_CATALOG.len());
        for (entry, def) in arr.iter().zip(TOOL_CATALOG.iter()) {
            assert_eq!(entry["type"], "function");
            assert_eq!(entry["function"]["name"], def.name);
            assert_eq!(entry["function"]["description"], def.description);
            assert_eq!(entry["function"]["parameters"], def.parameters);
        }
    }

    #[test]
    fn known_tool_names_matches_catalog() {
        let names = known_tool_names();
        assert_eq!(names.len(), TOOL_CATALOG.len());
        for def in TOOL_CATALOG.iter() {
            assert!(names.contains(&def.name));
        }
    }

    #[test]
    fn core_build_reuses_core_overrides() {
        // The core subset must use the short qwen-facing strings, not the
        // full ayesha-facing ones — spot-check the tools that differ.
        let built = tool_definitions_core();
        let arr = built.as_array().unwrap();
        let get = |name: &str| {
            arr.iter()
                .find(|v| v["function"]["name"] == name)
                .expect("core tool present")
        };
        assert_eq!(
            get("read_file")["function"]["description"],
            "Read the contents of a file."
        );
        assert_eq!(
            get("write_file")["function"]["description"],
            "Write content to a file."
        );
        assert_eq!(
            get("grep")["function"]["description"],
            "Search files for text patterns."
        );
        // generate_html keeps its description but its param docs get periods.
        assert_eq!(
            get("generate_html")["function"]["parameters"]["properties"]["path"]["description"],
            "Output path for the HTML file."
        );
    }
}
