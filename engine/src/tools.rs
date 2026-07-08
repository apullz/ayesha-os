use std::fs;
use anyhow::{Result, bail};
use serde_json::{json, Value};

use crate::sandbox::Sandbox;
use crate::ollama::OllamaClient;
use crate::memory::MemoryStore;
use crate::self_analysis::SelfAnalyzer;
use crate::tool_evolution::ToolEvolver;
use crate::prompt_refinement::PromptHistory;

const MAX_READ_SIZE: usize = 256 * 1024;

pub struct ToolExecutor {
    sandbox: Sandbox,
    project_root: std::path::PathBuf,
}

impl ToolExecutor {
    pub fn new(sandbox: Sandbox) -> Self {
        let project_root = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        Self { sandbox, project_root }
    }

    pub async fn execute(&self, name: &str, args: &Value) -> Result<String> {
        match name {
            "read_file" => self.read_file(args).await,
            "write_file" => self.write_file(args).await,
            "list_dir" => self.list_dir(args).await,
            "generate_html" => self.generate_html(args).await,
            "generate_pixel_art" => self.generate_pixel_art(args).await,
            "remember" => self.remember(args),
            "list_memories" => self.list_memories(args),
            "search_memories" => self.search_memories(args),
            "set_preference" => self.set_preference(args),
            "analyze_self" => self.analyze_self(args).await,
            "list_source_files" => self.list_source_files(),
            "evolve_tools" => self.evolve_tools().await,
            "refine_prompt" => self.refine_prompt().await,
            "get_tool_stats" => self.get_tool_stats(),
            _ => bail!("unknown tool: {}", name),
        }
    }

    async fn read_file(&self, args: &Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;

        let content = fs::read_to_string(&resolved)?;

        if content.len() > MAX_READ_SIZE {
            let truncated = &content[..MAX_READ_SIZE];
            Ok(format!(
                "{}\n\n... [truncated at {} bytes, file is {} bytes total]",
                truncated,
                MAX_READ_SIZE,
                content.len()
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

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&resolved, content)?;

        Ok(format!(
            "wrote {} bytes to '{}'",
            content.len(),
            resolved.display()
        ))
    }

    async fn list_dir(&self, args: &Value) -> Result<String> {
        let path = args["path"].as_str().unwrap_or(".");

        let resolved = self.sandbox.resolve(path)?;

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

    async fn generate_html(&self, args: &Value) -> Result<String> {
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'prompt' argument"))?;

        let output_path = args["output_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'output_path' argument"))?;

        self.sandbox.check_sensitive(output_path)?;
        let resolved = self.sandbox.resolve(output_path)?;

        let html_prompt = format!(
            r#"generate a single self-contained html file for this request:

{}

rules:
- output ONLY the raw html code, no markdown fences
- start with <!DOCTYPE html>
- all css must be embedded in <style> tags
- all js must be embedded in <script> tags
- use emojis, css shapes, inline svgs for visuals - NO external images
- make it interactive and visually appealing
- use modern dark theme with gradients
- include smooth animations

return only the html, nothing else."#,
            prompt
        );

        let client = OllamaClient::new("ayesha");

        use crate::ollama::ChatMessage;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "you are an expert html/css/js developer. output only raw html code, no explanations, no markdown fences.".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: html_prompt,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let response = client.chat(&messages, None).await?;
        let mut html = response.message.content;

        html = html.trim().to_string();
        if html.starts_with("`html") {
            html = html.strip_prefix("`html").unwrap_or(&html).to_string();
        } else if html.starts_with("`") {
            html = html.strip_prefix("`").unwrap_or(&html).to_string();
        }
        if html.ends_with("`") {
            html = html.strip_suffix("`").unwrap_or(&html).to_string();
        }
        html = html.trim().to_string();

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved, &html)?;

        Ok(format!(
            "generated html app ({} bytes) and saved to '{}'",
            html.len(),
            resolved.display()
        ))
    }

    async fn generate_pixel_art(&self, args: &Value) -> Result<String> {
        let description = args["description"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'description' argument"))?;

        let output_path = args["output_path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'output_path' argument"))?;

        self.sandbox.check_sensitive(output_path)?;
        let resolved = self.sandbox.resolve(output_path)?;

        let mut config = crate::pixel_striker::renderer::SpriteSheetConfig::default();

        if let Some(p) = args.get("palette").and_then(|v| v.as_object()) {
            let mut map = std::collections::HashMap::new();
            for (k, val) in p {
                map.insert(k.clone(), val.clone());
            }
            config.palette = crate::pixel_striker::palette::Palette::from_json_map(&map);
        }

        if let Some(states) = args.get("states").and_then(|v| v.as_array()) {
            config.states = states.iter().filter_map(|s| s.as_str().map(String::from)).collect();
        }

        if let Some(d) = args.get("dithering").and_then(|v| v.as_bool()) {
            config.dithering = d;
        }

        if let Some(sp) = args.get("subpixel_detail").and_then(|v| v.as_bool()) {
            config.subpixel_detail = sp;
        }

        let result = crate::pixel_striker::renderer::render_to_file(&config, &resolved)?;

        Ok(format!(
            "generated pixel art sprite sheet: {}\nsaved to '{}'\n  size: {}x{}\n  states: {}\n  total frames: {}",
            description,
            resolved.display(),
            result.width,
            result.height,
            result.states.join(", "),
            result.total_frames,
        ))
    }

    // === NEW: Memory Tools ===

    fn remember(&self, args: &Value) -> Result<String> {
        let category = args["category"].as_str().unwrap_or("general");
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;
        let tags: Vec<String> = args["tags"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let importance = args["importance"].as_u64().unwrap_or(5) as u8;

        let mut store = MemoryStore::load();
        store.add_memory(category, content, tags, importance);
        let id = format!("mem_{}", store.memories.len() - 1);
        store.save()?;

        Ok(format!(
            "saved memory '{}' (id: {}, category: {}, importance: {}/10)",
            content.chars().take(60).collect::<String>(),
            id,
            category,
            importance
        ))
    }

    pub fn list_memories(&self, args: &Value) -> Result<String> {
        let store = MemoryStore::load();

        if let Some(category) = args["category"].as_str() {
            let mems = store.by_category(category);
            if mems.is_empty() {
                return Ok(format!("no memories in category '{}'", category));
            }
            let list: Vec<String> = mems.iter().map(|m| {
                format!("  [{}] {} (importance: {}/10)", m.id, m.content.chars().take(80).collect::<String>(), m.importance)
            }).collect();
            return Ok(format!("memories in '{}':\n{}", category, list.join("\n")));
        }

        let recent = store.recent(20);
        if recent.is_empty() {
            return Ok("no memories stored yet".to_string());
        }

        let list: Vec<String> = recent.iter().map(|m| {
            format!("  [{}] {} ({})", m.id, m.content.chars().take(80).collect::<String>(), m.category)
        }).collect();

        Ok(format!("recent memories ({} total):\n{}", store.memories.len(), list.join("\n")))
    }

    pub fn search_memories(&self, args: &Value) -> Result<String> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'query' argument"))?;

        let store = MemoryStore::load();
        let results = store.search(query);

        if results.is_empty() {
            return Ok(format!("no memories matching '{}'", query));
        }

        let list: Vec<String> = results.iter().map(|m| {
            format!("  [{}] {} ({})", m.id, m.content, m.category)
        }).collect();

        Ok(format!("memories matching '{}':\n{}", query, list.join("\n")))
    }

    pub fn set_preference(&self, args: &Value) -> Result<String> {
        let key = args["key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'key' argument"))?;
        let value = args["value"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'value' argument"))?;

        let mut store = MemoryStore::load();
        store.set_preference(key, value);
        store.save()?;

        Ok(format!("set preference: {} = {}", key, value))
    }

    // === NEW: Self-Analysis Tools ===

    pub async fn analyze_self(&self, args: &Value) -> Result<String> {
        let file = args["file"].as_str().unwrap_or("main.rs");

        let analyzer = SelfAnalyzer::new(self.project_root.clone());
        let files = analyzer.source_files();

        let target = files.iter().find(|f| {
            f.to_string_lossy().contains(file)
        });

        let target = match target {
            Some(f) => f.clone(),
            None => {
                // Try to read from src directory directly
                let path = self.project_root.join("src").join(file);
                if path.exists() {
                    path
                } else {
                    bail!("file '{}' not found in project", file);
                }
            }
        };

        let source = fs::read_to_string(&target)?;
        let relative = target.strip_prefix(&self.project_root)
            .unwrap_or(&target)
            .to_string_lossy();

        let prompt = analyzer.generate_improvement_prompt(&source, &relative);

        let client = OllamaClient::new("ayesha");
        use crate::ollama::ChatMessage;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "you are a senior rust developer reviewing code. be specific and actionable.".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let response = client.chat(&messages, None).await?;
        Ok(format!("analysis of {}:\n\n{}", relative, response.message.content))
    }

    pub fn list_source_files(&self) -> Result<String> {
        let analyzer = SelfAnalyzer::new(self.project_root.clone());
        let files = analyzer.source_files();

        if files.is_empty() {
            return Ok("no source files found".to_string());
        }

        let list: Vec<String> = files.iter().map(|f| {
            let relative = f.strip_prefix(&self.project_root)
                .unwrap_or(f)
                .to_string_lossy();
            let lines = fs::read_to_string(f).map(|s| s.lines().count()).unwrap_or(0);
            format!("  {} ({} lines)", relative, lines)
        }).collect();

        Ok(format!("source files:\n{}", list.join("\n")))
    }

    // === NEW: Tool Evolution ===

    pub async fn evolve_tools(&self) -> Result<String> {
        let existing: Vec<String> = OllamaClient::tool_definitions()
            .iter()
            .filter_map(|t| t["function"]["name"].as_str().map(String::from))
            .collect();

        let evolver = ToolEvolver::new(existing);
        let gaps = evolver.analyze_gaps();

        if gaps.is_empty() {
            return Ok("all standard tool gaps are filled".to_string());
        }

        let mut results = Vec::new();
        for gap in gaps.iter().take(3) {
            match evolver.generate_tool_definition(gap).await {
                Ok(template) => {
                    let code = ToolEvolver::generate_tool_code(&template);
                    results.push(format!(
                        "suggested tool: {}\n  description: {}\n  params: {}\n  implementation:\n{}\n",
                        template.name,
                        template.description,
                        template.parameters.iter().map(|p| format!("{} ({})", p.name, p.param_type)).collect::<Vec<_>>().join(", "),
                        code
                    ));
                }
                Err(e) => {
                    results.push(format!("failed to generate tool for '{}': {}", gap, e));
                }
            }
        }

        Ok(format!("tool evolution suggestions:\n\n{}", results.join("\n---\n")))
    }

    // === NEW: Prompt Refinement ===

    pub async fn refine_prompt(&self) -> Result<String> {
        let history = PromptHistory::load();

        if history.records.is_empty() {
            return Ok("no tool usage history yet. keep using tools and come back later.".to_string());
        }

        let prompt = history.generate_analysis_prompt();

        let client = OllamaClient::new("ayesha");
        use crate::ollama::ChatMessage;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "you are a prompt engineer optimizing a system prompt for a coding assistant. be specific about what to add/change.".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let response = client.chat(&messages, None).await?;
        Ok(format!(
            "prompt refinement analysis (based on {} tool calls):\n\n{}",
            history.records.len(),
            response.message.content
        ))
    }

    pub fn get_tool_stats(&self) -> Result<String> {
        let history = PromptHistory::load();

        if history.records.is_empty() {
            return Ok("no tool usage history yet".to_string());
        }

        let stats = history.tool_stats();
        let stats_text: Vec<String> = stats.iter().map(|(name, total, success, rate)| {
            let bar_len = (rate * 10.0) as usize;
            let bar = format!("{}{}", "█".repeat(bar_len), "░".repeat(10 - bar_len));
            format!("  {} {} {}/{} ({:.0}%)", name, bar, success, total, rate * 100.0)
        }).collect();

        Ok(format!(
            "tool usage stats ({} total calls):\n{}\nprompt refinement version: {}",
            history.records.len(),
            stats_text.join("\n"),
            history.prompt_version
        ))
    }
}
