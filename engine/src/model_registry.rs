use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Capability {
    General,
    Coding,
    Vision,
    Tools,
    Thinking,
}

impl Capability {
    pub fn label(&self) -> &str {
        match self {
            Capability::General => "general",
            Capability::Coding => "coding",
            Capability::Vision => "vision",
            Capability::Tools => "tools",
            Capability::Thinking => "thinking",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub context_length: u32,
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
            auto_route: true,
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
    }

    fn known_models() -> Vec<ModelProfile> {
        vec![
            ModelProfile {
                name: "gemma4:e4b".into(),
                capabilities: vec![
                    Capability::General,
                    Capability::Tools,
                    Capability::Vision,
                    Capability::Thinking,
                ],
                context_length: 8192,
            },
            ModelProfile {
                name: "qwen2.5-coder:14b".into(),
                capabilities: vec![
                    Capability::General,
                    Capability::Tools,
                    Capability::Coding,
                ],
                context_length: 32768,
            },
            ModelProfile {
                name: "gemma3:12b".into(),
                capabilities: vec![Capability::General, Capability::Vision],
                context_length: 8192,
            },
            ModelProfile {
                name: "llama3.2-vision".into(),
                capabilities: vec![Capability::General, Capability::Vision],
                context_length: 8192,
            },
            ModelProfile {
                name: "moondream".into(),
                capabilities: vec![Capability::General, Capability::Vision],
                context_length: 4096,
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
        if lower.contains("gemma4") || lower.contains("ayesha") {
            caps.push(Capability::Vision);
            caps.push(Capability::Tools);
            caps.push(Capability::Thinking);
        }
        if lower.contains("gemma3") {
            caps.push(Capability::Vision);
        }
        if lower.contains("deepseek") || lower.contains("r1") {
            caps.push(Capability::Thinking);
        }
        caps
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
        &self.models[0]
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

    pub fn current(&self) -> &ModelProfile {
        &self.models[self.active_index]
    }

    pub fn list_models(&self) -> String {
        let mut out = String::from("available models:\n");
        for (i, m) in self.models.iter().enumerate() {
            let caps: Vec<&str> = m.capabilities.iter().map(|c| c.label()).collect();
            let arrow = if i == self.active_index && !self.auto_route {
                " << active"
            } else {
                ""
            };
            out.push_str(&format!(
                "  {:<25} [{}]{}\n",
                m.name,
                caps.join(", "),
                arrow
            ));
        }
        out.push_str(&format!(
            "routing: {}\n",
            if self.auto_route { "auto" } else { "manual" }
        ));
        out
    }
}
