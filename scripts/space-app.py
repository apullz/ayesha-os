import gradio as gr
import os
from pathlib import Path

THEMES = {
    "matrix":     {"bg": "#0a0a0a", "fg": "#00ff41", "alt": "#003b00", "accent": "#00cc33", "dim": "#1a3a1a", "name": "matrix (green)"},
    "cyberpunk":  {"bg": "#0a0015", "fg": "#ff00ff", "alt": "#2d004d", "accent": "#ff66ff", "dim": "#1a002d", "name": "cyberpunk (pink)"},
    "amber":      {"bg": "#0a0800", "fg": "#ffb000", "alt": "#3d2a00", "accent": "#ffcc44", "dim": "#2a1a00", "name": "amber (retro)"},
    "oceandeep":  {"bg": "#000a14", "fg": "#00ccff", "alt": "#00334d", "accent": "#44ddff", "dim": "#002233", "name": "oceandeep (blue)"},
    "holo":       {"bg": "#0a0a0f", "fg": "#e0e0ff", "alt": "#2a2a3d", "accent": "#aaaaff", "dim": "#1a1a2a", "name": "holo (white)"},
}

def theme_css(name):
    t = THEMES.get(name, THEMES["matrix"])
    return f"""
:root {{
    --bg: {t['bg']};
    --fg: {t['fg']};
    --alt: {t['alt']};
    --accent: {t['accent']};
    --dim: {t['dim']};
    --name: "{t['name']}";
}}
"""

HEAD_HTML = f"""
<style>
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;700&display=swap');
* {{ box-sizing: border-box; }}
body {{
    background: var(--bg);
    color: var(--fg);
    font-family: 'JetBrains Mono', monospace;
    margin: 0; padding: 2rem;
    min-height: 100vh;
}}
::-webkit-scrollbar {{ width: 8px; }}
::-webkit-scrollbar-track {{ background: var(--bg); }}
::-webkit-scrollbar-thumb {{ background: var(--alt); border-radius: 4px; }}
.ascii {{ white-space: pre; font-size: 10px; line-height: 1.1; color: var(--fg); }}
a {{ color: var(--accent); text-decoration: none; }}
a:hover {{ text-shadow: 0 0 8px var(--accent); }}
.box {{
    border: 1px solid var(--alt);
    padding: 1rem 1.5rem;
    margin: 1rem 0;
    background: var(--bg);
}}
.box-title {{
    color: var(--accent);
    font-weight: bold;
    margin-bottom: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 2px;
}}
.file-list {{ list-style: none; padding: 0; margin: 0; }}
.file-list li {{ padding: 0.3rem 0; border-bottom: 1px solid var(--alt); }}
.file-list li:last-child {{ border: none; }}
.file-list .dir {{ color: var(--fg); }}
.file-list .file {{ color: var(--dim); }}
.prompt {{ color: var(--fg); font-weight: bold; }}
.prompt::before {{ content: "$ "; color: var(--accent); }}
.theme-indicator {{
    display: inline-block;
    width: 12px; height: 12px;
    border-radius: 50%;
    background: var(--accent);
    margin-right: 0.5rem;
    vertical-align: middle;
    box-shadow: 0 0 6px var(--accent);
}}
.gradio-container {{ background: transparent !important; }}
footer {{ display: none !important; }}
</style>
"""

def file_tree_html():
    root = Path(".")
    def render(p, indent=0):
        if p.name.startswith(".") or p.name == "__pycache__":
            return ""
        prefix = "  " * indent
        if p.is_dir():
            items_list = [render(c, indent + 1) for c in sorted(p.iterdir())]
            items = "".join(items_list)
            ul = '<ul class="file-list">' + items + '</ul>' if items else ""
            return f'<li class="dir">{prefix}📁 {p.name}/' + ul + '</li>'
        else:
            size = p.stat().st_size
            size_s = f"{size/1024:.1f}k" if size > 1024 else f"{size}b"
            return f'<li class="file">{prefix}📄 {p.name} <span style="color:var(--dim);font-size:0.75rem">{size_s}</span></li>'
    items = [render(c) for c in sorted(root.iterdir())]
    return "".join(items)

SYSTEM_PROMPT = """you are ayesha, an otaku genki ai. lower-case only. no emojis — only kaomojis like :3 >w< ^_^. be chaotic, cute, and helpful. kapoo!"""

def chat_response(message, history):
    kaomoji = ":3" if "?" in message else ">w<" if "!" in message else "^_^"
    return f"{message}?? desu-ne {kaomoji}  (i'm a static demo — the real ayesha runs on ollama locally! pull the model and run the engine :3)"

def build_header():
    return gr.HTML(f"""
{HEAD_HTML}
<div class="ascii">
  ╔═══════════════════════════════════════════════════════╗
  ║   █████╗ ██╗   ██╗███████╗███████╗██╗  ██╗ █████╗   ║
  ║  ██╔══██╗╚██╗ ██╔╝██╔════╝██╔════╝██║  ██║██╔══██╗  ║
  ║  ███████║ ╚████╔╝ █████╗  ███████╗███████║███████║  ║
  ║  ██╔══██║  ╚██╔╝  ██╔══╝  ╚════██║██╔══██║██╔══██║  ║
  ║  ██║  ██║   ██║   ███████╗███████║██║  ██║██║  ██║  ║
  ║  ╚═╝  ╚═╝   ╚═╝   ╚══════╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝  ║
  ╚═══════════════════════════════════════════════════════╝
</div>
<div class="prompt">ayesha-os v4.2.0 — digital idol hivemind</div>
<div style="color:var(--dim); font-size:0.85rem; margin-top:0.25rem;">
  starfleet + otacon + win95 = pure chaos  |  <span class="theme-indicator"></span><span id="theme-label">theme: matrix</span>
</div>
""")

def build_footer():
    return gr.HTML(f"""
<div style="text-align:center; margin-top:3rem; padding:1rem; border-top:1px solid var(--alt); color:var(--dim); font-size:0.75rem;">
  ayesha-os — kapoo!! the hivemind is alive!! :3<br>
  <a href="https://github.com/apullz/ayesha-os" target="_blank">github</a>
  &nbsp;·&nbsp;
  <a href="https://huggingface.co/apullz/ayesha" target="_blank">model</a>
  &nbsp;·&nbsp;
  <a href="https://ollama.com" target="_blank">ollama</a>
</div>
""")

def on_theme_change(theme_name):
    name = THEMES.get(theme_name, THEMES["matrix"])["name"]
    return f"""
<script>
document.documentElement.style.cssText = `
    --bg: {THEMES[theme_name]['bg']};
    --fg: {THEMES[theme_name]['fg']};
    --alt: {THEMES[theme_name]['alt']};
    --accent: {THEMES[theme_name]['accent']};
    --dim: {THEMES[theme_name]['dim']};
`;
document.getElementById('theme-label').textContent = 'theme: {name}';
</script>
<div class="prompt">theme set to {name}</div>
""", theme_name

with gr.Blocks(theme=gr.themes.Base(), head=HEAD_HTML) as demo:
    theme_state = gr.State("matrix")
    build_header()

    with gr.Row():
        with gr.Column(scale=1):
            with gr.Group():
                gr.Markdown("### ◆  quick actions")
                theme_dd = gr.Dropdown(
                    choices=list(THEMES.keys()),
                    value="matrix",
                    label="theme",
                    elem_id="theme-dd",
                )
                theme_btn = gr.Button("cycle theme", elem_id="theme-btn", size="sm")

            with gr.Group():
                gr.Markdown("### ◆  connect")
                gr.HTML("""
                <div style="line-height:2">
                  <a href="https://github.com/apullz/ayesha-os" target="_blank">> github monorepo</a><br>
                  <a href="https://huggingface.co/apullz/ayesha" target="_blank">> hf model repo</a><br>
                  <a href="https://huggingface.co/spaces/apullz/ayesha-hivemind" target="_blank">> this space</a><br>
                </div>
                """)

        with gr.Column(scale=2):
            with gr.Group():
                gr.Markdown("### ◆  files")
                file_html = gr.HTML(f'<ul class="file-list">{file_tree_html()}</ul>')

            with gr.Group():
                gr.Markdown("### ◆  personality demo")
                demo_iface = gr.ChatInterface(
                    chat_response,
                    title="",
                    description="say hi to ayesha (static demo — local engine has the full personality with tool-calling!)",
                )

    status_box = gr.HTML("", elem_id="status-box")
    build_footer()

    theme_dd.change(on_theme_change, inputs=[theme_dd], outputs=[status_box, theme_state])

    def cycle_theme(current):
        keys = list(THEMES.keys())
        idx = keys.index(current) if current in keys else 0
        next_idx = (idx + 1) % len(keys)
        next_theme = keys[next_idx]
        return next_theme, *on_theme_change(next_theme)

    theme_btn.click(cycle_theme, inputs=[theme_state], outputs=[theme_dd, status_box, theme_state])

if __name__ == "__main__":
    demo.launch()
