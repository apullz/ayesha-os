#!/usr/bin/env python3
import os, re, subprocess, time, sys
from pathlib import Path
from datetime import datetime

REPO = Path(__file__).resolve().parent.parent
DURATION = 8 * 3600
INTERVAL = 30 * 60
KILO_URL = "https://api.kilo.ai/api/gateway/v1"

def log(m):
    ts = datetime.now().strftime("%H:%M:%S")
    print(f"[sleeper {ts}] {m}", flush=True)

def run(cmd, check=True):
    try:
        r = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=str(REPO))
        if check and r.returncode != 0:
            log(f"WARN: {cmd} -> {r.stderr.strip()[:200]}")
        return r
    except Exception as e:
        log(f"ERR: {e}")
        return None

def sweep():
    pats = [
        ("localhost:11434", KILO_URL),
        ("127.0.0.1:11434", "api.kilo.ai"),
        ("qwen2.5:7b", "kilo-auto/free"),
        ("qwen2.5:0.5b", "kilo-auto/free"),
        ("qwen2.5:3b", "kilo-auto/free"),
        ("qwen2.5-coder:14b", "kilo-auto/free"),
        ("llama3.2-vision", "kilo-auto/free"),
        ("mistral-nemo:12b", "kilo-auto/free"),
        ("deepseek-coder:33b", "kilo-auto/free"),
        ("nemotron-3-nano:4b", "kilo-auto/free"),
        ("OllamaClient", "LlmClient"),
        ("ollama_endpoint", "kilo_endpoint"),
        ("ollama pull", "kilo pull"),
        ("ollama create", "kilo create"),
        ("apullz/ayesha-os", "ayesha-os/ayesha-os"),
        ("apullz/ayesha", "ayesha-hivemind/ayesha"),
    ]
    exts = ["*.rs", "*.py", "*.ts", "*.tsx", "*.sh", "*.ps1", "*.md", "*.json", "*.toml"]
    changed = 0
    for ext in exts:
        cmd = f"find {REPO} -name \"{ext}\" -not -path \"{REPO}/.git/*\" -not -path \"*/node_modules/*\" -not -path \"*/target/*\" -not -path \"*/dist/*\" -not -path \"*/__pycache__/*\" -not -path \"*/.tri_mind_state/*\""
        r = run(cmd, check=False)
        if r and r.stdout.strip():
            for fp in r.stdout.strip().split("\n"):
                fp = fp.strip()
                if not fp or not os.path.isfile(fp):
                    continue
                try:
                    c = open(fp, encoding="utf-8").read()
                    o = c
                    for p, r2 in pats:
                        c = c.replace(p, r2)
                    if c != o:
                        open(fp, "w", encoding="utf-8").write(c)
                        changed += 1
                except:
                    pass
    log(f"sweep: {changed} files changed")

def cleanup_dist():
    d = REPO / "dist" / "applets"
    if d.exists():
        for ad in d.iterdir():
            if ad.is_dir():
                for sub in ["node_modules", "src", "dist"]:
                    p = ad / sub
                    if p.exists():
                        run(f"rm -rf {p}", check=False)
    log("dist cleanup done")

def commit_push(msg):
    run("git add -A", check=False)
    r = run(f"git commit -m \"{msg}\"", check=False)
    if r and r.returncode == 0:
        run("git push origin master", check=False)
        log(f"committed: {msg}")
    else:
        log("nothing to commit")

def main():
    start = time.time()
    log("autonomous sleeper started - 8h mode")
    if not (run("git rev-parse --is-inside-work-tree", check=False).returncode == 0):
        log("not a git repo, abort")
        return
    cycle = 0
    while time.time() - start < DURATION:
        cycle += 1
        log(f"--- cycle {cycle} ---")
        sweep()
        cleanup_dist()
        elapsed = int((time.time() - start) / 60)
        commit_push(f"autonomous sleeper cycle {cycle} ({elapsed}min)")
        if time.time() - start < DURATION:
            log(f"sleeping 30min...")
            time.sleep(INTERVAL)
    log("8h cycle complete")
    commit_push("autonomous sleeper: final")

if __name__ == "__main__":
    main()