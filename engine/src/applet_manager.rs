use std::collections::HashMap;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};
use colored::*;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppletEntry {
    pub path: String,
    #[allow(dead_code)]
    pub lang: String,
    pub desc: String,
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub foreground: bool,
}

#[derive(Debug, Deserialize)]
struct AyeshaConfig {
    #[serde(default)]
    pub projects: HashMap<String, AppletEntry>,
}

pub struct AppletManager {
    pub entries: HashMap<String, AppletEntry>,
    pub processes: HashMap<String, Child>,
    pub root: String,
}

impl AppletManager {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        // Walk up from CWD to find the directory containing ayesha.json and applets/
        let root = {
            let mut dir = cwd.as_path();
            let mut found = None;
            for _ in 0..10 {
                if dir.join("ayesha.json").exists() && dir.join("applets").exists() {
                    found = Some(dir.to_path_buf());
                    break;
                }
                match dir.parent() {
                    Some(p) => dir = p,
                    None => break,
                }
            }
            if found.is_none() {
                let mut dir = cwd.as_path();
                for _ in 0..10 {
                    if dir.join("ayesha.json").exists() {
                        found = Some(dir.to_path_buf());
                        break;
                    }
                    match dir.parent() {
                        Some(p) => dir = p,
                        None => break,
                    }
                }
            }
            found.unwrap_or(cwd)
        };
        let root = root.to_string_lossy().to_string();

        let config_path = std::path::Path::new(&root).join("ayesha.json");
        let entries = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| serde_json::from_str::<AyeshaConfig>(&s).ok())
                .map(|c| c.projects)
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        AppletManager {
            entries,
            processes: HashMap::new(),
            root,
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn is_running(&self, name: &str) -> bool {
        self.processes.contains_key(name)
    }

    pub fn is_foreground(&self, name: &str) -> bool {
        self.entries.get(name).map(|e| e.foreground).unwrap_or(false)
    }

    pub fn list(&self) -> String {
        let mut out = String::new();
        out.push_str("  ┌───── applets ───────────────────────────────────────────┐\n");
        let mut sorted: Vec<(&String, &AppletEntry)> = self.entries.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (name, entry) in &sorted {
            let running = if self.processes.contains_key(*name) { "●" } else { "○" };
            let port_str = match entry.port {
                Some(p) => format!(" :{}", p),
                None => String::new(),
            };
            let runnable = if entry.run.is_some() { "" } else { "  [no run]" };
            out.push_str(&format!("  │ {} /{:<10} {:<22}{}{} │\n",
                running, *name, entry.desc, port_str, runnable));
        }
        out.push_str("  │                                                        │\n");
        out.push_str("  │  ● running  ○ stopped  /<name> to launch              │\n");
        out.push_str("  └────────────────────────────────────────────────────────┘\n");
        out
    }

    pub fn launch(&mut self, name: &str) -> Result<(), String> {
        let entry = self.entries.get(name).ok_or_else(|| format!("unknown applet: {}", name))?;

        if self.processes.contains_key(name) {
            return Err(format!("{} is already running", name));
        }

        let run_cmd = entry.run.as_deref().ok_or_else(|| format!("no run command for {}", name))?;
        let work_dir = std::path::Path::new(&self.root).join(&entry.path);

        let parts: Vec<&str> = run_cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err(format!("invalid run command for {}", name));
        }

        let program = if parts[0] == "npm" {
            if cfg!(windows) {
                "npm.cmd"
            } else {
                "npm"
            }
        } else if parts[0] == "npx" {
            if cfg!(windows) {
                "npx.cmd"
            } else {
                "npx"
            }
        } else {
            parts[0]
        };

        if !work_dir.exists() {
            return Err(format!("path not found: {}", work_dir.display()));
        }

        let child = if cfg!(windows) {
            let args_joined = parts[1..].join(" ");
            let install_cmd = if work_dir.join("package.json").exists() {
                "(if not exist node_modules (echo installing applet dependencies... && npm install)) && "
            } else {
                ""
            };
            let full_cmd = format!("cd /d \"{}\" && {}{} {}", work_dir.display(), install_cmd, program, args_joined);
            Command::new("cmd.exe")
                .args(["/c", "start", "cmd.exe", "/k", &full_cmd])
                .spawn()
                .map_err(|e| format!("failed to launch {}: {}", name, e))?
        } else {
            Command::new(program)
                .args(&parts[1..])
                .current_dir(&work_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .spawn()
                .map_err(|e| format!("failed to launch {}: {}", name, e))?
        };

        self.processes.insert(name.to_string(), child);
        Ok(())
    }

    /// Spawn an applet as a child process attached to the *current* console,
    /// so it takes over this terminal window until it exits.
    pub fn launch_foreground(&mut self, name: &str) -> Result<Child, String> {
        let entry = self.entries.get(name).ok_or_else(|| format!("unknown applet: {}", name))?;
        let run_cmd = entry.run.as_deref().ok_or_else(|| format!("no run command for {}", name))?;
        let work_dir = std::path::Path::new(&self.root).join(&entry.path);

        let parts: Vec<&str> = run_cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err(format!("invalid run command for {}", name));
        }

        let program = if parts[0] == "npm" {
            if cfg!(windows) { "npm.cmd" } else { "npm" }
        } else if parts[0] == "npx" {
            if cfg!(windows) { "npx.cmd" } else { "npx" }
        } else {
            parts[0]
        };

        if !work_dir.exists() {
            return Err(format!("path not found: {}", work_dir.display()));
        }

        let child = if cfg!(windows) {
            let args_joined = parts[1..].join(" ");
            let install_cmd = if work_dir.join("package.json").exists() {
                "(if not exist node_modules (echo installing applet dependencies... && npm install)) && "
            } else {
                ""
            };
            let full_cmd = format!("cd /d \"{}\" && {}{} {}", work_dir.display(), install_cmd, program, args_joined);
            Command::new("cmd.exe")
                .args(["/c", &full_cmd])
                .spawn()
                .map_err(|e| format!("failed to launch {} in this window: {}", name, e))?
        } else {
            Command::new(program)
                .args(&parts[1..])
                .current_dir(&work_dir)
                .spawn()
                .map_err(|e| format!("failed to launch {} in this window: {}", name, e))?
        };

        Ok(child)
    }

    /// Run a foreground applet inside the current terminal window. The engine's
    /// input thread is suspended and raw mode is released so the applet owns the
    /// terminal; when it exits, the engine's UI is restored.
    pub fn run_in_window(
        &mut self,
        name: &str,
        steer_tx: &mpsc::Sender<String>,
        steer_rx: &mpsc::Receiver<String>,
        input_flag: &mut Arc<AtomicBool>,
        menu_flag: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        if !self.is_foreground(name) {
            self.launch(name)?;
            return Ok(());
        }

        // 1. stop the engine's input thread so it can't steal keystrokes
        crate::applet_runner::suspend_input(input_flag);

        // 2. release raw mode so the applet controls the terminal
        let _ = crossterm::terminal::disable_raw_mode();

        // 3. drain stale keystrokes queued before the suspend
        while steer_rx.try_recv().is_ok() {}

        // 4. hand the window over to the applet
        print!("\x1B[2J\x1B[1;1H");
        let _ = std::io::stdout().flush();
        println!();
        println!("  {} {}",
            "◆".bright_green(),
            format!("{} running in this window — exit it (or close) to return to ayesha", name).bright_cyan());
        println!();
        let _ = std::io::stdout().flush();

        let mut child = match self.launch_foreground(name) {
            Ok(c) => c,
            Err(e) => {
                let _ = crossterm::terminal::enable_raw_mode();
                return Err(e);
            }
        };
        let _ = child.wait();

        // 5. hand the window back to the engine
        let _ = crossterm::terminal::enable_raw_mode();
        let mut candidates: Vec<String> = vec![
            "help", "clear", "models", "auto", "sync", "apps", "run", "stop",
            "model", "toolmodel", "pull", "route", "name", "exit",
            "stats", "history", "compact", "save", "load", "system", "export", "ping",
            "joke", "time", "uptime", "config",
            "memory", "analyze", "evolve", "refine", "reset",
        ].into_iter().map(String::from).collect();
        for name in self.names() {
            candidates.push(name);
        }
        *input_flag = crate::applet_runner::spawn_input_thread(steer_tx.clone(), candidates, menu_flag.clone());
        print!("\x1B[2J\x1B[1;1H");
        let _ = std::io::stdout().flush();
        crate::ui::print_banner();

        Ok(())
    }

    pub fn stop(&mut self, name: &str) -> Result<(), String> {
        match self.processes.get_mut(name) {
            Some(child) => {
                let _ = child.kill();
                let _ = child.wait();
                self.processes.remove(name);
                Ok(())
            }
            None => Err(format!("{} is not running", name)),
        }
    }

    pub fn stop_all(&mut self) {
        let names: Vec<String> = self.processes.keys().cloned().collect();
        for name in names {
            let _ = self.stop(&name);
        }
    }

    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.entries.keys().cloned().collect();
        v.sort();
        v
    }
}

impl Drop for AppletManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}
