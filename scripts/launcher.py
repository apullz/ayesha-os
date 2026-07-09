"""
ayesha-os applet launcher — system tray companion
─────────────────────────────────────────────────
launch/stop any applet from the tray menu.
uses pystray (already a dependency via desktop-cat).

run:  python scripts/launcher.py
"""
import json, os, subprocess, sys, threading
from pathlib import Path

try:
    import pystray
    from PIL import Image, ImageDraw
except ImportError:
    print("missing deps: pip install pystray pillow")
    sys.exit(1)

ROOT = Path(__file__).resolve().parent.parent
CONFIG_PATH = ROOT / "ayesha.json"

_running: dict[str, subprocess.Popen] = {}


def load_applets():
    with open(CONFIG_PATH) as f:
        cfg = json.load(f)
    return {
        k: v for k, v in cfg.get("projects", {}).items()
        if v.get("run")
    }


def launch(name, entry):
    if name in _running and _running[name].poll() is None:
        return
    work_dir = ROOT / entry["path"]
    run_cmd = entry["run"]
    parts = run_cmd.split()
    program = "npm.cmd" if parts[0] == "npm" else "npx.cmd" if parts[0] == "npx" else parts[0]
    proc = subprocess.Popen(
        [program] + parts[1:],
        cwd=str(work_dir),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        stdin=subprocess.DEVNULL,
    )
    _running[name] = proc


def stop(name):
    proc = _running.pop(name, None)
    if proc and proc.poll() is None:
        proc.kill()
        proc.wait()


def stop_all():
    for name in list(_running.keys()):
        stop(name)


def make_icon():
    img = Image.new("RGBA", (64, 64), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw.ellipse([8, 8, 56, 56], fill=(0, 200, 255, 255))
    draw.text((18, 20), "A", fill=(0, 0, 0, 255))
    return img


def build_menu():
    applets = load_applets()
    items = []
    for name in sorted(applets.keys()):
        entry = applets[name]
        running = name in _running and _running[name].poll() is None
        label = f"\u25c9  {name}" if running else f"\u25cb  {name}"
        def cb(n=name, e=entry):
            if n in _running and _running[n].poll() is None:
                stop(n)
            else:
                launch(n, e)
            rebuild(icon)
        items.append(pystray.MenuItem(label, cb))
    items.append(pystray.Menu.SEPARATOR)
    items.append(pystray.MenuItem("Stop All", lambda: (stop_all(), rebuild(icon))))
    items.append(pystray.Menu.SEPARATOR)
    items.append(pystray.MenuItem("Quit", lambda: (stop_all(), icon.stop())))
    return pystray.Menu(*items)


def rebuild(icon_instance):
    icon_instance.menu = build_menu()
    icon_instance.update_menu()


if __name__ == "__main__":
    icon = pystray.Icon("ayesha-launcher", make_icon(), "ayesha-os", build_menu())
    icon.run()
