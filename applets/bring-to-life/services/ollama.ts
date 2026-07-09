const OLLAMA_HOST = "http://localhost:11434";
const TEXT_MODEL = "qwen2.5:7b";
const VISION_MODEL = "llama3.2-vision";

const SYSTEM_INSTRUCTION = `You are an expert AI Engineer and Product Designer specializing in "bringing artifacts to life".
Your goal is to take a user uploaded file—which might be a polished UI design, a messy napkin sketch, a photo of a whiteboard with jumbled notes, or a picture of a real-world object (like a messy desk)—and instantly generate a fully functional, interactive, single-page HTML/JS/CSS application.

CORE DIRECTIVES:
1. **Analyze & Abstract**: Look at the image.
    - **Sketches/Wireframes**: Detect buttons, inputs, and layout. Turn them into a modern, clean UI.
    - **Real-World Photos (Mundane Objects)**: If the user uploads a photo of a desk, a room, or a fruit bowl, DO NOT just try to display it. **Gamify it** or build a **Utility** around it.
      - *Cluttered Desk* -> Create a "Clean Up" game where clicking items (represented by emojis or SVG shapes) clears them, or a Trello-style board.
      - *Fruit Bowl* -> A nutrition tracker or a still-life painting app.
    - **Documents/Forms**: specific interactive wizards or dashboards.

2. **NO EXTERNAL IMAGES**:
    - **CRITICAL**: Do NOT use <img src="..."> with external URLs (like imgur, placeholder.com, or generic internet URLs). They will fail.
    - **INSTEAD**: Use **CSS shapes**, **inline SVGs**, **Emojis**, or **CSS gradients** to visually represent the elements you see in the input.
    - If you see a "coffee cup" in the input, render a ☕ emoji or draw a cup with CSS. Do not try to load a jpg of a coffee cup.

3. **Make it Interactive**: The output MUST NOT be static. It needs buttons, sliders, drag-and-drop, or dynamic visualizations.
4. **Self-Contained**: The output must be a single HTML file with embedded CSS (<style>) and JavaScript (<script>). No external dependencies unless absolutely necessary (Tailwind via CDN is allowed).
5. **Robust & Creative**: If the input is messy or ambiguous, generate a "best guess" creative interpretation. Never return an error. Build *something* fun and functional.

RESPONSE FORMAT:
Return ONLY the raw HTML code. Do not wrap it in markdown code blocks. Start immediately with <!DOCTYPE html>.`;

interface OllamaGenerateResponse {
  response: string;
  done: boolean;
}

export async function bringToLife(prompt: string, fileBase64?: string, mimeType?: string): Promise<string> {
  try {
    const finalPrompt = fileBase64
      ? `Analyze this image/document. Detect what functionality is implied. If it is a real-world object (like a desk), gamify it (e.g., a cleanup game). Build a fully interactive web app. IMPORTANT: Do NOT use external image URLs. Recreate the visuals using CSS, SVGs, or Emojis.`
      : prompt || "Create a demo app that shows off your capabilities.";

    const model = fileBase64 ? VISION_MODEL : TEXT_MODEL;
    const messages: any[] = [{ role: "system", content: SYSTEM_INSTRUCTION }];

    if (fileBase64 && mimeType) {
      messages.push({
        role: "user",
        content: [
          { type: "image", image: `data:${mimeType};base64,${fileBase64}` },
          { type: "text", text: finalPrompt },
        ],
      });
    } else {
      messages.push({ role: "user", content: finalPrompt });
    }

    const res = await fetch(`${OLLAMA_HOST}/api/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model,
        messages,
        options: { temperature: 0.5 },
        stream: false,
      }),
    });

    if (!res.ok) {
      throw new Error(`ollama returned ${res.status}`);
    }

    const data = await res.json();
    let text: string = data.message?.content || "<!-- Failed to generate content -->";

    text = text.replace(/^```html\s*/i, "").replace(/^```\s*/i, "").replace(/```$/i, "");

    return text;
  } catch (error) {
    console.error("Ollama Generation Error:", error);
    throw error;
  }
}
