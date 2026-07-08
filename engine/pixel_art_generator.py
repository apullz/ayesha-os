"""
pixel_art_generator.py - Advanced pixel art sprite sheet generator
Called by ayesha-engine to create sprites with shading, dithering, and animation states.

Usage: python pixel_art_generator.py <config_json_path> <output_png_path>
"""
import sys
import json
import os
from PIL import Image, ImageDraw

# === COLOR UTILS ===

def hex_to_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i+2], 16) for i in (0, 2, 4))

def rgb_to_hex(r, g, b):
    return f"#{r:02x}{g:02x}{b:02x}"

def shade_color(rgb, factor):
    """Shade a color: >1 = lighter, <1 = darker"""
    return tuple(max(0, min(255, int(c * factor))) for c in rgb)

def blend_colors(c1, c2, t):
    """Lerp between two colors"""
    return tuple(int(a + (b - a) * t) for a, b in zip(c1, c2))

def generate_shades(base_rgb, name="color"):
    """Generate 5 shades: shadow_dark, shadow, base, highlight, highlight_bright"""
    return {
        f"{name}_shadow_dark": shade_color(base_rgb, 0.4),
        f"{name}_shadow": shade_color(base_rgb, 0.7),
        f"{name}_base": base_rgb,
        f"{name}_highlight": shade_color(base_rgb, 1.3),
        f"{name}_highlight_bright": shade_color(base_rgb, 1.6),
    }

# === DITHERING ===

def dither_checkerboard(img, region, c1, c2, density=0.5):
    """Apply checkerboard dithering pattern in a region (x, y, w, h)"""
    x, y, w, h = region
    draw = ImageDraw.Draw(img)
    for dy in range(h):
        for dx in range(w):
            px, py = x + dx, y + dy
            if px >= img.width or py >= img.height:
                continue
            use_c1 = ((dx + dy) % 2 == 0) if density >= 0.5 else ((dx + dy) % 3 == 0)
            color = c1 if use_c1 else c2
            draw.point((px, py), fill=color)

def dither_gradient(img, region, c_top, c_bot, pattern="checker"):
    """Gradient dithering from top color to bottom color"""
    x, y, w, h = region
    draw = ImageDraw.Draw(img)
    for dy in range(h):
        t = dy / max(1, h - 1)
        base = blend_colors(c_top, c_bot, t)
        for dx in range(w):
            px, py = x + dx, y + dy
            if px >= img.width or py >= img.height:
                continue
            # Ordered dithering threshold
            threshold = ((px % 4) * 64 + (py % 4) * 16) % 256
            if threshold < (t * 255):
                color = blend_colors(base, c_bot, 0.15)
            else:
                color = blend_colors(base, c_top, 0.15)
            draw.point((px, py), fill=color)

# === SUB-PIXEL RENDERING ===

def draw_subpixel_highlight(draw, x, y, size, color, intensity=0.6):
    """Draw a sub-pixel highlight (1px accent on visor/edges)"""
    r, g, b = color
    a = int(255 * intensity)
    for dy in range(size):
        for dx in range(size):
            # Edge highlight: only on border pixels
            if dx == 0 or dx == size-1 or dy == 0 or dy == size-1:
                draw.point((x + dx, y + dy), fill=(r, g, b, a))

def draw_circuit_lines(draw, x, y, w, h, color, spacing=3):
    """Draw thin circuit-board style lines for tech aesthetic"""
    r, g, b = color
    for i in range(0, w, spacing):
        draw.line([(x + i, y), (x + i, y + h)], fill=(r, g, b, 80), width=1)
    for i in range(0, h, spacing):
        draw.line([(x, y + i), (x + w, y + i)], fill=(r, g, b, 80), width=1)

# === ANIMATION STATE DEFINITIONS ===

ANIMATION_STATES = {
    "idle": {
        "frames": 2,
        "description": "standing still, subtle breathing",
    },
    "walk_left": {
        "frames": 4,
        "description": "walking to the left",
    },
    "walk_right": {
        "frames": 4,
        "description": "walking to the right",
    },
    "hacking_active": {
        "frames": 3,
        "description": "typing/hacking animation with screen glow",
    },
}

# === SPRITE DRAWING ===

def draw_character_base(draw, ox, oy, palette, frame, state, pixel_size=1):
    """
    Draw a single character frame at (ox, oy) using the palette.
    palette keys: skin, hair, shirt, pants, shoes, visor, circuit
    """
    s = pixel_size
    skin = palette.get("skin", (255, 200, 150))
    hair = palette.get("hair", (80, 40, 120))
    shirt = palette.get("shirt", (40, 80, 160))
    pants = palette.get("pants", (60, 60, 80))
    shoes = palette.get("shoes", (40, 40, 50))
    visor = palette.get("visor", (0, 240, 255))
    circuit = palette.get("circuit", (0, 200, 180))

    skin_s = shade_color(skin, 0.7)
    skin_h = shade_color(skin, 1.3)
    hair_s = shade_color(hair, 0.6)
    hair_h = shade_color(hair, 1.2)
    shirt_s = shade_color(shirt, 0.65)
    shirt_h = shade_color(shirt, 1.25)

    # Animation offsets (sub-pixel bobbing)
    bob = 0
    arm_offset = 0
    leg_offset = 0
    if state == "idle":
        bob = 1 if frame % 2 == 1 else 0
    elif state in ("walk_left", "walk_right"):
        leg_offset = (frame % 2) * 2 - 1
        arm_offset = -leg_offset
        bob = 1 if frame % 2 == 0 else 0
    elif state == "hacking_active":
        arm_offset = frame * 1 - 1

    # --- HAIR (top) ---
    for dx in range(8):
        draw.point((ox + (1+dx)*s, oy + (1+bob)*s), fill=hair_s)
        draw.point((ox + (1+dx)*s, oy + (2+bob)*s), fill=hair)
    # hair highlight
    draw.point((ox + 3*s, oy + (1+bob)*s), fill=hair_h)

    # --- HEAD / SKIN ---
    for dy in range(4):
        for dx in range(6):
            c = skin if dx < 5 else skin_s
            draw.point((ox + (2+dx)*s, oy + (3+dy+bob)*s), fill=c)
    # skin highlight
    draw.point((ox + 3*s, oy + (3+bob)*s), fill=skin_h)

    # --- VISOR (sub-pixel detail) ---
    draw.point((ox + 5*s, oy + (4+bob)*s), fill=visor)
    draw.point((ox + 6*s, oy + (4+bob)*s), fill=shade_color(visor, 1.2))
    draw.point((ox + 7*s, oy + (4+bob)*s), fill=visor)
    # visor glow
    draw.point((ox + 6*s, oy + (3+bob)*s), fill=shade_color(visor, 0.5))

    # --- BODY / SHIRT ---
    for dy in range(5):
        for dx in range(8):
            c = shirt if dx < 6 else shirt_s
            if dy == 0:
                c = shirt_h
            draw.point((ox + (1+dx)*s, oy + (7+dy+bob)*s), fill=c)

    # circuit lines on shirt (sub-pixel)
    draw_circuit_lines(draw, ox + 2*s, oy + 8*bob*s if bob else oy + 8*s, 5*s, 3*s, circuit)

    # --- ARMS ---
    arm_y = 8 + bob
    for dy in range(4):
        # left arm
        draw.point((ox + 0*s, oy + (arm_y+dy+arm_offset)*s), fill=skin_s if dy < 2 else skin)
        # right arm
        draw.point((ox + 9*s, oy + (arm_y+dy-arm_offset)*s), fill=skin_s if dy < 2 else skin)

    # --- PANTS ---
    for dy in range(3):
        for dx in range(6):
            c = pants if dx < 4 else shade_color(pants, 0.8)
            draw.point((ox + (2+dx)*s, oy + (12+dy+bob)*s), fill=c)

    # --- LEGS ---
    leg_y = 15 + bob
    for dy in range(2):
        draw.point((ox + 3*s, oy + (leg_y+dy+leg_offset)*s), fill=pants)
        draw.point((ox + 6*s, oy + (leg_y+dy-leg_offset)*s), fill=pants)

    # --- SHOES ---
    shoe_y = 17 + bob
    for dx in range(3):
        draw.point((ox + (2+dx)*s, oy + (shoe_y+leg_offset)*s), fill=shoes)
        draw.point((ox + (5+dx)*s, oy + (shoe_y-leg_offset)*s), fill=shoes)

    return bob

def generate_sprite_sheet(config):
    """
    Generate a full sprite sheet from config.

    config = {
        "sprite_width": 32,
        "sprite_height": 32,
        "pixel_size": 3,
        "palette": {
            "skin": "#ffcc99",
            "hair": "#502878",
            "shirt": "#2850a0",
            "pants": "#3c3c50",
            "shoes": "#282832",
            "visor": "#00f0ff",
            "circuit": "#00c8b4"
        },
        "dithering": true,
        "subpixel_detail": true,
        "states": ["idle", "walk_left", "walk_right", "hacking_active"]
    }
    """
    sprite_w = config.get("sprite_width", 32)
    sprite_h = config.get("sprite_height", 32)
    pixel_size = config.get("pixel_size", 3)
    do_dither = config.get("dithering", True)
    do_subpixel = config.get("subpixel_detail", True)
    states = config.get("states", list(ANIMATION_STATES.keys()))

    # Parse palette
    palette = {}
    raw_palette = config.get("palette", {})
    for k, v in raw_palette.items():
        palette[k] = hex_to_rgb(v) if isinstance(v, str) else tuple(v)

    # Layout: each state is a row, columns are frames
    max_frames = max(ANIMATION_STATES.get(s, {}).get("frames", 2) for s in states)
    cols = max_frames
    rows = len(states)

    sheet_w = cols * sprite_w
    sheet_h = rows * sprite_h

    img = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    for row_idx, state in enumerate(states):
        state_info = ANIMATION_STATES.get(state, {"frames": 2})
        num_frames = state_info["frames"]

        for frame in range(cols):
            ox = frame * sprite_w
            oy = row_idx * sprite_h

            # Draw the character
            bob = draw_character_base(draw, ox, oy, palette, frame % num_frames, state, pixel_size)

            # Dithering: soft shadow under character
            if do_dither:
                shadow_c = shade_color(palette.get("pants", (60, 60, 80)), 0.3)
                bg_c = (0, 0, 0, 0)
                # Shadow ellipse area
                dither_checkerboard(
                    img,
                    (ox + 4*pixel_size, oy + 19*pixel_size, 6*pixel_size, 2*pixel_size),
                    (*shadow_c, 120), (*shadow_c, 40),
                    density=0.6
                )
                draw = ImageDraw.Draw(img)  # refresh draw after pixel manipulation

            # Sub-pixel visor glow (hacking state)
            if do_subpixel and state == "hacking_active":
                visor_c = palette.get("visor", (0, 240, 255))
                # Animated glow frames
                glow_intensity = [0.4, 0.8, 1.0][frame % 3]
                for gx in range(3):
                    for gy in range(2):
                        px = ox + (5 + gx) * pixel_size
                        py = oy + (4 + bob + gy) * pixel_size
                        if px < img.width and py < img.height:
                            gc = shade_color(visor_c, 1.0 + glow_intensity * 0.3)
                            a = int(180 * glow_intensity)
                            draw.point((px, py), fill=(*gc, a))

            # Sub-pixel circuit line highlight on shirt
            if do_subpixel:
                circuit_c = palette.get("circuit", (0, 200, 180))
                # Thin horizontal line across chest
                for cx in range(2, 7):
                    px = ox + cx * pixel_size
                    py = oy + (9 + bob) * pixel_size
                    if px < img.width and py < img.height:
                        draw.point((px, py), fill=(*circuit_c, 100))

    return img


def generate_from_json(config_path, output_path):
    """Load config JSON and generate sprite sheet."""
    with open(config_path, "r") as f:
        config = json.load(f)

    img = generate_sprite_sheet(config)

    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
    img.save(output_path)

    states_generated = config.get("states", list(ANIMATION_STATES.keys()))
    frame_count = sum(ANIMATION_STATES.get(s, {}).get("frames", 2) for s in states_generated)

    return {
        "output": output_path,
        "width": img.width,
        "height": img.height,
        "states": states_generated,
        "total_frames": frame_count,
    }


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("usage: python pixel_art_generator.py <config.json> <output.png>")
        sys.exit(1)

    config_path = sys.argv[1]
    output_path = sys.argv[2]

    result = generate_from_json(config_path, output_path)
    print(json.dumps(result, indent=2))
