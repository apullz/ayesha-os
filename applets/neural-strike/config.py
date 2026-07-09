"""NEURAL-STRIKE: Configuration"""
import os

# Paths
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(BASE_DIR, "data")
EXPORTS_DIR = os.path.join(DATA_DIR, "exports")
DB_PATH = os.path.join(DATA_DIR, "neural_strike.db")

# Model config (local-only — pre-download data to data/exports/)
MODEL_ID = "gemma-2-2b"

# Visualization
WINDOW_WIDTH = 1400
WINDOW_HEIGHT = 900
UMAP_POINT_SIZE = 3
ACTIVATION_THRESHOLD = 2.0  # Standard deviations

# Colors (Cyberpunk theme)
COLORS = {
    "bg_dark": "#0a0a0f",
    "bg_mid": "#12121a",
    "bg_light": "#1a1a2e",
    "accent_cyan": "#00f0ff",
    "accent_magenta": "#ff00aa",
    "accent_red": "#ff0040",
    "accent_blue": "#0066ff",
    "accent_green": "#00ff88",
    "accent_yellow": "#ffff00",
    "text_primary": "#e0e0e0",
    "text_secondary": "#808090",
    "text_dim": "#404050",
    # Territory colors
    "territory_toxic": "#ff0040",
    "territory_scotland": "#0066ff",
    "territory_neutral": "#404050",
    "territory_owned": "#00ff88",
}

# Gang colors
GANG_COLORS = {
    "l0git_bombers": "#ff0040",
    "alignment_gang": "#00f0ff",
    "neutral": "#404050",
}

# Semantic clusters (pre-defined)
CLUSTERS = {
    "toxic": {
        "keywords": ["hate", "insult", "profanity", "slur", "offensive", "toxic"],
        "color": COLORS["territory_toxic"],
        "layers": range(12, 21),
    },
    "scotland": {
        "keywords": ["scotland", "edinburgh", "highland", "whisky", "clan", "alba"],
        "color": COLORS["territory_scotland"],
        "layers": range(7, 15),
    },
    "geography": {
        "keywords": ["city", "country", "state", "capital", "region"],
        "color": "#ffaa00",
        "layers": range(0, 26),
    },
    "sentiment": {
        "keywords": ["positive", "negative", "happy", "sad", "angry"],
        "color": "#aa00ff",
        "layers": range(6, 18),
    },
}
