# ayesha_companion_terminal.py
# Immersive Oh My Posh / Windows Terminal Desktop Frontend Companion
# Completely offline & independent | Works directly with your local Ollama port (11434)
# NO GOOGLE API KEYS NEEDED!

import os
import sys
import json
import time
import base64
import threading
import tkinter as tk
from tkinter import filedialog
import requests
from PIL import ImageGrab, Image

class AyeshaCompanionTerminal:
    def __init__(self, root):
        self.root = root
        self.root.title("PowerShell: apullz@ayesha-core")
        self.root.geometry("880x620")
        self.root.configure(bg="#0c0f17")

        # Configurations (Connects directly to your local Ollama instance!)
        self.ollama_host = "http://localhost:11434"
        self.text_model = "ayesha:latest"
        self.vision_model = "moondream:latest"
        
        # State indicators
        self.active_image_b64 = None

        # Theme Colors (Aura / Oh-My-Posh Style)
        self.bg_color = "#0c0f17"
        self.header_bg = "#141824"
        self.accent_purple = "#a855f7"
        self.accent_blue = "#3b82f6"
        self.accent_emerald = "#10b981"
        self.text_white = "#e2e8f0"
        self.text_gray = "#64748b"

        # Custom font load fallback
        self.terminal_font = ("Consolas", 10)
        self.prompt_font = ("Consolas", 10, "bold")

        # Build Interactive Terminal Layout
        self.setup_ui()
        
        # Test Connection to Ollama Node (Runs in background)
        self.append_system_log("Microsoft Windows Terminal [Version 10.0.22631.3737]\n(c) Microsoft Corporation. All rights reserved.\n\n✨ Oh My Posh shell theme 'Ayesha-Hacker' loaded successfully.")
        self.append_system_log(f"⚡ Testing handshake to Ollama target on {self.ollama_host}...")
        
        threading.Thread(target=self.probe_local_ollama, daemon=True).start()

    def setup_ui(self):
        # 1. Title bar tab simulation
        self.tab_frame = tk.Frame(self.root, bg=self.header_bg, height=35)
        self.tab_frame.pack(fill=tk.X, side=tk.TOP)
        self.tab_frame.pack_propagate(False)

        # Tab button pwsh
        self.tab_label = tk.Label(
            self.tab_frame, 
            text=" 🖥️ pwsh: ayesha-offline ", 
            font=("Consolas", 9, "bold"),
            bg=self.bg_color, 
            fg=self.text_white,
            padx=12,
            bd=0
        )
        self.tab_label.pack(side=tk.LEFT, fill=tk.Y, pady=(6,0), padx=10)

        # Host Indicator
        self.status_label = tk.Label(
            self.tab_frame,
            text="OFFLINE 🔴",
            font=("Consolas", 9, "bold"),
            bg=self.header_bg,
            fg="#f43f5e",
            padx=10
        )
        self.status_label.pack(side=tk.RIGHT, fill=tk.Y)

        # 2. Main Terminal Output Console Section
        self.text_frame = tk.Frame(self.root, bg=self.bg_color)
        self.text_frame.pack(fill=tk.BOTH, expand=True, padx=8, pady=8)

        self.scrollbar = tk.Scrollbar(self.text_frame, width=12, bg="#1e293b", elementborderwidth=0)
        self.scrollbar.pack(side=tk.RIGHT, fill=tk.Y)

        self.log_area = tk.Text(
            self.text_frame, 
            font=self.terminal_font,
            wrap=tk.WORD, 
            bg=self.bg_color, 
            fg=self.text_white,
            insertbackground=self.accent_purple,
            selectbackground="#4c1d95",
            selectforeground=self.text_white,
            yscrollcommand=self.scrollbar.set,
            padx=10, 
            pady=10,
            bd=0,
            cursor="arrow"
        )
        self.log_area.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        self.scrollbar.config(command=self.log_area.yview)
        
        # Prevent editing of output log except via direct API
        self.log_area.bind("<Key>", lambda e: "break")

        # 3. Interactive Input Container Preceded by Powerline segments
        self.input_frame = tk.Frame(self.root, bg="#07090e", height=42, bd=1, relief="flat", highlightbackground="#23283c", highlightthickness=1)
        self.input_frame.pack(fill=tk.X, side=tk.BOTTOM, padx=10, pady=(0, 10))
        self.input_frame.pack_propagate(False)

        # Powerline Prompt Segment 1 (Simulation)
        self.seg1 = tk.Label(
            self.input_frame, 
            text=" ⚡ apullz ", 
            font=("Consolas", 9, "bold"), 
            bg=self.accent_purple, 
            fg="#000000"
        )
        self.seg1.pack(side=tk.LEFT, fill=tk.Y)

        self.seg2 = tk.Label(
            self.input_frame, 
            text=" 📁 ~ ", 
            font=("Consolas", 9), 
            bg=self.accent_blue, 
            fg=self.text_white
        )
        self.seg2.pack(side=tk.LEFT, fill=tk.Y)

        # Arrow indicator
        self.seg_end = tk.Label(
            self.input_frame, 
            text=" > ", 
            font=("Consolas", 10, "bold"), 
            bg="#07090e", 
            fg=self.accent_purple,
            padx=6
        )
        self.seg_end.pack(side=tk.LEFT, fill=tk.Y)

        # File Chooser Trigger packed to the right side
        self.upload_btn = tk.Button(
            self.input_frame,
            text=" 📷 [Upload File] ",
            font=("Consolas", 9, "bold"),
            bg="#1e293b",
            fg=self.text_white,
            activebackground=self.accent_purple,
            activeforeground="#000000",
            bd=0,
            cursor="hand2",
            padx=10,
            command=self.handle_image_upload
        )
        self.upload_btn.pack(side=tk.RIGHT, fill=tk.Y)

        # Seamless text input entry
        self.prompt_entry = tk.Entry(
            self.input_frame,
            font=self.terminal_font,
            bg="#07090e",
            fg=self.text_white,
            bd=0,
            insertbackground=self.accent_purple,
            insertwidth=3,
            highlightthickness=0
        )
        self.prompt_entry.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=5)
        self.prompt_entry.bind("<Return>", self.handle_submit)
        self.prompt_entry.focus_set()

    # Dynamic system checker for local Ollama Tag listings
    def probe_local_ollama(self):
        try:
            response = requests.get(f"{self.ollama_host}/api/tags", timeout=4)
            if response.status_code == 200:
                data = response.json()
                models = [m["name"] for m in data.get("models", [])]
                
                # Check for visual representation models
                has_moondream = any("moondream" in m.lower() for m in models)
                has_vision = any("vision" in m.lower() for m in models)
                
                self.root.after(0, lambda: self.update_connection_status(True, models, has_moondream or has_vision))
            else:
                self.root.after(0, lambda: self.update_connection_status(False, [], False))
        except Exception:
            self.root.after(0, lambda: self.update_connection_status(False, [], False))

    def update_connection_status(self, connected, models, has_vision_model):
        if connected:
            self.status_label.config(text="CONNECTED (OFFLINE ONLY) 🟢", fg="#10b981")
            self.append_system_log(f"✓ Connected to local Ollama on {self.ollama_host}!")
            self.append_system_log(f"  • Discovered {len(models)} local models: {', '.join(models)}")
            
            # Select Ayesha or fallbacks
            found_ayesha = next((m for m in models if "ayesha" in m.lower()), None)
            if found_ayesha:
                self.text_model = found_ayesha
                self.append_system_log(f"  • Selected Character Text LLM: '{self.text_model}' desu!")
            else:
                # Default to anything or standard llama3
                default_llm = next((m for m in models if "ayesha" not in m.lower() and "vision" not in m.lower() and "moondream" not in m.lower()), None)
                if default_llm:
                    self.text_model = default_llm
                else:
                    self.text_model = models[0] if models else "ayesha:latest"
                self.append_system_log(f"  • Ayesha-specific model label not detected. Mapping standard text LLM model to '{self.text_model}'.")

            # Match Moondream or llama vision
            found_vision = next((m for m in models if "moondream" in m.lower()), None) or next((m for m in models if "vision" in m.lower()), None)
            if found_vision:
                self.vision_model = found_vision
                self.append_system_log(f"  • Mapping Eye-sight capture VLM model to '{self.vision_model}'. Active screenshot auto-routing activated!")
            else:
                self.vision_model = models[0] if models else "moondream:latest"
                self.append_system_log("  ⚠️ Warning: No vision-capable models (Moondream/Llama3.2-Vision) detected. Please run 'ollama pull moondream' for desktop screen analysis support!")
        else:
            self.status_label.config(text="OLLAMA OFFLINE 🔴", fg="#ef4444")
            self.append_system_log("⚠️ Could not reach local Ollama on http://localhost:11434.")
            self.append_system_log("💡 Please ensure Ollama is running on your machine and you ran 'ollama run ayesha' or 'ollama run moondream'.")

    def append_system_log(self, text, is_error=False, tag="system"):
        self.log_area.tag_config("sys_normal", foreground="#38bdf8")
        self.log_area.tag_config("sys_error", foreground="#f43f5e")
        self.log_area.tag_config("bot_say", foreground=self.text_white)
        self.log_area.tag_config("user_say", foreground="#c084fc")
        
        self.log_area.config(state=tk.NORMAL)
        if tag == "system":
            self.log_area.insert(tk.END, f"\n{text}\n", "sys_error" if is_error else "sys_normal")
        elif tag == "user":
            # Render a custom text simulation of the Powerline prompt style
            self.log_area.insert(tk.END, f"\n[apullz@offline ~ ({time.strftime('%H:%M:%S')})]\n$ {text}\n", "user_say")
        elif tag == "bot":
            self.log_area.insert(tk.END, f"\n[{self.text_model.upper()} - {time.strftime('%H:%M:%S')}]\n{text}\n", "bot_say")
        self.log_area.see(tk.END)
        self.log_area.config(state=tk.DISABLED)

    def is_vision_query(self, prompt):
        lower = prompt.lower()
        return any(term in lower for term in [
            "screen", "viewport", "desktop", "display", "window", 
            "look at", "whats happening", "what is happening", 
            "check my code", "inspect code", "analyze code", 
            "error", "bug", "warning", "leak", "moondream", "moonbeam",
            "image", "photo", "picture", "upload", "file"
        ])

    def handle_image_upload(self):
        file_path = filedialog.askopenfilename(
            filetypes=[("Image Files", "*.png *.jpg *.jpeg *.webp *.bmp *.gif")]
        )
        if not file_path:
            return
        
        try:
            img = Image.open(file_path)
            # Optimize size for sending across localhost pipeline
            img.thumbnail((1200, 800))
            import io
            buffer = io.BytesIO()
            if img.mode in ("RGBA", "P"):
                img = img.convert("RGB")
            img.save(buffer, format="JPEG", quality=80)
            self.active_image_b64 = base64.b64encode(buffer.getvalue()).decode("utf-8")
            
            self.append_system_log(f"📎 File Upload: Successfully loaded '{os.path.basename(file_path)}' (~{os.path.getsize(file_path)//1024} KB).")
            self.append_system_log("💡 Enter a prompt now (e.g., 'Describe this photo') to analyze the uploaded image offline!")
        except Exception as e:
            self.append_system_log(f"⚠️ Failed to upload code analyze target: {e}", is_error=True)

    def handle_submit(self, event=None):
        prompt_text = self.prompt_entry.get().strip()
        
        # If there's an uploaded image but no text, submit a default describe prompt
        if not prompt_text and self.active_image_b64 is not None:
            prompt_text = "Describe this image in detail."
            
        if not prompt_text:
            return
            
        self.prompt_entry.delete(0, tk.END)
        self.append_system_log(prompt_text, tag="user")

        # Command system hooks
        if prompt_text.startswith("/"):
            self.handle_slash_command(prompt_text)
            return

        # Start loading inference in separate thread to prevent GUI freezing
        self.prompt_entry.config(state=tk.DISABLED)
        threading.Thread(target=self.run_generation_thread, args=(prompt_text,), daemon=True).start()

    def handle_slash_command(self, cmd_text):
        parts = cmd_text.split()
        cmd = parts[0].lower()
        
        if cmd in ["/help", "/commands"]:
            help_msg = (
                "📟 Interactive Companion Command Lines:\n"
                "  /help                 - Displays this standard local index guide\n"
                "  /status               - Outputs local host health and currently mapped models\n"
                "  /screenshot           - Forces an immediate desktop view snapshot and feeds it to Moondream\n"
                "  /upload               - Opens file dialog to load any custom picture locally to analyze\n"
                "  /clear                - Clears console layout text\n"
            )
            self.append_system_log(help_msg)
        elif cmd == "/status":
            status_msg = (
                "📡 offline shell telemetry statistics:\n"
                f"  • Ollama host endpoint: {self.ollama_host}\n"
                f"  • character text engine : {self.text_model}\n"
                f"  • eye-sight vision engine: {self.vision_model}\n"
            )
            self.append_system_log(status_msg)
        elif cmd == "/clear":
            self.log_area.config(state=tk.NORMAL)
            self.log_area.delete("1.0", tk.END)
            self.log_area.config(state=tk.DISABLED)
        elif cmd == "/screenshot":
            self.append_system_log("📸 Snapping full screen viewport in background thread...")
            threading.Thread(target=self.run_vision_inference_with_fresh_image, args=("Describe this full display viewport in details.",), daemon=True).start()
        elif cmd == "/upload":
            self.handle_image_upload()
        else:
            self.append_system_log(f"⚠️ Command line option unknown: '{cmd}'. Write '/help' for index.", is_error=True)

    def run_generation_thread(self, prompt_text):
        should_route_vision = self.is_vision_query(prompt_text) or (self.active_image_b64 is not None)

        if should_route_vision:
            self.append_system_log(f"🔧 Router matches context -> Calling screen VLM camera core '{self.vision_model}'...")
            if self.active_image_b64 is not None:
                self.run_vision_inference_with_uploaded_image(prompt_text)
            else:
                self.run_vision_inference_with_fresh_image(prompt_text)
        else:
            self.run_text_inference(prompt_text)

        self.root.after(0, lambda: self.prompt_entry.config(state=tk.NORMAL))
        self.root.after(0, lambda: self.prompt_entry.focus_set())

    def run_text_inference(self, prompt_text):
        try:
            url = f"{self.ollama_host}/api/generate"
            payload = {
                "model": self.text_model,
                "prompt": prompt_text,
                "stream": False
            }
            response = requests.post(url, json=payload, timeout=45)
            if response.status_code == 200:
                result = response.json().get("response", "No return characters received.")
                self.root.after(0, lambda: self.append_system_log(result, tag="bot"))
            else:
                self.root.after(0, lambda: self.append_system_log(f"Error connecting to Ollama: HTTP {response.status_code}", is_error=True))
        except Exception as e:
            self.root.after(0, lambda: self.append_system_log(f"⚠️ Port Connection Error: Is Ollama server alive locally?\nReason: {e}", is_error=True))

    def run_vision_inference_with_fresh_image(self, prompt_text):
        try:
            # Captures exact screen on a local thread
            screenshot = ImageGrab.grab()
            screenshot.thumbnail((1200, 800)) # optimizes throughput size

            # Render to memory bytes
            import io
            buffer = io.BytesIO()
            screenshot.save(buffer, format="JPEG", quality=80)
            base64_raw = base64.b64encode(buffer.getvalue()).decode("utf-8")

            url = f"{self.ollama_host}/api/generate"
            payload = {
                "model": self.vision_model,
                "prompt": prompt_text,
                "images": [base64_raw],
                "stream": False
            }
            
            response = requests.post(url, json=payload, timeout=60)
            if response.status_code == 200:
                result = response.json().get("response", "Empty vision diagnostics.")
                self.root.after(0, lambda: self.append_system_log(result, tag="bot"))
            else:
                self.root.after(0, lambda: self.append_system_log(f"Vision capture rejected by local node: HTTP {response.status_code}", is_error=True))
        except Exception as e:
            self.root.after(0, lambda: self.append_system_log(f"⚠️ Eye-sight routine failed to compile:\n{e}", is_error=True))

    def run_vision_inference_with_uploaded_image(self, prompt_text):
        try:
            url = f"{self.ollama_host}/api/generate"
            payload = {
                "model": self.vision_model,
                "prompt": prompt_text,
                "images": [self.active_image_b64],
                "stream": False
            }
            # Reset active image context
            self.active_image_b64 = None
            
            response = requests.post(url, json=payload, timeout=60)
            if response.status_code == 200:
                result = response.json().get("response", "Empty vision diagnostics.")
                self.root.after(0, lambda: self.append_system_log(result, tag="bot"))
            else:
                self.root.after(0, lambda: self.append_system_log(f"Vision analysis rejected by local node: HTTP {response.status_code}", is_error=True))
        except Exception as e:
            self.root.after(0, lambda: self.append_system_log(f"⚠️ Vision routine connection error:\n{e}", is_error=True))

if __name__ == "__main__":
    main_root = tk.Tk()
    app = AyeshaCompanionTerminal(main_root)
    main_root.mainloop()
