"""
ayesha-os theme extractor — derive a palette from an image and write it
into the `theme` section of ayesha.json (or print a dry-run preview).

run:
    python scripts/theme-from-image.py <image> [--name NAME] [--preset NAME]
        [--set role=#hex ...] [--out ayesha.json] [--dry-run]

Roles written: background, surface, text, primary, accent, secondary,
success, warning, error, dim, border, code_bg.

With --preset, the named built-in theme is used as the base and only the
roles the image actually informs are derived (dark cluster, pop cluster,
text); anything else keeps the preset values. Without it every role is
derived from the image.
"""
import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent

# fixed hues for semantic roles when we derive them (warm-pink default)
DEFAULT_SEMANTIC = {
    "success": "#62C884",
    "warning": "#E0B24E",
    "error": "#E5536A",
}

PRESETS = {
    "kook": {
        "background": "#101014", "surface": "#16161C", "text": "#E8E6F0",
        "primary": "#E75E9D", "accent": "#D782A7", "secondary": "#4C5D79",
        "success": "#62C884", "warning": "#E0B24E", "error": "#E5536A",
        "dim": "#6A6478", "border": "#3D3E45", "code_bg": "#1A1A22",
    },
    "cyberpunk": {
        "background": "#0D0D0D", "surface": "#111111", "text": "#E8E8E8",
        "primary": "#55FF55", "accent": "#55FFFF", "secondary": "#FFFF55",
        "success": "#55FF55", "warning": "#FFFF55", "error": "#FF5555",
        "dim": "#555555", "border": "#3A3A3A", "code_bg": "#1A1A1A",
    },
    "sakura": {
        "background": "#1B0F18", "surface": "#241221", "text": "#F5E6F0",
        "primary": "#FF6FA5", "accent": "#FFB3D1", "secondary": "#8E7CC3",
        "success": "#6FCF97", "warning": "#E8C17D", "error": "#E5536A",
        "dim": "#8A7084", "border": "#5A3B55", "code_bg": "#2A1826",
    },
    "win95": {
        "background": "#000000", "surface": "#1A1A1A", "text": "#C0C0C0",
        "primary": "#008080", "accent": "#00FFFF", "secondary": "#0000A0",
        "success": "#008000", "warning": "#FFFF00", "error": "#FF0000",
        "dim": "#808080", "border": "#A0A0A0", "code_bg": "#101010",
    },
    "mono": {
        "background": "#0A0A0A", "surface": "#111111", "text": "#CCCCCC",
        "primary": "#FFFFFF", "accent": "#FFFFFF", "secondary": "#999999",
        "success": "#FFFFFF", "warning": "#BBBBBB", "error": "#FF6B6B",
        "dim": "#666666", "border": "#333333", "code_bg": "#181818",
    },
}

ROLE_NAMES = list(PRESETS["kook"].keys())


def luminance(rgb):
    r, g, b = (c / 255.0 for c in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def saturation(rgb):
    mx, mn = max(rgb), min(rgb)
    denom = mx + mn
    if mx == 0 or denom == 0:
        return 0.0
    return (mx - mn) / denom


def rgb_to_hex(rgb):
    return "#{:02X}{:02X}{:02X}".format(*rgb)


def clamp(v):
    return max(0, min(255, int(round(v))))


def mix(c1, c2, t):
    return tuple(clamp(a + (b - a) * t) for a, b in zip(c1, c2))


def rgb_to_hsl(rgb):
    r, g, b = (c / 255.0 for c in rgb)
    mx, mn = max(r, g, b), min(r, g, b)
    l = (mx + mn) / 2
    if mx == mn:
        return 0.0, 0.0, l
    d = mx - mn
    s = d / (2 - mx - mn) if l > 0.5 else d / (mx + mn)
    if mx == r:
        h = (g - b) / d + (6 if g < b else 0)
    elif mx == g:
        h = (b - r) / d + 2
    else:
        h = (r - g) / d + 4
    return h / 6.0, s, l


def hsl_to_rgb(h, s, l):
    def hue2rgb(p, q, t):
        if t < 0:
            t += 1
        if t > 1:
            t -= 1
        if t < 1 / 6:
            return p + (q - p) * 6 * t
        if t < 1 / 2:
            return q
        if t < 2 / 3:
            return p + (q - p) * (2 / 3 - t) * 6
        return p

    if s == 0:
        return clamp(l * 255), clamp(l * 255), clamp(l * 255)
    q = l * (1 + s) if l < 0.5 else l + s - l * s
    p = 2 * l - q
    return tuple(clamp(hue2rgb(p, q, h + t) * 255) for t in (1 / 3, 0, 2 / 3))


def rotate_hue(rgb, degrees, s_scale=1.0, l=None):
    h, s, lv = rgb_to_hsl(rgb)
    return hsl_to_rgb((h + degrees / 360.0) % 1.0,
                      max(0.0, min(1.0, s * s_scale)),
                      lv if l is None else l)


def quantize_colors(path, size=160, n=14):
    img = Image.open(path).convert("RGB")
    img.thumbnail((size, size))
    small = img.quantize(colors=n, method=2)
    pal = small.getpalette()[: n * 3]
    data = getattr(small, "get_flattened_data", None)
    idxs = data() if data else list(small.getdata())
    counts = Counter(idxs)
    out = []
    for idx, cnt in counts.most_common():
        rgb = tuple(pal[idx * 3 : idx * 3 + 3])
        out.append((cnt, rgb))
    return out


def pick(candidates, key, reverse=False, default=(255, 255, 255)):
    if not candidates:
        return default
    return sorted(candidates, key=key, reverse=reverse)[0]


def extract_palette(path, preset_base=None, overrides=None):
    overrides = overrides or {}
    colors = quantize_colors(path)

    dark = [(cnt, rgb) for cnt, rgb in colors if luminance(rgb) < 0.30]
    pop = [(cnt, rgb) for cnt, rgb in colors if saturation(rgb) >= 0.35 and luminance(rgb) >= 0.18]
    bright = [(cnt, rgb) for cnt, rgb in colors if luminance(rgb) >= 0.62]

    dark_rgb = [rgb for _, rgb in dark]
    pop_rgb = [rgb for _, rgb in pop]
    bright_rgb = [rgb for _, rgb in bright]

    base = dict(preset_base or {})

    def set_r(r, value):
        if r not in overrides:
            base[r] = value

    # background = darkest, surface = most common neutral dark,
    # border = midpoint between the two
    if dark_rgb:
        bg = min(dark_rgb, key=luminance)
        set_r("background", rgb_to_hex(bg))
        neutral_darks = [
            (cnt, rgb) for cnt, rgb in dark
            if luminance(rgb) > luminance(bg) + 0.01
        ]
        if neutral_darks:
            surface = max(neutral_darks,
                          key=lambda t: t[0] * (1 - saturation(t[1])))[1]
        else:
            surface = max(dark_rgb, key=lambda c: luminance(c))
        set_r("surface", rgb_to_hex(surface))
        set_r("border", rgb_to_hex(mix(bg, surface, 0.5)))
        set_r("code_bg", rgb_to_hex(mix(bg, surface, 0.6)))

    # pop cluster drives primary/accent/secondary
    if pop_rgb:
        pop_ranked = [rgb for _, rgb in sorted(pop, key=lambda t: t[0] * saturation(t[1]), reverse=True)]
        primary = pop_ranked[0]
        set_r("primary", rgb_to_hex(primary))
        if len(pop_ranked) > 1:
            accent = pop_ranked[1]
            if luminance(accent) > luminance(primary):
                set_r("accent", rgb_to_hex(accent))
            else:
                set_r("accent", rgb_to_hex(mix(accent, (255, 255, 255), 0.25)))
        if len(pop_ranked) > 2:
            secondary = pop_ranked[2]
            h, s, lv = rgb_to_hsl(secondary)
            set_r("secondary", rgb_to_hex(hsl_to_rgb(h, max(0.15, min(0.45, s * 0.6)), 0.34)))
    elif base:
        set_r("primary", base.get("primary"))
        set_r("accent", base.get("accent"))
    else:
        # nothing colorful: theme from the darkest hue
        hue_rgb = sorted(dark_rgb, key=lambda c: -saturation(c))[0]
        h, _, _ = rgb_to_hsl(hue_rgb)
        set_r("primary", rgb_to_hex(hsl_to_rgb(h, 0.55, 0.62)))
        set_r("accent", rgb_to_hex(hsl_to_rgb((h + 0.03) % 1.0, 0.45, 0.72)))

    # secondary: most common neutral mid-tone (slate), else derive from primary
    mids = [(cnt, rgb) for cnt, rgb in colors
            if 0.18 <= luminance(rgb) <= 0.55 and saturation(rgb) < 0.35]
    if mids and "secondary" not in base:
        set_r("secondary", rgb_to_hex(max(mids, key=lambda t: t[0])[1]))
    if "secondary" not in base and base.get("primary"):
        h, s, lv = rgb_to_hsl(hex_to_rgb(base["primary"]))
        set_r("secondary", rgb_to_hex(hsl_to_rgb(h, max(0.18, s * 0.35), 0.38)))

    # text: brightest in image, else a near-white
    if bright_rgb:
        set_r("text", rgb_to_hex(max(bright_rgb, key=luminance)))
    elif not preset_base or "text" not in base:
        set_r("text", "#E8E6F0")
    set_r("dim", rgb_to_hex(mix(hex_to_rgb(base["text"]), hex_to_rgb(base["background"]), 0.55)))

    # semantic roles: themed hue rotations off primary
    if base.get("primary"):
        p = hex_to_rgb(base["primary"])
        h, s, lv = rgb_to_hsl(p)
        set_r("success", rgb_to_hex(hsl_to_rgb((h + 0.33) % 1.0, max(0.45, s), 0.55)))
        set_r("warning", rgb_to_hex(hsl_to_rgb((h + 0.15) % 1.0, max(0.45, s), 0.62)))
        if 0.85 <= h <= 1.0 or h <= 0.05:
            # pink/magenta primary: keep error in the red family for contrast
            set_r("error", rgb_to_hex(hsl_to_rgb(0.0, max(0.5, s * 0.9), 0.56)))
        else:
            set_r("error", rgb_to_hex(hsl_to_rgb((h + 0.92) % 1.0, max(0.45, s), 0.58)))
    else:
        set_r("success", DEFAULT_SEMANTIC["success"])
        set_r("warning", DEFAULT_SEMANTIC["warning"])
        set_r("error", DEFAULT_SEMANTIC["error"])

    for r, v in overrides.items():
        base[r] = v
    return {r: base.get(r) for r in ROLE_NAMES}


def hex_to_rgb(hexstr):
    h = hexstr.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def render_swatch(palette):
    def block(rgb):
        r, g, b = rgb
        return f"\x1b[48;2;{r};{g};{b}m  \x1b[0m"
    return "".join(block(hex_to_rgb(v)) for v in palette.values())


def render_ansi_text(text, hexstr):
    r, g, b = hex_to_rgb(hexstr)
    return f"\x1b[38;2;{r};{g};{b}m{text}\x1b[0m"


def main():
    ap = argparse.ArgumentParser(description="derive an ayesha-os theme palette from an image")
    ap.add_argument("image", help="path to the source image")
    ap.add_argument("--name", default="custom", help="theme name to write (default: custom)")
    ap.add_argument("--preset", default=None,
                    help="base theme preset to fill roles the image can't inform")
    ap.add_argument("--set", action="append", default=[],
                    metavar="role=#hex", help="force a role value (repeatable)")
    ap.add_argument("--out", default=str(ROOT / "ayesha.json"), help="config file to write")
    ap.add_argument("--dry-run", action="store_true", help="print only, don't write")
    args = ap.parse_args()

    image_path = Path(args.image)
    if not image_path.exists():
        print(f"error: no such image: {image_path}", file=sys.stderr)
        sys.exit(1)

    overrides = {}
    for pair in args.set:
        if "=" not in pair:
            print(f"error: --set needs role=#hex, got '{pair}'", file=sys.stderr)
            sys.exit(1)
        k, v = pair.split("=", 1)
        k = k.strip().lower()
        if k not in ROLE_NAMES:
            print(f"error: unknown role '{k}' (roles: {', '.join(ROLE_NAMES)})", file=sys.stderr)
            sys.exit(1)
        overrides[k] = v.strip()

    base = PRESETS.get(args.preset) if args.preset else None
    palette = extract_palette(image_path, preset_base=base, overrides=overrides)

    print(f"\n  {render_ansi_text(args.name, palette['primary'])}  {render_swatch(palette)}")
    for r in ROLE_NAMES:
        print(f"    {r:<11} {render_ansi_text(palette[r], palette[r])}")
    print()

    if args.dry_run:
        return

    out_path = Path(args.out)
    if out_path.exists():
        cfg = json.loads(out_path.read_text())
    else:
        cfg = {}
    cfg["theme"] = {"name": args.name, "palette": palette}
    out_path.write_text(json.dumps(cfg, indent=2, ensure_ascii=False) + "\n")
    print(f"  wrote theme '{args.name}' -> {out_path}")


if __name__ == "__main__":
    main()
