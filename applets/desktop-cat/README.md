╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║        ▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄       ║
║        █       ██       ██       ██       ██       █      ║
║        █    ▄▄▄██    ▄▄▄██    ▄███    ▄▄▄██    ▄▄▄█      ║
║        █   █▄▄▄▄▄█   █▄▄▄▄▄█    █▄▄ █   █▄▄▄▄▄█   █▄▄▄▄▄      ║
║        █    ▄▄▄▄▄█    ▄▄▄▄▄█    ▄▄▄█    ▄▄▄▄▄█    ▄▄▄▄▄█      ║
║        █   █▄▄▄▄▄█   █      █   █▄▄▄█   █▄▄▄▄▄█   █▄▄▄▄▄      ║
║        █       ██       ██       ██       ██       █      ║
║        ▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀   ▀▀▀▀▀▀▀       ║
║                                                              ║
║        desktop-cat  ::  v1.0                                 ║
║        "a digital kitty for your desktop"                    ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : desktop-cat
  Version  : v1.0
  Type     : Desktop Pet / Toy
  Engine   : Python 3.10+ / Tkinter
  Platform : Windows
  Origin   : ayesha-os monorepo

  ── D E S C R I P T I O N ──

  a simple tkinter desktop pet. a pixel-art cat that follows
  your cursor around the screen, sits, sleeps, scratches,
  and shows hearts. no AI, no network, no ollama. pure
  nostalgic desktop companion software.

  written in ~200 lines of python. uses PIL for sprite
  loading and tkinter for the transparent overlay window.
  zero dependencies beyond the python standard library.

  ── I N S T A L L ──

  1. ensure python 3.10+ is installed
  2. pip install Pillow
  3. python desktopcat.py

  ── U S A G E ──

  the cat follows your mouse. that's it.
  - moves at a gentle lerp speed
  - sits and idles when you stop
  - sleeps after 30s of inactivity
  - shows hearts periodically >w<
  - right-click menu to quit

  ── F I L E S ──

  desktopcat.py        ~200 lines  main app (tkinter overlay)
  sprites/             directory   pixel art cat frames

  ── N O T E S ──

  this is the simplest app in the ayesha-os lineup. no
  configurability, no commands, no api. it's just a cat
  on your screen. and that's beautiful.

  ── G R E E T S ──

  dedicated to every dev who needed a cat on their desktop
  while writing 10,000 lines of rust. kapoo!! :3
