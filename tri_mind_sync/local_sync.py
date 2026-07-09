"""
local filesystem sync module
watches local files and maintains manifest state
"""

import os
import fnmatch
import json
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional

from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

from .models import FileManifest, SyncSource, SyncStatus, SyncEvent
from .config import LocalConfig


class LocalSyncHandler(FileSystemEventHandler):
    def __init__(self, sync_callback):
        self.sync_callback = sync_callback

    def on_modified(self, event):
        if not event.is_directory:
            self.sync_callback(Path(event.src_path), "modified")

    def on_created(self, event):
        if not event.is_directory:
            self.sync_callback(Path(event.src_path), "created")

    def on_deleted(self, event):
        if not event.is_directory:
            self.sync_callback(Path(event.src_path), "deleted")


class LocalSync:
    """manages local filesystem sync state"""

    def __init__(self, config: LocalConfig, base_dir: Path):
        self.config = config
        self.base_dir = base_dir
        self.state_dir = config.get_state_path(base_dir)
        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.manifests_file = self.state_dir / "local_manifests.json"
        self.observer: Optional[Observer] = None

    def scan_directory(self) -> Dict[str, FileManifest]:
        manifests = {}
        for root, dirs, files in os.walk(self.base_dir):
            dirs[:] = [
                d for d in dirs
                if not any(fnmatch.fnmatch(d, pat) for pat in self.config.exclude_patterns)
            ]
            for fname in files:
                if any(fnmatch.fnmatch(fname, pat) for pat in self.config.exclude_patterns):
                    continue
                fpath = Path(root) / fname
                try:
                    manifest = FileManifest.from_path(fpath, self.base_dir, SyncSource.LOCAL)
                    manifests[manifest.rel_path] = manifest
                except Exception:
                    continue
        return manifests

    def load_manifests(self) -> Dict[str, FileManifest]:
        if not self.manifests_file.exists():
            return {}
        try:
            data = json.loads(self.manifests_file.read_text(encoding="utf-8"))
            return {
                k: FileManifest(
                    rel_path=v["rel_path"], size=v["size"],
                    modified_at=v["modified_at"], sha256=v["sha256"],
                    source=SyncSource(v["source"]),
                    status=SyncStatus(v["status"]),
                )
                for k, v in data.items()
            }
        except Exception:
            return {}

    def save_manifests(self, manifests: Dict[str, FileManifest]):
        data = {
            k: {
                "rel_path": m.rel_path, "size": m.size,
                "modified_at": m.modified_at, "sha256": m.sha256,
                "source": m.source.value, "status": m.status.value,
            }
            for k, m in manifests.items()
        }
        self.manifests_file.write_text(json.dumps(data, indent=2), encoding="utf-8")

    def compute_diff(self, new_manifests: Dict[str, FileManifest]) -> List[SyncEvent]:
        old_manifests = self.load_manifests()
        events = []
        all_paths = set(old_manifests.keys()) | set(new_manifests.keys())

        for path in all_paths:
            old = old_manifests.get(path)
            new = new_manifests.get(path)
            if old and new and old.has_changed(new):
                events.append(SyncEvent(file_path=path, source=SyncSource.LOCAL,
                                        status=SyncStatus.MODIFIED, message=f"modified: {path}"))
            elif new and not old:
                events.append(SyncEvent(file_path=path, source=SyncSource.LOCAL,
                                        status=SyncStatus.NEW, message=f"new: {path}"))
            elif old and not new:
                events.append(SyncEvent(file_path=path, source=SyncSource.LOCAL,
                                        status=SyncStatus.DELETED, message=f"deleted: {path}"))
        return events

    def sync(self) -> List[SyncEvent]:
        new_manifests = self.scan_directory()
        events = self.compute_diff(new_manifests)
        self.save_manifests(new_manifests)
        return events

    def start_watching(self, callback=None):
        if self.observer:
            return
        handler = LocalSyncHandler(lambda p, c: callback(p, c) if callback else None)
        self.observer = Observer()
        self.observer.schedule(handler, str(self.base_dir), recursive=True)
        self.observer.daemon = True
        self.observer.start()

    def stop_watching(self):
        if self.observer:
            self.observer.stop()
            self.observer.join()
            self.observer = None

    def get_changed_files(self) -> List[str]:
        current = self.scan_directory()
        saved = self.load_manifests()
        return [p for p, m in current.items() if p not in saved or m.has_changed(saved[p])]

    def get_status(self) -> dict:
        manifests = self.load_manifests()
        changed = self.get_changed_files()
        return {
            "total_files": len(manifests),
            "changed_files": len(changed),
            "state_dir": str(self.state_dir),
            "watching": self.observer is not None,
        }
