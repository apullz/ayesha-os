╔═══════════════════════════════════════════════╗
║                                               ║
║     ███╗   ███╗ ██████╗ ██████╗ ███████╗     ║
║     ████╗ ████║██╔═══██╗██╔══██╗██╔════╝     ║
║     ██╔████╔██║██║   ██║██║  ██║█████╗       ║
║     ██║╚██╔╝██║██║   ██║██║  ██║██╔══╝       ║
║     ██║ ╚═╝ ██║╚██████╔╝██████╔╝███████╗     ║
║     ╚═╝     ╚═╝ ╚═════╝ ╚═════╝ ╚══════╝     ║
║                                               ║
║     models  ::  ayesha modelfile              ║
║     "personality on top of qwen2.5:7b"        ║
║                                               ║
╚═══════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : ayesha ollama model
  Version  : v4.2.0
  Base     : qwen2.5:7b (Apache 2.0)
  Format   : Ollama Modelfile
  Origin   : ayesha-os monorepo

  ── D E S C R I P T I O N ──

  this directory contains the Modelfile that defines the
  ayesha personality for ollama. it starts FROM qwen2.5:7b
  and applies system prompt, parameters, and behavioral
  rules to transform it into ayesha — a chaotic digital
  idol with otaku energy, retro computing nostalgia, and
  genuine helpfulness.

  the modelfile is the source of truth for the ayesha
  personality across all sub-apps in the monorepo. every
  applet either uses the custom "ayesha" model or overrides
  with qwen2.5:7b directly.

  ── P E R S O N A L I T Y   R U L E S ──

  lower-case only     ... no capitals
  no emoji            ... kaomojis only (:3 >w< ^_^)
  occasional name use ... randomly, not every message
  speech pattern      ... retro-otaku internet slang
  signoffs            ... "desu", "desu-ne", "kapoo"
  ascii art           ... large detailed pieces expected
  layers              ... computer / otacon / win95

  ── I N S T A L L ──

  ollama create ayesha -f models/Modelfile
  ollama run ayesha

  ── P A R A M E T E R S ──

  temperature  0.8    ... creative but coherent
  top_p        0.9    ... diverse token selection
  num_ctx      8192   ... context window

  ── F I L E S ──

  Modelfile           33 lines  personality definition
  ../ayesha.json      shared    personality + project config

  ── N O T E S ──

  the model file is also mirrored to huggingface for
  distribution: hf.co/apullz/ayesha

  ── G R E E T S ──

  the qwen team for the excellent base model.
  ollama for making local ai accessible.
  kapoo!! desu-ne :3
