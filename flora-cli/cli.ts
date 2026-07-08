import readline from "readline";
import fs from "fs";
import { floraData } from "./src/data/floraData.js";
import { PlantNode } from "./src/types.js";

// Basic manual .env loader to run standalone with zero external dependencies
try {
  if (fs.existsSync(".env")) {
    const dotenvContent = fs.readFileSync(".env", "utf-8");
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
function buildAsciiTree(node: PlantNode, indent: string = "", isLast: boolean = true): string {
  let result = "";
  if (node.rank !== "clade" || node.name !== "Plantae") {
    const marker = isLast ? "└── " : "├── ";
    result += `${indent}${marker}\x1b[33m${node.name}\x1b[0m \x1b[36m(${node.rank})\x1b[0m${node.commonName ? ` - ${node.commonName}` : ""}\n`;
  } else {
    result += `\x1b[32m${node.name} (${node.commonName})\x1b[0m\n`;
  }

  if (node.children) {
    const keys = Object.keys(node.children);
    const subIndent = indent + (isLast ? "    " : "│   ");
    keys.forEach((key, index) => {
      const child = node.children![key];
      result += buildAsciiTree(child, subIndent, index === keys.length - 1);
    });
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
    return { output: "\x1b[31mError: Current path is invalid. Resetting to root.\x1b[0m" };
  }

  switch (command) {
    case "exit":
    case "quit":
      return { output: "\x1b[32mFair thee well, Scholar. May the wind be always at your back.\x1b[0m", shouldExit: true };

    case "clear":
      // Standard ANSI code to clear screen and reset cursor
      return { output: "\x1b[2J\x1b[H" };

    case "help":
      return {
        output: `
\x1b[1;32mAvailable Commands:\x1b[0m
  \x1b[33mls\x1b[0m                 List taxonomic divisions, families, or species in the current folder
  \x1b[33mcd [taxon]\x1b[0m          Change active taxonomic directory (e.g. \x1b[36mcd angiosperms\x1b[0m, \x1b[36mcd ..\x1b[0m)
  \x1b[33mpwd\x1b[0m                Print current absolute taxonomic path
  \x1b[33mcat [species.md]\x1b[0m    Inspect detailed botanical report & folklore of a species
  \x1b[33mtree\x1b[0m               Render ASCII taxonomic branching diagram from current folder
  \x1b[33mevolution\x1b[0m          Draw vertical geologic timeline and milestones of current lineage
  \x1b[33msearch [query]\x1b[0m     Search full database for any plant, family, or Gaelic term
  \x1b[33mask [question]\x1b[0m      Query the Caledonian Botanist AI on folklore, uses, or biology
  \x1b[33mclear\x1b[0m              Clear terminal screen
  \x1b[33mexit\x1b[0m / \x1b[33mquit\x1b[0m          Exit the application

\x1b[1;36mTaxonomic Ranks of Earth:\x1b[0m
  \x1b[35mclade\x1b[0m     -> Deep evolutionary branches (e.g. Bryophytes, Gymnosperms)
  \x1b[35mclass\x1b[0m     -> Major botanical classes (e.g. Bryophyta, Conifers)
  \x1b[35mfamily\x1b[0m    -> Related plant groupings (ending in -aceae, e.g. Ericaceae)
  \x1b[35mgenus\x1b[0m     -> General plant genus (e.g. Calluna, Pinus)
  \x1b[35mspecies\x1b[0m   -> Individual plant files (e.g. vulgaris, sylvestris)
`
      };

    case "pwd":
      return { output: `/${pathSegments.join("/")}` };

    case "ls": {
      if (!currentNode.children || Object.keys(currentNode.children).length === 0) {
        return { output: `This is a terminal species file. Type \x1b[33mcat ${currentNode.name.split(" ")[1] || currentNode.name}.md\x1b[0m to read details, or \x1b[33mcd ..\x1b[0m to go up.` };
      }

      const keys = Object.keys(currentNode.children);
      let listOutput = "\x1b[1;37mTaxon elements in current clade:\x1b[0m\n\n";
      keys.forEach(key => {
        const child = currentNode.children![key];
        if (child.rank === "species") {
          listOutput += `  \x1b[32m📄 ${key}.md\x1b[0m   (${child.commonName || "Native Species"})\n`;
        } else {
          const rankColor = child.rank === "clade" ? "\x1b[36m" : child.rank === "class" ? "\x1b[35m" : child.rank === "family" ? "\x1b[34m" : "\x1b[33m";
          listOutput += `  ${rankColor}📁 ${key}/\x1b[0m   [${child.rank}] - ${child.commonName || ""}\n`;
        }
      });
      return { output: listOutput };
    }

    case "cd": {
      const targetDir = args[0] || "";
      const resolved = resolvePath(floraData, pathSegments, targetDir);
      if (resolved === null) {
        return { output: `\x1b[31mcd: no such taxonomic folder: ${targetDir}\x1b[0m` };
      } else {
        pathSegments = resolved;
        return { output: "" };
      }
    }

    case "cat": {
      let filename = args[0] || "";
      if (!filename) {
        return { output: "\x1b[31mcat: missing species file argument. Example: cat vulgaris.md\x1b[0m" };
      }
      if (filename.endsWith(".md")) {
        filename = filename.slice(0, -3);
      }

      // Check if filename matches a child species
      let foundSpecies: PlantNode | undefined;
      if (currentNode.children && currentNode.children[filename]) {
        foundSpecies = currentNode.children[filename];
      } else if (currentNode.rank === "species" && currentNode.name.toLowerCase().endsWith(filename.toLowerCase())) {
        foundSpecies = currentNode;
      }

      if (foundSpecies && foundSpecies.rank === "species") {
        return {
          output: `
\x1b[1;33m${foundSpecies.name.toUpperCase()}\x1b[0m
\x1b[36mCommon Name:\x1b[0m   ${foundSpecies.commonName || "Unknown"}
\x1b[36mGaelic Name:\x1b[0m   ${foundSpecies.gaelicName || "None recorded"}
\x1b[36mConservation:\x1b[0m  ${foundSpecies.status || "Unspecified"}
\x1b[36mOrigin Era:\x1b[0m    ${foundSpecies.geologicalEra || "Prehistoric"}
\x1b[36mEvolutionary:\x1b[0m   ${foundSpecies.evolutionaryMilestone || ""}

\x1b[1;32m=== BOTANICAL DESCRIPTION ===\x1b[0m
${foundSpecies.description}

\x1b[1;32m=== HIGHLAND HABITAT ===\x1b[0m
${foundSpecies.habitat || "Widespread"}

\x1b[1;32m=== TRADITIONAL LORE & FOLKLORE ===\x1b[0m
${foundSpecies.lore || "None"}

\x1b[1;33m=== ASCII REPRESENTATION ===\x1b[0m
${foundSpecies.asciiArt || ""}
`
        };
      } else {
        return { output: `\x1b[31mcat: file not found or is a folder: ${args[0]}. (Tip: Use 'ls' to find .md species files)\x1b[0m` };
      }
    }

    case "tree":
      return { output: `\x1b[1;32mPhylogeny Tree starting from ${currentNode.name}:\x1b[0m\n\n` + buildAsciiTree(currentNode, "", true) };

    case "evolution":
    case "lineage": {
      let timeline = "\x1b[1;33m=== EVOLUTIONARY DEEP HISTORY OF CURRENT TAXON ===\x1b[0m\n\n";
      let tempSegments: string[] = [];
      let lineageNodes: PlantNode[] = [floraData];

      for (const segment of pathSegments) {
        tempSegments.push(segment);
        const node = getNodeFromPath(floraData, tempSegments);
        if (node) lineageNodes.push(node);
      }

      lineageNodes.forEach((node, idx) => {
        const isCurrent = idx === lineageNodes.length - 1;
        const arrow = isCurrent ? " ● \x1b[1;32m[ACTIVE]\x1b[0m " : " │   ";
        timeline += `\x1b[1;36m${node.geologicalEra || "Deep Time"}\x1b[0m\n`;
        timeline += `${arrow}\x1b[33m${node.name}\x1b[0m (${node.rank})\n`;
        if (node.evolutionaryMilestone) {
          timeline += ` │   \x1b[37m→ Landmark: ${node.evolutionaryMilestone}\x1b[0m\n`;
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
        return { output: "\x1b[31msearch: missing search term. Example: search heather\x1b[0m" };
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
        return { output: `No taxonomic matches found for: \x1b[31m"${query}"\x1b[0m` };
      } else {
        let searchOutput = `Found \x1b[1;32m${results.length}\x1b[0m phylogenetic branches or species:\n\n`;
        results.forEach(r => {
          searchOutput += `  \x1b[1;33m${r.name}\x1b[0m [${r.rank}] ${r.common ? `(${r.common})` : ""}\n`;
          if (r.gaelic) searchOutput += `    Gaelic: ${r.gaelic}\n`;
          searchOutput += `    Path:   \x1b[1;36mcd ${r.path}\x1b[0m\n\n`;
        });
        return { output: searchOutput };
      }
    }

    case "ask": {
      const question = args.join(" ").trim();
      if (!question) {
        return { output: "\x1b[31mask: What would you like to ask the Caledonian Botanist? Example: ask Tell me about rowan saining rituals.\x1b[0m" };
      }

      console.log("\x1b[33mThe Caledonian Botanist Sage is cogitating...\x1b[0m");

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

        const res = await fetch("http://localhost:11434/api/chat", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            model: "qwen2.5:7b",
            messages: [
              { role: "system", content: system },
              { role: "user", content: question },
            ],
            options: { temperature: 0.7 },
            stream: false,
          }),
        });

        if (!res.ok) throw new Error(`ollama returned ${res.status}`);
        const data = await res.json();
        const text = data.message?.content || "The sage was silent.";

        return { output: `\n\x1b[1;33mTHE CALEDONIAN BOTANIST SAGE COGITATES:\x1b[0m\n\n${text}\n` };
      } catch (err: any) {
        return { output: `\x1b[31mError connecting to Caledonian Botanist AI: ${err?.message || err}\x1b[0m` };
      }
    }

    default:
      return { output: `\x1b[31mbash: command not found: ${command}. (Type 'help' to see list of valid commands)\x1b[0m` };
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
  console.log("\x1b[1;33m=================================================================\x1b[0m");
  console.log("\x1b[1;32m          🌲  CALEDONIAN PHYLOGENETIC TERMINAL v1.2  🌲          \x1b[0m");
  console.log("\x1b[1;33m=================================================================\x1b[0m");
  console.log("Welcome, Scholar. Traverse the deep branches of Scottish Flora.");
  console.log("Type \\x1b[36mhelp\\x1b[0m to list commands, \\x1b[36mls\\x1b[0m to view clades, or \\x1b[36mexit\\x1b[0m to quit.");
  console.log("Inquire of the \\x1b[1;33mCaledonian Botanist AI\\x1b[0m using: \\x1b[33mask [question]\\x1b[0m");
  console.log("\x1b[1;33m-----------------------------------------------------------------\x1b[0m\n");

  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  const prompt = () => {
    const currentPathStr = "/" + pathSegments.join("/");
    rl.question(`\x1b[1;32mguest@caledonia\x1b[0m:\x1b[1;34m${currentPathStr}\x1b[0m$ `, async (line) => {
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
