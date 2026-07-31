use crate::sandbox::Sandbox;
use crate::ollama::OllamaClient;
use crate::memory::MemoryStore;
use crate::self_analysis::SelfAnalyzer;
use crate::tool_evolution::ToolEvolver;
use crate::prompt_refinement::PromptHistory;
use crate::applet_manager::AppletManager;
use anyhow::Result;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

/// Coding Agent - Capable of understanding and modifying code in the codebase
#[allow(dead_code)]
pub struct CodingAgent {
    sandbox: Sandbox,
    ollama: OllamaClient,
    memory: MemoryStore,
    analyzer: SelfAnalyzer,
    evolver: ToolEvolver,
    prometh: PromptHistory,
    applet_manager: AppletManager,
    project_root: PathBuf,
}

#[allow(dead_code)]
impl CodingAgent {
    pub fn new(sandbox: Sandbox, ollama: OllamaClient, memory: MemoryStore, 
               analyzer: SelfAnalyzer, evolver: ToolEvolver, 
               prometh: PromptHistory, applet_manager: AppletManager,
               project_root: PathBuf) -> Self {
        Self {
            sandbox,
            ollama,
            memory,
            analyzer,
            evolver,
            prometh,
            applet_manager,
            project_root,
        }
    }

    /// Main entry point for the coding agent tool
    pub async fn execute(&self, args: &Value) -> Result<String> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' argument"))?;

        match action {
            "read" => self.read_file(args).await,
            "write" => self.write_file(args).await,
            "edit" => self.edit_file(args).await,
            "list" => self.list_dir(args).await,
            "analyze" => self.analyze_code(args).await,
            "modify" => self.modify_code(args).await,
            "suggest" => self.suggest_improvements(args).await,
            "execute_tool" => self.execute_tool(args).await,
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }

    /// Read a file from the codebase
    async fn read_file(&self, args: &Value) -> Result<String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        
        let full_path = self.project_root.join(path);
        let content = fs::read_to_string(&full_path)
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path, e))?;
        
        Ok(json!({
            "path": path,
            "content": content
        }).to_string())
    }

    /// Write a file to the codebase
    async fn write_file(&self, args: &Value) -> Result<String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        let content = args.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' argument"))?;
        
        let full_path = self.project_root.join(path);
        // Ensure parent directories exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&full_path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write file {}: {}", path, e))?;
        
        Ok(json!({
            "path": path,
            "status": "written",
            "bytes": content.len()
        }).to_string())
    }

    /// Edit a file using AST-aware edits (using ast_edit tool internally)
    async fn edit_file(&self, args: &Value) -> Result<String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        let edits = args.get("edits")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid 'edits' array"))?;

        // For now, we'll use a simple approach - in the future this could use ast_edit
        let full_path = self.project_root.join(path);
        let mut content = fs::read_to_string(&full_path)
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path, e))?;

        // Apply edits in reverse order to maintain line numbers
        let mut edits_clone = edits.clone();
        edits_clone.reverse();
        
        for edit in edits_clone {
            let action = edit.get("action")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Edit missing 'action'"))?;
            
            match action {
                "replace" => {
                    let start = edit.get("start")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| anyhow::anyhow!("Replace edit missing 'start'"))? as usize;
                    let end = edit.get("end")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| anyhow::anyhow!("Replace edit missing 'end'"))? as usize;
                    let replacement = edit.get("replacement")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("Replace edit missing 'replacement'"))?;
                    
                    if start > content.len() || end > content.len() || start > end {
                        return Err(anyhow::anyhow!("Invalid range for replacement"));
                    }
                    if !content.is_char_boundary(start) || !content.is_char_boundary(end) {
                        return Err(anyhow::anyhow!("start/end offsets {} / {} fall inside a multi-byte character", start, end));
                    }
                    
                    let mut new_content = String::new();
                    new_content.push_str(&content[..start]);
                    new_content.push_str(replacement);
                    new_content.push_str(&content[end..]);
                    content = new_content;
                }
                "insert" => {
                    let line = edit.get("line")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| anyhow::anyhow!("Insert edit missing 'line'"))? as usize;
                    let text = edit.get("text")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("Insert edit missing 'text'"))?;
                    
                    if line > content.lines().count() {
                        return Err(anyhow::anyhow!("Line number out of bounds"));
                    }
                    
                    let mut lines: Vec<&str> = content.lines().collect();
                    lines.insert(line, text);
                    content = lines.join("\n");
                }
                "delete" => {
                    let start = edit.get("start")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| anyhow::anyhow!("Delete edit missing 'start'"))? as usize;
                    let end = edit.get("end")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| anyhow::anyhow!("Delete edit missing 'end'"))? as usize;
                    
                    if start > content.len() || end > content.len() || start > end {
                        return Err(anyhow::anyhow!("Invalid range for deletion"));
                    }
                    if !content.is_char_boundary(start) || !content.is_char_boundary(end) {
                        return Err(anyhow::anyhow!("start/end offsets {} / {} fall inside a multi-byte character", start, end));
                    }
                    
                    let mut new_content = String::new();
                    new_content.push_str(&content[..start]);
                    new_content.push_str(&content[end..]);
                    content = new_content;
                }
                _ => return Err(anyhow::anyhow!("Unknown edit action: {}", action)),
            }
        }

        fs::write(&full_path, &content)
            .map_err(|e| anyhow::anyhow!("Failed to write file {}: {}", path, e))?;
        
        Ok(json!({
            "path": path,
            "status": "edited",
            "bytes": content.len()
        }).to_string())
    }

    /// List directory contents
    async fn list_dir(&self, args: &Value) -> Result<String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        
        let full_path = self.project_root.join(path);
        let mut entries = Vec::new();
        
        if full_path.is_dir() {
            for entry in fs::read_dir(&full_path)? {
                let entry = entry?;
                let path = entry.path();
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("").to_string();
                
                let entry_type = if path.is_dir() { "directory" } else { "file" };
                let size = if path.is_file() {
                    path.metadata().map(|m| m.len()).unwrap_or(0)
                } else { 0 };
                
                entries.push(json!({
                    "name": name,
                    "type": entry_type,
                    "size": size
                }));
            }
        } else {
            return Err(anyhow::anyhow!("Path is not a directory: {}", path));
        }
        
        Ok(json!({
            "path": path,
            "entries": entries
        }).to_string())
    }

    /// Analyze code for issues, patterns, or suggestions
    async fn analyze_code(&self, args: &Value) -> Result<String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        
        let content_json = self.read_file(args).await?;
        let content_val: serde_json::Value = serde_json::from_str(&content_json)?;
        let content = content_val
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Could not extract content"))?
            .to_string();
        
        // Use the self-analyzer to get insights
        let analysis = self.analyzer.analyze_for_improvements(&content);
        
        Ok(json!({
            "path": path,
            "analysis": analysis
        }).to_string())
    }

    /// Modify code based on natural language description
    async fn modify_code(&self, args: &Value) -> Result<String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        let instruction = args.get("instruction")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'instruction' argument"))?;
        
        // Read the file
        let content_json = self.read_file(args).await?;
        let content_val: serde_json::Value = serde_json::from_str(&content_json)?;
        let content = content_val
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Could not extract content"))?
            .to_string();
        
        // Use Ollama to generate the modification
        let prompt = format!(
            "You are a coding agent. Modify the following code according to the instruction.\n\n\
            FILE: {}\n\nCURRENT CODE:\n{}\n\nINSTRUCTION:\n{}\n\n\
            Provide ONLY the modified code in your response, without any explanations or markdown formatting.",
            path, content, instruction
        );
        
        let messages = vec![crate::ollama::ChatMessage {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
        }];
        let response = self.ollama.chat(&messages, None).await?;
        let modified_code = response.message.content.trim();
        
        // Write the modified code back
        self.write_file(&json!({
            "path": path,
            "content": modified_code
        })).await?;
        
        Ok(json!({
            "path": path,
            "status": "modified",
            "bytes": modified_code.len()
        }).to_string())
    }

    /// Suggest improvements for code
    async fn suggest_improvements(&self, args: &Value) -> Result<String> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        
        let content_json = self.read_file(args).await?;
        let content_val: serde_json::Value = serde_json::from_str(&content_json)?;
        let content = content_val
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Could not extract content"))?
            .to_string();
        
        // Use Ollama to suggest improvements
        let prompt = format!(
            "You are a senior software engineer. Review the following code and suggest improvements.\n\n\
            FILE: {}\n\nCODE:\n{}\n\n\
            Provide specific, actionable suggestions for improving the code. Focus on:\n\
            - Code quality and readability\n\
            - Performance improvements\n\
            - Best practices and idioms\n\
            - Potential bugs or issues\n\n\
            Format your response as a bullet-point list.",
            path, content
        );
        
        let messages = vec![crate::ollama::ChatMessage {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
        }];
        let suggestions = self.ollama.chat(&messages, None).await?;
        let suggestions = suggestions.message.content;
        
        Ok(json!({
            "path": path,
            "suggestions": suggestions
        }).to_string())
    }

    /// Execute another tool through the coding agent
    async fn execute_tool(&self, _args: &Value) -> Result<String> {
        Ok("tool calls are handled by the main agent loop — just call any tool directly from your response, and it will be executed automatically. no need to use coding_agent for tool delegation.".to_string())
    }
}