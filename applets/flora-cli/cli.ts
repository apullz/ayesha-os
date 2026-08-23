import readline from "readline";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { floraData } from "./src/data/floraData.js";
import { PlantNode } from "./src/types.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Conversation history for multi-turn ask
const chatHistory: Array<{ role: string; content: string }> = [];

// Basic manual .env loader to run standalone with zero external dependencies
try {
  const envPath = path.join(__dirname, ".env");
  if (fs.existsSync(envPath)) {
    const dotenvContent = fs.readFileSync(envPath, "utf-8");
    dotenvContent.split(/\r?\n/).forEach((line) => {
      const trimmed = line.trim();
      if (trimmed && !trimmed.startsWith("#")) {
        const parts = trimmed.split("=");
        if (parts.length >= 2) {
          const key = parts[0].trim();
          const value = parts.slice(1).join("=").trim().replace(/^["']|["']$/g, "");
          process.env[key] = value;
        }
      }
    });
  }
} catch (e) {
  // Silence env load errors
}

// ── theme (driven by ayesha.json → theme.palette, truecolor ANSI) ───────
const _defaultPal: Record<string, string> = {
  primary: "#E75E9D", accent: "#D782A7", secondary: "#4C5D79",
  text: "#E8E6F0", dim: "#6A6478", success: "#62C884", error: "#E5536A",
};
const _palette: Record<string, string> = { ..._defaultPal };
try {
  const cfg = JSON.parse(fs.readFileSync(path.join(__dirname, "..", "..", "ayesha.json"), "utf-8"));
  const pal = cfg?.theme?.palette || {};
  for (const k of Object.keys(_defaultPal)) {
    if (typeof pal[k] === "string") _palette[k] = pal[k];
  }
} catch (e) {
  // keep default palette
}
const _rgbStr = (hex: string) => {
  const h = hex.replace("#", "");
  return `${parseInt(h.slice(0, 2), 16)};${parseInt(h.slice(2, 4), 16)};${parseInt(h.slice(4, 6), 16)}`;
};
const _fg = (hex: string, bold = false) => `\x1b[${bold ? "1;" : ""}38;2;${_rgbStr(hex)}m`;
const RST = "\x1b[0m";
// role codes: Y=primary(was yellow), C=accent(was cyan), G=success(green),
// R=error(red), M=secondary(magenta/blue), D=dim, W=text(white)
const Y = _fg(_palette.primary), YB = _fg(_palette.primary, true);
const C = _fg(_palette.accent), CB = _fg(_palette.accent, true);
const G = _fg(_palette.success), GB = _fg(_palette.success, true);
const R = _fg(_palette.error);
const M = _fg(_palette.secondary), MB = _fg(_palette.secondary, true);
const D = _fg(_palette.dim);
const W = _fg(_palette.text), WB = _fg(_palette.text, true);

// Utility to get a node from a path array
export function getNodeFromPath(root: PlantNode, pathSegments: string[]): PlantNode | null {
  let current = root;
  for (const segment of pathSegments) {
    if (!current.children || !current.children[segment]) {
      return null;
    }
    current = current.children[segment];
  }
  return current;
}

// Helper to check if a path exists and return its segments
function resolvePath(root: PlantNode, currentSegments: string[], targetPath: string): string[] | null {
  const cleanPath = targetPath.trim();
  if (!cleanPath) return []; // Home directory

  let segments = [...currentSegments];
  if (cleanPath.startsWith("/")) {
    segments = []; // start from root
  }

  const parts = cleanPath.split("/").filter(p => p && p !== ".");
  for (const part of parts) {
    if (part === "..") {
      if (segments.length > 0) {
        segments.pop();
      }
    } else {
      // Find case-insensitive match among children
      const currentNode = getNodeFromPath(root, segments);
      if (!currentNode || !currentNode.children) return null;

      const keys = Object.keys(currentNode.children);
      const match = keys.find(k => k.toLowerCase() === part.toLowerCase());
      if (!match) return null;

      segments.push(match);
    }
  }

  // Double check final node exists
  if (getNodeFromPath(root, segments)) {
    return segments;
  }
  return null;
}

// Recursive helper to build ASCII tree output
function buildAsciiTree(node: PlantNode, indent: string = "", isLast: boolean = true, maxDepth: number = 99, depth: number = 0): string {
  let result = "";
  if (node.rank !== "clade" || node.name !== "Plantae") {
    const marker = isLast ? "└── " : "├── ";
    result += `${indent}${marker}${Y}${node.name}${RST} ${C}(${node.rank})${RST}${node.commonName ? ` - ${node.commonName}` : ""}\n`;
  } else {
    result += `${G}${node.name} (${node.commonName})${RST}\n`;
  }

  if (node.children && depth < maxDepth) {
    const keys = Object.keys(node.children);
    const subIndent = indent + (isLast ? "    " : "│   ");
    keys.forEach((key, index) => {
      const child = node.children![key];
      result += buildAsciiTree(child, subIndent, index === keys.length - 1, maxDepth, depth + 1);
    });
  } else if (node.children && depth >= maxDepth) {
    const childCount = Object.keys(node.children).length;
    const marker = isLast ? "└── " : "├── ";
    result += `${indent}${marker}${D}... (${childCount} more children, use tree --depth ${maxDepth + 1} to expand)${RST}\n`;
  }
  return result;
}

// Global path state
let pathSegments: string[] = [];

// Handle a single command line input
async function handleCommand(inputLine: string): Promise<{ output: string; shouldExit?: boolean }> {
  const trimmedInput = inputLine.trim();
  if (!trimmedInput) return { output: "" };

  const tokens = trimmedInput.split(/\s+/);
  const command = tokens[0].toLowerCase();
  const args = tokens.slice(1);

  const currentNode = getNodeFromPath(floraData, pathSegments);
  if (!currentNode) {
    pathSegments = [];
    return { output: R + "Error: Current path is invalid. Reset to root." + RST };
  }

  switch (command) {
    case "exit":
    case "quit":
      return { output: G + "Fair thee well, Scholar. May the wind be always at your back." + RST, shouldExit: true };

    case "clear":
      // Standard ANSI code to clear screen and reset cursor
      return { output: "\x1b[2J\x1b[H" };

    case "help":
      return {
        output: `
${YB}Available Commands:${RST}
  ${Y}ls${RST}                 List taxonomic divisions, families, or species in the current folder
  ${Y}cd [taxon]${RST}          Change active taxonomic directory (e.g. ${C}cd angiosperms${RST}, ${C}cd ..${RST})
  ${Y}pwd${RST}                Print current absolute taxonomic path
  ${Y}cat [species.md]${RST}    Inspect detailed botanical report & folklore of a species
  ${Y}tree [--depth=N]${RST}    Render ASCII taxonomic branching diagram (default depth: 3)
  ${Y}evolution${RST}          Draw vertical geologic timeline and milestones of current lineage
  ${Y}search [query]${RST}     Search full database for any plant, family, or Gaelic term
  ${Y}ask [question]${RST}      Query the Caledonian Botanist AI on folklore, uses, or biology
  ${Y}clear${RST}              Clear terminal screen
  ${Y}history${RST}            Show conversation history with the Sage
  ${Y}exit${RST} / ${Y}quit${RST}          Exit the application

${CB}Taxonomic Ranks of Earth:${RST}
  ${M}clade${RST}     -> Deep evolutionary branches (e.g. Bryophytes, Gymnosperms)
  ${M}class${RST}     -> Major botanical classes (e.g. Bryophyta, Conifers)
  ${M}family${RST}    -> Related plant groupings (ending in -aceae, e.g. Ericaceae)
  ${M}genus${RST}     -> General plant genus (e.g. Calluna, Pinus)
  ${M}species${RST}   -> Individual plant files (e.g. vulgaris, sylvestris)
`
      };

    case "pwd":
      return { output: `/${pathSegments.join("/")}` };

    case "history": {
      if (chatHistory.length === 0) {
        return { output: D + "No conversation history yet." + RST };
      }
      let out = YB + "Conversation History:" + RST + "\n";
      for (const msg of chatHistory) {
        const role = msg.role === "user" ? C + "You" + RST : Y + "Sage" + RST;
        out += `  ${role}: ${msg.content.slice(0, 120)}${msg.content.length > 120 ? "..." : ""}\n`;
      }
      out += D + `(${chatHistory.length} messages, use 'ask /reset' to clear)` + RST;
      return { output: out };
    }

    case "ls": {
      if (!currentNode.children || Object.keys(currentNode.children).length === 0) {
        return { output: `This is a terminal species file. Type ${Y}cat ${currentNode.name.split(" ")[1] || currentNode.name}.md${RST} to read details, or ${Y}cd ..${RST} to go up.` };
      }

      const keys = Object.keys(currentNode.children);
      let listOutput = WB + "Taxon elements in current clade:" + RST + "\n\n";
      keys.forEach(key => {
        const child = currentNode.children![key];
        if (child.rank === "species") {
          listOutput += `  ${G}📄 ${key}.md${RST}   (${child.commonName || "Native Species"})\n`;
        } else {
          const rankColor = child.rank === "clade" ? C : child.rank === "class" ? M : child.rank === "family" ? M : Y;
          listOutput += `  ${rankColor}📁 ${key}/${RST}   [${child.rank}] - ${child.commonName || ""}\n`;
        }
      });
      return { output: listOutput };
    }

    case "cd": {
      const targetDir = args[0] || "";
      const resolved = resolvePath(floraData, pathSegments, targetDir);
      if (resolved === null) {
        return { output: R + `cd: no such taxonomic folder: ${targetDir}` + RST };
      } else {
        pathSegments = resolved;
        return { output: "" };
      }
    }

    case "cat": {
      let filename = args[0] || "";
      if (!filename) {
        return { output: R + "cat: missing species file argument. Example: cat vulgaris.md" + RST };
      }
      if (filename.endsWith(".md")) {
        filename = filename.slice(0, -3);
      }

      // Check if filename matches a child species (case-insensitive)
      let foundSpecies: PlantNode | undefined;
      if (currentNode.children) {
        const childKey = Object.keys(currentNode.children).find(
          k => k.toLowerCase() === filename.toLowerCase()
        );
        if (childKey) foundSpecies = currentNode.children[childKey];
      }
      if (!foundSpecies && currentNode.rank === "species" && currentNode.name.toLowerCase().endsWith(filename.toLowerCase())) {
        foundSpecies = currentNode;
      }

      if (foundSpecies && foundSpecies.rank === "species") {
        return {
          output: `
${YB}${foundSpecies.name.toUpperCase()}${RST}
${C}Common Name:${RST}   ${foundSpecies.commonName || "Unknown"}
${C}Gaelic Name:${RST}   ${foundSpecies.gaelicName || "None recorded"}
${C}Conservation:${RST}  ${foundSpecies.status || "Unspecified"}
${C}Origin Era:${RST}    ${foundSpecies.geologicalEra || "Prehistoric"}
${C}Evolutionary:${RST}   ${foundSpecies.evolutionaryMilestone || ""}

${GB}=== BOTANICAL DESCRIPTION ===${RST}
${foundSpecies.description}

${GB}=== HIGHLAND HABITAT ===${RST}
${foundSpecies.habitat || "Widespread"}

${GB}=== TRADITIONAL LORE & FOLKLORE ===${RST}
${foundSpecies.lore || "None"}

${YB}=== ASCII REPRESENTATION ===${RST}
${foundSpecies.asciiArt || ""}
`
        };
      } else {
        return { output: R + `cat: file not found or is a folder: ${args[0]}. (Tip: Use 'ls' to find .md species files)` + RST };
      }
    }

    case "tree": {
      const depthFlag = args.find(a => a.startsWith("--depth=") || a === "--depth");
      let maxDepth = 3;
      if (depthFlag) {
        const val = depthFlag.includes("=") ? depthFlag.split("=")[1] : args[args.indexOf(depthFlag) + 1];
        maxDepth = Math.max(1, parseInt(val) || 3);
      }
      return { output: GB + `Phylogeny Tree starting from ${currentNode.name} (depth ${maxDepth}):` + RST + "\n\n" + buildAsciiTree(currentNode, "", true, maxDepth) };
    }

    case "evolution":
    case "lineage": {
      let timeline = YB + "=== EVOLUTIONARY DEEP HISTORY OF CURRENT TAXON ===" + RST + "\n\n";
      let tempSegments: string[] = [];
      let lineageNodes: PlantNode[] = [floraData];

      for (const segment of pathSegments) {
        tempSegments.push(segment);
        const node = getNodeFromPath(floraData, tempSegments);
        if (node) lineageNodes.push(node);
      }

      lineageNodes.forEach((node, idx) => {
        const isCurrent = idx === lineageNodes.length - 1;
        const arrow = isCurrent ? " ● " + GB + "[ACTIVE]" + RST + " " : " │   ";
        timeline += `${CB}${node.geologicalEra || "Deep Time"}${RST}\n`;
        timeline += `${arrow}${Y}${node.name}${RST} (${node.rank})\n`;
        if (node.evolutionaryMilestone) {
          timeline += ` │   ${W}→ Landmark: ${node.evolutionaryMilestone}${RST}\n`;
        }
        if (!isCurrent) {
          timeline += ` │\n`;
        }
      });
      return { output: timeline };
    }

    case "search":
    case "locate": {
      const query = args.join(" ").trim().toLowerCase();
      if (!query) {
        return { output: R + "search: missing search term. Example: search heather" + RST };
      }

      let results: { name: string; rank: string; path: string; gaelic?: string; common?: string }[] = [];

      function recursiveSearch(node: PlantNode, currentPathArr: string[]) {
        const pathStr = "/" + currentPathArr.join("/");
        const isMatch =
          node.name.toLowerCase().includes(query) ||
          (node.commonName && node.commonName.toLowerCase().includes(query)) ||
          (node.gaelicName && node.gaelicName.toLowerCase().includes(query)) ||
          node.description.toLowerCase().includes(query) ||
          (node.lore && node.lore.toLowerCase().includes(query));

        if (isMatch) {
          results.push({
            name: node.name,
            rank: node.rank,
            path: pathStr,
            common: node.commonName,
            gaelic: node.gaelicName
          });
        }

        if (node.children) {
          Object.keys(node.children).forEach(key => {
            recursiveSearch(node.children![key], [...currentPathArr, key]);
          });
        }
      }

      recursiveSearch(floraData, []);

      if (results.length === 0) {
        return { output: `No taxonomic matches found for: ${R}"${query}"${RST}` };
      } else {
        let searchOutput = `Found ${GB}${results.length}${RST} phylogenetic branches or species:\n\n`;
        results.forEach(r => {
          searchOutput += `  ${YB}${r.name}${RST} [${r.rank}] ${r.common ? `(${r.common})` : ""}\n`;
          if (r.gaelic) searchOutput += `    Gaelic: ${r.gaelic}\n`;
          searchOutput += `    Path:   ${CB}cd ${r.path}${RST}\n\n`;
        });
        return { output: searchOutput };
      }
    }

    case "ask": {
      const question = args.join(" ").trim();
      if (!question) {
        return { output: R + "ask: What would you like to ask? Use 'ask /reset' to clear history." + RST };
      }

      if (question === "/reset") {
        chatHistory.length = 0;
        return { output: Y + "Conversation history cleared." + RST };
      }

      console.log(Y + "The Caledonian Botanist Sage is cogitating..." + RST);

      try {
        const system = `You are the legendary Caledonian Botanist AI, a wise and friendly Scottish naturalist, phytologist, and clan historian.
You are helping the user explore the magnificent evolutionary tree of Scottish Flora inside a terminal application.
Your tone should be knowledgeable, warm, and highly engaging—reminiscent of Scottish naturalists like John Muir.
Feel free to drop in traditional Scottish Gaelic terms, botanical lore, historical uses, and geological lineage, but keep it concise and highly readable for a terminal environment.

Context for current conversation:
- Current Directory Path in terminal: /${pathSegments.join("/")}
${currentNode.rank === "species" ? `- Active Species being inspected: ${currentNode.name}` : "- The user is currently in a taxonomic folder and has not targeted a specific species yet."}

Terminal Formatting Instructions:
- Keep responses compact (approx. 2 to 4 paragraphs, maximum 250 words) to fit the terminal screen.
- DO NOT use markdown heading tags (like #, ##, ###) because the terminal renders plain-text.
- Use dashes, capitals, or simple asterisks for lists or subtitles.
- If they ask general questions unrelated to Scottish botany, gently guide them back to the lore of the glens, ancient peatlands, Caledonian pine forests, and the deep evolution of plants.`;

        const messages = [
          { role: "system", content: system },
          ...chatHistory.slice(-20),
          { role: "user", content: question },
        ];

        let kiloHost = process.env.KILO_HOST || "https://api.kilo.ai";
        if (!kiloHost.startsWith("http://") && !kiloHost.startsWith("https://")) {
          kiloHost = "http://" + kiloHost;
        }
        kiloHost = kiloHost.replace(/\/+$/, "");
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 60000);
        const res = await fetch(`${kiloHost}/v1/chat/completions`, {
          method: "POST",
          headers: { "Content-Type": "application/json", "Authorization": `Bearer ${process.env.KILO_API_KEY}` },
          body: JSON.stringify({
            model: "kilo-auto/free",
            messages,
            temperature: 0.7,
            stream: false,
          }),
          signal: controller.signal,
        });
        clearTimeout(timeout);

        if (!res.ok) throw new Error(`kilo returned ${res.status}`);
        const data = await res.json();
        const text = data.choices?.[0]?.message?.content || data.message?.content || "The sage was silent.";

        chatHistory.push({ role: "user", content: question });
        chatHistory.push({ role: "assistant", content: text });
        if (chatHistory.length > 50) chatHistory.splice(0, chatHistory.length - 50);

        return { output: `\n${YB}THE CALEDONIAN BOTANIST SAGE COGITATES:${RST}\n\n${text}\n` };
      } catch (err: any) {
        return { output: R + `Error connecting to Caledonian Botanist AI: ${err?.message || err}` + RST };
      }
    }

    default:
      return { output: R + `bash: command not found: ${command}. (Type 'help' to see list of valid commands)` + RST };
  }
}

// Shell entrypoint
async function runShell() {
  // Check if arguments were passed directly to execute a single command
  const directArgs = process.argv.slice(2);
  if (directArgs.length > 0) {
    const rawCmd = directArgs.join(" ");
    const { output } = await handleCommand(rawCmd);
    console.log(output);
    process.exit(0);
  }

  // Clear screen and print Caledonian Botanical Terminal welcome banner
  console.log("\x1b[2J\x1b[H");
  console.log(YB + "=================================================================" + RST);
  console.log(GB + "          🌲  CALEDONIAN PHYLOGENETIC TERMINAL v1.2  🌲          " + RST);
  console.log(YB + "=================================================================" + RST);
  console.log("Welcome, Scholar. Traverse the deep branches of Scottish Flora.");
  console.log("Type " + C + "help" + RST + " to list commands, " + C + "ls" + RST + " to view clades, or " + C + "exit" + RST + " to quit.");
  console.log("Inquire of the " + YB + "Caledonian Botanist AI" + RST + " using: " + Y + "ask [question]" + RST);
  console.log(YB + "-----------------------------------------------------------------" + RST + "\n");

  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  const prompt = () => {
    const currentPathStr = "/" + pathSegments.join("/");
    rl.question(GB + "guest@caledonia" + RST + ":" + MB + currentPathStr + RST + "$ ", async (line) => {
      const { output, shouldExit } = await handleCommand(line);
      if (output) {
        console.log(output);
      }
      if (shouldExit) {
        rl.close();
        process.exit(0);
      }
      prompt();
    });
  };

  prompt();
}

runShell().catch((err) => {
  console.error("Shell error:", err);
  process.exit(1);
});
