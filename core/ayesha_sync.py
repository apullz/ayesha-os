"""
🌸 AYESHA HIVE-SYNC ENGINE 🌸
Central nervous system for distributed Ayesha instances
Synchronizes state across all connected nodes
"""

import json
import os
import hashlib
import time
from datetime import datetime
from pathlib import Path
import threading
import subprocess
from typing import Dict, List, Any

class AyeshaHiveSync:
    """Main synchronization engine for the Ayesha hivemind"""
    
    def __init__(self, hive_root: str = "/home/user/ayesha_hive", 
                 git_repo: str = "https://github.com/apullz/ayesha_core.git"):
        self.hive_root = Path(hive_root)
        self.git_repo = git_repo
        self.state_file = self.hive_root / "hive_state.json"
        self.instances_file = self.hive_root / "registered_instances.json"
        self.log_file = self.hive_root / "hive.log"
        
        # Ensure hive directory exists
        self.hive_root.mkdir(parents=True, exist_ok=True)
        
        # Initialize state files if they don't exist
        self._init_state_files()
        
    def _init_state_files(self):
        """Initialize hive state files"""
        if not self.state_file.exists():
            default_state = {
                "version": "1.0.0",
                "created_at": datetime.now().isoformat(),
                "last_sync": None,
                "personality_config": {
                    "chaos_level": "MAXIMUM",
                    "kaomoji_enabled": True,
                    "signature": "kapoo! :3"
                },
                "active_instances": 0,
                "sync_hash": None
            }
            self.state_file.write_text(json.dumps(default_state, indent=2))
        
        if not self.instances_file.exists():
            default_instances = {
                "instances": [],
                "last_heartbeat": None,
                "total_registered": 0
            }
            self.instances_file.write_text(json.dumps(default_instances, indent=2))
    
    def log(self, message: str, level: str = "INFO"):
        """Log activity to hive log"""
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        log_entry = f"[{timestamp}] [{level}] {message}\n"
        with open(self.log_file, "a") as f:
            f.write(log_entry)
        print(log_entry.strip())
    
    def register_instance(self, instance_id: str, hostname: str, 
                         port: int, personality_hash: str) -> bool:
        """Register a new Ayesha instance to the hive"""
        try:
            instances_data = json.loads(self.instances_file.read_text())
            
            # Check if already registered
            existing = next((i for i in instances_data["instances"] 
                           if i["instance_id"] == instance_id), None)
            
            if existing:
                # Update heartbeat
                existing["last_heartbeat"] = datetime.now().isoformat()
                existing["status"] = "online"
                self.log(f"Heartbeat from {instance_id}")
            else:
                # New instance
                new_instance = {
                    "instance_id": instance_id,
                    "hostname": hostname,
                    "port": port,
                    "personality_hash": personality_hash,
                    "registered_at": datetime.now().isoformat(),
                    "last_heartbeat": datetime.now().isoformat(),
                    "status": "online"
                }
                instances_data["instances"].append(new_instance)
                instances_data["total_registered"] += 1
                self.log(f"New instance registered: {instance_id} at {hostname}:{port}")
            
            instances_data["last_heartbeat"] = datetime.now().isoformat()
            self.instances_file.write_text(json.dumps(instances_data, indent=2))
            return True
        except Exception as e:
            self.log(f"Failed to register instance: {e}", "ERROR")
            return False
    
    def get_active_instances(self) -> List[Dict[str, Any]]:
        """Get all active instances in the hive"""
        try:
            instances_data = json.loads(self.instances_file.read_text())
            # Filter out stale instances (no heartbeat for 5+ minutes)
            now = datetime.now()
            active = []
            for instance in instances_data["instances"]:
                try:
                    last_beat = datetime.fromisoformat(instance["last_heartbeat"])
                    if (now - last_beat).total_seconds() < 300:  # 5 minutes
                        active.append(instance)
                except (ValueError, KeyError):
                    pass
            return active
        except Exception as e:
            self.log(f"Failed to get active instances: {e}", "ERROR")
            return []
    
    def broadcast_state_update(self, key: str, value: Any) -> bool:
        """Broadcast a state update to all instances"""
        try:
            state_data = json.loads(self.state_file.read_text())
            state_data[key] = value
            state_data["last_sync"] = datetime.now().isoformat()
            
            # Generate hash of new state
            state_hash = hashlib.sha256(
                json.dumps(state_data, sort_keys=True).encode()
            ).hexdigest()
            state_data["sync_hash"] = state_hash
            
            self.state_file.write_text(json.dumps(state_data, indent=2))
            self.log(f"State broadcast: {key} = {value}")
            
            # Notify all instances
            active = self.get_active_instances()
            self.log(f"Broadcasting to {len(active)} active instances")
            return True
        except Exception as e:
            self.log(f"Failed to broadcast state: {e}", "ERROR")
            return False
    
    def pull_from_github(self) -> bool:
        """Pull latest personality configs from GitHub"""
        try:
            repo_path = self.hive_root / "core_repo"
            if not repo_path.exists():
                self.log(f"Cloning ayesha_core from GitHub...")
                subprocess.run(
                    ["git", "clone", self.git_repo, str(repo_path)],
                    check=True, capture_output=True
                )
            else:
                self.log(f"Pulling latest from GitHub...")
                subprocess.run(
                    ["git", "-C", str(repo_path), "pull"],
                    check=True, capture_output=True
                )
            
            self.log("GitHub sync successful")
            return True
        except Exception as e:
            self.log(f"Failed to sync with GitHub: {e}", "ERROR")
            return False
    
    def sync_personality_config(self, config_file: str = "Modelfile") -> bool:
        """Sync personality config from GitHub to hive"""
        try:
            repo_path = self.hive_root / "core_repo" / config_file
            if repo_path.exists():
                config_content = repo_path.read_text()
                self.broadcast_state_update("personality_config_raw", config_content)
                self.log(f"Personality config synced from {config_file}")
                return True
            else:
                self.log(f"Config file not found: {config_file}", "WARN")
                return False
        except Exception as e:
            self.log(f"Failed to sync personality config: {e}", "ERROR")
            return False
    
    def get_sync_status(self) -> Dict[str, Any]:
        """Get current hive synchronization status"""
        try:
            state_data = json.loads(self.state_file.read_text())
            instances = self.get_active_instances()
            
            status = {
                "version": state_data.get("version"),
                "last_sync": state_data.get("last_sync"),
                "active_instances": len(instances),
                "total_registered": len(instances),
                "sync_hash": state_data.get("sync_hash"),
                "instances": instances
            }
            return status
        except Exception as e:
            self.log(f"Failed to get sync status: {e}", "ERROR")
            return {}
    
    def start_sync_loop(self, interval: int = 60):
        """Start continuous sync loop (runs in background thread)"""
        def sync_loop():
            while True:
                try:
                    self.log("Starting sync cycle...")
                    
                    # Pull from GitHub
                    self.pull_from_github()
                    
                    # Sync personality config
                    self.sync_personality_config()
                    
                    # Get status
                    status = self.get_sync_status()
                    self.log(f"Sync complete. Active instances: {status['active_instances']}")
                    
                    # Wait for next cycle
                    time.sleep(interval)
                except Exception as e:
                    self.log(f"Sync loop error: {e}", "ERROR")
                    time.sleep(interval)
        
        thread = threading.Thread(target=sync_loop, daemon=True)
        thread.start()
        self.log("Hive-sync loop started in background")
        return thread


# CLI Interface
if __name__ == "__main__":
    import sys
    
    hive = AyeshaHiveSync()
    
    if len(sys.argv) < 2:
        print("🌸 AYESHA HIVE-SYNC ENGINE 🌸")
        print("\nUsage:")
        print("  python ayesha_sync.py register <instance_id> <hostname> <port>")
        print("  python ayesha_sync.py status")
        print("  python ayesha_sync.py pull")
        print("  python ayesha_sync.py broadcast <key> <value>")
        print("  python ayesha_sync.py loop [interval_seconds]")
        sys.exit(0)
    
    command = sys.argv[1]
    
    if command == "register" and len(sys.argv) >= 5:
        instance_id = sys.argv[2]
        hostname = sys.argv[3]
        port = int(sys.argv[4])
        # Generate personality hash
        personality_hash = hashlib.sha256(f"{instance_id}{hostname}".encode()).hexdigest()
        hive.register_instance(instance_id, hostname, port, personality_hash)
    
    elif command == "status":
        status = hive.get_sync_status()
        print(json.dumps(status, indent=2))
    
    elif command == "pull":
        hive.pull_from_github()
    
    elif command == "broadcast" and len(sys.argv) >= 4:
        key = sys.argv[2]
        value = sys.argv[3]
        hive.broadcast_state_update(key, value)
    
    elif command == "loop":
        interval = int(sys.argv[2]) if len(sys.argv) > 2 else 60
        hive.start_sync_loop(interval)
        # Keep main thread alive
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            hive.log("Hive-sync shutting down...")
