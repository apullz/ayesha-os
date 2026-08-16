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
    "generate_test",
    "render_sprite",
    // process / applet control
    "manage_applet",
    // OS-level effects
    "send_hotkey",
    // network
    "fetch_url",
    "download_image",
    // persistent state (memories / preferences)
    "remember",
    "set_preference",
    // meta-tools that modify the agent itself
    "skill_tool",
    "coding_agent",
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

/// Decide what to do with a tool call, consulting config overrides.
pub fn decide(tool: &str, config: &Value) -> Verdict {
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

    if SENSITIVE_TOOLS.contains(&tool) {
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
            "read_memories", "search_memories", "analyze_self", "get_tool_stats",
        ] {
            assert_eq!(decide(tool, &config), Verdict::Allow, "{} should be auto-allowed", tool);
        }
    }

    #[test]
    fn sensitive_tools_prompt_by_default() {
        let config = serde_json::json!({});
        for tool in [
            "write_file", "send_hotkey", "manage_applet", "fetch_url",
            "download_image", "evolve_tools", "refine_prompt", "remember",
            "generate_html",
        ] {
            assert_eq!(decide(tool, &config), Verdict::Prompt, "{} should prompt", tool);
        }
    }

    #[test]
    fn always_override_skips_prompt() {
        let mut config = serde_json::json!({});
        set_override(&mut config, "write_file", "always");
        assert_eq!(decide("write_file", &config), Verdict::Allow);
    }

    #[test]
    fn never_override_blocks() {
        let mut config = serde_json::json!({});
        set_override(&mut config, "fetch_url", "never");
        match decide("fetch_url", &config) {
            Verdict::Denied(msg) => assert!(msg.starts_with("error:")),
            other => panic!("expected Denied, got {:?}", other),
        }
    }

    #[test]
    fn permission_mode_off_auto_allows_everything() {
        let config = serde_json::json!({ "permission_mode": "off" });
        assert_eq!(decide("write_file", &config), Verdict::Allow);
        assert_eq!(decide("send_hotkey", &config), Verdict::Allow);
    }
}
