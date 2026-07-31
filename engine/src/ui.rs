use colored::*;
use std::io::{stdout, Write};

// ── retro cyberpunk color scheme ────────────────────────────
// primary:   bright_green  (matrix green)
// secondary: bright_yellow (amber terminal)
// accent:    bright_cyan
// error:     bright_red
// dim:       bright_black
// thinking:  bright_black (dimmed)

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
        "ayesha-os v4.2.0".bright_cyan());
    println!("  {} {}",
        "  system online".bright_black(),
        "(๑蔷๑)".bright_magenta());
    println!("  {}",
        "──────────────────────────────────────────────".bright_black());
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
        "▶".bright_green().bold(),
        name.bright_yellow(),
        truncated.bright_black());
}

pub fn show_tool_ok(name: &str, msg: &str) {
    let first = msg.lines().next().unwrap_or(msg);
    let truncated = if first.chars().count() > 120 {
        format!("{}...", crate::util::truncate_chars(first, 119))
    } else {
        first.to_string()
    };
    println!("  {} {} {}",
        "✔".bright_green().bold(),
        name.bright_yellow(),
        truncated.bright_black());
    for line in msg.lines().skip(1).take(5) {
        println!("  {} {}",
            "│".bright_black(),
            line.bright_black());
    }
    if msg.lines().count() > 6 {
        println!("  {} {} {}",
            "│".bright_black(),
            "+".bright_black(),
            format!("{} more lines", msg.lines().count() - 6).bright_black());
    }
}

pub fn show_tool_err(name: &str, msg: &str) {
    println!("  {} {} {}",
        "✖".bright_red().bold(),
        name.bright_yellow(),
        msg.bright_red());
}

// ── system messages ───────────────────────────────────────

pub fn show_system(msg: &str) {
    println!("  {} {}",
        "◆".bright_cyan(),
        msg.bright_cyan());
}

pub fn show_error(msg: &str) {
    println!("  {} {}",
        "✖".bright_red(),
        msg.bright_red());
}

#[allow(dead_code)]
pub fn show_processing() {
    print!("  {} {}",
        "◆".bright_cyan(),
        "processing...".bright_black());
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
        "─".bright_black().repeat(3),
        model.bright_black(),
        "─".bright_black().repeat(3));
}

pub fn show_interrupted() {
    println!("  {}",
        "⏹  interrupted".bright_yellow().bold());
}

// ── prompt ─────────────────────────────────────────────────

pub fn prompt_line() {
    print!("  {} ",
        "$".bright_green().bold());
    stdout().flush().ok();
}

pub fn launcher_prompt() {
    print!("  {} {} ",
        "\u{25b6}".bright_yellow().bold(),
        "launch".bright_yellow());
    stdout().flush().ok();
}

pub fn page_switch_prompt() {
    print!("  {} {} ",
        "\u{25b6}".bright_cyan().bold(),
        "switch to".bright_cyan());
    stdout().flush().ok();
}

// ── page switcher overlay (ctrl+p) ─────────────────────────

/// Draw a numbered page list. Each page is (name, desc, running, foreground).
/// Page 1 is always the engine itself.
pub fn draw_page_switcher(pages: &[(String, String, bool, bool)]) -> String {
    let mut out = String::new();
    let inner_w = 52usize;
    let title = "page switcher (ctrl+p)";
    let dashes = inner_w - 1 - title.len();
    out.push_str(&format!("  ┌─{}{}┐\n", title, "─".repeat(dashes)));
    out.push_str(&format!("  │{}│\n", "─".repeat(inner_w)));
    for (i, (name, desc, running, _foreground)) in pages.iter().enumerate() {
        let n = i + 1;
        let status = if *running { "●" } else { "○" };
        let marker = if n == 1 { "◄" } else { " " };
        out.push_str(&format!("  │{:<2} {} {:<14} {:<29} {:<2}│\n", n, status, name, desc, marker));
    }
    out.push_str(&format!("  │{:<52}│\n", ""));
    out.push_str(&format!("  │{:<52}│\n", "● running   ○ stopped   ◄ = current page"));
    out.push_str(&format!("  └{}┘\n", "─".repeat(inner_w)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_switcher_lines_are_aligned() {
        let pages = vec![
            ("engine".to_string(), "terminal persona host".to_string(), true, true),
            ("flora-cli".to_string(), "scottish flora explorer".to_string(), false, true),
            ("desktop-cat".to_string(), "desktop pet cat".to_string(), true, false),
        ];
        let out = draw_page_switcher(&pages);
        let lines: Vec<&str> = out.lines().collect();
        assert!(!lines.is_empty());
        let expected = lines[0].chars().count();
        for line in &lines {
            assert_eq!(line.chars().count(), expected, "misaligned line: {}", line);
        }
        assert!(out.contains("◄"));
        assert!(out.contains("ctrl+p"));
    }
}

// ── command palette overlay ───────────────────────────────

pub fn draw_command_overlay(filter: Option<&str>) {
    let cmds = [
        ("help",    "show this help"),
        ("exit",    "quit ayesha-os"),
        ("clear",   "clear screen"),
        ("models",  "list available models"),
        ("model",   "switch model: /model <name>"),
        ("auto",    "re-enable auto-routing"),
        ("route",   "route a query: /route <query>"),
        ("pull",    "pull model: /pull <name>"),
        ("name",    "set your name: /name <you>"),
        ("run",     "launch applet: /run <name>"),
        ("apps",    "list applets"),
        ("stop",    "stop applet: /stop <name>"),
        ("stats",   "tool usage statistics"),
        ("history", "show conversation history"),
        ("compact", "trim conversation to last 8 messages"),
        ("save",    "save conversation to file"),
        ("load",    "load conversation from file"),
        ("system",  "show current system prompt"),
        ("memory",  "list stored memories"),
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
        "┌──────────────────────────────────────────────────┐".bright_green());
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
        format!("{:<47}│", header.bright_cyan()).bright_green());

    if filtered.is_empty() {
        println!("  │  {}",
            format!("{:<47}│",
                format!("  no match for '/{}'", filter.unwrap_or("")).bright_black().italic()
            ).bright_green());
    } else {
        println!("  │{}",
            format!("{:<48}│",
                "─".repeat(box_w - 4).bright_black()
            ).bright_green());
        let display = if filtered.len() > 10 { &filtered[..10] } else { &filtered };
        for (cmd, desc) in display {
            let line = format!("  │  /{:<10} {:<31}│", cmd, desc);
            println!("{}", line.bright_green());
        }
        println!("  │{}",
            format!("{:<48}│",
                "─".repeat(box_w - 4).bright_black()
            ).bright_green());
    }
    println!("  {}",
        "└──────────────────────────────────────────────────┘".bright_green());
    println!();
}

// ── help ───────────────────────────────────────────────────

pub fn print_help() {
    println!();
    println!("  {}",
        "┌─ commands ──────────────────────────────────┐".bright_green());
    println!("  {}",
        "│                                            │".bright_green());
    let help_cmds = [
        ("help",      "show this message"),
        ("exit",      "quit ayesha-os"),
        ("clear",     "clear screen"),
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
        ("system",    "show current system prompt"),
        ("memory",    "list stored memories"),
        ("analyze",   "analyze own source code"),
        ("evolve",    "suggest new tools"),
        ("refine",    "analyze prompt history"),
        ("ctrl+p",    "page switcher (in-window)"),
    ];
    for (cmd, desc) in &help_cmds {
        println!("  {} {:<14} {}",
            "│".bright_green(),
            cmd.bright_cyan(),
            format!("{:<28}{}", desc.bright_black(), "│").bright_black());
    }
    println!("  {}",
        "│                                            │".bright_green());
    println!("  {}",
        "├─ tools (auto-called by model) ─────────────┤".bright_green());
    let tool_cmds = [
        ("read_file",    "read any file on disk"),
        ("write_file",   "create or overwrite files"),
        ("list_dir",     "browse directories"),
        ("manage_applet","list/launch/stop applets"),
    ];
    for (cmd, desc) in &tool_cmds {
        println!("  {} {:<14} {}",
            "│".bright_green(),
            cmd.bright_cyan(),
            format!("{:<28}{}", desc.bright_black(), "│").bright_black());
    }
    println!("  {}",
        "│                                            │".bright_green());
    println!("  {}",
        "├─ auto-memory (markers in responses) ───────┤".bright_green());
    let mem_cmds = [
        ("[REMEMBER: x]",      "store a fact or preference"),
        ("[PREFERENCE: k = v]", "store a key-value preference"),
        ("[FACT: x]",           "store a learned fact"),
    ];
    for (cmd, desc) in &mem_cmds {
        println!("  {} {:<22} {}",
            "│".bright_green(),
            cmd.bright_magenta(),
            format!("{:<20}{}", desc.bright_black(), "│").bright_black());
    }
    println!("  {}",
        "└────────────────────────────────────────────┘".bright_green());
    println!();
    std::io::stdout().flush().ok();
}

// ── response formatting ───────────────────────────────────

#[allow(dead_code)]
fn color_kaomojis(text: &str) -> String {
    let mut result = text.to_string();
    for k in KAOMOJIS {
        let colored = format!("{}", k.bright_magenta());
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
            "▐".bright_black(),
            line.on_bright_black()
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
                out.push_str(&format!("{}\n", trimmed.bright_cyan().bold()));
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
