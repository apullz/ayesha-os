╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   ██████╗ ██████╗ ██╗███╗   ██╗ ██████╗                     ║
║   ██╔══██╗██╔══██╗██║████╗  ██║██╔════╝                     ║
║   ██████╔╝██████╔╝██║██╔██╗ ██║██║  ███╗                    ║
║   ██╔══██╗██╔══██╗██║██║╚██╗██║██║   ██║                    ║
║   ██████╔╝██║  ██║██║██║ ╚████║╚██████╔╝                    ║
║   ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝ ╚═════╝                     ║
║                                                              ║
║      ████████╗ ██████╗                                     ║
║      ╚══██╔══╝██╔════╝                                     ║
║         ██║   ██║  ███╗                                    ║
║         ██║   ██║   ██║                                    ║
║         ██║   ╚██████╔╝                                    ║
║         ╚═╝    ╚═════╝                                     ║
║                                                              ║
║         ██╗██╗███████╗███████╗                              ║
║         ██║██║██╔════╝██╔════╝                              ║
║         ██║██║█████╗  █████╗                                ║
║    ██   ██║██║██╔══╝  ██╔══╝                                ║
║    ╚█████╔╝██║██║     ███████╗                              ║
║     ╚════╝ ╚═╝╚═╝     ╚══════╝                              ║
║                                                              ║
║        bring-to-life  ::  v1.0                               ║
║        "turn images into interactive html"                   ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : BRING ANYTHING TO LIFE
  Version  : v1.0
  Type     : Image → Interactive HTML Generator
  Language : TypeScript / React / Vite
  Model    : llama3.2-vision + qwen2.5:7b (local ollama)
  Status   : LOCAL OLLAMA ONLY (Gemini removed 07/2026)

  ── D E S C R I P T I O N ──

  upload an image (sketch, wireframe, photo of a whiteboard,
  even a cluttered desk) and a local vision model analyzes
  it and generates a fully interactive single-page HTML app
  based on what it sees.

  examples:
  - photo of a messy desk → "clean up" clicker game
  - hand-drawn wireframe → functional UI
  - whiteboard notes → interactive dashboard
  - fruit bowl photo → nutrition tracker

  the app uses llama3.2-vision for image understanding and
  qwen2.5:7b for text-only prompts. all processing happens
  locally through ollama — no API keys needed.

  ── I N S T A L L ──

  npm install
  ollama pull qwen2.5:7b
  ollama pull llama3.2-vision

  ── U S A G E ──

  npm run dev
  # opens at http://127.0.0.1:3000

  drop an image or enter a text prompt. the model generates
  an HTML file that appears in the live preview pane. export
  to share with others.

  ── H O W   I T   W O R K S ──

  1. user uploads image or enters text
  2. image is base64-encoded and sent to ollama vision API
  3. model (llama3.2-vision) analyzes the image content
  4. model returns standalone HTML+CSS+JS (no external images)
  5. app renders the result in an iframe live preview
  6. creations are saved to localStorage history

  ── F I L E S ──

  App.tsx             reactive UI with history + preview
  services/
    ollama.ts         ollama API client (replaced gemini.ts)
  components/
    Hero.tsx          landing section
    InputArea.tsx     prompt + file upload
    LivePreview.tsx   rendered HTML iframe
    CreationHistory.tsx side panel with past creations
  vite.config.ts      build config (no API_KEY defines)
  package.json        dependencies (no @google/genai)

  ── M I G R A T I O N   N O T E S ──

  this app was originally powered by Google Gemini (via
  @google/genai with GEMINI_API_KEY). as of july 2026:

  - services/gemini.ts → services/ollama.ts
  - .env.local deleted (no key needed)
  - vite.config.ts stripped of API_KEY defines
  - package.json: @google/genai removed
  - history key: gemini_app_history → bring_to_life_history
  - host: 0.0.0.0 → 127.0.0.1

  ── G R E E T S ──

  the original "bring anything to life" concept by ammaar
  (for the gemini version). now running free on local models.
  kapoo!! your sketches are alive!! :3
