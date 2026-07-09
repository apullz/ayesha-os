"""
data models for tri-mind sync
"""

import hashlib
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Optional


class SyncSource(str, Enum):
    LOCAL = "local"
    GITHUB = "github"
    HUGGINGFACE = "huggingface"


class SyncStatus(str, Enum):
    SYNCED = "synced"
    MODIFIED = "modified"
    CONFLICT = "conflict"
    DELETED = "deleted"
    NEW = "new"


class ConflictResolution(str, Enum):
    LOCAL_WINS = "local_wins"
    REMOTE_WINS = "remote_wins"
    MANUAL = "manual"
    SKIP = "skip"


@dataclass
class FileManifest:
    """tracks a single file across all three sources"""
    rel_path: str
    size: int = 0
    modified_at: str = ""
    sha256: str = ""
    source: SyncSource = SyncSource.LOCAL
    status: SyncStatus = SyncStatus.SYNCED

    @classmethod
    def from_path(cls, path, base_dir=None, source: SyncSource = SyncSource.LOCAL) -> "FileManifest":
        import os
        stat = os.stat(path)
        rel = str(path.relative_to(base_dir)) if base_dir else path.name
        content = path.read_bytes()
        return cls(
            rel_path=rel,
            size=stat.st_size,
            modified_at=datetime.fromtimestamp(stat.st_mtime).isoformat(),
            sha256=hashlib.sha256(content).hexdigest(),
            source=source,
        )

    def has_changed(self, other: "FileManifest") -> bool:
        return self.sha256 != other.sha256


@dataclass
class SyncEvent:
    """represents a sync action"""
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())
    file_path: str = ""
    source: SyncSource = SyncSource.LOCAL
    target: SyncSource = SyncSource.GITHUB
    status: SyncStatus = SyncStatus.SYNCED
    message: str = ""


@dataclass
class Conflict:
    """represents a sync conflict"""
    file_path: str
    local_manifest: Optional[FileManifest] = None
    remote_manifest: Optional[FileManifest] = None
    remote_source: SyncSource = SyncSource.GITHUB
    detected_at: str = field(default_factory=lambda: datetime.now().isoformat())
    resolved: bool = False
    resolution: Optional[ConflictResolution] = None


@dataclass
class NodeStatus:
    """status of a sync node"""
    source: SyncSource
    connected: bool = False
    last_sync: str = ""
    files_tracked: int = 0
    files_synced: int = 0
    errors: int = 0
    message: str = ""


@dataclass
class SyncState:
    """global sync state"""
    version: str = "1.0.0"
    last_full_sync: str = ""
    manifests: dict = field(default_factory=dict)
    events: list = field(default_factory=list)
    conflicts: list = field(default_factory=list)
    nodes: dict = field(default_factory=dict)

    def add_event(self, event: SyncEvent):
        self.events.append(event)
        if len(self.events) > 1000:
            self.events = self.events[-500:]

    def add_conflict(self, conflict: Conflict):
        self.conflicts.append(conflict)
