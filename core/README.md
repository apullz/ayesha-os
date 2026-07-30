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
║     "web ui, mobile api, hivemind client"                    ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── D E S C R I P T I O N ──

  core is the control center of the ayesha hivemind. it
  provides a gradio web interface, a fastapi mobile API,
  and a client library for hivemind instance sync.

  ── C O M P O N E N T S ──

  gradio web ui (app.py)
    personality engine with three-layer response system:
    - computer (starfleet logic)
    - otacon (geek panic)
    - win95 (retro glitch)
    serves via gradio on port 7860

  fastapi mobile api (ayesha_mobile_api.py)
    REST API for mobile apps on port 8001:
    - POST /api/mobile/register   — device registration
    - POST /api/mobile/{id}/heartbeat — keepalive
    - GET  /api/hive/status       — hive status
    - GET  /api/hive/instances    — list instances
    - GET  /api/hive/personality  — get config
    - POST /api/hive/broadcast    — broadcast update
    - WS   /ws/hive/{id}          — real-time updates

  hivemind client (ayesha_hive_client.py)
    client library for instances to:
    - register with the hivemind
    - send heartbeats
    - sync state
    - discover sister instances (5-min window)
    - retrieve personality config

  ── I N S T A L L ──

  pip install -r requirements.txt

  ── U S A G E ──

  # start web interface
  python app.py

  # start mobile api
  python ayesha_mobile_api.py

  ── F I L E S ──

  app.py                          gradio web interface
  ayesha_hive_client.py           hivemind client library
  ayesha_mobile_api.py            fastapi mobile REST API
  mobile_app_hive_integrated.py   kivy mobile app (hive-connected)
  requirements.txt                pip deps

  ── N O T E S ──

  - all AI calls go through local ollama — no external APIs
  - the gradio ui has a static demo; real personality is in engine
  - the HF Space at apullz/ayesha-hivemind mirrors core/
  - mobile API binds to 127.0.0.1:8001

  ── G R E E T S ──

  gradio team for the easiest web ui library.
  fastapi for the clean async api framework.
  the hivemind is alive!! kapoo!! :3
