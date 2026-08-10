use colored::*;
use std::io::{stdout, Write};

use crate::syntax;
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

/// Rows the banner occupies. Must match `banner_lines()` exactly: 8 logo
/// lines + blank + version + system online + separator + blank.
pub const BANNER_HEIGHT: u16 = 13;

/// Render the banner (rainbow logo + themed info lines) as display lines.
/// Shared by the plain startup print and the docked top-pinned redraw so
/// they can't drift apart.
pub fn banner_lines() -> Vec<String> {
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
    let mut lines = Vec::new();
    for (line, color) in BANNER_LINES.iter().zip(colors.iter()) {
        lines.push(format!("  {}", line.color(*color)));
    }
    lines.push(String::new());
    lines.push(format!("  {} {}",
        "◆".bright_green(),
        "ayesha-os v4.5.0".bright_cyan()));
    lines.push(format!("  {} {}",
        theme::paint(Role::Dim, "  system online"),
        theme::paint(Role::Accent, "(๑蔷๑)")));
    lines.push(format!("  {}",
        theme::paint(Role::Dim, "──────────────────────────────────────────────")));
    lines.push(String::new());
    lines
}

pub fn print_banner() {
    for line in banner_lines() {
        println!("{}", line);
    }
}

// ── tool call / result ────────────────────────────────────

pub fn show_tool_call(name: &str, args: &str) {
    let truncated = if args.chars().count() > 80 {
        format!("{}...", crate::util::truncate_chars(args, 79))
    } else {
        args.to_string()
    };
    println!("  {} {} {}",
        theme::bold(Role::Primary, "▪"),
        theme::bold(Role::Text, name),
        theme::paint(Role::Dim, truncated));
}

const TOOL_BOX_WIDTH: usize = 78;

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1B' {
            while let Some(n) = chars.next() {
                if n == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn visible_len(s: &str) -> usize {
    strip_ansi(s).chars().count()
}

fn tool_box_line(content: &str, color: Role) -> String {
    let t = crate::util::truncate_chars(content, TOOL_BOX_WIDTH);
    // opencode renders tool/file content in syntax colors, not flat dim
    let highlighted = syntax::highlight_line(&t);
    let pad = TOOL_BOX_WIDTH.saturating_sub(visible_len(&highlighted));
    let padded = format!("{highlighted}{}", " ".repeat(pad));
    format!("  {}{}{}",
        theme::paint(color, "│"),
        theme::bg_fill(Role::CodeBg, padded),
        theme::paint(color, "│"))
}

pub fn show_tool_ok(name: &str, msg: &str) {
    let title = format!(" {} ", name);
    let dashes = TOOL_BOX_WIDTH.saturating_sub(title.chars().count());
    println!("  {}",
        theme::paint(Role::Dim, format!("┌{}{}┐", title, "─".repeat(dashes))));
    let content: Vec<&str> = msg.lines().collect();
    let shown = content.len().min(8);
    for line in content.iter().take(shown) {
        println!("{}", tool_box_line(line, Role::Dim));
    }
    if content.len() > shown {
        println!("{}", tool_box_line(&format!("… +{} more lines", content.len() - shown), Role::Dim));
    }
    println!("  {}",
        theme::paint(Role::Dim, format!("└{}┘", "─".repeat(TOOL_BOX_WIDTH))));
}

pub fn show_tool_err(name: &str, msg: &str) {
    let title = format!(" {} ", name);
    let dashes = TOOL_BOX_WIDTH.saturating_sub(title.chars().count());
    println!("  {}",
        theme::paint(Role::Error, format!("┌{}{}┐", title, "─".repeat(dashes))));
    println!("{}", tool_box_line(msg, Role::Error));
    println!("  {}",
        theme::paint(Role::Error, format!("└{}┘", "─".repeat(TOOL_BOX_WIDTH))));
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

/// Echo the user's message opencode-style: a solid pink accent bar, then the
/// message text on a raised panel, wrapped at a readable width.
pub fn show_user_msg(msg: &str) {
    print!("{}", format_user_msg(msg));
    stdout().flush().ok();
}

const USER_CARD_WIDTH: usize = 84;

fn user_card_line(content: &str) -> String {
    let bar = theme::bg(Role::Primary, " ");
    let body = theme::bg(Role::CodeBg, format!(" {:<width$} ", content, width = USER_CARD_WIDTH));
    format!("  {}{}", bar, body)
}

/// Pure formatter for the user message card (testable, no stdout).
fn format_user_msg(msg: &str) -> String {
    let wrap = USER_CARD_WIDTH - 6; // room for the "you ▸ " marker on line 1
    let first = theme::bold(Role::Primary, "you");
    let mut parts: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut count = 0usize;
    for c in msg.chars() {
        line.push(c);
        count += 1;
        if count >= wrap && c.is_whitespace() {
            parts.push(line.trim_end().to_string());
            line.clear();
            count = 0;
        }
    }
    if !line.trim().is_empty() {
        parts.push(line.trim().to_string());
    }
    if parts.is_empty() {
        parts.push(msg.to_string());
    }
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i == 0 {
            out.push_str(&user_card_line(&format!("{} ▸ {}", first, p)));
        } else {
            out.push_str(&user_card_line(p));
        }
        out.push('\n');
    }
    out
}

/// Thin dim separator between turns, opencode-style.
pub fn show_turn_separator() {
    println!("  {}",
        theme::paint(Role::Primary, "─".repeat(38)));
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
    println!("  {} {}",
        theme::bold(Role::Primary, "▪"),
        theme::bold(Role::Text, model));
}

pub fn show_interrupted() {
    println!("  {}",
        theme::bold(Role::Warning, "⏹  interrupted"));
}

// ── prompt ─────────────────────────────────────────────────

fn prompt_chars() -> String {
    format!("  {} {} ",
        theme::bg(Role::Primary, " "),
        theme::bold(Role::Primary, "$"))
}

pub fn prompt_line() {
    print!("{}", prompt_chars());
    stdout().flush().ok();
}

pub fn menu_prompt() {
    print!("  {} {} ",
        theme::bold(Role::Accent, "\u{25b6}"),
        theme::paint(Role::Accent, "launch"));
    stdout().flush().ok();
}

// ── docked bottom prompt (opencode-style) ──────────────────
// The conversation scrolls inside a DECSTBM region (rows 1..h-2) while the
// status bar (row h-1) and the input line (row h) stay pinned to the bottom.

static DOCK_STATUS: std::sync::OnceLock<std::sync::RwLock<String>> =
    std::sync::OnceLock::new();

fn dock_status_slot() -> &'static std::sync::RwLock<String> {
    DOCK_STATUS.get_or_init(|| std::sync::RwLock::new(String::new()))
}

/// Dock geometry as 1-based row numbers, or None when the terminal is too
/// small (banner + conversation + status + prompt can't all fit) — callers
/// then fall back to plain prompt behaviour.
fn dock_geometry() -> Option<(u16, u16, u16, u16)> {
    let (_, rows) = crossterm::terminal::size().ok()?;
    if rows <= 5 {
        return None;
    }
    let region_top = BANNER_HEIGHT + 1;
    let region_bottom = rows.saturating_sub(2);
    if region_top > region_bottom {
        return None;
    }
    Some((region_top, region_bottom, rows - 1, rows)) // (region_top, region_bottom, status_row, input_row)
}

/// Redraw the rainbow banner at the top of the screen (rows 1..BANNER_HEIGHT).
/// Keeps it pinned there: it lives above the DECSTBM scroll region, so
/// conversation output scrolls beneath it and `/clear` can restore it.
pub fn dock_draw_banner() {
    if dock_geometry().is_none() {
        return;
    }
    print!("\x1B[1;1H");
    for line in banner_lines() {
        print!("\x1B[2K{}\n", line);
    }
    stdout().flush().ok();
}

fn dock_status_str() -> String {
    dock_status_slot().read().map(|s| s.clone()).unwrap_or_default()
}

fn render_status(status: &str) -> String {
    let (cols, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let max = (cols as usize).saturating_sub(4);
    let t = crate::util::truncate_chars(status, max);
    format!("  {} {}",
        theme::bold(Role::Primary, "◆"),
        theme::paint(Role::Dim, t))
}

/// Set the initial status text and pin the scroll region + status bar +
/// prompt. Call once after the startup banner.
pub fn dock_init(status: &str) {
    let _ = dock_status_slot().write().map(|mut s| { *s = status.to_string(); });
    // ask the terminal to use the theme's background colour (opencode-style)
    let bg_hex = theme::get().hex(Role::Background).to_string();
    print!("\x1b]11;{}\x07", bg_hex);
    dock_redraw_bottom();
}

/// Update the status text (e.g. after a model switch) and redraw its line.
pub fn dock_status(status: &str) {
    let _ = dock_status_slot().write().map(|mut s| { *s = status.to_string(); });
    let Some((_, _, status_row, _)) = dock_geometry() else { return; };
    print!("\x1B[{};1H\x1B[2K{}", status_row, render_status(status));
    stdout().flush().ok();
}

/// Re-apply the scroll region and redraw banner + status bar + prompt. Used on
/// init, resize, clear and after leaving the popup screen.
pub fn dock_redraw_bottom() {
    let Some((region_top, region_bottom, status_row, input_row)) = dock_geometry() else { return; };
    dock_draw_banner();
    print!("\x1B[{};{}r", region_top, region_bottom);
    print!("\x1B[{};1H\x1B[2K{}", status_row, render_status(&dock_status_str()));
    print!("\x1B[{};1H\x1B[2K{}", input_row, prompt_chars());
    stdout().flush().ok();
}

/// True when the dock is active (terminal big enough). Lets the input thread
/// pick dock-aware behaviour.
pub fn dock_active() -> bool {
    dock_geometry().is_some()
}

/// Draw the input prompt at its docked row (or inline when not docked).
pub fn dock_prompt() {
    let Some((_, _, _, input_row)) = dock_geometry() else {
        prompt_line();
        return;
    };
    print!("\x1B[{};1H\x1B[2K{}", input_row, prompt_chars());
    stdout().flush().ok();
}

/// Move the cursor to the bottom of the conversation region so the echoed
/// input and everything printed this turn renders inside the dock, scrolling
/// older content up. No-op when not docked.
pub fn dock_submit_goto() {
    let Some((_, region_bottom, _, _)) = dock_geometry() else { return; };
    print!("\x1B[{};1H", region_bottom);
    stdout().flush().ok();
}

/// Re-establish the dock after the terminal was resized or the screen cleared.
pub fn dock_refresh() {
    dock_redraw_bottom();
}

// ── centered popup overlay ─────────────────────────────────

const APP_MENU_WIDTH: usize = 68;

/// Switch to the alternate screen buffer so a popup can be drawn centered;
/// leaving it restores the conversation scrollback exactly.
pub fn popup_enter() {
    // reset the scroll region on the alt screen so the popup can be centered
    print!("\x1B[?1049h\x1B[2J\x1B[r");
    stdout().flush().ok();
}

pub fn popup_leave() {
    print!("\x1B[?1049l");
    stdout().flush().ok();
    // primary buffer keeps its docked region; re-pin status bar + prompt
    dock_redraw_bottom();
}

/// Clear the overlay and redraw `rendered` centered on both axes.
/// Lines beyond the terminal height are clipped so a short window can't
/// overflow the popup.
pub fn draw_popup_centered(rendered: &str, line_count: usize) {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((120, 30));
    let max_rows = (rows as usize).saturating_sub(2).max(1);
    let clip = line_count.min(max_rows);
    let clipped: String = rendered.lines().take(clip).collect::<Vec<_>>().join("\n");
    let row = (rows as usize).saturating_sub(clip) / 2;
    // menu box (incl. the 2-space indent) is 68 chars; center the box itself
    let box_col = ((cols as usize).saturating_sub(APP_MENU_WIDTH - 2) / 2).max(3);
    let col = box_col.saturating_sub(2).max(1);
    print!("\x1B[2J\x1B[{};{}H{}", row.max(1), col, clipped);
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
    let cb = theme::get().color(Role::CodeBg);
    let pop_line = |fg: Role, s: String| -> String {
        theme::paint(fg, s).on_color(cb).to_string()
    };
    // every line is a solid raised panel so the popup reads as a popout
    out.push_str(&format!("  {}\n",
        pop_line(Role::Primary, format!("┌─{}{}┐", title, "─".repeat(dashes)))));
    out.push_str(&format!("  {}\n",
        pop_line(Role::Dim, format!("│{}│", "─".repeat(inner_w)))));
    // category header, opencode palette style
    out.push_str(&format!("  {}\n",
        pop_line(Role::Success, format!("│  {:<62}│", "applets"))));

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
        out.push_str(&format!("  {}\n",
            pop_line(Role::Dim, format!("│  {:<62}│", format!("no matches for '{}'", filter)))));
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
            if selected {
                // opencode-style hot-pink selection bar with black text,
                // full width across the panel
                let bar = theme::bg(Role::Primary, format!("  {:<62}", truncated)).black();
                out.push_str(&format!("  {}{}{}\n",
                    pop_line(Role::Primary, "│".to_string()),
                    bar,
                    pop_line(Role::Primary, "│".to_string())));
            } else {
                out.push_str(&format!("  {}\n",
                    pop_line(Role::Text, format!("│  {:<62}│", truncated))));
            }
        }
    }

    out.push_str(&format!("  {}\n",
        pop_line(Role::Dim, format!("│{}│", "─".repeat(inner_w)))));
    let filter_display = if filter.is_empty() { "type to filter...".to_string() } else { format!("filter: {}", filter) };
    out.push_str(&format!("  {}\n",
        pop_line(Role::Dim, format!("│  {:<62}│", filter_display))));
    out.push_str(&format!("  {}\n",
        pop_line(Role::Dim, format!("│  {:<62}│", "● running  ○ stopped  ↑↓ nav  enter launch  x stop  esc back"))));
    out.push_str(&format!("  {}\n",
        pop_line(Role::Primary, format!("└{}┘", "─".repeat(inner_w)))));

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
        let expected = strip_ansi(lines[0]).chars().count();
        for line in &lines {
            assert_eq!(strip_ansi(line).chars().count(), expected, "misaligned line: {}", line);
        }
        assert!(out.contains("►"));
        assert!(out.contains("applet launcher"));
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1B' {
                while let Some(n) = chars.next() {
                    if n == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn user_msg_single_line_for_short_input() {
        let out = strip_ansi(&format_user_msg("hello"));
        assert_eq!(out.lines().count(), 1);
        assert!(out.contains("hello"));
        assert!(out.contains("you"));
    }

    #[test]
    fn user_msg_wraps_long_input() {
        let long = "word ".repeat(30); // 150 chars, breaks at whitespace
        let out = strip_ansi(&format_user_msg(&long));
        assert!(out.lines().count() > 1, "long message was not wrapped");
        let lines: Vec<&str> = out.lines().collect();
        // marker only on the first line, all lines are card panels
        assert!(lines[0].contains("you ▸"), "bad first line: {}", lines[0]);
        for line in &lines {
            assert!(line.contains("word"), "missing content: {}", line);
        }
    }

    #[test]
    fn user_msg_handles_multibyte() {
        let out = strip_ansi(&format_user_msg("hmm desu-ne (๑•蔷•๑)"));
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

    // opencode-style solid palette popout: raised panel + pink frame
    let pop = |s: String| -> String { theme::bg_fill(Role::CodeBg, s) };

    println!();
    println!("  {}",
        pop(theme::paint(Role::Primary, "┌──────────────────────────────────────────────────┐").to_string()));
    let header = if let Some(f) = filter {
        if f.is_empty() {
            "  ◆  command palette".to_string()
        } else {
            format!("  ◆  command palette  /{}", f)
        }
    } else {
        "  ◆  command palette".to_string()
    };
    println!("  {}",
        pop(theme::paint(Role::Primary, format!("{:<47}│", theme::paint(Role::Accent, header))).to_string()));
    println!("  {}",
        pop(theme::bold(Role::Success, format!("│  {:<46}│", "commands")).to_string()));

    if filtered.is_empty() {
        println!("  {}",
            pop(theme::paint(Role::Primary,
                format!("{:<47}│",
                    theme::paint(Role::Dim, format!("  no match for '/{}'", filter.unwrap_or(""))).italic()
                )).to_string()));
    } else {
        println!("  {}",
            pop(theme::paint(Role::Primary,
                format!("{:<48}│",
                    theme::paint(Role::Dim, "─".repeat(box_w - 4))
                )).to_string()));
        let display = if filtered.len() > 10 { &filtered[..10] } else { &filtered };
        for (cmd, desc) in display {
            let line = format!("  │  /{:<10} {:<31}│", cmd, desc);
            println!("{}", pop(theme::paint(Role::Text, line).to_string()));
        }
        println!("  {}",
            pop(theme::paint(Role::Primary,
                format!("{:<48}│",
                    theme::paint(Role::Dim, "─".repeat(box_w - 4))
                )).to_string()));
    }
    println!("  {}",
        pop(theme::paint(Role::Primary, "└──────────────────────────────────────────────────┘").to_string()));
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
