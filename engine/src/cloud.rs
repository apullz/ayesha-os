use colored::Colorize;
use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Result;
use std::time::Duration;
use std::fs;

use crate::ollama::{ChatMessage, StreamResult, ToolCall, ToolFunction, ChatResponse, ChatResponseMessage};

#[derive(Debug)]
pub struct CloudClient {
    client: Client,
    pub model: String,
    pub provider: String,
    base_url: String,
    api_key: String,
}

impl CloudClient {
    fn get_api_key(provider: &str) -> Option<String> {
        let env_content = fs::read_to_string("C:\\ayesha-os\\.env").ok()?;
        let key_name = if provider == "openrouter" {
            "OPENROUTER_API_KEY"
        } else {
            "OPENCODE_API_KEY"
        };
        for line in env_content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == key_name {
                    return Some(v.trim().to_string());
                }
            }
        }
        None
    }

    fn get_base_url(provider: &str) -> Option<String> {
        let config_str = fs::read_to_string("C:\\ayesha-os\\ayesha.json").ok()?;
        let json_val: Value = serde_json::from_str(&config_str).ok()?;
        
        let provider_key = if provider == "openrouter" { "openrouter" } else { "opencode_zen" };
        let url = json_val.get("cloud_models")?.get(provider_key)?.get("endpoint")?.as_str()?;
        Some(url.to_string())
    }

    /// Create a new cloud client. Reads API key from .env file.
    /// provider is "openrouter" or "opencode"
    pub fn new(model: &str, provider: &str) -> Result<Self> {
        let api_key = Self::get_api_key(provider)
            .ok_or_else(|| anyhow::anyhow!("API key not found in .env for provider: {}", provider))?;
            
        let base_url = Self::get_base_url(provider)
            .unwrap_or_else(|| {
                if provider == "openrouter" {
                    "https://openrouter.ai/api/v1".to_string()
                } else {
                    "https://api.opencode.ai/v1".to_string()
                }
            });

        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()?,
            model: model.to_string(),
            provider: provider.to_string(),
            base_url,
            api_key,
        })
    }

    /// Check if cloud is configured (API key exists in .env)
    pub fn is_configured(provider: &str) -> bool {
        Self::get_api_key(provider).is_some()
    }

    /// List available cloud models for display
    pub fn available_models() -> Vec<(String, String, Vec<String>)> {
        vec![
            ("nvidia/nemotron-3-super:free".to_string(), "openrouter".to_string(), vec!["general".to_string(), "coding".to_string(), "agentic".to_string()]),
            ("meta-llama/llama-3.3-70b-instruct:free".to_string(), "openrouter".to_string(), vec!["general".to_string(), "coding".to_string()]),
            ("deepseek/deepseek-r1:free".to_string(), "openrouter".to_string(), vec!["thinking".to_string(), "coding".to_string()]),
            ("qwen/qwen-2.5-coder-32b-instruct:free".to_string(), "openrouter".to_string(), vec!["coding".to_string(), "tools".to_string()]),
            ("opencode/big-pickle".to_string(), "opencode".to_string(), vec!["coding".to_string(), "reasoning".to_string()]),
        ]
    }

    /// Non-streaming chat (same signature pattern as OllamaClient::chat)
    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
    ) -> Result<ChatResponse> {
        let req_body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "stream": false,
        });

        let mut req = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if self.provider == "openrouter" {
            req = req.header("HTTP-Referer", "https://github.com/apullz/ayesha-os")
                     .header("X-Title", "ayesha-os");
        }

        let resp = req.json(&req_body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            anyhow::bail!("cloud http error {}: {}", status, text);
        }

        let body = resp.text().await?;
        let json_resp: Value = serde_json::from_str(&body)?;

        let mut chat_msg = ChatResponseMessage::default();

        if let Some(choices) = json_resp.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(msg) = choice.get("message") {
                    if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
                        chat_msg.role = role.to_string();
                    }
                    if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                        chat_msg.content = content.to_string();
                    }
                    if let Some(tcs) = msg.get("tool_calls") {
                        if let Ok(parsed) = serde_json::from_value::<Vec<ToolCall>>(tcs.clone()) {
                            chat_msg.tool_calls = parsed;
                        }
                    }
                }
            }
        }

        Ok(ChatResponse { message: chat_msg })
    }

    /// Streaming chat with visible output (same pattern as OllamaClient::chat_stream_visible)
    pub async fn chat_stream_visible(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {
        use std::io::Write;

        let req_body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "stream": true,
        });

        let mut req = self.client.post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if self.provider == "openrouter" {
            req = req.header("HTTP-Referer", "https://github.com/apullz/ayesha-os")
                     .header("X-Title", "ayesha-os");
        }

        let mut resp = req.json(&req_body).send().await?;

        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("cloud http error: {}", e);
        }

        let mut buf = String::new();
        let mut full_content = String::new();
        let mut in_think = false;

        let mut recent = String::new();
        const RECENT_MAX: usize = 20;

        #[derive(Default)]
        struct ActiveToolCall {
            id: String,
            call_type: String,
            name: String,
            args: String,
        }
        let mut active_tool_calls: Vec<ActiveToolCall> = Vec::new();

        while let Some(chunk) = resp.chunk().await? {
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(nl) = buf.find('\n') {
                let line = buf[..nl].to_string();
                buf = buf[nl + 1..].to_string();

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == "data: [DONE]" {
                    break;
                }
                
                if let Some(data_str) = trimmed.strip_prefix("data: ") {
                    match serde_json::from_str::<Value>(data_str) {
                        Ok(json_obj) => {
                            if let Some(choices) = json_obj.get("choices").and_then(|c| c.as_array()) {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice.get("delta") {
                                        // Handle content
                                        if let Some(c) = delta.get("content").and_then(|c| c.as_str()) {
                                            if !c.is_empty() {
                                                full_content.push_str(c);
                                                recent.push_str(c);
                                                if recent.len() > RECENT_MAX {
                                                    recent = recent[recent.len() - RECENT_MAX..].to_string();
                                                }

                                                let no_think = !full_content.contains("<think>") && !full_content.contains("[think]");
                                                let think_ended = full_content.contains("</think>") || full_content.contains("[/think]");

                                                if !in_think && !no_think {
                                                    in_think = true;
                                                }
                                                if in_think && think_ended {
                                                    in_think = false;
                                                }

                                                if in_think {
                                                    print!("{}", c.bright_black());
                                                } else {
                                                    print!("{}", c);
                                                }
                                                std::io::stdout().flush().ok();
                                            }
                                        }

                                        // Handle tool calls
                                        if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                            for tc in tcs {
                                                if let Some(index) = tc.get("index").and_then(|i| i.as_u64()) {
                                                    let idx = index as usize;
                                                    while active_tool_calls.len() <= idx {
                                                        active_tool_calls.push(ActiveToolCall::default());
                                                    }
                                                    let active = &mut active_tool_calls[idx];

                                                    if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                                        active.id = id.to_string();
                                                    }
                                                    if let Some(t) = tc.get("type").and_then(|t| t.as_str()) {
                                                        active.call_type = t.to_string();
                                                    }
                                                    if let Some(f) = tc.get("function") {
                                                        if let Some(name) = f.get("name").and_then(|n| n.as_str()) {
                                                            active.name = name.to_string();
                                                        }
                                                        if let Some(args) = f.get("arguments").and_then(|a| a.as_str()) {
                                                            active.args.push_str(args);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            let preview = &data_str[..data_str.len().min(80)];
                            eprintln!("stream parse error: {} near: {}", e, preview);
                        }
                    }
                }
            }

            // Check for steering between chunks
            if let Ok(input) = steer_rx.try_recv() {
                println!();
                
                let mut final_tcs = Vec::new();
                for a in &active_tool_calls {
                    let args_val = if a.args.is_empty() {
                        json!({})
                    } else {
                        serde_json::from_str(&a.args).unwrap_or(json!({}))
                    };
                    final_tcs.push(ToolCall {
                        id: a.id.clone(),
                        call_type: if a.call_type.is_empty() { "function".to_string() } else { a.call_type.clone() },
                        function: ToolFunction {
                            name: a.name.clone(),
                            arguments: args_val,
                        }
                    });
                }
                
                return Ok(StreamResult { content: full_content, tool_calls: final_tcs, steering: Some(input) });
            }
        }

        println!();

        let mut final_tcs = Vec::new();
        for a in active_tool_calls {
            let args_val = if a.args.is_empty() {
                json!({})
            } else {
                serde_json::from_str(&a.args).unwrap_or(json!({}))
            };
            final_tcs.push(ToolCall {
                id: a.id,
                call_type: if a.call_type.is_empty() { "function".to_string() } else { a.call_type },
                function: ToolFunction {
                    name: a.name,
                    arguments: args_val,
                }
            });
        }

        Ok(StreamResult { content: full_content, tool_calls: final_tcs, steering: None })
    }
}
