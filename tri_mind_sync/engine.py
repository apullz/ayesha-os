"""
tri-mind sync engine
orchestrates local, github, and huggingface sync
"""

import json
import time
import threading
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict, List

from .config import TriMindConfig
from .models import SyncState, SyncEvent, SyncSource, SyncStatus, NodeStatus
from .local_sync import LocalSync
from .github_sync import GitHubSync
from .huggingface_sync import HuggingFaceSync
from .conflict import ConflictResolver


class TriMindEngine:
    """main engine that coordinates all three sync nodes"""

    def __init__(self, config: TriMindConfig = None, base_dir: str = None):
        self.config = config or TriMindConfig.load()
        self.base_dir = Path(base_dir) if base_dir else self.config.get_base_dir()
        self.state_file = self.base_dir / ".tri_mind_state" / "engine_state.json"

        self.local = LocalSync(self.config.local, self.base_dir)
        self.github = GitHubSync(self.config.github, self.base_dir)
        self.huggingface = HuggingFaceSync(self.config.huggingface, self.base_dir)
        self.resolver = ConflictResolver(self.config.conflict, self.base_dir)

        self.state = self._load_state()
        self._running = False
        self._thread: Optional[threading.Thread] = None

    def _load_state(self) -> SyncState:
        if self.state_file.exists():
            try:
                data = json.loads(self.state_file.read_text(encoding="utf-8"))
                state = SyncState(version=data.get("version", "1.0.0"),
                                  last_full_sync=data.get("last_full_sync", ""))
                return state
            except Exception:
                pass
        return SyncState()

    def _save_state(self):
        self.state_file.parent.mkdir(parents=True, exist_ok=True)
        data = {"version": self.state.version, "last_full_sync": self.state.last_full_sync,
                "nodes": {k: {"connected": v.connected, "last_sync": v.last_sync}
                          for k, v in self.state.nodes.items()}}
        self.state_file.write_text(json.dumps(data, indent=2), encoding="utf-8")

    def sync_local(self) -> List[SyncEvent]:
        events = self.local.sync()
        self.state.nodes["local"] = NodeStatus(
            source=SyncSource.LOCAL, connected=True,
            last_sync=datetime.now().isoformat(),
            files_tracked=len(self.local.load_manifests()))
        return events

    def sync_github(self) -> Dict[str, List]:
        local_manifests = self.local.load_manifests()
        result = self.github.sync_bidirectional(local_manifests)
        for path in result["pulled"]:
            src = self.github.repo_dir / path
            dst = self.base_dir / path
            if src.exists():
                dst.parent.mkdir(parents=True, exist_ok=True)
                dst.write_bytes(src.read_bytes())
        if result["conflicts"]:
            github_manifests = self.github._load_manifests()
            for path in result["conflicts"]:
                conflict = self.resolver.detect_conflicts(
                    {path: local_manifests[path]} if path in local_manifests else {},
                    {path: github_manifests[path]} if path in github_manifests else {},
                    SyncSource.GITHUB)
                for c in conflict:
                    self.resolver.resolve(c)
                    self.state.add_conflict(c)
        self.state.nodes["github"] = NodeStatus(
            source=SyncSource.GITHUB, connected=True,
            last_sync=datetime.now().isoformat(),
            files_tracked=len(self.github._load_manifests()))
        return result

    def sync_huggingface(self) -> Dict[str, List]:
        result = self.huggingface.sync_full()
        self.state.nodes["huggingface"] = NodeStatus(
            source=SyncSource.HUGGINGFACE, connected=True,
            last_sync=datetime.now().isoformat())
        return result

    def sync_all(self) -> Dict[str, any]:
        results = {"timestamp": datetime.now().isoformat(), "local": [], "github": {}, "huggingface": {}}
        local_events = self.sync_local()
        results["local"] = [e.message for e in local_events]
        results["github"] = self.sync_github()
        results["huggingface"] = self.sync_huggingface()
        self.state.last_full_sync = datetime.now().isoformat()
        self._save_state()
        return results

    def start_loop(self, github_interval: int = 300, hf_interval: int = 60):
        self._running = True
        def _loop():
            gc, hc, lc = 0, 0, 0
            while self._running:
                try:
                    lc += 1; gc += 1; hc += 1
                    if lc >= 30: self.sync_local(); lc = 0
                    if gc >= github_interval: self.sync_github(); gc = 0
                    if hc >= hf_interval: self.sync_huggingface(); hc = 0
                    time.sleep(1)
                except Exception:
                    time.sleep(5)
        self._thread = threading.Thread(target=_loop, daemon=True)
        self._thread.start()

    def stop_loop(self):
        self._running = False
        if self._thread: self._thread.join(timeout=5)

    def get_status(self) -> Dict:
        return {
            "version": self.state.version,
            "last_full_sync": self.state.last_full_sync,
            "nodes": {"local": self.local.get_status(), "github": self.github.get_repo_status(),
                      "huggingface": self.huggingface.get_status()},
            "conflicts_unresolved": len(self.resolver.get_unresolved_conflicts()),
        }

    def print_status(self):
        s = self.get_status()
        print("""
╔══════════════════════════════════════════════════════════╗
║              TRI-MIND SYNC STATUS                       ║
╠══════════════════════════════════════════════════════════╣""")
        print(f"║  Version:   {s['version']}")
        print(f"║  Last Sync: {s['last_full_sync'] or 'never'}")
        print(f"║  Conflicts: {s['conflicts_unresolved']}")
        print("╠══════════════════════════════════════════════════════════╣")
        l = s["nodes"]["local"]
        print(f"║  LOCAL:     {l['total_files']} files, {l['changed_files']} changed, watching={l['watching']}")
        g = s["nodes"]["github"]
        if "error" in g: print(f"║  GITHUB:    {g['error']}")
        else: print(f"║  GITHUB:    {g.get('branch','?')} @ {g.get('commit','?')}, dirty={g.get('dirty')}")
        h = s["nodes"]["huggingface"]
        print(f"║  HF:        space={h.get('space_id')}, token={'set' if h.get('token_set') else 'MISSING'}")
        print("╚══════════════════════════════════════════════════════════╝")
