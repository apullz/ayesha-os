use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Backend {
    Ollama,
    Cloud { provider: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Capability {
    General,
    Coding,
    Vision,
    Tools,
    Thinking,
    Agentic,
}

impl Capability {
    pub fn label(&self) -> &str {
        match self {
            Capability::General => "general",
            Capability::Coding => "coding",
            Capability::Vision => "vision",
            Capability::Tools => "tools",
            Capability::Thinking => "thinking",
            Capability::Agentic => "agentic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub context_length: u32,
    #[serde(default = "default_backend")]
    pub backend: Backend,
}

fn default_backend() -> Backend {
    Backend::Ollama
}

pub struct ModelRegistry {
    pub models: Vec<ModelProfile>,
    pub active_index: usize,
    pub auto_route: bool,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: Self::known_models(),
            active_index: 0,
            auto_route: false,
        }
    }

    pub async fn detect(&mut self) {
        match Self::detect_from_ollama().await {
            Ok(additional) => {
                for m in additional {
                    if !self.models.iter().any(|e| e.name == m.name) {
                        self.models.push(m);
                    }
                }
            }
            Err(_) => {}
        }
        // Add cloud models
        self.detect_cloud_models();
        // Default to ayesha:latest if available
        if let Some(idx) = self.models.iter().position(|m| m.name == "ayesha:latest") {
            self.active_index = idx;
        }
    }

    fn detect_cloud_models(&mut self) {
        let cloud_models = Self::cloud_models();
        for m in cloud_models {
            if !self.models.iter().any(|e| e.name == m.name) {
                self.models.push(m);
            }
        }
    }

    fn cloud_models() -> Vec<ModelProfile> {
        vec![
            ModelProfile {
                name: "nvidia/nemotron-3-super:free".into(),
                capabilities: vec![
                    Capability::General,
                    Capability::Coding,
                    Capability::Tools,
                    Capability::Agentic,
                ],
                context_length: 1_000_000,
                backend: Backend::Cloud { provider: "openrouter".into() },
            },
            ModelProfile {
                name: "meta-llama/llama-3.3-70b-instruct:free".into(),
                capabilities: vec![
                    Capability::General,
                    Capability::Coding,
                ],
                context_length: 131_072,
                backend: Backend::Cloud { provider: "openrouter".into() },
            },
            ModelProfile {
                name: "deepseek/deepseek-r1:free".into(),
                capabilities: vec![
                    Capability::Thinking,
                    Capability::Coding,
                ],
                context_length: 65_536,
                backend: Backend::Cloud { provider: "openrouter".into() },
            },
            ModelProfile {
                name: "qwen/qwen-2.5-coder-32b-instruct:free".into(),
                capabilities: vec![
                    Capability::Coding,
                    Capability::Tools,
                ],
                context_length: 32_768,
                backend: Backend::Cloud { provider: "openrouter".into() },
            },
            ModelProfile {
                name: "opencode/big-pickle".into(),
                capabilities: vec![
                    Capability::Coding,
                    Capability::Thinking,
                ],
                context_length: 200_000,
                backend: Backend::Cloud { provider: "opencode".into() },
            },
        ]
    }

    fn known_models() -> Vec<ModelProfile> {
        vec![
            ModelProfile {
                name: "qwen2.5-coder:14b".into(),
                capabilities: vec![
                    Capability::General,
                    Capability::Tools,
                    Capability::Coding,
                    Capability::Thinking,
                ],
                context_length: 32768,
                backend: Backend::Ollama,
            },
            ModelProfile {
                name: "llama3.2-vision".into(),
                capabilities: vec![Capability::General, Capability::Vision],
                context_length: 8192,
                backend: Backend::Ollama,
            },
        ]
    }

    async fn detect_from_ollama() -> anyhow::Result<Vec<ModelProfile>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        let resp = client
            .get("http://localhost:11434/api/tags")
            .send()
            .await?;
        let body: serde_json::Value = resp.json().await?;
        let models = body["models"].as_array().cloned().unwrap_or_default();
        Ok(models
            .iter()
            .map(|m| {
                let name = m["name"].as_str().unwrap_or("unknown").to_string();
                ModelProfile {
                    capabilities: Self::infer_capabilities(&name),
                    context_length: 4096,
                    name,
                    backend: Backend::Ollama,
                }
            })
            .collect())
    }

    fn infer_capabilities(name: &str) -> Vec<Capability> {
        let lower = name.to_lowercase();
        let mut caps = vec![Capability::General];
        if lower.contains("coder") || lower.contains("qwen") {
            caps.push(Capability::Coding);
            caps.push(Capability::Tools);
        }
        if lower.contains("vision") || lower.contains("moondream") || lower.contains("llava") {
            caps.push(Capability::Vision);
        }
        if lower.contains("ayesha") {
            caps.push(Capability::Tools);
            caps.push(Capability::Thinking);
        }
        if lower.contains("gemma") || lower.contains("llava") {
            caps.push(Capability::Vision);
        }
        if lower.contains("deepseek") || lower.contains("r1") {
            caps.push(Capability::Thinking);
        }
        if lower.contains("nemotron") {
            caps.push(Capability::Agentic);
            caps.push(Capability::Coding);
        }
        caps
    }

    /// Check if a model uses a cloud backend
    pub fn is_cloud_model(&self, name: &str) -> bool {
        self.models.iter().any(|m| m.name == name && matches!(m.backend, Backend::Cloud { .. }))
    }

    /// Get the provider for a cloud model
    pub fn cloud_provider(&self, name: &str) -> Option<String> {
        self.models.iter().find(|m| m.name == name).and_then(|m| {
            if let Backend::Cloud { ref provider } = m.backend {
                Some(provider.clone())
            } else {
                None
            }
        })
    }

    pub fn select_model(&self, query: &str) -> &ModelProfile {
        if !self.auto_route {
            return &self.models[self.active_index];
        }
        let lower = query.to_lowercase();
        let coding_keywords = [
            "code",
            "implement",
            "function",
            "class",
            "algorithm",
            "optimize",
            "refactor",
            "rust",
            "python",
            "javascript",
            "typescript",
            "write a",
            "program",
            "debug",
            "compile",
        ];
        if coding_keywords
            .iter()
            .any(|k| lower.contains(k))
        {
            if let Some(m) = self
                .models
                .iter()
                .find(|m| m.capabilities.contains(&Capability::Coding))
            {
                return m;
            }
        }
        let vision_keywords = [
            "image",
            "picture",
            "vision",
            "see",
            "look at",
            "screenshot",
            "screen",
            "view",
            "photo",
            "camera",
        ];
        if vision_keywords
            .iter()
            .any(|k| lower.contains(k))
        {
            if let Some(m) = self
                .models
                .iter()
                .find(|m| m.capabilities.contains(&Capability::Vision))
            {
                return m;
            }
        }
        &self.models[self.active_index]
    }

    pub fn set_model(&mut self, name: &str) -> anyhow::Result<()> {
        let idx = self
            .models
            .iter()
            .position(|m| m.name == name)
            .ok_or_else(|| anyhow::anyhow!("model '{}' not found", name))?;
        self.active_index = idx;
        self.auto_route = false;
        Ok(())
    }

    pub fn set_auto_route(&mut self, enabled: bool) {
        self.auto_route = enabled;
    }

    #[allow(dead_code)]
    pub fn current(&self) -> &ModelProfile {
        &self.models[self.active_index]
    }

    pub fn list_models(&self) -> String {
        let mut out = String::from("available models:\n");
        let mut last_backend = "";
        // Show local models first, then cloud
        let mut sorted: Vec<(usize, &ModelProfile)> = self.models.iter().enumerate().collect();
        sorted.sort_by_key(|(_, m)| matches!(m.backend, Backend::Cloud { .. }));
        for (i, m) in sorted {
            let backend_label = match &m.backend {
                Backend::Ollama => "local",
                Backend::Cloud { .. } => "cloud",
            };
            if backend_label != last_backend {
                out.push_str(&format!("\n  ── {} ──\n", backend_label));
                last_backend = backend_label;
            }
            let caps: Vec<&str> = m.capabilities.iter().map(|c| c.label()).collect();
            let arrow = if i == self.active_index && !self.auto_route {
                " << active"
            } else {
                ""
            };
            let provider_tag = match &m.backend {
                Backend::Cloud { provider } => format!(" ({})", provider),
                _ => String::new(),
            };
            out.push_str(&format!(
                "  {:<40} [{}]{}{}\n",
                m.name,
                caps.join(", "),
                provider_tag,
                arrow
            ));
        }
        out.push_str(&format!(
            "\nrouting: {}\n",
            if self.auto_route { "auto" } else { "manual" }
        ));
        out
    }
}
