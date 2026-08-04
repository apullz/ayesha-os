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
}

fn make(name: &str, hexes: &[&str; ROLE_COUNT]) -> Theme {
    Theme { name: name.to_string(), hexes: hexes.map(|h| h.to_string()) }
}

impl Theme {
    pub fn color(&self, role: Role) -> Color {
        rgb(&self.hexes[role as usize])
    }

    pub fn hex(&self, role: Role) -> &str {
        &self.hexes[role as usize]
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
        _ => None,
    }
}

pub fn preset(name: &str) -> Option<Theme> {
    preset_hexes(name).map(|hexes| make(name, &hexes))
}

pub fn names() -> Vec<&'static str> {
    vec!["kook", "cyberpunk", "sakura", "win95", "mono"]
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
    let mut chosen = active_name.and_then(|n| preset(n));

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
                        .and_then(|n| preset(n));
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

pub fn code_line(text: impl AsRef<str>) -> colored::ColoredString {
    get().code_line(text)
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
}
