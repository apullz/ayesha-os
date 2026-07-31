mod ollama;
mod cloud;
mod tools;
mod sandbox;
mod ui;
mod pixel_striker;
mod memory;
mod self_analysis;
mod tool_evolution;
mod prompt_refinement;
mod coding_agent;
mod model_registry;
mod applet_manager;

use std::io::Write;
use colored::*;
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
use serde_json;

/// Active backend — either local Ollama or cloud (OpenRouter/OpenCode)
enum ActiveBackend {
    Ollama(OllamaClient),
    Cloud(CloudClient),
}

/// Truncate tool results to prevent context overflow
fn truncate_tool_result(result: &str, max_chars: usize) -> String {
    if result.len() > max_chars {
        format!("{}...\n(truncated: showing {} of {} chars)", &result[..max_chars], max_chars, result.len())
    } else {
        result.to_string()
    }
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
                let icon = LoadIconW(module, 1 as *const u16);
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    winapi::init_console();

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
        "generate_html".into(), "generate_sprite".into(), "generate_tileset".into(),
        "generate_object".into(), "render_sprite".into(), "read_clipboard".into(),
        "remember".into(), "list_memories".into(), "search_memories".into(),
        "set_preference".into(), "analyze_self".into(), "list_source_files".into(),
        "evolve_tools".into(), "refine_prompt".into(), "get_tool_stats".into(),
        "coding_agent".into(),
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

    let user_name = match config.get("user_name").and_then(|v| v.as_str()) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => {
            print!("\n  {} {}",
                "◆".bright_cyan(),
                "what should i call you, senpai?".bright_green());
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
                "✔".bright_green(),
                format!("okay, {}!", name).bright_cyan(),
                "remember that one, desu~".bright_black());
            name
        }
    };

    // Enable raw mode for Ctrl+M detection
    let _ = crossterm::terminal::enable_raw_mode();

    // Steering channel: crossterm event thread sends input here
    let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();

    std::thread::spawn(move || {
        use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
        let mut input_buf = String::new();
        loop {
            match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                            if steer_tx.send("\0ctrl-m".to_string()).is_err() { break; }
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            if steer_tx.send("\0ctrl-c".to_string()).is_err() { break; }
                        }
                        (KeyCode::Up, KeyModifiers::SHIFT) => {
                            if steer_tx.send("\0shift-up".to_string()).is_err() { break; }
                        }
                        (KeyCode::Down, KeyModifiers::SHIFT) => {
                            if steer_tx.send("\0shift-down".to_string()).is_err() { break; }
                        }
                        (KeyCode::Enter, _) => {
                            let line = input_buf.trim().to_string();
                            if steer_tx.send(line).is_err() { break; }
                            input_buf.clear();
                            print!("\r\n");
                            let _ = std::io::stdout().flush();
                        }
                        (KeyCode::Char(c), _) if c as u8 >= 32 => {
                            input_buf.push(c);
                            print!("{}", c);
                            let _ = std::io::stdout().flush();
                        }
                        (KeyCode::Backspace, _) => {
                            input_buf.pop();
                            print!("\x08 \x08");
                            let _ = std::io::stdout().flush();
                        }
                        _ => {}
                    }
                }
                Ok(Event::Paste(s)) => {
                    for c in s.chars() {
                        if c as u8 >= 32 { input_buf.push(c); print!("{}", c); }
                    }
                    let _ = std::io::stdout().flush();
                }
                _ => {}
            }
        }
    });

    ui::print_banner();
    ui::show_system(&memory.summary());

    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage {
            role: "system".to_string(),
            content: OllamaClient::system_prompt(&user_name),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    let tools = OllamaClient::tool_definitions();
    let mut current_model = "ayesha:latest".to_string();
    ui::show_system(&format!("chat model: {} | tool model: {}", current_model, tool_model_name));

    // Send /set nothink to disable thinking tokens (hidden from user)
    {
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: "/set nothink".to_string(),
            tool_calls: None,
            tool_call_id: None,
        });
        let init_client = OllamaClient::new(&current_model);
        if let Ok(resp) = init_client.chat(&messages, None).await {
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: resp.message.content,
                tool_calls: None,
                tool_call_id: None,
            });
        }
    }

    // Holds steering input that needs to be processed as the next user message
    let mut pending_input: Option<String> = None;

    enum InputMode { Normal, Launcher }
    let mut input_mode = InputMode::Normal;
    let mut applet_cycle_idx: Option<usize> = None;

    loop {
        // ── read user input ──
        let input = if let Some(p) = pending_input.take() {
            p
        } else {
            match input_mode {
                InputMode::Normal => ui::prompt_line(),
                InputMode::Launcher => ui::launcher_prompt(),
            }
            let inp = match steer_rx.recv() {
                Ok(i) => i,
                Err(_) => break,
            };
            // Ctrl+C → exit
            if inp == "\0ctrl-c" {
                break;
            }
            // Ctrl+M → toggle launcher mode
            if inp == "\0ctrl-m" {
                input_mode = InputMode::Launcher;
                println!("\n{}", manager.list());
                ui::show_system("type applet name to launch/stop, `back` to exit, `list` to refresh");
                continue;
            }
            // Shift + Up / Shift + Down → cycle applets
            if inp == "\0shift-up" || inp == "\0shift-down" {
                let applets = manager.names();
                if !applets.is_empty() {
                    let next_idx = match applet_cycle_idx {
                        None => 0,
                        Some(i) => {
                            if inp == "\0shift-down" {
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
            if inp.is_empty() {
                // In launcher mode, empty line exits
                if matches!(input_mode, InputMode::Launcher) {
                    input_mode = InputMode::Normal;
                    print!("\x1B[2J\x1B[1;1H");
                    std::io::stdout().flush().ok();
                }
                continue;
            }
            inp
        };

        // ── handle launcher mode ──
        if matches!(input_mode, InputMode::Launcher) {
            match input.as_str() {
                "back" | "exit" | "/back" => {
                    input_mode = InputMode::Normal;
                    print!("\x1B[2J\x1B[1;1H");
                    std::io::stdout().flush().ok();
                    continue;
                }
                "list" | "apps" => {
                    println!("\n{}", manager.list());
                    continue;
                }
                "stop" | "stop all" => {
                    manager.stop_all();
                    ui::show_system("stopped all applets");
                    continue;
                }
                _ => {
                    if let Some(name) = input.strip_prefix("stop ") {
                        match manager.stop(name) {
                            Ok(()) => ui::show_system(&format!("stopped {}", name)),
                            Err(e) => ui::show_error(&e),
                        }
                    } else if manager.has(&input) {
                        if manager.is_running(&input) {
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
                    } else {
                        ui::show_error(&format!("unknown applet. type `list` to see available"));
                    }
                    continue;
                }
            }
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
        // Auto-parse memory markers from ayesha's last response
        if let Some(last) = messages.last() {
            if last.role == "assistant" {
                let cleaned = parse_and_store_memories(&last.content, &mut memory);
                if cleaned != last.content {
                    messages.last_mut().unwrap().content = cleaned;
                }
            }
        }

        let _ = memory.save();
                let _ = prompt_history.save();
                manager.stop_all();
                let _ = crossterm::terminal::disable_raw_mode();
                println!();
                println!("  {} {}", "●".bright_green(), "ayesha-os shutting down".bright_cyan());
                println!("  {} {}", "◆".bright_cyan(), format!("saved {}", memory.summary()).bright_black());
                println!();
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
            "models" => {
                println!();
                println!("{}", registry.list_models());
                continue;
            }
            "auto" => {
                registry.set_auto_route(true);
                ui::show_system("auto-routing enabled");
                continue;
            }
            "sync" => {
                ui::show_system("initiating tri-mind sync & pushing directly to github...");
                let _ = std::process::Command::new("python")
                    .args(["-m", "tri_mind_sync.cli", "sync"])
                    .output();

                let _ = std::process::Command::new("git")
                    .args(["add", "."])
                    .status();
                let _ = std::process::Command::new("git")
                    .args(["commit", "-m", "ayesha-os: auto sync update"])
                    .status();
                let push_status = std::process::Command::new("git")
                    .args(["push", "origin", "master"])
                    .status();

                match push_status {
                    Ok(s) if s.success() => {
                        ui::show_system("successfully pushed updates to https://github.com/apullz/ayesha-os! (๑>◡<๑)");
                    }
                    _ => {
                        ui::show_error("git push executed (check authentication if remote unchanged).");
                    }
                }
                continue;
            }
            "apps" | "applets" => {
                println!("\n{}", manager.list());
                continue;
            }
            _ if lower.starts_with("run ") => {
                let name = input[4..].trim();
                if manager.has(name) {
                    if manager.is_running(name) {
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
            _ if lower.starts_with("toolmodel ") => {
                let name = input[10..].trim();
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
                } else {
                    tool_client = ActiveBackend::Ollama(OllamaClient::new(name));
                    tool_model_name = name.to_string();
                    ui::show_system(&format!("tool model: {}", name));
                }
                continue;
            }
            _ if lower.starts_with("pull ") => {
                let name = input[5..].trim();
                ui::show_system(&format!("run `ollama pull {}` in another terminal, then `models` to refresh", name));
                continue;
            }
            "route" | "routes" => {
                ui::show_system("usage: /route <query>");
                continue;
            }
            _ if lower.starts_with("name ") || lower == "name" => {
                let name = input[5..].trim().to_string();
                if name.is_empty() {
                    ui::show_system("usage: /name <you>");
                } else {
                    config["user_name"] = serde_json::json!(name);
                    if let Ok(content) = serde_json::to_string_pretty(&config) {
                        let _ = std::fs::write(&config_path, content);
                    }
                    messages[0].content = OllamaClient::system_prompt(&name);
                    ui::show_system(&format!("okay, {} it is!", name));
                }
                continue;
            }
            _ => {
                // If it was a slash command, check applet short names
                if was_slash && manager.has(&input) {
                    if manager.is_running(&input) {
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
        if was_slash {
            ui::show_error(&format!("unknown command: /{}. type /help", input));
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
            if !first_result.content.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: first_result.content,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        }

        // Step 2: If ayesha didn't call tools, check if qwen2.5 would
        if !steer_happened && !first_had_tools {
            // Try qwen2.5 with tools to see if it wants to call any
            let qwen_result = tool_client
                .chat_stream_visible(&messages, Some(&tools), &steer_rx)
                .await;

            match qwen_result {
                Ok(qr) if qr.has_tool_calls() => {
                    // qwen wants to call tools — execute them
                    messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: qr.content.clone(),
                        tool_calls: Some(qr.tool_calls.clone()),
                        tool_call_id: None,
                    });

                    let mut tool_iterations = 0;
                    loop {
                        tool_iterations += 1;
                        if tool_iterations > 8 {
                            ui::show_error("max tool iterations (8). stopping.");
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

                        // Remove the assistant message with tool calls
                        messages.pop();

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

                        // Push assistant message back (after tool results)
                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: qr.content.clone(),
                            tool_calls: Some(current_tool_calls),
                            tool_call_id: None,
                        });

                        // Re-prompt qwen2.5 for next tool calls
                        let next_result = tool_client
                            .chat_stream_visible(&messages, Some(&tools), &steer_rx)
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
                                        content: nr.content.clone(),
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
                    // qwen didn't call tools either — ayesha's response is final
                    // (already pushed above)
                }
                Err(_) => {
                    // qwen failed — ayesha's response is final
                }
            }
        }

        if steer_happened {
            messages.truncate(msg_count_before);
            continue;
        }

        let _ = memory.save();
        let _ = prompt_history.save();
    }

    Ok(())
}
