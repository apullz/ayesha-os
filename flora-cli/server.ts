import express from "express";
import path from "path";
import { createServer as createViteServer } from "vite";
import { GoogleGenAI } from "@google/genai";
import dotenv from "dotenv";

dotenv.config();

async function startServer() {
  const app = express();
  const PORT = 3000;

  app.use(express.json());

  // API route for the Scottish Botanist AI
  app.post("/api/botanist", async (req, res) => {
    try {
      const { prompt, speciesContext, pathContext } = req.body;

      const apiKey = process.env.GEMINI_API_KEY;
      if (!apiKey || apiKey === "MY_GEMINI_API_KEY" || apiKey.trim() === "") {
        return res.status(400).json({
          error: "Caledonian Botanist AI is currently offline. To enable it, please add your GEMINI_API_KEY in the Settings > Secrets panel of AI Studio.",
          offline: true
        });
      }

      // Lazy-initialization to avoid server crashes if key is missing on startup
      const ai = new GoogleGenAI({
        apiKey: apiKey,
        httpOptions: {
          headers: {
            'User-Agent': 'aistudio-build',
          }
        }
      });

      const systemInstruction = `You are the legendary Caledonian Botanist AI, a wise and friendly Scottish naturalist, phytologist, and clan historian.
You are helping the user explore the magnificent evolutionary tree of Scottish Flora inside a simulated terminal.
Your tone should be knowledgeable, warm, and highly engaging—reminiscent of Scottish naturalists like John Muir.
Feel free to drop in traditional Scottish Gaelic terms, botanical lore, historical uses, and geological lineage, but keep it concise and highly readable for a terminal environment.

Context for current conversation:
- Current Directory Path in terminal: ${pathContext || "/"}
${speciesContext ? `- Active Species being inspected: ${speciesContext}` : "- The user is currently in a taxonomic folder and has not targeted a specific species yet."}

Terminal Formatting Instructions:
- Keep responses compact (approx. 2 to 4 paragraphs, maximum 250 words) to fit the terminal screen.
- DO NOT use markdown heading tags (like #, ##, ###) because the terminal renders plain-text with simplified styles.
- Use dashes, capitals, or simple asterisks for lists or subtitles.
- If they ask general questions unrelated to Scottish botany, gently guide them back to the lore of the glens, ancient peatlands, Caledonian pine forests, and the deep evolution of plants.`;

      const response = await ai.models.generateContent({
        model: "gemini-3.5-flash",
        contents: prompt,
        config: {
          systemInstruction: systemInstruction,
          temperature: 0.7,
        }
      });

      res.json({ text: response.text });
    } catch (err: any) {
      console.error("Gemini Botanist Error:", err);
      res.status(500).json({ error: err?.message || "The Highland Sage had trouble connecting. Please try again." });
    }
  });

  // Serve static assets or mount Vite in development
  if (process.env.NODE_ENV !== "production") {
    const vite = await createViteServer({
      server: { middlewareMode: true },
      appType: "spa",
    });
    app.use(vite.middlewares);
  } else {
    const distPath = path.join(process.cwd(), 'dist');
    app.use(express.static(distPath));
    app.get('*', (req: any, res: any) => {
      res.sendFile(path.join(distPath, 'index.html'));
    });
  }

  app.listen(PORT, "127.0.0.1", () => {
    console.log(`Caledonian Flora Server running on port ${PORT}`);
  });
}

startServer();
