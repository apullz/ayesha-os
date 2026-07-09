"""
conflict detection and resolution for tri-mind sync
"""

import json
import shutil
from datetime import datetime
from pathlib import Path
from typing import Optional, List

from .models import (
    FileManifest, Conflict, ConflictResolution,
    SyncSource, SyncStatus
)
from .config import ConflictConfig


class ConflictResolver:
    """detects and resolves sync conflicts between sources"""

    def __init__(self, config: ConflictConfig, base_dir: Path):
        self.config = config
        self.base_dir = base_dir
        self.conflict_dir = base_dir / config.conflict_dir
        self.conflict_dir.mkdir(parents=True, exist_ok=True)

    def detect_conflicts(
        self,
        local_manifests: dict,
        remote_manifests: dict,
        remote_source: SyncSource
    ) -> List[Conflict]:
        conflicts = []
        all_paths = set(local_manifests.keys()) | set(remote_manifests.keys())

        for path in all_paths:
            local = local_manifests.get(path)
            remote = remote_manifests.get(path)

            if local and remote:
                if local.has_changed(remote):
                    if local.modified_at != remote.modified_at:
                        conflicts.append(Conflict(
                            file_path=path,
                            local_manifest=local,
                            remote_manifest=remote,
                            remote_source=remote_source,
                        ))
        return conflicts

    def resolve(
        self,
        conflict: Conflict,
        resolution: Optional[ConflictResolution] = None
    ) -> ConflictResolution:
        if resolution is None:
            resolution = self._auto_resolve(conflict)

        conflict.resolved = True
        conflict.resolution = resolution

        if resolution == ConflictResolution.LOCAL_WINS:
            self._backup_remote(conflict)
        elif resolution == ConflictResolution.REMOTE_WINS:
            self._restore_remote(conflict)
        elif resolution == ConflictResolution.MANUAL:
            self._move_to_conflict_dir(conflict)

        return resolution

    def _auto_resolve(self, conflict: Conflict) -> ConflictResolution:
        if self.config.strategy == "last_write_wins":
            if conflict.local_manifest and conflict.remote_manifest:
                local_time = conflict.local_manifest.modified_at
                remote_time = conflict.remote_manifest.modified_at
                if local_time > remote_time:
                    return ConflictResolution.REMOTE_WINS
                else:
                    return ConflictResolution.LOCAL_WINS
            return ConflictResolution.LOCAL_WINS
        return ConflictResolution.MANUAL

    def _backup_remote(self, conflict: Conflict):
        if conflict.local_manifest:
            backup_path = self.conflict_dir / f"{conflict.file_path}.remote_backup"
            backup_path.parent.mkdir(parents=True, exist_ok=True)
            source = self.base_dir / conflict.file_path
            if source.exists():
                shutil.copy2(source, backup_path)

    def _restore_remote(self, conflict: Conflict):
        target = self.base_dir / conflict.file_path
        target.parent.mkdir(parents=True, exist_ok=True)
        # fetch from remote would happen here in a full impl

    def _move_to_conflict_dir(self, conflict: Conflict):
        source = self.base_dir / conflict.file_path
        if source.exists():
            dest = self.conflict_dir / conflict.file_path
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, dest)

    def get_unresolved_conflicts(self) -> List[Conflict]:
        conflict_file = self.conflict_dir / "conflicts.json"
        if not conflict_file.exists():
            return []
        try:
            data = json.loads(conflict_file.read_text(encoding="utf-8"))
            return [
                Conflict(
                    file_path=c["file_path"],
                    remote_source=SyncSource(c["remote_source"]),
                    detected_at=c["detected_at"],
                    resolved=c.get("resolved", False),
                )
                for c in data
                if not c.get("resolved", False)
            ]
        except Exception:
            return []

    def save_conflicts(self, conflicts: List[Conflict]):
        conflict_file = self.conflict_dir / "conflicts.json"
        data = [
            {
                "file_path": c.file_path,
                "remote_source": c.remote_source.value,
                "detected_at": c.detected_at,
                "resolved": c.resolved,
                "resolution": c.resolution.value if c.resolution else None,
            }
            for c in conflicts
        ]
        conflict_file.write_text(json.dumps(data, indent=2), encoding="utf-8")
