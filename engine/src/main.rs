mod ollama;
mod tools;
mod sandbox;
mod ui;
mod pixel_striker;
mod memory;
mod self_analysis;
mod tool_evolution;
mod prompt_refinement;
mod model_registry;
mod applet_manager;

use std::io::Write;
use colored::*;
use ollama::{OllamaClient, ChatMessage};
use tools::ToolExecutor;
use sandbox::Sandbox;
use prompt_refinement::PromptHistory;
use memory::MemoryStore;
use model_registry::ModelRegistry;
use applet_manager::AppletManager;
use serde_json;

#[cfg(windows)]
mod winapi {
    extern "system" {
        pub fn AllocConsole() -> i32;
        pub fn GetConsoleWindow() -> *mut core::ffi::c_void;
        pub fn SendMessageW(hwnd: *mut core::ffi::c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
        pub fn GetModuleHandleW(name: *const u16) -> *mut core::ffi::c_void;
        pub fn LoadIconW(instance: *mut core::ffi::c_void, name: *const u16) -> *mut core::ffi::c_void;
        pub fn SetConsoleTitleW(title: *const u16) -> i32;
    }

    pub fn init_console() {
        unsafe {
            AllocConsole();
            let console = GetConsoleWindow();
            if console.is_null() { return; }

            let module = GetModuleHandleW(std::ptr::null());
            let icon = LoadIconW(module, 1 as *const u16);
            if !icon.is_null() {
                SendMessageW(console, 0x0080, 0, icon as isize);
                SendMessageW(console, 0x0080, 1, icon as isize);
            }

            let title: Vec<u16> = "Ayesha-Engine\0".encode_utf16().collect();
            SetConsoleTitleW(title.as_ptr());
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
    let executor = ToolExecutor::new(sandbox);
    let mut client = OllamaClient::new("ayesha");
    let mut memory = MemoryStore::load();
    let mut prompt_history = PromptHistory::load();

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
    let mut current_model = "ayesha".to_string();
    let mut manager = AppletManager::new();

    // Holds steering input that needs to be processed as the next user message
    let mut pending_input: Option<String> = None;

    enum InputMode { Normal, Launcher }
    let mut input_mode = InputMode::Normal;

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
            "stats" => {
                match executor.get_tool_stats() {
                    Ok(stats) => println!("\n{}", stats),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            "memory" => {
                match executor.list_memories(&serde_json::json!({})) {
                    Ok(mem) => println!("\n{}", mem),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            "analyze" => {
                ui::show_system("analyzing main.rs...");
                match executor.analyze_self(&serde_json::json!({"file": "main.rs"})).await {
                    Ok(analysis) => println!("\n{}", analysis),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            "evolve" => {
                ui::show_system("evolving tools...");
                match executor.evolve_tools().await {
                    Ok(suggestions) => println!("\n{}", suggestions),
                    Err(e) => ui::show_error(&e.to_string()),
                }
                continue;
            }
            "refine" => {
                ui::show_system("analyzing prompt history...");
                match executor.refine_prompt().await {
                    Ok(analysis) => println!("\n{}", analysis),
                    Err(e) => ui::show_error(&e.to_string()),
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
                        client = OllamaClient::new(&current_model);
                        ui::show_system(&format!("switched to: {}", name));
                    }
                    Err(e) => ui::show_error(&e.to_string()),
                }
                registry.detect().await;
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
                client = OllamaClient::new(&current_model);
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
                client = OllamaClient::new(&current_model);
            }
        }

        // ── agent loop ──
        let msg_count_before = messages.len();

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: user_content,
            tool_calls: None,
            tool_call_id: None,
        });

        let mut steer_happened = false;
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > 10 {
                ui::show_error("max tool iterations (10). stopping.");
                break;
            }

            let result = client
                .chat_stream_visible(&messages, Some(&tools), &steer_rx)
                .await;

            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    ui::show_error(&format!("ollama error: {}", e));
                    break;
                }
            };

            if result.was_steered() {
                ui::show_interrupted();
                pending_input = result.steering;
                steer_happened = true;
                break;
            }

            if result.has_tool_calls() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: result.content.clone(),
                    tool_calls: Some(result.tool_calls.clone()),
                    tool_call_id: None,
                });

                for tool_call in &result.tool_calls {
                    let name = &tool_call.function.name;
                    let args = &tool_call.function.arguments;
                    let args_str = serde_json::to_string(args).unwrap_or_default();

                    ui::show_tool_call(name, &args_str);

                    let tool_result = match executor.execute(name, args).await {
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

                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: tool_result,
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }

                continue;
            }

            // Final response — already streamed to screen
            if !result.content.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: result.content,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }

            break;
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
