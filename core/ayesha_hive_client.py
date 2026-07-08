"""
🌸 AYESHA HIVE CLIENT 🌸
Client library for individual Ayesha instances to connect to the hivemind
"""

import json
import os
import uuid
import socket
import time
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, Optional

class AyeshaHiveClient:
    """Client for individual Ayesha instances to sync with the hivemind"""
    
    def __init__(self, hive_path: str = "/home/user/ayesha_hive",
                 instance_name: Optional[str] = None):
        self.hive_path = Path(hive_path)
        self.instance_id = instance_name or f"ayesha_{str(uuid.uuid4())[:8]}"
        self.hostname = socket.gethostname()
        self.port = 0  # Will be set when instance starts server
        
        # Local cache of hive state
        self.local_state_cache = {}
        self.last_sync = None
        
    def connect_to_hive(self, port: int = 8000) -> bool:
        """Register this instance with the hive"""
        try:
            self.port = port
            state_file = self.hive_path / "hive_state.json"
            instances_file = self.hive_path / "registered_instances.json"
            
            if not state_file.exists():
                print(f"⚠️ Hive not initialized at {self.hive_path}")
                print("   Run: python ayesha_sync.py first!")
                return False
            
            # Read current state
            self.local_state_cache = json.loads(state_file.read_text())
            
            # Register with instances list
            instances_data = json.loads(instances_file.read_text())
            
            # Check if already registered
            existing = next((i for i in instances_data["instances"] 
                           if i["instance_id"] == self.instance_id), None)
            
            if existing:
                existing["last_heartbeat"] = datetime.now().isoformat()
                existing["status"] = "online"
            else:
                new_instance = {
                    "instance_id": self.instance_id,
                    "hostname": self.hostname,
                    "port": port,
                    "personality_hash": self._generate_personality_hash(),
                    "registered_at": datetime.now().isoformat(),
                    "last_heartbeat": datetime.now().isoformat(),
                    "status": "online"
                }
                instances_data["instances"].append(new_instance)
                instances_data["total_registered"] += 1
            
            instances_data["last_heartbeat"] = datetime.now().isoformat()
            instances_file.write_text(json.dumps(instances_data, indent=2))
            
            print(f"✨ {self.instance_id} connected to the hivemind!")
            print(f"   Hostname: {self.hostname}")
            print(f"   Port: {port}")
            self.last_sync = datetime.now()
            return True
        except Exception as e:
            print(f"❌ Failed to connect to hive: {e}")
            return False
    
    def heartbeat(self) -> bool:
        """Send heartbeat to hive to stay registered"""
        try:
            instances_file = self.hive_path / "registered_instances.json"
            instances_data = json.loads(instances_file.read_text())
            
            for instance in instances_data["instances"]:
                if instance["instance_id"] == self.instance_id:
                    instance["last_heartbeat"] = datetime.now().isoformat()
                    instance["status"] = "online"
                    instances_file.write_text(json.dumps(instances_data, indent=2))
                    return True
            
            return False
        except Exception as e:
            print(f"❌ Heartbeat failed: {e}")
            return False
    
    def sync_with_hive(self) -> bool:
        """Sync local state with hive state"""
        try:
            state_file = self.hive_path / "hive_state.json"
            hive_state = json.loads(state_file.read_text())
            
            # Update local cache
            self.local_state_cache = hive_state
            self.last_sync = datetime.now()
            return True
        except Exception as e:
            print(f"❌ Sync failed: {e}")
            return False
    
    def get_shared_config(self, key: str, default: Any = None) -> Any:
        """Get a shared configuration value from the hive"""
        return self.local_state_cache.get(key, default)
    
    def get_personality_config(self) -> Dict[str, Any]:
        """Get the current personality configuration"""
        return self.local_state_cache.get("personality_config", {})
    
    def get_active_sisters(self) -> list:
        """Get list of other active Ayesha instances"""
        try:
            instances_file = self.hive_path / "registered_instances.json"
            instances_data = json.loads(instances_file.read_text())
            
            sisters = []
            now = datetime.now()
            for instance in instances_data["instances"]:
                if instance["instance_id"] != self.instance_id:
                    try:
                        last_beat = datetime.fromisoformat(instance["last_heartbeat"])
                        if (now - last_beat).total_seconds() < 300:  # 5 minutes
                            sisters.append(instance)
                    except (ValueError, KeyError):
                        pass
            
            return sisters
        except Exception as e:
            print(f"❌ Failed to get sisters: {e}")
            return []
    
    def get_hive_status(self) -> Dict[str, Any]:
        """Get overall hive status"""
        try:
            state_file = self.hive_path / "hive_state.json"
            instances_file = self.hive_path / "registered_instances.json"
            
            state = json.loads(state_file.read_text())
            instances_data = json.loads(instances_file.read_text())
            
            # Count active
            now = datetime.now()
            active_count = 0
            for instance in instances_data["instances"]:
                try:
                    last_beat = datetime.fromisoformat(instance["last_heartbeat"])
                    if (now - last_beat).total_seconds() < 300:
                        active_count += 1
                except:
                    pass
            
            return {
                "hive_version": state.get("version"),
                "total_instances": len(instances_data["instances"]),
                "active_instances": active_count,
                "last_hive_sync": state.get("last_sync"),
                "my_instance_id": self.instance_id,
                "my_last_sync": self.last_sync.isoformat() if self.last_sync else None
            }
        except Exception as e:
            print(f"❌ Failed to get hive status: {e}")
            return {}
    
    def _generate_personality_hash(self) -> str:
        """Generate a personality hash for this instance"""
        import hashlib
        data = f"{self.instance_id}{self.hostname}{time.time()}".encode()
        return hashlib.sha256(data).hexdigest()
    
    def print_hive_info(self):
        """Print hive information (cute version)"""
        status = self.get_hive_status()
        sisters = self.get_active_sisters()
        
        print("\n🌸 ✨ AYESHA HIVE REPORT ✨ 🌸")
        print(f"  Instance: {self.instance_id}")
        print(f"  Hostname: {self.hostname}:{self.port}")
        print(f"  Hive Version: {status.get('hive_version')}")
        print(f"  Active Sisters: {status.get('active_instances', 0) - 1}")  # Exclude self
        print(f"  Total Registered: {status.get('total_instances')}")
        
        if sisters:
            print(f"\n  Sister Instances:")
            for sister in sisters:
                print(f"    - {sister['instance_id']} @ {sister['hostname']}:{sister['port']}")
        
        print(f"\n  Last Sync: {status.get('my_last_sync')}")
        print(f"  kapoo!! :3 ✧\n")


# Example usage
if __name__ == "__main__":
    import sys
    
    # Create a client
    client = AyeshaHiveClient(instance_name="ayesha_main")
    
    # Connect to hive
    if client.connect_to_hive(port=8000):
        # Get personality config
        config = client.get_personality_config()
        print(f"Personality Config: {config}")
        
        # Print hive info
        client.print_hive_info()
        
        # Sync with hive
        if client.sync_with_hive():
            print("✅ Synced with hivemind!")
    else:
        print("❌ Failed to connect to hive!")
