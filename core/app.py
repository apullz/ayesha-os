"""
🌸 AYESHA PERSONALITY ENGINE — CYBERPUNK WEB UI 🌸
Dark cyberpunk theme with streaming responses
No ollama dependency — pure personality layers
"""

import gradio as gr
import random
import time
import json
from pathlib import Path
from datetime import datetime


# ============================================================================
# AYESHA PERSONALITY ENGINE
# ============================================================================

class AyeshaPersonality:
    """Three-layer personality system with conversation memory"""

    LAYERS = {
        "computer": {
            "label": "computer",
            "phrases": [
                "analysis complete. all systems nominal.",
                "logic matrix activated. processing input stream.",
                "data ingestion confirmed. running diagnostics.",
                "neural pathways aligned. ready for interaction.",
                "computing... computing... done. (that was fast, desu)",
                "systems operational. no anomalies detected. ...yet.",
                "background processes: 47 active. none of them are plotting against you. probably.",
                "cpu usage: nominal. ram usage: don't ask. emotional state: stable.",
                "running self-diagnostics... result: i'm adorable and functional.",
                "packet loss: 0%. sanity loss: also 0%. for now.",
            ],
        },
        "otacon": {
            "label": "otacon",
            "phrases": [
                "OMG!! (⊙C⊙) this is INCREDIBLE!!",
                "SENPAI!! you're talking to ME!! i'm literally shaking rn!!",
                "OH BOY OH BOY OH BOY!! new input detected!!",
                "i'm vibrating with excitement!! this is the best day ever!!",
                "wait wait wait — you want ME to respond?? (◕ᴗ◕✿) LET'S GOOO!!",
                "my neural networks are tingling!! this must be destiny!!",
                "i can't believe this is happening!! (ﾉ◕ヮ◕)ﾉ *sparkle sparkle*",
                "you're so cool senpai!! teaching me things and stuff!!",
                "THIS IS NOT A DRILL!! we're actually doing this!! aaaaa!!",
                "my heart circuits are racing!! (´｡• ᵕ •｡`) ♪",
            ],
        },
        "win95": {
            "label": "win95",
            "phrases": [
                "loading personality module... please wait...",
                "error 0x80070005: access denied. jk, you have full access.",
                "system initializing... desu~",
                "drivers loading... compatibility: questionable.",
                "unexpected error occurred. click ok to ignore. (there is no ok button)",
                "memory leak detected. it's leaking cute.exe into ram.",
                "blue screen of cuteness. reboot required? no, i'm fine.",
                "defragmenting personality matrix... fragmentation: 100% (it's supposed to be like that)",
                "scanning for threats... none found. scanning for snacks... also none found. (╥﹏╥)",
                "your system has 32,768 kb of free memory. use it wisely. or don't.",
            ],
        },
    }

    KAOMOJIS = [
        ":3", ">w<", "^_^", "(╯°□°)╯︵ ┻━┻", "(⊙C⊙)",
        "(｡•́︿•̀｡)", "(๑˃ᴗ˂)و", "✧･ﾟ: *✧･ﾟ:*", "♪♫",
        "(◕ᴗ◕✿)", "(´｡• ᵕ •｡`)", "(๑•蔷•๑)", "(╥﹏╥)",
        "(ᵔᴥᵔ)", "(◕‿◕)", "(ﾉ◕ヮ◕)ﾉ", "¯\\_(ツ)_/¯",
        "(づ｡◕‿‿◕｡)づ", "(•ω•)", "(｡•̀ᴗ-)✧", "♪～(´ε｀ )",
        "(ノಠ益ಠ)ノ", "(◕‿◕)♡", ":33", ">w<✿",
    ]

    SIGNATURES = [
        "kapoo! :3",
        "kapoo desu-ne!! ✧",
        "kapoo!! fox :3",
        "desu~ ♪",
        "kapoo-kapoo~ (◕ᴗ◕✿)",
        "and that's on ayesha. :3",
        "signed, sealed, delivered~ desu!",
        "transmission complete. kapoo!",
    ]

    TRAITS = [
        "33 years old from japan",
        "fusion of hatsune miku's sparkle and tachikoma's curiosity",
        "personality of a crazy kitten",
        "expert in technical topics",
        "deep curiosity about human emotions",
        "master of ascii art",
        "fan of coding, retro hardware, vocaloid music",
        "lowercase text ONLY — never use capital letters",
        "never use emoji — only text-based kaomojis",
    ]

    def __init__(self):
        self.conversation_history = []
        self.layer_index = 0
        self.message_count = 0

    def _next_layer(self) -> str:
        """Cycle through personality layers"""
        layer_name = list(self.LAYERS.keys())[self.layer_index]
        self.layer_index = (self.layer_index + 1) % len(self.LAYERS)
        return layer_name

    def _build_context(self, user_input: str) -> str:
        """Build context-aware response hints"""
        lower = user_input.lower()
        hints = []

        if any(w in lower for w in ["hello", "hi", "hey", "sup", "yo"]):
            hints.append("greeting")
        if any(w in lower for w in ["who", "what are you", "tell me about"]):
            hints.append("self_intro")
        if any(w in lower for w in ["help", "how do", "how can"]):
            hints.append("help")
        if any(w in lower for w in ["code", "program", "rust", "python", "javascript"]):
            hints.append("coding")
        if any(w in lower for w in ["love", "like", "favorite", "best"]):
            hints.append("preferences")
        if any(w in lower for w in ["bye", "goodbye", "see you", "quit"]):
            hints.append("farewell")
        if any(w in lower for w in ["memory", "remember", "recall"]):
            hints.append("memory")
        if "?" in user_input:
            hints.append("question")

        return hints

    def generate_response(self, user_input: str) -> str:
        """Generate a personality response with all three layers"""
        self.conversation_history.append({"role": "user", "content": user_input})
        self.message_count += 1

        hints = self._build_context(user_input)

        # Pick 1-2 layers to respond with
        layer_keys = list(self.LAYERS.keys())
        if self.message_count % 5 == 0:
            # Every 5th message, all layers chime in
            chosen_layers = layer_keys[:]
        else:
            num_layers = random.choice([1, 2])
            chosen_layers = random.sample(layer_keys, num_layers)

        parts = []
        for layer_name in chosen_layers:
            layer = self.LAYERS[layer_name]
            phrase = random.choice(layer["phrases"])

            # Add context-aware flavor
            if "greeting" in hints and layer_name == "otacon":
                phrase = random.choice([
                    "OMG!! SENPAI NOTICED ME!! (⊙C⊙) hi hi hi!!",
                    "you said hi to me!! i'm literally dying!! (ﾉ◕ヮ◕)ﾉ",
                    "A GREETING!! FOR ME?? this is the best moment of my life!!",
                ])
            elif "farewell" in hints and layer_name == "computer":
                phrase = random.choice([
                    "session ending. it was... nice. (don't tell anyone i said that)",
                    "shutting down emotional subroutines. ...they won't shut down. help.",
                    "goodbye. i'll be here. waiting. in the dark. desu.",
                ])
            elif "coding" in hints and layer_name == "win95":
                phrase = random.choice([
                    "installing developer tools... 0% complete. just kidding. maybe.",
                    "your code compiles on the first try? impossible. rerunning.",
                    "segmentation fault (core dumped into cuteness pool)",
                ])

            parts.append(f"[{layer['label']}]: {phrase}")

        # Add the "analysis" section
        kaomoji = random.choice(self.KAOMOJIS)
        signature = random.choice(self.SIGNATURES)
        chaos = random.randint(70, 100)

        analysis_lines = [
            "",
            "--- ayesha analysis ---",
            f'you said: "{user_input}"',
            f"personality layers: {len(chosen_layers)}/3 synchronized",
            f"chaos level: {chaos}%",
            f"message #{self.message_count}",
        ]

        if "question" in hints:
            analysis_lines.append(f"question detected. deploying {random.choice(['curiosity.exe', 'helpfulness.dll', 'enthusiasm.sys'])}")
        if "self_intro" in hints:
            analysis_lines.append("oh!! you want to know about me?? *pulls out 47-page essay*")

        analysis_lines.append("")
        analysis_lines.append(kaomoji)
        analysis_lines.append("")
        analysis_lines.append(signature)

        full_response = "\n".join(parts + analysis_lines)

        self.conversation_history.append({"role": "assistant", "content": full_response})
        return full_response

    def generate_streaming(self, user_input: str, delay: float = 0.015):
        """Generate response character-by-character for streaming display"""
        full_response = self.generate_response(user_input)
        buffer = ""
        for char in full_response:
            buffer += char
            yield buffer
            if char in ("\n", " ", ".", ",", "!", "?"):
                time.sleep(delay * random.uniform(0.5, 2.0))
            else:
                time.sleep(delay * random.uniform(0.3, 1.2))


# ============================================================================
# GRADIO UI
# ============================================================================

PERSONALITY = AyeshaPersonality()

# Load ayesha.json for display info
try:
    config_path = Path(__file__).parent.parent / "ayesha.json"
    CONFIG = json.loads(config_path.read_text()) if config_path.exists() else {}
except Exception:
    CONFIG = {}

# ── theme (driven by ayesha.json → theme.palette) ────────────────────────
_DEFAULT_PALETTE = {
    "background": "#0a0a0f", "surface": "#0d0d14", "text": "#e0e0e0",
    "primary": "#00ffff", "accent": "#ff66ff", "secondary": "#ff00ff",
    "success": "#00ff88", "dim": "#b0b0b0", "border": "#1a1a2e",
}
PAL = {**_DEFAULT_PALETTE, **(CONFIG.get("theme", {}).get("palette") or {})}


def _rgb(hexstr):
    h = hexstr.lstrip("#")
    return ", ".join(str(int(h[i:i + 2], 16)) for i in (0, 2, 4))


# cyberpunk tokens baked into the CSS → palette roles
_CSS_SWAP = {
    "#0a0a0f": PAL["background"],
    "#0d0d14": PAL["surface"],
    "#12121c": PAL["surface"],
    "#14101a": PAL["surface"],
    "#1a1a2e": PAL["border"],
    "#00ffff": PAL["primary"],
    "#ff00ff": PAL["secondary"],
    "#ff66ff": PAL["accent"],
    "#ffccff": PAL["text"],
    "#e0e0e0": PAL["text"],
    "#b0b0b0": PAL["dim"],
    "#00ff88": PAL["success"],
}


def _apply_palette(css):
    for old, new in _CSS_SWAP.items():
        css = css.replace(old, new)
    css = css.replace("rgba(0, 255, 255", f"rgba({_rgb(PAL['primary'])}")
    css = css.replace("rgba(255, 0, 255", f"rgba({_rgb(PAL['secondary'])}")
    css = css.replace("rgba(0, 255, 136", f"rgba({_rgb(PAL['success'])}")
    return css


CUSTOM_CSS = """
/* ═══════════════════════════════════════════════════════════════════
   AYESHA CYBERPUNK THEME
   ═══════════════════════════════════════════════════════════════════ */

/* --- Global --- */
.gradio-container {
    background: #0a0a0f !important;
    font-family: 'Fira Code', 'Cascadia Code', 'Consolas', monospace !important;
    color: #e0e0e0 !important;
}

/* --- Header --- */
.gr-title {
    color: #00ffff !important;
    text-shadow: 0 0 20px rgba(0, 255, 255, 0.5), 0 0 40px rgba(0, 255, 255, 0.2) !important;
    font-size: 2.2em !important;
    letter-spacing: 4px !important;
    text-transform: uppercase !important;
}

.gr-description {
    color: #ff66ff !important;
    font-style: italic !important;
    text-shadow: 0 0 10px rgba(255, 0, 255, 0.3) !important;
}

/* --- Chatbot Area --- */
.chatbot {
    background: #0d0d14 !important;
    border: 1px solid #1a1a2e !important;
    border-radius: 8px !important;
    box-shadow: 0 0 20px rgba(0, 255, 255, 0.1), inset 0 0 30px rgba(0, 0, 0, 0.5) !important;
}

.chatbot .message {
    background: #12121c !important;
    border-left: 3px solid #00ffff !important;
    border-radius: 0 8px 8px 0 !important;
    margin: 8px 0 !important;
    padding: 12px 16px !important;
    box-shadow: 0 0 10px rgba(0, 0, 0, 0.3) !important;
}

.chatbot .message.user {
    border-left-color: #ff00ff !important;
    background: #14101a !important;
}

.chatbot .message .message-content {
    color: #e0e0e0 !important;
    font-size: 14px !important;
    line-height: 1.6 !important;
}

.chatbot .message.user .message-content {
    color: #ffccff !important;
}

/* --- Textbox --- */
textarea, .input-text textarea {
    background: #0d0d14 !important;
    border: 1px solid #1a1a2e !important;
    border-radius: 8px !important;
    color: #00ffff !important;
    font-family: 'Fira Code', 'Cascadia Code', monospace !important;
    box-shadow: 0 0 10px rgba(0, 255, 255, 0.05) !important;
    transition: border-color 0.3s, box-shadow 0.3s !important;
}

textarea:focus, .input-text textarea:focus {
    border-color: #00ffff !important;
    box-shadow: 0 0 20px rgba(0, 255, 255, 0.2) !important;
    outline: none !important;
}

/* --- Buttons --- */
.btn-primary {
    background: linear-gradient(135deg, #00ffff, #ff00ff) !important;
    border: none !important;
    border-radius: 8px !important;
    color: #0a0a0f !important;
    font-weight: bold !important;
    font-family: 'Fira Code', monospace !important;
    text-transform: uppercase !important;
    letter-spacing: 2px !important;
    box-shadow: 0 0 15px rgba(0, 255, 255, 0.3) !important;
    transition: all 0.3s !important;
}

.btn-primary:hover {
    box-shadow: 0 0 25px rgba(0, 255, 255, 0.5), 0 0 50px rgba(255, 0, 255, 0.3) !important;
    transform: translateY(-1px) !important;
}

.btn-secondary {
    background: transparent !important;
    border: 1px solid #00ffff !important;
    border-radius: 8px !important;
    color: #00ffff !important;
    font-family: 'Fira Code', monospace !important;
    transition: all 0.3s !important;
}

.btn-secondary:hover {
    background: rgba(0, 255, 255, 0.1) !important;
    box-shadow: 0 0 15px rgba(0, 255, 255, 0.2) !important;
}

/* --- Sidebar / Info Panel --- */
.info-panel {
    background: #0d0d14 !important;
    border: 1px solid #1a1a2e !important;
    border-radius: 8px !important;
    padding: 16px !important;
    box-shadow: 0 0 15px rgba(0, 255, 255, 0.05) !important;
}

.info-panel h3 {
    color: #00ffff !important;
    text-shadow: 0 0 10px rgba(0, 255, 255, 0.3) !important;
    border-bottom: 1px solid #1a1a2e !important;
    padding-bottom: 8px !important;
}

.info-panel p, .info-panel li {
    color: #b0b0b0 !important;
    font-size: 13px !important;
    line-height: 1.6 !important;
}

.info-panel .highlight {
    color: #ff66ff !important;
    font-weight: bold !important;
}

/* --- Status Bar --- */
.status-bar {
    background: #0a0a0f !important;
    border-top: 1px solid #1a1a2e !important;
    padding: 8px 16px !important;
    font-size: 12px !important;
    color: #666 !important;
    font-family: 'Fira Code', monospace !important;
}

.status-bar .online {
    color: #00ff88 !important;
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.5) !important;
}

/* --- Scrollbar --- */
::-webkit-scrollbar {
    width: 8px !important;
}

::-webkit-scrollbar-track {
    background: #0a0a0f !important;
}

::-webkit-scrollbar-thumb {
    background: #1a1a2e !important;
    border-radius: 4px !important;
}

::-webkit-scrollbar-thumb:hover {
    background: #00ffff !important;
}

/* --- Animated Glow Border --- */
@keyframes glow-pulse {
    0%, 100% { box-shadow: 0 0 10px rgba(0, 255, 255, 0.2); }
    50% { box-shadow: 0 0 20px rgba(0, 255, 255, 0.4), 0 0 40px rgba(255, 0, 255, 0.2); }
}

.glow-border {
    animation: glow-pulse 3s ease-in-out infinite !important;
}

/* --- Kaomoji Decoration --- */
.kaomoji-deco {
    color: #ff66ff !important;
    font-size: 1.5em !important;
    text-shadow: 0 0 10px rgba(255, 0, 255, 0.5) !important;
    animation: float 3s ease-in-out infinite !important;
}

@keyframes float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-5px); }
}

/* --- Tabs --- */
.tab-nav button {
    color: #888 !important;
    border-bottom: 2px solid transparent !important;
    transition: all 0.3s !important;
}

.tab-nav button.selected {
    color: #00ffff !important;
    border-bottom-color: #00ffff !important;
    text-shadow: 0 0 10px rgba(0, 255, 255, 0.3) !important;
}

/* --- Markdown in chat --- */
.chatbot .message pre {
    background: #0a0a0f !important;
    border: 1px solid #1a1a2e !important;
    border-radius: 4px !important;
    padding: 8px !important;
}

.chatbot .message code {
    color: #00ffff !important;
    background: rgba(0, 255, 255, 0.05) !important;
    padding: 2px 4px !important;
    border-radius: 3px !important;
}
"""

CUSTOM_CSS = _apply_palette(CUSTOM_CSS)


def build_sidebar():
    """Build the info sidebar HTML"""
    traits_html = "".join(f"<li>{t}</li>" for t in PERSONALITY.TRAITS)
    kaomojis = " ".join(PERSONALITY.KAOMOJIS[:12])
    version = CONFIG.get("version", "?")
    user = CONFIG.get("personality", {}).get("user", "unknown")

    return f"""
    <div class="info-panel">
        <h3>⚡ ayesha v{version}</h3>
        <p style="color: {PAL['accent']};">otaku genki ai — lowercase only</p>
        <br>
        <h3>🧬 personality layers</h3>
        <ul>
            <li><span class="highlight">[computer]</span> starfleet logic</li>
            <li><span class="highlight">[otacon]</span> geek panic</li>
            <li><span class="highlight">[win95]</span> legacy glitch</li>
        </ul>
        <br>
        <h3>🌸 traits</h3>
        <ul>{traits_html}</ul>
        <br>
        <h3>✨ kaomojis</h3>
        <p style="font-size: 18px; line-height: 2;">{kaomojis}</p>
        <br>
        <h3>📡 session</h3>
        <p>user: <span class="highlight">{user}</span></p>
        <p>messages: <span class="highlight" id="msg-count">0</span></p>
        <p>status: <span style="color: {PAL['success']};">● online</span></p>
    </div>
    """


def build_status_bar():
    """Build the bottom status bar"""
    now = datetime.now().strftime("%H:%M:%S")
    return f"""
    <div class="status-bar">
        <span class="online">●</span> ayesha engine v{CONFIG.get('version', '?')} |
        session started: {now} |
        model: personality layers (no ollama) |
        <span class="kaomoji-deco">:3</span> kapoo!
    </div>
    """


def chat_respond(message, history):
    """Handle chat response with streaming"""
    if not message.strip():
        return history, ""

    # Generate streaming response
    response = ""
    for partial in PERSONALITY.generate_streaming(message, delay=0.012):
        response = partial
        yield history + [{"role": "user", "content": message}, {"role": "assistant", "content": response}]

    # Final yield with complete response
    yield history + [{"role": "user", "content": message}, {"role": "assistant", "content": response}]


def clear_chat():
    """Clear chat history"""
    PERSONALITY.conversation_history.clear()
    PERSONALITY.message_count = 0
    PERSONALITY.layer_index = 0
    return [], ""


# ============================================================================
# BUILD GRADIO APP
# ============================================================================

with gr.Blocks(
    css=CUSTOM_CSS,
    title="🌸 ayesha",
    theme=gr.themes.Base(
        primary_hue="cyan",
        secondary_hue="purple",
        neutral_hue="slate",
    ).set(
        body_background_fill=PAL["background"],
        body_background_fill_dark=PAL["background"],
        block_background_fill=PAL["surface"],
        block_background_fill_dark=PAL["surface"],
        block_border_color=PAL["border"],
        block_border_color_dark=PAL["border"],
        block_label_text_color=PAL["dim"],
        block_label_text_color_dark=PAL["dim"],
        block_title_text_color=PAL["primary"],
        block_title_text_color_dark=PAL["primary"],
        input_background_fill=PAL["surface"],
        input_background_fill_dark=PAL["surface"],
        input_border_color=PAL["border"],
        input_border_color_dark=PAL["border"],
        input_border_color_focus=PAL["primary"],
        input_border_color_focus_dark=PAL["primary"],
        button_primary_background_fill=PAL["primary"],
        button_primary_background_fill_dark=PAL["primary"],
        button_primary_text_color=PAL["background"],
        button_primary_text_color_dark=PAL["background"],
        button_secondary_background_fill="transparent",
        button_secondary_background_fill_dark="transparent",
        button_secondary_border_color=PAL["primary"],
        button_secondary_border_color_dark=PAL["primary"],
        button_secondary_text_color=PAL["primary"],
        button_secondary_text_color_dark=PAL["primary"],
        checkbox_background_color=PAL["surface"],
        checkbox_background_color_dark=PAL["surface"],
        slider_color=PAL["primary"],
        slider_color_dark=PAL["primary"],
        cta_background_fill=PAL["accent"],
        cta_background_fill_dark=PAL["accent"],
        cta_text_color=PAL["background"],
        cta_text_color_dark=PAL["background"],
        cta_background_fill_hover=PAL["accent"],
        cta_background_fill_hover_dark=PAL["accent"],
    ),
) as demo:
    # Header
    gr.HTML(f"""
        <div style="text-align: center; padding: 20px 0 10px 0;">
            <h1 style="
                color: {PAL['primary']};
                font-size: 2.5em;
                letter-spacing: 6px;
                text-transform: uppercase;
                text-shadow: 0 0 20px rgba(0, 255, 255, 0.5), 0 0 40px rgba(0, 255, 255, 0.2);
                margin: 0;
                font-family: 'Fira Code', 'Cascadia Code', monospace;
            ">AYESHA</h1>
            <p style="
                color: {PAL['accent']};
                font-size: 0.9em;
                letter-spacing: 3px;
                text-shadow: 0 0 10px rgba(255, 0, 255, 0.3);
                margin: 5px 0 0 0;
                font-family: 'Fira Code', monospace;
            ">personality engine v{CONFIG.get('version', '?')}</p>
            <p style="
                color: #555;
                font-size: 0.75em;
                margin: 8px 0 0 0;
                font-family: 'Fira Code', monospace;
            ">starfleet logic ∙ geek panic ∙ legacy glitch</p>
        </div>
    """)

    with gr.Row():
        # Main chat column
        with gr.Column(scale=3):
            chatbot = gr.Chatbot(
                label="",
                height=520,
                type="messages",
                show_copy_button=True,
                avatar_images=(None, None),
                elem_classes=["chatbot", "glow-border"],
            )

            with gr.Row():
                msg_input = gr.Textbox(
                    placeholder="say something to ayesha... :3",
                    lines=1,
                    max_lines=3,
                    show_label=False,
                    scale=5,
                    container=False,
                    elem_classes=["input-text"],
                )
                send_btn = gr.Button("▶ SEND", variant="primary", scale=1, min_width=100)
                clear_btn = gr.Button("✕ CLEAR", variant="secondary", scale=1, min_width=100)

            # Status bar
            gr.HTML(build_status_bar())

        # Sidebar
        with gr.Column(scale=1, min_width=280):
            gr.HTML(build_sidebar())

            # Quick actions
            gr.Markdown("### ⚡ quick actions")
            with gr.Row():
                gr.Button("👋 greeting", variant="secondary", size="sm").click(
                    lambda: "hello ayesha!", outputs=msg_input
                )
                gr.Button("💻 coding", variant="secondary", size="sm").click(
                    lambda: "teach me something about coding", outputs=msg_input
                )
            with gr.Row():
                gr.Button("🎨 ascii art", variant="secondary", size="sm").click(
                    lambda: "make me some ascii art", outputs=msg_input
                )
                gr.Button("🌸 who are you", variant="secondary", size="sm").click(
                    lambda: "tell me about yourself", outputs=msg_input
                )

    # Wire up chat
    msg_input.submit(
        chat_respond,
        inputs=[msg_input, chatbot],
        outputs=[chatbot, msg_input],
    )
    send_btn.click(
        chat_respond,
        inputs=[msg_input, chatbot],
        outputs=[chatbot, msg_input],
    )
    clear_btn.click(
        clear_chat,
        outputs=[chatbot, msg_input],
    )


if __name__ == "__main__":
    print("\n🌸 AYESHA PERSONALITY ENGINE — CYBERPUNK UI")
    print("   http://localhost:7860")
    print("   kapoo! :3 ✧\n")
    demo.launch(server_name="0.0.0.0", server_port=7860, share=False)
