import React, { useState, useEffect } from "react";
import { TreePine, Calendar, Eye, Terminal as TerminalIcon, GitFork, BookOpen, Sparkles, ChevronRight, HelpCircle } from "lucide-react";
import Terminal, { getNodeFromPath } from "./components/Terminal";
import VisualTree from "./components/VisualTree";
import FieldGuide from "./components/FieldGuide";
import { floraData } from "./data/floraData";
import { PlantNode } from "./types";

export default function App() {
  const [pathSegments, setPathSegments] = useState<string[]>([]);
  const [activeSpecies, setActiveSpecies] = useState<PlantNode | null>(null);
  const [chatLog, setChatLog] = useState<{ role: string; text: string }[]>([]);
  const [showHowToUse, setShowHowToUse] = useState(false);

  // Sync active species if pathSegments changes
  useEffect(() => {
    const node = getNodeFromPath(floraData, pathSegments);
    if (node && node.rank === "species") {
      setActiveSpecies(node);
    } else {
      setActiveSpecies(null);
    }
  }, [pathSegments]);

  // Navigate to a specific breadcrumb level
  const navigateToBreadcrumb = (index: number) => {
    const newPath = pathSegments.slice(0, index + 1);
    setPathSegments(newPath);
  };

  const resetToRoot = () => {
    setPathSegments([]);
  };

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 flex flex-col font-sans selection:bg-emerald-500/30 selection:text-emerald-200">
      
      {/* Immersive Top Bar */}
      <header className="border-b border-slate-900 bg-slate-950 px-6 py-4 flex flex-col md:flex-row items-start md:items-center justify-between gap-4 sticky top-0 z-40 backdrop-blur-md bg-opacity-95">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-emerald-950/60 border border-emerald-500/20 rounded-lg text-emerald-400">
            <TreePine className="w-6 h-6 animate-pulse" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-lg md:text-xl font-bold font-display tracking-tight text-slate-100">
                Scottish Flora Phylogeny Terminal
              </h1>
              <span className="hidden md:inline px-2 py-0.5 rounded text-[9px] font-mono font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                Caledonian Clades
              </span>
            </div>
            <p className="text-xs text-slate-400">
              Phylogenetic evolutionary taxonomy & traditional Gaelic ethnobotany
            </p>
          </div>
        </div>

        {/* Navigation Breadcrumbs */}
        <div className="flex items-center flex-wrap gap-1 bg-slate-900/40 border border-slate-800/40 px-3 py-1.5 rounded-lg text-xs font-mono max-w-full">
          <button
            onClick={resetToRoot}
            className={`hover:text-emerald-400 transition cursor-pointer ${
              pathSegments.length === 0 ? "text-emerald-400 font-bold" : "text-slate-400"
            }`}
          >
            Plantae
          </button>
          {pathSegments.map((seg, idx) => (
            <React.Fragment key={idx}>
              <ChevronRight className="w-3.5 h-3.5 text-slate-600" />
              <button
                onClick={() => navigateToBreadcrumb(idx)}
                className={`hover:text-emerald-400 transition cursor-pointer ${
                  idx === pathSegments.length - 1 ? "text-emerald-400 font-bold" : "text-slate-400"
                }`}
              >
                {seg}
              </button>
            </React.Fragment>
          ))}
        </div>

        {/* Global Toolbar */}
        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={() => setShowHowToUse(!showHowToUse)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-slate-900 hover:bg-slate-800 text-xs font-medium border border-slate-800 text-slate-300 transition cursor-pointer"
          >
            <HelpCircle className="w-3.5 h-3.5" />
            <span>Guide</span>
          </button>
        </div>
      </header>

      {/* Collapsible Tutorial / Help Banner */}
      {showHowToUse && (
        <div className="bg-gradient-to-r from-slate-900 to-emerald-950/20 border-b border-slate-800/80 p-6 text-slate-300 transition-all duration-300">
          <div className="max-w-6xl mx-auto grid md:grid-cols-3 gap-6">
            <div>
              <div className="flex items-center gap-2 mb-2 text-emerald-400">
                <TerminalIcon className="w-4 h-4" />
                <h4 className="text-xs font-mono font-bold uppercase">The Botanical Terminal</h4>
              </div>
              <p className="text-xs text-slate-400 leading-relaxed">
                Type commands directly to traverse taxonomic nodes. Use <span className="font-mono text-emerald-300 bg-slate-950 px-1 py-0.5 rounded">ls</span> to list clades and classes, and <span className="font-mono text-emerald-300 bg-slate-950 px-1 py-0.5 rounded">cd [taxon]</span> to burrow deep into evolution. Press <span className="font-mono text-slate-400 bg-slate-950 px-1 py-0.5 rounded">Tab</span> for autocomplete.
              </p>
            </div>
            <div>
              <div className="flex items-center gap-2 mb-2 text-sky-400">
                <GitFork className="w-4 h-4" />
                <h4 className="text-xs font-mono font-bold uppercase">Phylogenetic Tree Map</h4>
              </div>
              <p className="text-xs text-slate-400 leading-relaxed">
                The visual tree on the right maps out the evolutionary paths. Click on any circle or plant name to change directory in the terminal, highlight ancestral lines, and see the lineages glow in real-time.
              </p>
            </div>
            <div>
              <div className="flex items-center gap-2 mb-2 text-amber-400">
                <BookOpen className="w-4 h-4" />
                <h4 className="text-xs font-mono font-bold uppercase">Caledonian Botanist AI</h4>
              </div>
              <p className="text-xs text-slate-400 leading-relaxed">
                Inquire about Scottish flora folklore, clan uses, or plant adaptations. Use <span className="font-mono text-amber-300 bg-slate-950 px-1 py-0.5 rounded">ask [question]</span> in the terminal, or chat directly in the Botanist Journal tab on the right.
              </p>
            </div>
          </div>
          <div className="text-center mt-4">
            <button
              onClick={() => setShowHowToUse(false)}
              className="text-xs text-slate-400 hover:text-white underline"
            >
              Dismiss Guide
            </button>
          </div>
        </div>
      )}

      {/* Main Grid Workspace */}
      <main className="flex-1 p-6 max-w-7xl mx-auto w-full grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        
        {/* Left Column: Terminal Panel (Interactive Shell) */}
        <section className="lg:col-span-5 w-full flex flex-col gap-4">
          <div className="flex justify-between items-center px-1">
            <span className="text-[10px] font-mono uppercase text-slate-500 font-bold tracking-wider">
              Console Interface [TTY 1]
            </span>
            <div className="flex items-center gap-1 text-[10px] font-mono text-slate-400">
              <span className="w-2 h-2 rounded-full bg-emerald-500" />
              <span>Shell Online</span>
            </div>
          </div>
          <Terminal
            pathSegments={pathSegments}
            setPathSegments={setPathSegments}
            setActiveSpecies={setActiveSpecies}
            chatLog={chatLog}
            setChatLog={setChatLog}
          />
        </section>

        {/* Center/Right Columns: Interactive Tree and Companion Manual */}
        <section className="lg:col-span-7 w-full grid grid-cols-1 md:grid-cols-2 gap-6">
          
          {/* Visual Phylogenetic Tree Panel */}
          <div className="flex flex-col gap-4 w-full">
            <span className="text-[10px] font-mono uppercase text-slate-500 font-bold tracking-wider px-1">
              Phylogeny Cladogram
            </span>
            <VisualTree
              pathSegments={pathSegments}
              setPathSegments={setPathSegments}
            />
          </div>

          {/* Detailed Field Guide and AI Botanist Chat Panel */}
          <div className="flex flex-col gap-4 w-full">
            <span className="text-[10px] font-mono uppercase text-slate-500 font-bold tracking-wider px-1">
              Caledonian Herbarium Guide
            </span>
            <FieldGuide
              activeSpecies={activeSpecies}
              chatLog={chatLog}
              setChatLog={setChatLog}
              pathSegments={pathSegments}
              setPathSegments={setPathSegments}
            />
          </div>

        </section>

      </main>

      {/* Footer Credentials (No telemetry/clutter, only clean, honest credits) */}
      <footer className="border-t border-slate-900 bg-slate-950 py-4 px-6 mt-8">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4 text-xs font-mono text-slate-500">
          <div className="flex items-center gap-1.5 select-none">
            <Calendar className="w-3.5 h-3.5 text-slate-600" />
            <span>Phylogeny Timeline: Ordovician Colonization to Modern Holocene</span>
          </div>
          <div className="text-center sm:text-right">
            <span>Scottish Ethnobotany & Evolutionary Cladistics Explorer</span>
          </div>
        </div>
      </footer>

    </div>
  );
}
