use std::collections::HashMap;
use std::path::Path;
use image::{RgbaImage, Rgba};
use anyhow::Result;

use crate::pixel_striker::palette::{Palette, shade_color, blend_colors};
use crate::pixel_striker::character::CharacterSprite;

pub const ANIMATION_FRAMES: &[(&str, usize)] = &[
    ("idle", 2),
    ("walk_left", 4),
    ("walk_right", 4),
    ("hacking_active", 3),
];

#[derive(Clone, Debug)]
pub struct SpriteSheetConfig {
    pub sprite_width: u32,
    pub sprite_height: u32,
    pub pixel_size: u32,
    pub palette: Palette,
    pub dithering: bool,
    pub subpixel_detail: bool,
    pub states: Vec<String>,
}

impl Default for SpriteSheetConfig {
    fn default() -> Self {
        Self {
            sprite_width: 32,
            sprite_height: 32,
            pixel_size: 3,
            palette: Palette::default(),
            dithering: true,
            subpixel_detail: true,
            states: vec![
                "idle".to_string(),
                "walk_left".to_string(),
                "walk_right".to_string(),
                "hacking_active".to_string(),
            ],
        }
    }
}

impl SpriteSheetConfig {
    pub fn from_json_value(v: &serde_json::Value) -> Self {
        let mut config = Self::default();

        if let Some(w) = v.get("sprite_width").and_then(|v| v.as_u64()) {
            config.sprite_width = w as u32;
        }
        if let Some(h) = v.get("sprite_height").and_then(|v| v.as_u64()) {
            config.sprite_height = h as u32;
        }
        if let Some(ps) = v.get("pixel_size").and_then(|v| v.as_u64()) {
            config.pixel_size = ps as u32;
        }
        if let Some(d) = v.get("dithering").and_then(|v| v.as_bool()) {
            config.dithering = d;
        }
        if let Some(sp) = v.get("subpixel_detail").and_then(|v| v.as_bool()) {
            config.subpixel_detail = sp;
        }

        if let Some(pal) = v.get("palette").and_then(|v| v.as_object()) {
            let mut map = HashMap::new();
            for (k, val) in pal {
                map.insert(k.clone(), val.clone());
            }
            config.palette = Palette::from_json_map(&map);
        }

        if let Some(states) = v.get("states").and_then(|v| v.as_array()) {
            config.states = states.iter().filter_map(|s| s.as_str().map(String::from)).collect();
        }

        config
    }

    fn max_frames(&self) -> usize {
        self.states.iter().map(|s| {
            ANIMATION_FRAMES.iter().find(|(name, _)| name == s)
                .map(|(_, frames)| *frames)
                .unwrap_or(2)
        }).max().unwrap_or(2)
    }
}

#[derive(Clone, Debug)]
pub struct SpriteSheetResult {
    pub width: u32,
    pub height: u32,
    pub states: Vec<String>,
    pub total_frames: usize,
}

pub fn render_sprite_sheet(config: &SpriteSheetConfig) -> RgbaImage {
    let cols = config.max_frames() as u32;
    let rows = config.states.len() as u32;
    let sheet_w = cols * config.sprite_width;
    let sheet_h = rows * config.sprite_height;

    let mut img = RgbaImage::new(sheet_w, sheet_h);
    let char_sprite = CharacterSprite;

    for (row_idx, state) in config.states.iter().enumerate() {
        let num_frames = ANIMATION_FRAMES.iter()
            .find(|(name, _)| name == state)
            .map(|(_, frames)| *frames)
            .unwrap_or(2);

        for col in 0..cols {
            let ox = col * config.sprite_width;
            let oy = row_idx as u32 * config.sprite_height;

            let bob = char_sprite.draw(
                &mut img,
                ox, oy,
                &config.palette,
                col % num_frames as u32,
                state,
                config.pixel_size,
            );

            // Dithering: soft shadow under character
            if config.dithering {
                let shadow_c = shade_color(config.palette.pants, 0.3);
                dither_checkerboard(
                    &mut img,
                    ox + 4 * config.pixel_size,
                    oy + 19 * config.pixel_size + bob,
                    6 * config.pixel_size,
                    2 * config.pixel_size,
                    (shadow_c[0], shadow_c[1], shadow_c[2], 120),
                    (shadow_c[0], shadow_c[1], shadow_c[2], 40),
                    0.6,
                );
            }

            // Sub-pixel visor glow (hacking state)
            if config.subpixel_detail && state == "hacking_active" {
                let visor_c = config.palette.visor;
                let glow_intensity = [0.4, 0.8, 1.0][col as usize % 3];
                for gx in 0u32..3 {
                    for gy in 0u32..2 {
                        let px = ox + (5 + gx) * config.pixel_size;
                        let py = oy + (4 + bob + gy) * config.pixel_size;
                        if px < img.width() && py < img.height() {
                            let gc = shade_color(visor_c, 1.0 + glow_intensity * 0.3);
                            let alpha = (180.0 * glow_intensity) as u8;
                            img.put_pixel(px, py, Rgba([gc[0], gc[1], gc[2], alpha]));
                        }
                    }
                }
            }

            // Sub-pixel circuit line highlight on shirt
            if config.subpixel_detail {
                let circuit_c = config.palette.circuit;
                for cx in 2u32..7 {
                    let px = ox + cx * config.pixel_size;
                    let py = oy + (9 + bob) * config.pixel_size;
                    if px < img.width() && py < img.height() {
                        img.put_pixel(px, py, Rgba([circuit_c[0], circuit_c[1], circuit_c[2], 100]));
                    }
                }
            }
        }
    }

    img
}

pub fn render_to_file(config: &SpriteSheetConfig, output_path: &Path) -> Result<SpriteSheetResult> {
    let img = render_sprite_sheet(config);

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    img.save(output_path)?;

    let total_frames: usize = config.states.iter().map(|s| {
        ANIMATION_FRAMES.iter().find(|(name, _)| name == s)
            .map(|(_, frames)| *frames)
            .unwrap_or(2)
    }).sum();

    Ok(SpriteSheetResult {
        width: img.width(),
        height: img.height(),
        states: config.states.clone(),
        total_frames,
    })
}

/// Checkerboard dithering pattern in a rectangular region
pub fn dither_checkerboard(
    img: &mut RgbaImage,
    x: u32, y: u32, w: u32, h: u32,
    c1: (u8, u8, u8, u8),
    c2: (u8, u8, u8, u8),
    density: f64,
) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px >= img.width() || py >= img.height() {
                continue;
            }
            let use_c1 = if density >= 0.5 {
                (dx + dy) % 2 == 0
            } else {
                (dx + dy) % 3 == 0
            };
            let (cr, cg, cb, ca) = if use_c1 { c1 } else { c2 };
            img.put_pixel(px, py, Rgba([cr, cg, cb, ca]));
        }
    }
}

/// Gradient dithering from top color to bottom color
pub fn dither_gradient(
    img: &mut RgbaImage,
    x: u32, y: u32, w: u32, h: u32,
    c_top: [u8; 3],
    c_bot: [u8; 3],
) {
    for dy in 0..h {
        let t = dy as f32 / (h.max(1) - 1) as f32;
        let base = blend_colors(c_top, c_bot, t);
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px >= img.width() || py >= img.height() {
                continue;
            }
            let threshold = ((px % 4) * 64 + (py % 4) * 16) % 256;
            let color = if threshold < (t * 255.0) as u32 {
                blend_colors(base, c_bot, 0.15)
            } else {
                blend_colors(base, c_top, 0.15)
            };
            img.put_pixel(px, py, Rgba([color[0], color[1], color[2], 255]));
        }
    }
}

/// Draw thin circuit-board style lines for tech aesthetic
pub fn draw_circuit_lines(
    img: &mut RgbaImage,
    x: u32, y: u32, w: u32, h: u32,
    color: [u8; 3],
) {
    let spacing = 3u32;
    for i in (0..w).step_by(spacing as usize) {
        for py in y..y + h {
            let px = x + i;
            if px < img.width() && py < img.height() {
                img.put_pixel(px, py, Rgba([color[0], color[1], color[2], 80]));
            }
        }
    }
    for i in (0..h).step_by(spacing as usize) {
        for px in x..x + w {
            let py = y + i;
            if px < img.width() && py < img.height() {
                img.put_pixel(px, py, Rgba([color[0], color[1], color[2], 80]));
            }
        }
    }
}
