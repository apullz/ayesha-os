"""
huggingface sync module for tri-mind sync
syncs spaces and models to huggingface hub
"""

import json
import tempfile
from pathlib import Path
from typing import Dict, List, Optional

from .models import FileManifest, SyncSource, SyncStatus, SyncEvent
from .config import HuggingFaceConfig


class HuggingFaceSync:
    """sync with huggingface spaces and models"""

    def __init__(self, config: HuggingFaceConfig, base_dir: Path):
        self.config = config
        self.base_dir = base_dir
        self.state_dir = base_dir / ".tri_mind_state"
        self.manifests_file = self.state_dir / "hf_manifests.json"
        self._hf_api = None

    def _get_api(self):
        if self._hf_api is None:
            from huggingface_hub import HfApi
            self._hf_api = HfApi(token=self.config.token)
        return self._hf_api

    def sync_space(self, space_dir: Path = None) -> List[SyncEvent]:
        if not self.config.sync_spaces:
            return []
        source_dir = space_dir or self.base_dir / "core"
        if not source_dir.exists():
            return [SyncEvent(source=SyncSource.HUGGINGFACE, status=SyncStatus.CONFLICT,
                              message=f"space dir not found: {source_dir}")]
        try:
            api = self._get_api()
            api.upload_folder(folder_path=str(source_dir), repo_id=self.config.space_id,
                              repo_type="space")
            return [SyncEvent(source=SyncSource.HUGGINGFACE, target=SyncSource.HUGGINGFACE,
                              status=SyncStatus.SYNCED, message=f"synced: {self.config.space_id}")]
        except Exception as e:
            return [SyncEvent(source=SyncSource.HUGGINGFACE, status=SyncStatus.CONFLICT,
                              message=f"sync failed: {e}")]

    def pull_space(self) -> Optional[Path]:
        try:
            api = self._get_api()
            tmpdir = tempfile.mkdtemp(prefix="hf_space_")
            api.snapshot_download(repo_id=self.config.space_id, repo_type="space",
                                  local_dir=tmpdir)
            return Path(tmpdir)
        except Exception:
            return None

    def sync_model_files(self, model_dir: Path = None) -> List[SyncEvent]:
        if not self.config.sync_models:
            return []
        source_dir = model_dir or self.base_dir / "models"
        if not source_dir.exists():
            return []
        try:
            api = self._get_api()
            api.upload_folder(folder_path=str(source_dir), repo_id=self.config.model_id,
                              repo_type="model")
            return [SyncEvent(source=SyncSource.HUGGINGFACE, target=SyncSource.HUGGINGFACE,
                              status=SyncStatus.SYNCED, message=f"model synced: {self.config.model_id}")]
        except Exception as e:
            return [SyncEvent(source=SyncSource.HUGGINGFACE, status=SyncStatus.CONFLICT,
                              message=f"model sync failed: {e}")]

    def get_space_info(self) -> dict:
        try:
            api = self._get_api()
            info = api.repo_info(self.config.space_id, repo_type="space")
            return {"id": info.id,
                    "last_modified": info.lastModified.isoformat() if info.lastModified else None}
        except Exception:
            return {"error": "could not fetch space info"}

    def get_status(self) -> dict:
        manifests = self._load_manifests()
        return {
            "space_id": self.config.space_id,
            "model_id": self.config.model_id,
            "sync_spaces": self.config.sync_spaces,
            "sync_models": self.config.sync_models,
            "token_set": self.config.token is not None,
            "files_tracked": len(manifests),
        }

    def sync_full(self, local_manifests: dict = None) -> Dict[str, List]:
        result = {"space_synced": [], "model_synced": [], "errors": []}
        if self.config.sync_spaces:
            for e in self.sync_space():
                (result["space_synced"] if e.status == SyncStatus.SYNCED else result["errors"]).append(e.message)
        if self.config.sync_models:
            for e in self.sync_model_files():
                (result["model_synced"] if e.status == SyncStatus.SYNCED else result["errors"]).append(e.message)
        return result

    def _load_manifests(self) -> Dict[str, FileManifest]:
        if not self.manifests_file.exists():
            return {}
        try:
            data = json.loads(self.manifests_file.read_text(encoding="utf-8"))
            return {k: FileManifest(rel_path=v["rel_path"], size=v["size"],
                                    modified_at=v["modified_at"], sha256=v["sha256"],
                                    source=SyncSource.HUGGINGFACE) for k, v in data.items()}
        except Exception:
            return {}

    def _save_manifests(self, manifests: Dict[str, FileManifest]):
        data = {k: {"rel_path": m.rel_path, "size": m.size,
                     "modified_at": m.modified_at, "sha256": m.sha256}
                for k, m in manifests.items()}
        self.manifests_file.write_text(json.dumps(data, indent=2), encoding="utf-8")
