use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageRecord {
    pub tool_name: String,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: String,
    pub input_summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptHistory {
    pub records: Vec<ToolUsageRecord>,
    pub current_system_prompt: String,
    pub prompt_version: u32,
    pub refinements: Vec<PromptRefinement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRefinement {
    pub version: u32,
    pub change: String,
    pub reason: String,
    pub timestamp: String,
}

impl Default for PromptHistory {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            current_system_prompt: String::new(),
            prompt_version: 0,
            refinements: Vec::new(),
        }
    }
}

impl PromptHistory {
    fn store_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ayesha")
            .join("prompt_history.json")
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

    pub fn record_usage(&mut self, tool: &str, success: bool, error: Option<String>, input: &str) {
        self.records.push(ToolUsageRecord {
            tool_name: tool.to_string(),
            success,
            error,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| format!("{}s", d.as_secs()))
                .unwrap_or_else(|_| "unknown".to_string()),
            input_summary: input.chars().take(200).collect(),
        });
    }

    pub fn success_rate(&self, tool: &str) -> f64 {
        let relevant: Vec<&ToolUsageRecord> = self.records.iter()
            .filter(|r| r.tool_name == tool)
            .collect();
        if relevant.is_empty() {
            return 1.0;
        }
        let successes = relevant.iter().filter(|r| r.success).count() as f64;
        successes / relevant.len() as f64
    }

    pub fn common_errors(&self, tool: &str) -> Vec<String> {
        self.records.iter()
            .filter(|r| r.tool_name == tool && !r.success)
            .filter_map(|r| r.error.clone())
            .collect()
    }

    pub fn tool_stats(&self) -> Vec<(String, usize, usize, f64)> {
        let mut tools: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
        for record in &self.records {
            let entry = tools.entry(record.tool_name.clone()).or_insert((0, 0));
            entry.0 += 1;
            if record.success {
                entry.1 += 1;
            }
        }
        let mut stats: Vec<_> = tools.iter().map(|(name, (total, success))| {
            let rate = if *total > 0 { *success as f64 / *total as f64 } else { 1.0 };
            (name.clone(), *total, *success, rate)
        }).collect();
        stats.sort_by(|a, b| a.0.cmp(&b.0));
        stats
    }

    pub fn generate_analysis_prompt(&self) -> String {
        let stats = self.tool_stats();
        let stats_text: Vec<String> = stats.iter().map(|(name, total, success, rate)| {
            let bar_len = (rate * 10.0) as usize;
            let bar = format!("{}{}", "█".repeat(bar_len), "░".repeat(10 - bar_len));
            format!("  {} {} {}/{} ({:.0}%)", name, bar, success, total, rate * 100.0)
        }).collect();

        let recent_failures: Vec<String> = self.records.iter()
            .rev()
            .filter(|r| !r.success)
            .take(5)
            .map(|r| format!("  {} → {}", r.tool_name, r.error.as_deref().unwrap_or("unknown")))
            .collect();

        format!(
            r#"analyze this tool usage history and suggest improvements to the system prompt:

tool success rates:
{}

recent failures:
{}

current prompt version: {}

suggest specific changes to the system prompt that would:
1. reduce tool call errors
2. improve parameter accuracy
3. add missing context the model needs
4. clarify tool usage patterns

be specific - quote the exact prompt changes needed."#,
            stats_text.join("\n"),
            if recent_failures.is_empty() { "  none".to_string() } else { recent_failures.join("\n") },
            self.prompt_version,
        )
    }

    pub fn apply_refinement(&mut self, change: &str, reason: &str) {
        self.prompt_version += 1;
        self.refinements.push(PromptRefinement {
            version: self.prompt_version,
            change: change.to_string(),
            reason: reason.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| format!("{}s", d.as_secs()))
                .unwrap_or_else(|_| "unknown".to_string()),
        });
    }
}
