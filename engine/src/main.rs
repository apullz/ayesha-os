mod ollama;
mod cloud;
mod tools;
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
mod agent;
mod completion;
mod theme;
mod syntax;
mod render;
mod session;

use std::io::Write;
use theme::Role;
use ollama::{OllamaClient, ChatMessage, StreamResult};
use cloud::CloudClient;
use tools::{ToolExecutor, ToolContext};
use sandbox::Sandbox;
use prompt_refinement::PromptHistory;
use memory::MemoryStore;
use self_analysis::SelfAnalyzer;
use tool_evolution::ToolEvolver;
use model_registry::ModelRegistry;
use applet_manager::AppletManager;


/// Active backend — either local Ollama or cloud (OpenRouter/OpenCode)
enum ActiveBackend {
    Ollama(OllamaClient),
    Cloud(CloudClient),
}

/// Build the system prompt for a user, appending the skills hint if any
/// skills are installed in the project's skills/ folder.
fn build_system_prompt(user_name: &str, project_root: &std::path::Path) -> String {
    let mut prompt = OllamaClient::system_prompt(user_name);
    if let Some(hint) = skills::system_prompt_hint(project_root) {
        prompt.push_str(&hint);
    }
    prompt
}

/// Heuristic: does this user message likely need tool calls?
fn might_need_tools(msg: &str) -> bool {
    agent::needs_tools(msg)
}

/// Truncate tool results to prevent context overflow
fn truncate_tool_result(result: &str, max_chars: usize) -> String {
    agent::truncate_tool_result(result, max_chars)
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
            ActiveBackend::Ollama(c) => c.chat_stream_visible(messages, tools, steer_rx).await,
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
            ActiveBackend::Ollama(c) => c.chat_stream_collect(messages, tools, steer_rx).await,
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
                .unwrap_or("apullz/ayesha-os").to_string();
            let branch = g.and_then(|g| g.get("branch")).and_then(|b| b.as_str())
                .unwrap_or("master").to_string();
            let auto_push = g.and_then(|g| g.get("auto_push")).and_then(|b| b.as_bool())
                .unwrap_or(true);
            return (repo, branch, auto_push);
        }
    }
    ("apullz/ayesha-os".to_string(), "master".to_string(), true)
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
/// desktop-cat", or "stop poopy-tui" to a registered applet name.
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
    let _ = crossterm::terminal::disable_raw_mode();
    println!();
    println!("  {} {}", theme::paint(Role::Success, "●"), theme::paint(Role::Accent, "ayesha-os shutting down"));
    println!("  {} {}", theme::paint(Role::Accent, "◆"), theme::paint(Role::Dim, format!("saved {}", memory.summary())));
    println!();
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
    theme::apply_no_color();
    theme::load_from_config(None);

    let session_start = std::time::Instant::now();
    let sandbox = Sandbox::default_workspace();
    let mut manager = AppletManager::new();
    let executor = ToolExecutor::new(sandbox);
    let mut client = ActiveBackend::Ollama(OllamaClient::new("ayesha"));
    let mut tool_client = ActiveBackend::Ollama(OllamaClient::new("qwen2.5:7b"));
    let mut tool_model_name = "qwen2.5:7b".to_string();
    let mut memory = MemoryStore::load();
    let mut prompt_history = PromptHistory::load();

    // Self-analysis, tool evolution, and a separate ollama client for generative tools
    let project_root = std::env::current_dir().unwrap_or_default();
    let analyzer = SelfAnalyzer::new(project_root.clone());
    let tool_ollama = OllamaClient::new("ayesha");
    let evolver = ToolEvolver::new(vec![
        "read_file".into(), "write_file".into(), "list_dir".into(),
        "grep".into(), "glob".into(), "list_skills".into(), "read_skill".into(),
        "generate_html".into(), "generate_sprite".into(), "generate_tileset".into(),
        "generate_object".into(), "render_sprite".into(), "read_clipboard".into(),
        "remember".into(), "list_memories".into(), "search_memories".into(),
        "set_preference".into(), "analyze_self".into(), "list_source_files".into(),
        "evolve_tools".into(), "refine_prompt".into(), "get_tool_stats".into(),
        "coding_agent".into(), "manage_applet".into(),
    ]);

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
    // Default ON — mirrors the opencode lowercase-proxy. Disable with
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

    // Enable raw mode for Ctrl+M detection
    let _ = crossterm::terminal::enable_raw_mode();

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
    ].into_iter().map(String::from).collect();
    for name in manager.names() {
        completion_candidates.push(name);
    }

    let menu_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut input_flag = applet_runner::spawn_input_thread(steer_tx.clone(), completion_candidates, menu_flag.clone());

    ui::print_banner();
    ui::show_system(&memory.summary());

    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage {
            role: "system".to_string(),
            content: build_system_prompt(&user_name, &project_root),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    // Resume last session? (config: "resume_prompt": false to disable the prompt)
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

    let tools = OllamaClient::tool_definitions_core();
    let mut current_model = "ayesha:latest".to_string();
    ui::show_system(&format!("chat model: {} | tool model: {}", current_model, tool_model_name));

    // Warm up ollama — preload both models into memory so first user interaction is fast.
    // Runs off the boot path: the prompt appears instantly while models preload in the
    // background. Set "boot_warmup": false in config.json to disable entirely.
    if config.get("boot_warmup").and_then(|v| v.as_bool()).unwrap_or(true) {
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
                        let client = OllamaClient::new("ayesha");
                        let msgs = vec![ChatMessage { role: "user".to_string(), content: "hi".to_string(), tool_calls: None, tool_call_id: None }];
                        let _ = client.chat_stream_collect(&msgs, None, &warmup_rx).await;
                    },
                    async {
                        let client = OllamaClient::new(&tool_model);
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

    // Build the applet page list once at startup; statuses refresh on each menu open.
    let mut menu_pages: Vec<(String, String, bool, bool)> = Vec::new();
    {
        menu_pages.push(("engine".to_string(), "terminal persona host — that's me".to_string(), true, true));
        for name in manager.names() {
            if name == "engine" { continue; }
            if let Some(e) = manager.entries.get(&name) {
                menu_pages.push((name.clone(), e.desc.clone(), manager.is_running(&name), e.foreground));
            }
        }
    }

    loop {
        // ── read user input ──
        let input = if let Some(p) = pending_input.take() {
            p
        } else {
            match input_mode {
                InputMode::Normal => ui::prompt_line(),
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
                    let _ = write!(std::io::stdout(), "\x1B[u\x1B[J");
                    let _ = std::io::stdout().flush();
                }
                continue;
            }
            inp
        };

        // ── control keys (work from normal input and steering interrupts) ──
        // Ctrl+C → exit (graceful: save memory, stop applets, release raw mode)
        if input == "\0ctrl-c" {
            menu_flag.store(false, std::sync::atomic::Ordering::Relaxed);
            graceful_shutdown(&mut messages, &mut memory, &mut prompt_history, &mut manager, &project_root);
            break;
        }
        // Ctrl+M / Ctrl+P → open interactive applet menu
        if input == "\0ctrl-m" || input == "\0ctrl-p" {
            input_mode = InputMode::AppletMenu;
            // Refresh statuses in the page list
            for entry in menu_pages.iter_mut() {
                if entry.0 == "engine" {
                    entry.2 = true;
                } else {
                    entry.2 = manager.is_running(&entry.0);
                }
            }
            let mut menu_idx: usize = 0;
            let mut menu_filter: String = String::new();
            // Drain stale keystrokes queued before the menu opened
            while steer_rx.try_recv().is_ok() {}
            menu_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            // Save cursor once before the menu; each redraw restores to this point.
            let _ = write!(std::io::stdout(), "\x1B[s");
            let (rendered, _line_count) = ui::draw_applet_menu(&menu_pages, menu_idx, &menu_filter);
            let _ = write!(std::io::stdout(), "{}", rendered);
            let _ = std::io::stdout().flush();

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
                        graceful_shutdown(&mut messages, &mut memory, &mut prompt_history, &mut manager, &project_root);
                        return Ok(());
                    }
                    "\0menu-up" => {
                        menu_idx = menu_idx.saturating_sub(1);
                        let _ = write!(std::io::stdout(), "\x1B[u\x1B[J");
                        let (rendered, _lc) = ui::draw_applet_menu(&menu_pages, menu_idx, &menu_filter);
                        let _ = write!(std::io::stdout(), "{}", rendered);
                        let _ = std::io::stdout().flush();
                    }
                    "\0menu-down" => {
                        let filtered_len = menu_pages.iter().filter(|(n, d, _, _)| {
                            if menu_filter.is_empty() { true }
                            else { n.to_lowercase().contains(&menu_filter.to_lowercase()) || d.to_lowercase().contains(&menu_filter.to_lowercase()) }
                        }).count();
                        if filtered_len > 0 && menu_idx + 1 < filtered_len {
                            menu_idx += 1;
                        }
                        let _ = write!(std::io::stdout(), "\x1B[u\x1B[J");
                        let (rendered, _lc) = ui::draw_applet_menu(&menu_pages, menu_idx, &menu_filter);
                        let _ = write!(std::io::stdout(), "{}", rendered);
                        let _ = std::io::stdout().flush();
                    }
                    "\0menu-backspace" => {
                        menu_filter.pop();
                        menu_idx = 0;
                        let _ = write!(std::io::stdout(), "\x1B[u\x1B[J");
                        let (rendered, _lc) = ui::draw_applet_menu(&menu_pages, menu_idx, &menu_filter);
                        let _ = write!(std::io::stdout(), "{}", rendered);
                        let _ = std::io::stdout().flush();
                    }
                    "\0menu-esc" => {
                        break;
                    }
                    "\0menu-enter" => {
                        let filtered: Vec<(String, String, bool, bool)> = menu_pages.iter().filter(|&(n, d, _, _)| {
                            if menu_filter.is_empty() { true }
                            else { n.to_lowercase().contains(&menu_filter.to_lowercase()) || d.to_lowercase().contains(&menu_filter.to_lowercase()) }
                        }).cloned().collect();
                        if let Some((name, _desc, _running, _foreground)) = filtered.get(menu_idx) {
                            selected = Some(name.clone());
                        }
                        break;
                    }
                    s if s.starts_with("\0menu-char:") => {
                        let c = &s["\0menu-char:".len()..];
                        if menu_filter.is_empty() && c == "x" {
                            // Stop action
                            let filtered: Vec<(String, String, bool, bool)> = menu_pages.iter().filter(|&(n, d, _, _)| {
                                if menu_filter.is_empty() { true }
                                else { n.to_lowercase().contains(&menu_filter.to_lowercase()) || d.to_lowercase().contains(&menu_filter.to_lowercase()) }
                            }).cloned().collect();
                            if let Some((name, _, running, _)) = filtered.get(menu_idx) {
                                if *running {
                                    match manager.stop(name) {
                                        Ok(()) => ui::show_system(&format!("stopped {}", name)),
                                        Err(e) => ui::show_error(&e),
                                    }
                                } else {
                                    ui::show_system(&format!("{} is not running", name));
                                }
                                for entry in menu_pages.iter_mut() {
                                    if entry.0 != "engine" {
                                        entry.2 = manager.is_running(&entry.0);
                                    }
                                }
                            }
                        } else {
                            for ch in c.chars() {
                                menu_filter.push(ch);
                            }
                            menu_idx = 0;
                        }
                        let _ = write!(std::io::stdout(), "\x1B[u\x1B[J");
                        let (rendered, _lc) = ui::draw_applet_menu(&menu_pages, menu_idx, &menu_filter);
                        let _ = write!(std::io::stdout(), "{}", rendered);
                        let _ = std::io::stdout().flush();
                    }
                    _ => {}
                }
            }

            // Exit menu: clear region and switch off
            menu_flag.store(false, std::sync::atomic::Ordering::Relaxed);
            let _ = write!(std::io::stdout(), "\x1B[u\x1B[J");
            let _ = std::io::stdout().flush();
            input_mode = InputMode::Normal;

            // If Enter selected something, take action
            if let Some(name) = selected {
                switch_applet_page(&mut manager, &name, &steer_tx, &steer_rx, &mut input_flag, &menu_flag);
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

        // ── meta-commands ──
        match lower.as_str() {
            "exit" | "quit" | "q" => {
                graceful_shutdown(&mut messages, &mut memory, &mut prompt_history, &mut manager, &project_root);
                break;
            }
            "help" | "h" | "?" => {
                ui::print_help();
                continue;
            }
            "clear" | "cls" => {
                print!("\x1B[2J\x1B[1;1H");
                std::io::stdout().flush()?;
                continue;
            }
            "reset" => {
                messages = vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: build_system_prompt(&user_name, &project_root),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                ];
                memory = MemoryStore::load();
                ui::show_system("conversation history and memory cleared");
                continue;
            }
            "models" => {
                println!();
                println!("{}", registry.list_models());
                continue;
            }
            _ if lower == "auto" || lower.starts_with("auto ") => {
                let enabled = !lower.contains("off") && !lower.contains("disable");
                registry.set_auto_route(enabled);
                ui::show_system(if enabled { "auto-routing enabled" } else { "auto-routing disabled" });
                continue;
            }
            "sync" => {
                let (repo, branch, auto_push) = sync_config(&project_root);
                ui::show_system(&format!("initiating tri-mind sync → {repo} ({branch})"));
                let _ = std::process::Command::new("python")
                    .args(["-m", "tri_mind_sync.cli", "sync"])
                    .output();

                if auto_push {
                    let _ = std::process::Command::new("git")
                        .args(["add", "."])
                        .status();
                    let _ = std::process::Command::new("git")
                        .args(["commit", "-m", "ayesha-os: auto sync update"])
                        .status();
                    let push_status = std::process::Command::new("git")
                        .args(["push", "origin", &branch])
                        .status();

                    match push_status {
                        Ok(s) if s.success() => {
                            ui::show_system(&format!("successfully pushed updates to https://github.com/{repo} ({branch}) (๑>◡<๑)"));
                        }
                        _ => {
                            ui::show_error("git push failed (check authentication / branch).");
                        }
                    }
                } else {
                    ui::show_system("auto_push disabled in tri_mind_sync.json — skipped git push");
                }

                show_sync_state(&project_root);
                continue;
            }
            _ if lower == "sync status" || lower.starts_with("sync status ") => {
                show_sync_state(&project_root);
                continue;
            }
            "apps" | "applets" => {
                println!("\n{}", manager.list());
                continue;
            }
            _ if lower.starts_with("run ") => {
                let name = input[4..].trim();
                if manager.has(name) {
                    if manager.is_foreground(name) {
                        match manager.run_in_window(name, &steer_tx, &steer_rx, &mut input_flag, &menu_flag) {
                            Ok(()) => ui::show_system(&format!("returned from {} — press ctrl+p to switch pages again", name)),
                            Err(e) => ui::show_error(&e),
                        }
                    } else if manager.is_running(name) {
                        ui::show_system(&format!("{} is already running", name));
                    } else {
                        match manager.launch(name) {
                            Ok(()) => ui::show_system(&format!("launched {}", name)),
                            Err(e) => ui::show_error(&e),
                        }
                    }
                } else {
                    ui::show_error(&format!("unknown applet: {}", name));
                }
                continue;
            }
            _ if lower.starts_with("stop ") => {
                let name = input[5..].trim();
                match manager.stop(name) {
                    Ok(()) => ui::show_system(&format!("stopped {}", name)),
                    Err(e) => ui::show_error(&e),
                }
                continue;
            }
            _ if lower.starts_with("model ") => {
                let name = input[6..].trim();
                match registry.set_model(name) {
                    Ok(()) => {
                        current_model = name.to_string();
                        if registry.is_cloud_model(name) {
                            let provider = registry.cloud_provider(name).unwrap_or_default();
                            match CloudClient::new(name, &provider) {
                                Ok(cc) => {
                                    client = ActiveBackend::Cloud(cc);
                                    ui::show_system(&format!("switched to cloud model: {} ({})", name, provider));
                                }
                                Err(e) => {
                                    ui::show_error(&format!("cloud setup failed: {}. run .\\scripts\\setup-cloud.ps1", e));
                                    // Revert to default
                                    current_model = "ayesha".to_string();
                                    client = ActiveBackend::Ollama(OllamaClient::new("ayesha"));
                                }
                            }
                        } else {
                            client = ActiveBackend::Ollama(OllamaClient::new(&current_model));
                            ui::show_system(&format!("switched to: {}", name));
                        }
                    }
                    Err(e) => ui::show_error(&e.to_string()),
                }
                registry.detect().await;
                continue;
            }
            _ if lower.starts_with("toolmodel ") || lower == "toolmodel" => {
                let name = input.get(10..).unwrap_or("").trim();
                if registry.is_cloud_model(name) {
                    let provider = registry.cloud_provider(name).unwrap_or_default();
                    match CloudClient::new(name, &provider) {
                        Ok(cc) => {
                            tool_client = ActiveBackend::Cloud(cc);
                            tool_model_name = name.to_string();
                            ui::show_system(&format!("tool model: {} ({})", name, provider));
                        }
                        Err(e) => ui::show_error(&format!("cloud setup failed: {}", e)),
                    }
                } else if registry.models.iter().any(|m| m.name == name) {
                    tool_client = ActiveBackend::Ollama(OllamaClient::new(name));
                    tool_model_name = name.to_string();
                    ui::show_system(&format!("tool model: {}", name));
                } else {
                    ui::show_error(&format!("model '{}' not found. use 'models' to list available models", name));
                }
                continue;
            }
            _ if lower.starts_with("pull ") || lower == "pull" => {
                let name = input.get(5..).unwrap_or("").trim();
                ui::show_system(&format!("run `ollama pull {}` in another terminal, then `models` to refresh", name));
                continue;
            }
            _ if lower.starts_with("name ") || lower == "name" => {
                let name = input.get(5..).unwrap_or("").trim().to_string();
                if name.is_empty() {
                    ui::show_system("usage: /name <you>");
                } else {
                    config["user_name"] = serde_json::json!(name);
                    if let Ok(content) = serde_json::to_string_pretty(&config) {
                        let _ = std::fs::write(&config_path, content);
                    }
                    if !messages.is_empty() {
                        messages[0].content = build_system_prompt(&name, &project_root);
                    }
                    ui::show_system(&format!("okay, {} it is!", name));
                }
                continue;
            }
            _ if lower.starts_with("theme ") || lower == "theme" => {
                let arg = input.get(6..).unwrap_or("").trim();
                if arg.is_empty() {
                    let current = theme::get().name.clone();
                    println!("\n  {}", theme::bold(Role::Accent, format!("themes  (current: {})", current)));
                    for name in theme::names() {
                        let is_active = name == current;
                        println!("  {} {}",
                            theme::paint(if is_active { Role::Primary } else { Role::Dim },
                                if is_active { "▶" } else { " " }),
                            theme::paint(Role::Text, theme::render_swatch(name)));
                    }
                    println!();
                } else {
                    match theme::switch(arg) {
                        Ok(t) => {
                            config["theme"] = serde_json::json!(t.name);
                            if let Ok(content) = serde_json::to_string_pretty(&config) {
                                let _ = std::fs::write(&config_path, content);
                            }
                            ui::show_system(&format!("theme switched to {}", t.name));
                        }
                        Err(e) => ui::show_error(&e),
                    }
                }
                continue;
            }
            "stats" => {
                match executor.execute("get_tool_stats", &serde_json::json!({}), &mut ToolContext {
                    memory: &mut memory, prompt_history: &mut prompt_history,
                    analyzer: &analyzer, evolver: &evolver, ollama: &tool_ollama,
                    project_root: &project_root, applet_manager: &mut manager,
                    steer_tx: &steer_tx, steer_rx: &steer_rx, input_flag: &mut input_flag,
                    menu_flag: &menu_flag,
                }).await {
                    Ok(r) => println!("\n{}", r),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            _ if lower == "history" || lower.starts_with("history ") => {
                let n: usize = input.get(7..).unwrap_or("").trim().parse().unwrap_or(10);
                let recent = messages.iter().rev().take(n).rev();
                for m in recent {
                    let role = if m.role == "user" { "you" } else { "ayesha" };
                    let preview: String = m.content.chars().take(100).collect();
                    println!("  \x1b[90m{}\x1b[0m \x1b[36m{}\x1b[0m: {}{}", role,
                        if m.role == "user" { "" } else { "" },
                        preview,
                        if m.content.chars().count() > 100 { "..." } else { "" });
                }
                continue;
            }
            "compact" => {
                let before = messages.len();
                // Keep system message + as many recent messages as fit cleanly
                if messages.len() > 9 {
                    let system = messages[0].clone();
                    // Walk backwards to find a clean cut point (not in the middle of tool calls)
                    let mut cut = messages.len();
                    // Start from the end, count up to 8 messages
                    let mut kept = 0;
                    let mut i = messages.len();
                    while i > 1 && kept < 8 {
                        i -= 1;
                        kept += 1;
                        // If this is a tool result, skip all consecutive tool results
                        // so we don't split a tool-call sequence
                        if messages[i].role == "tool" {
                            while i > 1 && messages[i - 1].role == "tool" {
                                i -= 1;
                                kept += 1;
                            }
                            // Also include the preceding assistant with tool_calls
                            if i > 1 && messages[i - 1].tool_calls.is_some() {
                                i -= 1;
                                kept += 1;
                            }
                        }
                    }
                    cut = i;
                    let recent: Vec<_> = messages[cut..].to_vec();
                    messages.clear();
                    messages.push(system);
                    messages.extend(recent);
                    ui::show_system(&format!("compacted: {} → {} messages (kept system + last {})", before, messages.len(), kept));
                } else {
                    ui::show_system(&format!("already compact ({} messages)", before));
                }
                continue;
            }
            _ if lower == "save" || lower.starts_with("save ") => {
                let path_str = input.get(5..).unwrap_or("").trim();
                let path = if path_str.is_empty() {
                    project_root.join("conversation.json")
                } else {
                    std::path::PathBuf::from(path_str)
                };
                match serde_json::to_string_pretty(&messages) {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(&path, json) {
                            ui::show_error(&format!("failed to save: {}", e));
                        } else {
                            ui::show_system(&format!("saved {} messages to {}", messages.len(), path.display()));
                        }
                    }
                    Err(e) => ui::show_error(&format!("serialize error: {}", e)),
                }
                continue;
            }
            _ if lower == "load" || lower.starts_with("load ") => {
                let path_str = input.get(5..).unwrap_or("").trim();
                let path = if path_str.is_empty() {
                    project_root.join("conversation.json")
                } else {
                    std::path::PathBuf::from(path_str)
                };
                match std::fs::read_to_string(&path) {
                    Ok(json) => {
                        match serde_json::from_str::<Vec<ChatMessage>>(&json) {
                            Ok(loaded) => {
                                let n = loaded.len();
                                messages = loaded;
                                ui::show_system(&format!("loaded {} messages from {}", n, path.display()));
                            }
                            Err(e) => ui::show_error(&format!("parse error: {}", e)),
                        }
                    }
                    Err(e) => ui::show_error(&format!("read error: {}", e)),
                }
                continue;
            }
            "sessions" => {
                let names = session::list_sessions(&project_root);
                if names.is_empty() {
                    ui::show_system("no saved sessions yet");
                } else {
                    println!("\n  {}", theme::bold(Role::Accent, "sessions:"));
                    for name in names {
                        let file = std::path::Path::new(&project_root)
                            .join(session::SESSION_DIR)
                            .join(format!("{name}.json"));
                        let msgs = session::load_session(&project_root, &name)
                            .map(|m| m.len().saturating_sub(1))
                            .unwrap_or(0);
                        let mtime = file.metadata()
                            .and_then(|m| m.modified())
                            .map(|t| {
                                let d = std::time::SystemTime::now()
                                    .duration_since(t)
                                    .map(|x| x.as_secs())
                                    .unwrap_or(0);
                                format!("{}s ago", d)
                            })
                            .unwrap_or_else(|_| "?".to_string());
                        let is_default = name == session::DEFAULT_SESSION;
                        println!("  {} {}  ({} msgs, {})",
                            theme::paint(if is_default { Role::Primary } else { Role::Dim },
                                if is_default { "▶" } else { " " }),
                            theme::paint(Role::Text, &name),
                            msgs,
                            mtime);
                    }
                    println!();
                }
                continue;
            }
            _ if lower == "resume" || lower.starts_with("resume ") => {
                let name = input.get(7..).unwrap_or("").trim();
                let name = if name.is_empty() { session::DEFAULT_SESSION } else { name };
                match session::load_session(&project_root, name) {
                    Ok(saved) => {
                        let n = saved.len().saturating_sub(1);
                        messages.clear();
                        messages.push(ChatMessage {
                            role: "system".to_string(),
                            content: build_system_prompt(&user_name, &project_root),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                        messages.extend(saved.into_iter().skip(1));
                        ui::show_system(&format!("resumed session '{}' ({} messages)", name, n));
                    }
                    Err(e) => ui::show_error(&format!("no session '{}': {}", name, e)),
                }
                continue;
            }
            "newsession" => {
                let kept = messages.len();
                messages.truncate(1);
                ui::show_system(&format!("started a new session (dropped {} messages)", kept.saturating_sub(1)));
                continue;
            }
            "system" => {
                if !messages.is_empty() {
                    println!("\n{}", theme::bold(Role::Warning, "System Prompt:"));
                    for line in messages[0].content.lines() {
                        println!("  {}", theme::paint(Role::Dim, line));
                    }
                } else {
                    ui::show_system("no system prompt loaded");
                }
                continue;
            }
            _ if lower == "export" || lower.starts_with("export ") => {
                let path_str = input.get(7..).unwrap_or("").trim();
                let path = if path_str.is_empty() {
                    project_root.join("conversation.md")
                } else {
                    std::path::PathBuf::from(path_str)
                };
                let mut md = String::from("# ayesha conversation\n\n");
                for m in &messages {
                    if m.role == "system" { continue; }
                    let role = if m.role == "user" { "## you" } else { "## ayesha" };
                    md.push_str(&format!("{}\n\n{}\n\n", role, m.content));
                }
                match std::fs::write(&path, &md) {
                    Ok(()) => ui::show_system(&format!("exported {} messages to {}", messages.len().saturating_sub(1), path.display())),
                    Err(e) => ui::show_error(&format!("write error: {}", e)),
                }
                continue;
            }
            "ping" => {
                use std::time::Instant;
                let start = Instant::now();
                let test_msgs = vec![ChatMessage {
                    role: "user".to_string(),
                    content: "ping".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                }];
                match client.chat_stream_collect(&test_msgs, None, &steer_rx).await {
                    Ok(_) => {
                        let elapsed = start.elapsed().as_millis();
                        ui::show_system(&format!("pong! {}ms (model: {})", elapsed, current_model));
                    }
                    Err(e) => ui::show_error(&format!("ping failed: {}", e)),
                }
                continue;
            }
            "joke" => {
                let jokes = [
                    "why do programmers prefer dark mode? because light attracts bugs.",
                    "there are 10 types of people in the world: those who understand binary and those who don't.",
                    "a sql query walks into a bar, walks up to two tables and asks: 'can i join you?'",
                    "why was the javascript developer sad? because he didn't node how to express himself.",
                    "what's a programmer's favorite hangout place? foo bar.",
                    "why do java developers wear glasses? because they can't c#.",
                    "how many programmers does it take to change a light bulb? none — that's a hardware problem.",
                    "what is a robot's favorite type of music? heavy metal.",
                    "why did the developer go broke? because he used up all his cache.",
                    "what's a computer's least favorite food? spam.",
                ];
                let joke = {
                    use std::collections::hash_map::RandomState;
                    use std::hash::{BuildHasher, Hasher};
                    let key = RandomState::new().build_hasher().finish();
                    jokes[key as usize % jokes.len()]
                };
                println!("\n  {}", theme::paint(Role::Secondary, joke));
                continue;
            }
            "time" => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                // Simple UTC time display
                let hours = (now / 3600) % 24;
                let mins = (now / 60) % 60;
                let secs = now % 60;
                ui::show_system(&format!("utc time: {:02}:{:02}:{:02}", hours, mins, secs));
                continue;
            }
            "uptime" => {
                let elapsed = session_start.elapsed();
                let secs = elapsed.as_secs();
                let hours = secs / 3600;
                let mins = (secs % 3600) / 60;
                let secs = secs % 60;
                ui::show_system(&format!("uptime: {}h {}m {}s", hours, mins, secs));
                continue;
            }
            _ if lower == "config" || lower.starts_with("config ") => {
                let key = input.get(7..).unwrap_or("").trim();
                if key.is_empty() {
                    println!("\n{}", theme::bold(Role::Warning, "Configuration:"));
                    println!("  user_name: {}", config.get("user_name").and_then(|v| v.as_str()).unwrap_or("(not set)"));
                    println!("  tool_model: {}", tool_model_name);
                    println!("  chat_model: {}", current_model);
                    println!("  config_path: {}", config_path.display());
                } else {
                    let parts: Vec<&str> = key.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let k = parts[0].trim();
                        let v = parts[1].trim();
                        config[k] = serde_json::json!(v);
                        if let Ok(content) = serde_json::to_string_pretty(&config) {
                            let _ = std::fs::write(&config_path, content);
                        }
                        ui::show_system(&format!("set {} = {}", k, v));
                    } else {
                        let val = config.get(key).map(|v| v.to_string()).unwrap_or("(not found)".to_string());
                        println!("  {} = {}", theme::paint(Role::Accent, key), theme::paint(Role::Text, val));
                    }
                }
                continue;
            }
            "memory" => {
                match executor.execute("list_memories", &serde_json::json!({}), &mut ToolContext {
                    memory: &mut memory, prompt_history: &mut prompt_history,
                    analyzer: &analyzer, evolver: &evolver, ollama: &tool_ollama,
                    project_root: &project_root, applet_manager: &mut manager,
                    steer_tx: &steer_tx, steer_rx: &steer_rx, input_flag: &mut input_flag,
                    menu_flag: &menu_flag,
                }).await {
                    Ok(r) => println!("\n{}", r),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            "skills" => {
                match executor.execute("list_skills", &serde_json::json!({}), &mut ToolContext {
                    memory: &mut memory, prompt_history: &mut prompt_history,
                    analyzer: &analyzer, evolver: &evolver, ollama: &tool_ollama,
                    project_root: &project_root, applet_manager: &mut manager,
                    steer_tx: &steer_tx, steer_rx: &steer_rx, input_flag: &mut input_flag,
                    menu_flag: &menu_flag,
                }).await {
                    Ok(r) => println!("\n{}", r),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            "analyze" => {
                match executor.execute("analyze_self", &serde_json::json!({}), &mut ToolContext {
                    memory: &mut memory, prompt_history: &mut prompt_history,
                    analyzer: &analyzer, evolver: &evolver, ollama: &tool_ollama,
                    project_root: &project_root, applet_manager: &mut manager,
                    steer_tx: &steer_tx, steer_rx: &steer_rx, input_flag: &mut input_flag,
                    menu_flag: &menu_flag,
                }).await {
                    Ok(r) => println!("\n{}", r),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            "evolve" => {
                match executor.execute("evolve_tools", &serde_json::json!({}), &mut ToolContext {
                    memory: &mut memory, prompt_history: &mut prompt_history,
                    analyzer: &analyzer, evolver: &evolver, ollama: &tool_ollama,
                    project_root: &project_root, applet_manager: &mut manager,
                    steer_tx: &steer_tx, steer_rx: &steer_rx, input_flag: &mut input_flag,
                    menu_flag: &menu_flag,
                }).await {
                    Ok(r) => println!("\n{}", r),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            "refine" => {
                match executor.execute("refine_prompt", &serde_json::json!({}), &mut ToolContext {
                    memory: &mut memory, prompt_history: &mut prompt_history,
                    analyzer: &analyzer, evolver: &evolver, ollama: &tool_ollama,
                    project_root: &project_root, applet_manager: &mut manager,
                    steer_tx: &steer_tx, steer_rx: &steer_rx, input_flag: &mut input_flag,
                    menu_flag: &menu_flag,
                }).await {
                    Ok(r) => println!("\n{}", r),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            _ => {
                // If it was a slash command, check applet short names
                if was_slash && manager.has(&input) {
                    if manager.is_foreground(&input) {
                        match manager.run_in_window(&input, &steer_tx, &steer_rx, &mut input_flag, &menu_flag) {
                            Ok(()) => ui::show_system(&format!("returned from {} — press ctrl+p to switch pages again", input)),
                            Err(e) => ui::show_error(&e),
                        }
                    } else if manager.is_running(&input) {
                        match manager.stop(&input) {
                            Ok(()) => ui::show_system(&format!("stopped {}", input)),
                            Err(e) => ui::show_error(&e),
                        }
                    } else {
                        match manager.launch(&input) {
                            Ok(()) => ui::show_system(&format!("launched {}", input)),
                            Err(e) => ui::show_error(&e),
                        }
                    }
                    continue;
                }
            }
        }

        // After match, if was_slash but no command matched, show error
        // (route is handled below the match block, so exclude it here)
        if was_slash && !lower.starts_with("route ") && !lower.starts_with("routes ") {
            ui::show_error(&format!("unknown command: /{}. type /help", input));
            continue;
        }

        // ── natural-language applet phrases (no model round-trip) ──
        // "launch flora cli", "open desktop-cat", "stop poopy-tui", etc.
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

        // ── route (handle `route <query>` prefix) ──
        let user_content = if lower.starts_with("route ") {
            let query = input[6..].trim().to_string();
            let target = registry.select_model(&query);
            if target.name != current_model {
                ui::show_routing(&target.name);
                current_model = target.name.clone();
                if registry.is_cloud_model(&current_model) {
                    let provider = registry.cloud_provider(&current_model).unwrap_or_default();
                    match CloudClient::new(&current_model, &provider) {
                        Ok(cc) => client = ActiveBackend::Cloud(cc),
                        Err(_) => client = ActiveBackend::Ollama(OllamaClient::new("ayesha")),
                    }
                } else {
                    client = ActiveBackend::Ollama(OllamaClient::new(&current_model));
                }
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
                if registry.is_cloud_model(&current_model) {
                    let provider = registry.cloud_provider(&current_model).unwrap_or_default();
                    match CloudClient::new(&current_model, &provider) {
                        Ok(cc) => client = ActiveBackend::Cloud(cc),
                        Err(_) => client = ActiveBackend::Ollama(OllamaClient::new("ayesha")),
                    }
                } else {
                    client = ActiveBackend::Ollama(OllamaClient::new(&current_model));
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

        // Step 1: Call ayesha (no tools) for personality response
        let first_result = client
            .chat_stream_visible(&messages, None, &steer_rx)
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

                            let tool_result = match executor.execute(name, args, &mut ToolContext {
                                memory: &mut memory,
                                prompt_history: &mut prompt_history,
                                analyzer: &analyzer,
                                evolver: &evolver,
                                ollama: &tool_ollama,
                                project_root: &project_root,
                                applet_manager: &mut manager,
                                steer_tx: &steer_tx,
                                steer_rx: &steer_rx,
                                input_flag: &mut input_flag,
                                menu_flag: &menu_flag,
                            }).await {
                    Ok(r) => r,
                    Err(e) => {
                        let err_msg = format!("error: {}", e);
                        prompt_history.record_usage(name, false, Some(err_msg.clone()), &args_str);
                        let _ = prompt_history.save();
                        err_msg
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
            let final_result = client
                .chat_stream_visible(&messages, None, &steer_rx)
                .await;

            if let Ok(r) = final_result {
                if !r.content.is_empty() {
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: r.content,
                        tool_calls: None,
                        tool_call_id: None,
                    });
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

        // Step 2: If ayesha didn't call tools, check if qwen2.5 would
        // Skip for pure-chit-chat to save ~2s latency per message.
        if !steer_happened && !first_had_tools && needs_tools {
            // Try qwen2.5 with tools to see if it wants to call any
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
                .chat_stream_collect(&tool_messages, Some(&tools), &steer_rx)
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

                let tool_result = match executor.execute(name, args, &mut ToolContext {
                                memory: &mut memory,
                                prompt_history: &mut prompt_history,
                                analyzer: &analyzer,
                                evolver: &evolver,
                                ollama: &tool_ollama,
                                project_root: &project_root,
                                applet_manager: &mut manager,
                                steer_tx: &steer_tx,
                                steer_rx: &steer_rx,
                                input_flag: &mut input_flag,
                                menu_flag: &menu_flag,
                            }).await {
                                Ok(r) => r,
                                Err(e) => {
                                    let err_msg = format!("error: {}", e);
                                    prompt_history.record_usage(name, false, Some(err_msg.clone()), &args_str);
                                    let _ = prompt_history.save();
                                    err_msg
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

                        // Re-prompt qwen2.5 for next tool calls (invisible)
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
                            .chat_stream_collect(&tool_messages2, Some(&tools), &steer_rx)
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
                            .chat_stream_visible(&messages, None, &steer_rx)
                            .await;

                        if let Ok(r) = final_result {
                            if !r.content.is_empty() {
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
    let sandbox = Sandbox::default_workspace();
    let executor = ToolExecutor::new(sandbox.clone());
    let client = OllamaClient::new("ayesha");
    let tool_client = OllamaClient::new("qwen2.5:7b");
    let tools = OllamaClient::tool_definitions_core();
    let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();
    let mut memory = memory::MemoryStore::load();
    let mut prompt_history = PromptHistory::load();
    let analyzer = SelfAnalyzer::new(std::env::current_dir().unwrap_or_default());
    let evolver = ToolEvolver::new(vec![
        "read_file".into(), "write_file".into(), "list_dir".into(),
        "grep".into(), "glob".into(), "generate_html".into(),
    ]);
    let project_root = std::env::current_dir().unwrap_or_default();
    let mut input_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let menu_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut manager = AppletManager::new();

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
        ollama: &client,
        project_root: &project_root,
        applet_manager: &mut manager,
        steer_tx: &steer_tx,
        steer_rx: &steer_rx,
        input_flag: &mut input_flag,
        menu_flag: &menu_flag,
    };

    // Step 1: try qwen directly with tools
    let qwen_result = tool_client.chat_stream_collect(&messages, Some(&tools), &steer_rx).await;

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

/// Headless E2E smoke test — verifies ollama is reachable, both models respond,
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

    // 1. Ollama reachable
    let ollama_ok = OllamaClient::list_models().await.is_ok();
    check("ollama reachable at localhost:11434", ollama_ok, &mut checks);

    if !ollama_ok {
        println!("\n  \x1b[31mobort: ollama not reachable\x1b[0m\n");
        std::process::exit(1);
    }

    // 2. Ayesha model responds
    let ayesha = OllamaClient::new("ayesha");
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
    let qwen = OllamaClient::new("qwen2.5:7b");
    let qwen_resp = qwen.chat_stream_collect(&msgs, None, &rx).await;
    let qwen_ok = qwen_resp.as_ref().map(|r| !r.content.is_empty()).unwrap_or(false);
    check(
        &format!("qwen2.5 model responds{}", if let Err(e) = &qwen_resp {
            format!(" (error: {})", e)
        } else { String::new() }),
        qwen_ok,
        &mut checks,
    );

    // 4. Tool definitions parse
    let tools = OllamaClient::tool_definitions();
    let tools_ok = !tools.is_empty();
    check(&format!("{} tool definitions loaded", tools.len()), tools_ok, &mut checks);

    // 5. Qwen emits tool_calls when tools provided (regression guard)
    let tool_msgs = vec![ChatMessage {
        role: "user".to_string(),
        content: "read main.rs".to_string(),
        tool_calls: None,
        tool_call_id: None,
    }];
    let qwen_tool_resp = qwen.chat_stream_collect(&tool_msgs, Some(&tools), &rx).await;
    let qwen_tool_ok = qwen_tool_resp.as_ref().map(|r| r.has_tool_calls()).unwrap_or(false);
    check(
        &format!("qwen2.5 emits tool_calls{}", if let Err(e) = &qwen_tool_resp {
            format!(" (error: {})", e)
        } else { String::new() }),
        qwen_tool_ok,
        &mut checks,
    );

    // 6. Truncate tool result
    let trunc_ok = util::truncate_chars("hello world", 5) == "hello";
    check("truncate_chars works", trunc_ok, &mut checks);

    // 7. StreamParser works
    let mut parser = ollama::StreamParser::new();
    parser.feed_line(r#"{"message":{"content":"test"},"done":false}"#);
    parser.feed_line(r#"{"message":{"content":" ok"},"done":false}"#);
    parser.feed_line(r#"{"message":{"content":""},"done":true}"#);
    let parser_ok = parser.content == "test ok" && parser.done;
    check("StreamParser accumulates content", parser_ok, &mut checks);

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
        let n = names(&["flora-cli", "desktop-cat", "poopy-tui", "engine"]);

        assert_eq!(parse_applet_phrase(&n, "launch flora cli"), Some(("flora-cli".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "launch flora-cli"), Some(("flora-cli".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "open Desktop Cat"), Some(("desktop-cat".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "start poopy tui"), Some(("poopy-tui".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "run engine"), Some(("engine".to_string(), false)));
        assert_eq!(parse_applet_phrase(&n, "stop poopy-tui"), Some(("poopy-tui".to_string(), true)));
        assert_eq!(parse_applet_phrase(&n, "launch poopy"), None);
        assert_eq!(parse_applet_phrase(&n, "launch a test for me"), None);
        assert_eq!(parse_applet_phrase(&n, "what is the weather"), None);
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
}
