"""git Middleware — ayesha-os webhook + LLM task runner.

Entry point: uvicorn git Middleware.main:app --host 127.0.0.1 --port 9000
"""
from __future__ import annotations

import json
import logging
import sys
from pathlib import Path

from fastapi import FastAPI, Request, Header
from fastapi.responses import JSONResponse

# Ensure parent package is importable when running directly
_root = Path(__file__).resolve().parent.parent
if str(_root) not in sys.path:
    sys.path.insert(0, str(_root))

from git_middleware.router import LLMRouter
from git_middleware.webhook import handle_webhook
from git_middleware.auth import AuthMiddleware
from git_middleware.tasks import run_task
from git_middleware.models import TaskRequest, TaskResponse

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(levelname)s %(message)s")
logger = logging.getLogger("git_middleware.main")

app = FastAPI(title="ayesha-os git_middleware", version="0.1.0")
app.add_middleware(AuthMiddleware)

router = LLMRouter()


@app.get("/")
async def root():
    return {"service": "ayesha-os git_middleware", "version": "0.1.0", "status": "running"}


@app.get("/health")
async def health():
    return {"status": "ok", "models": router.all_task_types()}


@app.get("/config")
async def get_config():
    return {"summary": router.summary()}


@app.post("/webhook/gitea")
async def gitea_webhook(
    request: Request,
    x_gitea_event: str = Header(default="push"),
    x_hub_signature: str = Header(default=""),
):
    """Handle incoming gitea webhooks."""
    body = await request.body()
    result = await handle_webhook(x_gitea_event, body, x_hub_signature, router)
    status = 200 if "error" not in result else 400
    return JSONResponse(status_code=status, content=result)


@app.post("/task", response_model=TaskResponse)
async def run_task_endpoint(req: TaskRequest):
    """Run a single LLM task on demand."""
    result = await run_task(req.task_type, {"content": req.content, **req.context}, router)
    return TaskResponse(**result)


@app.post("/reload")
async def reload_config():
    """Hot-reload config.json from disk."""
    router.reload()
    return {"status": "reloaded", "models": router.all_task_types()}


# ──────────────────────────────────────────────────────────────
# Test suite — runs with: python main.py
# ──────────────────────────────────────────────────────────────

def _run_tests() -> bool:
    """Run built-in unit tests. Returns True if all pass."""
    import traceback
    passed = 0
    failed = 0
    errors: list[str] = []

    def assert_eq(name: str, actual: object, expected: object) -> None:
        nonlocal passed, failed
        if actual == expected:
            passed += 1
            print(f"  [PASS] {name}")
        else:
            failed += 1
            msg = f"  [FAIL] {name}: expected {expected!r}, got {actual!r}"
            print(msg)
            errors.append(msg)

    def assert_true(name: str, condition: bool) -> None:
        nonlocal passed, failed
        if condition:
            passed += 1
            print(f"  [PASS] {name}")
        else:
            failed += 1
            msg = f"  [FAIL] {name}: condition was false"
            print(msg)
            errors.append(msg)

    print("\n=== git Middleware Tests ===\n")

    # --- LLM Router Tests ---
    print("[LLMRouter]")
    cfg_path = Path(__file__).parent / "config.json"
    r = LLMRouter(cfg_path)

    assert_eq("endpoint", r.endpoint, "http://localhost:11434")
    assert_eq("model_name(quick_docs)", r.model_name("quick_docs"), "qwen2.5:7b")
    assert_eq("model_name(code_review)", r.model_name("code_review"), "deepseek-coder:33b")
    assert_eq("model_name(auto_summary)", r.model_name("auto_summary"), "qwen2.5:7b")
    assert_eq("model_name(security_scan)", r.model_name("security_scan"), "deepseek-coder:33b")
    assert_eq("model_name(unknown_fallback)", r.model_name("nonexistent"), "qwen2.5:7b")
    assert_true("all_task_types has entries", len(r.all_task_types()) > 0)
    assert_eq("tasks_for_event(push)", r.tasks_for_event("push"), ["commit_analysis", "auto_summary"])
    assert_eq("tasks_for_event(pull_request)", r.tasks_for_event("pull_request"), ["code_review", "auto_summary"])
    assert_eq("tasks_for_event(release)", r.tasks_for_event("release"), ["auto_summary"])
    assert_eq("tasks_for_event(unknown)", r.tasks_for_event("deploy"), [])
    assert_true("summary is non-empty", len(r.summary()) > 0)

    # --- Auth Middleware Tests ---
    print("\n[AuthMiddleware]")
    from git_middleware.auth import verify_webhook_signature
    import hmac as _hmac

    test_secret = "my_webhook_secret"
    test_payload = b'{"ref":"refs/heads/main"}'
    good_sig = "sha256=" + _hmac.new(test_secret.encode(), test_payload, "sha256").hexdigest()

    assert_true("valid signature verifies", verify_webhook_signature(test_payload, good_sig, test_secret))
    assert_true("bad signature rejected", not verify_webhook_signature(test_payload, "sha256=bad", test_secret))
    assert_true("empty secret skips verification", verify_webhook_signature(test_payload, "", ""))

    # --- Webhook mapping Tests ---
    print("\n[Webhook mapping]")
    from git_middleware.webhook import _map_event, _extract_context

    push_payload = {"ref": "refs/heads/main", "commits": [], "repository": {"name": "test"}}
    assert_eq("map push", _map_event("push", push_payload), "push")
    assert_eq("map pull_request", _map_event("pull_request", {"action": "opened"}), "pull_request")
    assert_eq("map pull_request_approved", _map_event("pull_request", {"action": "approved"}), "pull_request_approved")
    assert_eq("map release", _map_event("release", {"action": "published"}), "release")

    ctx = _extract_context("push", {
        "ref": "refs/heads/dev",
        "commits": [{"id": "abc12345", "message": "fix bug", "author": {"name": "fox"}, "added": ["a.py"], "modified": [], "removed": []}],
        "repository": {"full_name": "apullz/test", "html_url": "https://gitea.local/apullz/test"},
        "pusher": {"name": "fox"},
    })
    assert_eq("context branch", ctx["branch"], "dev")
    assert_eq("context repo", ctx["repo_name"], "apullz/test")
    assert_eq("context commit_count", ctx["commit_count"], 1)

    # --- Prompt fill Tests ---
    print("\n[Prompt filling]")
    from git_middleware.tasks import _fill_prompt

    filled = _fill_prompt("repo: {repo_name}", {"repo_name": "my-repo"})
    assert_eq("fill repo_name", filled, "repo: my-repo")

    filled = _fill_prompt("unknown: {foo}", {"bar": "baz"})
    assert_true("fill handles missing key", "unknown:" in filled)

    # --- Model config tests ---
    print("\n[Model config]")
    select_result = r.select("quick_docs")
    assert_true("select returns dict", isinstance(select_result, dict))
    assert_true("select has model key", "model" in select_result)

    # --- Summary ---
    print(f"\n{'='*40}")
    print(f"Results: {passed} passed, {failed} failed")
    if errors:
        print("\nFailures:")
        for e in errors:
            print(e)
    print()

    return failed == 0


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "test":
        success = _run_tests()
        sys.exit(0 if success else 1)
    else:
        # Default: run tests
        success = _run_tests()
        if success:
            print("All tests passed. Starting server...")
            import uvicorn
            cfg = json.loads((Path(__file__).parent / "config.json").read_text(encoding="utf-8"))
            server_cfg = cfg.get("server", {})
            uvicorn.run(
                "git_Middleware.main:app",
                host=server_cfg.get("host", "127.0.0.1"),
                port=server_cfg.get("port", 9000),
                reload=True,
            )
        else:
            print("Tests failed. Fix errors before starting server.")
            sys.exit(1)
