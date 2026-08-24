#!/usr/bin/env python3
import os, subprocess, time, json, sys, textwrap
from pathlib import Path
from datetime import datetime

REPO = Path(__file__).resolve().parent.parent
DURATION = 8 * 3600
INTERVAL = 30 * 60
ENGINE = REPO / "engine"
DIST = REPO / "dist"
PROGRESS = REPO / "progress.json"
KILO_URL = "https://api.kilo.ai/api/gateway/v1"

def log(m):
    ts = datetime.now().strftime("%H:%M:%S")
    print(f"[sleeper {ts}] {m}", flush=True)

def run(cmd, check=True, cwd=None, timeout=None):
    c = str(cwd or REPO)
    try:
        r = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=c, timeout=timeout)
        if r.stdout and r.stdout.strip():
            log(r.stdout.strip()[-600:])
        if check and r.returncode != 0:
            log(f"WARN: {cmd} -> {r.stderr.strip()[:400]}")
        return r
    except Exception as e:
        log(f"ERR: {e}")
        return None

def sync_to_github():
    log("syncing local copy to github ...")
    run("git add -A", check=False)
    run("git commit -m 'sleeper sync: mirror local ayesha-os' --allow-empty", check=False)
    run("git push origin master --force", check=False)

def cargo_test():
    log("running cargo test ...")
    r = run("rustup run stable-x86_64-pc-windows-msvc cargo test 2>&1", check=False, cwd=ENGINE, timeout=900)
    ok = r and r.returncode == 0
    log(f"cargo test {'ok' if ok else 'FAILED'}")
    return ok

def build_release():
    log("building ayesha-os engine (release) ...")
    r = run("rustup run stable-x86_64-pc-windows-msvc cargo build --release 2>&1", check=False, cwd=ENGINE, timeout=1200)
    ok = r and r.returncode == 0
    log(f"cargo build {'ok' if ok else 'FAILED'}")
    return ok

def selftest():
    exe = DIST / "ayesha-os.exe"
    if not exe.exists():
        log("dist/ayesha-os.exe not found, skipping selftest")
        return False
    log("running --selftest ...")
    r = run(f'"{exe}" --selftest', check=False, timeout=300)
    ok = r and r.returncode == 0
    log(f"selftest {'ok' if ok else 'FAILED'}")
    return ok

def director_prompt(cycle):
    return textwrap.dedent(f"""\
        you are ayesha-os, an autonomous agentic coding assistant working on yourself.
        it is cycle {cycle} of an 8-hour self-improvement session. your goal: make ayesha-os
        a better agentic coding agent (think JARVIS-level). this is a build phase — you can
        edit source code, run tests, and verify your changes.

        analyze the codebase and pick the single highest-impact improvement you can make
        right now in any of these subsystems:
        - engine/src/agent.rs (the core agentic loop, steering, mode directives)
        - engine/src/coding_agent.rs (read/write/edit/analyze/modify tools)
        - engine/src/tools.rs (26 tools — add, fix, or refine any)
        - engine/src/self_analysis.rs (source analysis + improvement prompts)
        - engine/src/tool_evolution.rs (LLM-driven tool generation)
        - engine/src/prompt_refinement.rs (system prompt auto-refinement)
        - engine/src/skills.rs (markdown skill discovery + injection)
        - engine/src/streaming.rs (token streaming, tool result handling)
        - engine/src/ui.rs (terminal UX, pixel-striker, kaomoji output)
        - engine/src/llm.rs / cloud.rs / model_registry.rs (model routing)
        - engine/src/sandbox.rs / permissions.rs (security model)
        - engine/src/memory.rs (persistent memory, sessions)
        - engine/src/theme.rs / syntax.rs / format.rs (presentation layer)
        - skills/*.md (add or refine skill guides)
        - applets/* (flora-cli, hivebeat, desktop-cat)
        - scripts/*.py + build-exe.ps1 (dev tooling)
        - ayesha.json / config schema

        prefer concrete, low-risk changes: fix an unwrap(), add a missing tool,
        refine a prompt, split a long line, resolve a TODO, improve error handling,
        add input validation, document a tricky bit, extract a magic constant.

        instructions:
        1. use read_file to inspect relevant source files.
        2. use edit or write to make your change (keep it focused — one change at a time).
        3. do NOT add comments unless asked. fix existing bugs.
        4. stop after one improvement is applied. you have ~30 min per cycle; do not
           overreach. quality over quantity.

        output format: after editing, print a single line: DONE: <brief description>.
        do not print anything else.
    """)

def drive_ayesha(cycle):
    """feed ayesha a director prompt and let her agent loop improve the codebase."""
    prompt = director_prompt(cycle)
    exe = DIST / "ayesha-os.exe"
    if exe.exists():
        cmd = f'"{exe}" --headless {json.dumps(prompt)}'
    else:
        # fall back to cargo run if dist exe doesn't exist yet
        cmd = f'rustup run stable-x86_64-pc-windows-msvc cargo run --release -- --headless {json.dumps(prompt)}'
    log(f"driving ayesha with director prompt (cycle {cycle}) ...")
    r = run(cmd, check=False, cwd=ENGINE, timeout=1500)
    out = (r.stdout or "") + (r.stderr or "") if r else ""
    # check for the DONE marker ayesha prints when finished
    done_line = ""
    for ln in out.splitlines():
        if ln.strip().upper().startswith("DONE:"):
            done_line = ln.strip()
            break
    if done_line:
        log(done_line)
    else:
        log("ayesha did not print DONE marker — continuing anyway")
    return done_line

def update_progress(cycle, build_ok, test_ok, selftest_ok, done_line):
    data = {
        "project": "ayesha-os autonomous sleeper",
        "version": "0.2.0",
        "status": "building",
        "last_cycle": cycle,
        "last_improvement": done_line,
        "build_ok": build_ok,
        "test_ok": test_ok,
        "selftest_ok": selftest_ok,
        "attempt_count": cycle,
    }
    try:
        PROGRESS.write_text(json.dumps(data, indent=2), encoding="utf-8")
    except Exception as e:
        log(f"could not write progress.json: {e}")

def commit_push(msg):
    run("git add -A", check=False)
    r = run(f'git commit -m "{msg}"', check=False)
    if r and r.returncode == 0:
        run("git push origin master --force", check=False)
        log(f"committed + force-pushed to github: {msg}")
    else:
        log("nothing to commit")

def main():
    start = time.time()
    log("autonomous sleeper started - 8h ayesha-os self-improvement mode")
    sync_to_github()

    if run("git rev-parse --is-inside-work-tree", check=False).returncode != 0:
        log("not a git repo, abort")
        return

    cycle = 0
    while time.time() - start < DURATION:
        cycle += 1
        log(f"=== cycle {cycle} ===")

        # 1. let ayesha decide + apply one improvement
        done_line = drive_ayesha(cycle)

        # 2. verify: cargo test
        test_ok = cargo_test()

        # 3. verify: build release
        build_ok = build_release() if test_ok else False

        # 4. verify: selftest on the dist binary
        if build_ok:
            ok = selftest()
            update_progress(cycle, build_ok, test_ok, ok, done_line)
        else:
            update_progress(cycle, False, test_ok, False, done_line)

        elapsed = int((time.time() - start) / 60)
        summary = f"build={'ok' if build_ok else 'fail'} test={'ok' if test_ok else 'fail'} selftest={'ok' if build_ok else 'skip'}"
        commit_push(f"sleeper cycle {cycle} ({elapsed}min) - {summary} - {done_line}")

        remaining = DURATION - (time.time() - start)
        if remaining > INTERVAL:
            log(f"sleeping 30min (remaining {int(remaining/60)}min)...")
            time.sleep(INTERVAL)

    log("8h cycle complete, final build + verify")
    test_ok = cargo_test()
    build_ok = build_release() if test_ok else False
    if build_ok:
        selftest()
    commit_push("autonomous sleeper: final self-improvement cycle complete")

if __name__ == "__main__":
    main()
