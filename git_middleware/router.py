"""LLM Router — selects the optimal model for a given task type."""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

_CONFIG_PATH = Path(__file__).parent / "config.json"


class LLMRouter:
    """Routes task types to ollama models based on config.json mappings."""

    def __init__(self, config_path: Path | None = None) -> None:
        self._config_path = config_path or _CONFIG_PATH
        self._config: dict[str, Any] = {}
        self._load()

    def _load(self) -> None:
        if self._config_path.exists():
            self._config = json.loads(self._config_path.read_text(encoding="utf-8"))
        else:
            self._config = {
                "ollama_endpoint": "http://localhost:11434",
                "models": {},
                "task_triggers": {},
            }

    def reload(self) -> None:
        """Hot-reload config from disk."""
        self._load()

    @property
    def endpoint(self) -> str:
        return self._config.get("ollama_endpoint", "http://localhost:11434")

    @property
    def models(self) -> dict[str, dict[str, Any]]:
        return self._config.get("models", {})

    @property
    def task_triggers(self) -> dict[str, list[str]]:
        return self._config.get("task_triggers", {})

    def select(self, task_type: str) -> dict[str, Any]:
        """Return model config for a task type, falling back to quick_docs."""
        return self.models.get(task_type, self.models.get("quick_docs", {
            "model": "qwen2.5:7b",
            "max_tokens": 2048,
            "temperature": 0.3,
        }))

    def model_name(self, task_type: str) -> str:
        return self.select(task_type).get("model", "qwen2.5:7b")

    def tasks_for_event(self, event_type: str) -> list[str]:
        """Return task types to run for a given gitea event type."""
        return self.task_triggers.get(event_type, [])

    def all_task_types(self) -> list[str]:
        return list(self.models.keys())

    def summary(self) -> str:
        lines = [f"endpoint: {self.endpoint}"]
        for name, cfg in self.models.items():
            lines.append(f"  {name:20s} -> {cfg['model']}")
        lines.append("triggers:")
        for evt, tasks in self.task_triggers.items():
            lines.append(f"  {evt:25s} -> {', '.join(tasks)}")
        return "\n".join(lines)
