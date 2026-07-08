use colored::*;
use std::io::{stdout, Write};
use std::time::Duration;

// ── retro DOS color scheme ─────────────────────────────────
// :: cyan   >> green   OK green   !! red   ;; yellow

const KAOMOJIS: &[&str] = &[
    "(╯°□°)╯︵ ┻━┻", "(◕ᴗ◕✿)", "(๑•蔷•๑)", "(╥﹏╥)",
    "^_^", ">w<", ":3", "(ᵔᴥᵔ)", "(◕‿◕)",
    "(ﾉ◕ヮ◕)ﾉ", "¯\\_(ツ)_/¯", "(づ｡◕‿‿◕｡)づ",
    "(•ω•)", "(｡•̀ᴗ-)✧", "♪～(´ε｀ )",
    "(ノಠ益ಠ)ノ", "┻━┻", "┬─┬", "◥▅◤",
    "kapoo!", "desu-ne", "desu--",
];

// ── banner ─────────────────────────────────────────────────

const BANNER: &str = r#"
                       _
                      | |
  __ _ _   _  ___  ___| |__   __ _ ______ ___  ___
 / _` | | | |/ _ \/ __| '_ \ / _` |______/ _ \/ __|
| (_| | |_| |  __/\__ \ | | | (_| |     | (_) \__ \
 \__,_|\__, |\___||___/_| |_|\__,_|      \___/|___/
        __/ |
       |___/
"#;

pub fn print_banner() {
    for line in BANNER.lines() {
        println!("  {}", line.bright_cyan());
    }
    println!();
    println!("  {} {}", "ayesha-os".bright_cyan().bold(), "v4.2.0".bright_black());
    println!("  {}", "(๑蔷蔷๑)".bright_magenta());
    println!();
    println!("  {}", "type 'help' for commands, 'exit' to quit".bright_black());
    println!("  {}", "──────────────────────────────────────────────".bright_black());
    println!();
}

// ── tool call / result (flash instantly, no typewriter) ───

pub fn show_tool_call(name: &str, args: &str) {
    let truncated = if args.len() > 80 {
        format!("{}...", &args[..79])
    } else {
        args.to_string()
    };
    println!("  {} {} {}", ">>".bright_green().bold(), name.bright_cyan(), truncated.bright_black());
}

pub fn show_tool_ok(name: &str, msg: &str) {
    let first = msg.lines().next().unwrap_or(msg);
    let truncated = if first.len() > 120 {
        format!("{}...", &first[..119])
    } else {
        first.to_string()
    };
    println!("  {} {} {}", "OK".bright_green().bold(), name.bright_cyan(), truncated.bright_black());
    for line in msg.lines().skip(1).take(5) {
        println!("  {} {}", "  ".bright_black(), line.bright_black());
    }
    if msg.lines().count() > 6 {
        println!("  {} {}", "  ".bright_black(), format!("+{} more lines", msg.lines().count() - 6).bright_black());
    }
}

pub fn show_tool_err(name: &str, msg: &str) {
    println!("  {} {} {}", "!!".bright_red().bold(), name.bright_cyan(), msg.bright_red());
}

// ── system messages ───────────────────────────────────────

pub fn show_system(msg: &str) {
    println!("  {} {}", "::".bright_cyan(), msg.bright_cyan());
}

pub fn show_error(msg: &str) {
    println!("  {} {}", "!!".bright_red(), msg.bright_red());
}

pub fn show_processing() {
    print!("  {} {}", "::".bright_cyan(), "processing...".bright_black());
    stdout().flush().ok();
}

pub fn hide_processing() {
    print!("\r");
    for _ in 0..40 {
        print!(" ");
    }
    print!("\r");
    stdout().flush().ok();
}

pub fn show_routing(model: &str) {
    println!("  {}", format!("-- {} --", model).bright_black());
}

pub fn show_interrupted() {
    println!("  {}", "-- interrupted --".bright_yellow().bold());
}

pub fn show_skipped() {
    // prints nothing, just a visual cue that we skipped
}

// ── prompt ─────────────────────────────────────────────────

pub fn prompt_line() {
    print!("  {} ", "fox>".bright_green().bold());
    stdout().flush().ok();
}

// ── help ───────────────────────────────────────────────────

pub fn print_help() {
    println!();
    println!("  {}", ":: commands ::".bright_cyan());
    println!();
    println!("    {:<18}{}", "help".bright_cyan(), "show this message".bright_black());
    println!("    {:<18}{}", "exit".bright_cyan(), "quit ayesha-os".bright_black());
    println!("    {:<18}{}", "clear".bright_cyan(), "clear screen".bright_black());
    println!("    {:<18}{}", "models".bright_cyan(), "list available models".bright_black());
    println!("    {:<18}{}", "model <name>".bright_cyan(), "switch model manually".bright_black());
    println!("    {:<18}{}", "auto".bright_cyan(), "re-enable auto-routing".bright_black());
    println!("    {:<18}{}", "pull <name>".bright_cyan(), "pull model from ollama".bright_black());
    println!();
    println!("  {}", ":: self-improvement ::".bright_cyan());
    println!();
    println!("    {:<18}{}", "stats".bright_cyan(), "tool usage statistics".bright_black());
    println!("    {:<18}{}", "memory".bright_cyan(), "list stored memories".bright_black());
    println!("    {:<18}{}", "analyze".bright_cyan(), "analyze own source code".bright_black());
    println!("    {:<18}{}", "evolve".bright_cyan(), "suggest new tools".bright_black());
    println!("    {:<18}{}", "refine".bright_cyan(), "analyze prompt history".bright_black());
    println!();
    println!("  {}", ":: tools (auto-called by model) ::".bright_cyan());
    println!();
    println!("    {:<20}{}", "read_file".bright_cyan(), "read any file on disk".bright_black());
    println!("    {:<20}{}", "write_file".bright_cyan(), "create or overwrite files".bright_black());
    println!("    {:<20}{}", "list_dir".bright_cyan(), "browse directories".bright_black());
    println!("    {:<20}{}", "generate_html".bright_cyan(), "render html to file".bright_black());
    println!("    {:<20}{}", "generate_sprite".bright_cyan(), "create pixel art characters".bright_black());
    println!("    {:<20}{}", "remember".bright_cyan(), "store a memory".bright_black());
    println!("    {:<20}{}", "search_memories".bright_cyan(), "search stored memories".bright_black());
    println!("    {:<20}{}", "analyze_self".bright_cyan(), "ai code review".bright_black());
    println!("    {:<20}{}", "evolve_tools".bright_cyan(), "suggest new tools".bright_black());
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
            "│".bright_black(),
            line.bright_white().on_bright_black()
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

// ── typewriter effect ─────────────────────────────────────

pub async fn typewrite_response(
    text: &str,
    steer_rx: &std::sync::mpsc::Receiver<String>,
) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '\x1b' && i + 1 < len && chars[i + 1] == '[' {
            let start = i;
            i += 2;
            while i < len && !chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let seq: String = chars[start..i].iter().collect();
            print!("{}", seq);
            stdout().flush().ok();
            continue;
        }

        print!("{}", chars[i]);
        stdout().flush().ok();

        match steer_rx.try_recv() {
            Ok(input) if !input.is_empty() => {
                let rest: String = chars[i + 1..].iter().collect();
                print!("{}", rest);
                stdout().flush().ok();
                println!();
                return Some(input);
            }
            Ok(_) => {
                let rest: String = chars[i + 1..].iter().collect();
                print!("{}", rest);
                stdout().flush().ok();
                println!();
                return None;
            }
            _ => {}
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
        i += 1;
    }

    None
}
