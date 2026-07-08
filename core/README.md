╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║      ██████╗ ██████╗ ██████╗ ███████╗                       ║
║     ██╔════╝██╔═══██╗██╔══██╗██╔════╝                       ║
║     ██║     ██║   ██║██████╔╝█████╗                         ║
║     ██║     ██║   ██║██╔══██╗██╔══╝                         ║
║     ╚██████╗╚██████╔╝██║  ██║███████╗                       ║
║      ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝                       ║
║                                                              ║
║     core  ::  hivemind orchestrator                          ║
║     "web ui, mobile api, tri-node sync"                      ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : ayesha-core (hivemind)
  Version  : v2.0.0
  Type     : Web UI / API Server / Sync Engine
  Language : Python 3.10+
  Stack    : Gradio, FastAPI, WebSockets, SQLite
  Model    : ayesha (via ollama, local only)
  Status   : FULLY OPERATIONAL — 3 nodes

  ── D E S C R I P T I O N ──

  core is the control center of the ayesha hivemind. it
  provides a gradio web interface, a mobile REST API, and
  a tri-node synchronization engine that keeps GitHub,
  local PC, and HuggingFace Spaces in sync.

  the hivemind architecture allows ayesha's personality to
  exist across multiple nodes simultaneously. changes to
  the personality config on one node propagate to all others
  via the sync engine.

  ── C O M P O N E N T S ──

  gradio web ui (app.py)
    retro terminal-themed interface with:
    - chat with ayesha personality
    - theme picker (matrix, cyberpunk, amber, oceandeep, holo)
    - file browser
    - personality demo (static — run engine for real)

  mobile api (ayesha_mobile_api.py)
    fastapi server on port 8001:
    - POST /api/mobile/register
    - POST /api/mobile/{uuid}/heartbeat
    - GET /api/hive/status
    - GET /api/hive/instances
    - GET /api/hive/personality
    - POST /api/hive/broadcast
    - WS /ws/hive/{uuid} (websocket)

  sync engine (ayesha_sync.py)
    multi-threaded sync loop that:
    - monitors git remotes for changes
    - pulls/pushes to GitHub
    - distributes personality config
    - manages instance registration

  tri-node mind (tri_node_mind.py)
    orchestrator for the tri-node architecture:
    - node: GitHub (source of truth)
    - node: Local PC (orchestrator)
    - node: HuggingFace (public interface)

  ── I N S T A L L ──

  pip install gradio fastapi uvicorn websockets requests

  ── U S A G E ──

  # start web interface
  python app.py

  # start mobile api
  python ayesha_mobile_api.py

  # start sync loop
  python ayesha_sync.py loop

  # start tri-node orchestrator
  python tri_node_mind.py loop

  ── F I L E S ──

  app.py                          gradio web interface
  ayesha_hive_client.py           client lib for hive nodes
  ayesha_mobile_api.py            fastapi mobile rest api
  ayesha_sync.py                  multi-threaded sync engine
  tri_node_mind.py                tri-node orchestrator
  mobile_app_hive_integrated.py   kivy mobile app
  mobile_app.py                   alternative mobile client
  Modelfile                       ayesha personality
  manifest.json                   version config
  setup.py                        package setup
  buildozer.spec                  android build spec
  requirements.txt                pip deps
  .github/workflows/              CI/CD pipelines

  ── A R C H I T E C T U R E ──

      GitHub (source of truth)
         │
    ayesha_sync ───── local PC (orchestrator)
         │                 │
    tri_node_mind    ayesha_mobile_api
         │                 │
  HuggingFace Space    mobile clients
      (web UI)        (android/ios/web)

  ── N O T E S ──

  - all AI calls go through local ollama — no external APIs
  - the gradio ui in app.py has a static demo; the real
    personality runs in the engine
  - the HF Space at /apullz/ayesha-hivemind mirrors core/
  - mobile API binds to 127.0.0.1:8001 (was 0.0.0.0 before
    security fix 07/2026)

  ── G R E E T S ──

  gradio team for the easiest web ui library.
  fastapi for the clean async api framework.
  the hivemind is alive!! kapoo!! :3
