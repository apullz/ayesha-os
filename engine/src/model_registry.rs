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
        if let Ok(additional) = Self::detect_from_ollama().await {
            for m in additional {
                if !self.models.iter().any(|e| e.name == m.name) {
                    self.models.push(m);
                }
            }
        }
        // Add cloud models
        self.detect_cloud_models();
        // Default to big pickle if available (default model is opencode/big-pickle)
        if let Some(idx) = self.models.iter().position(|m| m.name == "opencode/big-pickle") {
            self.active_index = idx;
        } else if let Some(idx) = self.models.iter().position(|m| m.name == "ayesha:latest") {
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

    fn m(name: &str, provider: &str, caps: &[Capability], ctx: u32) -> ModelProfile {
        ModelProfile {
            name: name.into(),
            capabilities: caps.to_vec(),
            context_length: ctx,
            backend: Backend::Cloud { provider: provider.into() },
        }
    }

    fn cloud_models() -> Vec<ModelProfile> {
        let mut v = vec![
            // openrouter (kept for backwards compat)
            Self::m("nvidia/nemotron-3-super-120b-a12b:free", "openrouter", &[Capability::General, Capability::Coding, Capability::Tools, Capability::Agentic], 1_000_000),
            Self::m("meta-llama/llama-3.3-70b-instruct:free", "openrouter", &[Capability::General, Capability::Coding], 131_072),
            Self::m("deepseek/deepseek-r1:free", "openrouter", &[Capability::Thinking, Capability::Coding], 65_536),
            Self::m("qwen/qwen-2.5-coder-32b-instruct:free", "openrouter", &[Capability::Coding, Capability::Tools], 32_768),
            Self::m("xiaomi/mimo-v2.5", "openrouter", &[Capability::General, Capability::Coding, Capability::Tools, Capability::Vision], 1_000_000),
            Self::m("xiaomi/mimo-v2.5-pro", "openrouter", &[Capability::General, Capability::Coding, Capability::Agentic, Capability::Thinking], 1_000_000),
            Self::m("opencode/big-pickle", "opencode", &[Capability::Coding, Capability::Thinking], 200_000),
            // sambanova
            Self::m("Meta-Llama-3.3-70B-Instruct", "sambanova", &[Capability::General, Capability::Coding, Capability::Tools], 131_072),
            Self::m("Meta-Llama-3.1-8B-Instruct", "sambanova", &[Capability::General], 131_072),
            Self::m("Meta-Llama-3.1-70B-Instruct", "sambanova", &[Capability::General, Capability::Coding], 131_072),
            Self::m("DeepSeek-R1", "sambanova", &[Capability::Thinking, Capability::Coding], 131_072),
            Self::m("DeepSeek-V3", "sambanova", &[Capability::General, Capability::Coding, Capability::Tools], 131_072),
            Self::m("Qwen2.5-72B-Instruct", "sambanova", &[Capability::General, Capability::Coding, Capability::Tools], 131_072),
            Self::m("Qwen2.5-Coder-32B-Instruct", "sambanova", &[Capability::Coding, Capability::Tools], 131_072),
            Self::m("Gemma-3-27B-IT", "sambanova", &[Capability::General, Capability::Vision], 131_072),
            // cloudflare
            Self::m("@cf/aisingapore/gemma-sea-lion-v4-27b-it", "cloudflare", &[Capability::General], 32_768),
            Self::m("@cf/deepseek-ai/deepseek-r1-distill-qwen-32b", "cloudflare", &[Capability::Thinking, Capability::Coding], 32_768),
            Self::m("@cf/google/gemma-4-26b-a4b-it", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            Self::m("@cf/ibm-granite/granite-4.0-h-micro", "cloudflare", &[Capability::General, Capability::Tools], 131_072),
            Self::m("@cf/meta/llama-3.1-8b-instruct-fp8", "cloudflare", &[Capability::General], 32_768),
            Self::m("@cf/meta/llama-3.2-11b-vision-instruct", "cloudflare", &[Capability::General, Capability::Vision], 32_768),
            Self::m("@cf/meta/llama-3.2-1b-instruct", "cloudflare", &[Capability::General], 32_768),
            Self::m("@cf/meta/llama-3.2-3b-instruct", "cloudflare", &[Capability::General], 32_768),
            Self::m("@cf/meta/llama-3.3-70b-instruct-fp8-fast", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            Self::m("@cf/meta/llama-4-scout-17b-16e-instruct", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            Self::m("@cf/meta/llama-guard-3-8b", "cloudflare", &[Capability::General], 32_768),
            Self::m("@cf/mistralai/mistral-small-3.1-24b-instruct", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            Self::m("@cf/moonshotai/kimi-k2.6", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            Self::m("@cf/moonshotai/kimi-k2.7-code", "cloudflare", &[Capability::Coding], 131_072),
            Self::m("@cf/nvidia/nemotron-3-120b-a12b", "cloudflare", &[Capability::General, Capability::Coding, Capability::Agentic], 131_072),
            Self::m("@cf/openai/gpt-oss-120b", "cloudflare", &[Capability::General, Capability::Coding, Capability::Tools], 131_072),
            Self::m("@cf/openai/gpt-oss-20b", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            Self::m("@cf/qwen/qwen2.5-coder-32b-instruct", "cloudflare", &[Capability::Coding, Capability::Tools], 32_768),
            Self::m("@cf/qwen/qwen3-30b-a3b-fp8", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            Self::m("@cf/qwen/qwq-32b", "cloudflare", &[Capability::Thinking, Capability::Coding], 32_768),
            Self::m("@cf/zai-org/glm-4.7-flash", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            Self::m("@cf/zai-org/glm-5.2", "cloudflare", &[Capability::General, Capability::Coding], 131_072),
            // replicate
            Self::m("meta/meta-llama-3-70b-instruct", "replicate", &[Capability::General, Capability::Coding], 131_072),
            Self::m("meta/meta-llama-3.1-70b-instruct", "replicate", &[Capability::General, Capability::Coding], 131_072),
            Self::m("meta/meta-llama-3.3-70b-instruct", "replicate", &[Capability::General, Capability::Coding], 131_072),
            Self::m("meta/llama-4-scout-instruct", "replicate", &[Capability::General, Capability::Coding, Capability::Vision], 1_000_000),
            Self::m("meta/llama-4-maverick-instruct", "replicate", &[Capability::General, Capability::Coding, Capability::Vision, Capability::Agentic], 1_000_000),
            // github
            Self::m("gpt-4o", "github", &[Capability::General, Capability::Coding, Capability::Vision, Capability::Tools], 131_072),
            Self::m("gpt-4.1", "github", &[Capability::General, Capability::Coding, Capability::Tools], 1_000_000),
            Self::m("gpt-4.1-mini", "github", &[Capability::General, Capability::Coding, Capability::Tools], 1_000_000),
            Self::m("gpt-4.1-nano", "github", &[Capability::General, Capability::Coding], 1_000_000),
            Self::m("o4-mini", "github", &[Capability::Thinking, Capability::Coding], 200_000),
            Self::m("o3", "github", &[Capability::Thinking, Capability::Coding], 200_000),
            Self::m("claude-sonnet-4-20250514", "github", &[Capability::General, Capability::Coding, Capability::Agentic], 200_000),
            Self::m("meta-llama-3.3-70b-instruct", "github", &[Capability::General, Capability::Coding], 131_072),
            Self::m("meta-llama-4-scout", "github", &[Capability::General, Capability::Coding, Capability::Vision], 1_000_000),
            Self::m("meta-llama-4-maverick", "github", &[Capability::General, Capability::Coding, Capability::Agentic], 1_000_000),
            Self::m("mistral-large-2411", "github", &[Capability::General, Capability::Coding], 131_072),
            Self::m("cohere-command-r-plus-08-2024", "github", &[Capability::General, Capability::Coding, Capability::Tools], 131_072),
            // xai
            Self::m("grok-4", "xai", &[Capability::General, Capability::Coding, Capability::Thinking, Capability::Tools, Capability::Vision], 262_144),
            Self::m("grok-4-mini", "xai", &[Capability::General, Capability::Coding, Capability::Thinking, Capability::Tools], 262_144),
            Self::m("grok-3", "xai", &[Capability::General, Capability::Coding, Capability::Thinking, Capability::Tools], 262_144),
            Self::m("grok-3-mini", "xai", &[Capability::General, Capability::Coding, Capability::Thinking], 131_072),
            // zai
            Self::m("glm-4.7", "zai", &[Capability::General, Capability::Coding, Capability::Tools], 128_000),
            Self::m("glm-4.7-flash", "zai", &[Capability::General, Capability::Coding], 128_000),
            Self::m("glm-4.6", "zai", &[Capability::General, Capability::Coding, Capability::Tools], 128_000),
            Self::m("glm-4.6-air", "zai", &[Capability::General, Capability::Coding], 128_000),
            Self::m("glm-4.5", "zai", &[Capability::General, Capability::Coding, Capability::Tools], 128_000),
            Self::m("glm-4.5-air", "zai", &[Capability::General, Capability::Coding], 128_000),
            Self::m("glm-4.5-flash", "zai", &[Capability::General, Capability::Coding], 128_000),
        ];
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    fn known_models() -> Vec<ModelProfile> {
        let mut v = vec![
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
            // models from the user's opencode ollama config
            ModelProfile {
                name: "ayesha:latest".into(),
                capabilities: vec![
                    Capability::General,
                    Capability::Tools,
                    Capability::Thinking,
                    Capability::Agentic,
                ],
                context_length: 32768,
                backend: Backend::Ollama,
            },
            ModelProfile {
                name: "qwen2.5:0.5b".into(),
                capabilities: vec![Capability::General],
                context_length: 32768,
                backend: Backend::Ollama,
            },
            ModelProfile {
                name: "qwen2.5:3b".into(),
                capabilities: vec![Capability::General, Capability::Coding],
                context_length: 32768,
                backend: Backend::Ollama,
            },
            ModelProfile {
                name: "qwen2.5:7b".into(),
                capabilities: vec![Capability::General, Capability::Coding, Capability::Tools],
                context_length: 32768,
                backend: Backend::Ollama,
            },
            ModelProfile {
                name: "mistral-nemo:12b".into(),
                capabilities: vec![Capability::General, Capability::Coding, Capability::Tools],
                context_length: 131072,
                backend: Backend::Ollama,
            },
            ModelProfile {
                name: "Paul:latest".into(),
                capabilities: vec![Capability::General],
                context_length: 32768,
                backend: Backend::Ollama,
            },
            ModelProfile {
                name: "mistral-small:latest".into(),
                capabilities: vec![Capability::General, Capability::Coding, Capability::Tools],
                context_length: 131072,
                backend: Backend::Ollama,
            },
            ModelProfile {
                name: "gojo:latest".into(),
                capabilities: vec![Capability::General, Capability::Coding],
                context_length: 131072,
                backend: Backend::Ollama,
            },
        ];
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
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
