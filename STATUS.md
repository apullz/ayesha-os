╔═══════════════════════════════════════════════╗
║       ayesha-os  ::  STATUS REPORT  ::         ║
║          "[RELEASE] v4.2.0"                   ║
╚═══════════════════════════════════════════════╝

  ── L E G E N D ──

  [✔] working
  [!] works with caveats
  [✖] broken / not started
  [→] requires local ollama model

  ── P R O J E C T S ──

  engine/          [✔]  CLI persona host with streaming,
                         model routing, tool-calling,
                         retro cyberpunk UI, command palette
                         └ model: ayesha (qwen2.5-coder:14b base)

  core/            [✔]  Gradio web UI, mobile API,
                         tri-node hive sync
                         └ needs ollama running locally

  screen/          [→]  Vision chatbox using ollama
                         llama3.2-vision + qwen2.5-coder:14b
                         └ replaced gemini endpoints 07/2026

  cosmic-rag/      [→]  Local RAG chatbot with vault
                         └ ollama llama3.2 needed

  screenshotai/    [→]  Screenshot capture + analysis
                         └ uses moondream via ollama

  desktop-cat/     [✔]  Desktop pet (no AI needed)
                         └ pure tkinter, always works

  flora-cli/       [→]  Scottish flora phylo explorer
                         └ replaced gemini with ollama 07/2026
                         └ "ask" command -> qwen2.5-coder:14b

  bring-to-life/   [→]  Image-to-interactive-HTML app
                         └ replaced gemini with ollama 07/2026
                         └ vision -> llama3.2-vision, text -> qwen2.5-coder:14b

  neural-strike/   [!]  SAE interpretability game
                         └ local-only (neuronpedia stubbed)
                         └ requires pre-downloaded data exports

  models/          [✔]  Modelfile for ayesha personality
                           └ FROM qwen2.5-coder:14b

  launcher/        [✔]  In-engine applet switcher
                          └ Ctrl+M opens launcher mode
                          └ /run, /stop, /apps slash commands
                          └ /appletname shortcut launches applets
                          └ scripts/launcher.py (tray companion)

  ── K N O W N   I S S U E S ──

  ! engine: auto-routing defaults to qwen2.5-coder:14b
    but detect() pulls all locally available models
    └ if you have gemma models installed, they'll still show in /models

  ! neural-strike: steer() and UMAP fetch disabled
    └ drop JSON exports into data/exports/ to use cached features

  ! HF Space: hotkeys and command palette removed 07/2026
    └ app.py currently crashing (SyntaxError) — redeploy fixed version

  ── S E C U R I T Y   N O T E S ──

  * no API keys committed — all .env* in .gitignore
  * git author changed to noreply email
  * 0.0.0.0 binds changed to 127.0.0.1
  * no personal paths or system info in source

  ── M O D E L S   N E E D E D ──

  ollama pull qwen2.5-coder:14b  # default text + coding model
  ollama pull llama3.2-vision    # vision tasks
  ollama pull moondream          # screenshotai
  ollama create ayesha -f models/Modelfile  # custom personality
