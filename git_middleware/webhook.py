"""Gitea webhook handler — parses push/PR/release events."""
from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any

from .auth import verify_webhook_signature
from .models import PushPayload, PullRequestPayload, ReleasePayload
from .router import LLMRouter
from .tasks import run_task

logger = logging.getLogger("git Middleware.webhook")

_CONFIG_PATH = Path(__file__).parent / "config.json"


def _load_secret() -> str:
    if _CONFIG_PATH.exists():
        cfg = json.loads(_CONFIG_PATH.read_text(encoding="utf-8"))
        return cfg.get("gitea_webhook_secret", "")
    return ""


async def handle_webhook(
    event_type: str,
    payload_bytes: bytes,
    signature: str,
    router: LLMRouter,
) -> dict[str, Any]:
    """Process an incoming gitea webhook.

    Returns a summary dict with results of triggered tasks.
    """
    secret = _load_secret()
    if not verify_webhook_signature(payload_bytes, signature, secret):
        return {"error": "invalid signature", "tasks": []}

    payload = json.loads(payload_bytes)
    results: list[dict[str, Any]] = []

    # Map gitea event types to our task trigger keys
    trigger_key = _map_event(event_type, payload)
    task_types = router.tasks_for_event(trigger_key)

    if not task_types:
        return {"event": event_type, "trigger": trigger_key, "tasks": [], "message": "no tasks configured"}

    # Build context from payload
    context = _extract_context(event_type, payload)

    for task_type in task_types:
        logger.info("triggering task=%s for event=%s", task_type, event_type)
        result = await run_task(task_type, context, router)
        results.append(result)

    return {
        "event": event_type,
        "trigger": trigger_key,
        "tasks": results,
    }


def _map_event(event_type: str, payload: dict[str, Any]) -> str:
    """Map gitea X-Gitea-Event to our internal trigger key."""
    if event_type == "push":
        return "push"
    if event_type == "pull_request":
        action = payload.get("action", "")
        if action == "approved":
            return "pull_request_approved"
        return "pull_request"
    if event_type == "release":
        return "release"
    return event_type


def _extract_context(event_type: str, payload: dict[str, Any]) -> dict[str, Any]:
    """Extract relevant context from the webhook payload for LLM tasks."""
    ctx: dict[str, Any] = {"event_type": event_type}

    repo = payload.get("repository", {})
    ctx["repo_name"] = repo.get("full_name", repo.get("name", "unknown"))
    ctx["repo_url"] = repo.get("html_url", "")

    if event_type == "push":
        ref = payload.get("ref", "")
        ctx["branch"] = ref.replace("refs/heads/", "")
        commits = payload.get("commits", [])
        ctx["commit_count"] = len(commits)
        ctx["commits"] = [
            {"id": c.get("id", "")[:8], "message": c.get("message", ""), "author": c.get("author", {}).get("name", "")}
            for c in commits[:10]
        ]
        ctx["diff_summary"] = _summarize_diff(commits)
        ctx["pusher"] = payload.get("pusher", {}).get("name", "")

    elif event_type == "pull_request":
        pr = payload.get("pull_request", {})
        ctx["pr_number"] = pr.get("number", 0)
        ctx["pr_title"] = pr.get("title", "")
        ctx["pr_body"] = pr.get("body", "")[:2000]
        ctx["pr_author"] = pr.get("user", {}).get("login", "")
        ctx["pr_branch"] = pr.get("head", {}).get("ref", "")
        ctx["pr_base"] = pr.get("base", {}).get("ref", "")
        ctx["action"] = payload.get("action", "")

    elif event_type == "release":
        release = payload.get("release", {})
        ctx["release_name"] = release.get("name", release.get("tag_name", ""))
        ctx["release_body"] = release.get("body", "")[:2000]
        ctx["action"] = payload.get("action", "")

    return ctx


def _summarize_diff(commits: list[dict[str, Any]]) -> str:
    """Build a compact diff summary from commit data."""
    lines: list[str] = []
    for c in commits:
        added = c.get("added", [])
        modified = c.get("modified", [])
        removed = c.get("removed", [])
        parts: list[str] = []
        if added:
            parts.append(f"+{len(added)} files")
        if modified:
            parts.append(f"~{len(modified)} files")
        if removed:
            parts.append(f"-{len(removed)} files")
        summary = ", ".join(parts) if parts else "no diff info"
        lines.append(f"  {c.get('id', '?')[:8]} {c.get('message', '')[:60]} ({summary})")
    return "\n".join(lines)
