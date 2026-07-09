"""Ollama task triggers — runs AI tasks via the local ollama instance."""
from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any

import httpx

from .router import LLMRouter

logger = logging.getLogger("git Middleware.tasks")

# Prompt templates for each task type
PROMPTS: dict[str, str] = {
    "commit_analysis": (
        "You are a commit analyst. Analyze these commits and provide:\n"
        "1. Whether they follow conventional commit format\n"
        "2. A brief risk assessment\n"
        "3. Suggested improvements\n\n"
        "Repository: {repo_name}\nBranch: {branch}\nCommits:\n{commits}\n\n"
        "Respond in lowercase, be concise."
    ),
    "code_review": (
        "You are a senior code reviewer. Review this pull request:\n\n"
        "PR #{pr_number}: {pr_title}\n"
        "Author: {pr_author}\n"
        "Branch: {pr_branch} -> {pr_base}\n"
        "Description:\n{pr_body}\n\n"
        "Provide:\n"
        "1. Code quality assessment\n"
        "2. Potential bugs or issues\n"
        "3. Architecture suggestions\n"
        "4. Security concerns\n\n"
        "Respond in lowercase, be thorough but concise."
    ),
    "auto_summary": (
        "You are a technical summarizer. Generate a summary for this event:\n\n"
        "Repository: {repo_name}\nEvent: {event_type}\n"
        "{extra_context}\n\n"
        "Provide a 2-3 sentence summary suitable for a changelog or notification.\n"
        "Respond in lowercase."
    ),
    "security_scan": (
        "You are a security auditor. Analyze this pull request for security issues:\n\n"
        "PR #{pr_number}: {pr_title}\n"
        "Description:\n{pr_body}\n\n"
        "Check for:\n"
        "1. Hardcoded secrets or credentials\n"
        "2. SQL injection / XSS vulnerabilities\n"
        "3. Unsafe deserialization\n"
        "4. Dependency risks\n"
        "5. Path traversal\n\n"
        "Respond in lowercase, flag any findings."
    ),
}


async def run_task(
    task_type: str,
    context: dict[str, Any],
    router: LLMRouter,
) -> dict[str, Any]:
    """Execute a single LLM task and return the result."""
    model_cfg = router.select(task_type)
    model_name = model_cfg.get("model", "qwen2.5:7b")
    endpoint = router.endpoint

    prompt_template = PROMPTS.get(task_type, "Analyze the following:\n{extra_context}")
    prompt = _fill_prompt(prompt_template, context)

    logger.info("running task=%s model=%s", task_type, model_name)

    try:
        result = await _call_ollama(endpoint, model_name, prompt, model_cfg)
        return {
            "task_type": task_type,
            "model": model_name,
            "result": result,
            "success": True,
            "error": None,
        }
    except Exception as exc:
        logger.error("task=%s failed: %s", task_type, exc)
        return {
            "task_type": task_type,
            "model": model_name,
            "result": "",
            "success": False,
            "error": str(exc),
        }


def _fill_prompt(template: str, ctx: dict[str, Any]) -> str:
    """Fill a prompt template with context, ignoring missing keys."""
    # Build extra_context from anything not already in the template
    known = {"repo_name", "branch", "commits", "commits_text", "diff_summary",
             "pr_number", "pr_title", "pr_body", "pr_author", "pr_branch", "pr_base",
             "release_name", "release_body", "event_type", "action", "extra_context"}
    extra_parts: list[str] = []
    for k, v in ctx.items():
        if k not in known and k != "event_type":
            extra_parts.append(f"{k}: {v}")

    if "commits" in ctx and "commits" not in template:
        commits_text = "\n".join(
            f"  {c['id']} {c['message']} by {c['author']}"
            for c in ctx["commits"]
        )
        extra_parts.append(f"Commits:\n{commits_text}")

    fill = dict(ctx)
    fill["extra_context"] = "\n".join(extra_parts) if extra_parts else f"Event: {ctx.get('event_type', 'unknown')}"

    try:
        return template.format(**fill)
    except KeyError:
        # If template references missing keys, do best-effort replacement
        result = template
        for k, v in fill.items():
            result = result.replace("{" + k + "}", str(v))
        return result


async def _call_ollama(
    endpoint: str,
    model: str,
    prompt: str,
    cfg: dict[str, Any],
) -> str:
    """Call ollama API and return the response text."""
    url = f"{endpoint}/api/chat"
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": False,
        "options": {
            "num_predict": cfg.get("max_tokens", 2048),
            "temperature": cfg.get("temperature", 0.3),
        },
    }

    async with httpx.AsyncClient(timeout=120.0) as client:
        resp = await client.post(url, json=payload)
        resp.raise_for_status()
        data = resp.json()
        return data.get("message", {}).get("content", "")
