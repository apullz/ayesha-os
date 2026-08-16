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
        print!("{}\r\n", line);
    }
}

// ── tool call / result ────────────────────────────────────

pub fn show_tool_call(name: &str, args: &str) {
    let truncated = if visible_cells(args) > 80 {
        format!("{}...", truncate_cells(args, 77))
    } else {
        args.to_string()
    };
    convo_line(&format!("  {} {} {}",
        theme::bold(Role::Primary, "▪"),
        theme::bold(Role::Text, name),
        theme::paint(Role::Dim, truncated)));
}

const TOOL_BOX_WIDTH: usize = 78;

/// The tool box needs the 2-space indent + both rails on top of
/// `TOOL_BOX_WIDTH`, so a hardcoded 78 overflowed on 80-col terminals and
/// wrapped the panel — one of the "layout buggy" desyncs. Shrink to fit.
fn tool_box_width() -> usize {
    TOOL_BOX_WIDTH.min(terminal_cols().saturating_sub(6).max(20))
}

fn terminal_cols() -> usize {
    crossterm::terminal::size().map(|(c, _)| c as usize).unwrap_or(80)
}

/// How many terminal cells a char occupies. Zero-width marks / joiners /
/// invisible controls take none; CJK, fullwidth and emoji take two;
/// everything else (including box-drawing and kaomoji glyphs, which
/// terminals render narrow by default) takes one. All wrap/truncate math
/// goes through here so the buffer always matches what the terminal paints.
pub(crate) fn char_cells(c: char) -> usize {
    let cp = c as u32;
    if is_zero_width(cp) {
        0
    } else if is_wide(cp) {
        2
    } else {
        1
    }
}

/// Zero-width code points: combining marks (Mn/Me/Mc) across the main
/// blocks, plus invisible controls (ZWSP/ZWJ, BOM, joiners, direction
/// marks, variation selectors, hangul fillers).
fn is_zero_width(cp: u32) -> bool {
    matches!(cp,
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x180E
        | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x206A..=0x206F
        | 0x3164 | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F | 0xFEFF | 0xFFA0
        | 0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x05BF
        | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7 | 0x0610..=0x061A
        | 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06DC | 0x06DF..=0x06E4
        | 0x06E7..=0x06E8 | 0x06EA..=0x06ED | 0x0711 | 0x0730..=0x074A
        | 0x07A6..=0x07B0 | 0x07EB..=0x07F3 | 0x07FD | 0x0816..=0x0819
        | 0x081B..=0x0823 | 0x0825..=0x0827 | 0x0829..=0x082D
        | 0x0859..=0x085B | 0x08D3..=0x0902 | 0x093A | 0x093C | 0x0941..=0x0948
        | 0x094D | 0x0951..=0x0957 | 0x0962..=0x0963 | 0x0981 | 0x09BC
        | 0x09C1..=0x09C4 | 0x09CD | 0x09E2..=0x09E3 | 0x09FE | 0x0A01..=0x0A02
        | 0x0A3C | 0x0A41..=0x0A42 | 0x0A47..=0x0A48 | 0x0A4B..=0x0A4D | 0x0A51
        | 0x0A70..=0x0A71 | 0x0A75 | 0x0A81..=0x0A82 | 0x0ABC | 0x0AC1..=0x0AC5
        | 0x0AC7..=0x0AC8 | 0x0ACD | 0x0AE2..=0x0AE3 | 0x0AFA..=0x0AFF | 0x0B01
        | 0x0B3C | 0x0B3F | 0x0B41..=0x0B44 | 0x0B4D | 0x0B55..=0x0B56
        | 0x0B62..=0x0B63 | 0x0B82 | 0x0BC0 | 0x0BCD | 0x0C00 | 0x0C04
        | 0x0C3E..=0x0C40 | 0x0C46..=0x0C48 | 0x0C4A..=0x0C4D | 0x0C55..=0x0C56
        | 0x0C62..=0x0C63 | 0x0C81 | 0x0CBC | 0x0CBF | 0x0CC6 | 0x0CCC..=0x0CCD
        | 0x0CE2..=0x0CE3 | 0x0D00..=0x0D01 | 0x0D3B..=0x0D3C | 0x0D41..=0x0D44
        | 0x0D4D | 0x0D62..=0x0D63 | 0x0D81 | 0x0DCA | 0x0DD2..=0x0DD4 | 0x0DD6
        | 0x0E31 | 0x0E34..=0x0E3A | 0x0E47..=0x0E4E | 0x0EB1 | 0x0EB4..=0x0EBC
        | 0x0EC8..=0x0ECD | 0x0F18..=0x0F19 | 0x0F35 | 0x0F37 | 0x0F39
        | 0x0F71..=0x0F7E | 0x0F80..=0x0F84 | 0x0F86..=0x0F87 | 0x0F8D..=0x0F97
        | 0x0F99..=0x0FBC | 0x0FC6 | 0x102B..=0x103E | 0x1056..=0x1059
        | 0x105E..=0x1060 | 0x1062..=0x1064 | 0x1067..=0x106D | 0x1071..=0x1074
        | 0x1082..=0x108D | 0x108F | 0x109A..=0x109D | 0x135D..=0x135F
        | 0x1712..=0x1714 | 0x1732..=0x1734 | 0x1752..=0x1753 | 0x1772..=0x1773
        | 0x17B4..=0x17B5 | 0x17B7..=0x17BD | 0x17C6 | 0x17C9..=0x17D3 | 0x17DD
        | 0x180B..=0x180D | 0x180F | 0x1885..=0x1886 | 0x18A9 | 0x1920..=0x192B
        | 0x1930..=0x193B | 0x1A17..=0x1A1B | 0x1A55 | 0x1A60..=0x1A6C | 0x1A7F
        | 0x1AB0..=0x1AC0 | 0x1B00..=0x1B03 | 0x1B34 | 0x1B36..=0x1B3A | 0x1B3C
        | 0x1B42 | 0x1B6B..=0x1B73 | 0x1B80..=0x1B81 | 0x1BA2..=0x1BA5
        | 0x1BA8..=0x1BA9 | 0x1BAB..=0x1BAD | 0x1BE6 | 0x1BE8..=0x1BE9 | 0x1BED
        | 0x1BEF..=0x1BF1 | 0x1C2C..=0x1C33 | 0x1C36..=0x1C37 | 0x1CD0..=0x1CD2
        | 0x1CD4..=0x1CE0 | 0x1CE2..=0x1CE8 | 0x1CED | 0x1CF4 | 0x1CF8..=0x1CF9
        | 0x1DC0..=0x1DFF | 0x20D0..=0x20F0 | 0x2CEF..=0x2CF1 | 0x2D7F
        | 0x2DE0..=0x2DFF | 0x302A..=0x302D | 0x3099..=0x309A | 0xA66F..=0xA672
        | 0xA674..=0xA67D | 0xA69E..=0xA69F | 0xA6F0..=0xA6F1 | 0xA802 | 0xA806
        | 0xA80B | 0xA823..=0xA824 | 0xA827 | 0xA880..=0xA881 | 0xA8B4..=0xA8C5
        | 0xA8E0..=0xA8F1 | 0xA8FF..=0xA909 | 0xA926..=0xA92D | 0xA947..=0xA953
        | 0xA980..=0xA983 | 0xA9B3 | 0xA9B6..=0xA9B9 | 0xA9BC..=0xA9BD | 0xA9E5
        | 0xAA29..=0xAA2E | 0xAA31..=0xAA32 | 0xAA35..=0xAA36 | 0xAA43 | 0xAA4C
        | 0xAA7C | 0xAAB0 | 0xAAB2..=0xAAB4 | 0xAAB7..=0xAAB8 | 0xAABE..=0xAABF
        | 0xAAC1 | 0xAAEC..=0xAAED | 0xAAF6 | 0xABE5 | 0xABE8 | 0xABED | 0xFB1E
        | 0xFF9E..=0xFF9F | 0x101FD | 0x102E0 | 0x10376..=0x1037A
        | 0x10A01..=0x10A03 | 0x10A05..=0x10A06 | 0x10A0C..=0x10A0F
        | 0x10A38..=0x10A3A | 0x10A3F | 0x10AE5..=0x10AE6 | 0x10D24..=0x10D27
        | 0x10EAB..=0x10EAC | 0x10F46..=0x10F50 | 0x11001 | 0x11038..=0x11046
        | 0x1107F..=0x11081 | 0x110B3..=0x110B6 | 0x110B9..=0x110BA
        | 0x11100..=0x11102 | 0x11127..=0x1112B | 0x1112D..=0x11134 | 0x11173
        | 0x11180..=0x11181 | 0x111B6..=0x111BE | 0x111C9..=0x111CC | 0x111CF
        | 0x1122F..=0x11231 | 0x11234 | 0x11236..=0x11237 | 0x1123E | 0x112DF
        | 0x112E3..=0x112EA | 0x11300..=0x11301 | 0x1133B..=0x1133C | 0x11340
        | 0x11366..=0x1136C | 0x11370..=0x11374 | 0x11438..=0x1143F
        | 0x11442..=0x11444 | 0x11446 | 0x1145E | 0x114B3..=0x114B8 | 0x114BA
        | 0x114BF..=0x114C0 | 0x114C2..=0x114C3 | 0x115B2..=0x115B5
        | 0x115BC..=0x115BD | 0x115BF..=0x115C0 | 0x115DC..=0x115DD
        | 0x11633..=0x1163A | 0x1163D | 0x1163F..=0x11640 | 0x116AB | 0x116AD
        | 0x116B0..=0x116B5 | 0x116B7 | 0x1171D..=0x1171F | 0x11722..=0x11725
        | 0x11727..=0x1172B | 0x1182F..=0x11837 | 0x11839..=0x1183A
        | 0x1193B..=0x1193C | 0x1193E | 0x11943 | 0x119D4..=0x119D7
        | 0x119DA..=0x119DB | 0x119E0 | 0x11A01..=0x11A0A | 0x11A33..=0x11A38
        | 0x11A3B..=0x11A3E | 0x11A47 | 0x11A51..=0x11A56 | 0x11A59..=0x11A5B
        | 0x11A8A..=0x11A96 | 0x11A98..=0x11A99 | 0x11C30..=0x11C36
        | 0x11C38..=0x11C3D | 0x11C3F | 0x11C92..=0x11CA7 | 0x11CAA..=0x11CB0
        | 0x11CB2..=0x11CB3 | 0x11CB5..=0x11CB6 | 0x11D31..=0x11D36 | 0x11D3A
        | 0x11D3C..=0x11D3D | 0x11D3F..=0x11D45 | 0x11D47 | 0x11D90..=0x11D91
        | 0x11D95 | 0x11D97 | 0x11EF3..=0x11EF4 | 0x16AF0..=0x16AF4
        | 0x16B30..=0x16B36 | 0x16F4F | 0x16F8F..=0x16F92 | 0x16FE4
        | 0x1BC9D..=0x1BC9E | 0x1CF00..=0x1CF2D | 0x1CF30..=0x1CF46
        | 0x1D165..=0x1D169 | 0x1D16D..=0x1D182 | 0x1D185..=0x1D18B
        | 0x1D1AA..=0x1D1AD | 0x1D242..=0x1D244 | 0x1DA00..=0x1DA36
        | 0x1DA3B..=0x1DA6C | 0x1DA75 | 0x1DA84 | 0x1DA9B..=0x1DA9F
        | 0x1DAA1..=0x1DAAF | 0x1E000..=0x1E006 | 0x1E008..=0x1E018
        | 0x1E01B..=0x1E021 | 0x1E023..=0x1E024 | 0x1E026..=0x1E02A
        | 0x1E130..=0x1E136 | 0x1E2EC..=0x1E2EF | 0x1E8D0..=0x1E8D6
        | 0x1E944..=0x1E94A | 0xE0100..=0xE01EF)
}

/// Wide code points: CJK ideographs + kana + hangul + fullwidth forms +
/// emoji / pictographs. Box-drawing and most kaomoji glyphs stay narrow
/// (the way Windows Terminal renders them by default).
fn is_wide(cp: u32) -> bool {
    matches!(cp,
        0x1100..=0x115F | 0x3130..=0x318F | 0xAC00..=0xD7A3
        | 0x2E80..=0x303F | 0x3040..=0x30FF | 0x3100..=0x312F
        | 0x31A0..=0x31BF | 0x31F0..=0x31FF | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF | 0xA000..=0xA48F | 0xA490..=0xA4CF
        | 0xF900..=0xFAFF | 0xFE10..=0xFE1F | 0xFE30..=0xFE4F
        | 0xFE50..=0xFE6F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6
        | 0x20000..=0x2FFFD | 0x30000..=0x3FFFD
        | 0x1F300..=0x1F64F | 0x1F680..=0x1F6FF | 0x1F900..=0x1F9FF
        | 0x1FA70..=0x1FAFF
        | 0x231A..=0x231B | 0x23E9..=0x23EC | 0x23F0 | 0x23F3
        | 0x25FD..=0x25FE | 0x2614..=0x2615 | 0x2648..=0x2653 | 0x267F
        | 0x2693 | 0x26A1 | 0x26AA..=0x26AB | 0x26BD..=0x26BE
        | 0x26C4..=0x26C5 | 0x26CE | 0x26D4 | 0x26EA | 0x26F2..=0x26F3
        | 0x26F5 | 0x26FA | 0x26FD | 0x2705 | 0x270A..=0x270B | 0x2728
        | 0x274C | 0x274E | 0x2753..=0x2755 | 0x2757 | 0x2795..=0x2797
        | 0x27B0 | 0x27BF | 0x2B1B..=0x2B1C | 0x2B50 | 0x2B55)
}

/// Total visible cells of a possibly-ANSI-colored string.
pub(crate) fn visible_cells(s: &str) -> usize {
    let mut cells = 0usize;
    let mut in_ansi = false;
    for c in s.chars() {
        if c == '\x1B' {
            in_ansi = true;
            continue;
        }
        if in_ansi {
            if c == 'm' || c == '\x07' {
                in_ansi = false;
            }
            continue;
        }
        cells += char_cells(c);
    }
    cells
}

/// True when `s` ends inside an incomplete ANSI escape (a producer split
/// an escape across chunks). Wrap math can't count split escapes, so the
/// caller passes such a chunk through untouched instead.
fn ansi_open(s: &str) -> bool {
    let mut in_ansi = false;
    for c in s.chars() {
        if c == '\x1B' {
            in_ansi = true;
        } else if in_ansi && (c == 'm' || c == '\x07') {
            in_ansi = false;
        }
    }
    in_ansi
}

/// Truncate plain (unstyled) text to `width` cells, never splitting a
/// UTF-8 char or a zero-width mark from the glyph before it.
pub(crate) fn truncate_cells(s: &str, width: usize) -> String {
    let mut out = String::new();
    let mut cells = 0usize;
    for c in s.chars() {
        let w = char_cells(c);
        if w == 0 {
            if cells < width {
                out.push(c);
            }
            continue;
        }
        if cells + w > width {
            break;
        }
        out.push(c);
        cells += w;
    }
    out
}

/// Truncate a styled line to `width` terminal cells for region redraws, so
/// a long line can't wrap onto the next physical row and desync the layout.
/// Complete ANSI sequences are preserved (incomplete ones dropped) and an
/// SGR reset is appended whenever anything is cut, so the next redraw row
/// starts with clean colors. Zero-width marks (combining, ZWJ, variation
/// selectors) stay glued to the glyph before them — a kaomoji never gets
/// chopped mid-grapheme.
fn truncate_visible(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut cells = 0usize;
    let mut cut = false;
    let mut in_ansi = false;
    let mut ansi = String::new();
    for c in s.chars() {
        if c == '\x1B' {
            in_ansi = true;
            ansi.push(c);
            continue;
        }
        if in_ansi {
            ansi.push(c);
            if c == 'm' || c == '\x07' {
                if !cut {
                    out.push_str(&ansi);
                }
                ansi.clear();
                in_ansi = false;
            }
            continue;
        }
        if cut {
            continue;
        }
        let w = char_cells(c);
        if w == 0 {
            // keep zero-width marks attached to the previous glyph even
            // when the cut lands right after it
            out.push(c);
            continue;
        }
        if cells + w > width {
            cut = true;
            out.push_str("\x1B[0m");
            continue;
        }
        out.push(c);
        cells += w;
    }
    out
}

/// Wrap a possibly-ANSI-colored string so no line exceeds `width` cells.
/// Existing newlines are kept; new ones are inserted at cell boundaries —
/// never inside an ANSI escape or between a glyph and its zero-width marks —
/// so the scrollback buffer's line accounting always matches the physical
/// rows the terminal actually paints. `carry` is the cell count already on
/// the current (incomplete) line from previous chunks.
fn wrap_cells(text: &str, width: usize, carry: usize) -> String {
    if width == 0 {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut cells = carry;
    let mut in_ansi = false;
    let mut ansi = String::new();
    for c in text.chars() {
        if c == '\x1B' {
            in_ansi = true;
            ansi.push(c);
            continue;
        }
        if in_ansi {
            ansi.push(c);
            if c == 'm' || c == '\x07' {
                out.push_str(&ansi);
                ansi.clear();
                in_ansi = false;
            }
            continue;
        }
        if c == '\n' {
            out.push(c);
            cells = 0;
            continue;
        }
        let w = char_cells(c);
        if w == 0 {
            out.push(c);
            continue;
        }
        if cells + w > width {
            out.push('\n');
            cells = 0;
        }
        out.push(c);
        cells += w;
    }
    out
}

/// Left-align `s` in a field of `width` cells, ANSI-aware and wide-char
/// safe — the way `{:<width$}` should have been.
fn pad_cells(s: &str, width: usize) -> String {
    let pad = width.saturating_sub(visible_cells(s));
    format!("{s}{}", " ".repeat(pad))
}

fn tool_box_line(content: &str, color: Role) -> String {
    let t = truncate_cells(content, tool_box_width());
    // opencode renders tool/file content in syntax colors, not flat dim
    let highlighted = syntax::highlight_line(&t);
    let pad = tool_box_width().saturating_sub(visible_cells(&highlighted));
    let padded = format!("{highlighted}{}", " ".repeat(pad));
    format!("  {}{}{}",
        theme::paint(color, "│"),
        theme::bg_fill(Role::CodeBg, padded),
        theme::paint(color, "│"))
}

pub fn show_tool_ok(name: &str, msg: &str) {
    let w = tool_box_width();
    let title = format!(" {} ", name);
    let dashes = w.saturating_sub(visible_cells(&title));
    convo_line(&format!("  {}",
        theme::paint(Role::Dim, format!("┌{}{}┐", title, "─".repeat(dashes)))));
    let content: Vec<&str> = msg.lines().collect();
    let shown = content.len().min(8);
    for line in content.iter().take(shown) {
        convo_line(&tool_box_line(line, Role::Dim));
    }
    if content.len() > shown {
        convo_line(&tool_box_line(&format!("… +{} more lines", content.len() - shown), Role::Dim));
    }
    convo_line(&format!("  {}",
        theme::paint(Role::Dim, format!("└{}┘", "─".repeat(w)))));
}

pub fn show_tool_err(name: &str, msg: &str) {
    let w = tool_box_width();
    let title = format!(" {} ", name);
    let dashes = w.saturating_sub(visible_cells(&title));
    convo_line(&format!("  {}",
        theme::paint(Role::Error, format!("┌{}{}┐", title, "─".repeat(dashes)))));
    convo_line(&tool_box_line(msg, Role::Error));
    convo_line(&format!("  {}",
        theme::paint(Role::Error, format!("└{}┘", "─".repeat(w)))));
}

// ── system messages ───────────────────────────────────────

pub fn show_system(msg: &str) {
    convo_line(&format!("  {} {}",
        theme::paint(Role::Accent, "◆"),
        theme::paint(Role::Accent, msg)));
}

pub fn show_error(msg: &str) {
    convo_line(&format!("  {} {}",
        theme::paint(Role::Error, "✖"),
        theme::paint(Role::Error, msg)));
}

/// Echo the user's message opencode-style: a solid pink accent bar, then the
/// message text on a raised panel, wrapped at a readable width.
pub fn show_user_msg(msg: &str) {
    convo_write(&format_user_msg(msg));
}

const USER_CARD_WIDTH: usize = 84;

/// Card content width shrunk to fit the terminal (the designed 84 plus the
/// indent + bar + padding needs 89 cells — too wide for an 80-col window).
fn user_card_width() -> usize {
    USER_CARD_WIDTH.min(terminal_cols().saturating_sub(5).max(20))
}

fn user_card_line_w(content: &str, content_w: usize) -> String {
    let bar = theme::bg(Role::Primary, " ");
    let body = theme::bg(Role::CodeBg, format!(" {} ", pad_cells(content, content_w)));
    format!("  {}{}", bar, body)
}

/// Pure formatter for the user message card (testable, no stdout).
fn format_user_msg(msg: &str) -> String {
    format_user_msg_w(msg, user_card_width())
}

/// Pure wrap at an explicit card content width in cells. Line 1's "you ▸ "
/// marker is carved out of its own budget; long unbroken tokens hard-wrap
/// at the edge so nothing overflows the panel.
fn format_user_msg_w(msg: &str, content_w: usize) -> String {
    let wrap = content_w.saturating_sub(7); // room for the "you ▸ " marker
    let first = theme::bold(Role::Primary, "you");
    let mut parts: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut cells = 0usize;
    let mut is_first = true;
    for c in msg.chars() {
        let budget = if is_first { wrap } else { content_w };
        let w = char_cells(c);
        if !line.is_empty() && cells + w > budget {
            // prefer the last space so prose breaks on word boundaries
            if let Some(k) = line.rfind(' ') {
                let (head, tail) = line.split_at(k);
                if !head.trim().is_empty() {
                    parts.push(head.trim_end().to_string());
                }
                line = tail.trim_start().to_string();
                cells = visible_cells(&line);
            } else {
                parts.push(line.trim_end().to_string());
                line.clear();
                cells = 0;
            }
            is_first = false;
        }
        line.push(c);
        cells += w;
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
            out.push_str(&user_card_line_w(&format!("{} ▸ {}", first, p), content_w));
        } else {
            out.push_str(&user_card_line_w(p, content_w));
        }
        out.push('\n');
    }
    out
}

/// Thin dim separator between turns, opencode-style.
pub fn show_turn_separator() {
    convo_line(&format!("  {}",
        theme::paint(Role::Primary, "─".repeat(38))));
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
    convo_line(&format!("  {} {}",
        theme::bold(Role::Primary, "▪"),
        theme::bold(Role::Text, model)));
}

pub fn show_interrupted() {
    convo_line(&format!("  {}",
        theme::bold(Role::Warning, "⏹  interrupted")));
}

// ── prompt ─────────────────────────────────────────────────

fn prompt_chars() -> String {
    format!("  {} {} ",
        theme::bg(Role::Primary, " "),
        theme::bold(Role::Primary, "$"))
}

/// Cells the prompt occupies on its row (indent + bar + spaces + "$").
fn prompt_cells() -> usize {
    visible_cells(&prompt_chars())
}

/// How many cells the input row can hold before typed text would wrap past
/// the right edge and scroll the whole screen out of the dock. The input
/// thread caps its buffer here.
pub fn input_capacity() -> usize {
    let cols = terminal_cols();
    cols.saturating_sub(prompt_cells() + 2).max(8)
}

/// Clear the input row, draw the prompt (docked or inline) and echo `buf`.
/// Used whenever the buffer is rewritten (tab completion) so wide chars
/// never leave stray cells behind.
pub fn redraw_prompt_with(buf: &str) {
    let shown = truncate_cells(buf, input_capacity());
    if dock_active() {
        dock_prompt();
    } else {
        print!("\r\x1B[2K{}", prompt_chars());
    }
    print!("{}", shown);
    stdout().flush().ok();
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
/// Lines are clipped to the terminal width so a narrow window can't wrap
/// the banner and shove the docked layout out of place.
pub fn dock_draw_banner() {
    if dock_geometry().is_none() {
        return;
    }
    let cols = terminal_cols();
    print!("\x1B[1;1H");
    for line in banner_lines() {
        print!("\x1B[2K{}\r\n", truncate_visible(&line, cols));
    }
    stdout().flush().ok();
}

fn dock_status_str() -> String {
    dock_status_slot().read().map(|s| s.clone()).unwrap_or_default()
}

fn render_status(status: &str) -> String {
    let (cols, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let max = (cols as usize).saturating_sub(4);
    let t = truncate_visible(status, max);
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
    convo_sync();
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
/// older content up. Also returns any scrolled-up view to the live bottom.
/// No-op when not docked.
pub fn dock_submit_goto() {
    let Some((_, region_bottom, _, _)) = dock_geometry() else { return; };
    convo_to_bottom();
    print!("\x1B[{};1H", region_bottom);
    stdout().flush().ok();
}

/// Re-establish the dock after the terminal was resized or the screen cleared.
pub fn dock_refresh() {
    dock_redraw_bottom();
    convo_sync();
}

// ── conversation scrollback + scrollbar ────────────────────
// Every line written into the conversation region is captured here (ANSI text,
// newest last) so the user can scroll back with PgUp/PgDn/End even though the
// terminal's own scrollback is frozen by the DECSTBM region. When scrolled up,
// output is buffered and suppressed; returning to the bottom repaints the
// region from the buffer.

const SCROLL_CAP: usize = 5000;

static SCROLL_STATE: std::sync::OnceLock<std::sync::Mutex<ScrollState>> =
    std::sync::OnceLock::new();

#[derive(Default)]
struct ScrollState {
    lines: Vec<String>, // complete lines, newest last
    partial: String,    // current unfinished line (mid-stream)
    offset: usize,      // 0 = live (bottom); N = scrolled up N lines
}

impl ScrollState {
    fn total(&self) -> usize {
        self.lines.len() + usize::from(!self.partial.is_empty())
    }

    /// The i-th display line (0 = oldest), where the partial line is last.
    fn display(&self, i: usize) -> &str {
        if i < self.lines.len() {
            &self.lines[i]
        } else {
            &self.partial
        }
    }
}

fn scroll_state() -> std::sync::MutexGuard<'static, ScrollState> {
    SCROLL_STATE
        .get_or_init(|| std::sync::Mutex::new(ScrollState::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Pure buffer absorb: split `text` on newlines into complete lines, keeping
/// the tail in `partial`. Separated from the stdout path so it's testable.
fn absorb(st: &mut ScrollState, text: &str) {
    for c in text.chars() {
        if c == '\n' {
            st.lines.push(std::mem::take(&mut st.partial));
        } else {
            st.partial.push(c);
        }
    }
    if st.lines.len() > SCROLL_CAP {
        let drop = st.lines.len() - SCROLL_CAP;
        st.lines.drain(0..drop);
    }
}

/// How far the viewport can scroll up (total lines minus region height).
fn scroll_clamp(offset: usize, total: usize, visible: usize) -> usize {
    offset.min(total.saturating_sub(visible))
}

fn thumb_height(total: usize, visible: usize) -> usize {
    (visible.saturating_mul(visible) / total.max(1)).clamp(1, visible.max(1))
}

fn thumb_top(offset: usize, total: usize, visible: usize) -> usize {
    let max_off = total.saturating_sub(visible);
    if max_off == 0 {
        return 0;
    }
    let th = thumb_height(total, visible);
    (((visible - th) as f64 * offset as f64 / max_off as f64).round() as usize)
        .min(visible.saturating_sub(th))
}

/// Write display text (ANSI strings with embedded newlines) into the
/// conversation region. This is the single funnel every conversation line
/// goes through: it wraps long lines at the terminal width, captures the
/// scrollback, and prints straight through while live. Wrapping here (not
/// at the producers) is what keeps the buffer's line accounting identical
/// to the physical rows the terminal paints, so scrollback and redraws can
/// never desync from a long wrapped line.
pub fn convo_write(text: &str) {
    let cols = terminal_cols();
    let mut st = scroll_state();
    let wrapped = if ansi_open(&st.partial) {
        // a producer split an ANSI escape across chunks; cell math can't
        // count it, so pass the chunk through unwrapped
        text.to_string()
    } else {
        let carry = visible_cells(&st.partial);
        wrap_cells(text, cols, carry)
    };
    absorb(&mut st, &wrapped);
    if st.offset == 0 {
        drop(st);
        let normalized = wrapped.replace("\r\n", "\n").replace("\n", "\r\n");
        print!("{}", normalized);
        stdout().flush().ok();
    }
}

/// Write one full line into the conversation region (drop-in for println!).
pub fn convo_line(line: &str) {
    convo_write(&format!("{}\n", line));
}

/// Page up / down: move the viewport by a sensible chunk of the region.
pub fn convo_page_up() {
    let Some((_, region_bottom, _, _)) = dock_geometry() else { return; };
    let page = ((region_bottom as usize).max(1) / 2).max(1);
    let mut st = scroll_state();
    st.offset = st.offset.saturating_add(page);
    drop(st);
    convo_redraw();
}

pub fn convo_page_down() {
    let Some((region_top, region_bottom, _, _)) = dock_geometry() else { return; };
    let page = ((region_bottom as usize).max(1) / 2).max(1);
    let mut st = scroll_state();
    let visible = (region_bottom - region_top + 1) as usize;
    st.offset = scroll_clamp(st.offset.saturating_sub(page), st.total(), visible);
    drop(st);
    convo_redraw();
}

pub fn convo_scroll_top() {
    let Some((region_top, region_bottom, _, _)) = dock_geometry() else { return; };
    let visible = (region_bottom - region_top + 1) as usize;
    let mut st = scroll_state();
    st.offset = scroll_clamp(usize::MAX, st.total(), visible);
    drop(st);
    convo_redraw();
}

/// Return to the live bottom (offset 0) and repaint from the buffer.
pub fn convo_to_bottom() {
    let mut st = scroll_state();
    if st.offset == 0 {
        return;
    }
    st.offset = 0;
    drop(st);
    convo_redraw();
}

/// Re-sync after resize / clear: reflow stale-width wraps, clamp the offset
/// and repaint whatever is on screen (region + scrollbar + status hint) so
/// it matches the buffer.
pub fn convo_sync() {
    let Some((region_top, region_bottom, _, _)) = dock_geometry() else { return; };
    let cols = terminal_cols();
    let visible = (region_bottom - region_top + 1) as usize;
    let mut st = scroll_state();
    reflow_buffer(&mut st, cols);
    let total = st.total();
    st.offset = scroll_clamp(st.offset, total, visible);
    let offset = st.offset;
    drop(st);

    if offset == 0 {
        if total > visible {
            print!("{}", scrollbar_render(region_top, total, 0, visible, cols as u16));
            print!("\x1B[{};1H", region_bottom);
            stdout().flush().ok();
        }
        return;
    }
    convo_redraw();
}

/// Repaint the whole conversation region from the scrollback buffer. When
/// offset == 0 the cursor ends at the bottom so live streaming can resume;
/// when scrolled up the status bar shows a hint and the scrollbar is drawn.
/// The cursor-position escape is always emitted LAST, after the scrollbar,
/// so streaming output appends at the region bottom instead of wherever
/// the scrollbar happened to leave the cursor.
fn convo_redraw() {
    let Some((region_top, region_bottom, status_row, _)) = dock_geometry() else { return; };
    let cols = terminal_cols();
    let visible = (region_bottom - region_top + 1) as usize;
    let mut st = scroll_state();
    let total = st.total();
    st.offset = scroll_clamp(st.offset, total, visible);
    let offset = st.offset;
    let start = total.saturating_sub(offset.saturating_add(visible));

    let mut rendered = String::new();
    let mut row = region_top;
    for _ in 0..visible {
        let idx = start + (row - region_top) as usize;
        let text = if idx < total { truncate_visible(st.display(idx), cols) } else { String::new() };
        rendered.push_str(&format!("\x1B[{};1H\x1B[2K{}", row, text));
        row += 1;
    }
    drop(st);

    if offset == 0 {
        // live: restore the real status bar, overlay the scrollbar, then
        // park the cursor at the region bottom for streaming appends
        rendered.push_str(&format!(
            "\x1B[{};1H\x1B[2K{}", status_row, render_status(&dock_status_str())
        ));
        rendered.push_str(&scrollbar_render(region_top, total, 0, visible, cols as u16));
        rendered.push_str(&format!("\x1B[{};1H", region_bottom));
    } else {
        // scrolled: put a hint on the status bar
        rendered.push_str(&format!(
            "\x1B[{};1H\x1B[2K  {} {}",
            status_row,
            theme::bold(Role::Primary, "◆"),
            theme::paint(Role::Dim, format!("scrolled up {} — pgup/pgdn/home/end to move", offset)),
        ));
        rendered.push_str(&scrollbar_render(region_top, total, offset, visible, cols as u16));
        rendered.push_str(&format!("\x1B[{};1H", region_bottom));
    }
    print!("{}", rendered);
    stdout().flush().ok();
}

/// Build the scrollbar for the rightmost column of the region when the
/// buffer overflows the viewport. Thumb = theme primary, track = dim
/// vertical bar. Pure (returns a string) so callers can order it before
/// their final cursor-position escape.
fn scrollbar_render(region_top: u16, total: usize, offset: usize, visible: usize, cols: u16) -> String {
    if total <= visible || visible == 0 || cols == 0 {
        return String::new();
    }
    let th = thumb_height(total, visible);
    let tt = thumb_top(offset, total, visible);
    let mut rendered = String::new();
    for i in 0..visible {
        let ch = if i >= tt && i < tt + th {
            theme::paint(Role::Primary, "█")
        } else {
            theme::paint(Role::Dim, "│")
        };
        rendered.push_str(&format!("\x1B[{};{}H{}", region_top + i as u16, cols, ch));
    }
    rendered
}

/// Re-wrap every buffered line at the current terminal width so a resize
/// reflows the scrollback instead of leaving stale-width wraps. Only lines
/// that overflow the new width are touched; the rest stay as-is.
fn reflow_buffer(st: &mut ScrollState, cols: usize) {
    let mut changed = false;
    for line in st.lines.iter_mut() {
        if visible_cells(line) > cols {
            *line = wrap_cells(line, cols, 0);
            changed = true;
        }
    }
    if visible_cells(&st.partial) > cols {
        st.partial = wrap_cells(&st.partial, cols, 0);
        changed = true;
    }
    if changed {
        // wrap_cells may have inserted newlines; split them back into
        // single-row buffer lines (partial keeps the unfinished tail)
        let mut flat: Vec<String> = Vec::new();
        for line in st.lines.drain(..) {
            flat.extend(line.split('\n').map(|s| s.to_string()));
        }
        let partial = std::mem::take(&mut st.partial);
        flat.extend(partial.split('\n').map(|s| s.to_string()));
        let last = flat.pop();
        st.lines = flat;
        st.partial = last.unwrap_or_default();
    }
}

// ── centered popup overlay ─────────────────────────────────

const APP_MENU_WIDTH: usize = 68;

/// Full popup box width (incl. rails + the 2-space indent) that fits
/// `cols` terminal columns. Menus clamp the inner field to 24 cells so a
/// tiny window still shows a usable list; APP_MENU_WIDTH (68) is the max.
/// The centered draw derives its width from the SAME formula so the box
/// centers on what it actually renders instead of a fixed 68.
fn menu_box_width(cols: usize) -> usize {
    APP_MENU_WIDTH.min(cols.saturating_sub(4)).max(28)
}

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

/// Centered-overlay geometry (pure, so the math is testable): 1-based
/// (row, col) for a `box_w`-cell box of `clip` lines inside a `cols` x
/// `rows` terminal. `clip` is capped to rows-2 so a scrollable terminal
/// always keeps a 1-row margin top/bottom when there's room.
fn popup_geometry(cols: usize, rows: usize, box_w: usize, clip: usize) -> (usize, usize) {
    let max_rows = rows.saturating_sub(2).max(1);
    let clip = clip.min(max_rows);
    let row = (rows.saturating_sub(clip) / 2).saturating_add(1).max(1);
    let col = (cols.saturating_sub(box_w) / 2).saturating_add(1).max(1);
    (row, col)
}

/// Clear the overlay and redraw `rendered` centered on both axes. The
/// box width is derived from the terminal width (shrinks on narrow
/// windows) and lines beyond the terminal height are clipped so a short
/// window can't overflow the popup.
pub fn draw_popup_centered(rendered: &str, line_count: usize) {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((120, 30));
    let cols = cols as usize;
    let rows = rows as usize;
    let max_rows = rows.saturating_sub(2).max(1);
    let clip = line_count.min(max_rows);
    let clipped: String = rendered.lines().take(clip).collect::<Vec<_>>().join("\n");
    let (row, col) = popup_geometry(cols, rows, menu_box_width(cols), clip);
    print!("\x1B[2J\x1B[{};{}H{}", row, col, clipped);
    stdout().flush().ok();
}

// ── interactive quick switcher (ctrl+p / ctrl+m) ─────────────

/// One row of the quick switcher. Applets and models share the struct;
/// fields a row's `kind` doesn't use are ignored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// Applet name, or model id (e.g. "qwen2.5:7b").
    pub name: String,
    /// Applet description, or the model's provider/backend tag
    /// (e.g. "ollama", "openrouter", "opencode").
    pub desc: String,
    pub kind: MenuItemKind,
    /// Applets: true while the applet is running (●). Models: ignored.
    pub running: bool,
    /// Models: true for the current model (✓ marker). Applets: ignored.
    pub active: bool,
    /// Applets: in-window → "[window]", otherwise "[bg]". Models: ignored.
    pub foreground: bool,
    /// Models only: context-length label (e.g. "128k"), right-aligned.
    pub ctx: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItemKind {
    Applet,
    Model,
}

/// Render one quick-switcher row: a full-width hot-pink bar with black
/// text when selected, otherwise a plain panel. `plain` is the ANSI-free
/// text (the selection bar renders it so nothing overrides the black
/// foreground); `colored` carries per-segment styling (active-marker
/// pink, dim tags). Both are padded to `fw` cells so every line lands at
/// the same width; overflow is clamped cell-aware with "..." before
/// padding so a row can never break the box.
fn menu_row_line(plain: &str, colored: &str, fw: usize, selected: bool, cb: Color) -> String {
    let (plain, colored) = if visible_cells(plain) > fw {
        let t = format!("{}...", truncate_cells(plain, fw.saturating_sub(3)));
        let ct = theme::paint(Role::Text, &t).to_string();
        (t, ct)
    } else {
        (plain.to_string(), colored.to_string())
    };
    if selected {
        let bar = theme::bg(Role::Primary, format!("  {}", pad_cells(&plain, fw))).black();
        format!("  {}{}{}\n",
            theme::paint(Role::Primary, "│").on_color(cb),
            bar,
            theme::paint(Role::Primary, "│").on_color(cb))
    } else {
        format!("  {}\n", theme::bg_fill(Role::CodeBg, format!("{}{}{}",
            theme::paint(Role::Text, "│"),
            format!("  {}", pad_cells(&colored, fw)),
            theme::paint(Role::Text, "│"))))
    }
}

/// Draw the interactive quick switcher (ctrl+p): an "apps" section (real
/// applets, engine pinned first by the caller) and a "models" section
/// share one box, one selection bar and one type-to-filter. Rows render
/// grouped by `kind` — applets, then models — so callers MUST pass
/// `items` already in that order (engine first); `idx` indexes the
/// filtered, grouped sequence. Returns
/// (rendered_string, line_count) for draw_popup_centered.
pub fn draw_launcher_menu(items: &[MenuItem], idx: usize, filter: &str) -> (String, usize) {
    let mut out = String::new();
    let inner_w = 64usize.min(terminal_cols().saturating_sub(8).max(24));
    let fw = inner_w.saturating_sub(2); // content field between the rails
    let title = "quick switcher (ctrl+p)";
    let dashes = inner_w.saturating_sub(1).saturating_sub(title.len());
    let cb = theme::get().color(Role::CodeBg);
    let pop_line = |fg: Role, s: String| -> String {
        theme::paint(fg, s).on_color(cb).to_string()
    };

    // title rail + separator, same raised-panel vocabulary as the applet menu
    out.push_str(&format!("  {}\n",
        pop_line(Role::Primary, format!("┌─{}{}┐", title, "─".repeat(dashes)))));
    out.push_str(&format!("  {}\n",
        pop_line(Role::Dim, format!("│{}│", "─".repeat(inner_w)))));

    // opencode-style section divider: Success label + dim dash run
    let header = |label: &str| -> String {
        let label = truncate_cells(label, inner_w.saturating_sub(3));
        let n = inner_w.saturating_sub(3).saturating_sub(visible_cells(&label));
        format!("  {}\n", theme::bg_fill(Role::CodeBg, format!("{}{}{}{}{}",
            theme::paint(Role::Text, "│"),
            theme::paint(Role::Success, format!("  {}", label)),
            theme::paint(Role::Dim, " "),
            theme::paint(Role::Dim, "─".repeat(n)),
            theme::paint(Role::Text, "│"))))
    };

    let lower = filter.to_lowercase();
    let filtered: Vec<&MenuItem> = items.iter()
        .filter(|it| filter.is_empty()
            || it.name.to_lowercase().contains(&lower)
            || it.desc.to_lowercase().contains(&lower))
        .collect();

    let display_idx = idx.min(filtered.len().saturating_sub(1));

    if filtered.is_empty() {
        let msg = format!("no matches for '{}'", filter);
        out.push_str(&format!("  {}\n",
            pop_line(Role::Dim, format!("│  {}│",
                pad_cells(&truncate_cells(&msg, fw), fw)))));
    } else {
        let apps: Vec<&MenuItem> = filtered.iter().copied()
            .filter(|it| it.kind == MenuItemKind::Applet).collect();
        let models: Vec<&MenuItem> = filtered.iter().copied()
            .filter(|it| it.kind == MenuItemKind::Model).collect();

        let mut row_idx = 0usize;

        if !apps.is_empty() {
            out.push_str(&header("apps"));
            for it in &apps {
                let selected = row_idx == display_idx;
                let cursor = if selected { "►" } else { " " };
                let status = if it.running { "●" } else { "○" };
                let mode_tag = if it.foreground { "[window]" } else { "[bg]" };
                let plain = format!("{} {} {}{}{}",
                    cursor, status,
                    pad_cells(&it.name, 11), pad_cells(&it.desc, 24), pad_cells(mode_tag, 8));
                out.push_str(&menu_row_line(&plain, &plain, fw, selected, cb));
                row_idx += 1;
            }
        }

        if !models.is_empty() {
            out.push_str(&header("models"));
            for it in &models {
                let selected = row_idx == display_idx;
                let cursor = if selected { "►" } else { " " };
                // active marker: ✓ (hot pink) — visually distinct from the
                // plain ●/○ applet running dots; space when not current
                let marker = if it.active { "✓" } else { " " };
                // name-first truncation keeps the tag/ctx columns rigid
                // even for monster ids like nvidia/nemotron-3-...-a12b:free
                let name_shown = if visible_cells(&it.name) > 35 {
                    format!("{}...", truncate_cells(&it.name, 32))
                } else {
                    it.name.clone()
                };
                let tag_shown = truncate_cells(&it.desc, 12);
                let ctx_shown = it.ctx.as_deref()
                    .map(|c| truncate_cells(c, 8)).unwrap_or_default();
                let pad = " ".repeat(8 - visible_cells(&ctx_shown));
                let plain = format!("{} {} {}{} {}{}{}",
                    cursor, marker,
                    pad_cells(&name_shown, 30), " ",
                    pad_cells(&tag_shown, 12), pad, ctx_shown);
                let colored = format!("{}{}{}{}{}{}{}{}",
                    theme::paint(Role::Text, cursor),
                    theme::paint(Role::Text, " "),
                    theme::paint(if it.active { Role::Primary } else { Role::Text }, marker),
                    theme::paint(Role::Text, " "),
                    theme::paint(Role::Text, pad_cells(&name_shown, 30)),
                    theme::paint(Role::Text, " "),
                    theme::paint(Role::Dim, pad_cells(&tag_shown, 12)),
                    theme::paint(Role::Dim, format!("{}{}", pad, ctx_shown)));
                out.push_str(&menu_row_line(&plain, &colored, fw, selected, cb));
                row_idx += 1;
            }
        }
    }

    // footer: separator, filter line, hints, bottom rail
    out.push_str(&format!("  {}\n",
        pop_line(Role::Dim, format!("│{}│", "─".repeat(inner_w)))));
    let filter_display = if filter.is_empty() {
        "type to filter...".to_string()
    } else {
        format!("filter: {}", filter)
    };
    out.push_str(&format!("  {}\n",
        pop_line(Role::Dim, format!("│  {}│",
            pad_cells(&truncate_cells(&filter_display, fw), fw)))));
    out.push_str(&format!("  {}\n",
        pop_line(Role::Dim, format!("│  {}│",
            pad_cells(&truncate_cells(
                "↑↓ nav  enter open  x stop  esc back  ● running  ✓ current", fw), fw)))));
    out.push_str(&format!("  {}\n",
        pop_line(Role::Primary, format!("└{}┘", "─".repeat(inner_w)))));

    let line_count = out.lines().count();
    (out, line_count)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn absorb_splits_newlines_and_keeps_partial() {
        let mut st = ScrollState::default();
        absorb(&mut st, "hello\nworld");
        assert_eq!(st.lines, vec!["hello".to_string()]);
        assert_eq!(st.partial, "world");
        absorb(&mut st, "\nend");
        assert_eq!(st.lines, vec!["hello".to_string(), "world".to_string()]);
        assert_eq!(st.partial, "end");
        // a bare blank line is captured as an empty line
        absorb(&mut st, "\n\n");
        assert_eq!(st.lines, vec!["hello".to_string(), "world".to_string(), "end".to_string(), String::new()]);
        assert_eq!(st.partial, String::new());
    }

    #[test]
    fn absorb_caps_scrollback() {
        let mut st = ScrollState::default();
        for i in 0..(SCROLL_CAP + 50) {
            absorb(&mut st, &format!("line{}\n", i));
        }
        assert_eq!(st.lines.len(), SCROLL_CAP);
        assert_eq!(st.lines.first().unwrap(), "line50");
    }

    #[test]
    fn scroll_clamp_never_exceeds_top() {
        assert_eq!(scroll_clamp(0, 10, 5), 0);
        assert_eq!(scroll_clamp(100, 10, 5), 5);
        assert_eq!(scroll_clamp(3, 3, 5), 0);
        assert_eq!(scroll_clamp(4, 10, 5), 4);
    }

    #[test]
    fn thumb_tracks_offset_position() {
        // 20 lines, 5 visible → thumb 1 tall; offset 0 → top 0, offset max → bottom
        let th = thumb_height(20, 5);
        assert_eq!(th, 1);
        assert_eq!(thumb_top(0, 20, 5), 0);
        assert_eq!(thumb_top(15, 20, 5), 4);
        // exact-fit content: no scrolling possible
        assert_eq!(thumb_top(0, 5, 5), 0);
        assert_eq!(thumb_height(5, 5), 5);
    }

    #[test]
    fn truncate_visible_caps_width_and_keeps_ansi() {
        let styled = format!("{}", "abcdefghij".bright_red());
        let t = truncate_visible(&styled, 4);
        assert_eq!(strip_ansi(&t), "abcd");
        assert!(t.contains("\x1B["), "ansi styling lost");
        // ansi sequence after the cutoff is still emitted for resets
        let t2 = truncate_visible(&format!("{}", "abcdef".bright_red()), 2);
        assert!(t2.contains("\x1B[0m") || t2.contains("\x1B[39m") || t2.contains("\x1B[m"));
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

    // ── width / truncation / wrap math (headless: pure functions only) ──

    #[test]
    fn char_cells_counts_kaomoji_widths() {
        // the table-flip: box glyphs + ° are narrow, ︵ (CJK form) is wide
        assert_eq!(char_cells('╯'), 1);
        assert_eq!(char_cells('°'), 1);
        assert_eq!(char_cells('□'), 1);
        assert_eq!(char_cells('︵'), 2);
        assert_eq!(visible_cells("(╯°□°)╯︵ ┻━┻"), 13);
        // (◕‿◕✿) is six narrow glyphs (dingbats stay narrow by default)
        assert_eq!(visible_cells("(◕‿◕✿)"), 6);
        // (๑•蔷•๑): Thai digits are narrow, only the CJK ideograph is wide
        assert_eq!(visible_cells("(๑•蔷•๑)"), 8);
        // combining marks and joiners take no cells
        assert_eq!(visible_cells("e\u{301}"), 1);
        assert_eq!(visible_cells("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"), 6);
    }

    #[test]
    fn truncate_visible_never_splits_wide_char() {
        // ︵ is 2 cells; a 7-cell budget cuts BEFORE it, not through it
        let t = truncate_visible("(╯°□°)╯︵ ┻━┻", 7);
        assert_eq!(strip_ansi(&t), "(╯°□°)╯");
        // and a 10-cell budget lands right after it
        let t2 = truncate_visible("(╯°□°)╯︵ ┻━┻", 10);
        assert_eq!(strip_ansi(&t2), "(╯°□°)╯︵ ");
        // every produced line is within budget (cells, not chars)
        let t3 = truncate_visible("(๑•蔷•๑) (◕‿◕✿)", 9);
        assert!(visible_cells(&strip_ansi(&t3)) <= 9);
        assert!(visible_cells(&strip_ansi(&t3)) >= 8, "wasted the whole budget");
    }

    #[test]
    fn truncate_visible_keeps_zero_width_marks_glued() {
        // e + combining acute must not be torn apart at the cutoff
        let t = truncate_visible("a\u{301}bcdef", 2);
        assert_eq!(strip_ansi(&t), "a\u{301}b");
        // a variation selector right after an included glyph stays with it
        let t2 = truncate_visible("x\u{FE0F}yz", 1);
        assert_eq!(strip_ansi(&t2), "x\u{FE0F}");
        // ...and one that trails past the cutoff is dropped with the cut
        let t3 = truncate_visible("xyz\u{FE0F}", 2);
        assert_eq!(strip_ansi(&t3), "xy");
    }

    #[test]
    fn truncate_visible_appends_reset_when_cut() {
        // cut inside a styled run must leave a reset so the next redraw
        // row starts with clean colors
        let styled = format!("{}", "abcdef".bright_red());
        let t = truncate_visible(&styled, 3);
        assert_eq!(strip_ansi(&t), "abc");
        assert!(t.contains("\x1B[0m"), "missing reset after cut: {t:?}");
        // no cut → the string's own reset suffix is preserved untouched,
        // but no spurious one is appended
        let t2 = truncate_visible(&styled, 99);
        assert_eq!(strip_ansi(&t2), "abcdef");
        assert_eq!(t2.matches("\x1B[0m").count(), 1, "spurious reset: {t2:?}");
    }

    #[test]
    fn truncate_visible_drops_incomplete_ansi() {
        // a split/partial escape at the end must not leak into the row
        let t = truncate_visible("ab\x1B[38;2;255;97", 10);
        assert_eq!(strip_ansi(&t), "ab");
    }

    #[test]
    fn wrap_cells_breaks_at_exact_cell_boundary() {
        assert_eq!(wrap_cells("aaaaaaaaaa", 5, 0), "aaaaa\naaaaa");
        // existing newlines are respected
        assert_eq!(wrap_cells("ab\ncd", 2, 0), "ab\ncd");
    }

    #[test]
    fn wrap_cells_respects_carry_from_partial_chunk() {
        // 3 cells already on the line, budget 5 → only 2 more fit
        assert_eq!(wrap_cells("abc", 5, 3), "ab\nc");
        // zero carry: full budget available
        assert_eq!(wrap_cells("abc", 5, 0), "abc");
    }

    #[test]
    fn wrap_cells_never_splits_wide_char_or_ansi() {
        // wide char that doesn't fit moves to the next line whole
        assert_eq!(wrap_cells("x蔷y", 3, 0), "x蔷\ny");
        // styled text wraps with escapes intact and every line ≤ budget
        let styled = format!("{}", "hello world".bright_red());
        let wrapped = wrap_cells(&styled, 6, 0);
        let plain_lines: Vec<String> = wrapped.split('\n').map(strip_ansi).collect();
        assert!(plain_lines.iter().all(|l| visible_cells(l) <= 6));
        for line in wrapped.split('\n') {
            assert!(line.contains("\x1B["), "ansi styling lost in wrap: {line:?}");
        }
    }

    #[test]
    fn ansi_open_detects_split_escapes() {
        assert!(!ansi_open("plain text"));
        assert!(!ansi_open("styled \x1B[38;2;255;97;136mok\x1B[0m"));
        assert!(ansi_open("split \x1B[38;2;255"));
    }

    #[test]
    fn truncate_cells_is_plain_and_cell_aware() {
        assert_eq!(truncate_cells("abcdef", 4), "abcd");
        // ︵ needs 2 cells → cuts before it
        assert_eq!(truncate_cells("(╯°□°)╯︵ ┻━┻", 8), "(╯°□°)╯");
        // zero-width mark kept inside budget
        assert_eq!(truncate_cells("a\u{301}bc", 2), "a\u{301}b");
    }

    #[test]
    fn pad_cells_aligns_wide_chars() {
        let padded = pad_cells("蔷", 4);
        assert_eq!(visible_cells(&padded), 4);
        assert_eq!(padded, "蔷  ");
        let padded2 = pad_cells("(๑•蔷•๑)", 12);
        assert_eq!(visible_cells(&padded2), 12);
    }

    #[test]
    fn format_user_msg_w_never_overflows_content_width() {
        let msg = "desu-ne (๑•蔷•๑) neko-chan (╯°□°)╯︵ ┻━┻ 1234567890";
        let out = strip_ansi(&format_user_msg_w(msg, 20));
        // card row = indent(2) + bar(1) + pad(1) + content + pad(1) → +5
        for line in out.lines() {
            assert!(visible_cells(line) <= 25, "card row overflows: {line:?}");
        }
        // long unbroken token hard-wraps instead of overflowing
        let long = format_user_msg_w(&"x".repeat(50), 20);
        assert!(strip_ansi(&long).lines().count() > 1);
        // every continuation row fits the card
        let long2 = format_user_msg_w(&"x".repeat(50), 20);
        for line in strip_ansi(&long2).lines() {
            assert!(visible_cells(line) <= 25);
        }
    }

    #[test]
    fn reflow_buffer_rewraps_stale_widths() {
        let mut st = ScrollState::default();
        absorb(&mut st, "short\n");
        absorb(&mut st, &"a".repeat(120));
        reflow_buffer(&mut st, 40);
        assert!(st.lines.iter().all(|l| visible_cells(l) <= 40), "stale-width wraps survived reflow");
        // 120 a's at 40/row = 3 full rows; the last one lives in `partial`
        // as the current (unfinished) row, not in `lines` yet
        assert!(st.lines.len() >= 3, "120 cells at 40/row should wrap into 3 buffer rows");
        assert_eq!(st.lines[0], "short", "short lines must be untouched");
        assert!(visible_cells(&st.partial) <= 40, "partial row still stale");
    }

    // ── quick switcher (draw_launcher_menu) ────────────────

    fn applet(name: &str, desc: &str, running: bool, foreground: bool) -> MenuItem {
        MenuItem {
            name: name.to_string(),
            desc: desc.to_string(),
            kind: MenuItemKind::Applet,
            running,
            active: false,
            foreground,
            ctx: None,
        }
    }

    fn model(name: &str, tag: &str, active: bool, ctx: Option<&str>) -> MenuItem {
        MenuItem {
            name: name.to_string(),
            desc: tag.to_string(),
            kind: MenuItemKind::Model,
            running: false,
            active,
            foreground: false,
            ctx: ctx.map(|s| s.to_string()),
        }
    }

    /// Assert every rendered line has the same visible cell count (the
    /// box invariant the applet menu also enforces) and return that width.
    fn assert_lines_aligned(out: &str) -> usize {
        let lines: Vec<&str> = out.lines().collect();
        assert!(!lines.is_empty(), "no lines rendered");
        let widths: Vec<usize> = lines.iter().map(|l| visible_cells(l)).collect();
        let first = widths[0];
        for (line, w) in lines.iter().zip(widths.iter()) {
            assert_eq!(*w, first, "misaligned line ({} cells, want {}): {}", w, first, line);
        }
        first
    }

    #[test]
    fn launcher_lines_align_with_long_models_and_wide_chars() {
        let items = vec![
            applet("engine", "terminal persona host", true, true),
            applet("flora-cli", "scottish flora explorer", false, true),
            applet("hivebeat", "(๑•蔷•๑) music synth", true, false),
            model("nvidia/nemotron-3-super-120b-a12b:free", "openrouter", false, Some("128k")),
            model("✿kaomoji✿ name", "ollama", true, Some("32k")),
            model("opencode/big-pickle", "opencode", false, None),
        ];
        let (out, line_count) = draw_launcher_menu(&items, 0, "");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), line_count);
        let w = assert_lines_aligned(&out);
        assert!(w >= 28, "box below the documented floor: {w}");
        // the long-model truncation and wide-char rows only fully render
        // on a wide-enough box; alignment holds at every width
        if w >= 60 {
            assert!(out.contains("..."), "long model name should ellipsize");
            assert!(strip_ansi(&out).contains("✿kaomoji✿ name"));
        }
    }

    /// Distinct 38;2; foreground escapes in a raw line (order-independent,
    /// race-free across the global theme — other test modules swap it).
    fn fg_codes(line: &str) -> Vec<String> {
        let mut codes = Vec::new();
        let mut rest = line;
        while let Some(i) = rest.find("\x1B[38;2;") {
            let tail = &rest[i + 7..];
            let end = tail.find('m').unwrap_or(tail.len());
            codes.push(tail[..end].to_string());
            rest = tail;
        }
        codes
    }

    #[test]
    fn launcher_has_apps_and_models_headers() {
        let items = vec![
            applet("engine", "terminal persona host", true, true),
            model("qwen2.5:7b", "ollama", true, Some("32k")),
        ];
        let (out, _) = draw_launcher_menu(&items, 0, "");
        let lines: Vec<&str> = out.lines().collect();
        let apps_header = lines.iter().find(|l| strip_ansi(l).contains("apps")).expect("apps header missing");
        let models_header = lines.iter().find(|l| strip_ansi(l).contains("models")).expect("models header missing");
        // a section divider carries Text rail + Success label + Dim dashes:
        // at least 3 distinct fg codes, on the solid CodeBg fill
        for header in [apps_header, models_header] {
            let mut codes = fg_codes(header);
            codes.sort();
            codes.dedup();
            assert!(codes.len() >= 3, "header not Success/dim-styled: {header}");
            assert!(header.contains("48;2;"), "header must sit on the CodeBg fill");
        }
        assert_lines_aligned(&out);
    }

    #[test]
    fn launcher_marks_active_model_and_selection_bar() {
        let items = vec![
            applet("engine", "terminal persona host", true, true),
            applet("flora-cli", "scottish flora explorer", false, true),
            model("qwen2.5:7b", "ollama", true, Some("32k")),
            model("deepseek/deepseek-r1:free", "openrouter", false, Some("64k")),
        ];
        let (out, _) = draw_launcher_menu(&items, 3, ""); // select the last visible row
        let plain = strip_ansi(&out);
        assert!(plain.contains("►"), "selection cursor missing");
        let qwen_line = plain.lines().find(|l| l.contains("qwen2.5:7b")).unwrap();
        assert!(qwen_line.contains("✓"), "active model not marked: {qwen_line}");
        let deepseek_line = plain.lines().find(|l| l.contains("deepseek")).unwrap();
        assert!(deepseek_line.contains("►"), "selected row must carry the cursor");
        assert!(!deepseek_line.contains("✓"), "non-active model must not be marked");
        // the selection bar: black-on-Primary full-width fill (black fg
        // + a solid 48;2; bg on the row that carries the cursor)
        let raw_lines: Vec<&str> = out.lines().collect();
        let bar_line = raw_lines.iter().find(|l| strip_ansi(l).contains("►")).unwrap();
        // colored merges fg+bg into one SGR: the black fg rides as ";30m"
        // on the bar's 48;2; fill
        assert!(bar_line.contains(";30m"), "bar must be black text: {bar_line}");
        assert!(bar_line.contains("48;2;"), "bar must be a solid fill: {bar_line}");

        // no active model → no ✓ on any row (the footer legend still mentions it)
        let items2 = vec![model("qwen2.5:7b", "ollama", false, None)];
        let (out2, _) = draw_launcher_menu(&items2, 0, "");
        let plain2 = strip_ansi(&out2);
        let qwen2 = plain2.lines().find(|l| l.contains("qwen2.5:7b")).unwrap();
        assert!(!qwen2.contains("✓"), "stray active marker on an inactive model");
    }

    #[test]
    fn launcher_filter_matches_apps_and_models() {
        let items = vec![
            applet("engine", "terminal persona host", true, true),
            applet("flora-cli", "scottish flora explorer", false, true),
            model("qwen2.5:7b", "ollama", true, Some("32k")),
            model("opencode/big-pickle", "opencode", false, Some("200k")),
        ];
        // name match in the apps section
        let (out, _) = draw_launcher_menu(&items, 0, "flora");
        let plain = strip_ansi(&out);
        let w = assert_lines_aligned(&out);
        assert!(plain.contains("flora-cli"));
        assert!(!plain.contains("qwen2.5:7b"));
        assert!(plain.contains("apps"), "apps header should remain");
        assert!(!plain.contains("models"), "models header must hide when its section is empty");
        // provider tag match in the models section
        let (out, _) = draw_launcher_menu(&items, 0, "ollama");
        let plain = strip_ansi(&out);
        assert!(plain.contains("qwen2.5:7b"));
        if w >= 60 {
            assert!(plain.contains("ollama"), "tag column should show the provider");
        }
        assert!(!plain.contains("flora-cli"));
        assert!(plain.contains("models"));
        assert!(!plain.contains("apps"), "apps header must hide when its section is empty");
        // model name match
        let (out, _) = draw_launcher_menu(&items, 0, "big-pickle");
        assert!(strip_ansi(&out).contains("opencode/big-pickle"));
        assert_lines_aligned(&out);
    }

    #[test]
    fn launcher_empty_filter_and_no_matches() {
        let items = vec![
            applet("engine", "terminal persona host", true, true),
            model("qwen2.5:7b", "ollama", true, Some("32k")),
        ];
        let (out, _) = draw_launcher_menu(&items, 0, "");
        let plain = strip_ansi(&out);
        assert!(plain.contains("type to filter..."));
        assert!(plain.contains("engine"));
        assert!(plain.contains("qwen2.5:7b"));

        let (out, _) = draw_launcher_menu(&items, 0, "zzz-nope");
        let plain = strip_ansi(&out);
        assert!(plain.contains("no matches for '"), "no-matches row missing");
        assert!(!plain.contains("engine"));
        assert!(!plain.contains("qwen2.5:7b"));
        assert!(!plain.contains("apps") && !plain.contains("models"), "headers must hide with no rows");
        assert_lines_aligned(&out);
    }

    #[test]
    fn launcher_truncates_very_long_names_and_stays_aligned() {
        let items = vec![
            applet("engine", "terminal persona host", true, true),
            model(&"x".repeat(80), "ollama", false, Some("128k")),
            model("✿薔薇✿-flower", "opencode", true, None),
        ];
        let (out, _) = draw_launcher_menu(&items, 1, "");
        let w = assert_lines_aligned(&out);
        assert!(w >= 28, "box below the documented floor: {w}");
        if w >= 60 {
            let plain = strip_ansi(&out);
            let long_line = plain.lines().find(|l| l.contains("ollama")).unwrap();
            assert!(long_line.contains("..."), "long name must ellipsize: {long_line}");
            assert!(long_line.contains("128k"), "right-aligned ctx column must survive: {long_line}");
            assert!(plain.contains("✿薔薇✿-flower"), "wide-char name lost");
        }
    }

    #[test]
    fn launcher_skips_empty_sections() {
        let items = vec![model("qwen2.5:7b", "ollama", true, Some("32k"))];
        let (out, _) = draw_launcher_menu(&items, 0, "");
        let plain = strip_ansi(&out);
        assert!(plain.contains("models"));
        assert!(!plain.contains("apps"), "empty apps section must not render a header");
        assert_lines_aligned(&out);
    }

    #[test]
    fn launcher_clamps_out_of_range_selection() {
        let items = vec![
            applet("engine", "terminal persona host", true, true),
            model("qwen2.5:7b", "ollama", true, Some("32k")),
        ];
        let (out, _) = draw_launcher_menu(&items, 99, "");
        let plain = strip_ansi(&out);
        let selected = plain.lines().find(|l| l.contains("►")).unwrap();
        assert!(selected.contains("qwen2.5:7b"), "selection should clamp to the last visible row");
        assert_lines_aligned(&out);
    }

    #[test]
    fn popup_geometry_centers_both_axes() {
        // 68-cell box on 120 cols → col 27: box 27..94, terminal center 60.5
        assert_eq!(popup_geometry(120, 30, 68, 10), (11, 27));
        // 68-cell box on 80 cols → col 7, dead center
        assert_eq!(popup_geometry(80, 24, 68, 12), (7, 7));
        // narrow window: shrunk box (36) centers on its own width
        assert_eq!(popup_geometry(40, 24, 36, 12), (7, 3));
        // clip is capped to rows-2 so the box keeps a margin
        assert_eq!(popup_geometry(80, 10, 68, 99), (2, 7));
        // tiny terminal clamps instead of underflowing
        assert_eq!(popup_geometry(20, 5, 28, 1), (3, 1));
        // menu box width matches the menus' inner_w floor at every size
        assert_eq!(menu_box_width(30), 28);
        assert_eq!(menu_box_width(40), 36);
        assert_eq!(menu_box_width(80), 68);
        assert_eq!(menu_box_width(120), 68);
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
