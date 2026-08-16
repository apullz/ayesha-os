// streaming code-block renderer.
//
// rides alongside `format::LowercaseStreamer`: it consumes already-enforced
// text (lowercased prose, preserved fences) and re-emits it with:
//   - fenced code lines drawn on the theme's code background with a dim
//     gutter and monokai++-style syntax token colors,
//   - inline `code` spans in prose tinted with the inline-code token color.
//
// it is line-buffered so each line is complete before it is painted, and it
// tolerates fences/code that arrive split across chunk boundaries. output is
// display-only — the plain content is still collected by the caller for
// message history.

use crate::theme::{self, Role, SyntaxRole};
use crate::syntax;
use crate::ui::{truncate_cells, visible_cells};

#[derive(Default)]
pub struct CodeStream {
    buf: String,
    in_fence: bool,
    fence_lang: Option<String>,
}

impl CodeStream {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one enforced text delta; returns the display text to print.
    pub fn feed(&mut self, text: &str) -> String {
        self.buf.push_str(text);
        let mut out = self.process_lines();
        if !self.in_fence {
            let tail = std::mem::take(&mut self.buf);
            if !tail.is_empty() {
                if tail.trim_start().starts_with("```") {
                    self.buf = tail;
                } else {
                    out.push_str(&color_inline(&tail));
                }
            }
        }
        out
    }

    /// Flush any leftover partial line (end of stream / early exit).
    pub fn finish(&mut self) -> String {
        let out = self.process_lines();
        let remaining = std::mem::take(&mut self.buf);
        if !remaining.is_empty() {
            let was_in_fence = self.in_fence;
            self.in_fence = false;
            if remaining.trim().starts_with("```") {
                return out;
            }
            let painted = if was_in_fence {
                paint_code_line(&remaining)
            } else {
                color_inline(&remaining)
            };
            return format!("{out}{painted}");
        }
        out
    }

    fn process_lines(&mut self) -> String {
        let mut out = String::new();
        let mut consumed = 0usize;
        let bytes = self.buf.as_bytes();

        let mut line_ends = Vec::new();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                line_ends.push(i);
            }
        }

        for &i in &line_ends {
            let line = &self.buf[consumed..i];
            let rendered = Self::handle_line(line, &mut self.in_fence, &mut self.fence_lang);
            if !rendered.is_empty() {
                out.push_str(&rendered);
                out.push('\n');
            }
            consumed = i + 1;
        }

        if consumed > 0 {
            self.buf.drain(..consumed);
        }
        out
    }

    fn handle_line(line: &str, in_fence: &mut bool, fence_lang: &mut Option<String>) -> String {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            let opening = !*in_fence;
            *in_fence = !*in_fence;
            if opening {
                let lang = trimmed[3..].trim().to_string();
                *fence_lang = if lang.is_empty() { None } else { Some(lang.clone()) };
                if !lang.is_empty() {
                    return paint_code_header(&lang);
                }
            } else {
                *fence_lang = None;
            }
            return String::new();
        }

        if *in_fence {
            paint_code_line(line)
        } else {
            color_inline(line)
        }
    }
}

/// Solid code-panel width for fenced blocks (opencode-style). Shrinks to
/// fit narrow terminals — the designed 80 needs the gutter + indent too,
/// which wrapped the panel on 80-col windows.
const CODE_PANEL_WIDTH: usize = 80;

fn panel_width() -> usize {
    CODE_PANEL_WIDTH.min(crossterm::terminal::size().map(|(c, _)| c as usize).unwrap_or(80).saturating_sub(6).max(24))
}

/// Paint a single code line: dim gutter + syntax-highlighted text on a solid
/// code-background panel.
fn paint_code_line(line: &str) -> String {
    let width = panel_width();
    let truncated: String = if visible_cells(line) > width {
        let mut s = truncate_cells(line, width.saturating_sub(3));
        s.push_str("...");
        s
    } else {
        line.to_string()
    };
    let highlighted = syntax::highlight_line(&truncated);
    let pad = width.saturating_sub(visible_cells(&highlighted));
    let padded = format!("{highlighted}{}", " ".repeat(pad));
    let gutter = theme::paint(Role::Dim, "▐");
    format!("  {gutter} {}", theme::bg_fill(Role::CodeBg, padded))
}

/// opencode-style code block header: the language name on the panel, followed
/// by a dim rule.
fn paint_code_header(lang: &str) -> String {
    let width = panel_width();
    let title = format!("{} ", theme::bold(Role::Secondary, lang));
    let pad = width.saturating_sub(visible_cells(&title));
    let dashes = theme::paint(Role::Dim, "─".repeat(pad));
    let gutter = theme::paint(Role::Dim, "▐");
    format!("  {gutter} {}", theme::bg_fill(Role::CodeBg, format!("{title}{dashes}")))
}

/// Tint inline `` `code` `` spans in a prose line with the inline-code token
/// color. Non-code text passes through untouched.
fn color_inline(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while !rest.is_empty() {
        if let Some(start) = rest.find('`') {
            out.push_str(&rest[..start]);
            rest = &rest[start..];
            if let Some(end_rel) = rest[1..].find('`') {
                let inner = &rest[1..end_rel + 1];
                out.push_str(&format!("{}", theme::paint_syntax(
                    SyntaxRole::InlineCode, inner)));
                rest = &rest[end_rel + 2..];
            } else {
                out.push('`');
                rest = &rest[1..];
            }
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

/// Convenience: write a fully-rendered (non-streaming) text blob. Used by
/// code paths that have the whole message at once. Returns nothing, prints.
#[allow(dead_code)]
pub fn print_rendered(text: &str) {
    let mut stream = CodeStream::new();
    let out = format!("{}{}", stream.feed(text), stream.finish());
    crate::ui::convo_write(&out);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
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

    fn set_monokai() {
        colored::control::set_override(true);
        std::env::set_var("COLORTERM", "truecolor");
        theme::set_active(theme::preset("monokai++").unwrap());
    }

    fn reset_color() {
        colored::control::set_override(false);
    }

    #[test]
    fn prose_passes_through_plain() {
        let mut s = CodeStream::new();
        assert_eq!(s.feed("hello world"), "hello world");
        assert_eq!(s.finish(), "");
    }

    #[test]
    fn fence_content_rendered_as_code() {
        set_monokai();
        let mut s = CodeStream::new();
        let out = s.feed("```rust\nfn main() {\n}\n```\n");
        let plain = strip_ansi(&out);
        // fence markers hidden, code lines kept with gutter + bg
        assert!(plain.contains("fn main() {"));
        assert!(plain.contains("▐"));
        // language header rendered on the panel
        assert!(plain.contains("rust"));
        // syntax-colored tokens (fg + bg escape sequences present)
        assert!(out.contains("\u{1b}["));
        let distinct = out.matches("\u{1b}[").count();
        assert!(distinct >= 2, "expected fg+bg escapes, got {distinct}");
        reset_color();
    }

    #[test]
    fn fence_split_across_chunks() {
        let mut s = CodeStream::new();
        assert_eq!(s.feed("here is code:\n```"), "here is code:\n");
        let plain = strip_ansi(&s.feed("rust\nx = 1\n```\n"));
        assert!(plain.contains("  ▐ x = 1"), "missing code line: {plain:?}");
    }

    #[test]
    fn inline_code_tinted() {
        set_monokai();
        let mut s = CodeStream::new();
        let out = s.feed("use `str` here\n");
        let plain = strip_ansi(&out);
        assert_eq!(plain, "use str here\n");
        assert!(out.contains("\u{1b}["));
        // the backtick span must be colorized differently from plain text
        assert_eq!(out.matches("\u{1b}[").count(), 2, "inline span should carry one SGR pair");
        reset_color();
    }

    #[test]
    fn unclosed_fence_flushed_on_finish() {
        let mut s = CodeStream::new();
        s.feed("```py\nprint(1)");
        let out = s.finish();
        assert!(strip_ansi(&out).contains("print(1)"));
    }

    #[test]
    fn code_line_never_exceeds_panel_width() {
        set_monokai();
        // "  ▐ " prefix + panel → panel_width + 4 cells max, even with
        // wide kaomoji/CJK content and long lines
        let out = paint_code_line("let s = \"(๑•蔷•๑)\"; // kaomoji comment");
        assert!(visible_cells(&strip_ansi(&out)) <= panel_width() + 4);
        let long = paint_code_line(&"蔷".repeat(200));
        let plain = strip_ansi(&long);
        assert!(visible_cells(&plain) <= panel_width() + 4, "panel overflowed: {}", plain.len());
        assert!(plain.contains("..."), "long line should be ellipsized");
        reset_color();
    }
}
