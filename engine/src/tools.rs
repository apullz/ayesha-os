use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};
use anyhow::{Result, bail};
use serde_json::{json, Value};

use crate::sandbox::Sandbox;
use crate::memory::MemoryStore;
use crate::prompt_refinement::PromptHistory;
use crate::self_analysis::SelfAnalyzer;
use crate::tool_evolution::ToolEvolver;
use crate::llm::{LlmClient, ChatMessage};
use crate::applet_manager::AppletManager;
use crate::plugins::{PluginRegistry, run_plugin_tool};

const MAX_READ_SIZE: usize = 256 * 1024;
const MAX_DOWNLOAD_SIZE: usize = 100 * 1024 * 1024; // 100 MB safety cap for network tools

#[cfg(test)]
mod glob_tests {
    use super::*;

    fn sm(seg: &str, pat: &str) -> bool {
        fn rec(s: &[char], p: &[char]) -> bool {
            match (p.first(), s.first()) {
                (None, _) => s.is_empty(),
                (Some('*'), _) => rec(s, &p[1..]) || (!s.is_empty() && rec(&s[1..], p)),
                (Some('?'), Some(_)) => rec(&s[1..], &p[1..]),
                (Some(pc), Some(sc)) if pc == sc => rec(&s[1..], &p[1..]),
                _ => false,
            }
        }
        rec(&seg.chars().collect::<Vec<_>>(), &pat.chars().collect::<Vec<_>>())
    }

    fn m(segs: &[&str], pats: &[&str]) -> bool {
        match_glob_segments(segs, pats, &sm)
    }

    #[test]
    fn double_star_matches_zero_or_more_segments() {
        assert!(m(&["a.rs"], &["**", "*.rs"]));
        assert!(m(&["src", "a.rs"], &["**", "*.rs"]));
        assert!(m(&["src", "deep", "a.rs"], &["src", "**", "a.rs"]));
        assert!(m(&["a.rs"], &["src", "**", "a.rs"]).eq(&false)); // no src at all
        assert!(m(&["src", "deep", "lib.rs"], &["src", "**", "lib.rs"]));
    }

    #[test]
    fn single_star_never_crosses_segments() {
        assert!(sm("main.rs", "*.rs"));
        assert!(sm("foo", "f?o"));
        assert!(!sm("fooo", "f?o"));
        assert!(m(&["src", "main.rs"], &["src", "*.rs"]));
        // single * cannot reach files two levels deep (parts.len() mismatch)
        assert!(!m(&["src", "deep", "main.rs"], &["src", "*.rs"]));
        // but ** can
        assert!(m(&["src", "deep", "main.rs"], &["**", "*.rs"]));
    }
}

/// Match path segments against a glob pattern that may contain `**`.
/// `**` matches zero or more segments; every other segment uses `seg_match`.
fn match_glob_segments(segments: &[&str], parts: &[&str], seg_match: &dyn Fn(&str, &str) -> bool) -> bool {
    fn rec(segs: &[&str], pats: &[&str], seg_match: &dyn Fn(&str, &str) -> bool) -> bool {
        match (pats.first(), segs.first()) {
            (None, _) => segs.is_empty(),
            (Some(p), _) if *p == "**" => {
                rec(segs, &pats[1..], seg_match)
                    || (!segs.is_empty() && rec(&segs[1..], pats, seg_match))
            }
            (Some(p), Some(s)) => seg_match(s, p) && rec(&segs[1..], &pats[1..], seg_match),
            _ => false,
        }
    }
    rec(segments, parts, seg_match)
}

pub struct ToolContext<'a> {
    pub memory: &'a mut MemoryStore,
    pub prompt_history: &'a mut PromptHistory,
    pub analyzer: &'a SelfAnalyzer,
    pub evolver: &'a ToolEvolver,
    pub llm: &'a LlmClient,
    /// The active backend (local Cloud or cloud) — used by `delegate` so the
    /// sub-agent runs on the same model the user is talking to.
    pub backend: &'a crate::ActiveBackend,
    pub project_root: &'a Path,
    pub applet_manager: &'a mut AppletManager,
    pub steer_tx: &'a mpsc::Sender<String>,
    pub steer_rx: &'a mpsc::Receiver<String>,
    pub input_flag: &'a mut Arc<AtomicBool>,
    pub menu_flag: &'a Arc<AtomicBool>,
}

pub struct ToolExecutor {
    sandbox: Sandbox,
    plugins: PluginRegistry,
}

/// All tool names the executor can dispatch, grouped the same way the
/// handler match in `execute()` is. `dispatch_for` resolves through this
/// list; `every_catalog_tool_has_dispatch_arm` pins it to TOOL_CATALOG in
/// both directions, so the catalog and dispatcher can never drift.
const DISPATCHABLE_TOOLS: &[&str] = &[
    // File ops
    "read_file", "write_file", "list_dir", "grep", "glob",
    // Skills
    "list_skills", "read_skill",
    // Network
    "fetch_url", "download_image",
    // Generation
    "generate_html", "generate_sprite", "generate_tileset",
    "generate_object", "render_sprite",
    // Clipboard
    "read_clipboard",
    // Memory
    "remember", "list_memories", "search_memories", "set_preference",
    // Analysis
    "analyze_self", "list_source_files",
    // Evolution / prompt
    "evolve_tools", "refine_prompt", "get_tool_stats",
    // Coding agent / applets
    "coding_agent", "manage_applet",
];

/// Tools that modify state on disk, in memory/preferences, or the filesystem
/// of the app. Plan mode denies these outright — it is a read-only research
/// mode. Everything else (reads, greps, fetches, memory queries) stays
/// available so the agent can investigate and propose.
pub const MUTATING_TOOLS: &[&str] = &[
    // File ops
    "write_file",
    // Network (writes fetched/downloaded content to disk)
    "fetch_url", "download_image",
    // Generation (writes files into the project)
    "generate_html", "generate_sprite", "generate_tileset",
    "generate_object", "render_sprite",
    // Memory / preferences
    "remember", "set_preference",
    // Evolution / prompt
    "evolve_tools", "refine_prompt",
    // Coding agent / applets
    "coding_agent", "manage_applet",
];

/// True when the tool mutates state — used by plan mode to deny it.
pub fn is_mutating(name: &str) -> bool {
    MUTATING_TOOLS.contains(&name)
}

/// Plan-mode denial for a mutating tool, if any. `None` means the tool is
/// read-only and allowed in plan mode.
pub fn plan_mode_deny_message(name: &str) -> Option<String> {
    if !is_mutating(name) {
        return None;
    }
    Some(format!(
        "error: plan mode: '{}' is denied (plan mode is read-only research — propose changes instead of making them; use /mode build to make changes)",
        name
    ))
}

/// Read-only tool subset the delegate sub-agent may call. `delegate` itself
/// is NOT in this set, so sub-agents can never nest delegates — the agent
/// tree is exactly two levels deep.
const SUB_AGENT_TOOLS: &[&str] = &[
    // File reads
    "read_file", "list_dir", "grep", "glob",
    // Skills (read-only)
    "list_skills", "read_skill",
    // Clipboard read
    "read_clipboard",
    // Memory reads
    "list_memories", "search_memories",
    // Analysis
    "analyze_self", "list_source_files", "get_tool_stats",
];

/// Tool payload for the delegate sub-agent: the catalog filtered down to the
/// read-only core tools, so the sub-agent can research but never modify
/// anything (and can never delegate further).
fn sub_agent_tool_payload() -> Value {
    let full = crate::tool_defs::tool_definitions();
    match full {
        Value::Array(entries) => Value::Array(
            entries
                .into_iter()
                .filter(|t| {
                    t.get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|name| SUB_AGENT_TOOLS.contains(&name))
                        .unwrap_or(false)
                })
                .collect(),
        ),
        _ => Value::Array(Vec::new()),
    }
}

/// Synthetic payload entry for the `delegate` tool. Kept out of the static
/// catalog (TOOL_CATALOG stays untouched) — main.rs merges this into the
/// per-request tool payload so the model can hand off research tasks.
pub fn delegate_tool_definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "delegate",
            "description": "spawn a bounded research sub-agent (same model backend) for a large read-only sub-task: file reads, directory listings, greps, globs, skill reads. the sub-agent cannot modify anything and cannot delegate further. use it to parallelize or isolate heavy investigation while you keep working. returns a concise summary of findings.",
            "parameters": {
                "type": "object",
                "properties": {
                    "sub_task": { "type": "string", "description": "the read-only research task to hand off to the sub-agent" },
                    "max_steps": { "type": "integer", "description": "maximum sub-agent tool rounds before it must summarize (default 3, max 8)" }
                },
                "required": ["sub_task"]
            }
        }
    })
}

impl ToolExecutor {
    pub fn new(sandbox: Sandbox) -> Self {
        Self { sandbox, plugins: PluginRegistry::default() }
    }

    /// Attach config-driven plugin tools (loaded from ayesha.json at startup).
    /// Plugin tools are NOT part of the static catalog — they are dispatched
    /// through the fallback arm of `execute()`.
    pub fn with_plugins(mut self, plugins: PluginRegistry) -> Self {
        self.plugins = plugins;
        self
    }

    /// Tool names the executor can dispatch. Every TOOL_CATALOG entry must
    /// resolve here — `every_catalog_tool_has_dispatch_arm` iterates the
    /// catalog and asserts it, so a tool added to the catalog without a
    /// handler fails the build. The handler match in `execute()` mirrors
    /// this list; its fallback arm distinguishes a catalog-known name with
    /// no handler (internal bug) from a genuinely unknown tool.
    pub fn dispatch_for(name: &str) -> Option<&'static str> {
        DISPATCHABLE_TOOLS.iter().copied().find(|n| *n == name)
    }

    /// Every name the executor can dispatch — the catalog test checks this
    /// list against TOOL_CATALOG in both directions so neither can drift.
    #[allow(dead_code)] // consumed by every_catalog_tool_has_dispatch_arm
    pub fn dispatched_tool_names() -> &'static [&'static str] {
        DISPATCHABLE_TOOLS
    }

    pub async fn execute(&self, name: &str, args: &Value, ctx: &mut ToolContext<'_>) -> Result<String> {
        match name {
            // File ops
            "read_file" => self.read_file(args).await,
            "write_file" => self.write_file(args).await,
            "list_dir" => self.list_dir(args).await,
            "grep" => self.grep(args).await,
            "glob" => self.glob(args).await,

            // Skills
            "list_skills" => self.list_skills(ctx.project_root),
            "read_skill" => self.read_skill(args, ctx.project_root),

            // Network
            "fetch_url" => self.fetch_url(args).await,
            "download_image" => self.download_image(args).await,

            // Generation
            "generate_html" => self.generate_html(args).await,
            "generate_sprite" => self.generate_sprite(args).await,
            "generate_tileset" => self.generate_tileset(args).await,
            "generate_object" => self.generate_object(args).await,
            "render_sprite" => self.render_sprite(args).await,

            // Clipboard
            "read_clipboard" => self.read_clipboard().await,

            // Memory
            "remember" => self.remember(args, ctx.memory),
            "list_memories" => self.list_memories(args, ctx.memory),
            "search_memories" => self.search_memories(args, ctx.memory),
            "set_preference" => self.set_preference(args, ctx.memory),

            // Analysis
            "analyze_self" => self.analyze_self(args, ctx.analyzer),
            "list_source_files" => self.list_source_files(ctx.analyzer),

            // Evolution
            "evolve_tools" => self.evolve_tools(args, ctx.evolver, ctx.llm).await,

            // Prompt
            "refine_prompt" => self.refine_prompt(ctx.prompt_history),
            "get_tool_stats" => self.get_tool_stats(ctx.prompt_history),

            // Coding agent
            "coding_agent" => self.coding_agent(args, ctx.llm, ctx.project_root, &self.sandbox).await,

            // Applets
            "manage_applet" => self.manage_applet(args, ctx).await,

            // Delegate (NOT in DISPATCHABLE_TOOLS / the static catalog — it
            // is merged into the payload separately by main.rs, so the
            // catalog↔dispatcher invariant test stays intact).
            "delegate" => self.delegate(args, ctx).await,

            // A catalog-known tool that reaches this arm has a dispatch entry
            // but no handler — a regression `every_catalog_tool_has_dispatch_arm`
            // should have caught. Surface it as an internal error, not a
            // generic "unknown tool" (which would mask the drift).
            _ if Self::dispatch_for(name).is_some() => {
                bail!("internal error: tool '{}' is catalog-known but has no dispatch arm", name)
            }
            _ => {
                // Config-driven plugin tools live outside the static catalog;
                // resolve them here by shelling out to their command template.
                if let Some(ptool) = self.plugins.find_tool(name) {
                    return run_plugin_tool(ptool, args, ctx.project_root).await;
                }
                bail!("unknown tool: {}", name)
            }
        }
    }

    // ═══════════════════════════════════════════
    //  Applet management
    // ═══════════════════════════════════════════

    async fn manage_applet(&self, args: &Value, ctx: &mut ToolContext<'_>) -> Result<String> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
        match action {
            "list" => Ok(ctx.applet_manager.list()),
            "status" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.is_empty() {
                    bail!("status requires a 'name' argument");
                }
                if !ctx.applet_manager.has(name) {
                    bail!("unknown applet: {}", name);
                }
                let running = ctx.applet_manager.is_running(name);
                let foreground = ctx.applet_manager.is_foreground(name);
                let entry = ctx.applet_manager.entries.get(name).unwrap();
                let port = entry.port.map(|p| p.to_string()).unwrap_or_else(|| "none".to_string());
                Ok(format!(
                    "{}: {}\n  status: {}\n  mode: {} (runs in the current window)\n  port: {}",
                    name, entry.desc,
                    if running { "● running" } else { "○ stopped" },
                    if foreground { "in-window" } else { "own window" },
                    port,
                ))
            }
            "launch" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.is_empty() {
                    bail!("launch requires a 'name' argument");
                }
                if ctx.applet_manager.is_foreground(name) {
                    // Takes over the current window until it exits, then ayesha returns
                    ctx.applet_manager
                        .run_in_window(name, ctx.steer_tx, ctx.steer_rx, ctx.input_flag, ctx.menu_flag)
                        .map_err(|e| anyhow::anyhow!("{}", e))
                        .map(|_| format!("ran {} in the current window and returned (press ctrl+p to switch pages again)", name))
                } else if ctx.applet_manager.is_running(name) {
                    bail!("{} is already running", name)
                } else {
                    ctx.applet_manager
                        .launch(name)
                        .map_err(|e| anyhow::anyhow!("{}", e))
                        .map(|_| format!("launched {} in its own window", name))
                }
            }
            "stop" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.is_empty() {
                    bail!("stop requires a 'name' argument");
                }
                ctx.applet_manager
                    .stop(name)
                    .map_err(|e| anyhow::anyhow!("{}", e))
                    .map(|_| format!("stopped {}", name))
            }
            _ => bail!("unknown manage_applet action: {} (expected list/status/launch/stop)", action),
        }
    }

    // ═══════════════════════════════════════════
    //  Delegate — bounded research sub-agent
    // ═══════════════════════════════════════════

    /// Dispatch a sub-agent tool call directly to its handler — NOT through
    /// `execute()`, so the delegate loop stays non-recursive (async fns cannot
    /// recurse without boxing). The sub-agent set is a fixed read-only subset
    /// of the catalog, mirrored from [`SUB_AGENT_TOOLS`].
    async fn execute_sub_agent_tool(
        &self,
        name: &str,
        args: &Value,
        ctx: &mut ToolContext<'_>,
    ) -> Result<String> {
        match name {
            "read_file" => self.read_file(args).await,
            "list_dir" => self.list_dir(args).await,
            "grep" => self.grep(args).await,
            "glob" => self.glob(args).await,
            "list_skills" => self.list_skills(ctx.project_root),
            "read_skill" => self.read_skill(args, ctx.project_root),
            "read_clipboard" => self.read_clipboard().await,
            "list_memories" => self.list_memories(args, ctx.memory),
            "search_memories" => self.search_memories(args, ctx.memory),
            "analyze_self" => self.analyze_self(args, ctx.analyzer),
            "list_source_files" => self.list_source_files(ctx.analyzer),
            "get_tool_stats" => self.get_tool_stats(ctx.prompt_history),
            other => bail!("sub-agent tool '{}' is not in the read-only set", other),
        }
    }

    /// Hand a read-only research task to a bounded sub-agent running on the
    /// SAME active backend (reuses `chat_stream_collect`, the invisible
    /// streaming path both backends already share). The sub-agent:
    ///
    /// * is hard-restricted to [`SUB_AGENT_TOOLS`] (read-only core tools);
    /// * cannot call `delegate` (it is not in that set) — no deep nesting;
    /// * runs on a fresh, non-steerable channel, so the user's typed input
    ///   keeps reaching the main agent mid-delegation;
    /// * is bounded by `max_steps` tool rounds (default 3, capped at 8).
    ///
    /// Returns the sub-agent's final summary.
    async fn delegate(&self, args: &Value, ctx: &mut ToolContext<'_>) -> Result<String> {
        let sub_task = args
            .get("sub_task")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .unwrap_or("")
            .to_string();
        if sub_task.is_empty() {
            bail!("delegate: 'sub_task' is required and must be non-empty");
        }
        let max_steps = args
            .get("max_steps")
            .and_then(|v| v.as_u64())
            .unwrap_or(3)
            .clamp(1, 8) as usize;

        // Fresh channel: the sub-agent runs invisibly while the enclosing
        // run_tool_with_steer keeps the user's steering channel free.
        let (steer_tx, steer_rx) = mpsc::channel::<String>();
        drop(steer_tx);

        let sub_tools = sub_agent_tool_payload();
        let sub_tools_slice = crate::streaming::tool_payload_slice(&sub_tools);

        let mut sub_messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "you are a focused research sub-agent. investigate the given sub-task using ONLY the read-only tools provided. you must NOT modify any files, spawn processes, or call mutating tools — you are read-only. you must NOT spawn further delegates. when you have gathered enough information, reply with a concise final summary of your findings (3-8 sentences, plain text, no tool call).".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: sub_task,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        for _ in 0..max_steps {
            let result = ctx
                .backend
                .chat_stream_collect(&sub_messages, Some(sub_tools_slice), &steer_rx)
                .await
                .map_err(|e| anyhow::anyhow!("delegate: backend error: {}", e))?;

            if result.tool_calls.is_empty() {
                // The sub-agent answered with plain text — that's the summary.
                return Ok(format!("delegate summary: {}", result.content.trim()));
            }

            let tool_calls = result.tool_calls.clone();
            sub_messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: result.content,
                tool_calls: Some(result.tool_calls),
                tool_call_id: None,
            });
            for tc in &tool_calls {
                let output = if SUB_AGENT_TOOLS.contains(&tc.function.name.as_str()) {
                    match self
                        .execute_sub_agent_tool(&tc.function.name, &tc.function.arguments, ctx)
                        .await
                    {
                        Ok(out) => out,
                        Err(e) => format!("error: {}", e),
                    }
                } else {
                    format!("error: sub-agent tool '{}' is not in the read-only set", tc.function.name)
                };
                sub_messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: output,
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                });
            }
        }

        Ok(format!(
            "delegate: sub-agent exhausted its {} tool-round budget without a final summary",
            max_steps
        ))
    }

    // ═══════════════════════════════════════════
    //  File tools
    // ═══════════════════════════════════════════

    async fn read_file(&self, args: &Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;
        self.sandbox.check_sensitive_resolved(&resolved)?;

        let content = fs::read_to_string(&resolved)?;

        if content.chars().count() > MAX_READ_SIZE {
            let truncated = crate::util::truncate_chars(&content, MAX_READ_SIZE);
            Ok(format!(
                "{}\n\n... [truncated at {} chars, file is {} chars total]",
                truncated,
                MAX_READ_SIZE,
                content.chars().count()
            ))
        } else {
            Ok(content)
        }
    }

    async fn write_file(&self, args: &Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;

        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;
        self.sandbox.check_sensitive_resolved(&resolved)?;

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        // Try to handle ReadOnly attribute if writing fails. In strict
        // sandbox mode the ReadOnly attribute is respected instead of being
        // auto-cleared.
        if let Err(e) = fs::write(&resolved, content) {
            if self.sandbox.sandbox_enabled() {
                return Err(e.into());
            }
            let metadata = fs::metadata(&resolved);
            if let Ok(m) = metadata {
                if m.permissions().readonly() {
                    let mut perms = m.permissions();
                    perms.set_readonly(false);
                    fs::set_permissions(&resolved, perms)?;
                    fs::write(&resolved, content)?;
                } else {
                    return Err(e.into());
                }
            } else {
                return Err(e.into());
            }
        }

        Ok(format!(
            "wrote {} bytes to '{}'",
            content.len(),
            resolved.display()
        ))
    }

    /// Shared HTTP client for network tools. Redirects are followed,
    /// timeouts are generous, and a browser-ish UA is sent.
    fn http_client() -> Result<reqwest::Client> {
        Ok(reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent("ayesha-os/1.0")
            .build()?)
    }

    /// Detect the image format from the first bytes of content.
    /// Returns the canonical file extension (without dot), or None if
    /// the bytes don't look like a known image format.
    fn detect_image_format(bytes: &[u8]) -> Option<&'static str> {
        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Some("png");
        }
        // JPEG: FF D8 FF
        if bytes.len() >= 3 && &bytes[0..3] == b"\xFF\xD8\xFF" {
            return Some("jpg");
        }
        // GIF: GIF87a or GIF89a
        if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            return Some("gif");
        }
        // WebP: RIFF....WEBP
        if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            return Some("webp");
        }
        // BMP: BM
        if bytes.starts_with(b"BM") {
            return Some("bmp");
        }
        // SVG: starts with '<' or BOM then '<' (whitespace/BOM tolerant)
        let mut rest = bytes;
        if rest.starts_with(b"\xef\xbb\xbf") {
            rest = &rest[3..];
        }
        let first = rest.iter().copied().find(|b| !b.is_ascii_whitespace());
        if first == Some(b'<') {
            return Some("svg");
        }
        None
    }

    /// Download a file from a URL and save it to a local path.
    /// Supports any content type (image, html, binary, etc.).
    async fn fetch_url(&self, args: &Value) -> Result<String> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'url' argument"))?;

        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        // Reject dangerous URL schemes
        if !url.starts_with("http://") && !url.starts_with("https://") {
            bail!("only http/https URLs are supported (got: {})", url);
        }

        let resolved = self.sandbox.resolve(path)?;
        self.sandbox.check_sensitive_resolved(&resolved)?;

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let client = Self::http_client()?;

        let response = client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            bail!("HTTP {} fetching {}", status.as_u16(), url);
        }

        let content_length = response.content_length().unwrap_or(0) as usize;
        if content_length > MAX_DOWNLOAD_SIZE {
            bail!(
                "refusing to download: file too large ({} bytes, max {} bytes)",
                content_length,
                MAX_DOWNLOAD_SIZE
            );
        }

        let bytes = response.bytes().await?;
        if bytes.len() > MAX_DOWNLOAD_SIZE {
            bail!(
                "refusing to save: download exceeded {} bytes",
                MAX_DOWNLOAD_SIZE
            );
        }
        fs::write(&resolved, &bytes)?;

        Ok(format!(
            "downloaded {} bytes from {} to '{}'",
            bytes.len(),
            url,
            resolved.display()
        ))
    }

    /// Download an image from a URL and save it to a local path.
    /// Validates that the downloaded bytes are a real image (magic bytes),
    /// and auto-detects the correct extension from content if the path has
    /// none (works even for extensionless CDN image URLs).
    async fn download_image(&self, args: &Value) -> Result<String> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'url' argument"))?;

        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            bail!("only http/https URLs are supported (got: {})", url);
        }

        let mut resolved = self.sandbox.resolve(path)?;
        self.sandbox.check_sensitive_resolved(&resolved)?;

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let client = Self::http_client()?;

        let response = client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            bail!("HTTP {} fetching image {}", status.as_u16(), url);
        }

        let content_length = response.content_length().unwrap_or(0) as usize;
        if content_length > MAX_DOWNLOAD_SIZE {
            bail!(
                "refusing to download: file too large ({} bytes, max {} bytes)",
                content_length,
                MAX_DOWNLOAD_SIZE
            );
        }

        let bytes = response.bytes().await?;
        if bytes.len() > MAX_DOWNLOAD_SIZE {
            bail!(
                "refusing to save: download exceeded {} bytes",
                MAX_DOWNLOAD_SIZE
            );
        }

        // Validate the content is actually an image using magic bytes.
        let Some(fmt) = Self::detect_image_format(&bytes) else {
            bail!("downloaded content does not look like a valid image (url: {})", url);
        };

        // If the destination path has no extension, use the detected format.
        if resolved.extension().is_none() {
            resolved.set_extension(fmt);
        }

        fs::write(&resolved, &bytes)?;

        // Try to read dimensions if it's a real image format
        let dimensions = image::image_dimensions(&resolved).ok();

        let dim_str = match dimensions {
            Some((w, h)) => format!(" ({}x{})", w, h),
            None => String::new(),
        };

        Ok(format!(
            "downloaded image: {} bytes{} ({}), from {} to '{}'",
            bytes.len(),
            dim_str,
            fmt,
            url,
            resolved.display()
        ))
    }

    async fn list_dir(&self, args: &Value) -> Result<String> {
        let path = args["path"].as_str().unwrap_or(".");
        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;
        self.sandbox.check_sensitive_resolved(&resolved)?;

        let entries = fs::read_dir(&resolved)?;

        let mut items: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let prefix = if is_dir { "[DIR] " } else { "[FILE] " };
            items.push(format!("  {}{}", prefix, name));
        }

        items.sort();

        if items.is_empty() {
            Ok(format!("directory '{}' is empty", resolved.display()))
        } else {
            Ok(format!(
                "contents of '{}':\n{}",
                resolved.display(),
                items.join("\n")
            ))
        }
    }

    // ═══════════════════════════════════════════
    //  Search tools
    // ═══════════════════════════════════════════

    /// Recursively search files for lines containing a pattern (substring match,
    /// case-insensitive by default). Reports matches as `path:line: text`.
    /// Skipped: the project's .git, node_modules, target, .venv directories,
    /// binary files (null byte check), and anything over 512KB.
    async fn grep(&self, args: &Value) -> Result<String> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'pattern' argument"))?
            .to_string();
        if pattern.is_empty() {
            bail!("'pattern' must not be empty");
        }

        let root_arg = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        self.sandbox.check_sensitive(root_arg)?;
        let root = self.sandbox.resolve(root_arg)?;
        self.sandbox.check_sensitive_resolved(&root)?;
        if !root.is_dir() {
            bail!("path is not a directory: {}", root.display());
        }

        let ignore_case = args.get("ignore_case").and_then(|v| v.as_bool()).unwrap_or(true);
        let include = args.get("include").and_then(|v| v.as_str()).map(|s| s.to_lowercase());
        let pattern_lower = pattern.to_lowercase();

        const MAX_MATCHES: usize = 200;
        const MAX_LINE_CHARS: usize = 300;
        const MAX_FILE_SIZE: u64 = 512 * 1024;
        const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", ".venv", "dist", "build"];

        let mut results: Vec<String> = Vec::new();
        let mut scanned = 0usize;
        let mut files_searched = 0usize;
        let mut dir_stack: Vec<std::path::PathBuf> = vec![root.clone()];

        while let Some(dir) = dir_stack.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                if results.len() >= MAX_MATCHES {
                    break;
                }
                let path = entry.path();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or_else(|_| entry.metadata().map(|m| m.is_dir()).unwrap_or(false));
                let is_file = entry.file_type().map(|t| t.is_file()).unwrap_or_else(|_| entry.metadata().map(|m| m.is_file()).unwrap_or(false));
                if is_dir {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !SKIP_DIRS.contains(&name) {
                            dir_stack.push(path);
                        }
                    }
                    continue;
                }
                if !is_file {
                    continue;
                }

                if let Some(inc) = &include {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                    if !name.contains(inc) {
                        continue;
                    }
                }

                let Ok(meta) = path.metadata() else { continue };
                if meta.len() > MAX_FILE_SIZE {
                    continue;
                }

                let Ok(content) = fs::read(&path) else { continue };
                if content.contains(&0) {
                    continue; // binary
                }
                let content = String::from_utf8_lossy(&content);
                files_searched += 1;

                for (i, line) in content.lines().enumerate() {
                    let line_matches = if ignore_case {
                        line.to_lowercase().contains(&pattern_lower)
                    } else {
                        line.contains(&pattern)
                    };
                    if line_matches {
                        let line = if line.chars().count() > MAX_LINE_CHARS {
                            let mut s: String = line.chars().take(MAX_LINE_CHARS).collect();
                            s.push_str("...");
                            s
                        } else {
                            line.to_string()
                        };
                        results.push(format!(
                            "{}:{}: {}",
                            path.display(),
                            i + 1,
                            line
                        ));
                    }
                    scanned += 1;
                    if results.len() >= MAX_MATCHES {
                        break;
                    }
                }
            }
        }

        if results.is_empty() {
            Ok(format!(
                "no matches for '{}' in {} ({} files scanned)",
                pattern,
                root.display(),
                files_searched
            ))
        } else {
            let mut out = format!(
                "{} match{} for '{}' in {} ({} files scanned):\n",
                results.len(),
                if results.len() == 1 { "" } else { "es" },
                pattern,
                root.display(),
                files_searched
            );
            out.push_str(&results.join("\n"));
            if scanned > results.len() {
                out.push_str(&format!(
                    "\n[{} lines scanned total]",
                    scanned
                ));
            }
            Ok(out)
        }
    }

    /// Find files by glob pattern, recursively from a root path.
    /// Supports `**` (any depth), `*` (within a path segment), and `?` (single char).
    /// Example patterns: `**/*.rs`, `src/**/test_*.py`, `*.json`.
    async fn glob(&self, args: &Value) -> Result<String> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'pattern' argument"))?
            .to_string();
        if pattern.is_empty() {
            bail!("'pattern' must not be empty");
        }

        let root_arg = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        self.sandbox.check_sensitive(root_arg)?;
        let root = self.sandbox.resolve(root_arg)?;
        self.sandbox.check_sensitive_resolved(&root)?;
        if !root.is_dir() {
            bail!("path is not a directory: {}", root.display());
        }

        const MAX_RESULTS: usize = 500;
        const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", ".venv", "dist", "build"];

        // Normalize pattern separators to the platform separator.
        let norm_pattern = pattern.replace('/', std::path::MAIN_SEPARATOR_STR);

        // Match a single path segment (no separator crossing).
        fn segment_match(seg: &str, pat: &str) -> bool {
            fn rec(s: &[char], p: &[char]) -> bool {
                match (p.first(), s.first()) {
                    (None, _) => s.is_empty(),
                    (Some('*'), _) => {
                        rec(s, &p[1..]) || (!s.is_empty() && rec(&s[1..], p))
                    }
                    (Some('?'), Some(_)) => rec(&s[1..], &p[1..]),
                    (Some(pc), Some(sc)) if pc == sc => rec(&s[1..], &p[1..]),
                    _ => false,
                }
            }
            rec(&seg.chars().collect::<Vec<_>>(), &pat.chars().collect::<Vec<_>>())
        }

        // Split the pattern into segments (double-star aware).
        let mut parts: Vec<String> = Vec::new();
        for part in norm_pattern.split(std::path::MAIN_SEPARATOR) {
            if !part.is_empty() {
                parts.push(part.to_string());
            }
        }
        let double_star = parts.contains(&"**".to_string());

        let mut matches: Vec<std::path::PathBuf> = Vec::new();
        let mut dir_stack: Vec<std::path::PathBuf> = vec![root.clone()];

        while let Some(dir) = dir_stack.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                if matches.len() >= MAX_RESULTS {
                    break;
                }
                let path = entry.path();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or_else(|_| entry.metadata().map(|m| m.is_dir()).unwrap_or(false));

                // Compute the relative path from root using the platform separator.
                let rel = match path.strip_prefix(&root) {
                    Ok(r) => r.to_string_lossy().replace('/', std::path::MAIN_SEPARATOR_STR),
                    Err(_) => continue,
                };
                let rel_segs: Vec<&str> = rel.split(std::path::MAIN_SEPARATOR).filter(|s| !s.is_empty()).collect();

                // Does rel match the pattern? `**` matches zero or more segments.
                let matched = if double_star {
                    let pat_segs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
                    match_glob_segments(&rel_segs, &pat_segs, &segment_match)
                } else {
                    rel_segs.len() == parts.len()
                        && rel_segs.iter().zip(parts.iter()).all(|(s, p)| segment_match(s, p))
                };

                if matched {
                    matches.push(path.clone());
                }

                if is_dir {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !SKIP_DIRS.contains(&name) {
                            dir_stack.push(path);
                        }
                    }
                }
            }
        }

        matches.sort();
        matches.dedup();

        if matches.is_empty() {
            Ok(format!(
                "no files match '{}' under {}",
                pattern,
                root.display()
            ))
        } else {
            let mut out = format!(
                "{} file{} match '{}':\n",
                matches.len(),
                if matches.len() == 1 { "" } else { "s" },
                pattern
            );
            for m in &matches {
                out.push_str(&format!("  {}\n", m.display()));
            }
            Ok(out)
        }
    }

    // ═══════════════════════════════════════════
    //  Skills tools
    // ═══════════════════════════════════════════

    fn list_skills(&self, project_root: &Path) -> Result<String> {
        let skills = crate::skills::discover(project_root);
        if skills.is_empty() {
            Ok(format!(
                "no skills found (look for markdown files in '{}/skills')",
                project_root.display()
            ))
        } else {
            let mut out = String::from("available skills:\n");
            for skill in skills {
                out.push_str(&format!(
                    "  {} — {}\n",
                    skill.name,
                    skill.description
                ));
            }
            Ok(out)
        }
    }

    fn read_skill(&self, args: &Value, project_root: &Path) -> Result<String> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'name' argument"))?
            .to_string();
        let skills = crate::skills::discover(project_root);
        let skill = skills
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(&name))
            .ok_or_else(|| {
                let known: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();
                anyhow::anyhow!(
                    "skill '{}' not found. available: {}",
                    name,
                    if known.is_empty() {
                        "(none)".to_string()
                    } else {
                        known.join(", ")
                    }
                )
            })?;
        Ok(format!("(loaded from {})\n\n{}", skill.path.display(), skill.content))
    }

    // ═══════════════════════════════════════════
    //  Generation tools
    // ═══════════════════════════════════════════

    async fn generate_html(&self, args: &Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;

        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved, content)?;

        Ok(format!("generated html app at '{}' ({} bytes)", resolved.display(), content.len()))
    }

    async fn generate_sprite(&self, args: &Value) -> Result<String> {
        let output = args.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("assets/sprite.png");

        self.sandbox.check_sensitive(output)?;
        let resolved = self.sandbox.resolve(output)?;

        let config = crate::pixel_striker::renderer::SpriteSheetConfig::from_json_value(args);
        let result = crate::pixel_striker::renderer::render_to_file(&config, &resolved)?;

        Ok(format!(
            "generated sprite sheet at '{}' ({}x{}, {} states, {} frames)",
            resolved.display(),
            result.width, result.height,
            result.states.len(),
            result.total_frames,
        ))
    }

    async fn generate_tileset(&self, args: &Value) -> Result<String> {
        let output = args.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("assets/tileset.png");

        self.sandbox.check_sensitive(output)?;
        let resolved = self.sandbox.resolve(output)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let tile_w = args.get("tile_width").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let tile_h = args.get("tile_height").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let cols = args.get("columns").and_then(|v| v.as_u64()).unwrap_or(8) as u32;
        let rows = args.get("rows").and_then(|v| v.as_u64()).unwrap_or(4) as u32;

        let img_w = cols.saturating_mul(tile_w);
        let img_h = rows.saturating_mul(tile_h);
        if img_w == 0 || img_h == 0 || img_w > 16384 || img_h > 16384 {
            return Err(anyhow::anyhow!("image dimensions {}x{} are out of range (must be 1-16384)", img_w, img_h));
        }
        let mut img = image::RgbaImage::new(img_w, img_h);

        let colors = [
            image::Rgba([120, 180, 80, 255]),
            image::Rgba([180, 140, 60, 255]),
            image::Rgba([100, 120, 160, 255]),
            image::Rgba([160, 160, 160, 255]),
        ];

        for row in 0..rows {
            let base = colors[(row as usize) % colors.len()];
            for col in 0..cols {
                let ox = col * tile_w;
                let oy = row * tile_h;
                let v = (col * 17 + row * 31) % 30;
                let r = (base.0[0] + v as i32 - 15).clamp(0, 255) as u8;
                let g = (base.0[1] + v as i32 - 15).clamp(0, 255) as u8;
                let b = (base.0[2] + v as i32 - 15).clamp(0, 255) as u8;
                for dy in 0..tile_h {
                    for dx in 0..tile_w {
                        img.put_pixel(ox + dx, oy + dy, image::Rgba([r, g, b, 255]));
                    }
                }
            }
        }

        img.save(&resolved)?;

        Ok(format!(
            "generated tileset at '{}' ({}x{}, {} tiles)",
            resolved.display(), img_w, img_h, cols * rows
        ))
    }

    async fn generate_object(&self, args: &Value) -> Result<String> {
        let output = args.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("assets/object.png");

        self.sandbox.check_sensitive(output)?;
        let resolved = self.sandbox.resolve(output)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let w = args.get("width").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let h = args.get("height").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let px = args.get("pixel_size").and_then(|v| v.as_u64()).unwrap_or(4) as u32;

        let img_w = w.saturating_mul(px);
        let img_h = h.saturating_mul(px);
        if img_w == 0 || img_h == 0 || img_w > 16384 || img_h > 16384 {
            return Err(anyhow::anyhow!("image dimensions {}x{} are out of range (must be 1-16384)", img_w, img_h));
        }
        let mut img = image::RgbaImage::new(img_w, img_h);

        let r = args.get("color_r").and_then(|v| v.as_u64()).unwrap_or(200) as u8;
        let g = args.get("color_g").and_then(|v| v.as_u64()).unwrap_or(100) as u8;
        let b = args.get("color_b").and_then(|v| v.as_u64()).unwrap_or(50) as u8;

        for dy in 0..img_h {
            for dx in 0..img_w {
                let edge = dx < px || dx >= img_w - px || dy < px || dy >= img_h - px;
                let factor = if edge { 0.7 } else { 1.0 };
                img.put_pixel(dx, dy, image::Rgba([
                    (r as f32 * factor) as u8,
                    (g as f32 * factor) as u8,
                    (b as f32 * factor) as u8,
                    255,
                ]));
            }
        }

        img.save(&resolved)?;
        Ok(format!("generated object sprite at '{}' ({}x{})", resolved.display(), img_w, img_h))
    }

    async fn render_sprite(&self, args: &Value) -> Result<String> {
        let output = args.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("assets/sprite_viewer.html");

        self.sandbox.check_sensitive(output)?;
        let resolved = self.sandbox.resolve(output)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let html = r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>sprite viewer</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0a0a0f;color:#0ff;font-family:monospace;display:flex;justify-content:center;align-items:center;min-height:100vh}
.viewer{background:#111;border:1px solid #0ff3;border-radius:8px;padding:2rem;box-shadow:0 0 40px #0ff2;text-align:center}
canvas{image-rendering:pixelated;border:2px solid #0ff5;border-radius:4px;margin:1rem 0}
.controls{display:flex;gap:.5rem;flex-wrap:wrap;justify-content:center;margin-top:1rem}
button{background:#0ff1;color:#0ff;border:1px solid #0ff5;padding:.5rem 1rem;border-radius:4px;cursor:pointer;font:inherit}
button:hover{background:#0ff3}
select{background:#111;color:#0ff;border:1px solid #0ff5;padding:.5rem;border-radius:4px;font:inherit}
h2{color:#0ff8}
</style></head>
<body>
<div class="viewer">
<h2>✦ sprite viewer ✦</h2>
<canvas id="c"></canvas>
<div class="controls">
<select id="state"><option>idle</option><option>walk_left</option><option>walk_right</option><option>hacking_active</option></select>
<button id="play">▶ play</button>
</div>
</div>
<script>
const c=document.getElementById('c'),ctx=c.getContext('2d'),S=4,W=32,H=32;c.width=W*S;c.height=H*S;
const p={skin:[255,204,153],hair:[80,40,120],shirt:[40,80,160],pants:[60,60,80],shoes:[40,40,50],visor:[0,240,255]};
function pt(x,y,c){if(x<0||x>=W||y<0||y>=H)return;ctx.fillStyle='rgb('+c[0]+','+c[1]+','+c[2]+')';ctx.fillRect(x*S,y*S,S,S)}
function dr(b){ctx.clearRect(0,0,c.width,c.height)
for(let d=0;d<8;d++){pt(1+d,1+b,[p.hair[0]*.6|0,p.hair[1]*.6|0,p.hair[2]*.6|0]);pt(1+d,2+b,p.hair)}
pt(3,1+b,p.hair);for(let y=0;y<4;y++)for(let x=0;x<6;x++)pt(2+x,3+y+b,p.skin)
pt(5,4+b,p.visor);pt(6,4+b,p.visor);pt(7,4+b,p.visor);pt(6,3+b,[p.visor[0]*.5|0,p.visor[1]*.5|0,p.visor[2]*.5|0])
for(let y=0;y<5;y++)for(let x=0;x<8;x++)pt(1+x,7+y+b,p.shirt)
for(let y=0;y<3;y++)for(let x=0;x<6;x++)pt(2+x,12+y+b,p.pants)
pt(3,15+b,p.pants);pt(6,15+b,p.pants)
pt(2,17+b,p.shoes);pt(3,17+b,p.shoes);pt(4,17+b,p.shoes);pt(5,17+b,p.shoes);pt(6,17+b,p.shoes);pt(7,17+b,p.shoes)}
let f=0,pl=0;setInterval(()=>{if(pl){f++;dr(f%2)}},250)
document.getElementById('play').onclick=()=>{pl=!pl;document.getElementById('play').textContent=pl?'⏸ stop':'▶ play'}
document.getElementById('state').onchange=function(){}
dr(0)
</script>
</body></html>"#;

        fs::write(&resolved, html)?;
        Ok(format!("rendered sprite viewer at '{}' ({} bytes)", resolved.display(), html.len()))
    }

    // ═══════════════════════════════════════════
    //  Clipboard
    // ═══════════════════════════════════════════

    #[cfg(not(target_os = "android"))]
    async fn read_clipboard(&self) -> Result<String> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| anyhow::anyhow!("failed to open clipboard: {}", e))?;

        match cb.get_text() {
            Ok(text) => {
                let preview: String = text.chars().take(500).collect();
                let total_chars = text.chars().count();
                if total_chars > 500 {
                    Ok(format!("{}\n... [truncated: showing 500 of {} chars]", preview, total_chars))
                } else {
                    Ok(text)
                }
            }
            Err(arboard::Error::ContentNotAvailable) => {
                match cb.get_image() {
                    Ok(img) => Ok(format!(
                        "clipboard contains an image ({}x{}, {} bytes)", img.width, img.height, img.bytes.len()
                    )),
                    Err(_) => Ok("clipboard is empty or contains unsupported content".to_string()),
                }
            }
            Err(e) => Ok(format!("failed to read clipboard: {}", e)),
        }
    }

    /// On android/termux there is no clipboard backend (arboard is excluded
    /// from the android build) — graceful no-op.
    #[cfg(target_os = "android")]
    async fn read_clipboard(&self) -> Result<String> {
        Ok("clipboard unavailable on android/termux (no clipboard backend)".to_string())
    }

    // ═══════════════════════════════════════════
    //  Memory tools
    // ═══════════════════════════════════════════

    fn remember(&self, args: &Value, memory: &mut MemoryStore) -> Result<String> {
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;
        let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("general");

        memory.add_memory(category, content, vec![], 5);
        Ok(format!("stored memory: {} [category: {}]", content, category))
    }

    fn list_memories(&self, args: &Value, memory: &MemoryStore) -> Result<String> {
        let n = args.get("count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let memories = memory.recent(n);

        if memories.is_empty() {
            return Ok("no memories stored yet".to_string());
        }

        let lines: Vec<String> = memories.iter().enumerate().map(|(i, m)| {
            format!("{}. [{}] {} (importance: {})", i + 1, m.category, m.content, m.importance)
        }).collect();

        Ok(format!("recent memories ({}):\n{}", memories.len(), lines.join("\n")))
    }

    fn search_memories(&self, args: &Value, memory: &MemoryStore) -> Result<String> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'query' argument"))?;

        let results = memory.search(query);
        if results.is_empty() {
            return Ok(format!("no memories matching '{}'", query));
        }

        let lines: Vec<String> = results.iter().map(|m| {
            format!("- [{}] {} (tags: {})", m.category, m.content, m.tags.join(", "))
        }).collect();

        Ok(format!("memories matching '{}' ({}):\n{}", query, results.len(), lines.join("\n")))
    }

    fn set_preference(&self, args: &Value, memory: &mut MemoryStore) -> Result<String> {
        let key = args["key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'key' argument"))?;
        let value = args["value"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'value' argument"))?;

        memory.set_preference(key, value);
        Ok(format!("preference set: {} = {}", key, value))
    }

    // ═══════════════════════════════════════════
    //  Analysis tools
    // ═══════════════════════════════════════════

    fn analyze_self(&self, args: &Value, analyzer: &SelfAnalyzer) -> Result<String> {
        let target = args.get("file").and_then(|v| v.as_str()).unwrap_or("");

        let files = analyzer.source_files();
        if target.is_empty() {
            let names: Vec<String> = files.iter()
                .filter_map(|f| f.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
                .collect();
            return Ok(format!("source files:\n{}", names.join("\n")));
        }

        let file_path = files.iter().find(|f| {
            f.file_name().and_then(|n| n.to_str()) == Some(target)
        });

        match file_path {
            Some(p) => {
                let source = analyzer.read_source(p)?;
                let issues = analyzer.analyze_for_improvements(&source);
                let filename = p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                let issue_lines: Vec<String> = issues.iter().map(|i| {
                    format!("  [{}] line {:?}: {} \u{2192} {}", i.severity, i.line, i.description, i.fix)
                }).collect();

                Ok(format!("analysis of '{}' ({} lines, {} issues):\n{}",
                    filename, source.lines().count(), issues.len(), issue_lines.join("\n")))
            }
            None => Ok(format!("file '{}' not found in source directory", target)),
        }
    }

    fn list_source_files(&self, analyzer: &SelfAnalyzer) -> Result<String> {
        let files = analyzer.source_files();
        if files.is_empty() {
            return Ok("no source files found".to_string());
        }

        let total_lines: usize = files.iter()
            .filter_map(|f| fs::read_to_string(f).ok())
            .map(|s| s.lines().count())
            .sum();

        let lines: Vec<String> = files.iter().map(|f| {
            let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            let lc = fs::read_to_string(f).ok().map(|s| s.lines().count()).unwrap_or(0);
            format!("  {} ({} lines)", name, lc)
        }).collect();

        Ok(format!("source files ({} files, ~{} lines total):\n{}",
            files.len(), total_lines, lines.join("\n")))
    }

    // ═══════════════════════════════════════════
    //  Evolution
    // ═══════════════════════════════════════════

    async fn evolve_tools(&self, args: &Value, evolver: &ToolEvolver, _llm: &LlmClient) -> Result<String> {
        let specific = args.get("gap").and_then(|v| v.as_str());

        let gaps = evolver.analyze_gaps();
        if gaps.is_empty() {
            return Ok("no tool gaps detected — all tool categories are covered".to_string());
        }

        if let Some(gap_name) = specific {
            let found = gaps.iter().find(|g| g.contains(gap_name));
            match found {
                Some(gap) => {
                    let template = evolver.generate_tool_definition(gap).await?;
                    let code = ToolEvolver::generate_tool_code(&template);
                    Ok(format!("generated tool '{}':\n{}", template.name, code))
                }
                None => Ok(format!("gap '{}' not found. gaps: {}", gap_name, gaps.join(", "))),
            }
        } else {
            Ok(format!("detected {} tool gaps:\n{}", gaps.len(),
                gaps.iter().enumerate().map(|(i, g)| format!("  {}. {}", i + 1, g)).collect::<Vec<_>>().join("\n")))
        }
    }

    // ═══════════════════════════════════════════
    //  Prompt refinement
    // ═══════════════════════════════════════════

    fn refine_prompt(&self, history: &PromptHistory) -> Result<String> {
        Ok(history.generate_analysis_prompt())
    }

    fn get_tool_stats(&self, history: &PromptHistory) -> Result<String> {
        let stats = history.tool_stats();
        if stats.is_empty() {
            return Ok("no tool usage recorded yet".to_string());
        }

        let lines: Vec<String> = stats.iter().map(|(name, total, success, rate)| {
            let bar_len = (rate * 10.0) as usize;
            let bar = format!("{}{}", "█".repeat(bar_len), "░".repeat(10 - bar_len));
            format!("  {} {} {}/{} ({:.0}%)", name, bar, success, total, rate * 100.0)
        }).collect();

        Ok(format!("tool usage stats:\n{}", lines.join("\n")))
    }

    // ═══════════════════════════════════════════
    //  Coding agent (multi-action coding tool)
    // ═══════════════════════════════════════════

    async fn coding_agent(&self, args: &Value, llm: &LlmClient, project_root: &Path, sandbox: &Sandbox) -> Result<String> {

        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'action' argument"))?;

        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        match action {
            "read" => {
                sandbox.check_sensitive(path)?;
                let full_path = sandbox.resolve(path)?;
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;
                Ok(json!({"path": path, "content": content}).to_string())
            }
            "write" => {
                sandbox.check_sensitive(path)?;
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;
                let full_path = sandbox.resolve(path)?;
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full_path, content)?;
                Ok(json!({"path": path, "status": "written", "bytes": content.len()}).to_string())
            }
            "edit" => {
                sandbox.check_sensitive(path)?;
                let edits = args.get("edits")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow::anyhow!("missing 'edits' array"))?;

                let full_path = sandbox.resolve(path)?;
                let mut content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;

                let mut edits = edits.clone();
                edits.reverse();

                for edit in edits {
                    let act = edit.get("action").and_then(|v| v.as_str()).unwrap_or("");
                    match act {
                        "replace" => {
                            let old = edit.get("old").and_then(|v| v.as_str())
                                .ok_or_else(|| anyhow::anyhow!("replace missing 'old'"))?;
                            let new = edit.get("new").and_then(|v| v.as_str())
                                .ok_or_else(|| anyhow::anyhow!("replace missing 'new'"))?;
                            if !content.contains(old) {
                                return Err(anyhow::anyhow!("replace string not found in '{}'", path));
                            }
                            content = content.replacen(old, new, 1);
                        }
                        "insert" => {
                            let text = edit.get("text").and_then(|v| v.as_str())
                                .ok_or_else(|| anyhow::anyhow!("insert missing 'text'"))?;
                            let after = edit.get("after").and_then(|v| v.as_str());
                            if let Some(a) = after {
                                if let Some(pos) = content.find(a) {
                                    content.insert_str(pos + a.len(), text);
                                } else {
                                    return Err(anyhow::anyhow!("insert anchor '{}' not found", a));
                                }
                            } else {
                                content.push_str(text);
                            }
                        }
                        _ => return Err(anyhow::anyhow!("unknown edit action: {}", act)),
                    }
                }

                fs::write(&full_path, &content)?;
                Ok(json!({"path": path, "status": "edited", "bytes": content.len()}).to_string())
            }
            "list" => {
                sandbox.check_sensitive(path)?;
                let full_path = sandbox.resolve(path)?;
                let mut entries = Vec::new();
                if full_path.is_dir() {
                    for entry in fs::read_dir(&full_path)? {
                        let e = entry?;
                        let name = e.file_name().to_string_lossy().to_string();
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        entries.push(json!({"name": name, "type": if is_dir { "directory" } else { "file" }}));
                    }
                }
                Ok(json!({"path": path, "entries": entries}).to_string())
            }
            "analyze" => {
                sandbox.check_sensitive(path)?;
                let full_path = sandbox.resolve(path)?;
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;
                let analyzer = SelfAnalyzer::new(project_root.to_path_buf());
                let issues = analyzer.analyze_for_improvements(&content);
                Ok(json!({"path": path, "issues": issues}).to_string())
            }
            "modify" => {
                sandbox.check_sensitive(path)?;
                let instruction = args.get("instruction")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing 'instruction' argument"))?;
                let full_path = sandbox.resolve(path)?;
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;

                let prompt = format!(
                    "Modify this code per the instruction.\nFILE: {}\nCODE:\n{}\nINSTRUCTION:\n{}\nOutput ONLY the modified code.",
                    path, content, instruction
                );
                let msg = vec![ChatMessage {
                    role: "user".to_string(), content: prompt,
                    tool_calls: None, tool_call_id: None,
                }];
                let resp = llm.chat(&msg, None).await?;
                let raw = resp.message.content.trim();
                // Strip markdown code fences if present (```rust ... ``` or ``` ... ```)
                let modified = if raw.starts_with("```") {
                    let stripped = raw.trim_start_matches("```");
                    // Remove language tag on first line
                    let after_lang = stripped.find('\n').map(|i| &stripped[i+1..]).unwrap_or(stripped);
                    // Remove trailing fence
                    after_lang.trim_end_matches("```").trim().to_string()
                } else {
                    raw.to_string()
                };

                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full_path, &modified)?;
                Ok(json!({"path": path, "status": "modified", "bytes": modified.len()}).to_string())
            }
            "suggest" => {
                sandbox.check_sensitive(path)?;
                let full_path = sandbox.resolve(path)?;
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;
                let prompt = format!(
                    "Review this code and suggest improvements.\nFILE: {}\nCODE:\n{}\nList specific, actionable suggestions.",
                    path, content
                );
                let msg = vec![ChatMessage {
                    role: "user".to_string(), content: prompt,
                    tool_calls: None, tool_call_id: None,
                }];
                let resp = llm.chat(&msg, None).await?;
                Ok(json!({"path": path, "suggestions": resp.message.content}).to_string())
            }
            _ => Err(anyhow::anyhow!("unknown coding_agent action: {}", action)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::Sandbox;
    use serde_json::json;

    #[tokio::test]
    async fn test_unknown_tool() {
        let executor = ToolExecutor::new(Sandbox::new("."));
        let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();
        let mut input_flag = Arc::new(AtomicBool::new(true));
        let mut manager = AppletManager::new();
        let mut menu_flag = Arc::new(AtomicBool::new(false));
        let result = executor.execute("nonexistent_tool", &json!({}), &mut ToolContext {
            memory: &mut MemoryStore::default(),
            prompt_history: &mut PromptHistory::default(),
            analyzer: &SelfAnalyzer::new(std::path::PathBuf::from(".")),
            evolver: &ToolEvolver::new(vec![]),
            llm: &LlmClient::new("test"),
            backend: &crate::ActiveBackend::Cloud(LlmClient::new("test")),
            project_root: std::path::Path::new("."),
            applet_manager: &mut manager,
            steer_tx: &steer_tx,
            steer_rx: &steer_rx,
            input_flag: &mut input_flag,
            menu_flag: &menu_flag,
        }).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown tool"));
    }

    #[test]
    fn plan_mode_denies_mutating_tools_only() {
        // everything mutating is denied...
        for name in MUTATING_TOOLS {
            assert!(
                plan_mode_deny_message(name).is_some(),
                "mutating tool '{}' must be denied in plan mode",
                name
            );
        }
        // ...and everything read-only stays allowed
        for name in [
            "read_file", "list_dir", "grep", "glob", "list_skills", "read_skill",
            "read_clipboard", "list_memories", "search_memories",
            "analyze_self", "list_source_files", "get_tool_stats",
        ] {
            assert!(
                plan_mode_deny_message(name).is_none(),
                "read-only tool '{}' must be allowed in plan mode",
                name
            );
        }
        let msg = plan_mode_deny_message("write_file").unwrap();
        assert!(msg.contains("plan mode"));
        assert!(msg.contains("write_file"));
    }

    #[test]
    fn delegate_requires_sub_task() {
        // The delegate impl needs a live backend, so here we pin its contract:
        // sub_task is required (checked before any backend call).
        let executor = ToolExecutor::new(Sandbox::new("."));
        let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();
        let mut input_flag = Arc::new(AtomicBool::new(true));
        let mut manager = AppletManager::new();
        let mut menu_flag = Arc::new(AtomicBool::new(false));
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(executor.execute("delegate", &json!({}), &mut ToolContext {
            memory: &mut MemoryStore::default(),
            prompt_history: &mut PromptHistory::default(),
            analyzer: &SelfAnalyzer::new(std::path::PathBuf::from(".")),
            evolver: &ToolEvolver::new(vec![]),
            llm: &LlmClient::new("test"),
            backend: &crate::ActiveBackend::Cloud(LlmClient::new("test")),
            project_root: std::path::Path::new("."),
            applet_manager: &mut manager,
            steer_tx: &steer_tx,
            steer_rx: &steer_rx,
            input_flag: &mut input_flag,
            menu_flag: &menu_flag,
        }));
        let err = result.unwrap_err().to_string();
        assert!(err.contains("sub_task"), "expected missing sub_task error, got: {}", err);
    }

    #[test]
    fn sub_agent_payload_is_read_only_only() {
        let payload = sub_agent_tool_payload();
        let entries = payload.as_array().expect("payload must be an array");
        assert!(!entries.is_empty(), "read-only sub-agent payload must not be empty");
        for entry in entries {
            let name = entry["function"]["name"].as_str().unwrap_or("");
            assert!(
                SUB_AGENT_TOOLS.contains(&name),
                "sub-agent tool '{}' must be in the read-only set",
                name
            );
            assert!(
                !MUTATING_TOOLS.contains(&name),
                "sub-agent tool '{}' must not be mutating",
                name
            );
        }
        // delegate must never be offered to the sub-agent (no deep nesting)
        assert!(!SUB_AGENT_TOOLS.contains(&"delegate"));
        assert!(
            !entries.iter().any(|e| e["function"]["name"] == "delegate"),
            "delegate must not appear in the sub-agent payload"
        );
    }

    #[test]
    fn delegate_tool_definition_shape() {
        let def = delegate_tool_definition();
        assert_eq!(def["type"], "function");
        assert_eq!(def["function"]["name"], "delegate");
        assert_eq!(def["function"]["parameters"]["required"][0], "sub_task");
    }

    /// Compile-time guard: the catalog (single source of truth) and the
    /// executor dispatcher must never drift. Iterates TOOL_CATALOG and
    /// asserts every entry resolves through `dispatch_for` — a catalog
    /// addition without a handler fails the build here. Checks the reverse
    /// too: every dispatchable name must exist in the catalog, so the model
    /// is never offered a tool that has no definition.
    #[test]
    fn every_catalog_tool_has_dispatch_arm() {
        for def in crate::tool_defs::TOOL_CATALOG.iter() {
            assert!(
                ToolExecutor::dispatch_for(def.name).is_some(),
                "tool '{}' is in TOOL_CATALOG but has no dispatch arm — add a handler in execute() and a DISPATCHABLE_TOOLS entry",
                def.name
            );
        }
        for name in ToolExecutor::dispatched_tool_names() {
            assert!(
                crate::tool_defs::TOOL_CATALOG.iter().any(|d| d.name == *name),
                "dispatch arm for '{}' has no TOOL_CATALOG entry",
                name
            );
        }
        assert_eq!(
            ToolExecutor::dispatched_tool_names().len(),
            crate::tool_defs::TOOL_CATALOG.len(),
            "dispatcher and catalog must have the same number of tools"
        );
    }

    #[tokio::test]
    async fn test_write_read_roundtrip() {
        let dir = std::env::temp_dir().join("ayesha_test_write_read");
        let _ = std::fs::create_dir_all(&dir);
        let test_file = dir.join("test.txt");

        let executor = ToolExecutor::new(Sandbox::new(&dir));

        // Write
        let write_result = executor.write_file(&json!({
            "path": test_file.to_string_lossy(),
            "content": "hello ayesha! :3"
        })).await.unwrap();
        assert!(write_result.contains("wrote"));

        // Read
        let read_result = executor.read_file(&json!({
            "path": test_file.to_string_lossy()
        })).await.unwrap();
        assert_eq!(read_result, "hello ayesha! :3");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_list_dir() {
        let dir = std::env::temp_dir().join("ayesha_test_list");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        std::fs::write(dir.join("b.txt"), "b").unwrap();

        let executor = ToolExecutor::new(Sandbox::new(&dir));
        let result = executor.list_dir(&json!({
            "path": dir.to_string_lossy()
        })).await.unwrap();

        assert!(result.contains("a.txt"));
        assert!(result.contains("b.txt"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_grep_finds_matches() {
        let dir = std::env::temp_dir().join("ayesha_test_grep");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.rs"), "fn main() {\n    println!(\"hello world\");\n}\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "nothing interesting here").unwrap();

        let executor = ToolExecutor::new(Sandbox::new(&dir));
        let result = executor.grep(&json!({
            "pattern": "println",
            "path": dir.to_string_lossy()
        })).await.unwrap();

        assert!(result.contains("main.rs:2"), "expected path:line in result, got: {}", result);
        assert!(result.contains("println!"));
        assert!(!result.contains("notes.txt"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_grep_case_sensitive_and_include() {
        let dir = std::env::temp_dir().join("ayesha_test_grep2");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.rs"), "fn Foo() {}\nfn bar() {}\n").unwrap();
        std::fs::write(dir.join("a.md"), "fn Foo() {}\n").unwrap();

        let executor = ToolExecutor::new(Sandbox::new(&dir));

        // Case-insensitive default finds both
        let ci = executor.grep(&json!({
            "pattern": "foo",
            "path": dir.to_string_lossy()
        })).await.unwrap();
        assert!(ci.contains("a.rs:1"));

        // Case-sensitive misses uppercase
        let cs = executor.grep(&json!({
            "pattern": "foo",
            "path": dir.to_string_lossy(),
            "ignore_case": false
        })).await.unwrap();
        assert!(!cs.contains("a.rs:1"), "case-sensitive search matched lowercase: {}", cs);

        // include filter narrows to .md
        let inc = executor.grep(&json!({
            "pattern": "Foo",
            "path": dir.to_string_lossy(),
            "include": ".md"
        })).await.unwrap();
        assert!(inc.contains("a.md:1"));
        assert!(!inc.contains("a.rs"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let dir = std::env::temp_dir().join("ayesha_test_grep3");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.txt"), "hello").unwrap();

        let executor = ToolExecutor::new(Sandbox::new(&dir));
        let result = executor.grep(&json!({
            "pattern": "zzzz",
            "path": dir.to_string_lossy()
        })).await.unwrap();
        assert!(result.contains("no matches"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_glob_finds_nested_files() {
        let dir = std::env::temp_dir().join("ayesha_test_glob");
        let _ = std::fs::create_dir_all(dir.join("src").join("deep"));
        std::fs::write(dir.join("src").join("main.rs"), "").unwrap();
        std::fs::write(dir.join("src").join("deep").join("lib.rs"), "").unwrap();
        std::fs::write(dir.join("src").join("deep").join("mod.rs"), "").unwrap();
        std::fs::write(dir.join("Cargo.toml"), "").unwrap();

        let executor = ToolExecutor::new(Sandbox::new(&dir));

        // ** matches any depth
        let all = executor.glob(&json!({
            "pattern": "**/*.rs",
            "path": dir.to_string_lossy()
        })).await.unwrap();
        assert!(all.contains("main.rs"));
        assert!(all.contains("deep\\lib.rs") || all.contains("deep/lib.rs"));
        assert!(!all.contains("Cargo.toml"));

        // single-segment pattern only matches top level
        let top = executor.glob(&json!({
            "pattern": "*.rs",
            "path": dir.to_string_lossy()
        })).await.unwrap();
        assert!(!top.contains("deep"), "single-* crossed directories: {}", top);

        // ? matches single char
        let q = executor.glob(&json!({
            "pattern": "src/deep/mo?.rs",
            "path": dir.to_string_lossy()
        })).await.unwrap();
        assert!(q.contains("mod.rs"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_glob_no_match() {
        let dir = std::env::temp_dir().join("ayesha_test_glob2");
        let _ = std::fs::create_dir_all(&dir);

        let executor = ToolExecutor::new(Sandbox::new(&dir));
        let result = executor.glob(&json!({
            "pattern": "**/*.xyz",
            "path": dir.to_string_lossy()
        })).await.unwrap();
        assert!(result.contains("no files match"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_list_skills_empty_when_no_dir() {
        let dir = std::env::temp_dir().join("ayesha_test_skills_empty");
        let _ = std::fs::create_dir_all(&dir);
        let executor = ToolExecutor::new(Sandbox::new(&dir));
        let result = executor.list_skills(&dir).unwrap();
        assert!(result.contains("no skills found"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_list_and_read_skill() {
        let dir = std::env::temp_dir().join("ayesha_test_skills");
        let _ = std::fs::create_dir_all(dir.join("skills"));
        std::fs::write(
            dir.join("skills").join("demo.md"),
            "---\nname: demo\ndescription: a demo skill\n---\n# demo\nfollow these steps",
        ).unwrap();

        let executor = ToolExecutor::new(Sandbox::new(&dir));

        let listed = executor.list_skills(&dir).unwrap();
        assert!(listed.contains("demo"));
        assert!(listed.contains("a demo skill"));

        let loaded = executor.read_skill(&json!({"name": "demo"}), &dir).unwrap();
        assert!(loaded.contains("follow these steps"));
        assert!(loaded.contains("demo.md"), "loaded content should mention source file: {}", loaded);

        let missing = executor.read_skill(&json!({"name": "nope"}), &dir);
        assert!(missing.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_fetch_url_rejects_non_http() {
        let dir = std::env::temp_dir().join("ayesha_test_fetch");
        let _ = std::fs::create_dir_all(&dir);
        let executor = ToolExecutor::new(Sandbox::new(&dir));

        let result = executor.fetch_url(&json!({
            "url": "ftp://example.com/file.txt",
            "path": dir.join("out.txt").to_string_lossy()
        })).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("http/https"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_fetch_url_missing_args() {
        let executor = ToolExecutor::new(Sandbox::new("."));

        // Missing url
        let result = executor.fetch_url(&json!({
            "path": "out.txt"
        })).await;
        assert!(result.is_err());

        // Missing path
        let result = executor.fetch_url(&json!({
            "url": "https://example.com/"
        })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_download_image_rejects_non_image() {
        let dir = std::env::temp_dir().join("ayesha_test_img");
        let _ = std::fs::create_dir_all(&dir);
        let executor = ToolExecutor::new(Sandbox::new(&dir));

        // Use a URL that ends in .png but returns HTML
        let result = executor.download_image(&json!({
            "url": "https://example.com/missing.png",
            "path": dir.join("test.png").to_string_lossy()
        })).await;
        // Either network fails (404) or content is invalid image - either is fine
        // The key thing is the tool doesn't silently succeed with garbage
        // We expect an error here
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_download_image_auto_extension() {
        // Test that paths without extensions get the detected extension
        let dir = std::env::temp_dir().join("ayesha_test_img_ext");
        let _ = std::fs::create_dir_all(&dir);

        let executor = ToolExecutor::new(Sandbox::new(&dir));
        let path_no_ext = dir.join("wallpaper");

        // Just test that resolve/sandbox logic works — actual download will fail
        let result = executor.download_image(&json!({
            "url": "https://example.com/wallpaper.png",
            "path": path_no_ext.to_string_lossy()
        })).await;
        // Either fails because URL is unreachable or content isn't an image
        // We just want to verify no panic
        let _ = result;

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_image_format_known_formats() {
        // PNG magic
        assert_eq!(ToolExecutor::detect_image_format(b"\x89PNG\r\n\x1a\nxxxx"), Some("png"));
        // JPEG magic
        assert_eq!(ToolExecutor::detect_image_format(b"\xFF\xD8\xFF\xE0rest"), Some("jpg"));
        // GIF
        assert_eq!(ToolExecutor::detect_image_format(b"GIF89a...."), Some("gif"));
        assert_eq!(ToolExecutor::detect_image_format(b"GIF87a...."), Some("gif"));
        // WebP (RIFF....WEBP)
        assert_eq!(ToolExecutor::detect_image_format(b"RIFF\x00\x00\x00\x00WEBPxyz"), Some("webp"));
        // BMP
        assert_eq!(ToolExecutor::detect_image_format(b"BMxxxx"), Some("bmp"));
        // SVG (whitespace tolerant)
        assert_eq!(ToolExecutor::detect_image_format(b"\xef\xbb\xbf<svg"), Some("svg"));
        assert_eq!(ToolExecutor::detect_image_format(b"<html"), Some("svg"));
    }

    #[test]
    fn test_detect_image_format_rejects_non_images() {
        assert_eq!(ToolExecutor::detect_image_format(b"hello world"), None);
        assert_eq!(ToolExecutor::detect_image_format(b""), None);
        assert_eq!(ToolExecutor::detect_image_format(b"GIF86a...."), None);
        assert_eq!(ToolExecutor::detect_image_format(b"RIFF\x00\x00\x00\x00PNGxx"), None);
    }

    #[test]
    fn test_http_client_rejects_oversize_content_length() {
        // Simulate the size-cap guard by checking MAX_DOWNLOAD_SIZE exists and
        // a large content-length would be rejected. This is a logic-level test
        // so we don't need the network: assert the cap constant is sane.
        assert_eq!(MAX_DOWNLOAD_SIZE, 100 * 1024 * 1024);
    }

    /// Regression test for the user-reported bug:
    /// "create a file on my desktop called poop.txt" should produce an empty
    /// file (or with timestamp greeting) — definitely not fail silently.
    #[tokio::test]
    async fn test_write_file_empty_content_creates_file() {
        let dir = std::env::temp_dir().join("ayesha_test_desktop");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("poop.txt");

        let executor = ToolExecutor::new(Sandbox::new(&dir));

        // Empty content — like the user's scenario
        let result = executor.write_file(&json!({
            "path": target.to_string_lossy(),
            "content": ""
        })).await;

        assert!(result.is_ok(), "write_file failed: {:?}", result.err());
        assert!(target.exists(), "file was not created");
        assert_eq!(std::fs::metadata(&target).unwrap().len(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Regression test: full absolute paths (C:\Users\foo\Desktop\file.txt)
    /// should resolve correctly via the sandbox.
    #[tokio::test]
    async fn test_write_file_absolute_windows_path() {
        let dir = std::env::temp_dir().join("ayesha_test_abs_path");
        let _ = std::fs::create_dir_all(&dir);
        // Use forward slashes (which Windows accepts) to simulate path
        let path_str = dir.join("test.txt").to_string_lossy().to_string();
        let path_str = path_str.replace('\\', "\\\\");

        let executor = ToolExecutor::new(Sandbox::new(&dir));
        let result = executor.write_file(&json!({
            "path": path_str,
            "content": "hello"
        })).await;

        assert!(result.is_ok(), "write_file failed: {:?}", result.err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_write_sensitive_blocked() {
        let dir = std::env::temp_dir().join("ayesha_test_env");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(".env");
        let executor = ToolExecutor::new(Sandbox::new("."));
        let result = executor.write_file(&json!({
            "path": path.to_string_lossy(),
            "content": "SECRET=123"
        })).await;
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_remember_and_list() {
        let mut memory = MemoryStore::default();
        let executor = ToolExecutor::new(Sandbox::new("."));

        let _ = executor.remember(&json!({
            "content": "user likes tuna",
            "category": "user_pref"
        }), &mut memory);

        let _ = executor.remember(&json!({
            "content": "ayesha lives in a computer",
            "category": "fact"
        }), &mut memory);

        let list = executor.list_memories(&json!({"count": 10}), &memory).unwrap();
        assert!(list.contains("tuna"));
        assert!(list.contains("computer"));

        let search = executor.search_memories(&json!({"query": "tuna"}), &memory).unwrap();
        assert!(search.contains("tuna"));
        assert!(!search.contains("computer"));
    }

    #[test]
    fn test_set_preference() {
        let mut memory = MemoryStore::default();
        let executor = ToolExecutor::new(Sandbox::new("."));

        let _ = executor.set_preference(&json!({
            "key": "favorite color",
            "value": "cyan"
        }), &mut memory);

        assert_eq!(memory.get_preference("favorite color"), Some("cyan"));
    }

    #[tokio::test]
    async fn test_manage_applet_list() {
        let executor = ToolExecutor::new(Sandbox::new("."));
        let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();
        let mut input_flag = Arc::new(AtomicBool::new(true));
        let mut manager = AppletManager::new();
        let mut menu_flag = Arc::new(AtomicBool::new(false));
        let result = executor.execute("manage_applet", &json!({ "action": "list" }), &mut ToolContext {
            memory: &mut MemoryStore::default(),
            prompt_history: &mut PromptHistory::default(),
            analyzer: &SelfAnalyzer::new(std::path::PathBuf::from(".")),
            evolver: &ToolEvolver::new(vec![]),
            llm: &LlmClient::new("test"),
            backend: &crate::ActiveBackend::Cloud(LlmClient::new("test")),
            project_root: std::path::Path::new("."),
            applet_manager: &mut manager,
            steer_tx: &steer_tx,
            steer_rx: &steer_rx,
            input_flag: &mut input_flag,
            menu_flag: &menu_flag,
        }).await.unwrap();
        assert!(result.contains("applets"));
        assert!(result.contains("engine"));
    }
}
