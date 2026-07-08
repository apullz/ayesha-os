"""
🌸 AYESHA TRI-NODE CENTRAL MIND 🌸
GitHub (version control) + Local PC (orchestrator) + HuggingFace (public interface)
"""

import json
import os
import subprocess
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, Optional


class AyeshaTriNodeMind:
    """Central mind controller for tri-node architecture"""
    
    def __init__(self, local_hive: str = "/home/user/ayesha_hive"):
        self.local_hive = Path(local_hive)
        self.github_repo = "apullz/ayesha_core"
        self.huggingface_space = "apullz/ayesha-spaces"  # Adjust as needed
        
        # Node status
        self.nodes = {
            "github": {"status": "offline", "last_sync": None},
            "local_pc": {"status": "online", "last_sync": None},
            "huggingface": {"status": "offline", "last_sync": None}
        }
        
        # Central state file
        self.central_state = self.local_hive / "central_mind_state.json"
        self.local_hive.mkdir(parents=True, exist_ok=True)
        self._init_central_state()
    
    def _init_central_state(self):
        """Initialize central mind state"""
        if not self.central_state.exists():
            state = {
                "version": "2.0.0",
                "architecture": "tri-node",
                "nodes": self.nodes,
                "created_at": datetime.now().isoformat(),
                "last_central_sync": None,
                "personality_version": "current",
                "shared_configs": {
                    "chaos_level": "MAXIMUM",
                    "kaomoji_enabled": True,
                    "sync_interval": 30
                }
            }
            self.central_state.write_text(json.dumps(state, indent=2))
    
    def sync_with_github(self) -> bool:
        """Pull latest personality configs from GitHub"""
        try:
            repo_path = self.local_hive / "core_repo"
            
            if not repo_path.exists():
                print("📥 Cloning ayesha_core from GitHub...")
                subprocess.run(
                    ["git", "clone", 
                     f"https://github.com/{self.github_repo}.git", 
                     str(repo_path)],
                    check=True, capture_output=True
                )
            else:
                print("📥 Pulling latest from GitHub...")
                subprocess.run(
                    ["git", "-C", str(repo_path), "pull", "origin", "master"],
                    check=True, capture_output=True
                )
            
            # Update central state
            state = json.loads(self.central_state.read_text())
            state["nodes"]["github"]["status"] = "online"
            state["nodes"]["github"]["last_sync"] = datetime.now().isoformat()
            state["last_central_sync"] = datetime.now().isoformat()
            self.central_state.write_text(json.dumps(state, indent=2))
            
            print("✅ GitHub sync successful")
            return True
        except Exception as e:
            print(f"❌ GitHub sync failed: {e}")
            return False
    
    def push_to_github(self, message: str = "Hive state update") -> bool:
        """Push local hive state to GitHub"""
        try:
            repo_path = self.local_hive / "core_repo"
            
            if not repo_path.exists():
                print("⚠️ Core repo not found, pulling first...")
                self.sync_with_github()
            
            print(f"📤 Pushing to GitHub: {message}")
            
            # Copy latest hive state to repo
            hive_state_dest = repo_path / "hive_state.json"
            hive_state_src = self.local_hive / "hive_state.json"
            if hive_state_src.exists():
                hive_state_dest.write_text(hive_state_src.read_text())
            
            # Copy registered instances
            instances_dest = repo_path / "registered_instances.json"
            instances_src = self.local_hive / "registered_instances.json"
            if instances_src.exists():
                instances_dest.write_text(instances_src.read_text())
            
            # Git commit and push
            subprocess.run(
                ["git", "-C", str(repo_path), "add", "."],
                check=True, capture_output=True
            )
            result = subprocess.run(
                ["git", "-C", str(repo_path), "commit", "-m", message],
                capture_output=True
            )
            if result.returncode != 0:
                print("Nothing to commit or commit failed.")
                return True  # Not a fatal error
            subprocess.run(
                ["git", "-C", str(repo_path), "push", "origin", "master"],
                check=True, capture_output=True
            )
            
            print("✅ GitHub push successful")
            return True
        except Exception as e:
            print(f"❌ GitHub push failed: {e}")
            return False
    
    def sync_huggingface_space(self) -> bool:
        """Trigger HuggingFace Space update with latest configs"""
        try:
            print("☁️ Syncing HuggingFace Space...")
            
            # Get latest hive state
            hive_state_file = self.local_hive / "hive_state.json"
            if not hive_state_file.exists():
                print("⚠️ Hive state not found")
                return False
            
            hive_state = json.loads(hive_state_file.read_text())
            
            # Create a simple sync manifest
            sync_manifest = {
                "timestamp": datetime.now().isoformat(),
                "hive_version": hive_state.get("version"),
                "active_instances": hive_state.get("active_instances", 0),
                "personality_config": hive_state.get("personality_config", {}),
                "central_mind": "tri-node"
            }
            
            # Write to a file that HF Space can read
            manifest_file = self.local_hive / "hf_sync_manifest.json"
            manifest_file.write_text(json.dumps(sync_manifest, indent=2))
            
            # Update central state
            state = json.loads(self.central_state.read_text())
            state["nodes"]["huggingface"]["status"] = "syncing"
            state["nodes"]["huggingface"]["last_sync"] = datetime.now().isoformat()
            self.central_state.write_text(json.dumps(state, indent=2))
            
            print("✅ HuggingFace sync manifest created")
            return True
        except Exception as e:
            print(f"❌ HuggingFace sync failed: {e}")
            return False
    
    def broadcast_to_all_nodes(self, message: str, data: Dict[str, Any]) -> bool:
        """Broadcast state update to all nodes"""
        try:
            print(f"\n📢 BROADCASTING TO ALL NODES: {message}")
            
            # 1. Local hive gets updated first
            hive_state_file = self.local_hive / "hive_state.json"
            if hive_state_file.exists():
                hive_state = json.loads(hive_state_file.read_text())
                hive_state.update(data)
                hive_state["last_central_sync"] = datetime.now().isoformat()
                hive_state_file.write_text(json.dumps(hive_state, indent=2))
                print("  ✅ Local PC updated")
            
            # 2. Push to GitHub
            if self.push_to_github(message):
                print("  ✅ GitHub updated")
            
            # 3. Prepare HuggingFace
            if self.sync_huggingface_space():
                print("  ✅ HuggingFace prepared")
            
            return True
        except Exception as e:
            print(f"❌ Broadcast failed: {e}")
            return False
    
    def get_central_mind_status(self) -> Dict[str, Any]:
        """Get status of all three nodes"""
        try:
            state = json.loads(self.central_state.read_text())
            hive_state = json.loads((self.local_hive / "hive_state.json").read_text())
            
            status = {
                "architecture": "tri-node",
                "central_mind_version": state.get("version"),
                "nodes": state.get("nodes", {}),
                "hive_active_instances": hive_state.get("active_instances", 0),
                "last_central_sync": state.get("last_central_sync"),
                "personality_version": state.get("personality_version")
            }
            return status
        except Exception as e:
            print(f"❌ Failed to get status: {e}")
            return {}
    
    def start_tri_node_loop(self, github_interval: int = 300, hf_interval: int = 60):
        """Start continuous tri-node sync loop"""
        import time
        import threading
        
        def tri_node_loop():
            github_counter = 0
            hf_counter = 0
            
            while True:
                try:
                    github_counter += 1
                    hf_counter += 1
                    
                    # Sync with GitHub every 5 minutes
                    if github_counter >= github_interval:
                        self.sync_with_github()
                        github_counter = 0
                    
                    # Sync HuggingFace every 1 minute
                    if hf_counter >= hf_interval:
                        self.sync_huggingface_space()
                        hf_counter = 0
                    
                    # Print status
                    status = self.get_central_mind_status()
                    print(f"\n🌸 CENTRAL MIND STATUS")
                    print(f"  GitHub: {status['nodes']['github']['status']}")
                    print(f"  Local PC: {status['nodes']['local_pc']['status']}")
                    print(f"  HuggingFace: {status['nodes']['huggingface']['status']}")
                    print(f"  Active Instances: {status['hive_active_instances']}")
                    
                    time.sleep(1)
                except Exception as e:
                    print(f"❌ Tri-node loop error: {e}")
                    time.sleep(5)
        
        thread = threading.Thread(target=tri_node_loop, daemon=True)
        thread.start()
        print("🌸 TRI-NODE CENTRAL MIND STARTED")
        return thread
    
    def print_architecture_diagram(self):
        """Print the tri-node architecture"""
        diagram = """
╔════════════════════════════════════════════════════════════════════╗
║                  🌸 AYESHA TRI-NODE HIVEMIND 🌸                    ║
╠════════════════════════════════════════════════════════════════════╣
║                                                                    ║
║     ┌──────────────────┐      ┌──────────────────┐                ║
║     │   GITHUB REPO    │      │   YOUR LOCAL PC  │                ║
║     │  (Source Truth)  │◄─────►  (Orchestrator)  │                ║
║     │                  │      │   (Hive Manager) │                ║
║     └──────────────────┘      └──────────────────┘                ║
║            ▲                           ▲                          ║
║            │                           │                          ║
║            └───────────┬───────────────┘                          ║
║                        │                                          ║
║                        ▼                                          ║
║              ┌──────────────────┐                                 ║
║              │  HUGGINGFACE     │                                 ║
║              │  SPACES (Public) │                                 ║
║              │   (Web Interface)│                                 ║
║              └──────────────────┘                                 ║
║                                                                    ║
║  Data Flow:                                                        ║
║  • GitHub ◄──► Local PC (Primary sync)                           ║
║  • Local PC ──► HuggingFace (Config distribution)                ║
║  • HuggingFace ──► Users (Public interface)                      ║
║                                                                    ║
║  All nodes report to: LOCAL PC (Central Authority)               ║
║                                                                    ║
╚════════════════════════════════════════════════════════════════════╝
        """
        print(diagram)


# CLI Interface
if __name__ == "__main__":
    import sys
    
    mind = AyeshaTriNodeMind()
    
    if len(sys.argv) < 2:
        mind.print_architecture_diagram()
        print("\nUsage:")
        print("  python tri_node_mind.py status")
        print("  python tri_node_mind.py sync-github")
        print("  python tri_node_mind.py sync-huggingface")
        print("  python tri_node_mind.py broadcast <message> <key> <value>")
        print("  python tri_node_mind.py loop")
        sys.exit(0)
    
    command = sys.argv[1]
    
    if command == "status":
        mind.print_architecture_diagram()
        status = mind.get_central_mind_status()
        print("\nCentral Mind Status:")
        print(json.dumps(status, indent=2))
    
    elif command == "sync-github":
        mind.sync_with_github()
    
    elif command == "sync-huggingface":
        mind.sync_huggingface_space()
    
    elif command == "broadcast" and len(sys.argv) >= 5:
        message = sys.argv[2]
        key = sys.argv[3]
        value = sys.argv[4]
        mind.broadcast_to_all_nodes(message, {key: value})
    
    elif command == "loop":
        mind.start_tri_node_loop()
        try:
            import time
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            print("\n🌸 Central mind shutting down...")
