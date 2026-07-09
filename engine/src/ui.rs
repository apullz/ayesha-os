use colored::*;
use std::io::{stdout, Write};

// ── retro cyberpunk color scheme ────────────────────────────
// primary:   bright_green  (matrix green)
// secondary: bright_yellow (amber terminal)
// accent:    bright_cyan
// error:     bright_red
// dim:       bright_black
// thinking:  bright_black (dimmed)

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
    let truncated = if args.len() > 80 {
        format!("{}...", &args[..79])
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
    let truncated = if first.len() > 120 {
        format!("{}...", &first[..119])
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

pub fn show_processing() {
    print!("  {} {}",
        "◆".bright_cyan(),
        "processing...".bright_black());
    stdout().flush().ok();
}

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
        ("stats",   "tool usage statistics"),
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
        "┌─ commands ─────────────────────────────────┐".bright_green());
    println!("  {}",
        "│                                           │".bright_green());
    let help_cmds = [
        ("help",    "show this message"),
        ("exit",    "quit ayesha-os"),
        ("clear",   "clear screen"),
        ("models",  "list available models"),
        ("model",   "switch model: model <name>"),
        ("auto",    "re-enable auto-routing"),
        ("pull",    "pull model: pull <name>"),
    ];
    for (cmd, desc) in &help_cmds {
        println!("  {} {:<14} {}",
            "│".bright_green(),
            cmd.bright_cyan(),
            format!("{:<27}{}", desc.bright_black(), "│").bright_black());
    }
    println!("  {}",
        "│                                           │".bright_green());
    println!("  {}",
        "├─ self-improvement ────────────────────────┤".bright_green());
    let si_cmds = [
        ("stats",   "tool usage statistics"),
        ("memory",  "list stored memories"),
        ("analyze", "analyze own source code"),
        ("evolve",  "suggest new tools"),
        ("refine",  "analyze prompt history"),
    ];
    for (cmd, desc) in &si_cmds {
        println!("  {} {:<14} {}",
            "│".bright_green(),
            cmd.bright_cyan(),
            format!("{:<27}{}", desc.bright_black(), "│").bright_black());
    }
    println!("  {}",
        "└───────────────────────────────────────────┘".bright_green());
    println!();
    println!("  {}",
        "┌─ tools (auto-called by model) ────────────┐".bright_green());
    println!("  {}",
        "│                                           │".bright_green());
    let tool_cmds = [
        ("read_file",       "read any file on disk"),
        ("write_file",      "create or overwrite files"),
        ("list_dir",        "browse directories"),
        ("generate_html",   "render html to file"),
        ("generate_sprite", "create pixel art characters"),
        ("remember",        "store a memory"),
        ("search_memories", "search stored memories"),
        ("analyze_self",    "ai code review"),
        ("evolve_tools",    "suggest new tools"),
    ];
    for (cmd, desc) in &tool_cmds {
        println!("  {} {:<18} {}",
            "│".bright_green(),
            cmd.bright_cyan(),
            format!("{:<23}{}", desc.bright_black(), "│").bright_black());
    }
    println!("  {}",
        "└───────────────────────────────────────────┘".bright_green());
    println!();
}

// ── response formatting ───────────────────────────────────

fn color_kaomojis(text: &str) -> String {
    let mut result = text.to_string();
    for k in KAOMOJIS {
        let colored = format!("{}", k.bright_magenta());
        result = result.replace(k, &colored);
    }
    result
}

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
