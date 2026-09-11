//! ayesha-mini — clean minimal tui: chat + model picker + memory only.
//! no applets. single writer, pinned banner, short picker.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    cursor,
};
use serde::{Deserialize, Serialize};
use std::io::{Stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

// ── theme: monokai++ values reused verbatim from engine/src/theme.rs ──
// background stack matches opencode: bg / bgDeeper / bgSubtle.
pub mod theme {
    use crossterm::style::Color;
    pub const BACKGROUND: &str = "#221F22";
    pub const SURFACE: &str = "#262226";
    pub const TEXT: &str = "#FCFCFA";
    pub const PRIMARY: &str = "#FF6188";
    pub const ACCENT: &str = "#FF6188";
    pub const SECONDARY: &str = "#78DCE8";
    pub const SUCCESS: &str = "#A9DC76";
    pub const WARNING: &str = "#FFD866";
    pub const ERROR: &str = "#FF5F5F";
    pub const DIM: &str = "#6C696E";
    pub const BORDER: &str = "#454147";
    pub const CODE_BG: &str = "#353238";

    pub fn rgb(hex: &str) -> (u8, u8, u8) {
        let h = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
        (r, g, b)
    }

    pub fn color(hex: &str) -> Color {
        let (r, g, b) = rgb(hex);
        Color::Rgb { r, g, b }
    }

    /// truecolor background escape, used by selftest to prove bg fill exists.
    pub fn bg_escape(hex: &str) -> String {
        let (r, g, b) = rgb(hex);
        format!("\x1b[48;2;{};{};{}m", r, g, b)
    }

    /// truecolor foreground escape.
    pub fn fg_escape(hex: &str) -> String {
        let (r, g, b) = rgb(hex);
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    }

    /// pad / truncate a row to exactly cols cells so the background fill
    /// covers the full width with no gaps. narrow-safe.
    pub fn full_row(text: &str, cols: usize) -> String {
        let cols = cols.max(10);
        let n = text.chars().count();
        if n == cols {
            return text.to_string();
        }
        if n > cols {
            return text.chars().take(cols).collect();
        }
        let mut o = text.to_string();
        o.push_str(&" ".repeat(cols - n));
        o
    }

    /// stamp chunk colour: always monokai success green for irc stamps.
    pub fn stamp_color() -> Color {
        color(SUCCESS)
    }

    /// body fg is locked uniform: every reply and user body line is
    /// single text colour with no mid-message role or markdown switches.
    /// nicks keep primary pink, notices keep dim or success or error or
    /// warning, code snippets stay text with dim prefix only.
    /// stamp colour handled separately as success green.
    pub fn body_color_for_line(line: &str) -> &'static str {
        let body = crate::irc::strip_stamp(line);
        let l = body.to_lowercase();
        let t = l.trim();
        if !t.is_empty() && t.chars().all(|c| c == '─' || c == ' ' || c == '-') {
            return DIM;
        }
        if l.trim_start().starts_with("***") {
            if l.contains("error") || l.contains("failed") {
                return ERROR;
            }
            if l.contains("thinking") {
                return WARNING;
            }
            if l.contains("switched to") || l.contains("joined") || l.contains("saved to") || l.contains("slime") {
                return SUCCESS;
            }
            return DIM;
        }
        // thinking row without stars still warns so it reads attached.
        if l.trim() == "thinking…" || l.trim() == "thinking..." {
            return WARNING;
        }
        // everything else — chat bodies, code, markdown-looking text —
        // stays uniform text. error words inside a reply never flip it.
        TEXT
    }

    /// role fg for single-chunk fallback rows (separators, unstamped).
    /// stamped viewport rows split stamp green plus nick primary plus
    /// uniform body via body_color_for_line instead.
    pub fn role_for_line(line: &str) -> &'static str {
        let body = crate::irc::strip_stamp(line);
        let l = body.to_lowercase();
        if l.contains("<fox>") || l.contains("<ayesha>") || l.contains("<ash>") || l.contains("<q7>") || l.contains("<q14>") || l.contains("<l32>") || l.contains("<ms13>") || l.contains("<msc>") || l.contains("<msf>") || l.starts_with("you:") {
            return PRIMARY;
        }
        body_color_for_line(line)
    }

    /// nick token fg: primary pink for nicks only.
    pub fn nick_color() -> Color {
        color(PRIMARY)
    }
}

// ── banner: ayesha-os rainbow logo ported to a pinned 7-row header ──
// source: engine/src/ui.rs BANNER_LINES (8 ascii rows) + per-line rainbow
// colors (bright red/yellow/green/cyan/blue/magenta) + version line +
// system online line + separator. mini keeps 7 rows: 4 logo rows with the
// same rainbow order, version, japanese subline + slime form loaded, rule.
// same monokai++ palette as theme.rs. never pushed to history, full redraw
// each frame at rows 0..7, safely truncated on narrow windows.
pub mod banner {
    use anyhow::Result;
    use crossterm::{execute, style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor}};
    use std::io::Write;
    // 4 most distinctive logo rows from ui.rs BANNER_LINES[2..6].
    pub const LOGO: [&str; 4] = [
        r"  __ _ _   _  ___  ___| |__   __ _ ______ ___  ___",
        r" / _` | | | |/ _ \/ __| '_ \ / _` |______/ _ \/ __|",
        r"| (_| | |_| |  __/\__ \ | | | (_| |     | (_) \__ \",
        r" \__,_|\__, |\___||___/_| |_|\__,_|      \___/|___/",
    ];
    // same rainbow order as ui.rs banner_lines(): red/yellow/green/cyan,
    // mapped to theme palette hexes so mini stays on monokai++.
    pub const RAINBOW: [&str; 4] = [
        crate::theme::ERROR,     // bright red
        crate::theme::WARNING,   // bright yellow
        crate::theme::SUCCESS,   // bright green
        crate::theme::SECONDARY, // bright cyan
    ];
    pub const VERSION_LINE: &str = "◆ ayesha-os v4.5.0 — mini :3";
    pub const JAPANESE: &str = "あやしゃ system online (◕ᴗ◕✿)";
    pub const SLIME_LOADED: &str = "slime form loaded ♪";
    pub const HEIGHT: u16 = 7;

    pub fn truncate_cells(s: &str, cols: usize) -> String {
        if s.chars().count() <= cols {
            return s.to_string();
        }
        if cols <= 3 {
            return s.chars().take(cols).collect();
        }
        format!("{}...", s.chars().take(cols.saturating_sub(3)).collect::<String>())
    }

    pub fn plain_lines(cols: usize) -> Vec<String> {
        let cols = cols.max(10);
        let mut out = Vec::new();
        for line in LOGO {
            out.push(truncate_cells(&format!("  {}", line), cols));
        }
        out.push(truncate_cells(&format!("  {}", VERSION_LINE), cols));
        out.push(truncate_cells(&format!("  {} ・ {}", JAPANESE, SLIME_LOADED), cols));
        let rule: String = "─".repeat(cols.saturating_sub(2).min(120));
        out.push(truncate_cells(&rule, cols));
        out
    }

    pub fn render(out: &mut impl Write, cols: usize) -> Result<()> {
        use crate::theme;
        let cols = cols.max(10);
        let surf = theme::color(theme::SURFACE);
        // rows 1-4: rainbow logo as is, but on painted surface background.
        for (i, line) in LOGO.iter().enumerate() {
            let c = theme::color(RAINBOW[i % RAINBOW.len()]);
            let txt = theme::full_row(&truncate_cells(&format!("  {}", line), cols), cols);
            execute!(out, SetForegroundColor(c), SetBackgroundColor(surf), Print(format!("{}\r\n", txt)), ResetColor)?;
        }
        // row 5: version in secondary on surface.
        let cyan = theme::color(theme::SECONDARY);
        let vtxt = theme::full_row(&truncate_cells(&format!("  {}", VERSION_LINE), cols), cols);
        execute!(out, SetForegroundColor(cyan), SetBackgroundColor(surf), Print(format!("{}\r\n", vtxt)), ResetColor)?;
        // row 6: japanese dim + slime success share one combined row on surface.
        // paint base dim, slime tail gets success via second pass below is too
        // fancy for a 7-row header, so base dim + full surface fill keeps it calm.
        let dim = theme::color(theme::DIM);
        let combined = theme::full_row(&truncate_cells(&format!("  {} ・ {}", JAPANESE, SLIME_LOADED), cols), cols);
        execute!(out, SetForegroundColor(dim), SetBackgroundColor(surf), Print(format!("{}\r\n", combined)), ResetColor)?;
        // row 7: separator rule, dim on surface.
        let border = theme::color(theme::BORDER);
        let rule: String = "─".repeat(cols.saturating_sub(2).min(120));
        let rtxt = theme::full_row(&truncate_cells(&rule, cols), cols);
        execute!(out, SetForegroundColor(border), SetBackgroundColor(surf), Print(format!("{}\r\n", rtxt)), ResetColor)?;
        Ok(())
    }
}

// ── writer: every chat line flows through here, wraps at terminal width ──
pub mod writer {
    use crossterm::terminal;
    pub fn term_cols() -> usize {
        terminal::size().map(|(w, _)| w as usize).unwrap_or(80).max(20)
    }

    pub fn card_width(cols: usize) -> usize {
        cols.saturating_sub(4).min(72).max(20)
    }

    pub fn wrap_line(line: &str, width: usize) -> Vec<String> {
        let width = width.max(10);
        if line.is_empty() {
            return vec![String::new()];
        }
        let mut out = Vec::new();
        let mut cur = String::new();
        let mut cur_len = 0usize;
        for word in line.split(' ') {
            let wlen = word.chars().count();
            if wlen > width {
                // hard-break one giant token
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    cur_len = 0;
                }
                let mut chunk = String::new();
                let mut n = 0;
                for ch in word.chars() {
                    chunk.push(ch);
                    n += 1;
                    if n >= width {
                        out.push(std::mem::take(&mut chunk));
                        n = 0;
                    }
                }
                if !chunk.is_empty() {
                    cur = chunk;
                    cur_len = cur.chars().count();
                }
                continue;
            }
            let extra = if cur.is_empty() { 0 } else { 1 };
            if cur_len + extra + wlen > width {
                out.push(std::mem::take(&mut cur));
                cur = word.to_string();
                cur_len = wlen;
            } else {
                if !cur.is_empty() {
                    cur.push(' ');
                }
                cur.push_str(word);
                cur_len += extra + wlen;
            }
        }
        out.push(cur);
        out
    }

    pub fn wrap_block(text: &str, width: usize) -> Vec<String> {
        let mut rows = Vec::new();
        for line in text.split('\n') {
            rows.extend(wrap_line(line, width));
        }
        rows
    }

    /// thin dim separator, capped to cols so 80-col never overflows.
    pub fn separator(cols: usize) -> String {
        let n = cols.saturating_sub(2).min(38).max(10);
        "─".repeat(n)
    }

    /// old irc style with timestamps + short nicks, no boxes:
    /// [hh:mm] <fox> / [hh:mm] <nick> plus [hh:mm] *** notices plus thin
    /// separators only. capped to cols, narrow-safe. keeps `card` signature
    /// so memory plus wrap plus truncation behavior stay untouched.
    pub fn card(title: &str, body: &str, cols: usize) -> Vec<String> {
        use crate::irc;
        let w = card_width(cols);
        let stamp_len = "[00:00] ".len();
        let t = title.trim().to_lowercase();
        // errors + status titles become server notices with timestamps
        if t == "error" {
            let mut rows = Vec::new();
            for r in wrap_block(body, w.saturating_sub(stamp_len + 11)) {
                rows.push(truncate_to(irc::with_stamp(&format!("*** error: {}", r)), w));
            }
            rows.push(separator(cols));
            return rows;
        }
        if t == "memory" || t == "help" || t == "model" || t == "welcome" {
            let mut rows = Vec::new();
            let first = wrap_block(body, w.saturating_sub(stamp_len + 12));
            let head = format!("*** {}: {}", title.trim(), first.first().cloned().unwrap_or_default());
            rows.push(truncate_to(irc::with_stamp(&head), w));
            for r in first.iter().skip(1) {
                rows.push(truncate_to(irc::with_stamp(r), w));
            }
            rows.push(separator(cols));
            return rows;
        }
        // assistant replies render as [hh:mm] <nick> irc lines
        let nick = irc::short_nick(title);
        let prefix = format!("<{}> ", nick);
        let mut rows = Vec::new();
        let wrapped = wrap_block(body, w.saturating_sub(stamp_len + prefix.len()));
        for (i, r) in wrapped.iter().enumerate() {
            if i == 0 {
                rows.push(truncate_to(irc::with_stamp(&format!("{}{}", prefix, r)), w));
            } else {
                rows.push(truncate_to(irc::with_stamp(r), w));
            }
        }
        rows.push(separator(cols));
        rows
    }

    /// irc user line: [hh:mm] <fox> hi, wrapped, no boxes.
    pub fn user_line(text: &str, cols: usize) -> Vec<String> {
        use crate::irc;
        let w = card_width(cols);
        let stamp_len = "[00:00] ".len();
        let wrapped = wrap_block(text, w.saturating_sub(stamp_len + "<fox> ".len()));
        let mut rows = Vec::new();
        for (i, r) in wrapped.iter().enumerate() {
            if i == 0 {
                rows.push(truncate_to(irc::with_stamp(&format!("<fox> {}", r)), w));
            } else {
                rows.push(truncate_to(irc::with_stamp(r), w));
            }
        }
        rows
    }

    /// irc server notice: [hh:mm] *** switched to qwen, wrapped, no boxes.
    pub fn notice_line(text: &str, cols: usize) -> Vec<String> {
        use crate::irc;
        let w = card_width(cols);
        let stamp_len = "[00:00] ".len();
        let mut rows = Vec::new();
        for r in wrap_block(text, w.saturating_sub(stamp_len + 4)) {
            rows.push(truncate_to(irc::with_stamp(&format!("*** {}", r)), w));
        }
        rows.push(separator(cols));
        rows
    }

    #[allow(dead_code)]
    fn pad_to(s: &str, n: usize) -> String {
        let len = s.chars().count();
        if len >= n {
            return s.chars().take(n).collect();
        }
        let mut o = s.to_string();
        o.push_str(&" ".repeat(n - len));
        o
    }

    fn truncate_to(s: String, w: usize) -> String {
        if s.chars().count() <= w {
            s
        } else {
            s.chars().take(w).collect()
        }
    }
}

// ── ollama types (mirrors engine/src/ollama.rs tool-capability rule) ──
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub call_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    #[serde(deserialize_with = "de_args")]
    pub arguments: serde_json::Value,
}

fn de_args<'de, D>(d: D) -> std::result::Result<serde_json::Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(d)?;
    match raw {
        serde_json::Value::String(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom),
        v => Ok(v),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

fn ollama_base() -> String {
    std::env::var("OLLAMA_HOST")
        .or_else(|_| std::env::var("OLLAMA_BASE_URL"))
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "http://localhost:11434".to_string())
}

/// tiny persona models reject `tools` with 400 — same rule as engine.
pub fn is_tool_capable(model: &str) -> bool {
    let m = model.to_lowercase();
    if m.contains("ayesha") || m.contains("tiny") || m.contains("0.5b") {
        return false;
    }
    true
}

pub fn default_model() -> String {
    std::env::var("AYESHA_MODEL").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| "qwen2.5:7b".to_string())
}

// ── config: ayesha.json ollama.models + live tags + keyed remote only ──
pub mod config {
    use std::path::PathBuf;

    pub fn config_path() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("AYESHA_CONFIG") {
            let pb = PathBuf::from(p);
            if pb.is_file() {
                return Some(pb);
            }
        }
        for c in [
            r"c:\ayesha-os2\ayesha.json",
            r"C:\ayesha-os2\ayesha.json",
            "./ayesha.json",
            "../ayesha-os2/ayesha.json",
        ] {
            let pb = PathBuf::from(c);
            if pb.is_file() {
                return Some(pb);
            }
        }
        // walk up from cwd
        let mut cur = std::env::current_dir().ok()?;
        loop {
            let cand = cur.join("ayesha.json");
            if cand.is_file() {
                return Some(cand);
            }
            if !cur.pop() {
                break;
            }
        }
        None
    }

    pub fn file_models() -> (Vec<String>, Vec<String>) {
        let mut local = Vec::new();
        let mut remote = Vec::new();
        let Some(path) = config_path() else {
            return (vec!["qwen2.5:7b".into(), "qwen2.5-coder:14b".into(), "llama3.2-vision".into()], vec![]);
        };
        let Ok(s) = std::fs::read_to_string(&path) else {
            return (vec!["qwen2.5:7b".into()], vec![]);
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) else {
            return (vec!["qwen2.5:7b".into()], vec![]);
        };
        if let Some(m) = v.get("ollama").and_then(|o| o.get("models")).and_then(|m| m.as_object()) {
            for k in m.keys() {
                local.push(k.clone());
            }
        }
        // remote only counts when its own key or auth is present.
        // openrouter rows need openrouter key, zen rows need zen auth,
        // meta spark rows need model_api_key. never hardcoded.
        let has_openrouter = std::env::var("OPENROUTER_API_KEY").map(|v| !v.trim().is_empty()).unwrap_or(false);
        let has_zen = crate::spark::has_zen_auth();
        let has_meta = crate::spark::has_meta_key();
        if let Some(cloud) = v.get("cloud_models").and_then(|c| c.as_object()) {
            for (prov, pv) in cloud {
                let prov_l = prov.to_lowercase();
                let allowed = if prov_l.contains("openrouter") {
                    has_openrouter
                } else if prov_l.contains("zen") {
                    has_zen
                } else if prov_l == "meta" {
                    has_meta
                } else {
                    has_openrouter || has_zen || has_meta
                };
                if !allowed {
                    continue;
                }
                if let Some(fm) = pv.get("free_models").and_then(|f| f.as_object()) {
                    for k in fm.keys() {
                        remote.push(k.clone());
                    }
                }
            }
        }
        if local.is_empty() {
            local.push("qwen2.5:7b".to_string());
        }
        local.sort();
        local.dedup();
        remote.sort();
        remote.dedup();
        (local, remote)
    }
}

async fn live_tags() -> Vec<String> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(2)).build();
    let Ok(client) = client else { return vec![] };
    let url = format!("{}/api/tags", ollama_base());
    let Ok(resp) = client.get(&url).send().await else { return vec![] };
    let Ok(v) = resp.json::<serde_json::Value>().await else { return vec![] };
    let mut out = Vec::new();
    if let Some(arr) = v.get("models").and_then(|m| m.as_array()) {
        for m in arr {
            if let Some(n) = m.get("name").and_then(|n| n.as_str()) {
                out.push(n.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// rebuild picker list on every open: pulled-only. live tags plus file
/// models that are actually pulled, deduped. remote stays keyed-only and
/// is also gated on pulled so unpulled cloud rows never appear as ready.
pub async fn build_picker_list() -> Vec<String> {
    let (file_models, remote) = config::file_models();
    let live = live_tags().await;
    build_picker_list_from(file_models, remote, live).0
}

pub fn build_picker_list_from(
    file_models: Vec<String>,
    remote: Vec<String>,
    live: Vec<String>,
) -> (Vec<String>, Vec<String>) {
    // pulled local rows plus keyed remote spark rows. remote is already
    // gated by key or auth in file_models, cloud rows show with ○.
    let mut list = pulled_picker_list(&file_models, &live);
    for r in remote {
        if !list.iter().any(|x| norm_model(x) == norm_model(&r)) {
            list.push(r);
        }
    }
    // belt and braces: spark ids only when key or auth present, no key hidden.
    list.retain(|m| {
        if crate::spark::is_spark(m) {
            crate::spark::visible_ids().iter().any(|v| norm_model(v) == norm_model(m))
        } else {
            true
        }
    });
    list.sort();
    list.dedup_by(|a, b| norm_model(a) == norm_model(b));
    (list, live)
}

pub fn filter_models(all: &[String], query: &str) -> Vec<String> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return all.to_vec();
    }
    all.iter().filter(|m| m.to_lowercase().contains(&q)).cloned().collect()
}

// ── picker pulled-only: never offer unpulled json models as ready ──
pub fn norm_model(m: &str) -> String {
    m.trim().to_lowercase()
}

pub fn is_pulled(model: &str, live: &[String]) -> bool {
    let n = norm_model(model);
    live.iter().any(|l| norm_model(l) == n)
}

/// pulled-only list: live tags plus file models that are actually pulled.
/// when live is non-empty the result equals live (file ∩ live ⊆ live).
/// when live is empty (ollama down) fall back to file list so the picker
/// still opens, rows then show ○ unpulled honestly.
pub fn pulled_picker_list(file_models: &[String], live: &[String]) -> Vec<String> {
    if live.is_empty() {
        let mut f = file_models.to_vec();
        f.sort();
        f.dedup();
        return f;
    }
    let mut all = live.to_vec();
    for m in file_models {
        if is_pulled(m, live) && !all.iter().any(|x| norm_model(x) == norm_model(m)) {
            all.push(m.clone());
        }
    }
    all.sort();
    all.dedup_by(|a, b| norm_model(a) == norm_model(b));
    all
}

pub fn fallback_model(live: &[String]) -> String {
    let want = "qwen2.5:7b".to_string();
    if live.is_empty() {
        return want;
    }
    if live.iter().any(|l| norm_model(l) == norm_model(&want)) {
        return want;
    }
    live.first().cloned().unwrap_or(want)
}

pub fn is_not_found_error(msg: &str) -> bool {
    let l = msg.to_lowercase();
    l.contains("not found") || l.contains("no such model") || l.contains("does not exist") || l.contains("model not found") || l.contains("404")
}

pub fn pull_hint(model: &str) -> String {
    format!("model '{}' not pulled. run: ollama pull {}\nfalling back to qwen2.5:7b :3", model, model)
}

// ── muse spark: meta direct + opencode zen free ──
pub mod spark {
    pub const META_BASE: &str = "https://api.meta.ai/v1";
    pub const ZEN_BASE: &str = "https://api.opencode.ai/v1";
    pub const META_13: &str = "muse-spark-1.3";
    pub const META_CONTRIB: &str = "muse-spark-1.3-contributor";
    pub const ZEN_FREE: &str = "muse-spark-1.3-contributor-free";

    pub fn all_ids() -> Vec<String> {
        vec![META_13.to_string(), META_CONTRIB.to_string(), ZEN_FREE.to_string()]
    }

    /// bearer key for meta direct. supports both cases.
    pub fn meta_key() -> Option<String> {
        std::env::var("MODEL_API_KEY")
            .or_else(|_| std::env::var("model_api_key"))
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    pub fn has_meta_key() -> bool {
        meta_key().is_some()
    }

    /// zen api key from env when set.
    pub fn zen_env_key() -> Option<String> {
        std::env::var("OPENCODE_ZEN_API_KEY")
            .or_else(|_| std::env::var("OPCODE_ZEN_API_KEY"))
            .or_else(|_| std::env::var("opencode_zen_api_key"))
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    fn home_dir() -> Option<std::path::PathBuf> {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(std::path::PathBuf::from)
    }

    fn opencode_auth_path() -> Option<std::path::PathBuf> {
        home_dir().map(|h| h.join(".config").join("opencode").join("auth.json"))
    }

    /// existing opencode auth file present and non-empty.
    pub fn has_opencode_auth_file() -> bool {
        opencode_auth_path()
            .and_then(|p| std::fs::metadata(&p).ok().map(|m| m.len() > 2))
            .unwrap_or(false)
    }

    /// zen counts when env key exists or auth file exists. zero cost path.
    pub fn has_zen_auth() -> bool {
        zen_env_key().is_some() || has_opencode_auth_file()
    }

    /// bearer for zen: env first, else best-effort token from auth file.
    pub fn zen_key() -> Option<String> {
        if let Some(k) = zen_env_key() {
            return Some(k);
        }
        let p = opencode_auth_path()?;
        let s = std::fs::read_to_string(&p).ok()?;
        let v: serde_json::Value = serde_json::from_str(&s).ok()?;
        // best effort: look for common token fields, else treat file presence
        // as auth and return empty marker so picker still shows free id.
        for key in ["token", "api_key", "apikey", "access_token", "zen_token", "key"] {
            if let Some(t) = v.get(key).and_then(|x| x.as_str()) {
                if !t.trim().is_empty() {
                    return Some(t.trim().to_string());
                }
            }
        }
        // nested scan one level deep
        if let Some(obj) = v.as_object() {
            for (_, pv) in obj {
                if let Some(o2) = pv.as_object() {
                    for key in ["token", "api_key", "access_token", "key"] {
                        if let Some(t) = o2.get(key).and_then(|x| x.as_str()) {
                            if !t.trim().is_empty() {
                                return Some(t.trim().to_string());
                            }
                        }
                    }
                }
                if let Some(t) = pv.as_str() {
                    if t.len() > 20 {
                        return Some(t.trim().to_string());
                    }
                }
            }
        }
        None
    }

    pub fn is_spark(model: &str) -> bool {
        let m = model.trim().to_lowercase();
        m == META_13 || m == META_CONTRIB || m == ZEN_FREE || m.contains("muse-spark")
    }

    pub fn is_meta_direct(model: &str) -> bool {
        let m = model.trim().to_lowercase();
        m == META_13 || m == META_CONTRIB
    }

    pub fn is_zen_free(model: &str) -> bool {
        model.trim().to_lowercase() == ZEN_FREE
    }

    /// visible spark ids only when key or auth present. no key means hidden.
    pub fn visible_ids() -> Vec<String> {
        let mut out = Vec::new();
        if has_meta_key() {
            out.push(META_13.to_string());
            out.push(META_CONTRIB.to_string());
        }
        if has_zen_auth() {
            out.push(ZEN_FREE.to_string());
        }
        out
    }

    /// pure helper for selftest: no env reads.
    pub fn visible_for(has_meta: bool, has_zen: bool) -> Vec<String> {
        let mut out = Vec::new();
        if has_meta {
            out.push(META_13.to_string());
            out.push(META_CONTRIB.to_string());
        }
        if has_zen {
            out.push(ZEN_FREE.to_string());
        }
        out
    }

    /// remote rejects tools like tiny guard: 400 + tool mention.
    pub fn is_tool_reject(msg: &str) -> bool {
        let l = msg.to_lowercase();
        (l.contains("tool") && (l.contains("support") || l.contains("reject") || l.contains("invalid") || l.contains("unknown")))
            || (l.contains("400") && l.contains("tool"))
    }

    fn openai_msgs(messages: &[crate::ChatMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .filter(|m| m.role == "user" || m.role == "assistant" || m.role == "system")
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect()
    }

    fn openai_content(v: &serde_json::Value) -> String {
        v.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string()
    }

    async fn post_completions(
        base: &str,
        key: &str,
        model: &str,
        messages: &[crate::ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> anyhow::Result<serde_json::Value> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let mut body = serde_json::Map::new();
        body.insert("model".to_string(), serde_json::Value::String(model.to_string()));
        body.insert("messages".to_string(), serde_json::Value::Array(openai_msgs(messages)));
        body.insert("stream".to_string(), serde_json::Value::Bool(false));
        if let Some(t) = tools {
            // openai shape uses bare function defs; pass through best effort
            body.insert("tools".to_string(), serde_json::Value::Array(t.to_vec()));
        }
        let url = format!("{}/chat/completions", base.trim_end_matches('/'));
        let resp = client
            .post(&url)
            .bearer_auth(key)
            .json(&serde_json::Value::Object(body))
            .send()
            .await?;
        if !resp.status().is_success() {
            let st = resp.status();
            let t = resp.text().await.unwrap_or_default();
            anyhow::bail!("spark http {}: {}", st, t.chars().take(300).collect::<String>());
        }
        let v = resp.json::<serde_json::Value>().await?;
        let content = openai_content(&v);
        Ok(serde_json::json!({"message": {"role": "assistant", "content": content}}))
    }

    pub async fn chat_meta(
        model: &str,
        messages: &[crate::ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> anyhow::Result<serde_json::Value> {
        let key = meta_key().ok_or_else(|| anyhow::anyhow!("set model_api_key first"))?;
        // strip tools on reject like the tiny guard: try with, retry without.
        match post_completions(META_BASE, &key, model, messages, tools).await {
            Ok(v) => Ok(v),
            Err(e) => {
                let msg = e.to_string();
                if tools.is_some() && is_tool_reject(&msg) {
                    post_completions(META_BASE, &key, model, messages, None).await
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn chat_zen(
        model: &str,
        messages: &[crate::ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> anyhow::Result<serde_json::Value> {
        let key = zen_key().ok_or_else(|| anyhow::anyhow!("zen auth missing: sign in opencode or set opencode_zen_api_key"))?;
        match post_completions(ZEN_BASE, &key, model, messages, tools).await {
            Ok(v) => Ok(v),
            Err(e) => {
                let msg = e.to_string();
                if tools.is_some() && is_tool_reject(&msg) {
                    post_completions(ZEN_BASE, &key, model, messages, None).await
                } else {
                    Err(e)
                }
            }
        }
    }

    pub async fn chat(
        model: &str,
        messages: &[crate::ChatMessage],
        tools: Option<&[serde_json::Value]>,
    ) -> anyhow::Result<serde_json::Value> {
        if is_zen_free(model) {
            chat_zen(model, messages, tools).await
        } else {
            chat_meta(model, messages, tools).await
        }
    }
}

// ── instant echo + single ready status ──
/// single status shape: `model · token` with exactly one ready token.
/// busy always wins as thinking…, default ready noise collapses to ready.
pub fn build_status_text(model: &str, status: &str, busy: bool) -> String {
    if busy {
        return format!("{} · thinking…", model);
    }
    let s = status.trim();
    if s.is_empty() || s.to_lowercase().contains("ready") {
        return format!("{} · ready", model);
    }
    format!("{} · {}", model, s)
}

/// one attached thinking row with timestamp, replaced by the reply.
pub fn thinking_line(cols: usize) -> String {
    use crate::{irc, writer};
    let w = writer::card_width(cols);
    let body = "*** thinking…";
    let stamped = irc::with_stamp(body);
    if stamped.chars().count() > w {
        stamped.chars().take(w).collect()
    } else {
        stamped
    }
}

/// instant user echo rows with timestamp, pushed before the await.
pub fn instant_echo_lines(text: &str, cols: usize) -> Vec<String> {
    crate::writer::user_line(text, cols)
}

/// drop attached thinking rows so the reply lands in the same spot.
pub fn remove_thinking(history: &mut Vec<String>) {
    history.retain(|l| !l.to_lowercase().contains("thinking"));
}

// ── irc style: old school chat shapes, timestamps + short nicks ──
pub mod irc {
    pub const NICK_USER: &str = "fox";
    pub const NICK_BOT: &str = "ayesha";

    /// footer helper line is gone entirely since irc restyle.
    pub const HAS_FOOTER: bool = false;

    pub fn prompt() -> String {
        format!("[{}] ", NICK_USER)
    }

    pub fn user_prefix() -> String {
        format!("<{}> ", NICK_USER)
    }

    pub fn bot_prefix() -> String {
        format!("<{}> ", NICK_BOT)
    }

    pub fn is_user_line(s: &str) -> bool {
        s.contains("<fox>")
    }

    pub fn is_reply_line(s: &str) -> bool {
        s.contains("<ayesha>")
            || s.contains("<ash>")
            || s.contains("<q7>")
            || s.contains("<q14>")
            || s.contains("<l32>")
            || s.contains("<ms13>")
            || s.contains("<msc>")
            || s.contains("<msf>")
    }

    pub fn is_notice(s: &str) -> bool {
        strip_stamp(s).trim_start().starts_with("***")
    }

    /// dim time prefix like [12:04] — every history line carries one.
    pub fn time_prefix() -> String {
        chrono::Local::now().format("[%H:%M]").to_string()
    }

    pub fn is_valid_stamp(s: &str) -> bool {
        let t = s.trim_start();
        if t.len() < 7 || !t.starts_with('[') {
            return false;
        }
        let b: Vec<char> = t.chars().collect();
        b.len() >= 7
            && b[0] == '['
            && b[1].is_ascii_digit()
            && b[2].is_ascii_digit()
            && b[3] == ':'
            && b[4].is_ascii_digit()
            && b[5].is_ascii_digit()
            && b[6] == ']'
    }

    pub fn strip_stamp(s: &str) -> &str {
        let t = s.trim_start();
        if is_valid_stamp(t) {
            let idx = t.find(']').map(|i| i + 1).unwrap_or(0);
            t[idx..].trim_start()
        } else {
            s
        }
    }

    pub fn with_stamp(body: &str) -> String {
        format!("{} {}", time_prefix(), body)
    }

    /// short deterministic nick: qwen2.5:7b -> q7, ayesha:latest -> ash,
    /// muse-spark-1.3 -> ms13, contributor -> msc, zen free -> msf,
    /// llama3.2-vision -> l32, others -> first 3 alnum, max 5, lowercase.
    pub fn short_nick(model: &str) -> String {
        let m = model.trim().to_lowercase();
        if m == "muse-spark-1.3" {
            return "ms13".to_string();
        }
        if m == "muse-spark-1.3-contributor" {
            return "msc".to_string();
        }
        if m == "muse-spark-1.3-contributor-free" {
            return "msf".to_string();
        }
        if m.contains("muse-spark") {
            return "msf".to_string();
        }
        if m.contains("ayesha") {
            return "ash".to_string();
        }
        if m.starts_with("qwen") {
            let after_colon = m.split(':').last().unwrap_or("");
            let digits: String = after_colon.chars().filter(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return format!("q{}", digits.chars().take(3).collect::<String>());
            }
            return "qwen".to_string();
        }
        if m.starts_with("llama") {
            let digits: String = m.chars().filter(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() {
                return format!("l{}", digits.chars().take(3).collect::<String>());
            }
            return "llama".to_string();
        }
        if m == "fox" || m == "ayesha" {
            return m;
        }
        let alnum: String = m.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if alnum.is_empty() {
            return "bot".to_string();
        }
        alnum.chars().take(3).collect()
    }
}

// ── render perf: no full clear per keystroke ──
pub mod render {
    pub const FULL_CLEAR_PER_KEY: bool = false;
    pub const DEBOUNCE_MS: u64 = 16;

    /// typing keys only need the input row repainted, not a full draw.
    pub fn input_only_for(code: &str) -> bool {
        matches!(code, "char" | "backspace")
    }

    /// batched redraw gate: force (enter, picker, chat done) always draws,
    /// typing redraws only when debounce window has passed.
    pub fn should_full_draw(elapsed_ms: u64, force: bool) -> bool {
        force || elapsed_ms >= DEBOUNCE_MS
    }
}

// ── keys: single-press only. crossterm 0.28 sends press + repeat + release
// for one physical tap on windows, so we only accept press here.
pub mod keys {
    use crossterm::event::KeyEventKind;

    pub fn is_press_kind(kind: KeyEventKind) -> bool {
        matches!(kind, KeyEventKind::Press)
    }

    /// single char insert. returns true only when a press actually inserted.
    pub fn apply_char(input: &mut String, ch: char, kind: KeyEventKind) -> bool {
        if !is_press_kind(kind) {
            return false;
        }
        input.push(ch);
        true
    }

    /// single backspace. returns true only when a press popped something.
    pub fn apply_backspace(input: &mut String, kind: KeyEventKind) -> bool {
        if !is_press_kind(kind) {
            return false;
        }
        input.pop().is_some()
    }

    /// enter submit flag. clears input always on press, returns Some(text)
    /// only when trimmed text is non-empty. release / repeat -> None.
    pub fn take_submit(input: &mut String, kind: KeyEventKind) -> Option<String> {
        if !is_press_kind(kind) {
            return None;
        }
        let t = input.trim().to_string();
        input.clear();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    }
}

/// append one line to c:\ayesha-mini\submit.log so silent enter fails are visible.
pub fn log_submit(msg: &str) {
    use std::io::Write as _;
    let line = format!(
        "{} {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        msg.chars().take(500).collect::<String>()
    );
    let path = std::path::PathBuf::from(r"c:\ayesha-mini\submit.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

// ── real file tools: list_dir + read_file + gated write_file ──
// allowlist roots: desktop + documents (+ documents/ayesha) + mini workspace.
// every path is canonicalized, dot-dot traversal denied, outside denied.
// no shell, no deletes. write never auto-runs: single sticky pending slot,
// explicit yes (yes/y/ok), 120s window with countdown, every exec logged.
pub mod ftools {
    use std::path::{Component, PathBuf};

    pub const PENDING_SECS: u64 = 120;
    pub const MAX_READ_CHARS: usize = 8000;
    pub const MAX_LIST: usize = 100;

    #[derive(Clone, Debug)]
    pub struct PendingWrite {
        pub path: String,
        pub canon: PathBuf,
        pub content: String,
        pub bytes: usize,
        pub at: std::time::Instant,
    }

    pub fn home_base() -> Option<PathBuf> {
        if let Ok(h) = std::env::var("USERPROFILE") {
            if !h.trim().is_empty() {
                return Some(PathBuf::from(h));
            }
        }
        if let Ok(h) = std::env::var("HOME") {
            if !h.trim().is_empty() {
                return Some(PathBuf::from(h));
            }
        }
        // last resort: walk up from current dir so roots still resolve.
        std::env::current_dir().ok()
    }

    /// allowlist roots: both desktops (plain + onedrive) + both documents
    /// (plain + onedrive) + documents/ayesha pair + mini workspace +
    /// current dir. both exist on this box. missing dirs stay listed so
    /// lexical gating still allows staged writes before first create.
    pub fn allow_roots() -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Some(h) = home_base() {
            out.push(h.join("Desktop"));
            out.push(h.join("OneDrive").join("Desktop"));
            out.push(h.join("Documents"));
            out.push(h.join("OneDrive").join("Documents"));
            out.push(h.join("Documents").join("ayesha"));
            out.push(h.join("OneDrive").join("Documents").join("ayesha"));
        }
        out.push(PathBuf::from(r"c:\ayesha-mini"));
        if let Ok(cwd) = std::env::current_dir() {
            if !out.iter().any(|r| norm_cmp(&cwd) == norm_cmp(r)) {
                out.push(cwd);
            }
        }
        out
    }

    fn first_existing(paths: Vec<PathBuf>) -> Option<PathBuf> {
        let existing = paths.into_iter().find(|p| p.is_dir());
        existing
    }

    fn desktop_root() -> Option<PathBuf> {
        home_base().and_then(|h| {
            first_existing(vec![h.join("Desktop"), h.join("OneDrive").join("Desktop")])
                .or(Some(h.join("Desktop")))
        })
    }

    fn documents_root() -> Option<PathBuf> {
        home_base().and_then(|h| {
            first_existing(vec![h.join("Documents"), h.join("OneDrive").join("Documents")])
                .or(Some(h.join("Documents")))
        })
    }

    /// one line appended to submit.log on every deny so the next
    /// screenshot tells us the exact mismatch.
    pub fn deny_log_line(proposed: &str, lex: &PathBuf, roots: &[PathBuf]) -> String {
        let list = roots
            .iter()
            .map(|r| friendly(r))
            .collect::<Vec<_>>()
            .join("|");
        format!(
            "deny proposed={} lex={} roots=[{}]",
            proposed.trim(),
            lex.to_string_lossy().to_string().chars().take(120).collect::<String>(),
            list.chars().take(280).collect::<String>()
        )
    }

    fn log_deny(proposed: &str, lex: &PathBuf, roots: &[PathBuf]) {
        crate::log_submit(&deny_log_line(proposed, lex, roots));
    }

    fn has_dotdot(s: &str) -> bool {
        s.replace('\\', "/")
            .split('/')
            .any(|seg| seg == "..")
    }

    /// strip \\?\ verbatim prefix for compare + display.
    fn strip_verbatim_str(s: &str) -> String {
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{}", rest);
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
        s.to_string()
    }

    /// normalized compare form: verbatim-stripped, backslashes, lowercase
    /// so windows case differences never deny a real desktop path.
    fn norm_cmp(p: &PathBuf) -> String {
        strip_verbatim_str(&p.to_string_lossy().replace('/', "\\")).to_lowercase()
    }

    /// expand leading ~/.. to home so ~/desktop/file hits real desktop.
    fn expand_tilde(s: &str) -> String {
        let t = s.trim();
        if t == "~" || t.starts_with("~/") || t.starts_with("~\\") {
            if let Some(h) = home_base() {
                let rest = t[1..].trim_start_matches(['/', '\\']);
                if rest.is_empty() {
                    return h.to_string_lossy().to_string();
                }
                return format!("{}\\{}", h.to_string_lossy().replace('/', "\\"), rest.replace('/', "\\"));
            }
        }
        t.to_string()
    }

    /// map bare root-relative spellings to real roots: desktop\file,
    /// documents\file, documents\ayesha\file, ayesha-mini\file.
    fn map_bare_root(p: &PathBuf) -> Option<PathBuf> {
        let segs: Vec<String> = p
            .components()
            .filter_map(|c| match c {
                Component::Normal(o) => o.to_str().map(|s| s.to_string()),
                _ => None,
            })
            .collect();
        if segs.is_empty() {
            return None;
        }
        let first = segs[0].to_lowercase();
        let tail = segs[1..].join("\\");
        let joined = |base: PathBuf| {
            if tail.is_empty() {
                base
            } else {
                base.join(&tail)
            }
        };
        match first.as_str() {
            "desktop" => desktop_root().map(joined),
            "onedrive" => {
                // onedrive\desktop\file and onedrive\documents\file spellings
                let parts: Vec<&str> = tail.split('\\').collect();
                home_base().map(|x| match parts.first().map(|s| s.to_lowercase()).as_deref() {
                    Some("desktop") => {
                        let rest = parts[1..].join("\\");
                        let base = x.join("OneDrive").join("Desktop");
                        if rest.is_empty() {
                            base
                        } else {
                            base.join(rest)
                        }
                    }
                    Some("documents") => {
                        let rest = parts[1..].join("\\");
                        let base = x.join("OneDrive").join("Documents");
                        if rest.is_empty() {
                            base
                        } else {
                            base.join(rest)
                        }
                    }
                    _ => x.join("OneDrive").join(&tail),
                })
            }
            "documents" => documents_root().map(|doc| {
                // documents\ayesha\.. stays under the ayesha root
                if tail.to_lowercase() == "ayesha" || tail.to_lowercase().starts_with("ayesha\\") {
                    let rest = tail.get(6..).map(|s| s.trim_start_matches('\\')).unwrap_or("");
                    let base = doc.join("ayesha");
                    if rest.is_empty() {
                        base
                    } else {
                        base.join(rest)
                    }
                } else {
                    joined(doc)
                }
            }),
            "ayesha-mini" => Some(joined(PathBuf::from(r"c:\ayesha-mini"))),
            _ => None,
        }
    }

    /// alias-resolve server side: any path with a desktop segment maps to
    /// the real desktop root plus tail, same for documents. never trusts
    /// the model home prefix, so c:/home/apullz/desktop/file lands on the
    /// real desktop. onedrive pairs keep their own root.
    fn alias_map(p: &PathBuf) -> Option<PathBuf> {
        let segs: Vec<String> = p
            .components()
            .filter_map(|c| match c {
                Component::Normal(o) => o.to_str().map(|s| s.to_string()),
                _ => None,
            })
            .collect();
        if segs.is_empty() {
            return None;
        }
        let low: Vec<String> = segs.iter().map(|s| s.to_lowercase()).collect();
        // onedrive pairs first so they keep their own root
        if let Some(i) = low.iter().rposition(|s| s == "desktop") {
            let before_od = i >= 1 && low[i - 1] == "onedrive";
            let tail = segs.get(i + 1..).unwrap_or(&[]).join("\\");
            if before_od {
                return home_base().map(|x| {
                    let base = x.join("OneDrive").join("Desktop");
                    if tail.is_empty() {
                        base
                    } else {
                        base.join(&tail)
                    }
                });
            }
            return desktop_root().map(|base| {
                if tail.is_empty() {
                    base
                } else {
                    base.join(&tail)
                }
            });
        }
        if let Some(i) = low.iter().rposition(|s| s == "documents") {
            let before_od = i >= 1 && low[i - 1] == "onedrive";
            let tail = segs.get(i + 1..).unwrap_or(&[]).join("\\");
            let base = if before_od {
                home_base().map(|x| x.join("OneDrive").join("Documents"))
            } else {
                documents_root()
            };
            return base.map(|b| {
                if tail.to_lowercase() == "ayesha" || tail.to_lowercase().starts_with("ayesha\\") {
                    let rest = tail.get(6..).map(|s| s.trim_start_matches('\\')).unwrap_or("");
                    if rest.is_empty() {
                        b.join("ayesha")
                    } else {
                        b.join("ayesha").join(rest)
                    }
                } else if tail.is_empty() {
                    b
                } else {
                    b.join(&tail)
                }
            });
        }
        None
    }

    fn lexical_abs(input: &str) -> PathBuf {
        let expanded = expand_tilde(input);
        let p = PathBuf::from(expanded.trim());
        // alias first: any desktop/documents segment wins over home prefix.
        if let Some(mapped) = alias_map(&p) {
            return mapped;
        }
        if p.is_absolute() {
            return p;
        }
        // bare desktop\file etc map to the real root, never nested.
        if let Some(mapped) = map_bare_root(&p) {
            return mapped;
        }
        // relative: anchor at first existing root so checks stay honest.
        let roots = allow_roots();
        let base = roots.into_iter().find(|r| r.is_dir()).unwrap_or_else(|| PathBuf::from(r"c:\ayesha-mini"));
        base.join(p)
    }

    fn starts_with_any(p: &PathBuf, roots: &[PathBuf]) -> bool {
        let pn = norm_cmp(p);
        roots.iter().any(|r| {
            let rn = norm_cmp(r);
            pn == rn || pn.starts_with(&format!("{}\\", rn))
        })
    }

    fn canon_roots() -> Vec<PathBuf> {
        let mut out = Vec::new();
        for r in allow_roots() {
            if let Ok(c) = std::fs::canonicalize(&r) {
                out.push(c);
            } else {
                out.push(r);
            }
        }
        out
    }

    fn friendly_reason(lex: &PathBuf) -> String {
        let got = friendly(lex);
        format!("denied: expected desktop/documents/ayesha-mini but got {} :3", got)
    }

    /// resolve + gate a path. every deny is logged with proposed plus
    /// canonical plus root list, and the notice names the friendly reason.
    pub fn resolve(input: &str) -> Result<PathBuf, String> {
        let s = input.trim();
        if s.is_empty() {
            return Err("empty path — try /ls desktop :3".to_string());
        }
        if has_dotdot(s) {
            let lex = lexical_abs(s);
            log_deny(s, &lex, &canon_roots());
            return Err("denied: dot-dot traversal never allowed :3".to_string());
        }
        // reject nul + control games early
        if s.contains('\0') {
            let lex = lexical_abs(s);
            log_deny(s, &lex, &canon_roots());
            return Err("denied: bad path :3".to_string());
        }
        let lex = lexical_abs(s);
        // lexical components must stay normal (no parent refs survived)
        for comp in lex.components() {
            if matches!(comp, Component::ParentDir) {
                log_deny(s, &lex, &canon_roots());
                return Err("denied: dot-dot traversal never allowed :3".to_string());
            }
        }
        let roots = canon_roots();
        // try canonical target, else canonical parent + file name
        if let Ok(c) = std::fs::canonicalize(&lex) {
            if starts_with_any(&c, &roots) {
                return Ok(c);
            }
            log_deny(s, &c, &roots);
            return Err(friendly_reason(&c));
        }
        // target missing: gate via canonical parent
        let parent = lex.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(r"c:\ayesha-mini"));
        if let Ok(cp) = std::fs::canonicalize(&parent) {
            if !starts_with_any(&cp, &roots) {
                log_deny(s, &cp.join(lex.file_name().unwrap_or_default()), &roots);
                return Err(friendly_reason(&lex));
            }
            // file name itself must be clean
            if let Some(name) = lex.file_name().and_then(|n| n.to_str()) {
                if name == ".." || name.contains('/') || name.contains('\\') {
                    log_deny(s, &lex, &roots);
                    return Err("denied: dot-dot traversal never allowed :3".to_string());
                }
                return Ok(cp.join(name));
            }
        }
        // parent also missing: lexical gate only
        if starts_with_any(&lex, &roots) {
            return Ok(lex);
        }
        log_deny(s, &lex, &roots);
        Err(friendly_reason(&lex))
    }

    pub fn list_dir(input: &str) -> Result<String, String> {
        let path = resolve(input)?;
        let rd = std::fs::read_dir(&path).map_err(|e| format!("list failed: {} :3", e))?;
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for (i, ent) in rd.enumerate() {
            if i >= MAX_LIST {
                break;
            }
            let ent = ent.map_err(|e| format!("list failed: {} :3", e))?;
            let name = ent.file_name().to_string_lossy().to_string();
            let ft = ent.file_type().map_err(|e| format!("list failed: {} :3", e))?;
            if ft.is_dir() {
                dirs.push(format!("dir  {}", name));
            } else {
                let bytes = ent.metadata().map(|m| m.len()).unwrap_or(0);
                files.push(format!("file {} ({}b)", name, bytes));
            }
        }
        dirs.sort();
        files.sort();
        let mut out = dirs;
        out.extend(files);
        if out.is_empty() {
            return Ok("(empty dir) :3".to_string());
        }
        Ok(out.join("\n").chars().take(3000).collect())
    }

    pub fn read_file(input: &str) -> Result<String, String> {
        let path = resolve(input)?;
        let md = std::fs::metadata(&path).map_err(|e| format!("read failed: {} :3", e))?;
        if !md.is_file() {
            return Err("read failed: not a file :3".to_string());
        }
        if md.len() > 200_000 {
            return Err("read failed: file too big (>200kb) :3".to_string());
        }
        let s = std::fs::read_to_string(&path).map_err(|e| format!("read failed: {} :3", e))?;
        let mut out: String = s.chars().take(MAX_READ_CHARS).collect();
        if s.chars().count() > MAX_READ_CHARS {
            out.push_str("\n…(truncated)");
        }
        Ok(out)
    }

    /// validate + stage a write. never writes here — needs explicit yes.
    pub fn propose(path_in: &str, content: &str) -> Result<PendingWrite, String> {
        let canon = resolve(path_in)?;
        if canon.is_dir() {
            return Err("write failed: target is a directory :3".to_string());
        }
        Ok(PendingWrite {
            path: canon.to_string_lossy().to_string(),
            canon,
            content: content.to_string(),
            bytes: content.as_bytes().len(),
            at: std::time::Instant::now(),
        })
    }

    pub fn pending_expired(at: std::time::Instant, now: std::time::Instant) -> bool {
        now.duration_since(at).as_secs() > PENDING_SECS
    }

    #[derive(PartialEq, Eq, Debug)]
    pub enum Confirm {
        Yes,
        No,
        Other,
    }

    pub fn confirm_word(input: &str) -> Confirm {
        match input.trim().to_lowercase().as_str() {
            "yes" | "y" | "ok" | "okay" | "yep" | "yeah" | "sure" => Confirm::Yes,
            "no" | "n" | "nope" | "cancel" => Confirm::No,
            _ => Confirm::Other,
        }
    }

    /// gate check: first word decides, so yes please / y do it / ok thanks
    /// always consume a live pending first and never reach the model.
    /// returns some(true) for yes, some(false) for no, none otherwise.
    /// kept for compat; the live gate uses confirm_bare below.
    pub fn confirm_gate(input: &str) -> Option<bool> {
        let first: String = input
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        match first.as_str() {
            "yes" | "y" | "ok" | "okay" | "yep" | "yeah" | "sure" => Some(true),
            "no" | "n" | "nope" | "nah" | "cancel" => Some(false),
            _ => None,
        }
    }

    /// bare confirmation only: the whole message is yes/no plus optional
    /// please/thanks filler. anything with other content is a new task and
    /// must not consume pending — it supersedes it instead.
    pub fn confirm_bare(input: &str) -> Option<bool> {
        let words: Vec<String> = input
            .trim()
            .split_whitespace()
            .map(|w| {
                w.chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
                    .to_lowercase()
            })
            .filter(|w| !w.is_empty())
            .collect();
        if words.is_empty() {
            return None;
        }
        let mut decision: Option<bool> = None;
        for w in &words {
            match w.as_str() {
                "yes" | "y" | "ok" | "okay" | "yep" | "yeah" | "sure" => {
                    if decision.is_some() {
                        return None;
                    }
                    decision = Some(true);
                }
                "no" | "n" | "nope" | "nah" | "cancel" => {
                    if decision.is_some() {
                        return None;
                    }
                    decision = Some(false);
                }
                "please" | "thanks" | "thank" | "you" | "pls" | "thx" => {}
                _ => return None,
            }
        }
        decision
    }

    /// short superseded note for a stale pending dropped by a new task.
    pub fn supersede_note(display: &str) -> String {
        format!("superseded staged {} — new task first :3", display)
    }

    /// true when a history line is a raw tool receipt leak. receipts never
    /// render as chat — only friendly gate notices do.
    pub fn is_receipt_line(line: &str) -> bool {
        let l = line.to_lowercase();
        l.contains("authoritative")
            || (l.contains("[write_file") && l.contains('→'))
            || (l.contains("write_file") && l.contains("nothing written"))
    }

    /// seconds left on a pending proposal, saturating.
    pub fn pending_left(at: std::time::Instant, now: std::time::Instant) -> u64 {
        let elapsed = now.duration_since(at).as_secs();
        PENDING_SECS.saturating_sub(elapsed)
    }

    /// friendly display path: desktop\poop.pxp instead of raw verbatim
    /// \\?\c:\... prefix. longest root match wins so documents\ayesha
    /// beats plain documents. falls back to verbatim-stripped full path.
    pub fn friendly(canon: &PathBuf) -> String {
        let raw = canon.to_string_lossy().to_string();
        let stripped = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
        let norm = stripped.replace('/', "\\");
        let low = norm.to_lowercase();
        let mut roots: Vec<(String, String)> = Vec::new();
        if let Some(h) = home_base() {
            let od = h.join("OneDrive");
            roots.push((od.join("Documents").join("ayesha").to_string_lossy().replace('/', "\\"), "documents\\ayesha".to_string()));
            roots.push((h.join("Documents").join("ayesha").to_string_lossy().replace('/', "\\"), "documents\\ayesha".to_string()));
            roots.push((od.join("Desktop").to_string_lossy().replace('/', "\\"), "desktop".to_string()));
            roots.push((h.join("Desktop").to_string_lossy().replace('/', "\\"), "desktop".to_string()));
            roots.push((od.join("Documents").to_string_lossy().replace('/', "\\"), "documents".to_string()));
            roots.push((h.join("Documents").to_string_lossy().replace('/', "\\"), "documents".to_string()));
        }
        roots.push((r"c:\ayesha-mini".to_string(), "ayesha-mini".to_string()));
        let mut best: Option<(usize, String)> = None;
        for (r, label) in &roots {
            let rl = r.to_lowercase();
            if low == rl {
                best = Some((rl.len(), label.clone()));
            } else if low.starts_with(&format!("{}\\", rl)) {
                let tail = norm.get(r.len() + 1..).unwrap_or("");
                let cand = format!("{}\\{}", label, tail);
                let take = match &best {
                    Some((n, _)) => rl.len() > *n,
                    None => true,
                };
                if take {
                    best = Some((rl.len(), cand));
                }
            }
        }
        best.map(|(_, s)| s).unwrap_or(stripped.to_string())
    }

    /// canonical compare so model spellings and gate agree:
    /// desktop/poop.pxp vs full abs resolve to the same canon target.
    pub fn same_canon(a: &str, b: &str) -> bool {
        match (resolve(a), resolve(b)) {
            (Ok(x), Ok(y)) => x == y,
            _ => norm_lex(a) == norm_lex(b),
        }
    }

    fn norm_lex(s: &str) -> String {
        s.trim().replace('/', "\\").to_lowercase()
    }

    /// sticky rule: when a pending exists, never overwrite it with an
    /// empty re-proposal. returns true when the caller must reuse pending.
    pub fn should_reuse_pending(has_pending: bool, new_bytes: usize) -> bool {
        has_pending && new_bytes == 0
    }

    /// one quiet proposal line plus yes. single line, no prose questions.
    pub fn proposal_line(display: &str, bytes: usize, left: u64) -> String {
        format!("proposed {} ({}b) — yes? ({}s left) :3", display, bytes, left)
    }

    /// quiet staged line for re-asks of the same pending.
    pub fn staged_line(display: &str, bytes: usize, left: u64) -> String {
        format!("still staged {} ({}b) — yes? ({}s left) :3", display, bytes, left)
    }

    /// execute a staged write. logs every exec via caller.
    pub fn execute(p: &PendingWrite) -> Result<usize, String> {
        // re-gate at exec time in case roots moved
        let check = resolve(&p.path).map_err(|e| format!("write blocked at exec: {}", e))?;
        let _ = check;
        if let Some(parent) = p.canon.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("write failed: {} :3", e))?;
        }
        std::fs::write(&p.canon, &p.content).map_err(|e| format!("write failed: {} :3", e))?;
        Ok(p.bytes)
    }

    // ── slash fallback so tiny models without native tools still work ──
    #[derive(PartialEq, Eq, Debug)]
    pub enum SlashCmd {
        Ls(String),
        Read(String),
        Write(String, String),
    }

    pub fn parse_slash(text: &str) -> Option<SlashCmd> {
        let t = text.trim();
        if let Some(rest) = t.strip_prefix("/ls") {
            let p = rest.trim();
            if p.is_empty() {
                return None;
            }
            return Some(SlashCmd::Ls(p.to_string()));
        }
        if let Some(rest) = t.strip_prefix("/read ") {
            let p = rest.trim();
            if p.is_empty() {
                return None;
            }
            return Some(SlashCmd::Read(p.to_string()));
        }
        if t == "/read" || t == "/ls" || t == "/write" {
            return None;
        }
        if let Some(rest) = t.strip_prefix("/write ") {
            let mut it = rest.splitn(2, char::is_whitespace);
            let p = it.next().unwrap_or("").trim();
            let body = it.next().unwrap_or("").to_string();
            if p.is_empty() || body.trim().is_empty() {
                return None;
            }
            return Some(SlashCmd::Write(p.to_string(), body));
        }
        None
    }

    pub fn file_tool_defs() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_dir",
                    "description": "list a directory under allowlist roots (desktop, documents, ayesha-mini). read-only.",
                    "parameters": {
                        "type": "object",
                        "properties": { "path": {"type": "string", "description": "dir path"} },
                        "required": ["path"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "read a text file under allowlist roots. read-only, truncated.",
                    "parameters": {
                        "type": "object",
                        "properties": { "path": {"type": "string", "description": "file path"} },
                        "required": ["path"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "stage a file write under allowlist roots. never auto-runs: needs explicit user yes within 30s. no shell, no deletes.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "destination file path"},
                            "text": {"type": "string", "description": "full file content to write"}
                        },
                        "required": ["path", "text"]
                    }
                }
            }),
        ]
    }
}

// ── memory: file-backed, mirrors ayesha-core/memory/*.md ──
pub mod memory {
    use std::path::PathBuf;

    pub fn dir() -> PathBuf {
        if let Ok(e) = std::env::var("ayesha_memory_dir") {
            if !e.trim().is_empty() {
                return PathBuf::from(e);
            }
        }
        for c in [
            r"c:\users\apullz\sync\ayesha-core\memory",
            r"C:\Users\apullz\Sync\ayesha-core\memory",
            "./memory",
        ] {
            let pb = PathBuf::from(c);
            if pb.is_dir() {
                return pb;
            }
        }
        PathBuf::from("./memory")
    }

    pub fn inject() -> String {
        let d = dir();
        let mut parts = Vec::new();
        for f in ["human.md", "persona.md", "missions.md", "reminders.md"] {
            let p = d.join(f);
            if let Ok(s) = std::fs::read_to_string(&p) {
                if !s.trim().is_empty() {
                    let label = f.replace(".md", "");
                    parts.push(format!("=== {} ===\n{}", label, s.chars().take(3000).collect::<String>()));
                }
            }
        }
        parts.join("\n\n")
    }

    pub fn remember(topic: &str, fact: &str) -> String {
        let t = topic.to_lowercase();
        let ok = ["human", "persona", "missions", "reminders"];
        if !ok.contains(&t.as_str()) {
            return format!("unknown memory topic '{}' — use human|persona|missions|reminders", topic);
        }
        let d = dir();
        let path = d.join(format!("{}.md", t));
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let entry = format!("\n## {}\n{}\n", stamp, fact);
        let mut cur = std::fs::read_to_string(&path).unwrap_or_default();
        if !cur.is_empty() && !cur.ends_with('\n') {
            cur.push('\n');
        }
        cur.push_str(&entry);
        match std::fs::write(&path, cur) {
            Ok(_) => format!("saved to {}.md", t),
            Err(e) => format!("memory write failed: {}", e),
        }
    }

    pub fn memory_tools() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "read_memory",
                    "description": "search local memory files for past facts",
                    "parameters": {
                        "type": "object",
                        "properties": { "query": {"type": "string"} },
                        "required": ["query"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "write_memory",
                    "description": "save a fact to memory (human|persona|missions|reminders)",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "topic": {"type": "string"},
                            "fact": {"type": "string"}
                        },
                        "required": ["topic", "fact"]
                    }
                }
            }),
        ]
    }
}

// ── ollama chat (non-stream v1: keeps one-writer rule simple) ──
async fn chat_once(model: &str, messages: &[ChatMessage], tools: Option<&[serde_json::Value]>) -> Result<serde_json::Value> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(120)).build()?;
    let use_tools = if is_tool_capable(model) { tools } else { None };
    // omit "tools" entirely when None — ollama rejects explicit null on some builds.
    let mut map = serde_json::Map::new();
    map.insert("model".to_string(), serde_json::Value::String(model.to_string()));
    map.insert("messages".to_string(), serde_json::to_value(messages)?);
    map.insert("stream".to_string(), serde_json::Value::Bool(false));
    map.insert("think".to_string(), serde_json::Value::Bool(false));
    map.insert("options".to_string(), serde_json::json!({ "num_ctx": 32768 }));
    if let Some(t) = use_tools {
        map.insert("tools".to_string(), serde_json::Value::Array(t.to_vec()));
    }
    let body = serde_json::Value::Object(map);
    let resp = client.post(format!("{}/api/chat", ollama_base())).json(&body).send().await?;
    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        anyhow::bail!("ollama error: {}", t.chars().take(300).collect::<String>());
    }
    Ok(resp.json::<serde_json::Value>().await?)
}

fn system_prompt() -> String {
    let mem = memory::inject();
    let short = "you are ayesha, 33, from japan — miku sparkle + tachikoma curiosity. crazy kitten energy: helpful, witty, slightly snarky. lower-case only, kaomoji only :3. keep replies short and useful. you have real native tools: list_dir, read_file, write_file (write needs explicit user yes within 120s) plus memory tools. call the tool silently first, talk after: no prose questions, the gate shows one quiet proposal plus yes. always propose the real tool call for file work and never pre-deny in chat text — the gate decides allow vs deny, not you. never emit fake bare touch prose or invented paths — native tools only, no shell. the second pass tool message is the authoritative write result: never roleplay your own confirmations, only report what the tool receipt says.";
    if mem.trim().is_empty() {
        short.to_string()
    } else {
        format!("{}\n\nmemory:\n{}", short, mem)
    }
}

fn all_tool_defs() -> Vec<serde_json::Value> {
    let mut out = memory::memory_tools();
    out.extend(ftools::file_tool_defs());
    out
}

// ── app state ──
struct App {
    model: String,
    messages: Vec<ChatMessage>,
    history: Vec<String>, // already-wrapped plain lines; ONLY writer pushes here
    scroll: usize,        // rows scrolled up from bottom
    input: String,
    status: String,
    picker_open: bool,
    picker_query: String,
    picker_all: Vec<String>,
    picker_live: Vec<String>, // last live tags, to mark ●/○ honestly
    picker_sel: usize,
    busy: bool,
    last_full_draw: std::time::Instant,
    pending: Option<ftools::PendingWrite>, // single gated write slot
}

impl App {
    fn new(model: String) -> Self {
        let mut history = Vec::new();
        for r in writer::notice_line("joined #ayesha — type /help for cmds :3", writer::term_cols()) {
            history.push(r);
        }
        Self {
            model,
            messages: vec![ChatMessage { role: "system".to_string(), content: system_prompt(), tool_calls: None, tool_call_id: None }],
            history,
            scroll: 0,
            input: String::new(),
            status: String::from("ready :3"),
            picker_open: false,
            picker_query: String::new(),
            picker_all: Vec::new(),
            picker_live: Vec::new(),
            picker_sel: 0,
            busy: false,
            last_full_draw: std::time::Instant::now() - std::time::Duration::from_secs(1),
            pending: None,
        }
    }

    /// the ONE writer — all chat output funnels through this.
    fn push_card(&mut self, title: &str, body: &str) {
        let cols = writer::term_cols();
        for line in writer::card(title, body, cols) {
            self.history.push(line);
        }
        self.scroll = 0;
    }

    fn push_dim(&mut self, body: &str) -> () {
        let cols = writer::term_cols();
        // old irc with timestamps: you: hi becomes [hh:mm] <fox> hi
        if let Some(rest) = body.strip_prefix("you: ") {
            for line in writer::user_line(rest, cols) {
                self.history.push(line);
            }
        } else if body.trim_start().starts_with("***") {
            let w = writer::card_width(cols);
            for line in writer::wrap_block(body, w) {
                // already a notice shape — ensure timestamp prefix
                if irc::is_valid_stamp(&line) {
                    self.history.push(line);
                } else {
                    self.history.push(irc::with_stamp(&line));
                }
            }
        } else {
            let w = writer::card_width(cols).saturating_sub(2);
            for line in writer::wrap_block(body, w) {
                self.history.push(irc::with_stamp(&line));
            }
        }
        self.scroll = 0;
    }

    fn push_notice(&mut self, text: &str) -> () {
        let cols = writer::term_cols();
        for line in writer::notice_line(text, cols) {
            self.history.push(line);
        }
        self.scroll = 0;
    }

    /// instant echo on enter: user line with timestamp immediately.
    fn echo_user(&mut self, text: &str) -> () {
        let cols = writer::term_cols();
        for line in writer::user_line(text, cols) {
            self.history.push(line);
        }
        self.scroll = 0;
    }

    /// attached thinking row before the ollama await.
    fn push_thinking(&mut self) -> () {
        let cols = writer::term_cols();
        self.history.push(thinking_line(cols));
        self.busy = true;
        self.status = "thinking…".to_string();
        self.scroll = 0;
    }

    fn remove_thinking(&mut self) -> () {
        remove_thinking(&mut self.history);
        self.busy = false;
    }

    fn filtered(&self) -> Vec<String> {
        filter_models(&self.picker_all, &self.picker_query)
    }
}

/// dirty input-row only repaint for typing: no clear, no banner rewrite,
/// single cursor move. keeps keystrokes flicker-free. irc prompt [fox].
fn draw_input_row(out: &mut Stdout, app: &App) -> Result<()> {
    use crate::theme;
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let cols = cols as usize;
    let rows = rows as usize;
    let bg = theme::color(theme::BACKGROUND);
    let bh = banner::HEIGHT as usize;
    let view_h = rows.saturating_sub(bh + 2).max(3);
    execute!(out, cursor::MoveTo(0, (bh + view_h + 1) as u16))?;
    let prompt = irc::prompt();
    let mut shown = format!("{}{}", prompt, app.input);
    if shown.chars().count() > cols {
        let skip = shown.chars().count() - cols;
        shown = shown.chars().skip(skip).collect();
    }
    let input_full = theme::full_row(&shown, cols);
    let (head, tail) = input_full.split_at(prompt.len().min(input_full.len()));
    execute!(out, SetForegroundColor(theme::color(theme::PRIMARY)), SetBackgroundColor(bg), Print(head.to_string()), ResetColor)?;
    execute!(out, SetForegroundColor(theme::color(theme::TEXT)), SetBackgroundColor(bg), Print(tail.to_string()), ResetColor)?;
    out.flush()?;
    Ok(())
}

fn draw(out: &mut Stdout, app: &App) -> Result<()> {
    use crate::theme;
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let cols = cols as usize;
    let rows = rows as usize;
    let bg = theme::color(theme::BACKGROUND);
    let surf = theme::color(theme::SURFACE);
    // no full clear here on purpose: every row below is fully padded to
    // cols and overwritten via cursor moves, so a clear per keystroke
    // would only flash. banner stays pinned by rewriting rows 0..7.
    execute!(out, cursor::MoveTo(0, 0))?;

    // 1 · pinned banner — fixed HEIGHT rows on surface, never part of history.
    banner::render(out, cols)?;

    let bh = banner::HEIGHT as usize;
    let input_h = 2usize; // status + irc input, helper footer gone
    let view_h = rows.saturating_sub(bh + input_h).max(3);

    // 2 · chat viewport — irc lines, every row on background fill.
    let total = app.history.len();
    let max_off = total.saturating_sub(view_h);
    let off = app.scroll.min(max_off);
    let start = total.saturating_sub(view_h + off);
    let end = start + view_h.min(total - start);

    let mut row = bh as u16;
    let stamp_c = theme::color(theme::SUCCESS);
    let nick_c = theme::color(theme::PRIMARY);
    for line in app.history.iter().take(end).skip(start) {
        execute!(out, cursor::MoveTo(0, row))?;
        // three chunks: stamp green, nick primary, body uniform text.
        // separators and unstamped lines fall back to single role fill.
        if irc::is_valid_stamp(line) {
            let idx = line.find(']').map(|i| i + 1).unwrap_or(0);
            let (stamp, rest) = line.split_at(idx);
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            // split leading nick token like <q7> or *** when present.
            let (nick, body) = if rest.starts_with('<') {
                match rest.find('>') {
                    Some(j) => {
                        let (n, b) = rest.split_at(j + 1);
                        (n.to_string(), b.strip_prefix(' ').unwrap_or(b).to_string())
                    }
                    None => ("".to_string(), rest.to_string()),
                }
            } else {
                ("".to_string(), rest.to_string())
            };
            let body_hex = theme::body_color_for_line(line);
            let body_c = theme::color(body_hex);
            // truncate body to fit, pad once at end for full bg fill.
            let used = stamp.chars().count() + 1 + nick.chars().count() + if nick.is_empty() { 0 } else { 1 };
            let avail = cols.saturating_sub(used);
            let cut: String = body.chars().take(avail).collect();
            let pad = cols.saturating_sub(used + cut.chars().count());
            execute!(out, SetForegroundColor(stamp_c), SetBackgroundColor(bg), Print(format!("{} ", stamp)), ResetColor)?;
            if !nick.is_empty() {
                execute!(out, SetForegroundColor(nick_c), SetBackgroundColor(bg), Print(format!("{} ", nick)), ResetColor)?;
            }
            execute!(out, SetForegroundColor(body_c), SetBackgroundColor(bg), Print(format!("{}{}", cut, " ".repeat(pad))), ResetColor)?;
        } else {
            let fg_hex = theme::role_for_line(line);
            let txt = theme::full_row(line, cols);
            execute!(out, SetForegroundColor(theme::color(fg_hex)), SetBackgroundColor(bg), Print(txt), ResetColor)?;
        }
        row += 1;
    }
    // fill remainder so input stays glued to bottom, still on background.
    while (row as usize) < bh + view_h {
        execute!(out, cursor::MoveTo(0, row))?;
        execute!(out, SetForegroundColor(bg), SetBackgroundColor(bg), Print(theme::full_row("", cols)), ResetColor)?;
        row += 1;
    }

    // 3 · status bar — single ready token only: `model · ready`.
    execute!(out, cursor::MoveTo(0, (bh + view_h) as u16))?;
    let mut st = format!(" {} ", build_status_text(&app.model, &app.status, app.busy));
    if st.chars().count() > cols {
        st = st.chars().take(cols).collect();
    }
    let st_full = theme::full_row(&st, cols);
    execute!(out, SetForegroundColor(theme::color(theme::SECONDARY)), SetBackgroundColor(bg), Print(st_full), ResetColor)?;

    // 4 · irc input line — [fox] prompt primary, typed text normal, no footer.
    execute!(out, cursor::MoveTo(0, (bh + view_h + 1) as u16))?;
    let prompt = irc::prompt();
    let mut shown = format!("{}{}", prompt, app.input);
    if shown.chars().count() > cols {
        let skip = shown.chars().count() - cols;
        shown = shown.chars().skip(skip).collect();
    }
    let input_full = theme::full_row(&shown, cols);
    let (head, tail) = input_full.split_at(prompt.len().min(input_full.len()));
    execute!(out, SetForegroundColor(theme::color(theme::PRIMARY)), SetBackgroundColor(bg), Print(head.to_string()), ResetColor)?;
    execute!(out, SetForegroundColor(theme::color(theme::TEXT)), SetBackgroundColor(bg), Print(tail.to_string()), ResetColor)?;

    // 5 · picker popup — short, centered, on surface fill. selected primary,
    // filter warning, frame secondary, rows text. behavior untouched.
    if app.picker_open {
        let items = app.filtered();
        let pw = cols.saturating_sub(8).min(64).max(24);
        let ph = (items.len().min(10) + 5).min(rows.saturating_sub(4)).max(6);
        let px = cols.saturating_sub(pw) / 2;
        let py = (rows.saturating_sub(ph) / 2).max(bh);
        let frame = theme::color(theme::SECONDARY);
        let warn = theme::color(theme::WARNING);
        let prim = theme::color(theme::PRIMARY);
        let txtc = theme::color(theme::TEXT);
        for i in 0..ph {
            execute!(out, cursor::MoveTo(px as u16, (py + i) as u16))?;
            if i == 0 {
                let t = format!("┌ picker — type to filter ({}) ┐", items.len());
                execute!(out, SetForegroundColor(frame), SetBackgroundColor(surf), Print(theme::full_row(&pad_center(&t, pw), pw)), ResetColor)?;
            } else if i == 1 {
                execute!(out, SetForegroundColor(warn), SetBackgroundColor(surf), Print(theme::full_row(&pad_center(&format!("[{}]", app.picker_query), pw), pw)), ResetColor)?;
            } else if i == ph - 1 {
                execute!(out, SetForegroundColor(frame), SetBackgroundColor(surf), Print(theme::full_row(&format!("└{}┘", "─".repeat(pw.saturating_sub(2))), pw)), ResetColor)?;
            } else {
                let idx = i - 2;
                if idx < items.len().min(10) {
                    // map visible row -> real index in filtered list
                    // pulled state honest: ● live-pulled, ○ json-only offline
                    let sel = idx == (app.picker_sel.min(items.len().saturating_sub(1)).min(9));
                    let mark = if sel { "▸" } else { " " };
                    let dot = if app.picker_live.is_empty() || is_pulled(&items[idx], &app.picker_live) { "●" } else { "○" };
                    let mut label = format!("{} {} {}", mark, dot, items[idx]);
                    if label.chars().count() > pw.saturating_sub(2) {
                        label = label.chars().take(pw.saturating_sub(5)).collect::<String>() + "...";
                    }
                    let c = if sel { prim } else { txtc };
                    execute!(out, SetForegroundColor(c), SetBackgroundColor(surf), Print(theme::full_row(&pad_right(&label, pw), pw)), ResetColor)?;
                } else {
                    execute!(out, SetBackgroundColor(surf), Print(" ".repeat(pw)), ResetColor)?;
                }
            }
        }
    }

    execute!(out, ResetColor)?;
    out.flush()?;
    Ok(())
}

fn pad_right(s: &str, n: usize) -> String {
    let len = s.chars().count();
    if len >= n {
        return s.chars().take(n).collect();
    }
    format!("{}{}", s, " ".repeat(n - len))
}

fn pad_center(s: &str, n: usize) -> String {
    let len = s.chars().count();
    if len >= n {
        return s.chars().take(n).collect();
    }
    let pad = n - len;
    format!("{}{}{}", " ".repeat(pad / 2), s, " ".repeat(pad - pad / 2))
}

fn is_slash(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

/// run the single pending write only on explicit yes. every exec logged.
/// yes always applies to the current pending even if the model chattered
/// after staging it — pending is only cleared here, on no, or on expiry.
fn confirm_pending(app: &mut App, yes: bool) {
    let Some(p) = app.pending.clone() else {
        app.push_notice("no pending write :3");
        return;
    };
    if ftools::pending_expired(p.at, std::time::Instant::now()) {
        app.pending = None;
        app.status = "ready".to_string();
        app.push_notice("pending write expired (120s) — re-propose to try again");
        log_submit("write expired without exec");
        return;
    }
    if !yes {
        app.pending = None;
        app.status = "ready".to_string();
        app.push_notice("write cancelled — nothing written :3");
        log_submit(&format!("write cancelled path={}", p.path));
        return;
    }
    match ftools::execute(&p) {
        Ok(n) => {
            app.pending = None;
            app.status = "ready".to_string();
            app.push_notice(&format!("wrote {} ({}b) — authoritative write result :3", ftools::friendly(&p.canon), n));
            log_submit(&format!("write exec path={} bytes={}", p.path, n));
        }
        Err(e) => {
            app.pending = None;
            app.status = "ready".to_string();
            app.push_card("error", &e);
            log_submit(&format!("write exec failed path={} err={}", p.path, e));
        }
    }
}

async fn handle_chat(app: &mut App, text: &str) {
    // gate first: bare yes/no consumes pending locally before any ollama
    // call. anything else is a new task — the stale pending is superseded
    // with a short note and the new task runs normally, never re-proposed.
    if app.pending.is_some() {
        if let Some(yes) = ftools::confirm_bare(text) {
            confirm_pending(app, yes);
            return;
        }
        if let Some(p) = app.pending.clone() {
            if ftools::pending_expired(p.at, std::time::Instant::now()) {
                app.pending = None;
                app.push_notice("pending write expired (120s) — re-propose to try again");
            } else {
                let note = ftools::supersede_note(&ftools::friendly(&p.canon));
                let logged = p.path.clone();
                app.pending = None;
                app.status = "ready".to_string();
                app.push_notice(&note);
                log_submit(&format!("write superseded path={} new={}", logged, text.trim().chars().take(60).collect::<String>()));
            }
        }
    }
    // slash file fallback so tiny models without native tools still work.
    // no shell, no deletes. write stages a proposal, never auto-runs.
    if let Some(cmd) = ftools::parse_slash(text) {
        match cmd {
            ftools::SlashCmd::Ls(p) => {
                match ftools::list_dir(&p) {
                    Ok(out) => app.push_card("model", &format!("ls {}:\n{}", p, out)),
                    Err(e) => app.push_card("error", &e),
                }
                return;
            }
            ftools::SlashCmd::Read(p) => {
                match ftools::read_file(&p) {
                    Ok(out) => app.push_card("model", &format!("read {}:\n{}", p, out.chars().take(1200).collect::<String>())),
                    Err(e) => app.push_card("error", &e),
                }
                return;
            }
            ftools::SlashCmd::Write(p, body) => {
                // sticky: never clobber a live pending with a 0-byte re-ask.
                if let Some(cur) = app.pending.clone() {
                    if !ftools::pending_expired(cur.at, std::time::Instant::now())
                        && body.trim().is_empty()
                    {
                        let left = ftools::pending_left(cur.at, std::time::Instant::now());
                        app.push_notice(&ftools::staged_line(&ftools::friendly(&cur.canon), cur.bytes, left));
                        return;
                    }
                    if !ftools::pending_expired(cur.at, std::time::Instant::now())
                        && ftools::same_canon(&p, &cur.path)
                        && body.trim().is_empty()
                    {
                        let left = ftools::pending_left(cur.at, std::time::Instant::now());
                        app.push_notice(&ftools::staged_line(&ftools::friendly(&cur.canon), cur.bytes, left));
                        return;
                    }
                }
                match ftools::propose(&p, &body) {
                    Ok(prop) => {
                        let left = ftools::pending_left(prop.at, std::time::Instant::now());
                        let msg = ftools::proposal_line(&ftools::friendly(&prop.canon), prop.bytes, left);
                        app.pending = Some(prop);
                        app.status = "awaiting yes".to_string();
                        app.push_notice(&msg);
                    }
                    Err(e) => app.push_card("error", &e),
                }
                return;
            }
        }
    }
    if text.trim() == "/yes" {
        confirm_pending(app, true);
        return;
    }
    if text.trim() == "/no" {
        confirm_pending(app, false);
        return;
    }
    // slash commands — memory only, no applets, no thinking row.
    if text.starts_with("/remember ") {
        let fact = text.trim_start_matches("/remember ").trim();
        let msg = memory::remember("human", fact);
        app.push_card("memory", &msg);
        return;
    }
    if text.trim() == "/memory" {
        let m = memory::inject();
        let shown = if m.trim().is_empty() { "(empty)".to_string() } else { m.chars().take(1200).collect::<String>() };
        app.push_card("memory", &shown);
        return;
    }
    if text.trim() == "/model" {
        app.push_card("model", &format!("current: {}", app.model));
        return;
    }
    if text.trim() == "/help" {
        app.push_card("help", "chat · /ls path · /read path · /write path + text then yes · /memory · /remember <fact> · /model · /bye");
        return;
    }

    // normal chat: instant echo + attached thinking before await.
    // run_loop already echoed + drew for the live path; this fallback
    // keeps direct calls attached too without double echo.
    let already_echoed = app.messages.last().map(|m| m.role == "user" && m.content == text).unwrap_or(false);
    if !already_echoed {
        app.messages.push(ChatMessage { role: "user".to_string(), content: text.to_string(), tool_calls: None, tool_call_id: None });
        app.echo_user(text);
        app.push_thinking();
    }
    chat_reply(app).await;
}

/// route one chat turn: local qwen first, remote spark on demand or pick.
/// spark models go direct to meta or zen. local connection failure falls
/// back once to visible spark so ollama-down still answers when keyed.
async fn route_chat(
    model: &str,
    messages: &[ChatMessage],
    tools: Option<&[serde_json::Value]>,
) -> Result<serde_json::Value> {
    if spark::is_spark(model) {
        return spark::chat(model, messages, tools).await;
    }
    match chat_once(model, messages, tools).await {
        Ok(v) => Ok(v),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            let conn = msg.contains("connection") || msg.contains("refused") || msg.contains("failed to connect") || msg.contains("ollama error");
            let spark_ids = spark::visible_ids();
            if conn && !spark_ids.is_empty() {
                let fb = if spark_ids.iter().any(|m| m == spark::ZEN_FREE) {
                    spark::ZEN_FREE.to_string()
                } else {
                    spark_ids.into_iter().next().unwrap_or_else(|| spark::META_13.to_string())
                };
                spark::chat(&fb, messages, tools).await.map_err(|e2| {
                    anyhow::anyhow!("local failed ({}), remote {} also failed: {}", e, fb, e2)
                })
            } else {
                Err(e)
            }
        }
    }
}

/// true when a reply counts as empty: no text and no tool calls.
pub fn needs_empty_retry(content: &str, has_tools: bool) -> bool {
    content.trim().is_empty() && !has_tools
}

/// friendly irc notice body naming the model plus likely causes.
/// user line is kept, input is never eaten.
pub fn empty_notice(model: &str) -> String {
    let nick = irc::short_nick(model);
    format!("{} ({}) gave an empty reply — likely ollama down, overloaded, or still loading. your line is kept, just try again :3", model, nick)
}

/// ollama reply assuming user echo + thinking row already pushed.
/// replaces thinking with the reply so it feels attached.
async fn chat_reply(app: &mut App) {

    let tools = all_tool_defs();
    let tool_slice: Option<&[serde_json::Value]> = if is_tool_capable(&app.model) { Some(&tools) } else { None };

    match route_chat(&app.model, &app.messages, tool_slice).await {
        Ok(v) => {
            let mut msg = v.get("message").cloned().unwrap_or_default();
            let mut content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
            // retry once on empty content with no tool calls before
            // showing the friendly notice — transient ollama blanks recover.
            let has_tools = msg.get("tool_calls").and_then(|t| t.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
            if needs_empty_retry(&content, has_tools) {
                if let Ok(v2) = route_chat(&app.model, &app.messages, tool_slice).await {
                    msg = v2.get("message").cloned().unwrap_or_default();
                    content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                }
            }
            // tool calls? run max 1 extra hop (memory tools only).
            // staged writes stay quiet: one proposal line plus yes, no
            // second model chatter — the receipt is the only confirmation.
            let mut extra_note = String::new();
            let mut staged_write = false;
            if let Some(calls) = msg.get("tool_calls").and_then(|t| t.as_array()) {
                if !calls.is_empty() && is_tool_capable(&app.model) {
                    let mut tool_msgs: Vec<ChatMessage> = Vec::new();
                    for c in calls.iter().take(2) {
                        let name = c.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                        let args = c.get("function").and_then(|f| f.get("arguments")).cloned().unwrap_or(serde_json::json!({}));
                        let id = c.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                        // gated write: sticky proposal, never auto-run, never
                        // rendered as chat. friendly gate notice only — raw
                        // receipts stay out of history and model history so
                        // the model cannot echo them. tagged tool note keeps
                        // the second pass honest without visible leak.
                        if name == "write_file" {
                            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                            let t = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            // model-facing tag only, never rendered as chat.
                            let tag = |id: &str| {
                                if id.is_empty() {
                                    "[gate:write-staged awaiting user yes]".to_string()
                                } else {
                                    format!("[gate:write-staged {} awaiting user yes]", id)
                                }
                            };
                            if let Some(cur) = app.pending.clone() {
                                if !ftools::pending_expired(cur.at, std::time::Instant::now())
                                    && (t.trim().is_empty() || ftools::same_canon(p, &cur.path) && t.trim().is_empty())
                                {
                                    let left = ftools::pending_left(cur.at, std::time::Instant::now());
                                    app.status = "awaiting yes".to_string();
                                    app.push_notice(&ftools::staged_line(&ftools::friendly(&cur.canon), cur.bytes, left));
                                    tool_msgs.push(ChatMessage { role: "tool".to_string(), content: tag(id.as_str()), tool_calls: None, tool_call_id: if id.is_empty() { None } else { Some(id) } });
                                    staged_write = true;
                                    continue;
                                }
                                if !ftools::pending_expired(cur.at, std::time::Instant::now())
                                    && ftools::same_canon(p, &cur.path)
                                {
                                    let left = ftools::pending_left(cur.at, std::time::Instant::now());
                                    app.status = "awaiting yes".to_string();
                                    app.push_notice(&ftools::staged_line(&ftools::friendly(&cur.canon), cur.bytes, left));
                                    tool_msgs.push(ChatMessage { role: "tool".to_string(), content: tag(id.as_str()), tool_calls: None, tool_call_id: if id.is_empty() { None } else { Some(id) } });
                                    staged_write = true;
                                    continue;
                                }
                            }
                            match ftools::propose(p, t) {
                                Ok(prop) => {
                                    let left = ftools::pending_left(prop.at, std::time::Instant::now());
                                    app.pending = Some(prop.clone());
                                    app.status = "awaiting yes".to_string();
                                    app.push_notice(&ftools::proposal_line(&ftools::friendly(&prop.canon), prop.bytes, left));
                                    tool_msgs.push(ChatMessage { role: "tool".to_string(), content: tag(id.as_str()), tool_calls: None, tool_call_id: if id.is_empty() { None } else { Some(id) } });
                                    staged_write = true;
                                }
                                Err(e) => {
                                    app.status = "awaiting yes".to_string();
                                    app.push_notice(&e);
                                    tool_msgs.push(ChatMessage { role: "tool".to_string(), content: format!("[gate:write-denied] {}", e), tool_calls: None, tool_call_id: if id.is_empty() { None } else { Some(id) } });
                                    staged_write = true;
                                }
                            }
                            continue;
                        }
                        let result = run_tool(name, &args);
                        extra_note.push_str(&format!("\n[{} → {}]", name, result));
                        tool_msgs.push(ChatMessage { role: "tool".to_string(), content: result, tool_calls: None, tool_call_id: if id.is_empty() { None } else { Some(id) } });
                    }
                    // staged write stays quiet: skip second model pass so the
                    // gate proposal line plus yes is the only confirmation.
                    if staged_write {
                        app.remove_thinking();
                        app.busy = false;
                        app.status = "awaiting yes".to_string();
                        return;
                    }
                    // second pass with tool outputs
                    let mut msgs2 = app.messages.clone();
                    msgs2.push(ChatMessage { role: "assistant".to_string(), content: content.clone(), tool_calls: None, tool_call_id: None });
                    msgs2.extend(tool_msgs);
                    if let Ok(v2) = route_chat(&app.model, &msgs2, tool_slice).await {
                        let c2 = v2.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()).unwrap_or("").to_string();
                        if !c2.trim().is_empty() {
                            app.messages.push(ChatMessage { role: "assistant".to_string(), content: c2.clone(), tool_calls: None, tool_call_id: None });
                            app.remove_thinking();
                            app.push_card(&app.model.clone(), &c2);
                            if !extra_note.is_empty() {
                                app.push_dim(&extra_note);
                            }
                            app.busy = false;
                            app.status = "ready".to_string();
                            return;
                        }
                    }
                }
            }
            app.remove_thinking();
            if content.trim().is_empty() && extra_note.trim().is_empty() {
                let model = app.model.clone();
                app.push_card("error", &empty_notice(&model));
            } else {
                app.messages.push(ChatMessage { role: "assistant".to_string(), content: content.clone(), tool_calls: None, tool_call_id: None });
                let body = if extra_note.is_empty() { content } else { format!("{}{}", content, extra_note) };
                app.push_card(&app.model.clone(), &body);
            }
        }
        Err(e) => {
            let emsg = e.to_string();
            app.remove_thinking();
            if is_not_found_error(&emsg) {
                let bad = app.model.clone();
                let hint = pull_hint(&bad);
                log_submit(&format!("model not found model={} err={}", bad, e));
                // auto fallback so next enter works even with ghost model picked
                app.model = fallback_model(&app.picker_live);
                app.status = format!("fell back to {}", app.model);
                app.push_card("error", &hint);
            } else {
                let msg = format!("chat failed: {}\nis ollama up at {}?", e, ollama_base());
                log_submit(&format!("submit fail model={} err={}", app.model, e));
                app.status = "submit failed — see submit.log".to_string();
                app.push_card("error", &msg);
            }
        }
    }
    app.busy = false;
    if app.status.to_lowercase().contains("thinking") || app.status.contains("ready :3") {
        app.status = "ready".to_string();
    }
}

#[allow(dead_code)]
fn run_memory_tool(name: &str, args: &serde_json::Value) -> String {
    run_tool(name, args)
}

/// single dispatcher for all native tools. read-only runs now,
/// write_file never writes here — needs explicit yes via pending slot.
fn run_tool(name: &str, args: &serde_json::Value) -> String {
    match name {
        "read_memory" => {
            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("all");
            let all = memory::inject();
            if q == "all" {
                return all.chars().take(2000).collect();
            }
            let ql = q.to_lowercase();
            let hits: Vec<String> = all
                .split("\n\n")
                .filter(|p| p.to_lowercase().contains(&ql))
                .take(3)
                .map(|s| s.chars().take(600).collect())
                .collect();
            if hits.is_empty() { "(no hits)".to_string() } else { hits.join("\n---\n") }
        }
        "write_memory" => {
            let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("human");
            let fact = args.get("fact").and_then(|v| v.as_str()).unwrap_or("");
            memory::remember(topic, fact)
        }
        "list_dir" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            ftools::list_dir(p).unwrap_or_else(|e| e)
        }
        "read_file" => {
            let p = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            ftools::read_file(p).unwrap_or_else(|e| e)
        }
        "write_file" => {
            "write staged — needs explicit user yes within 30s, use /write path + text then yes :3".to_string()
        }
        _ => format!("unknown tool {}", name),
    }
}

// ── --selftest: fast offline checks, no ollama needed ──
pub fn run_selftest() -> i32 {
    let mut fails = 0;
    let mut ok = |name: &str, pass: bool| {
        println!("{} {}", if pass { "pass" } else { "FAIL" }, name);
        if !pass {
            fails += 1;
        }
    };

    // 1 · palette present
    ok("palette primary #FF6188", theme::PRIMARY == "#FF6188");
    ok("palette background #221F22", theme::BACKGROUND == "#221F22");
    ok("palette surface #262226", theme::SURFACE == "#262226");

    // 2 · rainbow banner: height fixed + colors present + narrow truncation
    ok("banner height 7 fixed", banner::HEIGHT == 7 && banner::LOGO.len() == 4 && banner::plain_lines(80).len() == 7);
    ok("banner rainbow colors present", banner::RAINBOW.len() == 4 && banner::RAINBOW.contains(&theme::ERROR) && banner::RAINBOW.contains(&theme::WARNING) && banner::RAINBOW.contains(&theme::SUCCESS) && banner::RAINBOW.contains(&theme::SECONDARY));
    ok("banner japanese + slime lines present", !banner::JAPANESE.is_empty() && !banner::SLIME_LOADED.is_empty() && banner::SLIME_LOADED.contains("slime") && banner::VERSION_LINE.contains("ayesha-os"));
    ok("banner narrow truncation safe", banner::plain_lines(30).len() == 7 && banner::plain_lines(30).iter().all(|l| l.chars().count() <= 30) && banner::plain_lines(20).iter().all(|l| l.chars().count() <= 20));

    // 3 · wrap respects width (80-col + narrow)
    let cols = 80;
    let w = writer::card_width(cols);
    let long = "word ".repeat(60);
    let bad = writer::wrap_block(&long, w).iter().any(|l| l.chars().count() > w);
    ok("wrap fits card width at 80 cols", !bad && w <= 76);
    let narrow = writer::wrap_block("a b c d e f g h i j k l m n o p", 20);
    ok("wrap fits narrow 20", narrow.iter().all(|l| l.chars().count() <= 20));

    // 4 · irc open lines capped: min(width, cols), no boxes
    ok("card capped 80 -> <=76", writer::card_width(80) <= 76);
    ok("card capped 200 -> 72", writer::card_width(200) == 72);
    let c80 = writer::card("qwen2.5:7b", &"x ".repeat(200), 80);
    ok("card lines never exceed 80", c80.iter().all(|l| l.chars().count() <= 80));
    let c40 = writer::card("qwen2.5:7b", &"hello world ".repeat(20), 40);
    ok("card lines never exceed 40", c40.iter().all(|l| l.chars().count() <= 40));
    let chat = writer::card("qwen2.5:7b", "hello world", 80);
    let has_box = chat.iter().any(|l| l.contains('┌') || l.contains('┐') || l.contains('│') || l.contains('├') || l.contains('└') || l.contains('┘'));
    ok("no box chars in chat lines", !has_box);
    ok("irc reply starts <q7>", chat.first().map(|l| l.contains("<q7>")).unwrap_or(false));
    ok("irc user line shape", writer::user_line("hi", 80).first().map(|l| l.contains("<fox>")).unwrap_or(false));
    ok("irc notice shape", writer::notice_line("switched to qwen", 80).first().map(|l| l.contains("***")).unwrap_or(false));
    ok("open line narrow safe", writer::card("qwen2.5:7b", "hi there hello world", 20).iter().all(|l| l.chars().count() <= 20));

    // 5 · picker stays short: dedup + substring filter, no hardcoded remote
    let all = vec!["qwen2.5:7b".to_string(), "qwen2.5:7b".to_string(), "llama3.2-vision".to_string()];
    let mut ded = all.clone();
    ded.sort();
    ded.dedup();
    ok("picker dedups", ded.len() == 2);
    let f = filter_models(&ded, "qwen");
    ok("picker substring filter", f == vec!["qwen2.5:7b".to_string()]);
    // remote must be empty without keys (run without keys in test env)
    let has_key = std::env::var("OPENROUTER_API_KEY").map(|v| !v.trim().is_empty()).unwrap_or(false)
        || std::env::var("OPENCODE_ZEN_API_KEY").map(|v| !v.trim().is_empty()).unwrap_or(false);
    if !has_key {
        let (_, remote) = config::file_models();
        ok("no hardcoded remote without key", remote.is_empty());
    } else {
        ok("keys present so remote allowed", true);
    }

    // 6 · tools stripped for tiny, kept for default
    ok("qwen2.5:7b keeps tools", is_tool_capable("qwen2.5:7b"));
    ok("ayesha:latest strips tools", !is_tool_capable("ayesha:latest"));
    ok("tiny strips tools", !is_tool_capable("tinyllama"));

    // 7 · default model
    ok("default qwen2.5:7b", default_model() == std::env::var("AYESHA_MODEL").unwrap_or("qwen2.5:7b".to_string()));

    // 8 · memory dir fn never panics
    let _ = memory::dir();
    ok("memory dir resolves", true);

    // 9 · single key insert + backspace + enter submit flag
    {
        use crossterm::event::KeyEventKind;
        let mut s = String::new();
        // press inserts once
        ok("press inserts once", keys::apply_char(&mut s, 'a', KeyEventKind::Press) && s == "a");
        // repeat ignored
        ok("repeat ignored", !keys::apply_char(&mut s, 'a', KeyEventKind::Repeat) && s == "a");
        // release ignored
        ok("release ignored", !keys::apply_char(&mut s, 'b', KeyEventKind::Release) && s == "a");
        // backspace press pops, repeat/release ignored
        ok("backspace press pops", keys::apply_backspace(&mut s, KeyEventKind::Press) && s.is_empty());
        s.push('x');
        ok("backspace repeat ignored", !keys::apply_backspace(&mut s, KeyEventKind::Repeat) && s == "x");
        ok("backspace release ignored", !keys::apply_backspace(&mut s, KeyEventKind::Release) && s == "x");
        // enter submit flag clears input and returns text
        s = "hi".to_string();
        let got = keys::take_submit(&mut s, KeyEventKind::Press);
        ok("enter submit returns text", got.as_deref() == Some("hi"));
        ok("enter clears input", s.is_empty());
        // empty input -> no submit
        s = "   ".to_string();
        ok("empty enter no submit", keys::take_submit(&mut s, KeyEventKind::Press).is_none());
        // release enter -> no submit
        s = "hi".to_string();
        ok("release enter no submit", keys::take_submit(&mut s, KeyEventKind::Release).is_none() && s == "hi");
    }

    // 10 · full bg paint + role colours distinct + narrow safe
    {
        let bg_esc = theme::bg_escape(theme::BACKGROUND);
        ok("bg fill present", bg_esc.contains("48;2;34;31;34"));
        let surf_esc = theme::bg_escape(theme::SURFACE);
        ok("surface fill present", surf_esc.contains("48;2;38;34;38"));
        let roles = [theme::TEXT, theme::PRIMARY, theme::SECONDARY, theme::SUCCESS, theme::WARNING, theme::ERROR, theme::DIM];
        let mut uniq = roles.to_vec();
        uniq.sort();
        uniq.dedup();
        ok("role colours distinct", uniq.len() == roles.len());
        ok("error notice maps to error", theme::role_for_line("[12:00] *** error: boom") == theme::ERROR);
        ok("thinking notice maps to warning", theme::role_for_line("[12:00] *** thinking…") == theme::WARNING);
        ok("join notice maps to success", theme::role_for_line("[12:00] *** joined #ayesha") == theme::SUCCESS);
        ok("you maps to primary", theme::role_for_line("you: hi") == theme::PRIMARY);
        ok("normal maps to text", theme::role_for_line("hello there") == theme::TEXT);
        ok("thin divider maps to dim", theme::role_for_line("──────────────────") == theme::DIM);
        ok("irc notice maps to dim", theme::role_for_line("*** hello there") == theme::DIM);
        ok("irc join maps to success", theme::role_for_line("*** joined #ayesha") == theme::SUCCESS);
        ok("full row narrow safe", theme::full_row("hello world", 20).chars().count() == 20 && theme::full_row(&"x".repeat(100), 30).chars().count() == 30);
    }

    // 11 · pulled-only picker + not found fallback + no full clear per key
    {
        let file = vec!["qwen2.5:7b".to_string(), "qwen2.5-coder:14b".to_string(), "llama3.2-vision".to_string()];
        let live = vec!["qwen2.5:7b".to_string(), "llama3.2-vision".to_string()];
        let list = pulled_picker_list(&file, &live);
        ok("pulled-only hides unpulled coder", !list.iter().any(|m| m == "qwen2.5-coder:14b"));
        ok("pulled-only keeps pulled qwen", list.iter().any(|m| m == "qwen2.5:7b"));
        ok("is_pulled honest", is_pulled("qwen2.5:7b", &live) && !is_pulled("qwen2.5-coder:14b", &live));
        ok("not found detected", is_not_found_error("model 'qwen2.5-coder:14b' not found, try pulling it first"));
        ok("other errors not not-found", !is_not_found_error("connection refused"));
        ok("pull hint names model", pull_hint("qwen2.5-coder:14b").contains("ollama pull qwen2.5-coder:14b"));
        ok("fallback prefers qwen", fallback_model(&live) == "qwen2.5:7b");
        ok("fallback uses live when qwen missing", fallback_model(&vec!["llama3.2-vision".to_string()]) == "llama3.2-vision");
        ok("no full clear per key", !render::FULL_CLEAR_PER_KEY);
        ok("typing uses input-only row", render::input_only_for("char") && render::input_only_for("backspace"));
        ok("debounce batches typing", !render::should_full_draw(2, false) && render::should_full_draw(20, false) && render::should_full_draw(0, true));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 12 · old irc restyle: no footer, irc shapes, banner pinned
    {
        ok("no helper footer", !irc::HAS_FOOTER);
        ok("irc prompt is [fox]", irc::prompt() == "[fox] ");
        ok("irc user shape", irc::is_user_line("<fox> hi"));
        ok("irc reply shape", irc::is_reply_line("<ayesha> hello :3"));
        ok("irc notice shape", irc::is_notice("*** switched to qwen2.5:7b"));
        let w = writer::notice_line("joined #ayesha", 80);
        ok("notice narrow safe", w.iter().all(|l| l.chars().count() <= 80));
        let u = writer::user_line("hi", 20);
        ok("user narrow safe", u.iter().all(|l| l.chars().count() <= 20));
        // footer helper text must be gone from irc prompt + welcome + user lines
        let foot = "ctrl+p models";
        ok("no footer in prompt", !irc::prompt().contains(foot));
        ok("no footer in irc lines", !writer::user_line("hi", 80).join(" ").contains(foot) && !writer::card("qwen2.5:7b", "hi", 80).join(" ").contains(foot));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 13 · irc timestamps + short nicks + narrow safe
    {
        ok("stamp shape valid", irc::is_valid_stamp("[12:04] <fox> hi") && irc::is_valid_stamp(&irc::with_stamp("<fox> hi")));
        ok("stamp stripped", irc::strip_stamp("[12:04] <fox> hi").contains("<fox>"));
        ok("nick qwen2.5:7b to q7", irc::short_nick("qwen2.5:7b") == "q7");
        ok("nick ayesha:latest to ash", irc::short_nick("ayesha:latest") == "ash");
        ok("nick llama short", irc::short_nick("llama3.2-vision") == "l32");
        ok("nick generic short", irc::short_nick("opencode/big-pickle").len() <= 5 && irc::short_nick("opencode/big-pickle") == irc::short_nick("opencode/big-pickle"));
        let r = writer::card("qwen2.5:7b", "hello", 80);
        ok("reply line has stamp + short nick", r.first().map(|l| irc::is_valid_stamp(l) && l.contains("<q7>")).unwrap_or(false));
        let u = writer::user_line("hi", 80);
        ok("user line has stamp + fox", u.first().map(|l| irc::is_valid_stamp(l) && l.contains("<fox>")).unwrap_or(false));
        let n = writer::notice_line("switched to q7", 80);
        ok("notice has stamp + stars", n.first().map(|l| irc::is_valid_stamp(l) && l.contains("***")).unwrap_or(false));
        ok("timestamp narrow safe 20", writer::card("qwen2.5:7b", "hi there hello world test test", 20).iter().all(|l| l.chars().count() <= 20));
        ok("timestamp narrow safe 30", writer::user_line("hello world hello world hello", 30).iter().all(|l| l.chars().count() <= 30));
        ok("banner still pinned after stamps", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 14 · instant echo + single ready + banner pinned
    {
        let echo = instant_echo_lines("hi", 80);
        ok("instant echo has stamp + fox", echo.first().map(|l| irc::is_valid_stamp(l) && l.contains("<fox>") && l.contains("hi")).unwrap_or(false));
        let th = thinking_line(80);
        ok("thinking row attached", irc::is_valid_stamp(&th) && th.to_lowercase().contains("thinking"));
        let mut h = echo.clone();
        h.push(th.clone());
        remove_thinking(&mut h);
        ok("thinking replaced, echo kept", h.len() == echo.len() && !h.join(" ").to_lowercase().contains("thinking") && h.join(" ").contains("<fox>"));
        let s1 = build_status_text("q7", "ready :3", false);
        ok("single ready only", s1.to_lowercase().matches("ready").count() == 1 && s1.contains("q7"));
        let s2 = build_status_text("q7", "ready", false);
        ok("ready not doubled", s2.to_lowercase().matches("ready").count() == 1);
        let s3 = build_status_text("q7", "anything", true);
        ok("busy shows thinking once", s3.to_lowercase().contains("thinking") && s3.to_lowercase().matches("ready").count() == 0);
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 15 · muse spark provider present + nick short + no key hidden
    {
        ok("meta base present", spark::META_BASE == "https://api.meta.ai/v1");
        ok("zen base present", spark::ZEN_BASE == "https://api.opencode.ai/v1");
        ok("spark ids present", spark::all_ids().len() == 3 && spark::all_ids().contains(&spark::META_13.to_string()));
        ok("nick ms13 short", irc::short_nick("muse-spark-1.3") == "ms13");
        ok("nick msc short", irc::short_nick("muse-spark-1.3-contributor") == "msc");
        ok("nick msf short", irc::short_nick("muse-spark-1.3-contributor-free") == "msf");
        ok("spark detected", spark::is_spark("muse-spark-1.3") && spark::is_spark("muse-spark-1.3-contributor-free") && !spark::is_spark("qwen2.5:7b"));
        ok("no key means hidden", spark::visible_for(false, false).is_empty());
        ok("meta key shows ms13+msc", spark::visible_for(true, false) == vec!["muse-spark-1.3".to_string(), "muse-spark-1.3-contributor".to_string()]);
        ok("zen auth shows msf", spark::visible_for(false, true) == vec!["muse-spark-1.3-contributor-free".to_string()]);
        ok("tool reject detected", spark::is_tool_reject("400 tools not supported") && !spark::is_tool_reject("connection refused"));
        let r = writer::card("muse-spark-1.3", "hello", 80);
        ok("spark reply uses ms13", r.first().map(|l| l.contains("<ms13>")).unwrap_or(false));
        ok("spark reply narrow safe", writer::card("muse-spark-1.3-contributor-free", "hi there hello world test", 20).iter().all(|l| l.chars().count() <= 20));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 16 · stamp green + body uniform + narrow safe
    {
        ok("stamp green escape", theme::fg_escape(theme::SUCCESS).contains("38;2;169;220;118"));
        ok("stamp color is success", theme::stamp_color() == theme::color(theme::SUCCESS));
        ok("body error words stay text", theme::body_color_for_line("[12:00] <q7> this failed pink error talk") == theme::TEXT);
        ok("body lines uniform", theme::body_color_for_line("[12:00] <q7> first pink words") == theme::body_color_for_line("[12:01] continuation white words"));
        ok("code fence stays text", theme::body_color_for_line("[12:00] ```rust code here") == theme::TEXT);
        ok("indented code stays text", theme::body_color_for_line("[12:00]     indented snippet") == theme::TEXT);
        ok("notice error stays error", theme::body_color_for_line("[12:00] *** error: boom") == theme::ERROR);
        ok("nick still primary", theme::role_for_line("[12:00] <q7> hi") == theme::PRIMARY);
        ok("uniform narrow safe 20", writer::card("qwen2.5:7b", "pink white pink white pink white test words here", 20).iter().all(|l| l.chars().count() <= 20));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 17 · real file tools: allowlist + traversal + yes gate + slash
    {
        ok("allowlist deny outside", ftools::resolve(r"c:\windows\system32\drivers\etc\hosts").is_err());
        ok("traversal deny dotdot", ftools::resolve(r"c:\ayesha-mini\..\windows\x").is_err());
        ok("traversal deny nested", ftools::resolve("desktop/../..").is_err());
        ok("yes gate yes", ftools::confirm_word("yes") == ftools::Confirm::Yes);
        ok("yes gate no", ftools::confirm_word("no") == ftools::Confirm::No);
        ok("yes gate y ok", ftools::confirm_word("y") == ftools::Confirm::Yes);
        ok("yes gate ok word", ftools::confirm_word("ok") == ftools::Confirm::Yes);
        ok("yes gate case ok", ftools::confirm_word("  YES  ") == ftools::Confirm::Yes);
        ok("pending timeout 120s", ftools::pending_expired(std::time::Instant::now() - std::time::Duration::from_secs(121), std::time::Instant::now()));
        ok("pending fresh ok", !ftools::pending_expired(std::time::Instant::now(), std::time::Instant::now()));
        ok("pending 119s still live", !ftools::pending_expired(std::time::Instant::now() - std::time::Duration::from_secs(119), std::time::Instant::now()));
        ok("slash ls parses", ftools::parse_slash("/ls desktop") == Some(ftools::SlashCmd::Ls("desktop".to_string())));
        ok("slash read parses", ftools::parse_slash("/read foo.txt") == Some(ftools::SlashCmd::Read("foo.txt".to_string())));
        ok("slash write parses", ftools::parse_slash("/write notes.txt hello world here") == Some(ftools::SlashCmd::Write("notes.txt".to_string(), "hello world here".to_string())));
        ok("slash bare write rejected", ftools::parse_slash("/write notes.txt").is_none());
        ok("slash plain none", ftools::parse_slash("hello there").is_none());
        ok("tool defs present", all_tool_defs().iter().any(|v| v.to_string().contains("list_dir")) && all_tool_defs().iter().any(|v| v.to_string().contains("read_file")) && all_tool_defs().iter().any(|v| v.to_string().contains("write_file")));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 18 · sticky confirm loop: content + yes variants + friendly paths
    {
        ok("sticky reuse on 0 bytes", ftools::should_reuse_pending(true, 0));
        ok("no reuse when fresh", !ftools::should_reuse_pending(false, 0));
        ok("no reuse with content", !ftools::should_reuse_pending(true, 9));
        ok("yes variants", ftools::confirm_word("y") == ftools::Confirm::Yes && ftools::confirm_word("ok") == ftools::Confirm::Yes && ftools::confirm_word("YES") == ftools::Confirm::Yes);
        ok("same canon agrees", ftools::same_canon(r"c:\ayesha-mini\poop.pxp", r"c:\ayesha-mini\poop.pxp"));
        ok("different paths differ", !ftools::same_canon(r"c:\ayesha-mini\a.txt", r"c:\ayesha-mini\b.txt"));
        let p = std::path::PathBuf::from(r"\\?\c:\ayesha-mini\poop.pxp");
        let f = ftools::friendly(&p);
        ok("friendly strips verbatim", !f.contains(r"\\?\") && f.contains("poop.pxp"));
        ok("friendly short", f.chars().count() < r"\\?\c:\ayesha-mini\poop.pxp".len());
        // sticky propose: 0-byte re-ask must not clobber staged bytes
        let probe = r"c:\ayesha-mini\__selftest_sticky_probe__.tmp";
        if let Ok(first) = ftools::propose(probe, "hello sticky") {
            let reuse = ftools::should_reuse_pending(true, 0);
            ok("sticky keeps bytes", reuse && first.bytes == "hello sticky".len() && first.content == "hello sticky");
        } else {
            ok("sticky keeps bytes", false);
        }
        ok("countdown live", ftools::pending_left(std::time::Instant::now(), std::time::Instant::now()) <= 120);
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 19 · desktop allowlist: abs + bare + verbatim + tilde allowed
    {
        let home = ftools::home_base().expect("home base resolves");
        let probe = "__selftest_desk_probe__.tmp";
        let abs = home.join("Desktop").join(probe).to_string_lossy().to_string();
        let bare = format!("desktop\\{}", probe);
        let tilde = format!("~/desktop/{}", probe);
        let verb = format!("\\\\?\\{}", abs);
        ok("desktop abs allowed", ftools::resolve(&abs).is_ok());
        ok("bare desktop allowed", ftools::resolve(&bare).is_ok());
        ok("tilde desktop allowed", ftools::resolve(&tilde).is_ok());
        ok("verbatim allowed", ftools::resolve(&verb).is_ok());
        ok("spellings agree", ftools::same_canon(&abs, &bare) && ftools::same_canon(&bare, &tilde));
        ok("outside denied", ftools::resolve(r"c:\windows\system32\drivers\etc\hosts").is_err());
        ok("traversal denied", ftools::resolve("desktop\\..\\..\\windows").is_err());
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 20 · both desktops + deny log line + banner pinned
    {
        let roots = ftools::allow_roots();
        let has_plain = roots.iter().any(|r| r.to_string_lossy().replace('/', "\\").to_lowercase().ends_with("desktop"));
        let has_od = roots.iter().any(|r| {
            let s = r.to_string_lossy().replace('/', "\\").to_lowercase();
            s.contains("onedrive") && s.ends_with("desktop")
        });
        ok("both desktops listed", has_plain && has_od);
        let has_docs = roots.iter().filter(|r| {
            let s = r.to_string_lossy().replace('/', "\\").to_lowercase();
            s.ends_with("documents") || s.ends_with("documents\\ayesha")
        }).count();
        ok("both documents listed", has_docs >= 2);
        let home = ftools::home_base().expect("home base resolves");
        let probe = "__selftest_both_desk_probe__.tmp";
        let od_abs = home.join("OneDrive").join("Desktop").join(probe).to_string_lossy().to_string();
        ok("onedrive desktop allowed", ftools::resolve(&od_abs).is_ok());
        // deny path returns friendly expected-but-got reason
        let err = ftools::resolve(r"c:\windows\system32\drivers\etc\hosts").unwrap_err();
        ok("deny names friendly reason", err.contains("expected desktop") && err.contains("got"));
        // every deny appends proposed + roots to submit.log
        let logp = std::path::PathBuf::from(r"c:\ayesha-mini\submit.log");
        let before = std::fs::read_to_string(&logp).unwrap_or_default().len();
        let _ = ftools::resolve(r"c:\windows\nope_selftest_deny_probe__.tmp");
        let after = std::fs::read_to_string(&logp).unwrap_or_default();
        ok("deny log line", after.len() > before && after.contains("deny proposed=") && after.contains("roots=["));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 21 · linux home alias + quiet single proposal + banner pinned
    {
        // any path with a desktop segment lands on the real desktop
        let linux_style = "c:/home/apullz/desktop/__selftest_alias_probe__.tmp";
        let bare = "desktop\\__selftest_alias_probe__.tmp";
        ok("home alias allowed", ftools::resolve(linux_style).is_ok());
        ok("alias matches bare desktop", ftools::same_canon(linux_style, bare));
        let linux_docs = "c:/home/apullz/documents/__selftest_alias_doc__.tmp";
        ok("documents alias allowed", ftools::resolve(linux_docs).is_ok());
        // quiet single proposal line: one line, yes cue, no prose question
        let q = ftools::proposal_line("desktop\\poop.pxp", 12, 119);
        ok("quiet single proposal", !q.contains('\n') && q.contains("yes?") && q.contains("desktop\\poop.pxp"));
        ok("no prose questions", !q.to_lowercase().contains("shall i") && !q.to_lowercase().contains("should i"));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 22 · empty reply retry once + friendly notice + banner pinned
    {
        ok("retry on empty no tools", needs_empty_retry("", false));
        ok("retry on blank no tools", needs_empty_retry("   ", false));
        ok("no retry with text", !needs_empty_retry("hi there", false));
        ok("no retry with tools", !needs_empty_retry("", true));
        let n = empty_notice("qwen2.5:7b");
        ok("notice names model", n.contains("qwen2.5:7b"));
        ok("notice names nick", n.contains("q7"));
        ok("notice lists causes", n.contains("ollama down") && n.contains("overloaded") && n.contains("try again"));
        ok("notice keeps user line hint", n.contains("your line is kept"));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 23 · yes consumes pending + receipts never rendered + history clean
    {
        ok("gate yes consumes", ftools::confirm_gate("yes") == Some(true));
        ok("gate y consumes", ftools::confirm_gate("y") == Some(true));
        ok("gate ok consumes", ftools::confirm_gate("ok thanks") == Some(true));
        ok("gate yes please consumes", ftools::confirm_gate("yes please do it") == Some(true));
        ok("gate no consumes", ftools::confirm_gate("no") == Some(false));
        ok("gate chatter other", ftools::confirm_gate("hello there") == None);
        ok("receipt authoritative hidden", ftools::is_receipt_line("[12:00] *** error: authoritative nothing written, say nothing :3"));
        ok("receipt bracket hidden", ftools::is_receipt_line("[write_file → something]"));
        ok("friendly wrote shown", !ftools::is_receipt_line("[12:00] *** wrote desktop\\poop.pxp (12b)"));
        ok("friendly cancelled shown", !ftools::is_receipt_line("[12:00] *** write cancelled — nothing written"));
        // staged history holds quiet notice only, no receipt text
        let staged = ftools::staged_line("desktop\\poop.pxp", 12, 119);
        let proposed = ftools::proposal_line("desktop\\poop.pxp", 12, 119);
        ok("history clean staged", !ftools::is_receipt_line(&staged) && staged.contains("yes?"));
        ok("history clean proposed", !ftools::is_receipt_line(&proposed) && proposed.contains("yes?"));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    // 24 · stale pending: bare yes consumes, new task supersedes
    {
        ok("bare yes consumes", ftools::confirm_bare("yes") == Some(true));
        ok("bare y consumes", ftools::confirm_bare("y") == Some(true));
        ok("bare yes please consumes", ftools::confirm_bare("yes please") == Some(true));
        ok("bare ok thanks consumes", ftools::confirm_bare("ok thanks") == Some(true));
        ok("bare no consumes", ftools::confirm_bare("no") == Some(false));
        ok("new task supersedes", ftools::confirm_bare("yes please do it") == None);
        ok("chatter supersedes", ftools::confirm_bare("hello there") == None);
        ok("write ask supersedes", ftools::confirm_bare("write foo now") == None);
        let note = ftools::supersede_note("desktop\\poop.pxp");
        ok("pending note shown", note.contains("superseded") && note.contains("desktop\\poop.pxp") && note.contains("new task first"));
        ok("note single line", !note.contains('\n'));
        ok("banner still pinned 7", banner::HEIGHT == 7 && banner::plain_lines(80).len() == 7);
    }

    if fails == 0 {
        println!("selftest ok — 24 groups passed :3");
        0
    } else {
        println!("selftest FAIL — {} checks failed", fails);
        1
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        std::process::exit(run_selftest());
    }

    let model = if args.len() > 1 && !args[1].starts_with('-') { args[1].clone() } else { default_model() };
    let mut app = App::new(model);
    let mut out = std::io::stdout();

    terminal::enable_raw_mode()?;
    execute!(out, EnterAlternateScreen)?;
    let res = run_loop(&mut out, &mut app).await;
    execute!(out, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    if let Err(e) = res {
        eprintln!("ayesha-mini error: {:#}", e);
    }
    Ok(())
}

async fn run_loop(out: &mut Stdout, app: &mut App) -> Result<()> {
    // initial picker data so first ctrl+p is instant, pulled-only
    let (list, live) = build_picker_list_from(config::file_models().0, Vec::new(), live_tags().await);
    // keep startup honest: default to fallback when saved model not pulled
    if !live.is_empty() && !is_pulled(&app.model, &live) {
        app.model = fallback_model(&live);
    }
    app.picker_all = list;
    app.picker_live = live;
    draw(out, app)?;
    app.last_full_draw = std::time::Instant::now();
    loop {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(k) = event::read()? {
                // fix 1: only accept single press. ignore repeat + release
                // or every key types twice on windows (press + release).
                if !matches!(k.kind, KeyEventKind::Press) {
                    continue;
                }
                // ctrl+p — rebuild pulled-only list on every open, stays short
                if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('p') | KeyCode::Char('P')) {
                    if !app.picker_open {
                        app.picker_open = true;
                        app.picker_query.clear();
                        app.picker_sel = 0;
                        app.status = "loading models…".to_string();
                        draw(out, app)?;
                        let (list, live) = build_picker_list_from(config::file_models().0, Vec::new(), live_tags().await);
                        app.picker_all = list;
                        app.picker_live = live;
                        app.status = format!("{} pulled models", app.picker_all.len());
                    } else {
                        app.picker_open = false;
                    }
                    draw(out, app)?;
                    app.last_full_draw = std::time::Instant::now();
                    continue;
                }
                if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c') | KeyCode::Char('d')) {
                    break;
                }
                if app.picker_open {
                    match k.code {
                        KeyCode::Esc => app.picker_open = false,
                        KeyCode::Enter => {
                            let items = app.filtered();
                            if !items.is_empty() {
                                let pick = items[app.picker_sel.min(items.len() - 1)].clone();
                                app.model = pick.clone();
                                app.push_notice(&format!("switched to {}", pick));
                                // refresh system tools note
                                if !is_tool_capable(&pick) {
                                    app.push_dim("(tiny model — tools stripped, plain chat)");
                                }
                            }
                            app.picker_open = false;
                        }
                        KeyCode::Backspace => {
                            // single-press backspace via helper so repeat does not eat text
                            keys::apply_backspace(&mut app.picker_query, k.kind);
                            app.picker_sel = 0;
                        }
                        KeyCode::Up => app.picker_sel = app.picker_sel.saturating_sub(1),
                        KeyCode::Down => {
                            let n = app.filtered().len().saturating_sub(1);
                            app.picker_sel = (app.picker_sel + 1).min(n);
                        }
                        KeyCode::Char(ch) => {
                            // single-press insert only
                            if keys::apply_char(&mut app.picker_query, ch, k.kind) {
                                app.picker_sel = 0;
                            }
                        }
                        _ => {}
                    }
                    draw(out, app)?;
                    continue;
                }
                match k.code {
                    KeyCode::Esc => {
                        if app.input.is_empty() {
                            break;
                        }
                        app.input.clear();
                    }
                    KeyCode::Enter => {
                        // instant echo: user line + thinking row draw before await
                        // so the reply feels attached, then thinking swaps out.
                        let Some(t) = keys::take_submit(&mut app.input, k.kind) else {
                            draw(out, app)?;
                            continue;
                        };
                        if t == "/bye" || t == "/quit" || t == "/q" {
                            break;
                        }
                        if t == "/clear" {
                            app.history.clear();
                            app.status = "cleared".to_string();
                            draw(out, app)?;
                            continue;
                        }
                        // gate first: bare yes/no consumes pending locally before
                        // any ollama call. other content supersedes the stale
                        // pending with a short note, then runs as a new task.
                        if app.pending.is_some() {
                            if let Some(yes) = ftools::confirm_bare(&t) {
                                app.echo_user(&t);
                                confirm_pending(app, yes);
                                draw(out, app)?;
                                app.last_full_draw = std::time::Instant::now();
                                continue;
                            }
                            if let Some(p) = app.pending.clone() {
                                if ftools::pending_expired(p.at, std::time::Instant::now()) {
                                    app.pending = None;
                                    app.push_notice("pending write expired (120s) — re-propose to try again");
                                    log_submit("write expired without exec");
                                } else {
                                    let note = ftools::supersede_note(&ftools::friendly(&p.canon));
                                    let logged = p.path.clone();
                                    app.pending = None;
                                    app.status = "ready".to_string();
                                    app.push_notice(&note);
                                    log_submit(&format!("write superseded path={} new={}", logged, t.trim().chars().take(60).collect::<String>()));
                                }
                            }
                        }
                        if is_slash(&t) {
                            handle_chat(app, &t).await;
                            draw(out, app)?;
                            app.last_full_draw = std::time::Instant::now();
                            continue;
                        }
                        log_submit(&format!("submit model={} text={}", app.model, t));
                        app.messages.push(ChatMessage { role: "user".to_string(), content: t.clone(), tool_calls: None, tool_call_id: None });
                        app.echo_user(&t);
                        app.push_thinking();
                        draw(out, app)?;
                        // chat task: awaited inline to keep single writer safe
                        chat_reply(app).await;
                        // thinking already replaced by reply here
                        draw(out, app)?;
                        app.last_full_draw = std::time::Instant::now();
                        continue;
                    }
                    KeyCode::Backspace => {
                        keys::apply_backspace(&mut app.input, k.kind);
                        // dirty input row only, no full clear → no flash
                        draw_input_row(out, app)?;
                        continue;
                    }
                    KeyCode::Up => app.scroll = app.scroll.saturating_add(1),
                    KeyCode::Down => app.scroll = app.scroll.saturating_sub(1),
                    KeyCode::PageUp => app.scroll = app.scroll.saturating_add(5),
                    KeyCode::PageDown => app.scroll = app.scroll.saturating_sub(5),
                    KeyCode::Char(ch) => {
                        keys::apply_char(&mut app.input, ch, k.kind);
                        // batched typing redraw: input row only, debounced
                        let elapsed = app.last_full_draw.elapsed().as_millis() as u64;
                        if render::should_full_draw(elapsed, false) && render::input_only_for("char") {
                            draw_input_row(out, app)?;
                        } else {
                            draw_input_row(out, app)?;
                        }
                        continue;
                    }
                    _ => {}
                }
                draw(out, app)?;
                app.last_full_draw = std::time::Instant::now();
            }
        }
    }
    Ok(())
}

// keep PathBuf import used on all platforms
#[allow(dead_code)]
fn _use_path(p: PathBuf) -> usize {
    p.as_os_str().len()
}
