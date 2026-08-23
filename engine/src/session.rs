// conversation session persistence — auto-save the live chat to a repo-local
// `sessions/` dir and resume it on startup. Serialization mirrors the existing
// `/save` command (serde on `Vec<ChatMessage>`), stored as pretty JSON.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use crate::llm::ChatMessage;

/// Directory (relative to project_root) where sessions are stored.
pub const SESSION_DIR: &str = "sessions";

/// Session saved on shutdown + after each turn.
pub const DEFAULT_SESSION: &str = "default";

fn dir_for(root: &Path) -> PathBuf {
    root.join(SESSION_DIR)
}

/// Save a session snapshot. Returns the path written.
pub fn save_session(root: &Path, name: &str, messages: &[ChatMessage]) -> Result<PathBuf> {
    let dir = dir_for(root);
    fs::create_dir_all(&dir)?;
    let file = dir.join(format!("{name}.json"));
    let json = serde_json::to_string_pretty(messages)?;
    fs::write(&file, json)?;
    Ok(file)
}

/// Load a saved session. Returns an error if the file is missing/unparseable.
pub fn load_session(root: &Path, name: &str) -> Result<Vec<ChatMessage>> {
    let file = dir_for(root).join(format!("{name}.json"));
    let content = fs::read_to_string(&file)?;
    let messages: Vec<ChatMessage> = serde_json::from_str(&content)?;
    Ok(messages)
}

/// List available session names, newest first (by modified time).
pub fn list_sessions(root: &Path) -> Vec<String> {
    let dir = dir_for(root);
    let mut entries: Vec<(String, std::time::SystemTime)> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .filter_map(|e| {
                let name = e.path().file_stem()?.to_string_lossy().to_string();
                let mtime = e.metadata().ok()?.modified().ok()?;
                Some((name, mtime))
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    entries.into_iter().map(|(n, _)| n).collect()
}

/// True if a saved session exists under `name`.
pub fn session_exists(root: &Path, name: &str) -> bool {
    dir_for(root).join(format!("{name}.json")).is_file()
}

/// Remove a session snapshot.
#[allow(dead_code)]
pub fn delete_session(root: &Path, name: &str) -> Result<()> {
    let file = dir_for(root).join(format!("{name}.json"));
    if file.exists() {
        fs::remove_file(file)?;
    }
    Ok(())
}
