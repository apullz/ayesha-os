╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   ███████╗ ██████╗██████╗ ███████╗███████╗███╗   ██╗       ║
║   ██╔════╝██╔════╝██╔══██╗██╔════╝██╔════╝████╗  ██║       ║
║   ███████╗██║     ██████╔╝█████╗  █████╗  ██╔██╗ ██║       ║
║   ╚════██║██║     ██╔══██╗██╔══╝  ██╔══╝  ██║╚██╗██║       ║
║   ███████║╚██████╗██║  ██║███████╗███████╗██║ ╚████║       ║
║   ╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═══╝       ║
║                                                              ║
║        screenshotai  ::  v1.0                                ║
║        "capture and analyze screenshots locally"             ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : screenshotai
  Version  : v1.0
  Type     : Vision Utility
  Engine   : Python 3.10+ / Ollama + moondream
  Model    : moondream (via local ollama)
  Neural   : 100% local, zero API calls

  ── D E S C R I P T I O N ──

  screenshotai captures your screen (or a selected region)
  and sends it to a local ollama vision model for analysis.
  the tool uses moondream (a lightweight 1.8b vision model)
  that runs entirely on your machine via ollama.

  two modes:
  - CLI: capture + question in one command
  - Daemon: background monitor that watches for changes

  ── I N S T A L L ──

  pip install Pillow ollama
  ollama pull moondream

  ── U S A G E ──

  # capture entire screen and describe it
  python screenshotai.py "what's on my screen?"

  # capture a region
  python screenshotai.py --region "read the text"
  (click and drag to select area)

  # daemon mode (polls every 10s)
  python screenshotai.py --watch "detect changes"

  ── A R C H I T E C T U R E ──

  screenshotai -> PIL.ImageGrab -> ollama /api/generate -> response
  the image is base64-encoded and sent inline. moondream
  handles the vision side, returning text descriptions.

  ── F I L E S ──

  screenshotai.py     ~150 lines  main capture + analysis
  main.py             ~100 lines  alternative entry point
  .github/agents/     directory   ayesha agent definitions

  ── N O T E S ──

  moondream is tiny (1.8b params) but fast. for better
  accuracy, swap ollama pull moondream with
  ollama pull llama3.2-vision and update the model name.

  ── G R E E T S ──

  moondream team for the excellent tiny vision model.
  ollama for keeping it local. capturin' pixels since '26.
