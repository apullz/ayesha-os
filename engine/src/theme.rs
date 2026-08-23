// ayesha-os central theming — single source of truth for terminal colors.
// Presets are loaded from the `theme` section of ayesha.json (see
// load_from_config); role->color helpers let every module render in the
// active palette. `kook` is the default theme.
use colored::{Color, Colorize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

pub const ROLE_COUNT: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Role {
    Background = 0,
    Surface = 1,
    Text = 2,
    Primary = 3,
    Accent = 4,
    Secondary = 5,
    Success = 6,
    Warning = 7,
    Error = 8,
    Dim = 9,
    Border = 10,
    CodeBg = 11,
}

impl Role {
    pub const ALL: [Role; ROLE_COUNT] = [
        Role::Background, Role::Surface, Role::Text, Role::Primary,
        Role::Accent, Role::Secondary, Role::Success, Role::Warning,
        Role::Error, Role::Dim, Role::Border, Role::CodeBg,
    ];
}

// ── syntax token roles (used by the code-block renderer) ─────────
// Mirrors the monokai++ ayesha-os theme's `syntax*` tokens so code
// blocks can be colored independently of the chrome roles above.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum SyntaxRole {
    Keyword = 0,
    String = 1,
    Number = 2,
    Comment = 3,
    Function = 4,
    Type = 5,
    Operator = 6,
    Punctuation = 7,
    InlineCode = 8,
}

pub const SYNTAX_ROLE_COUNT: usize = 9;

fn role_from_str(s: &str) -> Option<Role> {
    match s {
        "background" => Some(Role::Background),
        "surface" => Some(Role::Surface),
        "text" => Some(Role::Text),
        "primary" => Some(Role::Primary),
        "accent" => Some(Role::Accent),
        "secondary" => Some(Role::Secondary),
        "success" => Some(Role::Success),
        "warning" => Some(Role::Warning),
        "error" => Some(Role::Error),
        "dim" => Some(Role::Dim),
        "border" => Some(Role::Border),
        "code_bg" => Some(Role::CodeBg),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn role_name(role: Role) -> &'static str {
    match role {
        Role::Background => "background",
        Role::Surface => "surface",
        Role::Text => "text",
        Role::Primary => "primary",
        Role::Accent => "accent",
        Role::Secondary => "secondary",
        Role::Success => "success",
        Role::Warning => "warning",
        Role::Error => "error",
        Role::Dim => "dim",
        Role::Border => "border",
        Role::CodeBg => "code_bg",
    }
}

fn rgb(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 {
        return Color::TrueColor { r: 0, g: 0, b: 0 };
    }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
    Color::TrueColor { r, g, b }
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    hexes: [String; ROLE_COUNT],
    syntax: [String; SYNTAX_ROLE_COUNT],
}

fn make(name: &str, hexes: &[&str; ROLE_COUNT]) -> Theme {
    let syntax = default_syntax_hexes();
    Theme { name: name.to_string(), hexes: hexes.map(|h| h.to_string()), syntax }
}

/// Fallback syntax palette when a preset doesn't ship its own (used by
/// non-monokai presets so code blocks always have distinct token colors).
fn default_syntax_hexes() -> [String; SYNTAX_ROLE_COUNT] {
    [
        "#FF6188".to_string(), // keyword
        "#FFD866".to_string(), // string
        "#AB9DF2".to_string(), // number
        "#6C696E".to_string(), // comment
        "#78DCE8".to_string(), // function
        "#78DCE8".to_string(), // type
        "#FF6188".to_string(), // operator
        "#FCFCFA".to_string(), // punctuation
        "#FFD866".to_string(), // inline code
    ]
}

impl Theme {
    pub fn color(&self, role: Role) -> Color {
        rgb(&self.hexes[role as usize])
    }

    pub fn hex(&self, role: Role) -> &str {
        &self.hexes[role as usize]
    }

    pub fn syntax_color(&self, role: SyntaxRole) -> Color {
        rgb(&self.syntax[role as usize])
    }

    #[allow(dead_code)]
    pub fn syntax_hex(&self, role: SyntaxRole) -> &str {
        &self.syntax[role as usize]
    }

    pub fn paint_syntax(&self, role: SyntaxRole, text: impl AsRef<str>) -> colored::ColoredString {
        text.as_ref().color(self.syntax_color(role))
    }

    pub fn paint(&self, role: Role, text: impl AsRef<str>) -> colored::ColoredString {
        text.as_ref().color(self.color(role))
    }

    pub fn bold(&self, role: Role, text: impl AsRef<str>) -> colored::ColoredString {
        text.as_ref().color(self.color(role)).bold()
    }

    #[allow(dead_code)]
    pub fn bg(&self, role: Role, text: impl AsRef<str>) -> colored::ColoredString {
        text.as_ref().on_color(self.color(role))
    }

    pub fn code_line(&self, text: impl AsRef<str>) -> colored::ColoredString {
        text.as_ref()
            .color(self.color(Role::Text))
            .on_color(self.color(Role::CodeBg))
    }

    pub fn with_overrides(&self, overrides: &HashMap<String, String>) -> Theme {
        let mut t = self.clone();
        for (k, v) in overrides {
            if let Some(role) = role_from_str(k.as_str()) {
                t.hexes[role as usize] = v.clone();
            }
        }
        t
    }

    /// Override a syntax token color by name (e.g. `theme.syntax.keyword`).
    #[allow(dead_code)]
    pub fn with_syntax_override(&self, key: &str, value: &str) -> Theme {
        let mut t = self.clone();
        if let Some(role) = syntax_role_from_str(key) {
            t.syntax[role as usize] = value.to_string();
        }
        t
    }
}

fn syntax_role_from_str(s: &str) -> Option<SyntaxRole> {
    match s {
        "keyword" => Some(SyntaxRole::Keyword),
        "string" => Some(SyntaxRole::String),
        "number" => Some(SyntaxRole::Number),
        "comment" => Some(SyntaxRole::Comment),
        "function" => Some(SyntaxRole::Function),
        "type" => Some(SyntaxRole::Type),
        "operator" => Some(SyntaxRole::Operator),
        "punctuation" => Some(SyntaxRole::Punctuation),
        "inline_code" => Some(SyntaxRole::InlineCode),
        _ => None,
    }
}

fn preset_hexes(name: &str) -> Option<[&'static str; ROLE_COUNT]> {
    match name {
        "kook" => Some([
            "#101014", "#16161C", "#E8E6F0", "#E75E9D", "#D782A7", "#4C5D79",
            "#62C884", "#E0B24E", "#E5536A", "#6A6478", "#3D3E45", "#1A1A22",
        ]),
        "cyberpunk" => Some([
            "#0D0D0D", "#111111", "#E8E8E8", "#55FF55", "#55FFFF", "#FFFF55",
            "#55FF55", "#FFFF55", "#FF5555", "#555555", "#3A3A3A", "#1A1A1A",
        ]),
        "sakura" => Some([
            "#1B0F18", "#241221", "#F5E6F0", "#FF6FA5", "#FFB3D1", "#8E7CC3",
            "#6FCF97", "#E8C17D", "#E5536A", "#8A7084", "#5A3B55", "#2A1826",
        ]),
        "win95" => Some([
            "#000000", "#1A1A1A", "#C0C0C0", "#008080", "#00FFFF", "#0000A0",
            "#008000", "#FFFF00", "#FF0000", "#808080", "#A0A0A0", "#101010",
        ]),
        "mono" => Some([
            "#0A0A0A", "#111111", "#CCCCCC", "#FFFFFF", "#FFFFFF", "#999999",
            "#FFFFFF", "#BBBBBB", "#FF6B6B", "#666666", "#333333", "#181818",
        ]),
        // monokai++ — ported from ~/.config/ayesha-os/themes/monokai++.json
        // background stack matches ayesha-os: bg / bgDeeper / bgSubtle
        "monokai++" | "monokai" => Some([
            "#221F22", "#262226", "#FCFCFA", "#FF6188", "#FF6188", "#78DCE8",
            "#A9DC76", "#FFD866", "#FF5F5F", "#6C696E", "#454147", "#353238",
        ]),
        _ => None,
    }
}

pub fn preset(name: &str) -> Option<Theme> {
    let hexes = preset_hexes(name)?;
    let mut t = make(name, &hexes);
    if name == "monokai++" || name == "monokai" {
        t.syntax = [
            "#FF6188".to_string(), // keyword
            "#A6E22E".to_string(), // string
            "#AB9DF2".to_string(), // number
            "#6C696E".to_string(), // comment
            "#78DCE8".to_string(), // function
            "#78DCE8".to_string(), // type
            "#FF6188".to_string(), // operator
            "#FCFCFA".to_string(), // punctuation
            "#A6E22E".to_string(), // inline code
        ];
    }
    Some(t)
}

pub fn names() -> Vec<&'static str> {
    vec!["kook", "cyberpunk", "sakura", "win95", "mono", "monokai++"]
}

fn default() -> Theme {
    preset("kook").expect("kook preset")
}

static THEME: OnceLock<RwLock<Theme>> = OnceLock::new();

pub fn set_active(t: Theme) {
    if let Some(lock) = THEME.get() {
        *lock.write().expect("theme lock") = t;
    } else {
        let _ = THEME.set(RwLock::new(t));
    }
}

pub fn get() -> Theme {
    match THEME.get() {
        Some(lock) => lock.read().expect("theme lock").clone(),
        None => default(),
    }
}

pub fn switch(name: &str) -> Result<Theme, String> {
    let t = preset(name).ok_or_else(|| {
        format!("unknown theme: {} (try /theme)", name)
    })?;
    set_active(t.clone());
    Ok(t)
}

/// Load the theme from ayesha.json (`theme.name` preset + optional
/// `theme.palette` overrides). `active_name` (from engine config.json) wins
/// over the file default. Falls back to the `kook` preset.
pub fn load_from_config(active_name: Option<&str>) {
    let mut chosen = active_name.and_then(preset);

    let root = find_config_root();
    if let Some(root) = root {
        let path = root.join("ayesha.json");
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                let theme = v.get("theme");
                if chosen.is_none() {
                    chosen = theme
                        .and_then(|t| t.get("name"))
                        .and_then(|n| n.as_str())
                        .and_then(preset);
                }
                if let Some(base) = chosen.clone() {
                    if let Some(pal) = theme.and_then(|t| t.get("palette")).and_then(|p| p.as_object()) {
                        let overrides: HashMap<String, String> = pal.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect();
                        chosen = Some(base.with_overrides(&overrides));
                    }
                }
            }
        }
    }

    set_active(chosen.unwrap_or_else(default));
}

/// Walk up from cwd (then from the exe dir) looking for the repo root that
/// contains ayesha.json, mirroring applet_manager's discovery.
fn find_config_root() -> Option<std::path::PathBuf> {
    let start = std::env::current_dir().ok()?;
    for dir in start.ancestors() {
        if dir.join("ayesha.json").is_file() {
            return Some(dir.to_path_buf());
        }
    }
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for dir in exe_dir.ancestors() {
        if dir.join("ayesha.json").is_file() {
            return Some(dir.to_path_buf());
        }
    }
    None
}

/// ANSI swatch line for a theme (used by /theme list).
pub fn render_swatch(name: &str) -> String {
    let t = preset(name).unwrap_or_else(default);
    let mut out = format!("{:<11}", name);
    for role in Role::ALL {
        out.push_str(&format!("{}", "  ".on_color(t.color(role))));
    }
    out.push_str(&format!("  {}", "".clear()));
    out.push(' ');
    for role in [Role::Primary, Role::Accent, Role::Secondary, Role::Dim] {
        out.push_str(&format!("{} ", t.hex(role)));
    }
    out
}

pub fn apply_no_color() {
    if std::env::var("NO_COLOR").map(|v| !v.is_empty()).unwrap_or(false) {
        colored::control::set_override(false);
    }
}

/// Make the `colored` crate emit 24-bit truecolor escapes. colored 2.x only
/// enables truecolor when it sees `COLORTERM=truecolor` (or `=24bit`) and
/// otherwise snaps every `Color::TrueColor` to the nearest 8-color ANSI —
/// monokai's #FF6188 pink renders as flat magenta. The rest of the UI already
/// assumes truecolor (raw `48;2;` fills, the `\x1b]11;` background OSC), so
/// force it for the crate too. An explicitly-set COLORTERM is left alone.
pub fn force_truecolor() {
    let already = std::env::var("COLORTERM")
        .map(|v| v == "truecolor" || v == "24bit")
        .unwrap_or(false);
    if !already {
        std::env::set_var("COLORTERM", "truecolor");
    }
}

// Convenience wrappers so call sites don't need to pull the Theme object.
pub fn paint(role: Role, text: impl AsRef<str>) -> colored::ColoredString {
    get().paint(role, text)
}

pub fn bold(role: Role, text: impl AsRef<str>) -> colored::ColoredString {
    get().bold(role, text)
}

#[allow(dead_code)]
pub fn bg(role: Role, text: impl AsRef<str>) -> colored::ColoredString {
    get().bg(role, text)
}

/// Wrap a (possibly already ANSI-colored) string so every cell — including
/// after each inner reset — sits on a solid `role` background. Used for
/// popup panels and code blocks where token colors must not punch holes in
/// the fill.
pub fn bg_fill(role: Role, text: impl AsRef<str>) -> String {
    let has_truecolor = std::env::var("COLORTERM")
        .map(|v| v.contains("truecolor") || v.contains("24bit"))
        .unwrap_or(false);
    if !has_truecolor {
        return text.as_ref().to_string(); // no truecolor, skip background fill
    }
    let color = get().color(role);
    let (r, g, b) = match color {
        colored::Color::TrueColor { r, g, b } => (r, g, b),
        _ => (0, 0, 0),
    };
    let bg = format!("\x1b[48;2;{r};{g};{b}m");
    let reset = "\x1b[0m";
    format!("{bg}{}{reset}", text.as_ref().replace(reset, &format!("{reset}{bg}")))
}

pub fn code_line(text: impl AsRef<str>) -> colored::ColoredString {
    get().code_line(text)
}

pub fn paint_syntax(role: SyntaxRole, text: impl AsRef<str>) -> colored::ColoredString {
    get().paint_syntax(role, text)
}

#[allow(dead_code)]
pub fn syntax_hex(role: SyntaxRole) -> String {
    get().syntax_hex(role).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_hexes_are_valid_rgb() {
        for name in names() {
            let t = preset(name).expect("preset exists");
            for role in Role::ALL {
                let hex = t.hex(role);
                assert_eq!(hex.len(), 7, "{name} {role:?} bad hex {hex}");
                assert!(hex.starts_with('#'));
            }
        }
    }

    #[test]
    fn kook_is_default() {
        assert_eq!(default().name, "kook");
    }

    #[test]
    fn unknown_role_string_maps_to_none() {
        assert!(role_from_str("neon").is_none());
        assert_eq!(role_from_str("primary"), Some(Role::Primary));
    }

    #[test]
    fn overrides_replace_hexes() {
        let base = preset("kook").unwrap();
        let mut map = HashMap::new();
        map.insert("primary".to_string(), "#123456".to_string());
        let t = base.with_overrides(&map);
        assert_eq!(t.hex(Role::Primary), "#123456");
        assert_eq!(t.hex(Role::Accent), base.hex(Role::Accent));
    }

    #[test]
    fn rgb_parses_hex() {
        assert_eq!(rgb("#E75E9D"), Color::TrueColor { r: 0xE7, g: 0x5E, b: 0x9D });
    }

    #[test]
    fn monokai_preset_ships_syntax_palette() {
        let t = preset("monokai++").expect("monokai++ preset");
        assert_eq!(t.hex(Role::Primary), "#FF6188");
        assert_eq!(t.hex(Role::Background), "#221F22");
        assert_eq!(t.syntax_hex(SyntaxRole::Keyword), "#FF6188");
        assert_eq!(t.syntax_hex(SyntaxRole::String), "#A6E22E");
        assert_eq!(t.syntax_hex(SyntaxRole::Number), "#AB9DF2");
    }

    #[test]
    fn monokai_alias_resolves() {
        assert_eq!(preset("monokai").expect("alias").name, "monokai");
    }
}
