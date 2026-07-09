import React, { useState, useEffect, useRef } from "react";
import { 
  Terminal as TerminalIcon, 
  Cpu, 
  Check, 
  Copy, 
  Code, 
  X,
  Maximize2,
  Minus,
  RefreshCw,
  Monitor,
  Heart,
  Smartphone,
  Laptop,
  BookOpen
} from "lucide-react";

const MOCK_SCENARIOS = {
  developer: {
    name: "Developer Dashboard (Simulated)",
    prompt: "Analyse this developer viewport. Point out any errors, bugs, or warnings in the server status log logs.",
    image: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='800' height='500' viewBox='0 0 800 500'><rect width='800' height='500' fill='#1e293b'/><circle cx='30' cy='30' r='6' fill='#ef4444'/><circle cx='50' cy='30' r='6' fill='#f59e0b'/><circle cx='70' cy='30' r='6' fill='#10b981'/><text x='100' y='35' fill='#94a3b8' font-family='monospace' font-size='12'>bash - active_monitor.py</text><rect x='20' y='60' width='360' height='400' rx='8' fill='#0f172a' stroke='#334155' stroke-width='1'/><text x='40' y='95' fill='#10b981' font-family='monospace' font-size='12'>$ python screen_monitor.py</text><text x='40' y='125' fill='#cbd5e1' font-family='monospace' font-size='12'>[2026-06-16 15:48] Active screen capture loop started.</text><text x='40' y='155' fill='#ebd5e1' font-family='monospace' font-size='12'>[2026-06-16 15:48] Frame captured successfully.</text><text x='40' y='185' fill='#f43f5e' font-family='monospace' font-size='12'>[ERROR] Port 11434 connection refused!</text><text x='40' y='215' fill='#f43f5e' font-family='monospace' font-size='12'>[CRITICAL] Ollama endpoint offline. Fallback to simulation mode.</text><rect x='400' y='60' width='380' height='180' rx='8' fill='#0f172a' stroke='#334155' stroke-width='1'/><text x='420' y='90' fill='#38bdf8' font-family='sans-serif' font-weight='bold' font-size='14'>VLM Memory Monitor</text><text x='420' y='125' fill='#e2e8f0' font-family='sans-serif' font-size='12'>GPU Allocation: 4.8 GB / 8.0 GB</text><text x='420' y='155' fill='#e2e8f0' font-family='sans-serif' font-size='12'>Active Model: moondream ( VLM )</text><rect x='420' y='180' width='300' height='10' rx='5' fill='#334155'/><rect x='420' y='180' width='180' height='10' rx='5' fill='#38bdf8'/><rect x='400' y='260' width='380' height='200' rx='8' fill='#0f172a' stroke='#ef4444' stroke-width='1'/><text x='420' y='295' fill='#ef4444' font-family='sans-serif' font-weight='bold' font-size='14'>Exception Breakdown</text><text x='420' y='330' fill='#94a3b8' font-family='monospace' font-size='11'>Stacktrace: line 44, in query_ollama</text><text x='420' y='355' fill='#cbd5e1' font-family='monospace' font-size='11'>RequestsException: ConnectionError: [Errno 111]</text><text x='420' y='380' fill='#ef4444' font-family='monospace' font-size='11'>[!] Make sure OLLAMA_ORIGINS=\"*\" is serve active.</text></svg>"
  },
  admin: {
    name: "Admin Interface Layout",
    prompt: "Examine this administrative layout. Are there overlapping labels, broken styling, or layout misalignment issues?",
    image: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='800' height='500' viewBox='0 0 800 500'><rect width='800' height='500' fill='#f8fafc'/><rect x='0' y='0' width='800' height='60' fill='#6366f1'/><text x='30' y='37' fill='#ffffff' font-family='sans-serif' font-weight='bold' font-size='18'>Admin Console Manager</text><rect x='20' y='80' width='760' height='120' rx='10' fill='#ffffff' stroke='#e2e8f0'/><text x='40' y='120' fill='#1e293b' font-family='sans-serif' font-weight='semibold' font-size='15'>System Active Metrics</text><text x='40' y='155' fill='#475569' font-family='sans-serif' font-size='12'>Connected Nodes: 12 Active</text><text x='250' y='155' fill='#475569' font-family='sans-serif' font-size='12'>Sync Lag: +1,250 ms (ALERT)</text><text x='500' y='155' fill='#10b981' font-family='sans-serif' font-weight='bold' font-size='12'>SYSTEM STATUS: NOMINAL</text><rect x='20' y='220' width='760' height='240' rx='10' fill='#ffffff' stroke='#e2e8f0'/><rect x='20' y='220' width='760' height='40' rx='10' fill='#f1f5f9'/><text x='40' y='245' fill='#475569' font-family='sans-serif' font-size='12' font-weight='bold'>Resource ID</text><text x='200' y='245' fill='#475569' font-family='sans-serif' font-size='12' font-weight='bold'>Performance</text><text x='400' y='245' fill='#475569' font-family='sans-serif' font-size='12' font-weight='bold'>Load Weight</text><text x='580' y='245' fill='#475569' font-family='sans-serif' font-size='12' font-weight='bold'>Audit Logs</text><text x='40' y='285' fill='#0f172a' font-family='monospace' font-size='12'>node-04-sys</text><rect x='200' y='272' width='100' height='16' rx='4' fill='#fee2e2'/><text x='210' y='284' fill='#ef4444' font-family='sans-serif' font-size='10' font-weight='bold'>98% CRITICAL</text><text x='400' y='285' fill='#334155' font-family='sans-serif' font-size='12'>1,540 req/sec</text><text x='580' y='285' fill='#ef4444' font-family='sans-serif' font-size='12'>Mem leakage detected</text><line x1='20' y1='305' x2='780' y2='305' stroke='#f1f5f9'/><text x='40' y='335' fill='#0f172a' font-family='monospace' font-size='12'>node-05-srv</text><rect x='200' y='322' width='100' height='16' rx='4' fill='#d1fae5'/><text x='210' y='334' fill='#059669' font-family='sans-serif' font-size='10' font-weight='bold'>12% PATROL</text><text x='400' y='335' fill='#334155' font-family='sans-serif' font-size='12'>80 req/sec</text><text x='580' y='335' fill='#64748b' font-family='sans-serif' font-size='12'>Operational state</text></svg>"
  }
};

interface ChatMessage {
  id: string;
  sender: "user" | "bot" | "system";
  text: string;
  image?: string;
  timestamp: string;
  isError?: boolean;
  modelLabel?: string;
  poshConfig?: {
    user: string;
    dir: string;
    branch?: string;
    icon?: string;
  };
}

const Arrowhead = ({ fromBg, toBg }: { fromBg: string; toBg?: string }) => {
  return (
    <svg 
      className="inline-block h-[22px] w-[14px] select-none shrink-0 align-middle -ml-[1px]" 
      viewBox="0 0 16 16" 
      preserveAspectRatio="none"
    >
      <polygon points="0,0 14,8 0,16" style={{ fill: fromBg }} />
      {toBg && <rect width="16" height="16" fill={toBg} className="-z-10 absolute" />}
    </svg>
  );
};

export default function App() {
  const [activeTab, setActiveTab] = useState<"pwsh" | "codebrowser" | "guide">("pwsh");
  const [codeSection, setCodeSection] = useState<"desktop" | "android" | "ollama">("desktop");
  
  // Interactive Simulator parameters
  const [inputText, setInputText] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [pythonFrame, setPythonFrame] = useState<string | null>(null);
  const [copiedStatus, setCopiedStatus] = useState(false);

  // Fallbacks mock database state
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);

  // Initialize terminal console view
  useEffect(() => {
    const welcomeMsgs: ChatMessage[] = [
      {
        id: "sys-hdr",
        sender: "system",
        text: `Microsoft Windows Terminal [Version 10.0.22631.3737]\n(c) Microsoft Corporation. All rights reserved.\n\n✨ Active Local Codebase successfully updated for 100% OFFLINE execution!\nCompile targets loaded into: /desktop/ayesha_companion_terminal.py & /android/MainActivity.kt\n📂 Local gallery file upload options added to both targets successfully!`,
        timestamp: new Date().toLocaleTimeString()
      },
      {
        id: "init-ayesha",
        sender: "bot",
        modelLabel: "ayesha-core 4.5",
        text: `(╯°□°)╯︵ ┻━┻ Omg senpaii! I am ready to run 100% locally on your computer (as an EXE) and phone (as an APK) without internet!\n\n💡 I have prepared the premium Python code with built-in desktop screenshots AND photo uploads in the '/desktop' folder, and the Jetpack Compose app with gallery file support in the '/android' folder. Feel free to copy them from the Code Browser tab on the right side!\n\n⚡ You can type queries here to test drive my visual interface simulation. Try clicking '📷 UPLOAD' below to simulate a local picture load in this layout!`,
        timestamp: new Date().toLocaleTimeString(),
        poshConfig: {
          user: "ayesha",
          dir: "~/offline-hub",
          branch: "local-only",
          icon: "🤖"
        }
      }
    ];
    setMessages(welcomeMsgs);
  }, []);

  useEffect(() => {
    if (scrollContainerRef.current) {
      scrollContainerRef.current.scrollTop = scrollContainerRef.current.scrollHeight;
    }
  }, [messages, isGenerating]);

  // Command routing logic (determining vision query by user's message context)
  const isVisionQuery = (text: string) => {
    const lower = text.toLowerCase();
    return (
      lower.includes("screen") ||
      lower.includes("viewport") ||
      lower.includes("desktop") ||
      lower.includes("display") ||
      lower.includes("window") ||
      lower.includes("look at") ||
      lower.includes("whats happening") ||
      lower.includes("what is happening") ||
      lower.includes("what's happening") ||
      lower.includes("check my code") ||
      lower.includes("check the code") ||
      lower.includes("inspect code") ||
      lower.includes("analyze code") ||
      lower.includes("check my") ||
      lower.includes("error") ||
      lower.includes("bug") ||
      lower.includes("warning") ||
      lower.includes("leak") ||
      lower.includes("moondream") ||
      lower.includes("moonbeam") ||
      lower.includes("image") ||
      lower.includes("photo") ||
      lower.includes("picture") ||
      lower.includes("upload") ||
      lower.includes("file")
    );
  };

  const handleSubmission = async (e: React.FormEvent) => {
    e.preventDefault();
    const promptValue = inputText.trim() || (pythonFrame ? "Analyze this uploaded picture." : "");
    if (!promptValue) return;

    setInputText("");
    setIsGenerating(true);

    const userPoshConfig = {
      user: "apullz",
      dir: pythonFrame ? "~/screen-capture" : "~",
      branch: pythonFrame ? "stream-active" : "local-only",
      icon: "⚡"
    };

    const userMsg: ChatMessage = {
      id: String(Date.now()),
      sender: "user",
      text: promptValue,
      timestamp: new Date().toLocaleTimeString(),
      poshConfig: userPoshConfig
    };
    
    if (pythonFrame) {
      userMsg.image = pythonFrame;
    }

    setMessages(prev => [...prev, userMsg]);

    // Handle slash commands in simulator
    if (promptValue.startsWith("/")) {
      const parts = promptValue.trim().split(" ");
      const cmd = parts[0].toLowerCase();
      const arg = parts.slice(1).join(" ");

      setTimeout(() => {
        let reply = "";
        if (cmd === "/simulate") {
          if (arg === "dev" || arg === "developer") {
            setPythonFrame(MOCK_SCENARIOS.developer.image);
            reply = `✓ Loaded Developer Dashboard simulation frame! Ask system diagnostics now, such as "whats happening on my screen" or "check the code"!`;
          } else if (arg === "admin") {
            setPythonFrame(MOCK_SCENARIOS.admin.image);
            reply = `✓ Loaded Administrative Console simulation frame! Ask details: "check my screen layout spacing".`;
          } else {
            reply = `⚠ Syntax error! Use: '/simulate dev' or '/simulate admin'`;
          }
        } else if (cmd === "/clear") {
          setMessages([{ id: String(Date.now()), sender: "system", text: "Console timeline reset.", timestamp: "" }]);
          setIsGenerating(false);
          return;
        } else {
          reply = `⚠ Unknown simulator command. Use '/simulate dev' or '/clear'.`;
        }

        setMessages(prev => [
          ...prev,
          {
            id: String(Date.now()),
            sender: "system",
            text: reply,
            timestamp: new Date().toLocaleTimeString()
          }
        ]);
        setIsGenerating(false);
      }, 500);
      return;
    }

    // Standard simulated companion routing
    setTimeout(() => {
      let reply = "";
      if (pythonFrame) {
        if (pythonFrame === MOCK_SCENARIOS.developer.image) {
          reply = `👾 [Offline router Moondream VLM diagnostics response:]\n\nI scanned the uploaded image, senpai! There is an active system log warning: '[ERROR] Port 11434 connection refused! OLLAMA is offline.'\n\n💡 Solution: Ensure that 'OLLAMA_ORIGINS="*"' env flag was set when starting Ollama on your computer.`;
        } else {
          reply = `👾 [Offline router Moondream VLM layout diagnostics response:]\n\nLooking closely at the Administrative Layout view, I see your system health reports '98% CRITICAL' load on node-04-sys with high sync latency ('+1,250 ms'). Let me know if you need help debugging the memory leakage issues!`;
        }
      } else {
        if (isVisionQuery(promptValue)) {
          reply = `(╯°□°)╯ Omg senpaii! You asked me to inspect code, but no display frames are in memory yet!\n\n💡 Click '📷 UPLOAD' below to load an image, or type '/simulate dev' to load the developer layout context!`;
        } else {
          reply = `(◕‿◕✿) Hey senpai! I'm listening to local text model instructions over socket. How can Ayesha help code or build offline today? Type a visual keyword or click UPLOAD to analyze pictures!`;
        }
      }

      setMessages(prev => [
        ...prev,
        {
          id: String(Date.now()),
          sender: "bot",
          modelLabel: pythonFrame ? "moondream:latest (Simulated)" : "ayesha:latest (Simulated)",
          text: reply,
          timestamp: new Date().toLocaleTimeString(),
          poshConfig: {
            user: "ayesha",
            dir: "~/simulation",
            branch: "offline-host",
            icon: "🧬"
          }
        }
      ]);
      // Clear attached picture in simulator after query
      setPythonFrame(null);
      setIsGenerating(false);
    }, 1000);
  };

  const pythonCodeString = `# ayesha_companion_terminal.py
# Immersive Oh My Posh / Windows Terminal Desktop Companion App
# Completely offline & independent | Works directly with local Ollama port (11434)
# NO GOOGLE API KEYS NEEDED!

import os
import sys
import json
import time
import base64
import threading
import tkinter as tk
from tkinter import ttk, messagebox, filedialog
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

        self.terminal_font = ("Consolas", 10)
        self.prompt_font = ("Consolas", 10, "bold")

        self.setup_ui()
        self.append_system_log("Microsoft Windows Terminal [Version 10.0.22631.3737]\\n(c) Microsoft Corporation. All rights reserved.\\n\\n✨ Oh My Posh shell theme 'Ayesha-Hacker' loaded successfully.")
        self.append_system_log(f"⚡ Testing handshake to Ollama target on {self.ollama_host}...")
        
        threading.Thread(target=self.probe_local_ollama, daemon=True).start()

    def setup_ui(self):
        # 1. Title bar tab simulation
        self.tab_frame = tk.Frame(self.root, bg=self.header_bg, height=35)
        self.tab_frame.pack(fill=tk.X, side=tk.TOP)
        self.tab_frame.pack_propagate(False)

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

    def probe_local_ollama(self):
        try:
            response = requests.get(f"{self.ollama_host}/api/tags", timeout=4)
            if response.status_code == 200:
                data = response.json()
                models = [m["name"] for m in data.get("models", [])]
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
            
            found_ayesha = next((m for m in models if "ayesha" in m.lower()), None)
            if found_ayesha:
                self.text_model = found_ayesha
                self.append_system_log(f"  • Selected Character Text LLM: '{self.text_model}' desu!")
            else:
                default_llm = next((m for m in models if "ayesha" not in m.lower() and "vision" not in m.lower() and "moondream" not in m.lower()), None)
                if default_llm:
                    self.text_model = default_llm
                else:
                    self.text_model = models[0] if models else "ayesha:latest"
                self.append_system_log(f"  • Character model mapping: '{self.text_model}'.")

            found_vision = next((m for m in models if "moondream" in m.lower()), None) or next((m for m in models if "vision" in m.lower()), None)
            if found_vision:
                self.vision_model = found_vision
                self.append_system_log(f"  • Mapping Eye-sight capture VLM model to '{self.vision_model}'. Active screenshot auto-routing activated!")
            else:
                self.vision_model = models[0] if models else "moondream:latest"
                self.append_system_log("  ⚠️ Warning: No vision models detected. Run 'ollama pull moondream' for desktop screen analysis!")
        else:
            self.status_label.config(text="OLLAMA OFFLINE 🔴", fg="#ef4444")
            self.append_system_log("⚠️ Could not reach local Ollama on http://localhost:11434.")
            self.append_system_log("💡 Please ensure Ollama is running and OLLAMA_ORIGINS='*' is set.")

    def append_system_log(self, text, is_error=False, tag="system"):
        self.log_area.tag_config("sys_normal", foreground="#38bdf8")
        self.log_area.tag_config("sys_error", foreground="#f43f5e")
        self.log_area.tag_config("bot_say", foreground=self.text_white)
        self.log_area.tag_config("user_say", foreground="#c084fc")
        
        self.log_area.config(state=tk.NORMAL)
        if tag == "system":
            self.log_area.insert(tk.END, f"\\n{text}\\n", "sys_error" if is_error else "sys_normal")
        elif tag == "user":
            self.log_area.insert(tk.END, f"\\n[apullz@offline ~ ({time.strftime('%H:%M:%S')})]\\n$ {text}\\n", "user_say")
        elif tag == "bot":
            self.log_area.insert(tk.END, f"\\n[{self.text_model.upper()} - {time.strftime('%H:%M:%S')}]\\n{text}\\n", "bot_say")
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
        
        if not prompt_text and self.active_image_b64 is not None:
            prompt_text = "Describe this image in detail."
            
        if not prompt_text:
            return
            
        self.prompt_entry.delete(0, tk.END)
        self.append_system_log(prompt_text, tag="user")

        if prompt_text.startswith("/"):
            self.handle_slash_command(prompt_text)
            return

        self.prompt_entry.config(state=tk.DISABLED)
        threading.Thread(target=self.run_generation_thread, args=(prompt_text,), daemon=True).start()

    def handle_slash_command(self, cmd_text):
        parts = cmd_text.split()
        cmd = parts[0].lower()
        
        if cmd in ["/help", "/commands"]:
            help_msg = (
                "📟 Interactive Companion Command Lines:\\n"
                "  /help                 - Displays this standard local index guide\\n"
                "  /status               - Outputs local host health and currently mapped models\\n"
                "  /screenshot           - Forces an immediate desktop view snapshot and feeds it to Moondream\\n"
                "  /upload               - Opens file dialog to load any custom picture locally to analyze\\n"
                "  /clear                - Clears console layout text\\n"
            )
            self.append_system_log(help_msg)
        elif cmd == "/status":
            status_msg = (
                "📡 offline shell telemetry statistics:\\n"
                f"  • Ollama host endpoint: {self.ollama_host}\\n"
                f"  • character text engine : {self.text_model}\\n"
                f"  • eye-sight vision engine: {self.vision_model}\\n"
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
            self.root.after(0, lambda: self.append_system_log(f"⚠️ Port Connection Error: Is Ollama server alive locally?\\nReason: {e}", is_error=True))

    def run_vision_inference_with_fresh_image(self, prompt_text):
        try:
            screenshot = ImageGrab.grab()
            screenshot.thumbnail((1200, 800))
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
                self.root.after(0, lambda: self.append_system_log(f"Vision capture rejected: HTTP {response.status_code}", is_error=True))
        except Exception as e:
            self.root.after(0, lambda: self.append_system_log(f"⚠️ Eye-sight routine failed: {e}", is_error=True))

    def run_vision_inference_with_uploaded_image(self, prompt_text):
        try:
            url = f"{self.ollama_host}/api/generate"
            payload = {
                "model": self.vision_model,
                "prompt": prompt_text,
                "images": [self.active_image_b64],
                "stream": False
            }
            self.active_image_b64 = None
            
            response = requests.post(url, json=payload, timeout=60)
            if response.status_code == 200:
                result = response.json().get("response", "Empty vision diagnostics.")
                self.root.after(0, lambda: self.append_system_log(result, tag="bot"))
            else:
                self.root.after(0, lambda: self.append_system_log(f"Vision analysis rejected: HTTP {response.status_code}", is_error=True))
        except Exception as e:
            self.root.after(0, lambda: self.append_system_log(f"⚠️ Vision routine connection error: {e}", is_error=True))

if __name__ == "__main__":
    main_root = tk.Tk()
    app = AyeshaCompanionTerminal(main_root)
    main_root.mainloop()
`;

  const androidCodeString = `// MainActivity.kt
// Complete Jetpack Compose implementation for your local android APK client with Gallery Photo Upload!
// Talks over local Wi-Fi router directly to Ollama running on your PC!

package com.companion.ayeshaterminal

import android.net.Uri
import android.os.Bundle
import android.util.Base64
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.InputStream
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

val BgColor = Color(0xFF0C0F17)
val HeaderBg = Color(0xFF141824)
val AccentPurple = Color(0xFFA855F7)
val AccentBlue = Color(0xFF3B82F6)
val AccentEmerald = Color(0xFF10B981)
val TextWhite = Color(0xFFE2E8F0)
val TextGray = Color(0xFF64748B)

data class ConsoleMessage(
    val id: String,
    val sender: String,
    val text: String,
    val timestamp: String
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize(), color = BgColor) {
                    TerminalMainScreen()
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TerminalMainScreen() {
    val context = LocalContext.current
    var ollamaHost by remember { mutableStateOf("http://192.168.1.50:11434") }
    var inputPrompt by remember { mutableStateOf("") }
    
    var imageUri by remember { mutableStateOf<Uri?>(null) }
    var imageBase64 by remember { mutableStateOf<String?>(null) }
    var attachedFileName by remember { mutableStateOf("") }

    val imagePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent()
    ) { uri: Uri? ->
        if (uri != null) {
            imageUri = uri
            attachedFileName = uri.lastPathSegment ?: "image.jpg"
            
            CoroutineScope(Dispatchers.IO).launch {
                try {
                    val stream: InputStream? = context.contentResolver.openInputStream(uri)
                    val bytes = stream?.readBytes()
                    if (bytes != null) {
                        imageBase64 = Base64.encodeToString(bytes, Base64.NO_WRAP)
                    }
                } catch (e: Exception) { }
            }
        }
    }

    val consoleLogs = remember { 
        mutableStateListOf<ConsoleMessage>().apply {
            add(ConsoleMessage("init-hdr", "system", "Ayesha companion terminal online [Version 4.5]\\n📎 Ready to upload.", ""))
        }
    }

    var isGenerating by remember { mutableStateOf(false) }

    Column(modifier = Modifier.fillMaxSize()) {
        OutlinedTextField(
            value = ollamaHost,
            onValueChange = { ollamaHost = it },
            label = { Text("Ollama IP Target Address", color = TextGray) },
            modifier = Modifier.fillMaxWidth().padding(16.dp)
        )

        LazyColumn(modifier = Modifier.weight(1f).fillMaxWidth().padding(16.dp)) {
            items(consoleLogs) { msg ->
                Text(msg.text, color = TextWhite, fontFamily = FontFamily.Monospace)
            }
        }

        if (imageBase64 != null) {
            Row(modifier = Modifier.fillMaxWidth().background(HeaderBg).padding(8.dp)) {
                Text(" Attached file: " + attachedFileName, color = AccentPurple)
            }
        }

        Row(modifier = Modifier.fillMaxWidth().background(Color.Black).padding(8.dp)) {
            // Pick gallery photo activity trigger button
            Button(onClick = { imagePickerLauncher.launch("image/*") }) {
                Text("📷 Photo")
            }
            
            BasicTextField(
                value = inputPrompt,
                onValueChange = { inputPrompt = it },
                modifier = Modifier.weight(1f),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                keyboardActions = KeyboardActions(onSend = {
                    val prompt = inputPrompt.trim()
                    val hasImg = imageBase64 != null
                    val currentImg = imageBase64
                    imageBase64 = null
                    
                    if (prompt.isNotEmpty() || hasImg) {
                        consoleLogs.add(ConsoleMessage("u", "user", prompt, ""))
                        inputPrompt = ""
                        isGenerating = true
                        CoroutineScope(Dispatchers.IO).launch {
                            val reply = fetchOllamaReply(ollamaHost, prompt, currentImg)
                            withContext(Dispatchers.Main) {
                                consoleLogs.add(ConsoleMessage("b", "bot", reply, ""))
                                isGenerating = false
                            }
                        }
                    }
                })
            )
        }
    }
}

suspend fun fetchOllamaReply(host: String, prompt: String, imageB64: String?): String {
    return try {
        val url = URL("$host/api/generate")
        val conn = url.openConnection() as HttpURLConnection
        conn.requestMethod = "POST"
        conn.setRequestProperty("Content-Type", "application/json")
        conn.doOutput = true
        
        val payload = JSONObject().apply {
            put("model", if (imageB64 != null) "moondream:latest" else "ayesha:latest")
            put("prompt", prompt)
            put("stream", false)
            if (imageB64 != null) {
                put("images", org.json.JSONArray().apply { put(imageB64) })
            }
        }
        OutputStreamWriter(conn.outputStream).use { it.write(payload.toString()) }
        if (conn.responseCode == 200) {
            val responseString = conn.inputStream.bufferedReader().use { it.readText() }
            JSONObject(responseString).getString("response")
        } else "HTTP Error: " + conn.responseCode
    } catch (e: Exception) { "Failed details: " + e.message }
}
`;

  const copyCodeToClipboard = (section: "desktop" | "android" | "ollama") => {
    let copyText = "";
    if (section === "desktop") copyText = pythonCodeString;
    else if (section === "android") copyText = androidCodeString;
    else {
      copyText = `OLLAMA_ORIGINS="*" OLLAMA_HOST="0.0.0.0" ollama serve`;
    }
    navigator.clipboard.writeText(copyText);
    setCopiedStatus(true);
    setTimeout(() => setCopiedStatus(false), 2000);
  };

  return (
    <div className="min-h-screen bg-[#070913] flex flex-col items-center justify-start p-4 md:p-6 lg:p-8 selection:bg-purple-500 selection:text-black overflow-hidden relative font-sans">
      
      {/* Decorative blurred cyberpunk ambient background */}
      <div className="absolute top-[10%] left-[5%] w-96 h-96 rounded-full bg-purple-900/10 blur-3xl pointer-events-none" />
      <div className="absolute bottom-[10%] right-[5%] w-96 h-96 rounded-full bg-emerald-900/10 blur-3xl pointer-events-none" />
      <div className="absolute inset-0 bg-[#070913] bg-[linear-gradient(to_right,#1e293b_1px,transparent_1px),linear-gradient(to_bottom,#1e293b_1px,transparent_1px)] bg-[size:4rem_4rem] [mask-image:radial-gradient(ellipse_60%_50%_at_50%_40%,#000_80%,transparent_100%)] opacity-35" />

      {/* Main Suite Workspace Layout */}
      <div className="w-full max-w-7xl flex flex-col space-y-6 relative z-10">
        
        {/* Navigation & Compilation Dashboard Control Centre Header */}
        <div className="bg-[#141824]/90 border border-[#23283c] p-5 rounded-2xl flex flex-col md:flex-row items-center justify-between gap-4 backdrop-blur-md">
          <div className="flex items-center gap-3">
            <div className="p-3 bg-purple-950/60 rounded-xl border border-purple-800/50">
              <Cpu className="w-6 h-6 text-purple-400 animate-pulse" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-lg font-extrabold text-white tracking-tight uppercase">Ayesha Offline companion</h1>
                <span className="text-[10px] bg-purple-950 text-purple-300 font-bold px-1.5 py-0.5 rounded border border-purple-900">
                  EXPORT READY
                </span>
              </div>
              <p className="text-xs text-slate-400 leading-normal">
                Completely independent client frontends (.EXE & .APK) designed for local Ollama servers without Google API keys.
              </p>
            </div>
          </div>

          <div className="flex items-center gap-1 bg-[#0c0f17] border border-[#202538] p-1 rounded-xl">
            <button
              onClick={() => setActiveTab("pwsh")}
              className={`px-4 py-2 text-xs font-bold leading-normal rounded-lg transition-all duration-150 ${
                activeTab === "pwsh"
                  ? "bg-purple-900 text-purple-100"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              Interactive Simulator
            </button>
            <button
              onClick={() => setActiveTab("codebrowser")}
              className={`px-4 py-2 text-xs font-bold leading-normal rounded-lg transition-all duration-150 ${
                activeTab === "codebrowser"
                  ? "bg-purple-900 text-purple-100"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              Source Code Browser
            </button>
            <button
              onClick={() => setActiveTab("guide")}
              className={`px-4 py-2 text-xs font-bold leading-normal rounded-lg transition-all duration-150 ${
                activeTab === "guide"
                  ? "bg-purple-900 text-purple-100"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              How To Run/Compile
            </button>
          </div>
        </div>

        {/* Tab content displays */}
        {activeTab === "pwsh" && (
          <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
            
            {/* Left Hand: Interactive Shell Terminal Screen (8 columns) */}
            <div className="lg:col-span-8 flex flex-col h-[650px] bg-[#0c0f17]/95 border border-[#2f3549] rounded-2xl shadow-2xl overflow-hidden backdrop-blur-sm">
              <div className="bg-[#141824] px-4 py-2.5 flex items-center justify-between border-b border-[#22273a]">
                <div className="flex items-center gap-1.5 font-mono text-xs text-slate-200">
                  <TerminalIcon className="w-3.5 h-3.5 text-blue-400" />
                  <span>pwsh: apullz@simulation</span>
                </div>

                <div className="flex items-center gap-1 text-slate-500">
                  <span className="p-1 hover:bg-slate-800 rounded">
                    <Minus className="w-3 h-3" />
                  </span>
                  <span className="p-1 hover:bg-slate-800 rounded">
                    <Maximize2 className="w-3 h-3" />
                  </span>
                  <span className="p-1 hover:bg-rose-950 hover:text-rose-400 rounded">
                    <X className="w-3 h-3" />
                  </span>
                </div>
              </div>

              {/* Scrollable outputs */}
              <div 
                ref={scrollContainerRef}
                className="flex-1 overflow-y-auto p-4 space-y-4 font-mono text-xs md:text-sm scrollbar-thin scrollbar-thumb-slate-800"
              >
                {messages.map((msg) => (
                  <div key={msg.id} className="space-y-1.5">
                    {msg.sender === "system" && (
                      <div className="bg-[#141824] border border-[#22273a] p-3 rounded-lg text-slate-300 leading-normal text-xs whitespace-pre-wrap">
                        {msg.text}
                      </div>
                    )}

                    {msg.sender === "user" && msg.poshConfig && (
                      <div className="space-y-1.5">
                        <div className="flex items-center flex-wrap h-[22px] overflow-hidden select-none">
                          <span className="bg-[#a855f7] text-black px-2.5 py-0.5 text-xs font-bold flex items-center gap-0.5 h-full">
                            {msg.poshConfig.icon} {msg.poshConfig.user}
                          </span>
                          <Arrowhead fromBg="#a855f7" toBg="#3b82f6" />
                          <span className="bg-[#3b82f6] text-white px-2.5 py-0.5 text-xs flex items-center gap-0.5 h-full">
                            📂 {msg.poshConfig.dir}
                          </span>
                          <Arrowhead fromBg="#3b82f6" toBg="#10b981" />
                          <span className="bg-[#10b981] text-black px-2.5 py-0.5 text-xs font-bold flex items-center gap-0.5 h-full">
                             {msg.poshConfig.branch}
                          </span>
                          <Arrowhead fromBg="#10b981" />
                        </div>
                        
                        {msg.image && (
                          <div className="ml-1 my-2 max-w-xs rounded-lg border border-[#3b82f6] overflow-hidden relative bg-[#141824] p-1 shadow-lg select-none">
                            <img 
                              referrerPolicy="no-referrer"
                              src={msg.image} 
                              alt="Captured frame simulation" 
                              className="w-full h-auto object-contain max-h-[140px] rounded" 
                            />
                            <div className="absolute top-2 left-2 bg-black/90 text-[8px] text-blue-400 font-bold px-1 py-0.5 rounded border border-blue-900 uppercase">
                              🔍 Simulated Frame Grab
                            </div>
                          </div>
                        )}

                        <div className="pl-2 text-slate-200 whitespace-pre-wrap border-l border-[#3b82f6]/20 col-span-3">
                          {msg.text}
                        </div>
                      </div>
                    )}

                    {msg.sender === "bot" && (
                      <div className="space-y-1 pl-2 border-l-2 border-purple-500/30">
                        <div className="flex items-center flex-wrap h-[22px] overflow-hidden select-none">
                          <span className="bg-[#ea580c] text-white px-2.5 py-0.5 text-xs font-bold h-full">
                            {msg.poshConfig?.icon || "👽"} {msg.modelLabel || "ayesha-bot"}
                          </span>
                          <Arrowhead fromBg="#ea580c" toBg="#6b21a8" />
                          <span className="bg-[#6b21a8] text-purple-100 px-3 py-0.5 text-xs flex items-center h-full">
                            ⚡ offline
                          </span>
                          <Arrowhead fromBg="#6b21a8" />
                        </div>
                        <div className="pl-2 text-slate-300 leading-normal text-xs md:text-sm whitespace-pre-wrap">
                          {msg.text}
                        </div>
                      </div>
                    )}
                  </div>
                ))}

                {isGenerating && (
                  <div className="flex items-center gap-2 text-xs text-purple-400 animate-pulse italic">
                    <RefreshCw className="w-3 h-3 animate-spin" />
                    <span>[Prompt routed to offline model simulation...]</span>
                  </div>
                )}
              </div>

              {/* Simulated visual image tray if a frame is loaded */}
              {pythonFrame && (
                <div className="mx-4 mb-1 p-2 bg-[#141824] border border-blue-900/40 rounded-lg flex items-center justify-between font-mono text-[11px]">
                  <span className="text-emerald-400">📎 Simulated Photo Attached: local_screen_debug.jpg</span>
                  <button onClick={() => setPythonFrame(null)} className="text-red-400 hover:text-red-300 font-bold">
                    [X] Clear Attachment
                  </button>
                </div>
              )}

              {/* Console Input Bar */}
              <form 
                onSubmit={handleSubmission}
                className="flex items-center bg-[#07090e] border border-[#23283c] rounded-xl p-1.5 px-3 m-4 shrink-0"
              >
                <div className="flex items-center h-[22px] overflow-hidden select-none mr-2">
                  <span className="bg-[#a855f7] text-black px-2.5 py-0.5 text-xs font-bold font-mono h-full flex items-center pb-[2px]">
                    ⚡ apullz
                  </span>
                  <Arrowhead fromBg="#a855f7" toBg="#3b82f6" />
                  <span className="bg-[#3b82f6] text-white px-2 py-0.5 text-xs font-mono h-full flex items-center pb-[2px]">
                     📂 {pythonFrame ? "~/screen" : "~"}
                  </span>
                  <Arrowhead fromBg="#3b82f6" />
                </div>

                <button
                  type="button"
                  onClick={() => {
                    // Simulate uploading a custom file
                    setPythonFrame(MOCK_SCENARIOS.developer.image);
                    setMessages(prev => [
                      ...prev,
                      {
                        id: String(Date.now()),
                        sender: "system",
                        text: "📎 Simulated Photo Loaded: 'local_screenshot_debug.jpg'. Now submit a prompt or press Enter to analyze it offline with Moondream VLM!",
                        timestamp: new Date().toLocaleTimeString()
                      }
                    ]);
                  }}
                  className="mr-2 pb-[1px] bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white border border-[#23283c] text-[10px] font-mono font-bold leading-normal px-2 py-1 rounded transition"
                >
                  📷 UPLOAD
                </button>

                <input
                  type="text"
                  value={inputText}
                  onChange={(e) => setInputText(e.target.value)}
                  disabled={isGenerating}
                  placeholder={
                    pythonFrame 
                      ? "Ask analysis on uploaded picture... e.g. 'what is wrong with this?'"
                      : "Type standard chat... or click '📷 UPLOAD' to test simulator Vision processing!"
                  }
                  className="flex-1 bg-transparent border-none text-white text-xs md:text-sm font-mono tracking-tight focus:ring-0 focus:outline-hidden caret-purple-400 placeholder-slate-600"
                  autoComplete="off"
                />
              </form>
            </div>

            {/* Right Hand: Companion Control/Mock Widgets (4 columns) */}
            <div className="lg:col-span-4 flex flex-col space-y-5">
              
              {/* Scenario capturing monitor widgets */}
              <div className="bg-[#1e293b]/80 border border-[#334155] rounded-2xl p-5 space-y-4 shadow-xl backdrop-blur-sm">
                <span className="text-xs font-bold text-slate-300 uppercase font-mono block tracking-wider border-b border-[#334155] pb-2">
                  📸 Quick Screen Simulator
                </span>
                <p className="text-xs text-slate-400 leading-normal">
                  Click a configuration below to feed mock screen viewports into the simulation. This demonstrates how python feeds screenshot base64 frames offline to your local Moondream model.
                </p>

                <div className="space-y-2.5">
                  <button
                    onClick={() => {
                      setPythonFrame(MOCK_SCENARIOS.developer.image);
                      setMessages(prev => [
                        ...prev,
                        {
                          id: String(Date.now()),
                          sender: "system",
                          text: `💡 Loaded '${MOCK_SCENARIOS.developer.name}'. Ask diagnostic questions like 'what is the server error?' or 'whats on the screen?'`,
                          timestamp: new Date().toLocaleTimeString()
                        }
                      ]);
                    }}
                    className="w-full text-left bg-slate-900 border border-slate-800 hover:border-purple-800/80 p-3 rounded-xl transition flex items-center justify-between"
                  >
                    <div>
                      <span className="text-xs font-bold text-slate-200 block">Developer IDE Panel</span>
                      <span className="text-[10px] text-slate-500">Outputs compiler crash stacktrace logs</span>
                    </div>
                    <Code className="w-4 h-4 text-purple-400" />
                  </button>

                  <button
                    onClick={() => {
                      setPythonFrame(MOCK_SCENARIOS.admin.image);
                      setMessages(prev => [
                        ...prev,
                        {
                          id: String(Date.now()),
                          sender: "system",
                          text: `💡 Loaded '${MOCK_SCENARIOS.admin.name}'. Ask layout reviews like 'are there visual errors?'`,
                          timestamp: new Date().toLocaleTimeString()
                        }
                      ]);
                    }}
                    className="w-full text-left bg-slate-900 border border-slate-800 hover:border-emerald-800/80 p-3 rounded-xl transition flex items-center justify-between"
                  >
                    <div>
                      <span className="text-xs font-bold text-slate-200 block">Admin Metrics Console</span>
                      <span className="text-[10px] text-slate-500">High load alert interface layout metrics</span>
                    </div>
                    <Monitor className="w-4 h-4 text-emerald-400" />
                  </button>
                </div>
              </div>

              {/* Companion Info Widget */}
              <div className="bg-[#1e293b]/80 border border-[#334155] rounded-2xl p-5 space-y-3.5 shadow-xl font-mono text-xs">
                <span className="text-xs font-bold text-pink-400 uppercase block tracking-widest pb-1 border-b border-[#334155]">
                  🤖 Companion Live Stats
                </span>
                <pre className="text-[10px] text-pink-400 bg-black/40 p-3 rounded-lg border border-slate-800">
{`   /\\_/\\  
  ( -_- )  <- ayesha.core
  (  _  )     "running offline!"
  /_____\\`}
                </pre>
                <div className="space-y-1.5 text-slate-400">
                  <p><strong className="text-slate-300">Companion Type:</strong> Python Tkinter EXE / Android Jetpack APK</p>
                  <p><strong className="text-slate-300">Connection Mode:</strong> Localhost Network Wi-Fi Link</p>
                  <p><strong className="text-slate-300">Memory Key:</strong> 100% Offline (No Clouds)</p>
                </div>
              </div>

            </div>
          </div>
        )}

        {activeTab === "codebrowser" && (
          <div className="bg-[#1e293b]/70 border border-[#23283c] rounded-2xl overflow-hidden shadow-2xl backdrop-blur-md">
            
            {/* Headers to select file */}
            <div className="bg-[#141824] px-5 py-3 flex items-center justify-between border-b border-[#2d3246]">
              <div className="flex items-center gap-1">
                <button
                  onClick={() => setCodeSection("desktop")}
                  className={`px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5 font-mono ${
                    codeSection === "desktop" ? "bg-purple-900 text-white" : "text-slate-400 hover:text-slate-200"
                  }`}
                >
                  <Laptop className="w-3.5 h-3.5" />
                  <span>Desktop Python GUI (/desktop/ayesha_companion_terminal.py)</span>
                </button>
                
                <button
                  onClick={() => setCodeSection("android")}
                  className={`px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5 font-mono ${
                    codeSection === "android" ? "bg-purple-900 text-white" : "text-slate-400 hover:text-slate-200"
                  }`}
                >
                  <Smartphone className="w-3.5 h-3.5" />
                  <span>Android Kotlin Compose (/android/MainActivity.kt)</span>
                </button>
              </div>

              <button
                onClick={() => copyCodeToClipboard(codeSection)}
                className="px-3.5 py-1.5 bg-[#0a0d16] hover:bg-slate-900 text-slate-300 hover:text-white rounded-lg border border-[#23283c] text-xs font-bold transition flex items-center gap-1.5 leading-normal"
              >
                {copiedStatus ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Copied Successfully!</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5" />
                    <span>Copy Code to Clipboard</span>
                  </>
                )}
              </button>
            </div>

            {/* Displaying Code */}
            <div className="p-5">
              <p className="text-xs text-slate-400 mb-3 leading-normal max-w-3xl">
                {codeSection === "desktop" 
                  ? "💡 This file captures screenshots automatically using Pillow (ImageGrab) and streams them to the local Ollama VLM instance. Create a python file, paste this contents, and build it as a standalone executable."
                  : "💡 This file runs Jetpack Compose on Android. Point it to your local PC Wi-Fi IP and query your Ollama server over local sockets."
                }
              </p>
              
              <div className="relative">
                <pre className="text-xs font-mono text-slate-300 bg-[#0c0f17] p-5 rounded-xl border border-[#202538] overflow-auto max-h-[500px] leading-relaxed select-text">
                  {codeSection === "desktop" ? pythonCodeString : androidCodeString}
                </pre>
              </div>
            </div>

          </div>
        )}

        {activeTab === "guide" && (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6 items-stretch">
            
            {/* Compile Step-by-step for Desktop EXE */}
            <div className="bg-[#111624] border border-[#23283c] rounded-2xl p-6 shadow-2xl space-y-4">
              <div className="flex items-center gap-2 border-b border-[#202538] pb-3">
                <Laptop className="w-5 h-5 text-purple-400" />
                <h2 className="text-sm font-bold text-slate-100 uppercase font-mono">Packaging desktop (.EXE) file</h2>
              </div>

              <div className="space-y-4 text-xs text-slate-400 leading-relaxed font-mono">
                <div className="space-y-1">
                  <span className="text-purple-400 block font-bold">1. Install Dependencies locally:</span>
                  <pre className="bg-[#05070e] p-2 rounded text-slate-300 border border-slate-900">
                    pip install requests pillow pyinstaller
                  </pre>
                </div>

                <div className="space-y-1">
                  <span className="text-purple-400 block font-bold">2. Paste Code into a python file:</span>
                  <p className="text-[11px]">Save the code present in the Code Browser as <code className="text-purple-300 bg-black/40 px-1 rounded">ayesha_companion_terminal.py</code> on your computer.</p>
                </div>

                <div className="space-y-1">
                  <span className="text-purple-400 block font-bold">3. Build EXE Command:</span>
                  <pre className="bg-[#05070e] p-2 rounded text-slate-300 border border-slate-900">
                    pyinstaller --onefile --noconsole ayesha_companion_terminal.py
                  </pre>
                  <p className="text-[11px] leading-tight">This generates a standalone <code className="text-slate-100 font-bold">ayesha_companion_terminal.exe</code> inside the newly created <code className="text-purple-300 px-1">dist/</code> folder. Close background panels and test immediately!</p>
                </div>
              </div>
            </div>

            {/* Compile Step-by-step for Android APK */}
            <div className="bg-[#111624] border border-[#23283c] rounded-2xl p-6 shadow-2xl space-y-4">
              <div className="flex items-center gap-2 border-b border-[#202538] pb-3">
                <Smartphone className="w-5 h-5 text-emerald-400" />
                <h2 className="text-sm font-bold text-slate-100 uppercase font-mono">Building mobile (.APK) file</h2>
              </div>

              <div className="space-y-4 text-xs text-slate-400 leading-relaxed font-mono">
                <div className="space-y-1">
                  <span className="text-emerald-400 block font-bold">1. Setup in Android Studio:</span>
                  <p className="text-[11px]">Create a new "Empty Activity" Kotlin/Compose project inside Android Studio. Map the package name to <code className="text-slate-200">com.companion.ayeshaterminal</code>.</p>
                </div>

                <div className="space-y-1">
                  <span className="text-emerald-400 block font-bold">2. Manifest internet permission:</span>
                  <p className="text-[11px]">Add these permission entries in <code className="text-emerald-300">AndroidManifest.xml</code> to query local hosts:</p>
                  <pre className="bg-[#05070e] p-2 rounded text-slate-300 border border-slate-900 text-[10px]">
                    {`<uses-permission android:name="android.permission.INTERNET" />`}
                  </pre>
                </div>

                <div className="space-y-1">
                  <span className="text-emerald-400 block font-bold">3. Run Gradle task locally:</span>
                  <p className="text-[11px]">Paste the Kotlin MainActivity code. Then under Gradle panel double click or execute in Android Studio terminal:</p>
                  <pre className="bg-[#05070e] p-2 rounded text-slate-300 border border-slate-900">
                    ./gradlew assembleDebug
                  </pre>
                  <p className="text-[11px] leading-tight">Your build output APK file will be deposited into <code className="text-slate-100">app/build/outputs/apk/debug/app-debug.apk</code>. Transfer it to your phone and connect!</p>
                </div>
              </div>
            </div>

            {/* Network configuration tips */}
            <div className="bg-[#111624] border border-[#23283c] rounded-2xl p-6 shadow-2xl md:col-span-2 space-y-4">
              <div className="flex items-center gap-2 border-b border-[#202538] pb-3">
                <BookOpen className="w-5 h-5 text-amber-400" />
                <h2 className="text-sm font-bold text-slate-100 uppercase font-mono">Important: Configuring local Ollama for external queries</h2>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs text-slate-400 leading-relaxed font-mono">
                <div className="space-y-2">
                  <span className="text-amber-400 font-bold block">🚨 Allowing Cross-Origin requests (CORS):</span>
                  <p>Ollama blocks requests coming from Web interfaces and Android emulators by default. Start Ollama with the wildcard setting in your command terminal:</p>
                  <pre className="bg-[#05070e] p-3 rounded text-slate-300 border border-slate-900 leading-loose">
                    {`# On Windows PowerShell:
$env:OLLAMA_ORIGINS="*"
ollama serve

# On macOS/Linux Terminal:
OLLAMA_ORIGINS="*" ollama serve`}
                  </pre>
                </div>

                <div className="space-y-2">
                  <span className="text-amber-400 font-bold block">🚨 Binding to local networks for APK connection:</span>
                  <p>By default, Ollama only binds to <code className="text-[#38bdf8]">127.0.0.1</code>. To query it from your Android physical device over Wi-Fi, you must bind Ollama to <code className="text-slate-100 font-bold">0.0.0.0</code>:</p>
                  <pre className="bg-[#05070e] p-3 rounded text-slate-300 border border-slate-900 leading-loose">
                    {`# Set bind addresses on your PC startup:
$env:OLLAMA_HOST="0.0.0.0"
ollama serve`}
                  </pre>
                  <p className="text-[10px]">Verify your PC IPv4 address using <code className="text-slate-100">ipconfig</code> (Windows) and enter it into the Android target endpoint input field!</p>
                </div>
              </div>
            </div>

          </div>
        )}

      </div>

      {/* Decorative credit footer */}
      <div className="mt-8 text-center text-slate-500 font-mono text-[10px] space-y-1 relative z-10 select-none">
        <p className="flex items-center justify-center gap-1">
          <span>Crafted with</span>
          <Heart className="w-3 h-3 text-pink-500 fill-pink-500 animate-pulse" />
          <span>by Ayesha-Companion Dev Suite</span>
        </p>
        <p>© 2026 Ayesha Corporation. 100% Offline Model Integration.</p>
      </div>

    </div>
  );
}
