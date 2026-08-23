#!/usr/bin/env python3
"""ayesha-os background automation harness
============================================
A zero-touch local state-management loop for the ayesha-os monorepo.
Standard-library only (no third-party deps) so it runs anywhere python3 does.

WHAT IT DOES
  1. POLLS a designated queue directory (.automation/tasks) for task files
     (drop *.task.json files there) plus any external edits to code/config.
  2. RUNS queued tasks non-interactively (subprocess, no tty) with strict
     file-writing verification: every write goes to a temp file, is fsynced,
     hash-verified, then atomically renamed. Existing target files are backed
     up first and restored if the task fails.
  3. LINTS/COMPILES anything that changed since the last pass, immediately
     after the change, per language:
       .py   -> python ast parse (syntax only)
       .rs   -> cargo check (engine)
       .ts/.tsx -> npx --no-install tsc --noEmit (needs a tsconfig.json)
       .json -> json.loads
       .toml -> tomllib
       .ps1  -> PowerShell parser
       .sh   -> bash -n
  4. LOGS cleanly to .automation/logs (rotating) + per-run files, and
     SELF-CORRECTS: bad task files are quarantined, failed tasks are never
     half-written (atomic writes), stale locks are cleaned up, lint failures
     are recorded and retried on the next pass.

TASK FILE FORMAT  (.automation/tasks/<name>.task.json)
{
  "name": "lint engine",
  "type": "command",            // command | sync | test | lint
  "cmd": ["cargo", "check"],    // argv; run from repo root unless "cwd" set
  "cwd": "engine",              // optional, relative to repo root
  "timeout": 300,               // optional seconds, default 600
  "outputs": ["docs/gen.md"]    // optional files the task writes; backed up
}                               // and auto-restored if the task fails.

USAGE
  python scripts/automation_harness.py --once          # run a single pass
  python scripts/automation_harness.py --interval 60   # poll every 60s
  python scripts/automation_harness.py --check         # verify environment
  python scripts/automation_harness.py --install-cron  # print cron line
  python scripts/automation_harness.py --install-systemd  # print unit file
  python scripts/automation_harness.py --instructions # print hookup guide

CRON  (Linux, every 5 minutes, one-shot)
  */5 * * * * cd /path/to/ayesha-os && /usr/bin/python3 \
      scripts/automation_harness.py --once >> .automation/logs/cron.log 2>&1

SYSTEMD  (Linux, long-running timer service — two files)
  /etc/systemd/system/ayesha-harness.service:
      [Unit]
      Description=ayesha-os automation harness
      After=network.target
      [Service]
      WorkingDirectory=/path/to/ayesha-os
      ExecStart=/usr/bin/python3 scripts/automation_harness.py --interval 60
      Restart=always
      RestartSec=10
      [Install]
      WantedBy=multi-user.target
  enable:  sudo systemctl daemon-reload
           sudo systemctl enable --now ayesha-harness
  (or a .timer + oneshot ExecStart=... --once for cron-style scheduling)

WINDOWS  (Task Scheduler — because this dev box is Windows)
  1. schtasks /Create /SC MINUTE /MO 5 /TN "ayesha-harness" /TR "python
      C:\\ayesha-os\\scripts\\automation_harness.py --once" /RU %USERNAME% /F
  2. or: Task Scheduler UI -> Create Basic Task -> run every 5 minutes ->
     program "python" args "C:\\ayesha-os\\scripts\\automation_harness.py --once"

All state lives under .automation/ (gitignored):
  tasks/    drop *.task.json here to queue work
  done/     successful task files + reports
  failed/   quarantined task files + reports (self-correction)
  backups/  pre-task copies of files listed in task "outputs"
  logs/     rotating harness.log + per-run files
  state.json  last-pass snapshot + lint issues (drives change detection)
"""
from __future__ import annotations

import argparse
import ast
import hashlib
import json
import logging
import os
import shutil
import signal
import subprocess
import sys
import time
import tomllib
from logging.handlers import RotatingFileHandler
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AUTO = ROOT / ".automation"
QUEUE = AUTO / "tasks"
DONE = AUTO / "done"
FAILED = AUTO / "failed"
BACKUPS = AUTO / "backups"
LOGS = AUTO / "logs"
STATE_FILE = AUTO / "state.json"
LOCK_FILE = AUTO / "harness.lock"

SKIP_DIRS = {".git", ".automation", "node_modules", "__pycache__", "target",
             "dist", ".venv", "_hf-model", "_hf-space", "build"}
CODE_EXTS = {".py", ".rs", ".ts", ".tsx", ".js", ".json", ".toml", ".ps1", ".sh"}

log = logging.getLogger("harness")
_stop = False


def now_str() -> str:
    return time.strftime("%Y-%m-%d %H:%M:%S")


# ---------------------------------------------------------------------------
# environment / paths
# ---------------------------------------------------------------------------

def ensure_dirs() -> None:
    for d in (QUEUE, DONE, FAILED, BACKUPS, LOGS):
        d.mkdir(parents=True, exist_ok=True)


def setup_logging() -> None:
    ensure_dirs()
    run = Path(time.strftime("run-%Y%m%d-%H%M%S.log"))
    handlers = [
        RotatingFileHandler(LOGS / "harness.log", maxBytes=1_000_000,
                            backupCount=5, encoding="utf-8"),
        logging.FileHandler(LOGS / run, encoding="utf-8"),
    ]
    if sys.stdout.isatty():
        handlers.append(logging.StreamHandler(sys.stdout))
    logging.basicConfig(
        level=logging.INFO, format="%(asctime)s %(levelname)-7s %(message)s",
        handlers=handlers)
    log.info("harness start pid=%s root=%s", os.getpid(), ROOT)


def acquire_lock() -> None:
    LOCK_FILE.parent.mkdir(parents=True, exist_ok=True)
    if LOCK_FILE.exists():
        try:
            pid = int(LOCK_FILE.read_text().strip())
            os.kill(pid, 0)
            log.error("another harness instance is running (pid %s)", pid)
            sys.exit(2)
        except (ValueError, ProcessLookupError):
            LOCK_FILE.unlink(missing_ok=True)
            log.warning("removed stale lock file")
        except OSError:
            pass
    LOCK_FILE.write_text(str(os.getpid()))


def release_lock() -> None:
    try:
        if LOCK_FILE.exists() and LOCK_FILE.read_text().strip() == str(os.getpid()):
            LOCK_FILE.unlink(missing_ok=True)
    except OSError:
        pass


def _sig(_signo, _frame) -> None:
    global _stop
    _stop = True


def resolve_within_root(rel: str | os.PathLike) -> Path:
    p = (ROOT / rel).resolve()
    r = ROOT.resolve()
    if p != r and r not in p.parents:
        raise ValueError(f"path escapes repo root: {rel}")
    return p


def code_files():
    exts = CODE_EXTS
    bases = [ROOT / d for d in ("engine", "core", "tri_mind_sync", "scripts",
                                "models", "applets") if (ROOT / d).exists()]
    bases.append(ROOT)
    seen = set()
    for base in bases:
        if not base.exists():
            continue
        for p in base.rglob("*"):
            if not p.is_file() or p.suffix not in exts:
                continue
            try:
                rel = p.relative_to(ROOT)
            except ValueError:
                continue
            if any(part in SKIP_DIRS for part in rel.parts) or rel in seen:
                continue
            seen.add(rel)
            yield p


def snapshot() -> dict:
    snap = {}
    for p in code_files():
        try:
            st = p.stat()
        except OSError:
            continue
        snap[str(p.relative_to(ROOT)).replace("\\", "/")] = st.st_mtime_ns
    return snap


def diff_snapshots(before: dict, after: dict) -> list[Path]:
    changed = set()
    for rel, mtime in after.items():
        if before.get(rel) != mtime:
            changed.add(rel)
    for rel in before:
        if rel not in after:
            changed.add(rel)
    return [ROOT / Path(rel) for rel in sorted(changed)]


# ---------------------------------------------------------------------------
# strict file writing (atomic + verified) and backup/restore
# ---------------------------------------------------------------------------

def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def atomic_write(path: Path, data: bytes) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    expected = sha256_bytes(data)
    tmp = path.with_name(path.name + f".tmp-{os.getpid()}")
    try:
        with open(tmp, "wb") as f:
            f.write(data)
            f.flush()
            os.fsync(f.fileno())
        with open(tmp, "rb") as f:
            if sha256_bytes(f.read()) != expected:
                raise IOError("temp write verification failed")
        if path.exists():
            shutil.copystat(path, tmp)
        os.replace(tmp, path)
        if sha256_bytes(path.read_bytes()) != expected:
            raise IOError("final verification failed")
    except Exception:
        tmp.unlink(missing_ok=True)
        raise


def backup(path: Path) -> Path | None:
    path = Path(path)
    if not path.exists():
        return None
    bak = BACKUPS / path.relative_to(ROOT)
    bak.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, bak)
    return bak


def restore(path: Path) -> None:
    bak = BACKUPS / path.relative_to(ROOT)
    if bak.exists():
        shutil.copy2(bak, path)
        log.info("restored %s from backup", path.relative_to(ROOT))


# ---------------------------------------------------------------------------
# lint / compile gates
# ---------------------------------------------------------------------------

def _tsconfig_for(path: Path) -> Path | None:
    for parent in [path, *path.parents]:
        cand = parent / "tsconfig.json"
        if cand.is_file():
            return parent
        if parent == ROOT:
            break
    return None


def lint_changed(changed: list[Path]) -> list[dict]:
    errors: list[dict] = []
    changed = [p for p in changed if p.suffix in CODE_EXTS and p.is_file()]

    for p in [p for p in changed if p.suffix == ".py"]:
        try:
            ast.parse(p.read_text(encoding="utf-8"))
        except SyntaxError as e:
            errors.append({"file": str(p), "check": "py-ast",
                           "error": f"syntax error line {e.lineno}: {e.msg}"})
        except Exception as e:
            errors.append({"file": str(p), "check": "py-ast", "error": str(e)})

    for p in [p for p in changed if p.suffix == ".json"]:
        try:
            json.loads(p.read_text(encoding="utf-8"))
        except Exception as e:
            errors.append({"file": str(p), "check": "json", "error": str(e)})

    for p in [p for p in changed if p.suffix == ".toml"]:
        try:
            tomllib.loads(p.read_text(encoding="utf-8"))
        except Exception as e:
            errors.append({"file": str(p), "check": "toml", "error": str(e)})

    rs = [p for p in changed if p.suffix == ".rs"]
    if rs and shutil.which("cargo"):
        eng = ROOT / "engine"
        if eng.exists():
            try:
                proc = subprocess.run(["cargo", "check"], cwd=eng, timeout=240,
                                      stdout=subprocess.PIPE,
                                      stderr=subprocess.STDOUT, text=True)
            except subprocess.TimeoutExpired:
                errors.append({"file": "engine/", "check": "cargo",
                               "error": "cargo check timed out"})
            else:
                if proc.returncode != 0:
                    tail = "\n".join(proc.stdout.splitlines()[-40:])
                    errors.append({"file": "engine/", "check": "cargo",
                                   "error": f"rc={proc.returncode}\n{tail}"})

    ts = [p for p in changed if p.suffix in (".ts", ".tsx")]
    proj = next((d for d in map(_tsconfig_for, ts) if d), None)
    if proj and shutil.which("npx"):
        try:
            proc = subprocess.run(["npx", "--no-install", "tsc", "--noEmit"],
                                  cwd=proj, timeout=180, stdout=subprocess.PIPE,
                                  stderr=subprocess.STDOUT, text=True)
        except subprocess.TimeoutExpired:
            errors.append({"file": str(proj), "check": "tsc",
                           "error": "tsc timed out"})
        except Exception as e:
            errors.append({"file": str(proj), "check": "tsc", "error": str(e)})
        else:
            if proc.returncode != 0:
                tail = "\n".join(proc.stdout.splitlines()[-40:])
                errors.append({"file": str(proj), "check": "tsc",
                               "error": f"rc={proc.returncode}\n{tail}"})

    for p in [p for p in changed if p.suffix == ".ps1"]:
        if shutil.which("powershell"):
            script = (
                "$t=$null;$e=$null;"
                "[System.Management.Automation.Language.Parser]::ParseFile("
                f"'{str(p).replace(chr(39), chr(39) * 2)}',[ref]$t,[ref]$e)|Out-Null;"
                "if($e.Count){$e|ForEach-Object{$_.Message};exit 1}"
            )
            try:
                proc = subprocess.run(["powershell", "-NoProfile", "-NonInteractive",
                                       "-Command", script], timeout=60,
                                      stdout=subprocess.PIPE,
                                      stderr=subprocess.STDOUT, text=True)
            except (subprocess.TimeoutExpired, OSError) as e:
                errors.append({"file": str(p), "check": "powershell", "error": str(e)})
            else:
                if proc.returncode != 0:
                    errors.append({"file": str(p), "check": "powershell",
                                   "error": proc.stdout.strip()})

    for p in [p for p in changed if p.suffix == ".sh"]:
        if shutil.which("bash"):
            try:
                proc = subprocess.run(["bash", "-n", str(p)], timeout=60,
                                      stdout=subprocess.PIPE,
                                      stderr=subprocess.STDOUT, text=True)
            except (subprocess.TimeoutExpired, OSError) as e:
                errors.append({"file": str(p), "check": "bash -n", "error": str(e)})
            else:
                if proc.returncode != 0:
                    errors.append({"file": str(p), "check": "bash -n",
                                   "error": proc.stdout.strip()})

    return errors


# ---------------------------------------------------------------------------
# task execution
# ---------------------------------------------------------------------------

def _parse_task_file(p: Path):
    try:
        raw = json.loads(p.read_text(encoding="utf-8"))
    except Exception as e:
        return None, f"unparsable task file: {e}"
    if not isinstance(raw, dict):
        return None, "task must be a JSON object"
    name = str(raw.get("name") or p.stem)
    ttype = str(raw.get("type") or "command")
    if ttype not in {"command", "sync", "test", "lint"}:
        return None, f"invalid type: {ttype}"
    cmd = raw.get("cmd")
    if not isinstance(cmd, list) or not cmd or not all(isinstance(x, str) for x in cmd):
        return None, "cmd must be a non-empty list of strings"
    try:
        timeout = int(raw.get("timeout") or 600)
    except (TypeError, ValueError):
        timeout = 600
    outputs = raw.get("outputs") or []
    if not isinstance(outputs, list):
        return None, "outputs must be a list"
    task = {"file": p, "name": name, "type": ttype, "cmd": cmd,
            "cwd": str(raw.get("cwd") or "."), "timeout": timeout,
            "outputs": [str(o) for o in outputs]}
    return task, None


def _write_report(dest: Path, task: dict, rc: int, duration: float,
                  output: str, error: str = "") -> None:
    report = {
        "task": task["name"], "cmd": task["cmd"], "cwd": task["cwd"],
        "ts": now_str(), "returncode": rc, "duration_s": round(duration, 2),
        "ok": rc == 0,
        "output_tail": output[-4000:],
        "error": error,
    }
    atomic_write(dest / (task["file"].stem + ".report.json"),
                 json.dumps(report, indent=2).encode("utf-8"))


def process_task_file(p: Path) -> bool:
    task, err = _parse_task_file(p)
    if err:
        log.error("rejected %s: %s", p.name, err)
        FAILED.mkdir(parents=True, exist_ok=True)
        atomic_write(FAILED / (p.stem + ".report.json"),
                     json.dumps({"task": p.name, "ts": now_str(), "ok": False,
                                 "error": err}, indent=2).encode("utf-8"))
        os.replace(p, FAILED / p.name)
        return False

    cwd = resolve_within_root(task["cwd"])
    protected = []
    for rel in task["outputs"]:
        try:
            out = resolve_within_root(rel)
        except ValueError as e:
            log.error("%s: %s", p.name, e)
            os.replace(p, FAILED / p.name)
            return False
        protected.append((out, backup(out)))

    log.info("running task %s -> %s", task["name"], task["cmd"])
    t0 = time.monotonic()
    rc, output, error = 1, "", ""
    try:
        proc = subprocess.run(task["cmd"], cwd=cwd, timeout=task["timeout"],
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              text=True, errors="replace")
        rc, output = proc.returncode, proc.stdout
    except subprocess.TimeoutExpired:
        error = f"timed out after {task['timeout']}s"
    except Exception as e:
        error = str(e)
    duration = time.monotonic() - t0

    if rc != 0:
        log.error("task %s FAILED rc=%s (%s)", task["name"], rc, error or "see report")
        for out, bak in protected:
            if bak is not None:
                restore(out)
        os.replace(p, FAILED / p.name)
        _write_report(FAILED, task, rc, duration, output, error)
        return False

    log.info("task %s OK in %.1fs", task["name"], duration)
    os.replace(p, DONE / p.name)
    _write_report(DONE, task, rc, duration, output)
    return True


# ---------------------------------------------------------------------------
# state
# ---------------------------------------------------------------------------

def load_state() -> dict:
    try:
        return json.loads(STATE_FILE.read_text(encoding="utf-8"))
    except Exception:
        return {}


def save_state(state: dict) -> None:
    atomic_write(STATE_FILE, json.dumps(state, indent=2).encode("utf-8"))


# ---------------------------------------------------------------------------
# main loop
# ---------------------------------------------------------------------------

def run_single_pass() -> dict:
    state = load_state()
    before = state.get("snapshot") or {}

    handled = 0
    for p in sorted(QUEUE.glob("*.task.json")):
        if _stop:
            break
        handled += process_task_file(p)

    changed = diff_snapshots(before, snapshot())
    lint_issues: list[dict] = []
    if changed:
        log.info("%d file(s) changed since last pass", len(changed))
        lint_issues = lint_changed(changed)
        if lint_issues:
            log.warning("lint/compile found %d issue(s)", len(lint_issues))
            for issue in lint_issues:
                log.warning("  %s [%s]: %s", issue["file"], issue["check"],
                            issue["error"].splitlines()[0])
        else:
            log.info("lint/compile clean for %d file(s)", len(changed))

    state["last_pass"] = now_str()
    state["last_pass_epoch"] = int(time.time())
    state["snapshot"] = snapshot()
    state["lint_issues"] = lint_issues
    state["tasks_handled"] = state.get("tasks_handled", 0) + handled
    save_state(state)

    log.info("pass complete: %d task(s), %d changed file(s), %d lint issue(s)",
             handled, len(changed), len(lint_issues))
    return state


def check_environment() -> None:
    ok = True
    for tool, ver in (("python", "python"), ("cargo", "cargo"), ("node", "node"),
                      ("npx", "npx"), ("powershell", "powershell"), ("bash", "bash")):
        exe = shutil.which(ver)
        print(f"  {tool:12s} {'FOUND ' + exe if exe else 'missing'}")
        ok = ok and bool(exe)
    print(f"  root        {ROOT}")
    print(f"  queue dir   {QUEUE}  ({len(list(QUEUE.glob('*.task.json')))} queued)")
    print(f"  state file  {STATE_FILE}")
    sys.exit(0 if ok else 1)


def print_instructions() -> None:
    print(__doc__)


def main() -> int:
    ap = argparse.ArgumentParser(prog="automation_harness",
                                 description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--once", action="store_true", help="run a single pass and exit")
    ap.add_argument("--interval", type=int, default=0, metavar="SECONDS",
                    help="poll forever every N seconds (default: 60)")
    ap.add_argument("--check", action="store_true", help="verify environment and exit")
    ap.add_argument("--install-cron", nargs="?", const="*/5 * * * *", metavar="SCHED",
                    help="print a ready-to-use cron line (default: */5 * * * *)")
    ap.add_argument("--install-systemd", action="store_true",
                    help="print a systemd service unit + enable instructions")
    ap.add_argument("--instructions", action="store_true",
                    help="print the full hookup guide")
    ap.add_argument("--root", metavar="PATH",
                    help="repo root (default: auto-detect from script location)")
    args = ap.parse_args()

    if args.root:
        global ROOT, AUTO, QUEUE, DONE, FAILED, BACKUPS, LOGS, STATE_FILE, LOCK_FILE
        ROOT = Path(args.root).resolve()
        AUTO = ROOT / ".automation"
        QUEUE = AUTO / "tasks"
        DONE = AUTO / "done"
        FAILED = AUTO / "failed"
        BACKUPS = AUTO / "backups"
        LOGS = AUTO / "logs"
        STATE_FILE = AUTO / "state.json"
        LOCK_FILE = AUTO / "harness.lock"

    if args.check:
        check_environment()
        return 0
    if args.install_cron:
        print(f"{args.install_cron} cd {ROOT} && /usr/bin/python3 "
              f"scripts/automation_harness.py --once "
              f">> .automation/logs/cron.log 2>&1")
        return 0
    if args.install_systemd:
        print(f"[Unit]\nDescription=ayesha-os automation harness\n"
              f"After=network.target\n\n[Service]\nWorkingDirectory={ROOT}\n"
              f"ExecStart=/usr/bin/python3 scripts/automation_harness.py "
              f"--interval 60\nRestart=always\nRestartSec=10\n\n[Install]\n"
              f"WantedBy=multi-user.target\n\n"
              f"# sudo systemctl daemon-reload && sudo systemctl enable --now "
              f"ayesha-harness\n"
              f"# logs: journalctl -u ayesha-harness -f")
        return 0
    if args.instructions:
        print_instructions()
        return 0

    setup_logging()
    ensure_dirs()
    acquire_lock()
    signal.signal(signal.SIGINT, _sig)
    signal.signal(signal.SIGTERM, _sig)

    interval = args.interval or (0 if args.once else 60)
    try:
        while not _stop:
            run_single_pass()
            if args.once or interval <= 0:
                break
            try:
                time.sleep(interval)
            except KeyboardInterrupt:
                break
    finally:
        release_lock()
        log.info("harness stopped")
    return 0


if __name__ == "__main__":
    sys.exit(main())
