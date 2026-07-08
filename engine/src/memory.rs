use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub category: String,
    pub content: String,
    pub tags: Vec<String>,
    pub timestamp: String,
    #[serde(default)]
    pub importance: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryStore {
    pub memories: Vec<Memory>,
    pub user_preferences: std::collections::HashMap<String, String>,
    pub learned_facts: Vec<String>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self {
            memories: Vec::new(),
            user_preferences: std::collections::HashMap::new(),
            learned_facts: Vec::new(),
        }
    }
}

impl MemoryStore {
    fn store_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ayesha")
            .join("memory.json")
    }

    pub fn load() -> Self {
        let path = Self::store_path();
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::store_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn add_memory(&mut self, category: &str, content: &str, tags: Vec<String>, importance: u8) -> &Memory {
        let id = format!("mem_{}", self.memories.len());
        let timestamp = chrono_now();
        self.memories.push(Memory {
            id: id.clone(),
            category: category.to_string(),
            content: content.to_string(),
            tags,
            timestamp,
            importance,
        });
        self.memories.last().unwrap()
    }

    pub fn search(&self, query: &str) -> Vec<&Memory> {
        let q = query.to_lowercase();
        self.memories
            .iter()
            .filter(|m| {
                m.content.to_lowercase().contains(&q)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || m.category.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn recent(&self, n: usize) -> Vec<&Memory> {
        self.memories.iter().rev().take(n).collect()
    }

    pub fn by_category(&self, category: &str) -> Vec<&Memory> {
        self.memories
            .iter()
            .filter(|m| m.category == category)
            .collect()
    }

    pub fn set_preference(&mut self, key: &str, value: &str) {
        self.user_preferences.insert(key.to_string(), value.to_string());
    }

    pub fn get_preference(&self, key: &str) -> Option<&str> {
        self.user_preferences.get(key).map(|s| s.as_str())
    }

    pub fn add_fact(&mut self, fact: &str) {
        if !self.learned_facts.iter().any(|f| f == fact) {
            self.learned_facts.push(fact.to_string());
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "memories: {} | preferences: {} | facts: {}",
            self.memories.len(),
            self.user_preferences.len(),
            self.learned_facts.len()
        )
    }
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}s", d.as_secs()))
        .unwrap_or_else(|_| "unknown".to_string())
}
