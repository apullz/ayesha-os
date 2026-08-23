use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Result;
use std::time::Duration;
use std::fs;
use std::path::PathBuf;

use crate::llm::{ChatMessage, StreamResult, ToolCall, ChatResponse, ChatResponseMessage};

#[derive(Debug)]
pub struct CloudClient {
    client: Client,
    pub model: String,
    pub provider: String,
    base_url: String,
    api_key: String,
}

impl CloudClient {
    fn find_project_root() -> PathBuf {
        // Try exe directory, then current directory, then ancestor search for a
        // dir that actually contains an ayesha.json or .env marker.
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        {
            let mut cur = dir.clone();
            candidates.push(cur.clone());
            while let Some(parent) = cur.parent() {
                candidates.push(parent.to_path_buf());
                cur = parent.to_path_buf();
            }
        }
        candidates.push(std::env::current_dir().unwrap_or_default());
        for dir in candidates {
            if dir.join("ayesha.json").exists() || dir.join(".env").exists() {
                return dir;
            }
        }
        std::env::current_dir().unwrap_or_default()
    }

fn kilo_auth_json() -> Option<Value> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME")).ok()?;
    let path = PathBuf::from(home)
        .join(".local").join("share").join("ayesha").join("auth.json");
    let s = fs::read_to_string(path).ok()?;
    serde_json::from_str(&s).ok()
}

fn auth_json_key(provider: &str) -> Option<String> {
    Self::kilo_auth_json()?
        .get(provider)?
        .get("key")?
        .as_str()
        .map(|k| k.to_string())
}

    /// Environment variable ayesha-os would resolve `{env:...}` from.
    fn env_key_name(provider: &str) -> Option<&'static str> {
        Some(match provider {
            "kilo" => "KILO_API_KEY",
            _ => return None,
        })
    }

fn get_api_key(provider: &str) -> Option<String> {
    // 1. environment variable (matches ayesha-os's {env:...} option)
    if let Some(var) = Self::env_key_name(provider) {
        if let Ok(v) = std::env::var(var) {
            if !v.trim().is_empty() {
                return Some(v.trim().to_string());
            }
        }
    }
    // 2. ayesha-os's auth store (~/.local/share/ayesha/auth.json)
    if let Some(k) = Self::auth_json_key(provider) {
        return Some(k);
    }
    // 3. legacy project .env
    let root = Self::find_project_root();
    let env_content = fs::read_to_string(root.join(".env")).ok()?;
    let mut key_names: Vec<&'static str> = match Self::env_key_name(provider) {
        Some(name) => vec![name],
        None => vec![],
    };
    if !key_names.contains(&"OPENCODE_API_KEY") {
        key_names.push("OPENCODE_API_KEY");
    }
    for line in env_content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if key_names.contains(&k.trim()) {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn get_base_url(provider: &str) -> Option<String> {
    let root = Self::find_project_root();
    let config_str = fs::read_to_string(root.join("ayesha.json")).ok()?;
    let json_val: Value = serde_json::from_str(&config_str).ok()?;

    let provider_key = "kilo";
    let url = json_val.get("cloud_models")?.get(provider_key)?.get("endpoint")?.as_str()?;
    Some(url.to_string())
}

/// Base URLs mirrored from the user's ayesha-os provider config.
fn provider_base_url(provider: &str) -> Option<&'static str> {
    Some(match provider {
        "kilo" => "https://api.kilo.ai/api/gateway/v1",
        _ => return None,
    })
}

    /// Create a new cloud client. Resolves the API key from the environment,
    /// ayesha-os's auth store, then the legacy project .env.
    /// provider must be "kilo".
    pub fn new(model: &str, provider: &str) -> Result<Self> {
        let api_key = Self::get_api_key(provider)
            .ok_or_else(|| anyhow::anyhow!("API key not found for provider: {}", provider))?;

        let base_url = Self::provider_base_url(provider)
            .map(|u| u.to_string())
            .or_else(|| Self::get_base_url(provider))
            .unwrap_or_else(|| "https://api.kilo.ai/api/gateway/v1".to_string());

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
    #[allow(dead_code)]
    pub fn is_configured(provider: &str) -> bool {
        Self::get_api_key(provider).is_some()
    }

    /// List available cloud models for display
    #[allow(dead_code)]
    pub fn available_models() -> Vec<(String, String, Vec<String>)> {
        vec![
            ("kilo-auto/free".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string(), "thinking".to_string()]),
            ("kilo-auto/small".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string()]),
            ("kilo-auto/efficient".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string()]),
            ("stepfun/step-3.7-flash:free".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string()]),
            ("poolside/laguna-s-2.1:free".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string(), "tools".to_string()]),
            ("poolside/laguna-xs-2.1:free".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string()]),
            ("nvidia/nemotron-3-ultra-550b-a55b:free".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string(), "agentic".to_string()]),
            ("tencent/hy3:free".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string(), "thinking".to_string()]),
            ("kilo-auto/free".to_string(), "kilo".to_string(), vec!["general".to_string(), "coding".to_string(), "agentic".to_string()]),
        ]
    }

    /// Non-streaming chat (same signature pattern as LlmClient::chat)
    #[allow(dead_code)]
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

    /// Build a streaming /chat/completions request for this client.
    fn stream_chat_request(
        &self,
        body: Value,
    ) -> Result<reqwest::RequestBuilder> {
        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        Ok(req.json(&body))
    }

    /// Streaming chat with visible output (same pattern as LlmClient::chat_stream_visible)
    pub async fn chat_stream_visible(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {
        let req_body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "stream": true,
        });

        let mut resp = self.stream_chat_request(req_body)?.send().await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("cloud http error: {}", e);
        }

        let mut decoder = crate::streaming::SseDecoder::new();
        crate::streaming::stream_to_result(&mut resp, steer_rx, true, &mut decoder).await
    }

    /// Streaming chat that collects without printing — used by the tool model
    /// so its deliberation text is invisible to the user.
    pub async fn chat_stream_collect(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[Value]>,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {
        let req_body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "stream": true,
        });

        let mut resp = self.stream_chat_request(req_body)?.send().await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("cloud http error: {}", e);
        }

        let mut decoder = crate::streaming::SseDecoder::new();
        crate::streaming::stream_to_result(&mut resp, steer_rx, false, &mut decoder).await
    }

    /// Stream a vision description of an image (OpenAI-compatible multimodal).
    /// `data_uri` is a full `data:<mime>;base64,<b64>` URI used in an
    /// image_url content part.
    pub async fn chat_with_image(
        &self,
        prompt: &str,
        data_uri: &str,
        steer_rx: &std::sync::mpsc::Receiver<String>,
    ) -> Result<StreamResult> {
        let req_body = json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {"url": data_uri}},
                ]
            }],
            "stream": true,
        });

        let mut resp = self.stream_chat_request(req_body)?.send().await?;
        if let Err(e) = resp.error_for_status_ref() {
            anyhow::bail!("cloud vision http error: {}", e);
        }

        let mut decoder = crate::streaming::SseDecoder::new();
        crate::streaming::stream_to_result(&mut resp, steer_rx, true, &mut decoder).await
    }
}

#[cfg(test)]
mod live_smoke_tests {
    use super::*;

    const FREE_MODEL: &str = "kilo-auto/free";

    #[test]
    #[ignore = "live smoke test: requires a kilo gateway key"]
    fn cloud_client_uses_kilo_url() {
        // When a kilo key is present, kilo-auto/free must hit the kilo
        // gateway with the correct base url. Without
        // a key we just skip — the openrouter fallback is tested separately.
        let Ok(client) = CloudClient::new("kilo-auto/free", "kilo") else {
            println!("skip: no kilo gateway key configured");
            return;
        };
        assert_eq!(client.model, "kilo-auto/free");
        assert_eq!(client.base_url, "https://api.kilo.ai/api/gateway/v1");
    }

    #[test]
    #[ignore = "live smoke test: makes a real free openrouter call"]
    fn cloud_replies_with_new_prompt() {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        drop(tx);
        let result = rt.block_on(async {
            let prompt = crate::llm::LlmClient::system_prompt("senpai", ".");
            let client = CloudClient::new(FREE_MODEL, "openrouter")
                .expect("openrouter key must resolve");
            let tools = crate::tool_defs::tool_definitions_core();
            let tools = crate::streaming::tool_payload_slice(&tools);
            let msgs = vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: prompt,
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "explain in 2-3 sentences the difference between a stack and a heap in rust, with one concrete example of when you must use the heap".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ];
            client.chat_stream_collect(&msgs, Some(tools), &rx).await
        });
        let r = result.expect("cloud stream should succeed");
        println!("--- cloud reply ---\n{}\n--- end ---", r.content);
        println!("has_tool_calls: {}", r.has_tool_calls());
        assert!(!r.content.trim().is_empty(), "cloud model must produce a real answer");
    }
}
