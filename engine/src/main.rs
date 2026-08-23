mod llm;
mod permissions;
mod cloud;
mod streaming;
mod tools;
mod tool_defs;
mod skills;
mod sandbox;
mod ui;
mod util;
mod format;
mod pixel_striker;
mod memory;
mod self_analysis;
mod tool_evolution;
mod prompt_refinement;
mod coding_agent;
mod model_registry;
mod applet_manager;
mod applet_runner;
mod plugins;
mod agent;
mod completion;
mod theme;
mod syntax;
mod render;
mod session;
mod vision;

use std::io::Write;
use theme::Role;
use llm::{LlmClient, ChatMessage, StreamResult};
use cloud::CloudClient;
use streaming::StreamDecoder;
use tools::{ToolExecutor, ToolContext};
use sandbox::Sandbox;
use prompt_refinement::PromptHistory;
use memory::MemoryStore;
use self_analysis::SelfAnalyzer;
use tool_evolution::ToolEvolver;
use model_registry::ModelRegistry;
use applet_manager::AppletManager;
use permissions::Verdict;


/// Active backend — either local LLM or cloud (kilo gateway)
enum ActiveBackend {
    Local(LlmClient),
    Cloud(CloudClient),
}

/// Agent operation mode. `auto` lets the agent decide; `plan` is read-only
/// research (mutating tools are denied); `build` is the full toolset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Plan,
    Build,
    Auto,
}

impl Mode {
    fn as_str(&self) -> &'static str {
        match self {
            Mode::Plan => "plan",
            Mode::Build => "build",
            Mode::Auto => "auto",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "plan" => Some(Mode::Plan),
            "build" => Some(Mode::Build),
            "auto" => Some(Mode::Auto),
            _ => None,
        }
    }
}

/// Build the system prompt for a user, appending the skills hint if any
/// skills are installed in the project's skills/ folder.
fn build_system_prompt(user_name: &str, project_root: &std::path::Path) -> String {
    let mut prompt = LlmClient::system_prompt(user_name, &project_root.to_string_lossy());
    if let Some(hint) = skills::system_prompt_hint(project_root) {
        prompt.push_str(&hint);
    }
    prompt
}

/// Full system prompt: base + skills hint + mode directive + plugin snippets.
fn build_system_prompt_full(
    user_name: &str,
    project_root: &std::path::Path,
    mode: Mode,
    plugin_suffix: &str,
) -> String {
    let mut prompt = build_system_prompt(user_name, project_root);
    match mode {
        Mode::Plan => {
            prompt.push_str(
                "\n\n### mode: plan\n\
                 you are in PLAN MODE (read-only research). you must NOT modify any files, \
                 create anything, remember things, or change settings — investigate, analyze, \
                 and propose. the mutating tools are denied and will error if you attempt them. \
                 present your plan/proposal as your final answer. switch with /mode build when \
                 the user approves the plan.",
            );
        }
        Mode::Build => {
            prompt.push_str("\n\n### mode: build\nchanges are allowed — you have the full toolset.");
        }
        Mode::Auto => {
            prompt.push_str("\n\n### mode: auto\nuse tools as you judge appropriate.");
        }
    }
    if !plugin_suffix.is_empty() {
        prompt.push_str(plugin_suffix);
    }
    prompt
}

/// Heuristic: does this user message likely need tool calls?
fn might_need_tools(msg: &str) -> bool {
    agent::needs_tools(msg)
}

/// Run a vision query across the model chain (user-selected → cloudflare →
/// gpt-4o) until one succeeds. Returns a steering message if the user
/// interrupted mid-stream.
async fn run_vision(
    registry: &ModelRegistry,
    vision_model: &str,
    data_uri: &str,
    label: &str,
    question: &str,
    steer_rx: &std::sync::mpsc::Receiver<String>,
) -> Option<String> {
    ui::show_system(&format!("seeing '{}' with {}", label, vision_model));

    let mut chain: Vec<(String, String)> = Vec::new();
    let vp = registry
        .cloud_provider(vision_model)
        .unwrap_or_else(|| "llm".to_string());
    chain.push((vision_model.to_string(), vp));
    for (m, p) in vision::DEFAULT_FALLBACKS.iter().copied() {
        if !chain.iter().any(|(cm, _)| cm == m) {
            chain.push((m.to_string(), p.to_string()));
        }
    }

    let prompt = vision::describe_prompt(question);
    for (model, provider) in &chain {
        let attempt = format!("{} ({})", model, provider);
        let result = if provider == "llm" {
            vision::describe_llm(model, data_uri, &prompt, steer_rx).await
        } else {
            vision::describe_cloud(model, provider, data_uri, &prompt, steer_rx).await
        };
        match result {
            Ok(r) => return r.steering,
            Err(e) => ui::show_error(&format!("vision via {} failed: {}", attempt, e)),
        }
    }
    None
}

/// Truncate tool results to prevent context overflow
fn truncate_tool_result(result: &str, max_chars: usize) -> String {
    agent::truncate_tool_result(result, max_chars)
}

/// Result of running a tool while live-steering is armed.
enum ToolOutcome<T> {
    /// tool future completed normally (Ok result or Err to handle as usual)
    Done(anyhow::Result<T>),
    /// user typed while the tool was running — redirect immediately
    Interrupted(String),
}

/// Run a tool future while polling the steering channel on a 50ms interval.
/// std mpsc has no async recv, so the execute future and the steer poll are
/// raced with tokio::select!. If the user types while the tool is running we
/// return the input so the caller can redirect; the dropped future cancels
/// cleanly. During foreground applets the input thread is suspended and
/// run_in_window blocks on child.wait(), so no steer can fire there.
async fn run_tool_with_steer<F, T>(
    fut: F,
    steer_rx: &std::sync::mpsc::Receiver<String>,
) -> ToolOutcome<T>
where
    F: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
    tokio::pin!(fut);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Ok(input) = steer_rx.try_recv() {
                    return ToolOutcome::Interrupted(input);
                }
            }
            res = &mut fut => return ToolOutcome::Done(res),
        }
    }
}

/// Result of a permission prompt.
#[derive(Debug)]
enum AskResult {
    /// Run the tool.
    Allow,
    /// Do not run — message to feed back to the model as the tool result.
    Deny(String),
    /// Stop executing the turn. Carries the raw abort input (`\0ctrl-c`, which
    /// the outer loop turns into a graceful shutdown) or an empty string for
    /// soft aborts (menu-esc / closed channel).
    Abort(String),
}

/// Ask the user to allow/deny a sensitive tool call, reading the answer from
/// the steer channel (the same channel hotkeys and steering arrive on). Typed
/// lines arrive WITHOUT the `\0` prefix, so plain "y"/"a"/"n"/"d" answers can
/// never collide with control codes. "always" / "deny-forever" choices are
/// persisted into config.json so the prompt isn't repeated.
///
/// Answer lines must match a keyword EXACTLY. Any other text is not consumed
/// as an answer — it's steering, so the prompt is cancelled and the text is
/// handed back (the caller routes it into `pending_input` like any other
/// mid-generation steer) instead of being auto-answered or error-looped.
/// If no answer arrives within `timeout`, the call is denied with a "timed
/// out" tool error rather than hanging the turn forever.
fn ask_permission(
    name: &str,
    args_str: &str,
    steer_rx: &std::sync::mpsc::Receiver<String>,
    config: &mut serde_json::Value,
    config_path: &std::path::Path,
    timeout: std::time::Duration,
) -> AskResult {
    let preview = if args_str.chars().count() > 80 {
        format!("{}...", crate::util::truncate_chars(args_str, 79))
    } else {
        args_str.to_string()
    };
    ui::show_system(&format!("permission needed — `{}` ({})", name, preview));
    ui::show_system("  [y] allow once · [a] always allow · [n] deny once · [d] deny forever · [ctrl+c] abort");
    ui::dock_redraw_bottom();

    // The steer channel is std::sync::mpsc, so the sync equivalent of
    // tokio::time::timeout is a recv_timeout deadline.
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let input = if remaining.is_zero() {
            None
        } else {
            match steer_rx.recv_timeout(remaining) {
                Ok(i) => Some(i),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    return AskResult::Abort(String::new()); // channel closed
                }
            }
        };
        let Some(input) = input else {
            let msg = format!(
                "error: permission prompt for `{}` timed out after {}s — tool call denied",
                name,
                timeout.as_secs()
            );
            ui::show_system(&format!("permission prompt for `{}` timed out — denied", name));
            return AskResult::Deny(msg);
        };
        // Control codes can't be permission answers; ctrl+c aborts the turn.
        if let Some(code) = input.strip_prefix('\0') {
            match code {
                "ctrl-c" => return AskResult::Abort(input),
                "menu-esc" => return AskResult::Abort(String::new()),
                _ => continue, // other hotkeys don't answer permission prompts
            }
        }
        let trimmed = input.trim();
        match trimmed.to_lowercase().as_str() {
            "y" | "yes" | "allow" | "allow once" => return AskResult::Allow,
            "a" | "always" | "always allow" => {
                permissions::set_override(config, name, "always");
                if let Ok(content) = serde_json::to_string_pretty(&config) {
                    let _ = std::fs::write(&config_path, content);
                }
                ui::show_system(&format!("`{}` will always be allowed (remove the override in config.json to undo)", name));
                return AskResult::Allow;
            }
            "n" | "no" | "deny" | "deny once" => {
                return AskResult::Deny("error: tool call denied by user".to_string());
            }
            "d" | "never" | "deny forever" => {
                permissions::set_override(config, name, "never");
                if let Ok(content) = serde_json::to_string_pretty(&config) {
                    let _ = std::fs::write(&config_path, content);
                }
                return AskResult::Deny(format!(
                    "error: tool call to `{}` blocked by user (deny-forever)",
                    name
                ));
            }
            "abort" => return AskResult::Abort(String::new()),
            "" => continue,
            _ => {
                // Not a permission answer — the user is steering, not
                // answering. Do NOT consume the text as an answer and do NOT
                // error-loop: cancel the prompt and hand the text back so the
                // caller routes it to pending_input like a mid-generation
                // steer (the raw line, keeping its original casing).
                ui::show_interrupted();
                return AskResult::Abort(trimmed.to_string());
            }
        }
    }
}

/// Strip pseudo-tool-call syntax out of ayesha's text so that when she slips
/// into tool JSON/function syntax (despite the system prompt), the fallback
/// can still show the user a readable reply instead of raw tool garbage.
fn strip_pseudo_tool_syntax(text: &str) -> String {
    let mut out = text.to_string();
    // Remove fenced code blocks (```...```)
    loop {
        if let Some(start) = out.find("```") {
            if let Some(end_rel) = out[start + 3..].find("```") {
                let end = start + 3 + end_rel + 3;
                out.replace_range(start..end, "");
            } else {
                out.replace_range(start.., "");
                break;
            }
        } else {
            break;
        }
    }
    // Remove { ... } JSON-ish blocks (tool call payloads)
    loop {
        if let Some(start) = out.find('{') {
            if let Some(end_rel) = out[start..].find('}') {
                let end = start + end_rel + 1;
                out.replace_range(start..end, "");
            } else {
                out.replace_range(start.., "");
                break;
            }
        } else {
            break;
        }
    }
    out.trim().to_string()
}

/// Parse memory markers from ayesha's output and store them automatically.
/// Returns the cleaned text with markers stripped.
fn parse_and_store_memories(text: &str, memory: &mut MemoryStore) -> String {
    let mut cleaned = text.to_string();

    // [REMEMBER: content] — store as user_pref memory
    while let Some(start) = cleaned.find("[REMEMBER:") {
        if let Some(end) = cleaned[start..].find(']') {
            let inner = cleaned[start + 10..start + end].trim();
            if !inner.is_empty() {
                memory.add_memory("user_pref", inner, vec!["user_request".to_string()], 7);
            }
            cleaned.replace_range(start..=start + end, "");
        } else {
            break;
        }
    }

    // [PREFERENCE: key = value] — store in user_preferences map
    while let Some(start) = cleaned.find("[PREFERENCE:") {
        if let Some(end) = cleaned[start..].find(']') {
            let inner = cleaned[start + 12..start + end].trim();
            if let Some(eq_pos) = inner.find('=') {
                let key = inner[..eq_pos].trim().to_string();
                let value = inner[eq_pos + 1..].trim().to_string();
                if !key.is_empty() && !value.is_empty() {
                    memory.set_preference(&key, &value);
                }
            }
            cleaned.replace_range(start..=start + end, "");
        } else {
            break;
        }
    }

    // [FACT: content] — store as fact
    while let Some(start) = cleaned.find("[FACT:") {
        if let Some(end) = cleaned[start..].find(']') {
            let inner = cleaned[start + 6..start + end].trim();
            if !inner.is_empty() {
                memory.add_memory("fact", inner, vec!["user_request".to_string()], 6);
            }
            cleaned.replace_range(start..=start + end, "");
        } else {
            break;
        }
    }

    // Collapse multiple blank lines left by stripped markers
    while cleaned.contains("\n\n\n") {
        cleaned = cleaned.replace("\n\n\n", "\n\n");
    }

    cleaned.trim().to_string()
}

impl ActiveBackend {
    async fn chat_stream_visible(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[serde_json::Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> anyhow::Result<StreamResult> {
        match self {
            ActiveBackend::Local(c) => c.chat_stream_visible(messages, tools, steer_rx).await,
            ActiveBackend::Cloud(c) => c.chat_stream_visible(messages, tools, steer_rx).await,
        }
    }

    /// Invisible streaming — collects response without printing. Used for the
    /// tool model so its deliberation text doesn't appear on screen.
    async fn chat_stream_collect(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[serde_json::Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> anyhow::Result<StreamResult> {
        match self {
            ActiveBackend::Local(c) => c.chat_stream_collect(messages, tools, steer_rx).await,
            ActiveBackend::Cloud(c) => c.chat_stream_collect(messages, tools, steer_rx).await,
        }
    }
}

#[cfg(windows)]
mod winapi {
    const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5u32;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    extern "system" {
        pub fn AllocConsole() -> i32;
        pub fn GetConsoleWindow() -> *mut core::ffi::c_void;
        pub fn SendMessageW(hwnd: *mut core::ffi::c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
        pub fn GetModuleHandleW(name: *const u16) -> *mut core::ffi::c_void;
        pub fn LoadIconW(instance: *mut core::ffi::c_void, name: *const u16) -> *mut core::ffi::c_void;
        pub fn SetConsoleTitleW(title: *const u16) -> i32;
        pub fn GetStdHandle(std_handle: u32) -> *mut core::ffi::c_void;
        pub fn GetConsoleMode(console: *mut core::ffi::c_void, mode: *mut u32) -> i32;
        pub fn SetConsoleMode(console: *mut core::ffi::c_void, mode: u32) -> i32;
    }

    pub fn init_console() {
        unsafe {
            let console = GetConsoleWindow();
            if console.is_null() {
                AllocConsole();
            }

            let console = GetConsoleWindow();
            if !console.is_null() {
                let module = GetModuleHandleW(std::ptr::null());
                let icon = LoadIconW(module, std::ptr::dangling::<u16>());
                if !icon.is_null() {
                    SendMessageW(console, 0x0080, 0, icon as isize);
                    SendMessageW(console, 0x0080, 1, icon as isize);
                }

                let title: Vec<u16> = "Ayesha-Engine\0".encode_utf16().collect();
                SetConsoleTitleW(title.as_ptr());
            }

            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if !handle.is_null() {
                let mut mode: u32 = 0;
                if GetConsoleMode(handle, &mut mode) != 0 {
                    SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod winapi {
    pub fn init_console() {}
}

/// Read the tri-mind sync configuration (repo, branch, auto_push) from
/// tri_mind_sync.json at the project root. Falls back to the github defaults.
fn sync_config(root: &std::path::Path) -> (String, String, bool) {
    let path = root.join("tri_mind_sync.json");
    if let Ok(s) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            let g = v.get("github");
            let repo = g.and_then(|g| g.get("repo")).and_then(|r| r.as_str())
                .unwrap_or("ayesha-os").to_string();
            let branch = g.and_then(|g| g.get("branch")).and_then(|b| b.as_str())
                .unwrap_or("master").to_string();
            let auto_push = g.and_then(|g| g.get("auto_push")).and_then(|b| b.as_bool())
                .unwrap_or(true);
            return (repo, branch, auto_push);
        }
    }
    ("ayesha-os".to_string(), "master".to_string(), true)
}

/// Print a status summary from .tri_mind_state/engine_state.json (per-node
/// connectivity + last-sync times). Silently no-ops if state doesn't exist.
fn show_sync_state(root: &std::path::Path) {
    let path = root.join(".tri_mind_state").join("engine_state.json");
    let s = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            ui::show_system("no engine_state.json yet — run /sync once to generate it");
            return;
        }
    };
    let v: serde_json::Value = match serde_json::from_str(&s) {
        Ok(v) => v,
        Err(_) => {
            ui::show_system("engine_state.json exists but is unreadable");
            return;
        }
    };

    let last_full = v.get("last_full_sync").and_then(|x| x.as_str()).unwrap_or("unknown");
    println!();
    println!("  {} {}", theme::paint(Role::Accent, "tri-mind engine state:"),
        theme::paint(Role::Dim, format!("last full sync: {last_full}")));
    if let Some(nodes) = v.get("nodes").and_then(|n| n.as_object()) {
        for (name, node) in nodes {
            let connected = node.get("connected").and_then(|c| c.as_bool()).unwrap_or(false);
            let last = node.get("last_sync").and_then(|x| x.as_str()).unwrap_or("unknown");
            let dot = if connected { theme::paint(Role::Success, "●") } else { theme::paint(Role::Error, "○") };
            let status = if connected { "connected" } else { "disconnected" };
            println!("  {} {}  {}  {}", dot, theme::paint(Role::Text, name),
                theme::paint(if connected { Role::Success } else { Role::Error }, status),
                theme::paint(Role::Dim, format!("last sync: {last}")));
        }
    }
    println!();
}

/// Match natural-language applet phrases like "launch flora cli", "open
/// hivebeat", or "stop flora-cli" to a registered applet name.
/// Returns (applet name, is_stop).
fn parse_applet_phrase(names: &[String], input: &str) -> Option<(String, bool)> {
    let normalize = |s: &str| -> String {
        s.chars()
            .filter(|c| *c != '-' && *c != '_' && *c != ' ')
            .collect::<String>()
            .to_lowercase()
    };
    let phrases: [(&str, bool); 5] = [
        ("launch ", false),
        ("open ", false),
        ("start ", false),
        ("run ", false),
        ("stop ", true),
    ];
    let lower = input.to_lowercase();
    for (prefix, is_stop) in phrases {
        if !lower.starts_with(prefix) {
            continue;
        }
        let candidate = input[prefix.len()..].trim();
        let target = normalize(candidate);
        let matched = names.iter().find(|n| normalize(n) == target).cloned();
        return matched.map(|m| (m, is_stop));
    }
    None
}

/// Open an applet as a page in the current window (foreground applets) or in
/// its own window (background applets).
fn switch_applet_page(
    manager: &mut AppletManager,
    name: &str,
    steer_tx: &std::sync::mpsc::Sender<String>,
    steer_rx: &std::sync::mpsc::Receiver<String>,
    input_flag: &mut std::sync::Arc<std::sync::atomic::AtomicBool>,
    menu_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    if manager.is_foreground(name) {
        match manager.run_in_window(name, steer_tx, steer_rx, input_flag, menu_flag) {
            Ok(()) => ui::show_system(&format!("returned from {} — press ctrl+p to switch pages again", name)),
            Err(e) => ui::show_error(&e),
        }
    } else if manager.is_running(name) {
        ui::show_system(&format!("{} is already running in its own window", name));
    } else {
        match manager.launch(name) {
            Ok(()) => ui::show_system(&format!("launched {} in its own window", name)),
            Err(e) => ui::show_error(&e),
        }
    }
}

/// Graceful shutdown: persist memory/history, stop applets, and — critically —
/// release raw mode so the terminal isn't left in a broken state.
fn graceful_shutdown(
    messages: &mut Vec<ChatMessage>,
    memory: &mut MemoryStore,
    prompt_history: &mut PromptHistory,
    manager: &mut AppletManager,
    project_root: &std::path::Path,
) {
    // Auto-parse memory markers from ayesha's last response
    if let Some(last) = messages.last() {
        if last.role == "assistant" {
            let cleaned = parse_and_store_memories(&last.content, memory);
            if cleaned != last.content {
                messages.last_mut().unwrap().content = cleaned;
            }
        }
    }

    let _ = memory.save();
    let _ = prompt_history.save();
    if messages.len() > 1 {
        if let Ok(path) = session::save_session(project_root, session::DEFAULT_SESSION, messages) {
            println!("  {} {}", theme::paint(Role::Dim, "◆"), theme::paint(Role::Dim, format!("session saved → {}", path.display())));
        }
    }
    manager.stop_all();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    let _ = crossterm::terminal::disable_raw_mode();
    println!();
    println!("  {} {}", theme::paint(Role::Success, "●"), theme::paint(Role::Accent, "ayesha-os shutting down"));
    println!("  {} {}", theme::paint(Role::Accent, "◆"), theme::paint(Role::Dim, format!("saved {}", memory.summary())));
    println!();
}

/// Backend/provider tag for a model's quick-switcher row (the dim tag
/// column next to the model name), e.g. "llm", "kilo".
fn model_backend_tag(m: &model_registry::ModelProfile) -> String {
    match &m.backend {
        model_registry::Backend::Local => "llm".to_string(),
        model_registry::Backend::Cloud { provider } => provider.clone(),
    }
}

/// Compact context-length label for a model's quick-switcher row
/// (right-aligned column), e.g. "32k", "200k", "1m".
fn model_ctx_label(ctx: u32) -> String {
    if ctx >= 1_000_000 && ctx % 1_000_000 == 0 {
        format!("{}m", ctx / 1_000_000)
    } else if ctx >= 1_000 {
        format!("{}k", ctx / 1_000)
    } else {
        ctx.to_string()
    }
}

/// Switch the active model: update the registry, rebuild the client backend
/// (cloud setup-failure reverts to kilo-auto/free), refresh the dock
/// status, and report the result. Shared by the `/model` command and the
/// ctrl+p quick-switcher's model rows.
async fn apply_model_switch(
    registry: &mut ModelRegistry,
    current_model: &mut String,
    client: &mut ActiveBackend,
    name: &str,
    tool_model_name: &str,
) {
    match registry.set_model(name) {
        Ok(()) => {
            *current_model = name.to_string();
            if registry.is_cloud_model(name) {
                let provider = registry.cloud_provider(name).unwrap_or_default();
                match CloudClient::new(name, &provider) {
                    Ok(cc) => {
                        *client = ActiveBackend::Cloud(cc);
                        ui::show_system(&format!("switched to cloud model: {} ({})", name, provider));
                    }
                    Err(e) => {
                        ui::show_error(&format!("cloud setup failed: {}. run .\\scripts\\setup-cloud.ps1", e));
                        // Revert to default
                        *current_model = "kilo-auto/free".to_string();
                        *client = match CloudClient::new("kilo-auto/free", "kilo") {
                            Ok(cc) => ActiveBackend::Cloud(cc),
                            Err(_) => ActiveBackend::Local(LlmClient::new("ayesha")),
                        };
                    }
                }
            } else {
                *client = ActiveBackend::Local(LlmClient::new(current_model));
                ui::show_system(&format!("switched to: {}", name));
            }
        }
        Err(e) => ui::show_error(&e.to_string()),
    }
    registry.detect().await;
    ui::dock_status(&format!("{} | tool: {}", current_model, tool_model_name));
}

#[tokio::main]
#[allow(unused_assignments)]
async fn main() -> anyhow::Result<()> {
    // --selftest: headless E2E smoke test, exit 0 on success
    if std::env::args().any(|a| a == "--selftest") {
        return selftest().await;
    }

    // --headless "message": run one agent turn non-interactively, exit 0 if tools executed
    if let Some(headless_msg) = std::env::args().skip_while(|a| a != "--headless").nth(1) {
        return run_headless(&headless_msg).await;
    }

    winapi::init_console();
    theme::force_truecolor();
    theme::apply_no_color();
    theme::load_from_config(None);

    let session_start = std::time::Instant::now();
    let mut manager = AppletManager::new();
    let sandbox = Sandbox::default_workspace().with_sandbox(manager.sandbox);
    // Agent mode (plan/build/auto) + declarative plugins from ayesha.json —
    // both loaded once at startup and applied to every payload/system prompt.
    let mut current_mode = Mode::from_str(&manager.mode).unwrap_or(Mode::Auto);
    let plugin_registry = plugins::PluginRegistry::from_config(&manager.plugins);
    let plugin_suffix = plugins::snippet_from_configs(&manager.plugins);
    let executor = ToolExecutor::new(sandbox).with_plugins(plugin_registry.clone());
    let mut current_model = "kilo-auto/free".to_string();
    let fallback_model = "xiaomi/mimo-v2.5-pro";
    let mut client = match CloudClient::new("kilo-auto/free", "kilo") {
        Ok(cc) => ActiveBackend::Cloud(cc),
        Err(_) => match CloudClient::new(fallback_model, "openrouter") {
            Ok(cc) => {
                current_model = fallback_model.to_string();
                ActiveBackend::Cloud(cc)
            }
            Err(_) => {
                current_model = "ayesha".to_string();
                ActiveBackend::Local(LlmClient::new("ayesha"))
            }
        },
    };
    let mut tool_client = match CloudClient::new("kilo-auto/free", "kilo") {
        Ok(cc) => ActiveBackend::Cloud(cc),
        Err(_) => match CloudClient::new(fallback_model, "openrouter") {
            Ok(cc) => ActiveBackend::Cloud(cc),
            Err(_) => ActiveBackend::Local(LlmClient::new("kilo-auto/free")),
        },
    };
    let mut tool_model_name =     match &tool_client {
        ActiveBackend::Cloud(c) => c.model.clone(),
        ActiveBackend::Local(_) => "kilo-auto/free".to_string(),
    };
    let mut memory = MemoryStore::load();
    let mut prompt_history = PromptHistory::load();

    // Self-analysis, tool evolution, and a separate llm client for generative tools
    let project_root = std::env::current_dir().unwrap_or_default();
    let analyzer = SelfAnalyzer::new(project_root.clone());
    let tool_llm = LlmClient::new("ayesha");
    let evolver = ToolEvolver::new(
        tool_defs::known_tool_names().into_iter().map(|s| s.to_string()).collect()
    );

    // Model registry
    let mut registry = ModelRegistry::new();
    registry.detect().await;

    // Load config, prompt for user name
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let config_path = exe_dir.join("config.json");
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Reload theme with any persisted selection from config.json
    let active_theme = config.get("theme").and_then(|v| v.as_str()).map(|s| s.to_string());
    theme::load_from_config(active_theme.as_deref());

    // Output enforcement (lowercase + emoji strip, code-fence aware).
    // Default ON — mirrors the ayesha-os lowercase-proxy. Disable with
    // "lowercase_enforce": false in config.json if you need raw output.
    let enforce = config.get("lowercase_enforce").and_then(|v| v.as_bool()).unwrap_or(true);
    format::set_enabled(enforce);

    let user_name = match config.get("user_name").and_then(|v| v.as_str()) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => {
            print!("\n  {} {}",
                theme::paint(Role::Accent, "◆"),
                theme::paint(Role::Primary, "what should i call you, senpai?"));
            std::io::stdout().flush()?;
            let mut name = String::new();
            std::io::stdin().read_line(&mut name)?;
            let name = name.trim().to_string();
            let name = if name.is_empty() { "user".to_string() } else { name };
            config["user_name"] = serde_json::json!(name);
            if let Ok(content) = serde_json::to_string_pretty(&config) {
                let _ = std::fs::write(&config_path, content);
            }
            println!("  {} {} {}",
                theme::bold(Role::Success, "✔"),
                theme::paint(Role::Accent, format!("okay, {}!", name)),
                theme::paint(Role::Dim, "remember that one, desu~"));
            name
        }
    };

    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage {
            role: "system".to_string(),
            content: build_system_prompt_full(&user_name, &project_root, current_mode, &plugin_suffix),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    // Resume last session? (config: "resume_prompt": false to disable the prompt)
    // Must run in cooked mode, BEFORE raw mode is enabled and BEFORE the input
    // thread starts. In raw mode Enter arrives as \r (never \n), so read_line()
    // blocks forever, and the input thread races us for the console anyway.
    if config.get("resume_prompt").and_then(|v| v.as_bool()).unwrap_or(true)
        && session::session_exists(&project_root, session::DEFAULT_SESSION)
    {
        print!("  {} {} ",
            theme::bold(Role::Primary, "⟲"),
            theme::paint(Role::Text, "resume last session? (y/n)"));
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        let answer = answer.trim().to_lowercase();
        if answer == "y" || answer == "yes" {
            match session::load_session(&project_root, session::DEFAULT_SESSION) {
                Ok(saved) => {
                    let n = saved.len().saturating_sub(1);
                    messages.extend(saved.into_iter().skip(1));
                    ui::show_system(&format!("resumed last session ({} messages)", n));
                }
                Err(e) => ui::show_error(&format!("failed to resume session: {}", e)),
            }
        }
    }

    // Enable raw mode for Ctrl+M detection
    let _ = crossterm::terminal::enable_raw_mode();

    // Enable mouse capture once at startup — NOT in the input thread, to
    // avoid a race where an old thread's DisableMouseCapture clobbers the
    // new thread's capture when the input thread is recycled.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);

    // Steering channel: input thread sends keys here. The thread is poll-based
    // so it can be suspended while a foreground applet owns the terminal.
    let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();

    // Build completion candidates: slash commands + applet names
    let mut completion_candidates: Vec<String> = vec![
        "help", "clear", "models", "auto", "sync", "apps", "run", "stop",
        "model", "toolmodel", "pull", "route", "name", "exit",
        "stats", "history", "compact", "save", "load", "system", "export", "ping", "reset",
        "joke", "time", "uptime", "config",
        "memory", "skills", "analyze", "evolve", "refine",
        "sessions", "resume", "newsession",
        "ascii",
    ].into_iter().map(String::from).collect();
    for name in manager.names() {
        completion_candidates.push(name);
    }

    let menu_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut input_flag = applet_runner::spawn_input_thread(steer_tx.clone(), completion_candidates, menu_flag.clone());

    ui::print_banner();
    ui::show_system(&memory.summary());

    let tools = tool_defs::tool_definitions_core();
    // Merge in the delegate tool (synthetic, kept out of the static catalog)
    // and any config-driven plugin tools, then slice for the payload.
    let mut tools_arr = match tools {
        serde_json::Value::Array(arr) => arr,
        _ => Vec::new(),
    };
    tools_arr.push(tools::delegate_tool_definition());
    tools_arr.extend(plugin_registry.tool_definitions());
    let tools_payload = serde_json::Value::Array(tools_arr);
    let tools = streaming::tool_payload_slice(&tools_payload);
    let mut vision_model = "kilo-auto/free".to_string();
    ui::show_system(&format!("chat model: {} | tool model: {}", current_model, tool_model_name));

    // Warm up llm — preload local models into memory so first interaction is
    // fast. Skipped when the default model is cloud (kilo-auto/free), which has no
    // local preload. Set "boot_warmup": false in config.json to disable entirely.
    let default_is_local = !registry.is_cloud_model(&current_model);
    if default_is_local && config.get("boot_warmup").and_then(|v| v.as_bool()).unwrap_or(true) {
        let tool_model = tool_model_name.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build warmup runtime");
            rt.block_on(async move {
                let (_, warmup_rx) = std::sync::mpsc::channel::<String>();
                let _ = tokio::join!(
                    async {
                        let client = LlmClient::new("ayesha");
                        let msgs = vec![ChatMessage { role: "user".to_string(), content: "hi".to_string(), tool_calls: None, tool_call_id: None }];
                        let _ = client.chat_stream_collect(&msgs, None, &warmup_rx).await;
                    },
                    async {
                        let client = LlmClient::new(&tool_model);
                        let msgs = vec![ChatMessage { role: "user".to_string(), content: "hi".to_string(), tool_calls: None, tool_call_id: None }];
                        let _ = client.chat_stream_collect(&msgs, None, &warmup_rx).await;
                    }
                );
            });
        });
    }

    // Holds steering input that needs to be processed as the next user message
    let mut pending_input: Option<String> = None;

    enum InputMode { Normal, AppletMenu }
    let mut input_mode = InputMode::Normal;
    let mut applet_cycle_idx: Option<usize> = None;

    // Build the quick-switcher item list once at startup (all applets only);
    // statuses refresh on each menu open.
    let mut menu_items: Vec<ui::MenuItem> = Vec::new();
    {
        menu_items.push(ui::MenuItem {
            name: "engine".to_string(),
            desc: "terminal persona host — that's me".to_string(),
            kind: ui::MenuItemKind::Applet,
            running: true,
            active: false,
            foreground: true,
            ctx: None,
        });
        for name in manager.names() {
            if name == "engine" { continue; }
            if let Some(e) = manager.entries.get(&name) {
                menu_items.push(ui::MenuItem {
                    name: name.clone(),
                    desc: e.desc.clone(),
                    kind: ui::MenuItemKind::Applet,
                    running: manager.is_running(&name),
                    active: false,
                    foreground: e.foreground,
                    ctx: None,
                });
            }
        }
    }

    // Pin status bar + input prompt to the bottom of the screen, ayesha-os-style.
    ui::dock_init(&format!("{} | tool: {}", current_model, tool_model_name));

    loop {
        // ── read user input ──
        let input = if let Some(p) = pending_input.take() {
            p
        } else {
            match input_mode {
                InputMode::Normal => ui::dock_prompt(),
                InputMode::AppletMenu => ui::menu_prompt(),
            }
            let inp = match steer_rx.recv() {
                Ok(i) => i,
                Err(_) => break,
            };
            if inp.is_empty() {
                if matches!(input_mode, InputMode::AppletMenu) {
                    menu_flag.store(false, std::sync::atomic::Ordering::Relaxed);
                    input_mode = InputMode::Normal;
                    ui::popup_leave();
                }
                continue;
            }
            inp
        };

        // All output this turn (slash commands, user card, model stream)
        // renders inside the docked region, above the pinned prompt.
        ui::dock_submit_goto();

        // ── control keys (work from normal input and steering interrupts) ──
        // Ctrl+C → exit (graceful: save memory, stop applets, release raw mode)
        if input == "\0ctrl-c" {
            menu_flag.store(false, std::sync::atomic::Ordering::Relaxed);
            graceful_shutdown(&mut messages, &mut memory, &mut prompt_history, &mut manager, &project_root);
            break;
        }
        // Ctrl+M / Ctrl+P → open interactive applet switcher
        if input == "\0ctrl-m" || input == "\0ctrl-p" {
            input_mode = InputMode::AppletMenu;
            // Refresh applet running statuses
            for item in menu_items.iter_mut() {
                if item.kind != ui::MenuItemKind::Applet { continue; }
                item.running = item.name == "engine" || manager.is_running(&item.name);
            }
            let mut menu_idx: usize = 0;
            let mut menu_filter: String = String::new();
            // Drain stale keystrokes queued before the menu opened
            while steer_rx.try_recv().is_ok() {}
            menu_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            // Draw the menu on a clean alternate-screen popup, centered.
            ui::popup_enter();
            let (rendered, line_count) = ui::draw_launcher_menu(&menu_items, menu_idx, &menu_filter);
            ui::draw_popup_centered(&rendered, line_count);

            // Inner menu loop
            let mut selected: Option<String> = None;
            loop {
                let menu_input = match steer_rx.recv() {
                    Ok(i) => i,
                    Err(_) => break,
                };
                match menu_input.as_str() {
                    "\0ctrl-c" => {
                        menu_flag.store(false, std::sync::atomic::Ordering::Relaxed);
                        ui::popup_leave();
                        graceful_shutdown(&mut messages, &mut memory, &mut prompt_history, &mut manager, &project_root);
                        return Ok(());
                    }
                    "\0menu-up" => {
                        menu_idx = menu_idx.saturating_sub(1);
                        let (rendered, lc) = ui::draw_launcher_menu(&menu_items, menu_idx, &menu_filter);
                        ui::draw_popup_centered(&rendered, lc);
                    }
                    "\0menu-down" => {
                        let lower = menu_filter.to_lowercase();
                        let filtered_len = menu_items.iter().filter(|it| {
                            menu_filter.is_empty()
                                || it.name.to_lowercase().contains(&lower)
                                || it.desc.to_lowercase().contains(&lower)
                        }).count();
                        if filtered_len > 0 && menu_idx + 1 < filtered_len {
                            menu_idx += 1;
                        }
                        let (rendered, lc) = ui::draw_launcher_menu(&menu_items, menu_idx, &menu_filter);
                        ui::draw_popup_centered(&rendered, lc);
                    }
                    "\0menu-backspace" => {
                        menu_filter.pop();
                        menu_idx = 0;
                        let (rendered, lc) = ui::draw_launcher_menu(&menu_items, menu_idx, &menu_filter);
                        ui::draw_popup_centered(&rendered, lc);
                    }
                    "\0menu-esc" => {
                        break;
                    }
                    "\0menu-enter" => {
                        let lower = menu_filter.to_lowercase();
                        let filtered: Vec<&ui::MenuItem> = menu_items.iter().filter(|it| {
                            menu_filter.is_empty()
                                || it.name.to_lowercase().contains(&lower)
                                || it.desc.to_lowercase().contains(&lower)
                        }).collect();
                        if let Some(item) = filtered.get(menu_idx) {
                            if item.kind == ui::MenuItemKind::Applet {
                                selected = Some(item.name.clone());
                            }
                        }
                        break;
                    }
                    s if s.starts_with("\0menu-char:") => {
                        let c = &s["\0menu-char:".len()..];
                        if menu_filter.is_empty() && c == "x" {
                            // Stop action — applets only (filter is empty here,
                            // so this list is every item in menu order)
                            let filtered: Vec<&ui::MenuItem> = menu_items.iter().collect();
                            if let Some(item) = filtered.get(menu_idx) {
                                if item.kind == ui::MenuItemKind::Applet {
                                    if item.running {
                                        match manager.stop(&item.name) {
                                            Ok(()) => ui::show_system(&format!("stopped {}", item.name)),
                                            Err(e) => ui::show_error(&e),
                                        }
                                    } else {
                                        ui::show_system(&format!("{} is not running", item.name));
                                    }
                                    for entry in menu_items.iter_mut() {
                                        if entry.kind == ui::MenuItemKind::Applet && entry.name != "engine" {
                                            entry.running = manager.is_running(&entry.name);
                                        }
                                    }
                                }
                            }
                        } else {
                            for ch in c.chars() {
                                menu_filter.push(ch);
                            }
                            menu_idx = 0;
                        }
                        let (rendered, lc) = ui::draw_launcher_menu(&menu_items, menu_idx, &menu_filter);
                        ui::draw_popup_centered(&rendered, lc);
                    }
                    _ => {}
                }
            }

            // Exit menu: leave the popup screen and switch off
            menu_flag.store(false, std::sync::atomic::Ordering::Relaxed);
            ui::popup_leave();
            input_mode = InputMode::Normal;

            // If Enter selected something, take action
            if let Some(name) = selected {
                switch_applet_page(&mut manager, &name, &steer_tx, &steer_rx, &mut input_flag, &menu_flag);
            }
            continue;
        }

        // Ctrl+V → see what's on the clipboard (image or copied image file)
        if input == "\0paste-vision" || input.starts_with("\0paste-vision:") {
            let effective_vision_model = if registry.has_vision(&vision_model) {
                vision_model.clone()
            } else {
                let fallback = registry.models.iter()
                    .find(|m| m.capabilities.contains(&model_registry::Capability::Vision))
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| "kilo-auto/free".to_string());
                if fallback != vision_model {
                    ui::show_system(&format!("auto-routing vision to: {}", fallback));
                }
                fallback
            };
            let data_uri: Option<(String, String)> = if let Some(p) = input.strip_prefix("\0paste-vision:") {
                vision::image_data_uri(&std::path::PathBuf::from(p))
                    .ok()
                    .map(|u| (u, p.to_string()))
            } else if let Some(uri) = vision::clipboard_image_data_uri() {
                Some((uri, "clipboard".to_string()))
            } else {
                // clipboard text that is a path to an image file
                #[cfg(not(target_os = "android"))]
                let text = arboard::Clipboard::new()
                    .ok()
                    .and_then(|mut cb| cb.get_text().ok())
                    .unwrap_or_default();
                // android/termux has no clipboard backend — graceful no-op
                #[cfg(target_os = "android")]
                let text = String::new();
                vision::resolve_path(&text)
                    .and_then(|p| vision::image_data_uri(&p).ok().map(|u| (u, p.display().to_string())))
            };

            match data_uri {
                Some((uri, label)) => {
                    if let Some(steer) = run_vision(&registry, &effective_vision_model, &uri, &label, "", &steer_rx).await {
                        pending_input = Some(steer);
                    }
                }
                None => ui::show_system("no image in clipboard — copy an image file or take a screenshot (win+shift+s), then ctrl+v"),
            }
            continue;
        }

        // Shift + Up / Shift + Down → cycle applets
        if input == "\0shift-up" || input == "\0shift-down" {
            let applets = manager.names();
            if !applets.is_empty() {
                let next_idx = match applet_cycle_idx {
                    None => 0,
                    Some(i) => {
                        if input == "\0shift-down" {
                            (i + 1) % applets.len()
                        } else {
                            if i == 0 { applets.len() - 1 } else { i - 1 }
                        }
                    }
                };
                applet_cycle_idx = Some(next_idx);
                let target = &applets[next_idx];
                match manager.launch(target) {
                    Ok(()) => ui::show_system(&format!("(Shift+Arrow) Launched applet: /{}", target)),
                    Err(e) => ui::show_system(&format!("(Shift+Arrow) Applet /{} status: {}", target, e)),
                }
            }
            continue;
        }

        // ── slash-command handling ──
        let was_slash = input.starts_with('/');
        let input = if was_slash {
            let cmd = input[1..].trim().to_string();
            if cmd.is_empty() {
                ui::draw_command_overlay(None);
                continue;
            }
            cmd
        } else {
            input
        };

        let lower = input.to_lowercase();

        // ── exit only ──
        match lower.as_str() {
            "exit" | "quit" | "q" => {
                graceful_shutdown(&mut messages, &mut memory, &mut prompt_history, &mut manager, &project_root);
                break;
            }
            _ => {}
        }
        let is_ascii_cmd = lower == "ascii" || lower.starts_with("ascii ");
        // ── natural-language applet phrases (no model round-trip) ──
        // "launch flora cli", "open hivebeat", "stop flora-cli", etc.
        if let Some((name, is_stop)) = parse_applet_phrase(&manager.names(), &input) {
            if is_stop {
                match manager.stop(&name) {
                    Ok(()) => ui::show_system(&format!("stopped {}", name)),
                    Err(e) => ui::show_error(&e),
                }
            } else {
                switch_applet_page(&mut manager, &name, &steer_tx, &steer_rx, &mut input_flag, &menu_flag);
            }
            continue;
        }

        // ── ascii art shortcut (no model round-trip for bare `ascii`) ──
        let user_content = if is_ascii_cmd {
            let subject = if lower == "ascii" {
                "ayesha".to_string()
            } else {
                input[6..].trim().to_string()
            };
            if subject.is_empty() {
                ui::show_system("usage: ascii <subject> — display ascii art of something (e.g. ascii cat)");
                continue;
            }
            format!(
                "generate a large, detailed ASCII art piece of a {subject}. \
                use depth, shading, and character detail as a master of ascii art. \
                output ONLY the ascii art, nothing else — no commentary, no explanation."
            )
        } else if lower.starts_with("route ") {
            // ── route (handle `route <query>` prefix) ──
            let query = input[6..].trim().to_string();
            let target = registry.select_model(&query);
            if target.name != current_model {
                ui::show_routing(&target.name);
                current_model = target.name.clone();
                if registry.is_cloud_model(&current_model) {
                    let provider = registry.cloud_provider(&current_model).unwrap_or_default();
                    match CloudClient::new(&current_model, &provider) {
                        Ok(cc) => client = ActiveBackend::Cloud(cc),
                        Err(_) => client = match CloudClient::new("kilo-auto/free", "kilo") {
                            Ok(cc) => ActiveBackend::Cloud(cc),
                            Err(_) => ActiveBackend::Local(LlmClient::new("ayesha")),
                        },
                    }
                } else {
                    client = ActiveBackend::Local(LlmClient::new(&current_model));
                }
                ui::dock_status(&format!("{} | tool: {}", current_model, tool_model_name));
            }
            query
        } else {
            input
        };

        // ── auto-routing ──
        if registry.auto_route {
            let target = registry.select_model(&user_content);
            if target.name != current_model {
                ui::show_routing(&target.name);
                current_model = target.name.clone();
                ui::dock_status(&format!("{} | tool: {}", current_model, tool_model_name));
                if registry.is_cloud_model(&current_model) {
                    let provider = registry.cloud_provider(&current_model).unwrap_or_default();
                    match CloudClient::new(&current_model, &provider) {
                        Ok(cc) => client = ActiveBackend::Cloud(cc),
                        Err(_) => client = match CloudClient::new("kilo-auto/free", "kilo") {
                            Ok(cc) => ActiveBackend::Cloud(cc),
                            Err(_) => ActiveBackend::Local(LlmClient::new("ayesha")),
                        },
                    }
                } else {
                    client = ActiveBackend::Local(LlmClient::new(&current_model));
                }
            }
        }

        // ── agent loop (dual-model: ayesha for personality, qwen for tools) ──
        let msg_count_before = messages.len();
        let mut needs_tools = might_need_tools(&user_content);

        ui::show_user_msg(&user_content);
        ui::show_turn_separator();

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_content,
            tool_calls: None,
            tool_call_id: None,
        });

        let mut steer_happened = false;

        // Step 1: Call ayesha WITH tools — she's a real agent and can act directly
        let first_result = client
            .chat_stream_visible(&messages, Some(tools), &steer_rx)
            .await;

        let first_result = match first_result {
            Ok(r) => r,
            Err(e) => {
                ui::show_error(&format!("ayesha error: {}", e));
                messages.truncate(msg_count_before);
                continue;
            }
        };

        // Extract flags before consuming content
        let first_was_steered = first_result.was_steered();
        let first_had_tools = first_result.has_tool_calls();
        let first_tool_calls = first_result.tool_calls.clone();

        // ayesha's raw text plus whether it currently sits in message history.
        // Used by the safety net below (pseudo-tool-call routing) and the qwen
        // no-tools fallback, so they're declared at iteration scope.
        let mut ayesha_text = String::new();
        let mut ayesha_text_pushed = false;

        if first_was_steered {
            ui::show_interrupted();
            pending_input = first_result.steering;
            steer_happened = true;
        } else if first_had_tools {
            // Ayesha called tools directly (unlikely but possible)
            // Execute them, then get final response
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: first_result.content,
                tool_calls: Some(first_tool_calls.clone()),
                tool_call_id: None,
            });

            for tool_call in &first_tool_calls {
                let name = &tool_call.function.name;
                let args = &tool_call.function.arguments;
                let args_str = serde_json::to_string(args).unwrap_or_default();

                ui::show_tool_call(name, &args_str);

                // Plan-mode gate: mutating tools (and shell-backed plugin
                // tools) are denied outright — plan mode is read-only.
                if current_mode == Mode::Plan
                    && (crate::tools::is_mutating(name) || plugin_registry.has_tool(name))
                {
                    let deny = crate::tools::plan_mode_deny_message(name).unwrap_or_else(|| {
                        format!(
                            "error: plan mode: plugin tool '{}' is denied (plan mode is read-only research)",
                            name
                        )
                    });
                    ui::show_tool_err(name, &deny);
                    prompt_history.record_usage(name, false, Some(deny.clone()), &args_str);
                    let _ = prompt_history.save();
                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: deny,
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                    continue;
                }

                // Permission gate: sensitive tools need the user's ok.
                // Plugin tools count as sensitive too — they run arbitrary
                // shell commands, so they must never skip the prompt in
                // build/auto mode (unless the user set an override).
                match permissions::decide_with(name, &config, |n| plugin_registry.has_tool(n)) {
                    Verdict::Allow => {}
                    Verdict::Denied(msg) => {
                        ui::show_tool_err(name, &msg);
                        prompt_history.record_usage(name, false, Some(msg.clone()), &args_str);
                        let _ = prompt_history.save();
                        memory.add_memory(
                            "error",
                            &format!("tool '{}' blocked by user permission: {}", name, msg),
                            vec![name.to_string(), "error".to_string()],
                            3,
                        );
                        messages.push(ChatMessage {
                            role: "tool".to_string(),
                            content: msg,
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                        });
                        continue;
                    }
                    Verdict::Prompt => {
                        match ask_permission(name, &args_str, &steer_rx, &mut config, &config_path, std::time::Duration::from_secs(120)) {
                            AskResult::Allow => {}
                            AskResult::Deny(msg) => {
                                ui::show_tool_err(name, &msg);
                                prompt_history.record_usage(name, false, Some(msg.clone()), &args_str);
                                let _ = prompt_history.save();
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: msg,
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id.clone()),
                                });
                                continue;
                            }
                            AskResult::Abort(input) => {
                                ui::show_interrupted();
                                // Don't leave the assistant message with an
                                // orphaned tool call: push a synthetic result
                                // so history stays valid whatever path the
                                // truncation guard takes next turn.
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: "tool call cancelled".to_string(),
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id.clone()),
                                });
                                if !input.is_empty() {
                                    pending_input = Some(input);
                                } else {
                                    ui::show_system("tool call cancelled");
                                }
                                steer_happened = true;
                                break;
                            }
                        }
                    }
                }

                let tool_outcome = run_tool_with_steer(
                    executor.execute(name, args, &mut ToolContext {
                        memory: &mut memory,
                        prompt_history: &mut prompt_history,
                        analyzer: &analyzer,
                        evolver: &evolver,
                    llm: &tool_llm,
                    backend: &client,
                    project_root: &project_root,
                        applet_manager: &mut manager,
                        steer_tx: &steer_tx,
                        steer_rx: &steer_rx,
                        input_flag: &mut input_flag,
                        menu_flag: &menu_flag,
                    }),
                    &steer_rx,
                ).await;

                let tool_result = match tool_outcome {
                    ToolOutcome::Done(Ok(r)) => r,
                    ToolOutcome::Done(Err(e)) => {
                        let err_msg = format!("error: {}", e);
                        prompt_history.record_usage(name, false, Some(err_msg.clone()), &args_str);
                        let _ = prompt_history.save();
                        err_msg
                    }
                    ToolOutcome::Interrupted(input) => {
                        ui::show_interrupted();
                        // Synthetic tool result keeps the history valid — the
                        // assistant message holds this tool_call_id.
                        messages.push(ChatMessage {
                            role: "tool".to_string(),
                            content: "tool call cancelled".to_string(),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                        });
                        pending_input = Some(input);
                        steer_happened = true;
                        break;
                    }
                };

                if !tool_result.starts_with("error:") {
                    prompt_history.record_usage(name, true, None, &args_str);
                } else {
                    memory.add_memory(
                        "error",
                        &format!("tool '{}' failed: {}", name, tool_result),
                        vec![name.to_string(), "error".to_string()],
                        3,
                    );
                }

                if tool_result.starts_with("error:") {
                    ui::show_tool_err(name, &tool_result);
                } else {
                    ui::show_tool_ok(name, &tool_result);
                }

                let tool_result = truncate_tool_result(&tool_result, 8192);

                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: tool_result,
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                });
            }

            // Get final ayesha response after tool execution
            if !steer_happened {
                let final_result = client
                    .chat_stream_visible(&messages, Some(tools), &steer_rx)
                    .await;

                if let Ok(r) = final_result {
                    if r.was_steered() {
                        // The FINAL stream was steered — route the steer back
                        // as the next user input and drop the partial content
                        // (the truncation guard below resets messages).
                        ui::show_interrupted();
                        pending_input = r.steering;
                        steer_happened = true;
                    } else if !r.content.is_empty() {
                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: r.content,
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                }
            }
        } else {
            // Pure chat response from ayesha — done
            // Safety net: detect pseudo-tool-call attempts and DON'T push them
            // to messages (they would confuse qwen). The routing happens below.
            ayesha_text = first_result.content;
            let attempted_tool_call = ayesha_text.contains("<function=")
                || ayesha_text.contains("{\"name\":")
                || ayesha_text.contains("\"action\":")
                || ayesha_text.contains("write_file(")
                || ayesha_text.contains("read_file(")
                || ayesha_text.contains("list_dir(")
                || ayesha_text.contains("manage_applet(");

            // Track whether ayesha's text is sitting in the history. When it was
            // a pseudo-tool-call we hold it back and route to qwen; if qwen then
            // declines to call tools we must still surface the text (stripped of
            // the tool-ish syntax) so the user isn't left with silence.
            if attempted_tool_call {
                ui::show_system("ayesha attempted tool call — routing to qwen");
                needs_tools = true;
                // Don't push the raw bad text to messages yet
            } else if !ayesha_text.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: ayesha_text.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                });
                ayesha_text_pushed = true;
            }
        }

        // Step 2: If ayesha didn't call tools, check if kilo-auto/free would
        // Skip for pure-chit-chat to save ~2s latency per message.
        if !steer_happened && !first_had_tools && needs_tools {
            // Try kilo-auto/free with tools to see if it wants to call any
            // (invisible — tool model deliberation is not shown to user)
            // IMPORTANT: strip ayesha's system prompt ("never output tool calls")
            // and replace with a tool-friendly instruction for the tool model.
            let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let desktop = dirs::desktop_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let tool_friendly_prompt = format!(
                "you are a tool-calling assistant. analyze the conversation and call the appropriate tools to fulfill the user's request. \
                you MUST output tool calls using the function calling format. do NOT output plain text responses — only tool calls. \
                call tools EXACTLY ONCE — do NOT call tools repeatedly in a loop. one round of tool calls is enough. \
                always use ABSOLUTE paths. the user's home is {home}, their desktop is {desktop}. \
                if the user says 'on my desktop', use path like {desktop}\\filename.html. \
                if no location specified, use {home}\\filename.html."
            );
            let tool_messages: Vec<ChatMessage> = messages.iter().enumerate().filter_map(|(i, m)| {
                if i == 0 && m.role == "system" {
                    Some(ChatMessage {
                        role: "system".to_string(),
                        content: tool_friendly_prompt.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                    })
                } else {
                    Some(m.clone())
                }
            }).collect();
            let qwen_result = tool_client
                .chat_stream_collect(&tool_messages, Some(tools), &steer_rx)
                .await;

            match qwen_result {
                Ok(qr) if qr.has_tool_calls() => {
                    // qwen wants to call tools — execute them
                    // Note: qwen's content is deliberation text, not user-facing.
                    // Only push tool_calls, not content, to avoid polluting history.
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: String::new(),
                        tool_calls: Some(qr.tool_calls.clone()),
                        tool_call_id: None,
                    });

                    let mut tool_iterations = 0;
                    let mut wrote_file = false;
                    loop {
                        tool_iterations += 1;
                        if tool_iterations > 3 {
                            ui::show_error("max tool iterations (3). stopping.");
                            // Add synthetic tool results so message history is valid
                            let last_tcs = messages.last()
                                .and_then(|m| m.tool_calls.as_ref())
                                .cloned()
                                .unwrap_or_default();
                            for tc in &last_tcs {
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: "error: max tool iterations reached".to_string(),
                                    tool_calls: None,
                                    tool_call_id: Some(tc.id.clone()),
                                });
                            }
                            break;
                        }

                        // Execute all tool calls from this iteration
                        let current_tool_calls = messages.last()
                            .and_then(|m| m.tool_calls.as_ref())
                            .cloned()
                            .unwrap_or_default();

                        if current_tool_calls.is_empty() {
                            break;
                        }

                        for tool_call in &current_tool_calls {
                            let name = &tool_call.function.name;
                            let args = &tool_call.function.arguments;
                            let args_str = serde_json::to_string(args).unwrap_or_default();

                            ui::show_tool_call(name, &args_str);

                            // Plan-mode gate: mutating tools (and shell-backed
                            // plugin tools) are denied outright — plan mode is
                            // read-only.
                            if current_mode == Mode::Plan
                                && (crate::tools::is_mutating(name) || plugin_registry.has_tool(name))
                            {
                                let deny = crate::tools::plan_mode_deny_message(name).unwrap_or_else(|| {
                                    format!(
                                        "error: plan mode: plugin tool '{}' is denied (plan mode is read-only research)",
                                        name
                                    )
                                });
                                ui::show_tool_err(name, &deny);
                                prompt_history.record_usage(name, false, Some(deny.clone()), &args_str);
                                let _ = prompt_history.save();
                                messages.push(ChatMessage {
                                    role: "tool".to_string(),
                                    content: deny,
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id.clone()),
                                });
                                continue;
                            }

                            // Permission gate: sensitive tools need the user's ok.
                            // Plugin tools count as sensitive too — they run
                            // arbitrary shell commands, so they must never skip
                            // the prompt in build/auto mode (unless the user
                            // set an override).
                            match permissions::decide_with(name, &config, |n| plugin_registry.has_tool(n)) {
                                Verdict::Allow => {}
                                Verdict::Denied(msg) => {
                                    ui::show_tool_err(name, &msg);
                                    prompt_history.record_usage(name, false, Some(msg.clone()), &args_str);
                                    let _ = prompt_history.save();
                                    memory.add_memory(
                                        "error",
                                        &format!("tool '{}' blocked by user permission: {}", name, msg),
                                        vec![name.to_string(), "error".to_string()],
                                        3,
                                    );
                                    messages.push(ChatMessage {
                                        role: "tool".to_string(),
                                        content: msg,
                                        tool_calls: None,
                                        tool_call_id: Some(tool_call.id.clone()),
                                    });
                                    continue;
                                }
                                Verdict::Prompt => {
                                    match ask_permission(name, &args_str, &steer_rx, &mut config, &config_path, std::time::Duration::from_secs(120)) {
                                        AskResult::Allow => {}
                                        AskResult::Deny(msg) => {
                                            ui::show_tool_err(name, &msg);
                                            prompt_history.record_usage(name, false, Some(msg.clone()), &args_str);
                                            let _ = prompt_history.save();
                                            messages.push(ChatMessage {
                                                role: "tool".to_string(),
                                                content: msg,
                                                tool_calls: None,
                                                tool_call_id: Some(tool_call.id.clone()),
                                            });
                                            continue;
                                        }
                                        AskResult::Abort(input) => {
                                            ui::show_interrupted();
                                            // Don't leave the assistant
                                            // message with an orphaned tool
                                            // call: synthetic result keeps
                                            // history valid.
                                            messages.push(ChatMessage {
                                                role: "tool".to_string(),
                                                content: "tool call cancelled".to_string(),
                                                tool_calls: None,
                                                tool_call_id: Some(tool_call.id.clone()),
                                            });
                                            if !input.is_empty() {
                                                pending_input = Some(input);
                                            } else {
                                                ui::show_system("tool call cancelled");
                                            }
                                            steer_happened = true;
                                            break;
                                        }
                                    }
                                }
                            }

                            let tool_outcome = run_tool_with_steer(
                                executor.execute(name, args, &mut ToolContext {
                                    memory: &mut memory,
                                    prompt_history: &mut prompt_history,
                                    analyzer: &analyzer,
                                    evolver: &evolver,
                        llm: &tool_llm,
                        backend: &client,
                        project_root: &project_root,
                                    applet_manager: &mut manager,
                                    steer_tx: &steer_tx,
                                    steer_rx: &steer_rx,
                                    input_flag: &mut input_flag,
                                    menu_flag: &menu_flag,
                                }),
                                &steer_rx,
                            ).await;

                            let tool_result = match tool_outcome {
                                ToolOutcome::Done(Ok(r)) => r,
                                ToolOutcome::Done(Err(e)) => {
                                    let err_msg = format!("error: {}", e);
                                    prompt_history.record_usage(name, false, Some(err_msg.clone()), &args_str);
                                    let _ = prompt_history.save();
                                    err_msg
                                }
                                ToolOutcome::Interrupted(input) => {
                                    ui::show_interrupted();
                                    // Synthetic tool result keeps the history
                                    // valid — the assistant message holds this
                                    // tool_call_id.
                                    messages.push(ChatMessage {
                                        role: "tool".to_string(),
                                        content: "tool call cancelled".to_string(),
                                        tool_calls: None,
                                        tool_call_id: Some(tool_call.id.clone()),
                                    });
                                    pending_input = Some(input);
                                    steer_happened = true;
                                    break;
                                }
                            };

                            if !tool_result.starts_with("error:") {
                                prompt_history.record_usage(name, true, None, &args_str);
                            } else {
                                memory.add_memory(
                                    "error",
                                    &format!("tool '{}' failed: {}", name, tool_result),
                                    vec![name.to_string(), "error".to_string()],
                                    3,
                                );
                            }

                            if tool_result.starts_with("error:") {
                                ui::show_tool_err(name, &tool_result);
                            } else {
                                ui::show_tool_ok(name, &tool_result);
                            }

                            // Check if a file-writing tool succeeded BEFORE moving tool_result
                            let is_file_tool = name == "generate_html" || name == "write_file"
                                || name == "generate_sprite" || name == "generate_tileset"
                                || name == "generate_object" || name == "render_sprite";
                            let tool_succeeded = !tool_result.starts_with("error:") && is_file_tool;

                            let tool_result = truncate_tool_result(&tool_result, 8192);

                            messages.push(ChatMessage {
                                role: "tool".to_string(),
                                content: tool_result,
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                            });

                            if tool_succeeded {
                                wrote_file = true;
                            }
                        }

                        // If a file was written, we're done — skip re-prompting qwen
                        if wrote_file {
                            break;
                        }

                        // Interrupted mid-tool — skip the re-prompt, the
                        // steer_happened guard below redirects the whole turn.
                        if steer_happened {
                            break;
                        }

                        // Re-prompt kilo-auto/free for next tool calls (invisible)
                        let tool_messages2: Vec<ChatMessage> = messages.iter().enumerate().filter_map(|(i, m)| {
                            if i == 0 && m.role == "system" {
                                Some(ChatMessage {
                                    role: "system".to_string(),
                                    content: tool_friendly_prompt.to_string(),
                                    tool_calls: None,
                                    tool_call_id: None,
                                })
                            } else {
                                Some(m.clone())
                            }
                        }).collect();
                        let next_result = tool_client
                            .chat_stream_collect(&tool_messages2, Some(tools), &steer_rx)
                            .await;

                        match next_result {
                            Ok(nr) => {
                                if nr.was_steered() {
                                    ui::show_interrupted();
                                    pending_input = nr.steering;
                                    steer_happened = true;
                                    break;
                                }
                                if nr.has_tool_calls() {
                                    messages.push(ChatMessage {
                                        role: "assistant".to_string(),
                                        content: String::new(),
                                        tool_calls: Some(nr.tool_calls.clone()),
                                        tool_call_id: None,
                                    });
                                    // Continue loop to execute these tool calls
                                } else {
                                    // qwen is done calling tools — get ayesha's final response
                                    if !nr.content.is_empty() {
                                        messages.push(ChatMessage {
                                            role: "assistant".to_string(),
                                            content: nr.content,
                                            tool_calls: None,
                                            tool_call_id: None,
                                        });
                                    }
                                    break;
                                }
                            }
                            Err(e) => {
                                ui::show_error(&format!("tool model error: {}", e));
                                // Add synthetic tool results so message history is valid
                                let last_tcs = messages.last()
                                    .and_then(|m| m.tool_calls.as_ref())
                                    .cloned()
                                    .unwrap_or_default();
                                for tc in &last_tcs {
                                    messages.push(ChatMessage {
                                        role: "tool".to_string(),
                                        content: format!("error: tool model failed: {}", e),
                                        tool_calls: None,
                                        tool_call_id: Some(tc.id.clone()),
                                    });
                                }
                                break;
                            }
                        }
                    }

                    // Final response from ayesha
                    if !steer_happened {
                        let final_result = client
                            .chat_stream_visible(&messages, Some(tools), &steer_rx)
                            .await;

                        if let Ok(r) = final_result {
                            if r.was_steered() {
                                // N2: the FINAL stream was steered — route the
                                // steer back and drop partial content (the
                                // truncation guard resets messages).
                                ui::show_interrupted();
                                pending_input = r.steering;
                                steer_happened = true;
                            } else if !r.content.is_empty() {
                                messages.push(ChatMessage {
                                    role: "assistant".to_string(),
                                    content: r.content,
                                    tool_calls: None,
                                    tool_call_id: None,
                                });
                            }
                        }
                    }
                }
                Ok(_) => {
                    // qwen didn't call tools either — ayesha's response is final.
                    // If it was held back (pseudo-tool-call), surface it now
                    // (stripped of the tool-ish syntax) so the user isn't silent.
                    if !ayesha_text_pushed && !ayesha_text.is_empty() {
                        let cleaned = strip_pseudo_tool_syntax(&ayesha_text);
                        if !cleaned.is_empty() {
                            messages.push(ChatMessage {
                                role: "assistant".to_string(),
                                content: cleaned,
                                tool_calls: None,
                                tool_call_id: None,
                            });
                        }
                    }
                }
                Err(_) => {
                    // qwen failed — ayesha's response is final. Same handling.
                    if !ayesha_text_pushed && !ayesha_text.is_empty() {
                        let cleaned = strip_pseudo_tool_syntax(&ayesha_text);
                        if !cleaned.is_empty() {
                            messages.push(ChatMessage {
                                role: "assistant".to_string(),
                                content: cleaned,
                                tool_calls: None,
                                tool_call_id: None,
                            });
                        }
                    }
                }
            }
        }

        if steer_happened {
            messages.truncate(msg_count_before);
            continue;
        }

        let _ = memory.save();
        let _ = prompt_history.save();
        let _ = session::save_session(&project_root, session::DEFAULT_SESSION, &messages);
    }

    Ok(())
}

/// Headless single-turn agent test — sends one message through the full tool
/// pipeline (needs_tools → tool model → execute) and prints what happened.
/// Exit 0 if a tool was executed successfully, exit 1 otherwise.
/// Called via `ayesha-os --headless "message"`.
async fn run_headless(message: &str) -> anyhow::Result<()> {
    let mut manager = AppletManager::new();
    let sandbox = Sandbox::default_workspace().with_sandbox(manager.sandbox);
    let executor = ToolExecutor::new(sandbox.clone());
    let client = LlmClient::new("ayesha");
    let tool_client = LlmClient::new("kilo-auto/free");
    let backend = ActiveBackend::Local(client.clone());
    let tools = tool_defs::tool_definitions_core();
    let tools = streaming::tool_payload_slice(&tools);
    let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();
    let mut memory = memory::MemoryStore::load();
    let mut prompt_history = PromptHistory::load();
    let analyzer = SelfAnalyzer::new(std::env::current_dir().unwrap_or_default());
    let evolver = ToolEvolver::new(
        tool_defs::known_tool_names().into_iter().map(|s| s.to_string()).collect()
    );
    let project_root = std::env::current_dir().unwrap_or_default();
    let mut input_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let menu_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Build the tool-friendly system prompt (same as main loop)
    let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let desktop = dirs::desktop_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let system_prompt = format!(
        "you are a tool-calling assistant. analyze the conversation and call the appropriate tools to fulfill the user's request. \
        you MUST output tool calls using the function calling format. do NOT output plain text responses — only tool calls. \
        always use ABSOLUTE paths. the user's home is {home}, their desktop is {desktop}. \
        if the user says 'on my desktop', use path like {desktop}\\filename.html. \
        if no location specified, use {home}\\filename.html."
    );

    let mut messages = vec![
        ChatMessage { role: "system".to_string(), content: system_prompt.clone(), tool_calls: None, tool_call_id: None },
        ChatMessage { role: "user".to_string(), content: message.to_string(), tool_calls: None, tool_call_id: None },
    ];

    let mut ctx = ToolContext {
        memory: &mut memory,
        prompt_history: &mut prompt_history,
        analyzer: &analyzer,
        evolver: &evolver,
        llm: &client,
        backend: &backend,
        project_root: &project_root,
        applet_manager: &mut manager,
        steer_tx: &steer_tx,
        steer_rx: &steer_rx,
        input_flag: &mut input_flag,
        menu_flag: &menu_flag,
    };

    // Step 1: try qwen directly with tools
    let qwen_result = tool_client.chat_stream_collect(&messages, Some(tools), &steer_rx).await;

    let mut tools_executed = false;
    match qwen_result {
        Ok(qr) if qr.has_tool_calls() => {
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: String::new(),
                tool_calls: Some(qr.tool_calls.clone()),
                tool_call_id: None,
            });
            for tc in &qr.tool_calls {
                let name = tc.function.name.clone();
                let args_str = serde_json::to_string(&tc.function.arguments).unwrap_or_default();
                println!("  ▶ {name} {}", truncate_tool_result(&args_str, 200));
                let result = executor.execute(&name, &tc.function.arguments, &mut ctx).await;
                match result {
                    Ok(r) => {
                        println!("  ✓ {name} succeeded ({} chars)", r.len());
                        tools_executed = true;
                    }
                    Err(e) => println!("  ✖ {name} error: {e}"),
                }
            }
        }
        Ok(_) => println!("  ⚠ tool model returned no tool calls"),
        Err(e) => println!("  ✖ tool model error: {e}"),
    }

    let _ = memory.save();
    manager.stop_all();
    if tools_executed {
        println!("\n  ✓ headless test passed");
        std::process::exit(0);
    } else {
        println!("\n  ✖ headless test failed — no tools executed");
        std::process::exit(1);
    }
}

/// Headless E2E smoke test — verifies llm is reachable, both models respond,
/// and basic tool execution works. Called via `ayesha-os --selftest`.
async fn selftest() -> anyhow::Result<()> {
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut checks: Vec<(String, bool)> = Vec::new();

    let check = |name: &str, result: bool, checks: &mut Vec<(String, bool)>| {
        checks.push((name.to_string(), result));
        if result {
            print!("  \x1b[32m✓\x1b[0m {}", name);
        } else {
            print!("  \x1b[31m✗\x1b[0m {}", name);
        }
        println!();
    };

    println!("\n  \x1b[36m◆ ayesha-os selftest\x1b[0m\n");

    // 1. Cloud reachable
    let llm_ok = LlmClient::list_models().await.is_ok();
    check("llm reachable at kilo gateway", llm_ok, &mut checks);

    if !llm_ok {
        println!("\n  \x1b[31mobort: llm not reachable\x1b[0m\n");
        std::process::exit(1);
    }

    // 2. Ayesha model responds
    let ayesha = LlmClient::new("ayesha");
    let (_, rx) = std::sync::mpsc::channel::<String>();
    let msgs = vec![ChatMessage {
        role: "user".to_string(),
        content: "say hi in 3 words".to_string(),
        tool_calls: None,
        tool_call_id: None,
    }];
    let ayesha_resp = ayesha.chat_stream_visible(&msgs, None, &rx).await;
    let ayesha_ok = ayesha_resp.as_ref().map(|r| !r.content.is_empty()).unwrap_or(false);
    check(
        &format!("ayesha model responds{}", if let Err(e) = &ayesha_resp {
            format!(" (error: {})", e)
        } else { String::new() }),
        ayesha_ok,
        &mut checks,
    );

    // 3. Qwen model responds
    let qwen = LlmClient::new("kilo-auto/free");
    let qwen_resp = qwen.chat_stream_collect(&msgs, None, &rx).await;
    let qwen_ok = qwen_resp.as_ref().map(|r| !r.content.is_empty()).unwrap_or(false);
    check(
        &format!("kilo-auto/free model responds{}", if let Err(e) = &qwen_resp {
            format!(" (error: {})", e)
        } else { String::new() }),
        qwen_ok,
        &mut checks,
    );

    // 4. Tool definitions parse
    let tools = tool_defs::tool_definitions();
    let tools = streaming::tool_payload_slice(&tools);
    let tools_ok = !tools.is_empty();
    check(&format!("{} tool definitions loaded", tools.len()), tools_ok, &mut checks);

    // 5. Qwen emits tool_calls when tools provided (regression guard)
    let tool_msgs = vec![ChatMessage {
        role: "user".to_string(),
        content: "read main.rs".to_string(),
        tool_calls: None,
        tool_call_id: None,
    }];
    let qwen_tool_resp = qwen.chat_stream_collect(&tool_msgs, Some(tools), &rx).await;
    let qwen_tool_ok = qwen_tool_resp.as_ref().map(|r| r.has_tool_calls()).unwrap_or(false);
    check(
        &format!("kilo-auto/free emits tool_calls{}", if let Err(e) = &qwen_tool_resp {
            format!(" (error: {})", e)
        } else { String::new() }),
        qwen_tool_ok,
        &mut checks,
    );

    // 6. Truncate tool result
    let trunc_ok = util::truncate_chars("hello world", 5) == "hello";
    check("truncate_chars works", trunc_ok, &mut checks);

    // 7. Shared stream decoders work
    let mut decoder = streaming::CloudDecoder::new();
    let events = decoder.feed(
        br#"{"message":{"content":"test"},"done":false}
{"message":{"content":" ok"},"done":false}
{"message":{"content":""},"done":true}
"#,
    );
    let content: String = events
        .iter()
        .filter_map(|e| match e {
            streaming::StreamEvent::Chunk(c) => Some(c.as_str()),
            _ => None,
        })
        .collect();
    let done_seen = events.iter().any(|e| matches!(e, streaming::StreamEvent::Done));
    let parser_ok = content == "test ok" && done_seen;
    check("stream decoders accumulate content", parser_ok, &mut checks);

    // 8. needs_tools heuristic
    let needs_tools_ok = agent::needs_tools("read main.rs") && !agent::needs_tools("hi");
    check("needs_tools heuristic", needs_tools_ok, &mut checks);

    // Summary
    for (_, ok) in &checks {
        if *ok { pass += 1; } else { fail += 1; }
    }
    println!();
    if fail == 0 {
        println!("  \x1b[32mall {} checks passed ✓\x1b[0m\n", pass);
    } else {
        println!("  \x1b[33m{} passed, \x1b[31m{} failed\x1b[0m\n", pass, fail);
    }

    if fail > 0 { std::process::exit(1); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn applet_phrase_matches() {
        let n = names(&["flora-cli", "hivebeat", "engine"]);

        assert_eq!(parse_applet_phrase(&n, "launch flora cli"), Some(("flora-cli".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "launch flora-cli"), Some(("flora-cli".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "open Hive Beat"), Some(("hivebeat".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "start hivebeat"), Some(("hivebeat".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "run engine"), Some(("engine".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "stop flora-cli"), Some(("flora-cli".to_string(), true)));
        assert_eq!(parse_applet_phrase(&n, "launch flora"), None);
        assert_eq!(parse_applet_phrase(&n, "launch a test for me"), None);
        assert_eq!(parse_applet_phrase(&n, "what is the weather"), None);
    }

    #[test]
    fn model_ctx_labels_are_compact() {
        assert_eq!(model_ctx_label(1_000_000), "1m");
        assert_eq!(model_ctx_label(200_000), "200k");
        assert_eq!(model_ctx_label(131_072), "131k");
        assert_eq!(model_ctx_label(65_536), "65k");
        assert_eq!(model_ctx_label(32_768), "32k");
        assert_eq!(model_ctx_label(8_192), "8k");
        assert_eq!(model_ctx_label(4096), "4k");
    }

    #[tokio::test]
    async fn model_switch_rebuilds_llm_client() {
        let mut registry = ModelRegistry::new();
        let mut current_model = "kilo-auto/free".to_string();
        let mut client = ActiveBackend::Local(LlmClient::new("ayesha"));
        apply_model_switch(&mut registry, &mut current_model, &mut client, "kilo-auto/free", "kilo-auto/free").await;
        assert_eq!(current_model, "kilo-auto/free");
        assert!(matches!(client, ActiveBackend::Local(_)));
        assert!(registry.models.iter().any(|m| m.name == "kilo-auto/free"));
    }

    #[tokio::test]
    async fn model_switch_unknown_model_keeps_current() {
        let mut registry = ModelRegistry::new();
        let mut current_model = "kilo-auto/free".to_string();
        let mut client = ActiveBackend::Local(LlmClient::new("ayesha"));
        apply_model_switch(&mut registry, &mut current_model, &mut client, "no-such-model", "kilo-auto/free").await;
        assert_eq!(current_model, "kilo-auto/free");
        assert!(matches!(client, ActiveBackend::Local(_)));
    }

    #[test]
    fn strip_pseudo_tool_removes_fences() {
        let input = "sure, here you go:\n```tool_call\n{\"name\":\"write_file\"}\n```\ndone!";
        let cleaned = strip_pseudo_tool_syntax(input);
        assert!(!cleaned.contains("```"));
        assert!(!cleaned.contains("write_file"));
        assert!(cleaned.contains("sure"));
        assert!(cleaned.contains("done"));
    }

    #[test]
    fn strip_pseudo_tool_removes_json_blocks() {
        let input = "I'll read it. {\"name\": \"read_file\", \"arguments\": {\"path\": \"x\"}} On it.";
        let cleaned = strip_pseudo_tool_syntax(input);
        assert!(!cleaned.contains("{\"name\""));
        assert!(cleaned.contains("I'll read it"));
        assert!(cleaned.contains("On it"));
    }

    #[test]
    fn strip_pseudo_tool_leaves_plain_text() {
        let input = "just a normal chat reply, nothing to see here";
        assert_eq!(strip_pseudo_tool_syntax(input), input);
    }

    #[test]
    fn mode_parses_and_roundtrips() {
        assert_eq!(Mode::from_str("plan"), Some(Mode::Plan));
        assert_eq!(Mode::from_str("build"), Some(Mode::Build));
        assert_eq!(Mode::from_str("auto"), Some(Mode::Auto));
        assert_eq!(Mode::from_str("PLAN"), Some(Mode::Plan));
        assert_eq!(Mode::from_str("banana"), None);
        assert_eq!(Mode::from_str(""), None);
        assert_eq!(Mode::Plan.as_str(), "plan");
        assert_eq!(Mode::Build.as_str(), "build");
        assert_eq!(Mode::Auto.as_str(), "auto");
    }

    #[test]
    fn system_prompt_includes_mode_directive() {
        let root = std::path::Path::new(".");
        let plan_prompt = build_system_prompt_full("tester", root, Mode::Plan, "");
        assert!(plan_prompt.contains("PLAN MODE"));
        let build_prompt = build_system_prompt_full("tester", root, Mode::Build, "");
        assert!(build_prompt.contains("### mode: build"));
        let auto_prompt = build_system_prompt_full("tester", root, Mode::Auto, "");
        assert!(auto_prompt.contains("### mode: auto"));
        let with_plugins = build_system_prompt_full("tester", root, Mode::Auto, "\n### plugins\nuse plugin x");
        assert!(with_plugins.contains("use plugin x"));
    }

    #[test]
    fn parse_memories_remember() {
        let mut mem = crate::memory::MemoryStore::default();
        let result = parse_and_store_memories("hello [REMEMBER: user likes cats] world", &mut mem);
        assert_eq!(result, "hello  world");
        assert_eq!(mem.memories.len(), 1);
        assert_eq!(mem.memories[0].category, "user_pref");
        assert_eq!(mem.memories[0].content, "user likes cats");
    }

    #[test]
    fn parse_memories_preference() {
        let mut mem = crate::memory::MemoryStore::default();
        let result = parse_and_store_memories("set [PREFERENCE: theme = dark] now", &mut mem);
        assert_eq!(result, "set  now");
        assert_eq!(mem.user_preferences.get("theme"), Some(&"dark".to_string()));
    }

    #[test]
    fn parse_memories_fact() {
        let mut mem = crate::memory::MemoryStore::default();
        let result = parse_and_store_memories("know [FACT: the sky is blue] please", &mut mem);
        assert_eq!(result, "know  please");
        assert_eq!(mem.memories.len(), 1);
        assert_eq!(mem.memories[0].category, "fact");
    }

    #[test]
    fn parse_memories_multiple() {
        let mut mem = crate::memory::MemoryStore::default();
        let result = parse_and_store_memories(
            "[REMEMBER: a] text [FACT: b] more [PREFERENCE: x = y] end", &mut mem
        );
        assert!(result.contains("text"));
        assert!(result.contains("more"));
        assert!(result.contains("end"));
        assert_eq!(mem.memories.len(), 2);
        assert_eq!(mem.user_preferences.get("x"), Some(&"y".to_string()));
    }

    #[test]
    fn parse_memories_missing_closing() {
        let mut mem = crate::memory::MemoryStore::default();
        let result = parse_and_store_memories("hello [REMEMBER: no close", &mut mem);
        assert_eq!(result, "hello [REMEMBER: no close");
        assert_eq!(mem.memories.len(), 0);
    }

    #[test]
    fn parse_memories_empty_marker() {
        let mut mem = crate::memory::MemoryStore::default();
        let result = parse_and_store_memories("hello [REMEMBER:] world", &mut mem);
        assert_eq!(result, "hello  world");
        assert_eq!(mem.memories.len(), 0);
    }

    // ── ask_permission (Gate C hardening) ──

    fn permission_test_config_path(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("ask_permission_{}_{}.json", name, std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn ask_permission_times_out_and_denies() {
        // A live-but-silent steer channel with a short timeout must deny the
        // tool call (with a "timed out" tool error), not hang forever.
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let mut config = serde_json::json!({});
        let path = permission_test_config_path("timeout");
        let result = ask_permission(
            "write_file",
            r#"{"path": "x"}"#,
            &rx,
            &mut config,
            &path,
            std::time::Duration::from_millis(80),
        );
        drop(tx);
        let _ = std::fs::remove_file(&path);
        match result {
            AskResult::Deny(msg) => {
                assert!(msg.contains("timed out"), "expected timed-out deny, got: {}", msg);
                assert!(msg.contains("write_file"));
            }
            other => panic!("expected Deny on timeout, got {:?}", other),
        }
    }

    #[test]
    fn ask_permission_steering_text_returns_as_abort() {
        // A line that isn't an exact keyword is steering, not an answer: the
        // prompt must be cancelled and the raw text handed back so the caller
        // routes it into pending_input (it must NOT be auto-answered).
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let mut config = serde_json::json!({});
        let path = permission_test_config_path("steer");
        tx.send("hold on, let me think about this first".to_string()).unwrap();
        let result = ask_permission(
            "write_file",
            r#"{"path": "x"}"#,
            &rx,
            &mut config,
            &path,
            std::time::Duration::from_secs(10),
        );
        let _ = std::fs::remove_file(&path);
        match result {
            AskResult::Abort(text) => {
                assert_eq!(text, "hold on, let me think about this first");
            }
            other => panic!("expected Abort carrying the steering text, got {:?}", other),
        }
    }

    #[test]
    fn ask_permission_exact_keywords_only() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let mut config = serde_json::json!({});
        let path = permission_test_config_path("keywords");

        // exact "n" denies
        tx.send("n".to_string()).unwrap();
        let r1 = ask_permission("write_file", "{}", &rx, &mut config, &path, std::time::Duration::from_secs(10));
        assert!(matches!(r1, AskResult::Deny(_)), "expected Deny for 'n', got {:?}", r1);

        // "abort" soft-aborts
        tx.send("abort".to_string()).unwrap();
        let r2 = ask_permission("write_file", "{}", &rx, &mut config, &path, std::time::Duration::from_secs(10));
        assert!(matches!(r2, AskResult::Abort(ref s) if s.is_empty()), "expected empty Abort for 'abort', got {:?}", r2);

        // ctrl+c aborts carrying the raw control input
        tx.send("\0ctrl-c".to_string()).unwrap();
        let r3 = ask_permission("write_file", "{}", &rx, &mut config, &path, std::time::Duration::from_secs(10));
        assert!(matches!(r3, AskResult::Abort(ref s) if s == "\0ctrl-c"), "expected raw Abort for ctrl-c, got {:?}", r3);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ask_permission_always_persists_override() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let mut config = serde_json::json!({});
        let path = permission_test_config_path("always");
        tx.send("always".to_string()).unwrap();
        let result = ask_permission(
            "write_file",
            r#"{"path": "x"}"#,
            &rx,
            &mut config,
            &path,
            std::time::Duration::from_secs(10),
        );
        assert!(matches!(result, AskResult::Allow), "expected Allow for 'always', got {:?}", result);
        // the override is persisted in memory...
        assert_eq!(config["permissions"]["write_file"], "always");
        // ...and on disk, so the prompt isn't repeated next time
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        assert!(on_disk.contains("\"write_file\""), "config file must contain the override, got: {}", on_disk);
        assert!(on_disk.contains("always"));
    }
}
