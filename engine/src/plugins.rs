//! Config-driven plugin tools (declarative only — no native loading).
//!
//! `ayesha.json` may carry an optional `"plugins"` array:
//!
//! ```json
//! {
//!   "plugins": [
//!     {
//!       "name": "my-helper",
//!       "prompt": "you can use the my-helper plugin to do X",
//!       "tools": [
//!         {
//!           "name": "helper_search",
//!           "description": "search an external index",
//!           "parameters": { "type": "object", "properties": { "query": { "type": "string" } }, "required": ["query"] },
//!           "command": "python -m my_helper search {args}"
//!         }
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! - `name`: plugin id (used in the system-prompt snippet).
//! - `prompt`: a system-prompt snippet appended to the agent's system prompt.
//! - `tools[].name/description/parameters`: the model-facing tool definition
//!   (same JSON shape as the built-in catalog entries).
//! - `tools[].command`: shell command template. Execution model:
//!   the template is split on whitespace — first token is the program, the
//!   rest are static args. If any static arg contains `{args}`, that
//!   placeholder is replaced with the tool-call arguments serialized as JSON;
//!   otherwise the JSON is piped to the process's stdin. The working
//!   directory is the project root. Output (stdout + stderr tail) is captured
//!   and capped at 64 KiB; a 30s timeout kills runaway plugins.
//!
//! Plugin tools are merged into the per-request tool payload in main.rs
//! (without touching tool_defs.rs) and dispatched through the fallback arm of
//! `ToolExecutor::execute`, which shells out via [`run_plugin_tool`].

use std::path::Path;

use anyhow::Result;
use serde::Deserialize;
use serde_json::{Value, json};

/// One `plugins[]` entry from ayesha.json (declarative config only).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PluginConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub tools: Vec<PluginToolConfig>,
}

/// One tool definition inside a plugin config entry.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PluginToolConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// JSON-schema object describing the call arguments (`{}` when absent).
    #[serde(default)]
    pub parameters: Value,
    /// Shell command template (see module docs for the execution model).
    #[serde(default)]
    pub command: String,
}

/// A resolved plugin tool ready to be offered to the model and executed.
#[derive(Debug, Clone)]
pub struct PluginTool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub command: String,
    /// Owning plugin id (for diagnostics / plan-mode messaging).
    pub plugin: String,
}

/// Holds every plugin tool flattened from ayesha.json.
#[derive(Debug, Clone, Default)]
pub struct PluginRegistry {
    tools: Vec<PluginTool>,
}

/// The program, static args and whether `{args}` should be piped via stdin.
/// Split out of [`run_plugin_tool`] so the template rules are unit-testable.
pub fn render_command(command: &str, args_json: &str) -> (String, Vec<String>, bool) {
    let mut parts = command.split_whitespace();
    let program = parts.next().unwrap_or("").to_string();
    let mut use_stdin = true;
    let mut static_args: Vec<String> = Vec::new();
    for p in parts {
        if p.contains("{args}") {
            static_args.push(p.replace("{args}", args_json));
            use_stdin = false;
        } else {
            static_args.push(p.to_string());
        }
    }
    (program, static_args, use_stdin)
}

impl PluginRegistry {
    pub fn from_config(configs: &[PluginConfig]) -> Self {
        let mut tools = Vec::new();
        for cfg in configs {
            for t in &cfg.tools {
                if t.name.is_empty() || t.command.is_empty() {
                    continue; // malformed entries are skipped, not fatal
                }
                tools.push(PluginTool {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: if t.parameters.is_null() {
                        json!({ "type": "object", "properties": {} })
                    } else {
                        t.parameters.clone()
                    },
                    command: t.command.clone(),
                    plugin: cfg.name.clone(),
                });
            }
        }
        Self { tools }
    }

    #[allow(dead_code)] // used by tests; natural API for callers
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name)
    }

    pub fn find_tool(&self, name: &str) -> Option<&PluginTool> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Model-facing definitions in the same JSON shape the built-in catalog
    /// uses: `[{ "type": "function", "function": { name, description,
    /// parameters } }]`.
    pub fn tool_definitions(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect()
    }
}

/// Build the system-prompt snippet from the raw plugin configs (the registry
/// only stores tools; prompts are a plugin-level property).
pub fn snippet_from_configs(configs: &[PluginConfig]) -> String {
    let mut out = String::new();
    for cfg in configs {
        let prompt = cfg.prompt.trim();
        if prompt.is_empty() {
            continue;
        }
        if out.is_empty() {
            out.push_str("\n\n### plugins\n");
        }
        out.push_str(prompt);
        out.push('\n');
    }
    out
}

/// Execute a plugin tool: shell out to `command` (see module docs), passing
/// the call arguments as JSON on stdin or via `{args}` substitution, bounded
/// by a 30s timeout.
pub async fn run_plugin_tool(
    tool: &PluginTool,
    args: &Value,
    project_root: &Path,
) -> Result<String> {
    let args_json = serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string());
    let (program, static_args, use_stdin) = render_command(&tool.command, &args_json);
    if program.is_empty() {
        anyhow::bail!("plugin tool '{}' has an empty command", tool.name);
    }

    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&static_args)
        .current_dir(project_root)
        .kill_on_drop(true)
        .stdin(if use_stdin {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let fut = async move {
        let mut child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!(
                "plugin '{}' (from '{}') failed to spawn '{}': {}",
                tool.name,
                tool.plugin,
                program,
                e
            )
        })?;
        if use_stdin {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(args_json.as_bytes()).await?;
            }
            // stdin is dropped here -> pipe closed, child can finish reading
        }
        let output = child.wait_with_output().await?;
        let mut out = String::from_utf8_lossy(&output.stdout).to_string();
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            anyhow::bail!(
                "plugin '{}' exited with {:?}: {}",
                tool.name,
                output.status.code(),
                err.trim()
            );
        }
        if !err.trim().is_empty() {
            out.push_str(&format!("\n[stderr] {}", err.trim()));
        }
        if out.chars().count() > 65536 {
            out = out.chars().take(65536).collect();
        }
        Ok(out)
    };

    tokio::time::timeout(std::time::Duration::from_secs(30), fut)
        .await
        .map_err(|_| anyhow::anyhow!("plugin '{}' timed out after 30s", tool.name))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads_and_merges_plugin_tools() {
        let cfg: PluginConfig = serde_json::from_value(json!({
            "name": "idx",
            "prompt": "you may use the idx plugin for index lookups",
            "tools": [{
                "name": "idx_search",
                "description": "search the index",
                "parameters": { "type": "object", "properties": { "q": { "type": "string" } }, "required": ["q"] },
                "command": "python -m idx search {args}"
            }]
        }))
        .unwrap();
        let registry = PluginRegistry::from_config(&[cfg]);
        assert!(registry.has_tool("idx_search"));
        assert!(!registry.has_tool("nope"));
        assert!(registry.find_tool("idx_search").is_some());
        assert_eq!(registry.find_tool("idx_search").unwrap().plugin, "idx");

        let defs = registry.tool_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0]["type"], "function");
        assert_eq!(defs[0]["function"]["name"], "idx_search");
        assert_eq!(defs[0]["function"]["description"], "search the index");
        assert_eq!(defs[0]["function"]["parameters"]["required"][0], "q");
    }

    #[test]
    fn malformed_plugin_entries_are_skipped() {
        let cfg: PluginConfig = serde_json::from_value(json!({
            "name": "bad",
            "tools": [
                { "name": "", "command": "echo hi" },          // no name
                { "name": "no_cmd", "command": "" },           // no command
                { "name": "ok", "description": "fine", "command": "echo ok" }
            ]
        }))
        .unwrap();
        let registry = PluginRegistry::from_config(&[cfg]);
        assert!(registry.has_tool("ok"));
        assert!(!registry.has_tool("no_cmd"));
        assert_eq!(registry.tool_definitions().len(), 1);
    }

    #[test]
    fn empty_registry_is_harmless() {
        let registry = PluginRegistry::default();
        assert!(registry.is_empty());
        assert!(registry.tool_definitions().is_empty());
        assert!(registry.find_tool("x").is_none());
    }

    #[test]
    fn command_render_substitutes_args_or_uses_stdin() {
        let args = json!({ "q": "hello" });
        let args_json = serde_json::to_string(&args).unwrap();
        // {args} substitution path
        let (prog, argv, stdin) = render_command("python -m idx search {args}", &args_json);
        assert_eq!(prog, "python");
        assert_eq!(argv, vec!["-m".to_string(), "idx".to_string(), "search".to_string(), args_json.clone()]);
        assert!(!stdin);
        // stdin path
        let (prog, argv, stdin) = render_command("python -m idx search", &args_json);
        assert_eq!(prog, "python");
        assert_eq!(argv, vec!["-m".to_string(), "idx".to_string(), "search".to_string()]);
        assert!(stdin);
        // empty command
        let (prog, _, _) = render_command("", &args_json);
        assert!(prog.is_empty());
    }

    #[test]
    fn snippet_collects_plugin_prompts() {
        let cfgs: Vec<PluginConfig> = serde_json::from_value(json!([
            { "name": "a", "prompt": "use plugin a for x" },
            { "name": "b", "prompt": "use plugin b for y" },
            { "name": "c", "prompt": "" }
        ]))
        .unwrap();
        let snippet = snippet_from_configs(&cfgs);
        assert!(snippet.contains("use plugin a for x"));
        assert!(snippet.contains("use plugin b for y"));
        assert!(!snippet.contains("plugin c"));
    }
}
