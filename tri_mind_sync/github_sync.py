"""
github sync module for tri-mind sync
bidirectional sync with GitHub repos via git + API
"""

import json
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional

import git

from .models import FileManifest, SyncSource, SyncStatus, SyncEvent
from .config import GitHubConfig


class GitHubSync:
    """bidirectional sync with GitHub"""

    def __init__(self, config: GitHubConfig, base_dir: Path):
        self.config = config
        self.base_dir = base_dir
        self.repo_dir = base_dir / ".tri_mind_state" / "github_repo"
        self.manifests_file = self.repo_dir / ".tri_mind_manifests.json"

    def _ensure_repo(self) -> git.Repo:
        if not self.repo_dir.exists():
            self.repo_dir.mkdir(parents=True, exist_ok=True)
            url = f"https://github.com/{self.config.repo}.git"
            if self.config.token:
                url = f"https://{self.config.token}@github.com/{self.config.repo}.git"
            repo = git.Repo.clone_from(url, str(self.repo_dir), branch=self.config.branch)
        else:
            repo = git.Repo(str(self.repo_dir))
        return repo

    def pull(self) -> List[SyncEvent]:
        try:
            repo = self._ensure_repo()
            repo.git.pull("origin", self.config.branch)
            return self._scan_repo_files()
        except Exception as e:
            return [SyncEvent(source=SyncSource.GITHUB, status=SyncStatus.CONFLICT,
                              message=f"pull failed: {e}")]

    def _scan_repo_files(self) -> List[SyncEvent]:
        events = []
        new_manifests = {}
        for root, dirs, files in os.walk(self.repo_dir):
            dirs[:] = [d for d in dirs if d != ".git"]
            for fname in files:
                fpath = Path(root) / fname
                try:
                    manifest = FileManifest.from_path(fpath, self.repo_dir, SyncSource.GITHUB)
                    new_manifests[manifest.rel_path] = manifest
                except Exception:
                    continue

        old_manifests = self._load_manifests()
        all_paths = set(old_manifests.keys()) | set(new_manifests.keys())
        for path in all_paths:
            old = old_manifests.get(path)
            new = new_manifests.get(path)
            if old and new and old.has_changed(new):
                events.append(SyncEvent(file_path=path, source=SyncSource.GITHUB,
                                        status=SyncStatus.MODIFIED, message=f"changed: {path}"))
            elif new and not old:
                events.append(SyncEvent(file_path=path, source=SyncSource.GITHUB,
                                        status=SyncStatus.NEW, message=f"new: {path}"))
            elif old and not new:
                events.append(SyncEvent(file_path=path, source=SyncSource.GITHUB,
                                        status=SyncStatus.DELETED, message=f"deleted: {path}"))
        self._save_manifests(new_manifests)
        return events

    def push(self, files: List[str] = None, message: str = "tri-mind sync") -> bool:
        try:
            repo = self._ensure_repo()
            if files:
                for rel in files:
                    src = self.base_dir / rel
                    dst = self.repo_dir / rel
                    if src.exists():
                        dst.parent.mkdir(parents=True, exist_ok=True)
                        dst.write_bytes(src.read_bytes())
                    elif dst.exists():
                        dst.unlink()
            repo.git.add(A=True)
            if repo.is_dirty():
                repo.index.commit(message)
                repo.git.push("origin", self.config.branch)
            return True
        except Exception:
            return False

    def push_file(self, rel_path: str, message: str = None) -> bool:
        src = self.base_dir / rel_path
        if not src.exists():
            return False
        repo = self._ensure_repo()
        dst = self.repo_dir / rel_path
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_bytes(src.read_bytes())
        try:
            repo.git.add(str(dst.relative_to(self.repo_dir)))
            repo.index.commit(message or f"sync: {rel_path}")
            repo.git.push("origin", self.config.branch)
            return True
        except Exception:
            return False

    def pull_file(self, rel_path: str) -> Optional[Path]:
        repo = self._ensure_repo()
        repo.git.pull("origin", self.config.branch)
        src = self.repo_dir / rel_path
        if src.exists():
            dst = self.base_dir / rel_path
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.write_bytes(src.read_bytes())
            return dst
        return None

    def get_repo_status(self) -> dict:
        try:
            repo = git.Repo(str(self.repo_dir))
            return {
                "branch": str(repo.active_branch),
                "dirty": repo.is_dirty(),
                "commit": str(repo.head.commit.hexsha[:8]),
                "message": repo.head.commit.message.strip(),
            }
        except Exception:
            return {"error": "repo not initialized"}

    def sync_bidirectional(self, local_manifests: dict) -> Dict[str, List]:
        result = {"pulled": [], "pushed": [], "conflicts": []}
        pull_events = self.pull()
        result["pulled"] = [e.file_path for e in pull_events if e.status != SyncStatus.CONFLICT]

        repo_manifests = self._load_manifests()
        all_paths = set(local_manifests.keys()) | set(repo_manifests.keys())
        to_push = []
        for path in all_paths:
            local = local_manifests.get(path)
            remote = repo_manifests.get(path)
            if local and remote:
                if local.has_changed(remote):
                    result["conflicts"].append(path)
            elif local and not remote:
                to_push.append(path)
            elif remote and not local:
                result["pulled"].append(path)

        if to_push:
            self.push(to_push, "tri-mind: sync new local files")
            result["pushed"] = to_push
        return result

    def _load_manifests(self) -> Dict[str, FileManifest]:
        if not self.manifests_file.exists():
            return {}
        try:
            data = json.loads(self.manifests_file.read_text(encoding="utf-8"))
            return {
                k: FileManifest(rel_path=v["rel_path"], size=v["size"],
                                modified_at=v["modified_at"], sha256=v["sha256"],
                                source=SyncSource.GITHUB)
                for k, v in data.items()
            }
        except Exception:
            return {}

    def _save_manifests(self, manifests: Dict[str, FileManifest]):
        data = {k: {"rel_path": m.rel_path, "size": m.size,
                     "modified_at": m.modified_at, "sha256": m.sha256}
                for k, m in manifests.items()}
        self.manifests_file.write_text(json.dumps(data, indent=2), encoding="utf-8")
