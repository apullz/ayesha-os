/// Events the agent loop sends to the UI layer — decouples agent logic from terminal output.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum UiEvent {
    ToolCall { name: String, args: String },
    ToolOk { name: String, result: String },
    ToolErr { name: String, error: String },
    Error(String),
    Interrupted,
}

/// Truncate tool results to prevent context overflow.
pub fn truncate_tool_result(result: &str, max_chars: usize) -> String {
    let char_count = result.chars().count();
    if char_count > max_chars {
        format!("{}...\n(truncated: showing {} of {} chars)",
            crate::util::truncate_chars(result, max_chars), max_chars, char_count)
    } else {
        result.to_string()
    }
}

/// Check if the qwen tool model should be called for this user message.
/// Returns false for pure chit-chat to save ~2s latency per message.
pub fn needs_tools(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    let hints = [
        "read", "write", "edit", "create", "open", "show", "list", "delete",
        "move", "copy", "search", "find", "grep", "look", "run", "execute",
        "build", "compile", "test", "deploy", "launch", "stop", "start",
        "check", "install", "update", "sync", "push", "pull", "commit",
        "git", "cargo", "npm", "pip", "python", "rust", "code", "file",
        "directory", "folder", "path", "src", "engine", "core", "applet",
        "how many", "show me", "tell me about", "what's in", "what's the",
        "analyze", "refactor", "fix", "bug", "error", "issue", "problem",
        "http", "https", ".com", ".org", ".rs", ".py", ".ts", ".js",
        "page", "switch", "applet",
    ];
    hints.iter().any(|h| lower.contains(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short() {
        assert_eq!(truncate_tool_result("hello", 100), "hello");
    }

    #[test]
    fn truncate_long() {
        let long = "a".repeat(200);
        let truncated = truncate_tool_result(&long, 100);
        assert!(truncated.contains("truncated"));
        assert!(truncated.contains("100 of 200"));
    }

    #[test]
    fn truncate_exact_boundary() {
        let exact = "x".repeat(100);
        assert_eq!(truncate_tool_result(&exact, 100), exact);
    }

    #[test]
    fn truncate_multibyte_safe() {
        let kaomoji = "^_^ desu~".repeat(100);
        let truncated = truncate_tool_result(&kaomoji, 50);
        assert!(truncated.contains("truncated"));
    }

    #[test]
    fn needs_tools_detects_file_ops() {
        assert!(needs_tools("read main.rs"));
        assert!(needs_tools("write to the file"));
        assert!(needs_tools("show me the directory"));
        assert!(needs_tools("search for error"));
        assert!(needs_tools("git commit"));
        assert!(needs_tools("run cargo test"));
        assert!(needs_tools("check http://example.com"));
        assert!(needs_tools("what's in src/"));
    }

    #[test]
    fn needs_tools_skips_chitchat() {
        assert!(!needs_tools("hi"));
        assert!(!needs_tools("hello"));
        assert!(!needs_tools("what's your name"));
        assert!(!needs_tools("tell me a joke"));
        assert!(!needs_tools("how are you"));
        assert!(!needs_tools("who are you"));
        assert!(!needs_tools("good morning"));
        assert!(!needs_tools("thanks"));
    }

    #[test]
    fn ui_event_clone() {
        let e = UiEvent::ToolCall { name: "test".into(), args: "{}".into() };
        let e2 = e.clone();
        assert!(matches!(e2, UiEvent::ToolCall { .. }));
    }
}
