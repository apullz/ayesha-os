╔═══════════════════════════════════════════════╗
║                                               ║
║     ███████╗ ██████╗██████╗ ███████╗███████╗ ║
║     ██╔════╝██╔════╝██╔══██╗██╔════╝██╔════╝ ║
║     ███████╗██║     ██████╔╝█████╗  █████╗   ║
║     ╚════██║██║     ██╔══██╗██╔══╝  ██╔══╝   ║
║     ███████║╚██████╗██║  ██║███████╗███████╗ ║
║     ╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝ ║
║                                               ║
║     screen  ::  vision applet                 ║
║     "ollama-powered screen analysis"          ║
║                                               ║
╚═══════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : screen vision chatbox
  Version  : v4.0
  Type     : Vision Chat / Screen Analysis
  Stack    : TypeScript / Express / React / Ollama
  Models   : llama3.2-vision + qwen2.5:7b
  Status   : LOCAL ONLY — no external APIs since 07/2026

  ── D E S C R I P T I O N ──

  the screen/ directory contains a full-stack screen analysis
  app. it captures or receives screenshots, sends them to a
  local ollama vision model (llama3.2-vision) for analysis,
  and provides a chat interface for follow-up questions.

  key features:
  - screen capture via python companion script
  - vision analysis with task modes (ocr, ui-review, detector)
  - chat fallback using qwen2.5:7b
  - ollama proxy endpoint with SSRF protection
  - retro cyberpunk ui

  ── S U B - A P P L E T S ──

  ollama-screen-vision-chatbox/
    the main app — express server + react frontend
    see its README for details

  ── A R C H I T E C T U R E ──

  python monitor -> POST /api/screen/update -> in-memory frame
  web UI        -> GET  /api/screen/latest -> renders frame
  web UI        -> POST /api/vision/analyze -> llama3.2-vision
  web UI        -> POST /api/chat -> qwen2.5:7b
  web UI        -> POST /api/ollama/proxy -> direct ollama access

  ── Q U I C K   S T A R T ──

  cd ollama-screen-vision-chatbox
  npm install
  npm run dev
  # open http://127.0.0.1:3000

  ── F I L E S ──

  ollama-screen-vision-chatbox/   full app (express + react)
  README.md                       this file

  ── N O T E S ──

  the gemini endpoints (/api/gemini/analyze, /api/gemini/chat)
  were replaced with local ollama endpoints in july 2026.
  the .env.example now uses OLLAMA_HOST instead of GEMINI_API_KEY.

  ── G R E E T S ──

  llama3 team for the vision model. express for the backend.
  kapoo! screens analyzed, desu-ne :3
