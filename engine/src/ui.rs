use colored::*;
use std::io::{stdout, Write};

use crate::theme::{self, Role};

// ── colors come from the active theme (crate::theme) ────────────
// role map: primary = brand/boxes/prompt, accent = system/highlight,
// secondary = names/warnings, error = failures, dim = muted, etc.

#[allow(dead_code)]
const KAOMOJIS: &[&str] = &[
    "(╯°□°)╯︵ ┻━┻", "(◕ᴗ◕✿)", "(๑•蔷•๑)", "(╥﹏╥)",
    "^_^", ">w<", ":3", "(ᵔᴥᵔ)", "(◕‿◕)",
    "(ﾉ◕ヮ◕)ﾉ", "¯\\_(ツ)_/¯", "(づ｡◕‿‿◕｡)づ",
    "(•ω•)", "(｡•̀ᴗ-)✧", "♪～(´ε｀ )",
    "(ノಠ益ಠ)ノ", "┻━┻", "┬─┬", "◥▅◤",
    "kapoo!", "desu-ne", "desu--",
];

// ── banner ─────────────────────────────────────────────────

const BANNER_LINES: &[&str] = &[
    r#"                       _"#,
    r#"                      | |"#,
    r#"  __ _ _   _  ___  ___| |__   __ _ ______ ___  ___"#,
    r#" / _` | | | |/ _ \/ __| '_ \ / _` |______/ _ \/ __|"#,
    r#"| (_| | |_| |  __/\__ \ | | | (_| |     | (_) \__ \\"#,
    r#" \__,_|\__, |\___||___/_| |_|\__,_|      \___/|___/"#,
    r#"        __/ |"#,
    r#"       |___/"#,
];

pub fn print_banner() {
    // keep the classic rainbow logo, then theme the info lines below it
    let colors: &[Color] = &[
        Color::BrightRed,
        Color::BrightYellow,
        Color::BrightGreen,
        Color::BrightCyan,
        Color::BrightBlue,
        Color::BrightMagenta,
        Color::BrightRed,
        Color::BrightYellow,
    ];
    for (line, color) in BANNER_LINES.iter().zip(colors.iter()) {
        println!("  {}", line.color(*color));
    }
    println!();
    println!("  {} {}",
        "◆".bright_green(),
        "ayesha-os v4.5.0".bright_cyan());
    println!("  {} {}",
        theme::paint(Role::Dim, "  system online"),
        theme::paint(Role::Accent, "(๑蔷๑)"));
    println!("  {}",
        theme::paint(Role::Dim, "──────────────────────────────────────────────"));
    println!();
}

// ── tool call / result ────────────────────────────────────

pub fn show_tool_call(name: &str, args: &str) {
    let truncated = if args.chars().count() > 80 {
        format!("{}...", crate::util::truncate_chars(args, 79))
    } else {
        args.to_string()
    };
    println!("  {} {} {}",
        theme::bold(Role::Primary, "▶"),
        theme::paint(Role::Secondary, name),
        theme::paint(Role::Dim, truncated));
}

pub fn show_tool_ok(name: &str, msg: &str) {
    let first = msg.lines().next().unwrap_or(msg);
    let truncated = if first.chars().count() > 120 {
        format!("{}...", crate::util::truncate_chars(first, 119))
    } else {
        first.to_string()
    };
    println!("  {} {} {}",
        theme::bold(Role::Success, "✔"),
        theme::paint(Role::Secondary, name),
        theme::paint(Role::Dim, truncated));
    for line in msg.lines().skip(1).take(5) {
        println!("  {} {}",
            theme::paint(Role::Dim, "│"),
            theme::paint(Role::Dim, line));
    }
    if msg.lines().count() > 6 {
        println!("  {} {} {}",
            theme::paint(Role::Dim, "│"),
            theme::paint(Role::Dim, "+"),
            theme::paint(Role::Dim, format!("{} more lines", msg.lines().count() - 6)));
    }
}

pub fn show_tool_err(name: &str, msg: &str) {
    println!("  {} {} {}",
        theme::bold(Role::Error, "✖"),
        theme::paint(Role::Secondary, name),
        theme::paint(Role::Error, msg));
}

// ── system messages ───────────────────────────────────────

pub fn show_system(msg: &str) {
    println!("  {} {}",
        theme::paint(Role::Accent, "◆"),
        theme::paint(Role::Accent, msg));
}

pub fn show_error(msg: &str) {
    println!("  {} {}",
        theme::paint(Role::Error, "✖"),
        theme::paint(Role::Error, msg));
}

/// Echo the user's message opencode-style: a bold `you` marker followed by the
/// message text, wrapped at ~100 chars so long inputs stay readable.
pub fn show_user_msg(msg: &str) {
    print!("{}", format_user_msg(msg));
    stdout().flush().ok();
}

/// Pure formatter for the user message echo (testable, no stdout).
fn format_user_msg(msg: &str) -> String {
    let marker = "you";
    let wrap = 96usize;
    let chars: Vec<char> = msg.chars().collect();
    if chars.len() <= wrap {
        return format!("  {} ▸ {}\n", theme::bold(Role::Primary, marker), msg);
    }
    let mut out = String::new();
    let mut line = String::new();
    let mut count = 0usize;
    for c in chars {
        line.push(c);
        count += 1;
        if count >= wrap && c.is_whitespace() {
            out.push_str(&format!("  {} ▸ {}\n", theme::bold(Role::Primary, marker), line.trim_end()));
            line.clear();
            count = 0;
        }
    }
    if !line.trim().is_empty() {
        out.push_str(&format!("  {} ▸ {}\n", theme::bold(Role::Primary, marker), line.trim_end()));
    }
    out
}

/// Thin dim separator between turns, opencode-style.
pub fn show_turn_separator() {
    println!("  {}",
        theme::paint(Role::Dim, "─".repeat(38)));
}

#[allow(dead_code)]
pub fn show_processing() {
    print!("  {} {}",
        theme::paint(Role::Accent, "◆"),
        theme::paint(Role::Dim, "processing..."));
    stdout().flush().ok();
}

#[allow(dead_code)]
pub fn hide_processing() {
    print!("\r");
    for _ in 0..40 { print!(" "); }
    print!("\r");
    stdout().flush().ok();
}

pub fn show_routing(model: &str) {
    println!("  {} {} {}",
        theme::paint(Role::Dim, "─").repeat(3),
        theme::paint(Role::Dim, model),
        theme::paint(Role::Dim, "─").repeat(3));
}

pub fn show_interrupted() {
    println!("  {}",
        theme::bold(Role::Warning, "⏹  interrupted"));
}

// ── prompt ─────────────────────────────────────────────────

pub fn prompt_line() {
    print!("  {} ",
        theme::bold(Role::Primary, "$"));
    stdout().flush().ok();
}

pub fn menu_prompt() {
    print!("  {} {} ",
        theme::bold(Role::Accent, "\u{25b6}"),
        theme::paint(Role::Accent, "launch"));
    stdout().flush().ok();
}

// ── interactive applet menu (ctrl+p / ctrl+m) ────────────────

/// Draw an interactive applet menu box. Pages: (name, desc, running, foreground).
/// Returns (rendered_string, line_count).
pub fn draw_applet_menu(pages: &[(String, String, bool, bool)], idx: usize, filter: &str) -> (String, usize) {
    let mut out = String::new();
    let inner_w = 64usize;
    let title = "applet launcher (ctrl+p)";
    let dashes = inner_w - 1 - title.len();
    out.push_str(&format!("  ┌─{}{}┐\n", title, "─".repeat(dashes)));
    out.push_str(&format!("  │{}│\n", "─".repeat(inner_w)));

    let filtered: Vec<(usize, &(String, String, bool, bool))> = pages
        .iter()
        .enumerate()
        .filter(|(_, (name, desc, _, _))| {
            if filter.is_empty() {
                true
            } else {
                name.to_lowercase().contains(&filter.to_lowercase())
                    || desc.to_lowercase().contains(&filter.to_lowercase())
            }
        })
        .collect();

    let display_idx = idx.min(filtered.len().saturating_sub(1));

    if filtered.is_empty() {
        out.push_str(&format!("  │  {:<62}│\n", format!("no matches for '{}'", filter)));
    } else {
        for (i, (_orig_i, (name, desc, running, foreground))) in filtered.iter().enumerate() {
            let selected = i == display_idx;
            let cursor = if selected { "►" } else { " " };
            let status = if *running { "●" } else { "○" };
            let mode_tag = if *foreground { "[window]" } else { "[bg]" };
            let label = format!("{} {} {:<11} {:<24} {:<8}", cursor, status, name, desc, mode_tag);
            // pad/truncate label to fit inner content width (62 chars)
            let truncated = if label.chars().count() > inner_w - 2 {
                let mut s: String = label.chars().take(inner_w - 5).collect();
                s.push_str("...");
                s
            } else {
                label
            };
            out.push_str(&format!("  │  {:<62}│\n", truncated));
        }
    }

    out.push_str(&format!("  │{}│\n", "─".repeat(inner_w)));
    let filter_display = if filter.is_empty() { "type to filter...".to_string() } else { format!("filter: {}", filter) };
    out.push_str(&format!("  │  {:<62}│\n", filter_display));
    out.push_str(&format!("  │  {:<62}│\n", "● running  ○ stopped  ↑↓ nav  enter launch  x stop  esc back"));
    out.push_str(&format!("  └{}┘\n", "─".repeat(inner_w)));

    let line_count = out.lines().count();
    (out, line_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applet_menu_lines_are_aligned() {
        let pages = vec![
            ("engine".to_string(), "terminal persona host".to_string(), true, true),
            ("flora-cli".to_string(), "scottish flora explorer".to_string(), false, true),
            ("desktop-cat".to_string(), "desktop pet cat".to_string(), true, false),
        ];
        let (out, line_count) = draw_applet_menu(&pages, 0, "");
        let lines: Vec<&str> = out.lines().collect();
        assert!(!lines.is_empty());
        assert_eq!(lines.len(), line_count);
        let expected = lines[0].chars().count();
        for line in &lines {
            assert_eq!(line.chars().count(), expected, "misaligned line: {}", line);
        }
        assert!(out.contains("►"));
        assert!(out.contains("applet launcher"));
    }

    #[test]
    fn user_msg_single_line_for_short_input() {
        let out = format_user_msg("hello");
        assert_eq!(out.lines().count(), 1);
        assert!(out.contains("hello"));
        assert!(out.contains("you"));
    }

    #[test]
    fn user_msg_wraps_long_input() {
        let long = "word ".repeat(30); // 150 chars, breaks at whitespace
        let out = format_user_msg(&long);
        assert!(out.lines().count() > 1, "long message was not wrapped");
        // every line starts with the marker
        for line in out.lines() {
            assert!(line.trim_start().starts_with("you ▸") || line.contains("you ▸"), "bad line: {}", line);
        }
    }

    #[test]
    fn user_msg_handles_multibyte() {
        let out = format_user_msg("hmm desu-ne (๑•蔷•๑)");
        assert!(out.contains("(๑•蔷•๑)"));
    }
}

// ── command palette overlay ───────────────────────────────

pub fn draw_command_overlay(filter: Option<&str>) {
    let cmds = [
        ("help",    "show this help"),
        ("exit",    "quit ayesha-os"),
        ("clear",   "clear screen"),
        ("reset",   "clear conversation history"),
        ("models",  "list available models"),
        ("model",   "switch model: /model <name>"),
        ("auto",    "re-enable auto-routing"),
        ("route",   "route a query: /route <query>"),
        ("pull",    "pull model: /pull <name>"),
        ("name",    "set your name: /name <you>"),
        ("run",     "launch applet: /run <name>"),
        ("apps",    "list applets"),
        ("stop",    "stop applet: /stop <name>"),
        ("sync",    "sync to github & hf"),
        ("toolmodel","switch tool model: /toolmodel <name>"),
        ("stats",   "tool usage statistics"),
        ("history", "show conversation history"),
        ("compact", "trim conversation to last 8 messages"),
        ("save",    "save conversation to file"),
        ("load",    "load conversation from file"),
        ("sessions","list auto-saved sessions"),
        ("resume",  "resume a session: /resume [name]"),
        ("newsession","start a fresh conversation"),
        ("system",  "show current system prompt"),
        ("export",  "export conversation as markdown"),
        ("ping",    "measure response latency"),
        ("joke",    "tell a random joke"),
        ("time",    "show current UTC time"),
        ("uptime",  "show session uptime"),
        ("theme",   "list/switch theme: theme [name]"),
        ("config",  "view/edit config: config [key=value]"),
        ("memory",  "list stored memories"),
        ("skills",  "list available skills"),
        ("analyze", "analyze own source code"),
        ("evolve",  "suggest new tools"),
        ("refine",  "analyze prompt history"),
    ];

    let filtered: Vec<&(&str, &str)> = match filter {
        Some(f) if !f.is_empty() => {
            let lower = f.to_lowercase();
            cmds.iter().filter(|(name, _)| {
                name.starts_with(&lower) || name.contains(&lower)
            }).collect()
        }
        _ => cmds.iter().collect(),
    };

    let box_w = 54;

    println!();
    println!("  {}",
        theme::paint(Role::Primary, "┌──────────────────────────────────────────────────┐"));
    let header = if let Some(f) = filter {
        if f.is_empty() {
            "  ◆  command palette".to_string()
        } else {
            format!("  ◆  command palette  /{}", f)
        }
    } else {
        "  ◆  command palette".to_string()
    };
    println!("  │  {}",
        theme::paint(Role::Primary, format!("{:<47}│", theme::paint(Role::Accent, header))));

    if filtered.is_empty() {
        println!("  │  {}",
            theme::paint(Role::Primary,
                format!("{:<47}│",
                    theme::paint(Role::Dim, format!("  no match for '/{}'", filter.unwrap_or(""))).italic()
                )));
    } else {
        println!("  │{}",
            theme::paint(Role::Primary,
                format!("{:<48}│",
                    theme::paint(Role::Dim, "─".repeat(box_w - 4))
                )));
        let display = if filtered.len() > 10 { &filtered[..10] } else { &filtered };
        for (cmd, desc) in display {
            let line = format!("  │  /{:<10} {:<31}│", cmd, desc);
            println!("{}", theme::paint(Role::Primary, line));
        }
        println!("  │{}",
            theme::paint(Role::Primary,
                format!("{:<48}│",
                    theme::paint(Role::Dim, "─".repeat(box_w - 4))
                )));
    }
    println!("  {}",
        theme::paint(Role::Primary, "└──────────────────────────────────────────────────┘"));
    println!();
}

// ── help ───────────────────────────────────────────────────

pub fn print_help() {
    println!();
    println!("  {}",
        theme::paint(Role::Primary, "┌─ commands ──────────────────────────────────┐"));
    println!("  {}",
        theme::paint(Role::Primary, "│                                            │"));
    let help_cmds = [
        ("help",      "show this message"),
        ("exit",      "quit ayesha-os"),
        ("clear",     "clear screen"),
        ("reset",     "clear conversation history"),
        ("models",    "list available models"),
        ("model",     "switch chat model: model <name>"),
        ("toolmodel", "switch tool model: toolmodel <name>"),
        ("auto",      "re-enable auto-routing"),
        ("pull",      "pull model: pull <name>"),
        ("sync",      "sync to github & hf"),
        ("apps",      "list applets"),
        ("run",       "launch applet: run <name>"),
        ("stop",      "stop applet: stop <name>"),
        ("stats",     "tool usage statistics"),
        ("history",   "show conversation history"),
        ("compact",   "trim conversation to last 8 messages"),
        ("save",      "save conversation to file"),
        ("load",      "load conversation from file"),
        ("sessions",  "list auto-saved sessions"),
        ("resume",    "resume a session: resume [name]"),
        ("newsession","start a fresh conversation"),
        ("system",    "show current system prompt"),
        ("export",    "export conversation as markdown"),
        ("ping",      "measure response latency"),
        ("joke",      "tell a random joke"),
        ("time",      "show current UTC time"),
        ("uptime",    "show session uptime"),
        ("theme",     "list/switch theme: theme [name]"),
        ("config",    "view/edit config: config [key=value]"),
        ("memory",    "list stored memories"),
        ("skills",    "list available skills"),
        ("analyze",   "analyze own source code"),
        ("evolve",    "suggest new tools"),
        ("refine",    "analyze prompt history"),
        ("ctrl+p",    "page switcher (in-window)"),
    ];
    for (cmd, desc) in &help_cmds {
        println!("  {} {:<14} {}",
            theme::paint(Role::Primary, "│"),
            theme::paint(Role::Accent, cmd),
            theme::paint(Role::Dim, format!("{:<28}{}", desc, "│")));
    }
    println!("  {}",
        theme::paint(Role::Primary, "│                                            │"));
    println!("  {}",
        theme::paint(Role::Primary, "├─ tools (auto-called by model) ─────────────┤"));
    let tool_cmds = [
        ("read_file",    "read any file on disk"),
        ("write_file",   "create or overwrite files"),
        ("list_dir",     "browse directories"),
        ("grep",         "search files for text"),
        ("glob",         "find files by pattern"),
        ("list_skills",  "list available skills"),
        ("read_skill",   "load a skill's instructions"),
        ("manage_applet","list/launch/stop applets"),
    ];
    for (cmd, desc) in &tool_cmds {
        println!("  {} {:<14} {}",
            theme::paint(Role::Primary, "│"),
            theme::paint(Role::Accent, cmd),
            theme::paint(Role::Dim, format!("{:<28}{}", desc, "│")));
    }
    println!("  {}",
        theme::paint(Role::Primary, "│                                            │"));
    println!("  {}",
        theme::paint(Role::Primary, "├─ auto-memory (markers in responses) ───────┤"));
    let mem_cmds = [
        ("[REMEMBER: x]",      "store a fact or preference"),
        ("[PREFERENCE: k = v]", "store a key-value preference"),
        ("[FACT: x]",           "store a learned fact"),
    ];
    for (cmd, desc) in &mem_cmds {
        println!("  {} {:<22} {}",
            theme::paint(Role::Primary, "│"),
            theme::paint(Role::Accent, cmd),
            theme::paint(Role::Dim, format!("{:<20}{}", desc, "│")));
    }
    println!("  {}",
        theme::paint(Role::Primary, "└────────────────────────────────────────────┘"));
    println!();
    std::io::stdout().flush().ok();
}

// ── response formatting ───────────────────────────────────

#[allow(dead_code)]
fn color_kaomojis(text: &str) -> String {
    let mut result = text.to_string();
    for k in KAOMOJIS {
        let colored = format!("{}", theme::paint(Role::Accent, k));
        result = result.replace(k, &colored);
    }
    result
}

#[allow(dead_code)]
fn format_code_block(code: &str) -> String {
    let mut out = String::new();
    for line in code.lines() {
        out.push_str(&format!(
            "  {} {}\n",
            theme::paint(Role::Dim, "▐"),
            theme::code_line(line)
        ));
    }
    out
}

#[allow(dead_code)]
pub fn format_response(text: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let mut code_buf = String::new();

    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            if in_code {
                out.push_str(&format_code_block(&code_buf));
                code_buf.clear();
            }
            in_code = !in_code;
            continue;
        }

        if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
        } else {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                out.push('\n');
            } else if trimmed.starts_with("#") {
                out.push_str(&format!("{}\n", theme::bold(Role::Accent, trimmed)));
            } else {
                out.push_str(&format!("{}\n", line));
            }
        }
    }

    if in_code && !code_buf.is_empty() {
        out.push_str(&format_code_block(&code_buf));
    }

    color_kaomojis(&out)
}
