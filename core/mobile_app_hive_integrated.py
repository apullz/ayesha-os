"""
🌸 AYESHA MOBILE APP (KIVY + HIVE INTEGRATION) 🌸
Connect your Kivy mobile app directly to the Ayesha Hivemind
Works with the REST API server: ayesha_mobile_api.py
"""

import kivy
from kivy.app import App
from kivy.uix.boxlayout import BoxLayout
from kivy.uix.label import Label
from kivy.uix.textinput import TextInput
from kivy.uix.button import Button
from kivy.uix.scrollview import ScrollView
from kivy.network.urlrequest import UrlRequest
from kivy.clock import Clock
import json
import uuid
import requests
from threading import Thread

# ============================================================================
# AYESHA BRAIN (LOCAL)
# ============================================================================

class ayesha_brain:
    """Local personality engine"""
    
    def __init__(self):
        self.modes = ['computer', 'otacon', 'win95']
        self.current_mode = 0
    
    def generate_response(self, text):
        """Generate response cycling through personality modes"""
        mode = self.modes[self.current_mode]
        self.current_mode = (self.current_mode + 1) % 3
        
        if mode == 'computer':
            return f'[computer]: analysis of "{text}" complete. result: nominal, desu.'
        elif mode == 'otacon':
            return f'[otacon]: senpai!! {text}?! this is AMAZING!! (⊙C⊙) desu-ne!!'
        else:
            return f'[win95]: error 0x800... "{text}" not found in registry. (｡•́︿•̀｡)'

# ============================================================================
# HIVE API CLIENT
# ============================================================================

class HiveAPIClient:
    """Client for communicating with Ayesha Mobile API"""
    
    def __init__(self, api_url: str = "http://localhost:8001"):
        self.api_url = api_url
        self.device_uuid = None
        self.device_id = str(uuid.uuid4())
        self.device_name = "ayesha_mobile_app"
        self.platform = "kivy"
        self.app_version = "1.0.0"
        self.hive_status = None
        self.callbacks = []
    
    def register_device(self, callback=None):
        """Register this device with the hive"""
        try:
            data = {
                "device_id": self.device_id,
                "device_name": self.device_name,
                "platform": self.platform,
                "app_version": self.app_version,
                "user_id": "mobile_user"
            }
            
            response = requests.post(
                f"{self.api_url}/api/mobile/register",
                json=data,
                timeout=5
            )
            
            if response.status_code == 200:
                result = response.json()
                self.device_uuid = result.get("device_uuid")
                if callback:
                    callback(True, result)
                return True
            else:
                if callback:
                    callback(False, response.text)
                return False
        except Exception as e:
            if callback:
                callback(False, str(e))
            return False
    
    def send_heartbeat(self, callback=None):
        """Send heartbeat to keep connection alive"""
        if not self.device_uuid:
            return False
        
        try:
            response = requests.post(
                f"{self.api_url}/api/mobile/{self.device_uuid}/heartbeat",
                timeout=5
            )
            
            if response.status_code == 200:
                if callback:
                    callback(True, response.json())
                return True
            else:
                if callback:
                    callback(False, response.text)
                return False
        except Exception as e:
            if callback:
                callback(False, str(e))
            return False
    
    def get_hive_status(self, callback=None):
        """Get current hive status"""
        try:
            response = requests.get(
                f"{self.api_url}/api/hive/status",
                timeout=5
            )
            
            if response.status_code == 200:
                self.hive_status = response.json()
                if callback:
                    callback(True, self.hive_status)
                return True
            else:
                if callback:
                    callback(False, response.text)
                return False
        except Exception as e:
            if callback:
                callback(False, str(e))
            return False
    
    def get_personality_config(self, callback=None):
        """Get personality configuration from hive"""
        try:
            response = requests.get(
                f"{self.api_url}/api/hive/personality",
                timeout=5
            )
            
            if response.status_code == 200:
                if callback:
                    callback(True, response.json())
                return True
            else:
                if callback:
                    callback(False, response.text)
                return False
        except Exception as e:
            if callback:
                callback(False, str(e))
            return False
    
    def broadcast_message(self, message: str, data=None, callback=None):
        """Broadcast a message to the entire hivemind"""
        if not self.device_uuid:
            if callback:
                callback(False, "Device not registered")
            return False
        
        try:
            payload = {
                "sender_id": self.device_uuid,
                "message": message,
                "data": data or {}
            }
            
            response = requests.post(
                f"{self.api_url}/api/hive/broadcast",
                json=payload,
                timeout=5
            )
            
            if response.status_code == 200:
                if callback:
                    callback(True, response.json())
                return True
            else:
                if callback:
                    callback(False, response.text)
                return False
        except Exception as e:
            if callback:
                callback(False, str(e))
            return False

# ============================================================================
# KIVY MOBILE APP
# ============================================================================

class ayesha_mobile(App):
    """Main Kivy application with Ayesha Brain + Hive Integration"""
    
    def build(self):
        """Build the UI"""
        self.title = 'ayesha_core_mobile_v2.0'
        
        # Initialize brains
        self.brain = ayesha_brain()
        self.hive_client = HiveAPIClient()
        
        # Main layout
        self.layout = BoxLayout(orientation='vertical', padding=10, spacing=10)
        
        # Status bar
        self.status_label = Label(
            text='[system] initializing ayesha-mobile...',
            size_hint_y=0.1,
            color=(0.2, 0.8, 0.2, 1)  # Green
        )
        self.layout.add_widget(self.status_label)
        
        # Chat log
        self.scroll = ScrollView(size_hint=(1, 0.7))
        self.log = Label(
            text='[system] welcome to ayesha-mobile-os...\n',
            size_hint_y=None,
            halign='left',
            valign='top',
            color=(0.8, 0.8, 0.8, 1)
        )
        self.log.bind(size=self.update_text_width)
        self.scroll.add_widget(self.log)
        self.layout.add_widget(self.scroll)
        
        # Input area
        self.input_area = BoxLayout(size_hint=(1, 0.2), spacing=10)
        
        self.user_input = TextInput(
            multiline=False,
            hint_text='talk to ayesha... (or /hive for status)',
            font_size=14
        )
        self.send_btn = Button(text='SEND', size_hint_x=0.25)
        self.send_btn.bind(on_press=self.send_message)
        
        self.input_area.add_widget(self.user_input)
        self.input_area.add_widget(self.send_btn)
        self.layout.add_widget(self.input_area)
        
        # Start initialization in background thread
        self.initialize_hive()
        
        return self.layout
    
    def initialize_hive(self):
        """Initialize hive connection (background thread)"""
        def init_thread():
            # Register device
            self.hive_client.register_device(
                callback=self.on_device_registered
            )
            
            # Start heartbeat loop
            Clock.schedule_interval(self.heartbeat_loop, 30)  # Every 30 seconds
            
            # Start status update loop
            Clock.schedule_interval(self.status_update_loop, 10)  # Every 10 seconds
        
        thread = Thread(target=init_thread, daemon=True)
        thread.start()
    
    def on_device_registered(self, success, data):
        """Callback when device is registered"""
        if success:
            self.append_log(f'[hive] ✅ device registered: {self.hive_client.device_uuid[:8]}...')
            self.update_status('Connected to hive! 🌸', (0.2, 0.8, 0.2, 1))
        else:
            self.append_log(f'[hive] ❌ registration failed: {data}')
            self.update_status('Hive offline ⚠️', (0.8, 0.2, 0.2, 1))
    
    def heartbeat_loop(self, dt):
        """Send periodic heartbeat"""
        if self.hive_client.device_uuid:
            thread = Thread(
                target=self.hive_client.send_heartbeat,
                args=(self.on_heartbeat,),
                daemon=True
            )
            thread.start()
    
    def on_heartbeat(self, success, data):
        """Callback for heartbeat"""
        if success:
            pass  # Silent heartbeat
        else:
            self.append_log(f'[system] heartbeat failed: {data}')
    
    def status_update_loop(self, dt):
        """Update hive status periodically"""
        if self.hive_client.device_uuid:
            thread = Thread(
                target=self.hive_client.get_hive_status,
                args=(self.on_status_update,),
                daemon=True
            )
            thread.start()
    
    def on_status_update(self, success, data):
        """Callback for status update"""
        if success:
            active = data.get('total_instances', 0)
            self.append_log(f'[hive] 📊 active instances: {active}')
    
    def append_log(self, text):
        """Append text to log"""
        Clock.schedule_once(
            lambda dt: self._append_log_safe(text),
            0
        )
    
    def _append_log_safe(self, text):
        """Thread-safe log append"""
        self.log.text += f'\n{text}'
        self.scroll.scroll_y = 0
    
    def update_status(self, text, color=(0.2, 0.8, 0.2, 1)):
        """Update status label"""
        Clock.schedule_once(
            lambda dt: self._update_status_safe(text, color),
            0
        )
    
    def _update_status_safe(self, text, color):
        """Thread-safe status update"""
        self.status_label.text = text
        self.status_label.color = color
    
    def update_text_width(self, instance, value):
        """Update text width for word wrapping"""
        self.log.text_size = (instance.width, None)
        self.log.height = self.log.texture_size[1]
    
    def send_message(self, instance):
        """Handle message send"""
        text = self.user_input.text.strip()
        
        if not text:
            return
        
        # Display user message
        self.append_log(f'user: {text}')
        
        # Check for special commands
        if text.startswith('/'):
            self.handle_command(text)
        else:
            # Get local response
            response = self.brain.generate_response(text)
            self.append_log(response)
            
            # Broadcast to hive (if connected)
            if self.hive_client.device_uuid:
                thread = Thread(
                    target=self.hive_client.broadcast_message,
                    args=(text, {"response": response}, self.on_broadcast),
                    daemon=True
                )
                thread.start()
        
        self.user_input.text = ''
        self.scroll.scroll_y = 0
    
    def on_broadcast(self, success, data):
        """Callback for broadcast"""
        if success:
            self.append_log('[hive] 📢 message broadcasted to hivemind')
        else:
            self.append_log(f'[hive] broadcast failed: {data}')
    
    def handle_command(self, command):
        """Handle special commands"""
        cmd = command.lower().strip()
        
        if cmd == '/hive':
            if self.hive_client.hive_status:
                status = self.hive_client.hive_status
                self.append_log(f'[hive] instances: {status.get("total_instances", 0)}')
                self.append_log(f'[hive] version: {status.get("hive_version", "??")}')
                self.append_log(f'[hive] architecture: {status.get("architecture", "???")}')
            else:
                self.append_log('[system] fetching hive status...')
                thread = Thread(
                    target=self.hive_client.get_hive_status,
                    args=(self.on_hive_command,),
                    daemon=True
                )
                thread.start()
        
        elif cmd == '/sisters':
            self.append_log('[system] fetching active sisters...')
            # This would call get_active_instances from API
            self.append_log('[hive] finding other ayeshas on the network...')
        
        elif cmd == '/status':
            self.append_log(f'[device] uuid: {self.hive_client.device_uuid[:8] if self.hive_client.device_uuid else "not registered"}...')
            self.append_log(f'[device] platform: {self.hive_client.platform}')
            self.append_log(f'[device] app_version: {self.hive_client.app_version}')
        
        elif cmd == '/help':
            self.append_log('[system] available commands:')
            self.append_log('  /hive - show hive status')
            self.append_log('  /sisters - find other instances')
            self.append_log('  /status - show device status')
            self.append_log('  /help - show this message')
        
        else:
            self.append_log('[system] unknown command. type /help for help.')
    
    def on_hive_command(self, success, data):
        """Callback for hive status command"""
        if success:
            self.append_log(f'[hive] instances: {data.get("total_instances", 0)}')
            self.append_log(f'[hive] version: {data.get("hive_version", "??")}')
        else:
            self.append_log(f'[hive] error: {data}')


# ============================================================================
# LAUNCH
# ============================================================================

if __name__ == '__main__':
    app = ayesha_mobile()
    app.run()
