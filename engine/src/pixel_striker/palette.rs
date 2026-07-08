use std::collections::HashMap;

pub fn hex_to_rgb(h: &str) -> [u8; 3] {
    let h = h.trim_start_matches('#');
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
    [r, g, b]
}

pub fn shade_color(rgb: [u8; 3], factor: f32) -> [u8; 3] {
    [
        (rgb[0] as f32 * factor).clamp(0.0, 255.0) as u8,
        (rgb[1] as f32 * factor).clamp(0.0, 255.0) as u8,
        (rgb[2] as f32 * factor).clamp(0.0, 255.0) as u8,
    ]
}

pub fn blend_colors(c1: [u8; 3], c2: [u8; 3], t: f32) -> [u8; 3] {
    [
        (c1[0] as f32 + (c2[0] as f32 - c1[0] as f32) * t) as u8,
        (c1[1] as f32 + (c2[1] as f32 - c1[1] as f32) * t) as u8,
        (c1[2] as f32 + (c2[2] as f32 - c1[2] as f32) * t) as u8,
    ]
}

#[derive(Clone, Debug)]
pub struct Palette {
    pub skin: [u8; 3],
    pub hair: [u8; 3],
    pub shirt: [u8; 3],
    pub pants: [u8; 3],
    pub shoes: [u8; 3],
    pub visor: [u8; 3],
    pub circuit: [u8; 3],
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            skin: [255, 204, 153],
            hair: [80, 40, 120],
            shirt: [40, 80, 160],
            pants: [60, 60, 80],
            shoes: [40, 40, 50],
            visor: [0, 240, 255],
            circuit: [0, 200, 180],
        }
    }
}

impl Palette {
    pub fn from_json_map(map: &HashMap<String, serde_json::Value>) -> Self {
        fn get_rgb(map: &HashMap<String, serde_json::Value>, key: &str, default: [u8; 3]) -> [u8; 3] {
            map.get(key).and_then(|v| v.as_str()).map(hex_to_rgb).unwrap_or(default)
        }
        Self {
            skin: get_rgb(map, "skin", [255, 204, 153]),
            hair: get_rgb(map, "hair", [80, 40, 120]),
            shirt: get_rgb(map, "shirt", [40, 80, 160]),
            pants: get_rgb(map, "pants", [60, 60, 80]),
            shoes: get_rgb(map, "shoes", [40, 40, 50]),
            visor: get_rgb(map, "visor", [0, 240, 255]),
            circuit: get_rgb(map, "circuit", [0, 200, 180]),
        }
    }

    pub fn shade(&self, _name: &str) -> [u8; 3] {
        self.skin
    }
}
