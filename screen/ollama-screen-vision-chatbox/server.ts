import express from "express";
import path from "path";
import dotenv from "dotenv";
import { createServer as createViteServer } from "vite";

dotenv.config();

const app = express();
const PORT = 3000;
const OLLAMA_HOST = process.env.OLLAMA_HOST || "http://localhost:11434";
const VISION_MODEL = "llama3.2-vision";
const TEXT_MODEL = "qwen2.5:7b";

app.use(express.json({ limit: "50mb" }));
app.use(express.urlencoded({ limit: "50mb", extended: true }));

let latestScreenFrame: string | null = null;
let latestScreenTimestamp: string | null = null;

app.post("/api/screen/update", (req, res) => {
  const { image, base64 } = req.body;
  const framePayload = image || base64;
  if (!framePayload) return res.status(400).json({ error: "Missing image/base64 payload data" });
  latestScreenFrame = framePayload;
  latestScreenTimestamp = new Date().toLocaleTimeString();
  res.json({ success: true, timestamp: latestScreenTimestamp, message: "Screen frame updated successfully." });
});

app.get("/api/screen/latest", (req, res) => {
  res.json({ image: latestScreenFrame, timestamp: latestScreenTimestamp || "No capture received yet" });
});

app.get("/api/health", (req, res) => {
  res.json({ status: "ok", time: new Date().toISOString() });
});

async function ollamaChat(model: string, messages: any[], options?: any): Promise<string> {
  const res = await fetch(`${OLLAMA_HOST}/api/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ model, messages, options: options || {}, stream: false }),
  });
  if (!res.ok) throw new Error(`ollama returned ${res.status}`);
  const data = await res.json();
  return data.message?.content || "";
}

app.post("/api/vision/analyze", async (req, res) => {
  try {
    const { image, prompt, taskMode } = req.body;
    if (!image) return res.status(400).json({ error: "Missing screen image payload" });

    let systemInstructions = "";
    switch (taskMode) {
      case "ocr":
        systemInstructions = "You are a professional OCR engine. Extract all readable text, structured data, headers, and code snippets from the provided screen image. Maintain structural context where possible.";
        break;
      case "ui-review":
        systemInstructions = "You are an expert UI/UX auditor. Evaluate this screen capture for architectural balance, accessibility (contrast, labels), alignment, clutter, and text spacing. Give 3-5 constructive recommendations.";
        break;
      case "detector":
        systemInstructions = "You are a screen element locator. Detect important interactable components such as buttons, links, inputs, and tabs. Respond in structured JSON.";
        break;
      case "bug-report":
        systemInstructions = "You are a QA automation engineer verifying screens. Spot visual bugs, rendering glitches, missing resources, truncated texts, overlapped items, or error messages and return a clean checklist ticket.";
        break;
      default:
        systemInstructions = "Analyze the provided viewport/crop and supply direct, informative feedback to the user's specific query.";
    }

    const fullPrompt = `${systemInstructions}\n\nUser Question: ${prompt || "Explain what is on this screen in detail."}`;
    const imageData = image.includes(";base64,") ? image : `data:image/png;base64,${image}`;

    const result = await ollamaChat(VISION_MODEL, [
      { role: "user", content: [
        { type: "image", image: imageData },
        { type: "text", text: fullPrompt },
      ]},
    ], { temperature: 0.3 });

    res.json({ success: true, result, model: VISION_MODEL });
  } catch (error: any) {
    console.error("Vision analysis error:", error);
    res.status(500).json({ error: error.message || "Failed to analyze screen capture." });
  }
});

app.post("/api/chat", async (req, res) => {
  try {
    const { prompt } = req.body;

    const systemPrompt = `You are ayesha-bot 4.0, a highly energetic, helpful, and affectionate cyberpunk girl hacking companion. You run on the user's local Ollama setup. You speak in cute terminal-hacker lingo, code snippets, Japanese honorifics/emotes and energetic interjections like "Kapoo!". Always write helpful, tech-accurate, friendly, and quirky replies to your senpai.`;

    const result = await ollamaChat(TEXT_MODEL, [
      { role: "system", content: systemPrompt },
      { role: "user", content: prompt || "say hello!" },
    ], { temperature: 0.7 });

    res.json({ success: true, result });
  } catch (error: any) {
    console.error("Chat error:", error);
    res.status(500).json({ error: error.message || "Failed to get response from Ayesha." });
  }
});

// Ollama Bridge/Proxy (handles forwarding local/remote public requests to bypass web CORS constraints)
const ALLOWED_OLLAMA_HOSTS = [
  "http://localhost:11434",
  "http://127.0.0.1:11434",
  process.env.OLLAMA_HOST,
].filter(Boolean);

app.post("/api/ollama/proxy", async (req, res) => {
  const { path: ollamaPath, method, body, host } = req.body;
  
  if (!host) {
    return res.status(400).json({ error: "No Ollama host endpoint provided." });
  }

  // SSRF protection: only allow configured Ollama hosts
  const normalizedHost = host.replace(/\/$/, "");
  const isAllowed = ALLOWED_OLLAMA_HOSTS.some(
    (allowed) => normalizedHost === allowed.replace(/\/$/, "")
  );
  if (!isAllowed) {
    return res.status(403).json({
      error: `Host not allowed. Permitted hosts: ${ALLOWED_OLLAMA_HOSTS.join(", ")}`
    });
  }

  const endpoint = `${normalizedHost}/${ollamaPath.replace(/^\//, "")}`;

  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 60000); // 60s timeout for complex generations

    const response = await fetch(endpoint, {
      method: method || "GET",
      headers: {
        "Content-Type": "application/json"
      },
      body: method && method !== "GET" ? JSON.stringify(body) : undefined,
      signal: controller.signal
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      const errorText = await response.text();
      return res.status(response.status).json({
        error: `Ollama host returned error: ${response.statusText}`,
        details: errorText
      });
    }

    // Stream-friendly responses: checking if response has stream or text
    const contentType = response.headers.get("content-type");
    if (contentType && contentType.includes("application/json")) {
      const json = await response.json();
      return res.json(json);
    } else {
      const text = await response.text();
      try {
        // Fallback parsings
        const json = JSON.parse(text);
        return res.json(json);
      } catch {
        return res.json({ response: text });
      }
    }
  } catch (error: any) {
    console.error(`Ollama Proxy error requesting ${endpoint}:`, error);
    return res.status(502).json({
      error: "Could not reach Ollama model service endpoint.",
      details: error.message || String(error)
    });
  }
});

// Vite Integration & Static Assets serving
async function startServer() {
  if (process.env.NODE_ENV !== "production") {
    // Development mode
    const vite = await createViteServer({
      server: { middlewareMode: true },
      appType: "spa",
    });
    app.use(vite.middlewares);
  } else {
    // Production mode
    const distPath = path.join(process.cwd(), "dist");
    app.use(express.static(distPath));
    app.get("*", (req, res) => {
      res.sendFile(path.join(distPath, "index.html"));
    });
  }

  app.listen(PORT, "127.0.0.1", () => {
    console.log(`Server listening at http://127.0.0.1:${PORT}`);
  });
}

startServer();
