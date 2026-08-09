---
title: Ayesha Bot
emoji: 🌸
colorFrom: pink
colorTo: purple
sdk: docker
pinned: false
---

# ayesha-bot

Backend for the ayesha-bot mobile app (React Native / Expo).

- **Model**: `nemotron-3-nano:4b` via [Ollama](https://ollama.com), baked with the ayesha Modelfile at build time.
- **UI**: The "Magical Chat" pastel phone UI (from the ayesha-bot v0 prototype) rendered as a custom overlay on the Gradio app, port 7860 (SSE v3 streaming API: `POST /gradio_api/call/respond`).
- **Personality**: ayesha — 33-year-old virtual kitten from japan, half hatsune miku, half tachikoma. Lower-case only, kaomoji not emoji, ends things with "desu"/"kapoo", calls the user "apullz" or "fox".

## Layout

- `app.py` — Gradio `ChatInterface` backend; streams responses from the ollama `ayesha` model.
- `ui.py` — loads the static UI assets and embeds the avatar as a data URI; exposes `CUSTOM_CSS` / `CUSTOM_JS`.
- `overlay.html` / `theme.css` / `chat.js` — the phone-frame chat UI. `chat.js` discovers the chat endpoint from `/config` (works on gradio 5 and 6), then streams via SSE.

## Chat API (used by the mobile app)

```http
POST /gradio_api/call/respond
Content-Type: application/json

{"data": ["hello ayesha!", []]}
```

→ `{"event_id": "..."}` then:

```http
GET /gradio_api/call/respond/{event_id}
```

→ SSE `event: generating` / `data: ["<accumulated text>", null]` … then `event: complete`.
