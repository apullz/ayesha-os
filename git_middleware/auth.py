"""Public-access auth middleware — bypasses auth for /public_repos/*."""
from __future__ import annotations

import hmac
from pathlib import Path
from typing import Any, Callable

from starlette.middleware.base import BaseHTTPMiddleware
from starlette.requests import Request
from starlette.responses import JSONResponse
import json


_CONFIG_PATH = Path(__file__).parent / "config.json"


def _load_config() -> dict[str, Any]:
    if _CONFIG_PATH.exists():
        return json.loads(_CONFIG_PATH.read_text(encoding="utf-8"))
    return {}


class AuthMiddleware(BaseHTTPMiddleware):
    """Enforce token auth on all routes except those under /public_repos/*."""

    async def dispatch(self, request: Request, call_next: Callable) -> Any:
        path = request.url.path

        # Bypass auth for public repos
        if path.startswith("/public_repos/"):
            return await call_next(request)

        # Bypass auth for health / docs
        if path in ("/health", "/docs", "/openapi.json", "/"):
            return await call_next(request)

        config = _load_config()
        valid_tokens = config.get("auth_tokens", [])

        # If no tokens configured, allow all (dev mode)
        if not valid_tokens:
            return await call_next(request)

        # Check Authorization header
        auth_header = request.headers.get("authorization", "")
        if auth_header.startswith("Bearer "):
            token = auth_header[7:]
            if token in valid_tokens:
                return await call_next(request)

        return JSONResponse(
            status_code=401,
            content={"error": "unauthorized", "message": "valid bearer token required"},
        )


def verify_webhook_signature(payload: bytes, signature: str, secret: str) -> bool:
    """Verify gitea webhook HMAC-SHA256 signature."""
    if not secret:
        return True  # no secret configured = skip verification (dev mode)
    expected = "sha256=" + hmac.new(
        secret.encode(), payload, "sha256"
    ).hexdigest()
    return hmac.compare_digest(expected, signature)
