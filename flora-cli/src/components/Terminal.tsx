import React, { useState, useRef, useEffect } from "react";
import { Terminal as TerminalIcon, ShieldAlert, Sparkles, CornerDownLeft } from "lucide-react";
import { PlantNode } from "../types";
import { floraData } from "../data/floraData";

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

// Parse terminal color codes into JSX
function parseAnsiColors(text: string): React.ReactNode[] {
  const parts = text.split(/(\x1b\[\d+m)/);
  let currentColorClass = "text-emerald-300";

  return parts.map((part, i) => {
    if (part.startsWith("\x1b[")) {
      if (part === "\x1b[0m") currentColorClass = "text-emerald-300"; // reset
      else if (part === "\x1b[31m") currentColorClass = "text-rose-400"; // red
      else if (part === "\x1b[32m") currentColorClass = "text-green-400"; // green
      else if (part === "\x1b[33m") currentColorClass = "text-amber-300 font-semibold"; // gold
      else if (part === "\x1b[34m") currentColorClass = "text-sky-400"; // blue
      else if (part === "\x1b[35m") currentColorClass = "text-purple-400"; // purple
      else if (part === "\x1b[36m") currentColorClass = "text-teal-300"; // cyan
      else if (part === "\x1b[37m") currentColorClass = "text-slate-200"; // white
      return null;
    }
    return <span key={i} className={currentColorClass}>{part}</span>;
  }).filter(Boolean);
}

interface LogEntry {
  id: string;
  type: "input" | "output" | "error" | "ai";
  text: string;
  path: string;
}

interface TerminalProps {
  pathSegments: string[];
  setPathSegments: (segments: string[]) => void;
  setActiveSpecies: (node: PlantNode | null) => void;
  chatLog: { role: string; text: string }[];
  setChatLog: React.Dispatch<React.SetStateAction<{ role: string; text: string }[]>>;
}

export default function Terminal({
  pathSegments,
  setPathSegments,
  setActiveSpecies,
  chatLog,
  setChatLog
}: TerminalProps) {
  const [input, setInput] = useState("");
  const [logs, setLogs] = useState<LogEntry[]>([
    {
      id: "welcome-1",
      type: "output",
      text: "\x1b[33m=== CALEDONIAN PHYLOGENETIC TERMINAL v1.2 ===\x1b[0m",
      path: "/"
    },
    {
      id: "welcome-2",
      type: "output",
      text: "Welcome, Scholar. Explore the deep evolutionary branches of Scottish Flora.",
      path: "/"
    },
    {
      id: "welcome-3",
      type: "output",
      text: "Type \x1b[36mhelp\x1b[0m to list commands, \x1b[36mls\x1b[0m to view taxonomic nodes, or \x1b[36mtree\x1b[0m for complete phylogeny.",
      path: "/"
    },
    {
      id: "welcome-4",
      type: "output",
      text: "Ask the \x1b[33mCaledonian Botanist AI\x1b[0m anything about Highland folklore and botany using: \x1b[33mask [question]\x1b[0m",
      path: "/"
    },
    {
      id: "welcome-space",
      type: "output",
      text: "",
      path: "/"
    }
  ]);

  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [isAiLoading, setIsAiLoading] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto-scroll to bottom of terminal
  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs, isAiLoading]);

  // Focus input on container click
  const focusInput = () => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
  };

  useEffect(() => {
    focusInput();
  }, [pathSegments]);

  const addLog = (type: "input" | "output" | "error" | "ai", text: string) => {
    setLogs(prev => [
      ...prev,
      {
        id: Math.random().toString(),
        type,
        text,
        path: "/" + pathSegments.join("/")
      }
    ]);
  };

  // Autocomplete via Tab
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Tab") {
      e.preventDefault();
      const currentNode = getNodeFromPath(floraData, pathSegments);
      if (!currentNode || !currentNode.children) return;

      const tokens = input.trim().split(/\s+/);
      const command = tokens[0]?.toLowerCase();
      const argument = tokens[1] || "";

      if (command === "cd" || command === "cat") {
        const childrenKeys = Object.keys(currentNode.children);
        const matches = childrenKeys.filter(k => k.startsWith(argument.toLowerCase()));

        if (matches.length === 1) {
          // Unique match
          const suffix = currentNode.children[matches[0]].rank === "species" && command === "cat" ? ".md" : "";
          setInput(`${command} ${matches[0]}${suffix}`);
        } else if (matches.length > 1) {
          // Print potential matches
          addLog("input", input);
          addLog("output", matches.map(m => {
            const rank = currentNode.children![m].rank;
            const emoji = rank === "species" ? "📄" : "📁";
            return `${emoji} ${m}`;
          }).join("   "));
        }
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (history.length === 0) return;
      const nextIndex = historyIndex === -1 ? history.length - 1 : Math.max(0, historyIndex - 1);
      setHistoryIndex(nextIndex);
      setInput(history[nextIndex]);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (historyIndex === -1) return;
      if (historyIndex === history.length - 1) {
        setHistoryIndex(-1);
        setInput("");
      } else {
        const nextIndex = historyIndex + 1;
        setHistoryIndex(nextIndex);
        setInput(history[nextIndex]);
      }
    }
  };

  const handleCommandSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmedInput = input.trim();
    if (!trimmedInput) return;

    // Add to logs and command history
    addLog("input", trimmedInput);
    setHistory(prev => [...prev.filter(c => c !== trimmedInput), trimmedInput]);
    setHistoryIndex(-1);
    setInput("");

    const tokens = trimmedInput.split(/\s+/);
    const command = tokens[0].toLowerCase();
    const args = tokens.slice(1);

    const currentNode = getNodeFromPath(floraData, pathSegments)!;

    switch (command) {
      case "help":
        addLog("output", `
Available Commands:
  \x1b[33mls\x1b[0m                 List taxonomic divisions, families, or species in current folder
  \x1b[33mcd [taxon]\x1b[0m          Change active taxonomic directory (e.g. cd angiosperms, cd ..)
  \x1b[33mpwd\x1b[0m                Print current absolute taxonomic path
  \x1b[33mcat [species.md]\x1b[0m    Inspect detailed botanical report & folklore of a species
  \x1b[33mtree\x1b[0m               Render ASCII taxonomic branching diagram from current folder
  \x1b[33mevolution\x1b[0m          Draw vertical geologic timeline and milestones of current lineage
  \x1b[33msearch [query]\x1b[0m     Search full database for any plant, family, or gaelic term
  \x1b[33mask [question]\x1b[0m      Query the Caledonian Botanist AI on folklore, uses, or biology
  \x1b[33mclear\x1b[0m              Clear terminal screen

Taxonomic Ranks of Earth:
  \x1b[36mclade\x1b[0m     -> Deep evolutionary branches (e.g. Bryophytes, Gymnosperms)
  \x1b[36mclass\x1b[0m     -> Major botanical classes (e.g. Bryophyta, Conifers)
  \x1b[36mfamily\x1b[0m    -> Related plant groupings (ending in -aceae, e.g. Ericaceae)
  \x1b[36mgenus\x1b[0m     -> General plant genus (e.g. Calluna, Pinus)
  \x1b[36mspecies\x1b[0m   -> Individual plant files (e.g. vulgaris, sylvestris)
        `);
        break;

      case "pwd":
        addLog("output", `/${pathSegments.join("/")}`);
        break;

      case "ls":
        if (!currentNode.children || Object.keys(currentNode.children).length === 0) {
          addLog("output", `This is a terminal species file. Type \x1b[33mcat ${currentNode.name.split(" ")[1] || currentNode.name}.md\x1b[0m to read details, or \x1b[33mcd ..\x1b[0m to go up.`);
          break;
        }

        const keys = Object.keys(currentNode.children);
        let listOutput = "\x1b[37mTaxon elements in current clade:\x1b[0m\n\n";
        keys.forEach(key => {
          const child = currentNode.children![key];
          if (child.rank === "species") {
            listOutput += `  \x1b[32m📄 ${key}.md\x1b[0m   (${child.commonName || "Native Species"})\n`;
          } else {
            const rankColor = child.rank === "clade" ? "\x1b[36m" : child.rank === "class" ? "\x1b[35m" : child.rank === "family" ? "\x1b[34m" : "\x1b[33m";
            listOutput += `  ${rankColor}📁 ${key}/\x1b[0m   [${child.rank}] - ${child.commonName || ""}\n`;
          }
        });
        addLog("output", listOutput);
        break;

      case "cd":
        const targetDir = args[0] || "";
        const resolved = resolvePath(floraData, pathSegments, targetDir);
        if (resolved === null) {
          addLog("error", `cd: no such taxonomic folder: ${targetDir}`);
        } else {
          setPathSegments(resolved);
          const newNode = getNodeFromPath(floraData, resolved);
          if (newNode && newNode.rank === "species") {
            setActiveSpecies(newNode);
          }
        }
        break;

      case "cat":
        let filename = args[0] || "";
        if (!filename) {
          addLog("error", "cat: missing species file argument. Example: cat vulgaris.md");
          break;
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
          setActiveSpecies(foundSpecies);
          addLog("output", `
\x1b[33m${foundSpecies.name.toUpperCase()}\x1b[0m
\x1b[36mCommon Name:\x1b[0m   ${foundSpecies.commonName || "Unknown"}
\x1b[36mGaelic Name:\x1b[0m   ${foundSpecies.gaelicName || "None recorded"}
\x1b[36mConservation:\x1b[0m  ${foundSpecies.status || "Unspecified"}
\x1b[36mOrigin Era:\x1b[0m    ${foundSpecies.geologicalEra || "Prehistoric"}
\x1b[36mEvolutionary:\x1b[0m   ${foundSpecies.evolutionaryMilestone || ""}

\x1b[32m=== BOTANICAL DESCRIPTION ===\x1b[0m
${foundSpecies.description}

\x1b[32m=== HIGHLAND HABITAT ===\x1b[0m
${foundSpecies.habitat || "Widespread"}

\x1b[32m=== TRADITIONAL LORE & FOLKLORE ===\x1b[0m
${foundSpecies.lore || "None"}

\x1b[33m=== ASCII REPRESENTATION ===\x1b[0m
${foundSpecies.asciiArt || ""}
          `);
        } else {
          addLog("error", `cat: file not found or is a folder: ${args[0]}. (Tip: Use 'ls' to find .md species files)`);
        }
        break;

      case "tree":
        addLog("output", `\x1b[32mPhylogeny Tree starting from ${currentNode.name}:\x1b[0m\n\n` + buildAsciiTree(currentNode, "", true));
        break;

      case "evolution":
      case "lineage":
        let timeline = "\x1b[33m=== EVOLUTIONARY DEEP HISTORY OF CURRENT TAXON ===\x1b[0m\n\n";
        let tempSegments: string[] = [];
        let lineageNodes: PlantNode[] = [floraData];

        for (const segment of pathSegments) {
          tempSegments.push(segment);
          const node = getNodeFromPath(floraData, tempSegments);
          if (node) lineageNodes.push(node);
        }

        lineageNodes.forEach((node, idx) => {
          const isCurrent = idx === lineageNodes.length - 1;
          const arrow = isCurrent ? " ● \x1b[32m[ACTIVE]\x1b[0m " : " │   ";
          timeline += `\x1b[36m${node.geologicalEra || "Deep Time"}\x1b[0m\n`;
          timeline += `${arrow}\x1b[33m${node.name}\x1b[0m (${node.rank})\n`;
          if (node.evolutionaryMilestone) {
            timeline += ` │   \x1b[37m→ Landmark: ${node.evolutionaryMilestone}\x1b[0m\n`;
          }
          if (!isCurrent) {
            timeline += ` │\n`;
          }
        });
        addLog("output", timeline);
        break;

      case "search":
      case "locate":
        const query = args.join(" ").trim().toLowerCase();
        if (!query) {
          addLog("error", "search: missing search term. Example: search heather");
          break;
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
          addLog("output", `No taxonomic matches found for: \x1b[31m"${query}"\x1b[0m`);
        } else {
          let searchOutput = `Found \x1b[32m${results.length}\x1b[0m phylogenetic branches or species:\n\n`;
          results.forEach(r => {
            searchOutput += `  \x1b[33m${r.name}\x1b[0m [${r.rank}] ${r.common ? `(${r.common})` : ""}\n`;
            if (r.gaelic) searchOutput += `    Gaelic: ${r.gaelic}\n`;
            searchOutput += `    Path:   \x1b[36mcd ${r.path}\x1b[0m\n\n`;
          });
          addLog("output", searchOutput);
        }
        break;

      case "clear":
        setLogs([]);
        break;

      case "ask":
        const question = args.join(" ").trim();
        if (!question) {
          addLog("error", "ask: What would you like to ask the Caledonian Botanist? Example: ask Tell me about rowan saining rituals.");
          break;
        }

        setIsAiLoading(true);
        try {
          const response = await fetch("/api/botanist", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              prompt: question,
              speciesContext: currentNode.rank === "species" ? currentNode.name : undefined,
              pathContext: "/" + pathSegments.join("/")
            })
          });

          const data = await response.json();
          if (response.ok) {
            // Append to central chat logs so both components are updated
            setChatLog(prev => [...prev, { role: "user", text: question }, { role: "ai", text: data.text }]);
            addLog("ai", `\x1b[33mTHE CALEDONIAN BOTANIST SAGE COGELATES:\x1b[0m\n\n${data.text}`);
          } else {
            addLog("error", data.error || "The Sage had trouble hearing you. Try again.");
          }
        } catch (error) {
          addLog("error", "Could not reach the server-side botanist route. The server might be booting up or offline. Peatland magic has encountered an interruption.");
        } finally {
          setIsAiLoading(false);
        }
        break;

      default:
        addLog("error", `bash: command not found: ${command}. (Type 'help' to see list of valid commands)`);
        break;
    }
  };

  const executeShortcut = (cmdStr: string) => {
    setInput(cmdStr);
    // Submit in next tick or run directly
    setTimeout(() => {
      if (inputRef.current) {
        inputRef.current.focus();
      }
    }, 50);
  };

  return (
    <div
      id="terminal-container"
      className="flex flex-col h-[520px] md:h-[600px] rounded-xl bg-slate-950 border border-slate-800 shadow-2xl font-mono overflow-hidden focus:outline-none"
      onClick={focusInput}
    >
      {/* Terminal Title Bar */}
      <div className="flex items-center justify-between px-4 py-3 bg-slate-900 border-b border-slate-800 select-none">
        <div className="flex items-center gap-2">
          <TerminalIcon className="w-4 h-4 text-emerald-400" />
          <span className="text-xs font-semibold text-slate-300 tracking-wider uppercase">Caledonian Botanical Terminal</span>
        </div>
        <div className="flex gap-1.5">
          <span className="w-3 h-3 rounded-full bg-rose-500/80" />
          <span className="w-3 h-3 rounded-full bg-amber-500/80" />
          <span className="w-3 h-3 rounded-full bg-green-500/80" />
        </div>
      </div>

      {/* Terminal Log Output */}
      <div
        ref={containerRef}
        className="flex-1 p-4 overflow-y-auto text-emerald-300 text-xs leading-relaxed space-y-2 select-text"
      >
        {logs.map((log) => (
          <div key={log.id} className="whitespace-pre-wrap">
            {log.type === "input" && (
              <div className="flex items-start text-slate-400 gap-1.5 font-bold">
                <span className="text-amber-400">guest@caledonia:{log.path}$</span>
                <span className="text-emerald-200">{log.text}</span>
              </div>
            )}
            {log.type === "output" && (
              <div className="text-emerald-300 pl-2 border-l border-emerald-950">
                {parseAnsiColors(log.text)}
              </div>
            )}
            {log.type === "error" && (
              <div className="text-rose-400 pl-2 border-l border-rose-950 flex items-start gap-1.5">
                <ShieldAlert className="w-3.5 h-3.5 mt-0.5 shrink-0 text-rose-500" />
                <span>{log.text}</span>
              </div>
            )}
            {log.type === "ai" && (
              <div className="text-amber-200 pl-3 border-l-2 border-amber-500/30 bg-amber-500/5 py-1 px-2 rounded">
                {parseAnsiColors(log.text)}
              </div>
            )}
          </div>
        ))}

        {isAiLoading && (
          <div className="flex items-center gap-2 text-amber-300 italic animate-pulse py-1 pl-3 border-l-2 border-amber-500/30">
            <Sparkles className="w-3.5 h-3.5 animate-spin text-amber-400" />
            <span>The Caledonian Sage is searching through records and folklore...</span>
          </div>
        )}
      </div>

      {/* Shortcut Command Tray */}
      <div className="flex flex-wrap items-center gap-1.5 px-4 py-2 bg-slate-900/60 border-t border-slate-900 text-[10px] text-slate-400">
        <span className="font-semibold select-none text-slate-500">Quick Commands:</span>
        <button
          onClick={() => executeShortcut("ls")}
          className="px-1.5 py-0.5 rounded bg-slate-800 hover:bg-slate-700 hover:text-emerald-200 transition cursor-pointer"
        >
          ls
        </button>
        <button
          onClick={() => executeShortcut("tree")}
          className="px-1.5 py-0.5 rounded bg-slate-800 hover:bg-slate-700 hover:text-emerald-200 transition cursor-pointer"
        >
          tree
        </button>
        <button
          onClick={() => executeShortcut("evolution")}
          className="px-1.5 py-0.5 rounded bg-slate-800 hover:bg-slate-700 hover:text-emerald-200 transition cursor-pointer"
        >
          lineage
        </button>
        <button
          onClick={() => executeShortcut("cd ..")}
          className="px-1.5 py-0.5 rounded bg-slate-800 hover:bg-slate-700 hover:text-emerald-200 transition cursor-pointer"
        >
          cd ..
        </button>
        <button
          onClick={() => executeShortcut("help")}
          className="px-1.5 py-0.5 rounded bg-slate-800 hover:bg-slate-700 hover:text-emerald-200 transition cursor-pointer"
        >
          help
        </button>
      </div>

      {/* Terminal Input Prompt */}
      <form
        onSubmit={handleCommandSubmit}
        className="flex items-center gap-2 px-4 py-3 bg-slate-950 border-t border-slate-900"
      >
        <span className="text-amber-400 font-bold text-xs select-none shrink-0">
          guest@caledonia:{"/" + pathSegments.join("/")}$
        </span>
        <div className="relative flex-1 flex items-center">
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            className="w-full bg-transparent text-emerald-100 text-xs font-mono outline-none border-none p-0 focus:ring-0"
            placeholder="Type command here..."
            autoFocus
            autoComplete="off"
            spellCheck={false}
          />
          {input === "" && (
            <span className="absolute left-0 text-emerald-900 text-xs pointer-events-none select-none animate-pulse">
              Type 'help' or click a command...
            </span>
          )}
        </div>
        <button type="submit" className="text-slate-500 hover:text-emerald-400 transition cursor-pointer">
          <CornerDownLeft className="w-4 h-4" />
        </button>
      </form>
    </div>
  );
}
