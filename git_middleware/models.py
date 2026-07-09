"""Pydantic models for webhook payloads and API requests."""
from __future__ import annotations

from pydantic import BaseModel, Field
from typing import Any


class PushCommit(BaseModel):
    id: str = ""
    message: str = ""
    author: dict[str, Any] = Field(default_factory=dict)
    added: list[str] = Field(default_factory=list)
    modified: list[str] = Field(default_factory=list)
    removed: list[str] = Field(default_factory=list)
    url: str = ""


class PushPayload(BaseModel):
    ref: str = ""
    before: str = ""
    after: str = ""
    repository: dict[str, Any] = Field(default_factory=dict)
    commits: list[PushCommit] = Field(default_factory=list)
    pusher: dict[str, Any] = Field(default_factory=dict)
    sender: dict[str, Any] = Field(default_factory=dict)


class PullRequestPayload(BaseModel):
    action: str = ""
    number: int = 0
    pull_request: dict[str, Any] = Field(default_factory=dict)
    repository: dict[str, Any] = Field(default_factory=dict)
    sender: dict[str, Any] = Field(default_factory=dict)


class ReleasePayload(BaseModel):
    action: str = ""
    release: dict[str, Any] = Field(default_factory=dict)
    repository: dict[str, Any] = Field(default_factory=dict)
    sender: dict[str, Any] = Field(default_factory=dict)


class TaskRequest(BaseModel):
    task_type: str
    content: str
    context: dict[str, Any] = Field(default_factory=dict)


class TaskResponse(BaseModel):
    task_type: str
    model: str
    result: str
    success: bool
    error: str | None = None
