use std::fs;
use anyhow::{Result, bail};
use serde_json::Value;

use crate::sandbox::Sandbox;

const MAX_READ_SIZE: usize = 256 * 1024;

pub struct ToolExecutor {
    sandbox: Sandbox,
}

impl ToolExecutor {
    pub fn new(sandbox: Sandbox) -> Self {
        Self { sandbox }
    }

    pub async fn execute(&self, name: &str, args: &Value) -> Result<String> {
        match name {
            "read_file" => self.read_file(args).await,
            "write_file" => self.write_file(args).await,
            "list_dir" => self.list_dir(args).await,
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
}
