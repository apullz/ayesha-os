use image::{RgbaImage, Rgba};
use crate::pixel_striker::palette::{Palette, shade_color};
use crate::pixel_striker::renderer::draw_circuit_lines;

fn put_pixel(img: &mut RgbaImage, x: u32, y: u32, color: [u8; 3]) {
    if x < img.width() && y < img.height() {
        img.put_pixel(x, y, Rgba([color[0], color[1], color[2], 255]));
    }
}

fn add_offset(base: u32, off: i32) -> u32 {
    if off >= 0 { base + off as u32 } else { base.saturating_sub((-off) as u32) }
}

#[derive(Clone, Debug)]
pub struct CharacterSprite;

impl CharacterSprite {
    pub fn draw(
        &self,
        img: &mut RgbaImage,
        ox: u32,
        oy: u32,
        palette: &Palette,
        frame: u32,
        state: &str,
        pixel_size: u32,
    ) -> u32 {
        let s = pixel_size;
        let skin = palette.skin;
        let hair = palette.hair;
        let shirt = palette.shirt;
        let pants = palette.pants;
        let shoes = palette.shoes;
        let visor = palette.visor;

        let skin_s = shade_color(skin, 0.7);
        let skin_h = shade_color(skin, 1.3);
        let hair_s = shade_color(hair, 0.6);
        let hair_h = shade_color(hair, 1.2);
        let shirt_s = shade_color(shirt, 0.65);
        let shirt_h = shade_color(shirt, 1.25);

        let bob: i32;
        let arm_offset: i32;
        let leg_offset: i32;

        match state {
            "idle" => {
                bob = if frame % 2 == 1 { 1 } else { 0 };
                arm_offset = 0;
                leg_offset = 0;
            }
            "walk_left" | "walk_right" => {
                leg_offset = ((frame as i32) % 2) * 2 - 1;
                arm_offset = -leg_offset;
                bob = if frame % 2 == 0 { 1 } else { 0 };
            }
            "hacking_active" => {
                arm_offset = (frame as i32).saturating_sub(1);
                leg_offset = 0;
                bob = 0;
            }
            _ => {
                bob = 0;
                arm_offset = 0;
                leg_offset = 0;
            }
        }

        let bob_u = bob.max(0) as u32;

        // HAIR (top)
        for dx in 0u32..8 {
            put_pixel(img, ox + (1 + dx) * s, add_offset(oy + 1 * s, bob), hair_s);
            put_pixel(img, ox + (1 + dx) * s, add_offset(oy + 2 * s, bob), hair);
        }
        put_pixel(img, ox + 3 * s, add_offset(oy + 1 * s, bob), hair_h);

        // HEAD / SKIN
        for dy in 0u32..4 {
            for dx in 0u32..6 {
                let c = if dx < 5 { skin } else { skin_s };
                put_pixel(img, ox + (2 + dx) * s, add_offset(oy + (3 + dy) * s, bob), c);
            }
        }
        put_pixel(img, ox + 3 * s, add_offset(oy + 3 * s, bob), skin_h);

        // VISOR
        put_pixel(img, ox + 5 * s, add_offset(oy + 4 * s, bob), visor);
        put_pixel(img, ox + 6 * s, add_offset(oy + 4 * s, bob), shade_color(visor, 1.2));
        put_pixel(img, ox + 7 * s, add_offset(oy + 4 * s, bob), visor);
        put_pixel(img, ox + 6 * s, add_offset(oy + 3 * s, bob), shade_color(visor, 0.5));

        // BODY / SHIRT
        for dy in 0u32..5 {
            for dx in 0u32..8 {
                let c = if dx < 6 { shirt } else { shirt_s };
                let c = if dy == 0 { shirt_h } else { c };
                put_pixel(img, ox + (1 + dx) * s, add_offset(oy + (7 + dy) * s, bob), c);
            }
        }

        // circuit lines on shirt (sub-pixel)
        let circuit_y = if bob_u > 0 { oy + 8 * bob_u * s } else { oy + 8 * s };
        draw_circuit_lines(img, ox + 2 * s, circuit_y, 5 * s, 3 * s, palette.circuit);

        // ARMS
        let arm_y = 8i32 + bob;
        for dy in 0u32..4 {
            let left_c = if dy < 2 { skin_s } else { skin };
            let right_c = if dy < 2 { skin_s } else { skin };
            put_pixel(img, ox, add_offset(add_offset(oy, arm_y + dy as i32), arm_offset), left_c);
            put_pixel(img, ox + 9 * s, add_offset(add_offset(oy, arm_y + dy as i32), -arm_offset), right_c);
        }

        // PANTS
        for dy in 0u32..3 {
            for dx in 0u32..6 {
                let c = if dx < 4 { pants } else { shade_color(pants, 0.8) };
                put_pixel(img, ox + (2 + dx) * s, add_offset(oy + (12 + dy) * s, bob), c);
            }
        }

        // LEGS
        let leg_y = 15i32 + bob;
        for dy in 0u32..2 {
            put_pixel(img, ox + 3 * s, add_offset(add_offset(oy, leg_y + dy as i32), leg_offset), pants);
            put_pixel(img, ox + 6 * s, add_offset(add_offset(oy, leg_y + dy as i32), -leg_offset), pants);
        }

        // SHOES
        let shoe_y = 17i32 + bob;
        for dx in 0u32..3 {
            put_pixel(img, ox + (2 + dx) * s, add_offset(add_offset(oy, shoe_y), leg_offset), shoes);
            put_pixel(img, ox + (5 + dx) * s, add_offset(add_offset(oy, shoe_y), -leg_offset), shoes);
        }

        bob_u
    }
}
