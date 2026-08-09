import base64
import json
import os

_DIR = os.path.dirname(os.path.abspath(__file__))


def _read(name):
    with open(os.path.join(_DIR, name), "r", encoding="utf-8") as f:
        return f.read()


def _avatar_data_uri():
    with open(os.path.join(_DIR, "ayesha-bot.png"), "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode("ascii")


_STRUCT = _read("overlay.html").replace("{{AVATAR_SRC}}", _avatar_data_uri())

CUSTOM_CSS = _read("theme.css")
CUSTOM_JS = _read("chat.js").replace("{{STRUCT_JSON}}", json.dumps(_STRUCT))
