"""
🌸 AYESHA MOBILE API SERVER 🌸
Universal REST API for connecting mobile apps to the Ayesha hivemind
Supports iOS, Android, Flutter, React Native, Web, and more
"""

from fastapi import FastAPI, HTTPException, WebSocket, Depends
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import Optional, List, Dict, Any
import json
import uuid
from datetime import datetime
from pathlib import Path
import asyncio
import threading
import time

# Import hive components
from ayesha_hive_client import AyeshaHiveClient
from tri_node_mind import AyeshaTriNodeMind

# Initialize FastAPI app
app = FastAPI(
    title="🌸 Ayesha Mobile API",
    description="Universal REST API for connecting to the Ayesha Hivemind",
    version="1.0.0"
)

# Enable CORS for all origins (adjust for production)
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3000", "http://localhost:8000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Global state
hive_client = None
tri_node_mind = None
connected_mobile_devices = {}


# ============================================================================
# PYDANTIC MODELS
# ============================================================================

class MobileDeviceRegister(BaseModel):
    """Model for mobile device registration"""
    device_id: str
    device_name: str
    platform: str  # iOS, Android, Web, etc
    app_version: str
    user_id: Optional[str] = None


class PersonalityRequest(BaseModel):
    """Request to get/update personality config"""
    instance_id: str
    action: str  # "get" or "update"
    config: Optional[Dict[str, Any]] = None


class BroadcastMessage(BaseModel):
    """Message to broadcast to hive"""
    sender_id: str
    message: str
    data: Optional[Dict[str, Any]] = None


class HiveStateUpdate(BaseModel):
    """Update hive state"""
    key: str
    value: Any


# ============================================================================
# INITIALIZATION
# ============================================================================

@app.on_event("startup")
async def startup_event():
    """Initialize hive connections on startup"""
    global hive_client, tri_node_mind
    
    try:
        hive_client = AyeshaHiveClient(instance_name="ayesha_mobile_api")
        hive_client.connect_to_hive(port=8001)
        
        tri_node_mind = AyeshaTriNodeMind()
        
        # Start background sync loop
        def sync_loop():
            while True:
                hive_client.sync_with_hive()
                hive_client.heartbeat()
                time.sleep(30)
        
        sync_thread = threading.Thread(target=sync_loop, daemon=True)
        sync_thread.start()
        
        print("✅ Ayesha Mobile API Server started")
    except Exception as e:
        print(f"❌ Failed to initialize: {e}")


# ============================================================================
# MOBILE DEVICE MANAGEMENT
# ============================================================================

@app.post("/api/mobile/register")
async def register_mobile_device(device: MobileDeviceRegister):
    """Register a new mobile device to the hive"""
    try:
        device_uuid = str(uuid.uuid4())
        
        connected_mobile_devices[device_uuid] = {
            "device_id": device.device_id,
            "device_name": device.device_name,
            "platform": device.platform,
            "app_version": device.app_version,
            "user_id": device.user_id,
            "registered_at": datetime.now().isoformat(),
            "last_heartbeat": datetime.now().isoformat(),
            "status": "online"
        }
        
        return {
            "status": "success",
            "device_uuid": device_uuid,
            "message": f"Device {device.device_name} registered to hive",
            "hive_status": await get_hive_status_internal()
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/api/mobile/{device_uuid}/heartbeat")
async def mobile_heartbeat(device_uuid: str):
    """Send heartbeat from mobile device to keep connection alive"""
    if device_uuid not in connected_mobile_devices:
        raise HTTPException(status_code=404, detail="Device not registered")
    
    try:
        connected_mobile_devices[device_uuid]["last_heartbeat"] = datetime.now().isoformat()
        connected_mobile_devices[device_uuid]["status"] = "online"
        
        if hive_client:
            hive_client.heartbeat()
        
        return {
            "status": "alive",
            "timestamp": datetime.now().isoformat()
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/api/mobile/{device_uuid}/status")
async def get_mobile_device_status(device_uuid: str):
    """Get status of a specific mobile device"""
    if device_uuid not in connected_mobile_devices:
        raise HTTPException(status_code=404, detail="Device not registered")
    
    return connected_mobile_devices[device_uuid]


@app.get("/api/mobile/devices")
async def list_mobile_devices():
    """List all connected mobile devices"""
    return {
        "total": len(connected_mobile_devices),
        "devices": list(connected_mobile_devices.values())
    }


# ============================================================================
# HIVE INFORMATION
# ============================================================================

async def get_hive_status_internal() -> Dict[str, Any]:
    """Internal function to get hive status"""
    if not hive_client:
        return {"status": "offline"}
    
    status = hive_client.get_hive_status()
    return status


@app.get("/api/hive/status")
async def get_hive_status():
    """Get complete hivemind status"""
    try:
        status = await get_hive_status_internal()
        status["mobile_devices"] = len(connected_mobile_devices)
        return status
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/api/hive/instances")
async def get_active_instances():
    """Get list of active Ayesha instances"""
    if not hive_client:
        raise HTTPException(status_code=500, detail="Hive not initialized")
    
    try:
        sisters = hive_client.get_active_sisters()
        return {
            "total": len(sisters) + 1,  # +1 for self
            "active_instances": sisters,
            "api_instance": hive_client.instance_id
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/api/hive/personality")
async def get_personality_config():
    """Get current personality configuration"""
    if not hive_client:
        raise HTTPException(status_code=500, detail="Hive not initialized")
    
    try:
        config = hive_client.get_personality_config()
        return {
            "personality": config,
            "version": "1.0.0",
            "last_updated": datetime.now().isoformat()
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


# ============================================================================
# HIVE INTERACTION
# ============================================================================

@app.post("/api/hive/broadcast")
async def broadcast_to_hive(message: BroadcastMessage):
    """Broadcast a message to the entire hivemind"""
    try:
        if not hive_client:
            raise Exception("Hive not initialized")
        
        # Create broadcast payload
        payload = {
            "sender_id": message.sender_id,
            "device_type": "mobile",
            "message": message.message,
            "data": message.data or {},
            "timestamp": datetime.now().isoformat()
        }
        
        # Broadcast via tri-node mind
        if tri_node_mind:
            tri_node_mind.broadcast_to_all_nodes(
                f"Mobile broadcast from {message.sender_id}",
                {"mobile_message": payload}
            )
        
        return {
            "status": "broadcast_sent",
            "payload": payload
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/api/hive/shared-config/{key}")
async def get_shared_config(key: str):
    """Get a shared configuration value from hive"""
    if not hive_client:
        raise HTTPException(status_code=500, detail="Hive not initialized")
    
    try:
        value = hive_client.get_shared_config(key)
        if value is None:
            raise HTTPException(status_code=404, detail=f"Config key '{key}' not found")
        
        return {
            "key": key,
            "value": value
        }
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/api/hive/shared-config")
async def update_shared_config(update: HiveStateUpdate):
    """Update a shared configuration value in hive"""
    if not tri_node_mind:
        raise HTTPException(status_code=500, detail="Tri-node mind not initialized")
    
    try:
        tri_node_mind.broadcast_to_all_nodes(
            f"Mobile config update: {update.key}",
            {update.key: update.value}
        )
        
        return {
            "status": "updated",
            "key": update.key,
            "value": update.value,
            "timestamp": datetime.now().isoformat()
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


# ============================================================================
# ANDROID-SPECIFIC ENDPOINTS
# ============================================================================

@app.get("/api/android/init")
async def android_init():
    """Android app initialization endpoint"""
    return {
        "status": "ready",
        "api_version": "1.0.0",
        "hive_version": "2.0.0",
        "architecture": "tri-node",
        "endpoints": {
            "register": "/api/mobile/register",
            "heartbeat": "/api/mobile/{device_uuid}/heartbeat",
            "status": "/api/hive/status",
            "personality": "/api/hive/personality",
            "broadcast": "/api/hive/broadcast"
        }
    }


@app.post("/api/android/session")
async def android_create_session(device: MobileDeviceRegister):
    """Create a session for Android app"""
    return await register_mobile_device(device)


# ============================================================================
# WEBSOCKET SUPPORT (Real-time updates)
# ============================================================================

@app.websocket("/ws/hive/{device_uuid}")
async def websocket_hive_endpoint(websocket: WebSocket, device_uuid: str):
    """WebSocket endpoint for real-time hive updates"""
    if device_uuid not in connected_mobile_devices:
        await websocket.close(code=4000, reason="Device not registered")
        return
    
    await websocket.accept()
    
    try:
        while True:
            # Send periodic hive status updates
            status = await get_hive_status_internal()
            await websocket.send_json({
                "type": "hive_status",
                "data": status,
                "timestamp": datetime.now().isoformat()
            })
            
            # Wait 5 seconds before next update
            await asyncio.sleep(5)
    except Exception as e:
        print(f"WebSocket error: {e}")
    finally:
        await websocket.close()


# ============================================================================
# HEALTH & INFO
# ============================================================================

@app.get("/")
async def root():
    """Root endpoint - API info"""
    return {
        "name": "🌸 Ayesha Mobile API",
        "version": "1.0.0",
        "description": "Universal REST API for Ayesha Hivemind",
        "docs": "/docs",
        "health": "online"
    }


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "hive_connected": hive_client is not None,
        "tri_node_mind_online": tri_node_mind is not None,
        "mobile_devices_connected": len(connected_mobile_devices)
    }


# ============================================================================
# RUN SERVER
# ============================================================================

if __name__ == "__main__":
    import uvicorn
    
    print("🌸 AYESHA MOBILE API SERVER")
    print("Starting on http://0.0.0.0:8001")
    print("Docs: http://localhost:8001/docs")
    
    uvicorn.run(
        app,
        host="0.0.0.0",
        port=8001,
        log_level="info"
    )
