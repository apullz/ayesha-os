use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::ollama::{OllamaClient, ChatMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTemplate {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParamDef>,
    pub implementation_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

pub struct ToolEvolver {
    existing_tools: Vec<String>,
}

impl ToolEvolver {
    pub fn new(existing_tools: Vec<String>) -> Self {
        Self { existing_tools }
    }

    pub fn analyze_gaps(&self) -> Vec<String> {
        let mut gaps = Vec::new();

        let has = |name: &str| self.existing_tools.iter().any(|t| t == name);

        if !has("execute_command") {
            gaps.push("shell command execution (with sandbox)".to_string());
        }
        if !has("search_files") {
            gaps.push("content search across files (grep-like)".to_string());
        }
        if !has("search_web") {
            gaps.push("web search capability".to_string());
        }
        if !has("edit_file") {
            gaps.push("partial file editing (find and replace)".to_string());
        }
        if !has("copy_file") {
            gaps.push("file copy/move operations".to_string());
        }
        if !has("run_tests") {
            gaps.push("test execution and reporting".to_string());
        }
        if !has("git_status") {
            gaps.push("git integration (status, diff, commit)".to_string());
        }
        if !has("remember") {
            gaps.push("persistent memory/knowledge storage".to_string());
        }
        if !has("analyze_self") {
            gaps.push("self-code-analysis".to_string());
        }
        if !has("evolve_tools") {
            gaps.push("tool evolution and generation".to_string());
        }
        if !has("refine_prompt") {
            gaps.push("prompt refinement from history".to_string());
        }
        if !has("list_memories") {
            gaps.push("memory retrieval and search".to_string());
        }

        gaps
    }

    pub async fn generate_tool_definition(&self, gap: &str) -> Result<ToolTemplate> {
        let client = OllamaClient::new("ayesha");

        let prompt = format!(
            r#"you are a rust developer designing a new tool for a CLI agent.

existing tools: {}

gap to fill: {}

generate a tool definition as JSON with this structure:
{{
  "name": "tool_name",
  "description": "what the tool does",
  "parameters": [
    {{ "name": "param_name", "param_type": "string|integer|boolean", "description": "what it does", "required": true }}
  ],
  "implementation_hint": "brief description of how to implement this in rust"
}}

output ONLY the JSON, no explanation."#,
            self.existing_tools.join(", "),
            gap,
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "you are a tool architect. output only valid JSON.".to_string(),
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
        let content = response.message.content.trim();

        // Extract JSON from response
        let json_str = if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                &content[start..=end]
            } else {
                content
            }
        } else {
            content
        };

        let template: ToolTemplate = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("failed to parse tool template: {}", e))?;

        Ok(template)
    }

    pub fn generate_tool_code(template: &ToolTemplate) -> String {
        let params: Vec<String> = template.parameters.iter().map(|p| {
            format!("    let {} = args[\"{}\"]\n        .as_{}()\n        .ok_or_else(|| anyhow::anyhow!(\"missing '{}' argument\"))?;",
                p.name, p.name,
                if p.param_type == "string" { "str" }
                else if p.param_type == "integer" { "i64" }
                else { "bool" },
                p.name)
        }).collect();

        format!(
            r#"async fn {}(&self, args: &Value) -> Result<String> {{
    // TODO: implement - {}
    {}

    Ok(format!("{} executed successfully"))
}}"#,
            template.name,
            template.description,
            params.join("\n\n"),
            template.name,
        )
    }
}
