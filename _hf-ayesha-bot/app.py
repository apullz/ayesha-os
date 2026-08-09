import json
import os
import threading
import time

import gradio as gr
import requests

import ui

OLLAMA_SERVER = os.environ.get("OLLAMA_SERVER", "http://127.0.0.1:11434")
MODEL = "ayesha"


def wait_for_model(timeout=180):
    start = time.time()
    while time.time() - start < timeout:
        try:
            r = requests.get(f"{OLLAMA_SERVER}/api/tags", timeout=5)
            if r.status_code == 200:
                names = [m.get("name", "") for m in r.json().get("models", [])]
                if any(name.split(":")[0] == MODEL for name in names):
                    print(f"[ayesha] model '{MODEL}' ready after {time.time() - start:.0f}s", flush=True)
                    return
        except Exception:
            pass
        time.sleep(1)
    print(f"[ayesha] WARNING: model '{MODEL}' not ready after {timeout}s", flush=True)


def _extract_text(content):
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for block in content:
            if isinstance(block, dict):
                text = block.get("text")
                if text is not None:
                    parts.append(text)
        return "\n".join(parts)
    return str(content)


def _stream_chat(payload):
    with requests.post(
        f"{OLLAMA_SERVER}/api/chat", json=payload, stream=True, timeout=300
    ) as resp:
        resp.raise_for_status()
        full = ""
        n = 0
        for line in resp.iter_lines(decode_unicode=True):
            if not line:
                continue
            n += 1
            chunk = json.loads(line)
            if chunk.get("done"):
                break
            content = chunk.get("message", {}).get("content", "")
            if content:
                full += content
                yield full
            elif n % 20 == 0:
                yield full + "ayesha is sparking… ✧"
        print(f"[ayesha] /api/chat ok lines={n} len={len(full)}", flush=True)


def respond(message, history):
    messages = [
        {"role": h["role"], "content": _extract_text(h.get("content", ""))}
        for h in history
    ]
    messages.append({"role": "user", "content": message})

    payload = {
        "model": MODEL,
        "messages": messages,
        "stream": True,
        "think": False,
        "options": {
            "temperature": 0.9,
            "num_predict": 512,
        },
    }
    try:
        yield from _stream_chat(payload)
    except Exception as e:
        print(f"[ayesha] think=False failed ({e}); retrying without think", flush=True)
        payload.pop("think", None)
        try:
            yield from _stream_chat(payload)
        except Exception as e2:
            print(f"[ayesha] /api/chat error: {e2}", flush=True)
            yield f"[error talking to the model: {e2}]"


threading.Thread(target=wait_for_model, daemon=True).start()

demo = gr.ChatInterface(fn=respond)

demo.launch(
    server_name="0.0.0.0",
    server_port=7860,
    css=ui.CUSTOM_CSS,
    js=ui.CUSTOM_JS,
)
