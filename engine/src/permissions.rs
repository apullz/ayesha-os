//! Tool-call permission gating.
//!
//! Before a *sensitive* tool runs, the agent loop asks the user on the steer
//! channel (allow once / deny once / always allow / deny forever). Read-only
//! introspection tools never prompt. Persisted overrides live in config.json
//! under `permissions.<tool>` (`"always"` | `"never"`); setting
//! `"permission_mode": "off"` disables prompting entirely.
//!
//! The interactive prompt itself lives in main.rs (`ask_permission`) because it
//! needs the steer channel, the docked UI and config.json persistence; this
//! module stays pure so the decision logic is unit-testable.

use serde_json::Value;

/// Tools that can change state on disk, launch processes, hit the network or
/// modify the agent itself. These prompt unless the user picked a per-tool
/// override. Everything NOT in this list is treated as read-only
/// introspection and never prompts.
const SENSITIVE_TOOLS: &[&str] = &[
    // filesystem writers
    "write_file",
    "generate_html",
    "generate_object",
    "generate_sprite",
    "generate_tileset",
    "render_sprite",
    // process / applet control
    "manage_applet",
    // network
    "fetch_url",
    "download_image",
    // persistent state (memories / preferences)
    "remember",
    "set_preference",
    // meta-tools that modify the agent itself
    "coding_agent",
    // "delegate" prompts (it spawns a sub-agent) but is INTENTIONALLY absent
    // from tools::MUTATING_TOOLS, so plan mode allows it: the sub-agent is
    // hard-restricted to read-only tools and cannot delegate further, so
    // delegating research is itself read-only.
    "delegate",
    "evolve_tools",
    "refine_prompt",
];

/// Outcome of a permission decision.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Run the tool now.
    Allow,
    /// Do not run — the string is the tool-result message the model sees.
    Denied(String),
    /// Ask the user on the steer channel.
    Prompt,
}

/// Decide what to do with a tool call, consulting config overrides. The
/// `is_sensitive` predicate covers tools whose names can't live in the static
/// list — plugin tools execute arbitrary shell commands, so they must prompt
/// exactly like the built-in sensitive set (unless the user picked a per-tool
/// override or disabled permission mode). Pass `|_| false` when there are no
/// dynamic tools.
pub fn decide_with(tool: &str, config: &Value, is_sensitive: impl Fn(&str) -> bool) -> Verdict {
    // Global kill-switch: `"permission_mode": "off"` auto-allows everything.
    if config.get("permission_mode").and_then(|v| v.as_str()) == Some("off") {
        return Verdict::Allow;
    }

    // Per-tool persisted override wins over the static list.
    if let Some(over) = config
        .get("permissions")
        .and_then(|p| p.get(tool))
        .and_then(|v| v.as_str())
    {
        return match over {
            "always" => Verdict::Allow,
            "never" => Verdict::Denied(format!(
                "error: tool call to `{}` blocked by user (deny-forever in config.json)",
                tool
            )),
            _ => Verdict::Prompt, // unknown override string -> fall through to prompt
        };
    }

    if SENSITIVE_TOOLS.contains(&tool) || is_sensitive(tool) {
        Verdict::Prompt
    } else {
        Verdict::Allow
    }
}

/// Persist a per-tool override into `config["permissions"]`.
pub fn set_override(config: &mut Value, tool: &str, verdict: &str) {
    if !config.is_object() {
        *config = serde_json::json!({});
    }
    let perms = config.get_mut("permissions").and_then(|p| p.as_object_mut());
    match perms {
        Some(map) => {
            map.insert(tool.to_string(), serde_json::json!(verdict));
        }
        None => {
            let mut map = serde_json::Map::new();
            map.insert(tool.to_string(), serde_json::json!(verdict));
            config["permissions"] = serde_json::Value::Object(map);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_tools_never_prompt() {
        let config = serde_json::json!({});
        for tool in [
            "read_file", "grep", "glob", "list_dir", "list_memories",
            "search_memories", "analyze_self", "get_tool_stats",
        ] {
            assert_eq!(decide_with(tool, &config, |_| false), Verdict::Allow, "{} should be auto-allowed", tool);
        }
    }

    #[test]
    fn sensitive_tools_prompt_by_default() {
        let config = serde_json::json!({});
        for tool in [
            "write_file", "manage_applet", "fetch_url",
            "download_image", "evolve_tools", "refine_prompt", "remember",
            "generate_html",
        ] {
            assert_eq!(decide_with(tool, &config, |_| false), Verdict::Prompt, "{} should prompt", tool);
        }
    }

    #[test]
    fn always_override_skips_prompt() {
        let mut config = serde_json::json!({});
        set_override(&mut config, "write_file", "always");
        assert_eq!(decide_with("write_file", &config, |_| false), Verdict::Allow);
    }

    #[test]
    fn never_override_blocks() {
        let mut config = serde_json::json!({});
        set_override(&mut config, "fetch_url", "never");
        match decide_with("fetch_url", &config, |_| false) {
            Verdict::Denied(msg) => assert!(msg.starts_with("error:")),
            other => panic!("expected Denied, got {:?}", other),
        }
    }

    #[test]
    fn permission_mode_off_auto_allows_everything() {
        let config = serde_json::json!({ "permission_mode": "off" });
        assert_eq!(decide_with("write_file", &config, |_| false), Verdict::Allow);
        assert_eq!(decide_with("manage_applet", &config, |_| false), Verdict::Allow);
    }

    #[test]
    fn plugin_tools_prompt_like_sensitive_tools() {
        // Plugin tool names are dynamic, so the static SENSITIVE_TOOLS list
        // can't know them — decide_with must prompt for them (Gate C HIGH:
        // plugin tools run arbitrary shell commands and previously bypassed
        // the permission prompt in build/auto mode).
        let config = serde_json::json!({});
        let is_plugin = |n: &str| n == "my_plugin_tool";
        assert_eq!(
            decide_with("my_plugin_tool", &config, is_plugin),
            Verdict::Prompt,
            "plugin tool names must prompt unless the user opted in"
        );
        // non-plugin tools are unaffected by the extra predicate
        assert_eq!(decide_with("read_file", &config, is_plugin), Verdict::Allow);
        // built-in sensitive tools still prompt even without the predicate
        assert_eq!(decide_with("write_file", &config, |_| false), Verdict::Prompt);
        // per-tool overrides still win for plugin tools
        let mut config = serde_json::json!({});
        set_override(&mut config, "my_plugin_tool", "always");
        assert_eq!(decide_with("my_plugin_tool", &config, is_plugin), Verdict::Allow);
        let mut config = serde_json::json!({});
        set_override(&mut config, "my_plugin_tool", "never");
        match decide_with("my_plugin_tool", &config, is_plugin) {
            Verdict::Denied(msg) => assert!(msg.starts_with("error:")),
            other => panic!("expected Denied for never-override, got {:?}", other),
        }
        // permission_mode off still auto-allows plugin tools
        let config = serde_json::json!({ "permission_mode": "off" });
        assert_eq!(decide_with("my_plugin_tool", &config, is_plugin), Verdict::Allow);
    }
}
