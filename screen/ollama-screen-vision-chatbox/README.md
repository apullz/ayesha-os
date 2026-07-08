╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║    ███████╗ ██████╗██████╗ ███████╗███████╗███╗   ██╗      ║
║    ██╔════╝██╔════╝██╔══██╗██╔════╝██╔════╝████╗  ██║      ║
║    ███████╗██║     ██████╔╝█████╗  █████╗  ██╔██╗ ██║      ║
║    ╚════██║██║     ██╔══██╗██╔══╝  ██╔══╝  ██║╚██╗██║      ║
║    ███████║╚██████╗██║  ██║███████╗███████╗██║ ╚████║      ║
║    ╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═══╝      ║
║                                                              ║
║      ██╗   ██╗██╗███████╗██╗ ██████╗ ███╗   ██╗            ║
║      ██║   ██║██║██╔════╝██║██╔═══██╗████╗  ██║            ║
║      ██║   ██║██║███████╗██║██║   ██║██╔██╗ ██║            ║
║      ╚██╗ ██╔╝██║╚════██║██║██║   ██║██║╚██╗██║            ║
║       ╚████╔╝ ██║███████║██║╚██████╔╝██║ ╚████║            ║
║        ╚═══╝  ╚═╝╚══════╝╚═╝ ╚═════╝ ╚═╝  ╚═══╝            ║
║                                                              ║
║       ██████╗██╗  ██╗ █████╗ ████████╗██████╗  ██████╗     ║
║      ██╔════╝██║  ██║██╔══██╗╚══██╔══╝██╔══██╗██╔═══██╗    ║
║      ██║     ███████║███████║   ██║   ██████╔╝██║   ██║    ║
║      ██║     ██╔══██║██╔══██║   ██║   ██╔══██╗██║   ██║    ║
║      ╚██████╗██║  ██║██║  ██║   ██║   ██████╔╝╚██████╔╝    ║
║       ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═════╝  ╚═════╝     ║
║                                                              ║
║        ollama-screen-vision-chatbox  ::  v4.0                ║
║        "real-time screen analysis with local vision ai"      ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : AYESHA SCREEN VISION CHATBOX
  Version  : v4.0
  Type     : Screen Capture + Vision Analysis + Chat
  Stack    : TypeScript / Express / React / Vite
  Models   : llama3.2-vision (vision) + qwen2.5:7b (chat)
  Status   : FULLY LOCAL — no external APIs since 07/2026
  Port     : 3000 (127.0.0.1 only)

  ── D E S C R I P T I O N ──

  this is the main screen analysis app in the ayesha-os
  family. it combines a python screen capture monitor with
  a full-stack express + react web ui for real-time vision
  analysis using local ollama models.

  features:
  - live screen capture via python companion script
  - upload or paste screenshots
  - vision analysis with 5 task modes:
    * general analysis (default)
    * ocr (text extraction)
    * ui-review (layout critique)
    * detector (interactive element detection)
    * bug-report (visual QA)
  - text chat with ayesha personality (qwen2.5:7b)
  - direct ollama proxy endpoint for API access
  - fullscreen live preview

  ── I N S T A L L ──

  npm install
  ollama pull llama3.2-vision
  ollama pull qwen2.5:7b

  # optional: python companion for live screen capture
  pip install Pillow requests
  python desktop/ayesha_companion_terminal.py

  ── U S A G E ──

  npm run dev
  # opens at http://127.0.0.1:3000

  # for live screen capture, run the python companion:
  python desktop/ayesha_companion_terminal.py

  # then in the web ui, click "capture screen" to pull
  # the latest frame from the companion's in-memory store

  ── A P I   E N D P O I N T S ──

  POST /api/screen/update      python -> server (frame push)
  GET  /api/screen/latest      web ui -> server (frame pull)
  POST /api/vision/analyze     vision analysis (llama3.2-vision)
  POST /api/chat               text chat (qwen2.5:7b)
  POST /api/ollama/proxy       direct ollama proxy (SSRF-safe)
  GET  /api/health             health check

  ── M I G R A T I O N   N O T E S ──

  this app was originally powered by Google Gemini for both
  vision analysis and chat. as of july 2026:

  - /api/gemini/analyze → /api/vision/analyze (llama3.2-vision)
  - /api/gemini/chat → /api/chat (qwen2.5:7b)
  - @google/genai dependency removed
  - .env.example: GEMINI_API_KEY → OLLAMA_HOST
  - host: 0.0.0.0 → 127.0.0.1
  - hotkeys/command UI removed from frontend

  ── A R C H I T E C T U R E ──

  python companion
       │ POST /api/screen/update
       ▼
  express server (this app)
       │
       ├── GET  /api/screen/latest  →  react UI renders frame
       ├── POST /api/vision/analyze →  llama3.2-vision
       ├── POST /api/chat           →  qwen2.5:7b
       └── POST /api/ollama/proxy   →  direct ollama (any model)

  ── F I L E S ──

  server.ts               express backend (all endpoints)
  index.html              vite entry point
  package.json            dependencies
  .env.example            config (no api keys)
  src/
    index.css             retro cyberpunk styles
    (react components)    frontend source
  desktop/
    ayesha_companion_terminal.py  python screen capture

  ── N O T E S ──

  - the ollama proxy endpoint has SSRF protection — only
    allows configured localhost hosts
  - vision analysis uses llama3.2-vision for efficiency
    (swap to moondream for even lower resource usage)
  - the companion terminal's windows build number banner
    was replaced with a generic version string (security fix)
  - all 0.0.0.0 binds changed to 127.0.0.1

  ── G R E E T S ──

  llama3 vision team for the excellent local vision model.
  express team for the robust backend framework.
  your screen is being watched. locally. desu-ne!! :3
