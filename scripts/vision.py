#!/usr/bin/env python3
"""
vision.py — describe images using gemini flash via google generativelanguage api.

usage:
    python scripts/vision.py "describe this image" image1.png [image2.png ...]

reads api key from ~/.local/share/opencode/auth.json (google.key)
"""
import base64, json, os, sys, urllib.request
from pathlib import Path

# resolve opencode auth path from env or default to user home
_auth_path = os.environ.get(
    "OPENCODE_AUTH_PATH",
    str(Path.home() / ".local" / "share" / "opencode" / "auth.json"),
)
auth = json.load(open(_auth_path))
key = auth["google"]["key"]

def ask(question, *paths):
    parts = []
    for p in paths:
        with open(p, "rb") as f:
            data = base64.b64encode(f.read()).decode()
        mime = "image/png"
        if p.lower().endswith(".jpg") or p.lower().endswith(".jpeg"):
            mime = "image/jpeg"
        elif p.lower().endswith(".webp"):
            mime = "image/webp"
        parts.append({"inline_data": {"mime_type": mime, "data": data}})
    parts.append({"text": question})
    body = {"contents": [{"parts": parts}]}
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:generateContent?key={key}"
    req = urllib.request.Request(url, json.dumps(body).encode(), {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        resp = json.load(r)
    return resp["candidates"][0]["content"]["parts"][0]["text"]

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("usage: python vision.py 'question' image1.png [image2.png ...]")
        sys.exit(1)
    q = sys.argv[1]
    paths = sys.argv[2:]
    print(ask(q, *paths))
