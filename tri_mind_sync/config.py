"""
tri-mind sync configuration
defines what syncs where, credentials, and sync behavior
"""

import json
import os
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Optional


@dataclass
class GitHubConfig:
    """github sync settings"""
    repo: str = "apullz/ayesha-os"
    branch: str = "main"
    token_env: str = "GITHUB_TOKEN"
    auto_push: bool = True
    auto_pull: bool = True
    sync_interval: int = 300
    watch_patterns: list = field(default_factory=lambda: ["*.py", "*.json", "*.md"])

    @property
    def token(self) -> Optional[str]:
        return os.environ.get(self.token_env)


@dataclass
class HuggingFaceConfig:
    """huggingface sync settings"""
    space_id: str = "apullz/ayesha-spaces"
    model_id: str = "apullz/ayesha"
    token_env: str = "HF_TOKEN"
    auto_sync: bool = True
    sync_interval: int = 60
    sync_models: bool = False
    sync_spaces: bool = True

    @property
    def token(self) -> Optional[str]:
        return os.environ.get(self.token_env)


@dataclass
class LocalConfig:
    """local filesystem sync settings"""
    watch_dir: str = ""
    state_dir: str = ".tri_mind_state"
    auto_watch: bool = True
    sync_interval: int = 30
    exclude_patterns: list = field(default_factory=lambda: [
        "__pycache__", "*.pyc", ".git", "node_modules",
        "*.egg-info", ".env", ".env.*", "target"
    ])

    def get_state_path(self, base: Path) -> Path:
        return base / self.state_dir


@dataclass
class ConflictConfig:
    """conflict resolution settings"""
    strategy: str = "last_write_wins"
    auto_resolve: bool = True
    log_conflicts: bool = True
    conflict_dir: str = ".tri_mind_conflicts"


@dataclass
class TriMindConfig:
    """main tri-mind sync configuration"""
    github: GitHubConfig = field(default_factory=GitHubConfig)
    huggingface: HuggingFaceConfig = field(default_factory=HuggingFaceConfig)
    local: LocalConfig = field(default_factory=LocalConfig)
    conflict: ConflictConfig = field(default_factory=ConflictConfig)

    sync_enabled: bool = True
    log_level: str = "INFO"
    config_path: str = "tri_mind_sync.json"

    @classmethod
    def load(cls, path: Optional[str] = None) -> "TriMindConfig":
        config_path = Path(path or cls().config_path)
        if config_path.exists():
            try:
                data = json.loads(config_path.read_text(encoding="utf-8"))
                return cls._from_dict(data)
            except Exception:
                pass
        return cls()

    @classmethod
    def _from_dict(cls, data: dict) -> "TriMindConfig":
        config = cls()
        if "github" in data:
            for k, v in data["github"].items():
                if hasattr(config.github, k):
                    setattr(config.github, k, v)
        if "huggingface" in data:
            for k, v in data["huggingface"].items():
                if hasattr(config.huggingface, k):
                    setattr(config.huggingface, k, v)
        if "local" in data:
            for k, v in data["local"].items():
                if hasattr(config.local, k):
                    setattr(config.local, k, v)
        if "conflict" in data:
            for k, v in data["conflict"].items():
                if hasattr(config.conflict, k):
                    setattr(config.conflict, k, v)
        for k in ["sync_enabled", "log_level", "config_path"]:
            if k in data:
                setattr(config, k, data[k])
        return config

    def save(self, path: Optional[str] = None):
        config_path = Path(path or self.config_path)
        config_path.write_text(json.dumps(asdict(self), indent=2), encoding="utf-8")

    def get_base_dir(self) -> Path:
        return Path(__file__).parent.parent
