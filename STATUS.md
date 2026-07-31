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

  engine/          [✔]  Agentic coding assistant + jarvis chatbot.
                         CLI persona host with streaming,
                         model routing, tool-calling (20 tools),
                         cloud support (openrouter/opencode),
                         pixel striker sprite engine,
                         self-improvement loop, command palette,
                         retro cyberpunk UI, applet manager.
                         └ model: ayesha (qwen2.5-coder:14b base)
                         └ --selftest for headless E2E verification

  core/            [✔]  Gradio web UI, FastAPI mobile API,
                         hivemind client for instance sync.
                         └ needs ollama running locally

  tri_mind_sync/   [✔]  Bidirectional sync engine
                         (github, huggingface, local).
                         Fixed: now pushes modified files.

  git_middleware/   [✔]  Gitea webhook receiver + LLM task runner.
                         Code review, security scan, auto-summary.

  desktop-cat/     [✔]  Desktop pet (no AI needed).
                         └ pure tkinter + Win32 API

  flora-cli/       [→]  Scottish flora phylo explorer.
                         └ replaced gemini with ollama 07/2026

  neural-strike/   [!]  SAE interpretability game.
                         └ local-only (neuronpedia stubbed)
                         └ requires pre-downloaded data exports

  poopy-tui/       [✔]  Discord terminal client with voice + TUI.
                         └ login: token / QR / email+password
                         └ token persisted to ~/.poopy-tui/token
                         └ requires DISCORD_TOKEN in .env OR first-run login

  models/          [✔]  Modelfile for ayesha personality.
                           └ FROM qwen2.5-coder:14b

  launcher/        [✔]  In-engine applet switcher.
                          └ Ctrl+M opens launcher mode
                          └ Ctrl+P opens page switcher (in-window applets)
                          └ /run, /stop, /apps slash commands
                          └ foreground applets (flora-cli, poopy-tui) run in
                            the current window as pages; ctrl+p switches back
                          └ scripts/launcher.py (tray companion)

  ── K N O W N   I S S U E S ──

  ! neural-strike: steer() and UMAP fetch disabled
    └ drop JSON exports into data/exports/ to use cached features

  ! poopy-tui: requires DISCORD_TOKEN in .env
    └ uses discord.py-self for user account access

  ── S E C U R I T Y   N O T E S ──

  * no API keys committed — all .env* in .gitignore
  * git author changed to noreply email
  * 0.0.0.0 binds changed to 127.0.0.1
  * no personal paths or system info in source
  * sandbox blocks sensitive paths (.env, .ssh, .gnupg, .aws)

  ── M O D E L S   N E E D E D ──

  ollama pull qwen2.5-coder:14b  # default text + coding model
  ollama pull llama3.2-vision    # vision tasks
  ollama create ayesha -f models/Modelfile  # custom personality

  ── C L O U D   M O D E L S   (f r e e) ──

  openrouter:   nvidia/nemotron-3-super:free
                meta-llama/llama-3.3-70b-instruct:free
                deepseek/deepseek-r1:free
                qwen/qwen-2.5-coder-32b-instruct:free
  opencode:     opencode/big-pickle

  ── D E P L O Y M E N T ──

  github:       git push origin master
  huggingface:  $env:HF_TOKEN = "hf_..."; .\scripts\sync-all.ps1
  standalone:   .\scripts\build-exe.ps1 -> dist\ayesha-os.exe
