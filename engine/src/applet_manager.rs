use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppletEntry {
    pub path: String,
    pub lang: String,
    pub desc: String,
    #[serde(default)]
    pub run: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
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
        // Walk up from CWD to find the directory containing ayesha.json
        let root = {
            let mut dir = cwd.as_path();
            let mut found = None;
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

        let child = Command::new(program)
            .args(&parts[1..])
            .current_dir(&work_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to launch {}: {}", name, e))?;

        self.processes.insert(name.to_string(), child);
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
