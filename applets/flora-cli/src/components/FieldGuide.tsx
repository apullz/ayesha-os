import React, { useState } from "react";
import { BookOpen, TreePine, Eye, Compass, MessageSquareCode, Sparkles, Copy, Check, Send } from "lucide-react";
import { PlantNode } from "../types";

interface FieldGuideProps {
  activeSpecies: PlantNode | null;
  chatLog: { role: string; text: string }[];
  setChatLog: React.Dispatch<React.SetStateAction<{ role: string; text: string }[]>>;
  pathSegments: string[];
  setPathSegments: (segments: string[]) => void;
}

export default function FieldGuide({
  activeSpecies,
  chatLog,
  setChatLog,
  pathSegments,
  setPathSegments
}: FieldGuideProps) {
  const [activeTab, setActiveTab] = useState<"guide" | "chat">("guide");
  const [isCopied, setIsCopied] = useState(false);
  const [question, setQuestion] = useState("");
  const [isSending, setIsSending] = useState(false);

  // Suggested folders to CD to
  const habitats = [
    {
      name: "Peatland Blanket Bogs",
      glen: "The Great Flow Country",
      suggestedPath: ["bryophytes", "mosses", "sphagnaceae", "sphagnum", "capillifolium"],
      desc: "Vast peat-forming wetlands holding more carbon than all of the UK's forests combined, dominated by water-storing mosses."
    },
    {
      name: "Caledonian Pine Forest",
      glen: "Glen Affric & Rothiemurchus",
      suggestedPath: ["gymnosperms", "pinaceae", "pinus", "sylvestris"],
      desc: "Ancient post-glacial pine and birch woods, the home of red squirrels, capercaillie, and wildcats."
    },
    {
      name: "The Celtic Rainforest",
      glen: "Taynish & West Highland Glens",
      suggestedPath: ["bryophytes", "liverworts", "herbertaceae", "herbertus", "borealis"],
      desc: "Hyper-humid oakwoods and gorges drenched in Atlantic sea mists, supporting rare oceanic liverworts and mosses found nowhere else in Europe."
    },
    {
      name: "Highland Mountaintops",
      glen: "The Cairngorms Plateau",
      suggestedPath: ["angiosperms", "eudicots", "rosaceae", "rubus", "chamaemorus"],
      desc: "Sub-arctic alpine tundras and high blanket bogs housing rare glacial relics from the last Ice Age."
    }
  ];

  const handleCopyAscii = () => {
    if (activeSpecies?.asciiArt) {
      navigator.clipboard.writeText(activeSpecies.asciiArt);
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);
    }
  };

  const handleSendQuestion = async (e: React.FormEvent) => {
    e.preventDefault();
    const query = question.trim();
    if (!query || isSending) return;

    setQuestion("");
    setIsSending(true);
    // Add user message to log
    setChatLog(prev => [...prev, { role: "user", text: query }]);
    setActiveTab("chat");

    try {
      const response = await fetch("/api/botanist", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          prompt: query,
          speciesContext: activeSpecies ? activeSpecies.name : undefined,
          pathContext: "/" + pathSegments.join("/")
        })
      });

      const data = await response.json();
      if (response.ok) {
        setChatLog(prev => [...prev, { role: "ai", text: data.text }]);
      } else {
        setChatLog(prev => [...prev, { role: "ai", text: `The Sage has retreated momentarily: ${data.error || "connection error"}` }]);
      }
    } catch (err) {
      setChatLog(prev => [...prev, { role: "ai", text: "The Caledonian Sage is currently meditating. Please check if your Express dev server has completed compilation." }]);
    } finally {
      setIsSending(false);
    }
  };

  return (
    <div className="flex flex-col h-[520px] md:h-[600px] rounded-xl bg-slate-900 border border-slate-800 shadow-xl overflow-hidden">
      {/* Navigation Tabs */}
      <div className="flex items-center justify-between bg-slate-950 border-b border-slate-800 px-4">
        <div className="flex gap-4">
          <button
            onClick={() => setActiveTab("guide")}
            className={`flex items-center gap-1.5 py-3 text-xs font-semibold tracking-wider uppercase border-b-2 transition-all cursor-pointer ${
              activeTab === "guide"
                ? "border-emerald-500 text-emerald-400"
                : "border-transparent text-slate-500 hover:text-slate-300"
            }`}
          >
            <BookOpen className="w-3.5 h-3.5" />
            <span>Field Manual</span>
          </button>
          <button
            onClick={() => setActiveTab("chat")}
            className={`flex items-center gap-1.5 py-3 text-xs font-semibold tracking-wider uppercase border-b-2 transition-all cursor-pointer ${
              activeTab === "chat"
                ? "border-amber-500 text-amber-400"
                : "border-transparent text-slate-500 hover:text-slate-300"
            }`}
          >
            <MessageSquareCode className="w-3.5 h-3.5" />
            <span>Botanist AI Journal</span>
            {chatLog.length > 0 && (
              <span className="ml-1 w-4 h-4 rounded-full bg-amber-500/20 text-amber-400 flex items-center justify-center text-[9px] font-bold">
                {chatLog.filter(c => c.role === "ai").length}
              </span>
            )}
          </button>
        </div>
      </div>

      {/* Manual Tab content */}
      {activeTab === "guide" && (
        <div className="flex-1 overflow-y-auto p-5 space-y-5 bg-slate-900/40 text-slate-300">
          {activeSpecies ? (
            <div className="space-y-4">
              {/* Header Info */}
              <div className="border-b border-slate-800 pb-3">
                <div className="flex justify-between items-start">
                  <div>
                    <h2 className="text-xl font-bold text-slate-100 font-mono italic">
                      {activeSpecies.name}
                    </h2>
                    <p className="text-xs text-emerald-400 font-sans tracking-wide">
                      {activeSpecies.commonName} • <span className="font-semibold">{activeSpecies.gaelicName}</span>
                    </p>
                  </div>
                  <span className="px-2 py-0.5 rounded text-[9px] font-mono uppercase font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    {activeSpecies.status || "Native"}
                  </span>
                </div>
              </div>

              {/* Evolutionary Stats Grid */}
              <div className="grid grid-cols-2 gap-3 text-xs font-mono">
                <div className="p-2.5 rounded bg-slate-950/40 border border-slate-800/40">
                  <span className="text-[10px] text-slate-500 block uppercase mb-0.5">Origin Epoch</span>
                  <span className="text-amber-300 font-semibold">{activeSpecies.geologicalEra || "Deep Paleozoic"}</span>
                </div>
                <div className="p-2.5 rounded bg-slate-950/40 border border-slate-800/40">
                  <span className="text-[10px] text-slate-500 block uppercase mb-0.5">Key Adaptation</span>
                  <span className="text-teal-300 leading-tight block">{activeSpecies.evolutionaryMilestone || "Seed dispersion"}</span>
                </div>
              </div>

              {/* Botanical Description */}
              <div>
                <h3 className="text-[10px] uppercase font-bold text-slate-500 tracking-wider flex items-center gap-1 mb-1 font-mono">
                  <Eye className="w-3 h-3 text-emerald-400" />
                  Botanical Summary
                </h3>
                <p className="text-xs leading-relaxed text-slate-300 font-sans bg-slate-950/20 p-3 rounded border border-slate-800/30">
                  {activeSpecies.description}
                </p>
              </div>

              {/* Highland Habitat */}
              <div>
                <h3 className="text-[10px] uppercase font-bold text-slate-500 tracking-wider flex items-center gap-1 mb-1 font-mono">
                  <Compass className="w-3 h-3 text-sky-400" />
                  Scottish Habitat
                </h3>
                <p className="text-xs leading-relaxed text-slate-300 font-sans bg-slate-950/20 p-3 rounded border border-slate-800/30">
                  {activeSpecies.habitat}
                </p>
              </div>

              {/* Folklore and Clan Uses */}
              <div>
                <h3 className="text-[10px] uppercase font-bold text-slate-500 tracking-wider flex items-center gap-1 mb-1 font-mono">
                  <TreePine className="w-3 h-3 text-amber-400" />
                  Gaelic Lore & Traditional Uses
                </h3>
                <p className="text-xs leading-relaxed text-slate-300 font-sans bg-slate-950/20 p-3 rounded border border-slate-800/30">
                  {activeSpecies.lore}
                </p>
              </div>

              {/* ASCII Art Box */}
              {activeSpecies.asciiArt && (
                <div className="space-y-1.5">
                  <div className="flex justify-between items-center text-[10px] uppercase text-slate-500 font-mono">
                    <span>ASCII Herbarium Frame</span>
                    <button
                      onClick={handleCopyAscii}
                      className="flex items-center gap-1 text-[9px] hover:text-emerald-400 transition bg-slate-800 px-1.5 py-0.5 rounded cursor-pointer"
                    >
                      {isCopied ? <Check className="w-2.5 h-2.5 text-green-400" /> : <Copy className="w-2.5 h-2.5" />}
                      <span>{isCopied ? "Copied" : "Copy ASCII"}</span>
                    </button>
                  </div>
                  <pre className="text-[10px] font-mono leading-none bg-slate-950 border border-slate-800/60 p-4 rounded text-emerald-400 text-center overflow-x-auto whitespace-pre">
                    {activeSpecies.asciiArt}
                  </pre>
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-6">
              {/* No Active Selection Banner */}
              <div className="text-center py-6 border border-dashed border-slate-800 rounded-lg bg-slate-950/20">
                <Compass className="w-8 h-8 text-slate-600 mx-auto mb-2 animate-pulse" />
                <h3 className="text-xs font-mono font-bold text-slate-300 uppercase tracking-wider">Caledonia Flora Explorer</h3>
                <p className="text-[11px] text-slate-500 max-w-xs mx-auto mt-1">
                  Type <span className="font-mono text-emerald-400 bg-slate-950 px-1 py-0.5 rounded">cat [file].md</span> or click a species node in the visual tree to view custom field data.
                </p>
              </div>

              {/* Key Scottish Habitats Overview */}
              <div className="space-y-3">
                <h4 className="text-[10px] font-mono uppercase font-bold text-slate-500 tracking-wider">
                  Major Highland Ecosystems & Glens:
                </h4>
                <div className="grid gap-3">
                  {habitats.map((hab, idx) => (
                    <div
                      key={idx}
                      onClick={() => setPathSegments(hab.suggestedPath)}
                      className="p-3 rounded-lg border border-slate-800/80 bg-slate-950/40 hover:bg-slate-800/30 hover:border-emerald-500/30 transition-all cursor-pointer group"
                    >
                      <div className="flex justify-between items-start">
                        <h5 className="text-xs font-bold text-slate-200 font-mono group-hover:text-emerald-400 transition-colors">
                          {hab.name}
                        </h5>
                        <span className="text-[8px] font-mono text-emerald-500 bg-emerald-500/10 px-1 py-0.5 rounded uppercase">
                          Explore Path
                        </span>
                      </div>
                      <p className="text-[9px] text-slate-500 font-mono mt-0.5 italic">
                        Stronghold: {hab.glen}
                      </p>
                      <p className="text-[11px] text-slate-400 font-sans mt-1.5 leading-relaxed">
                        {hab.desc}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Chat Tab content */}
      {activeTab === "chat" && (
        <div className="flex-1 flex flex-col overflow-hidden bg-slate-950/20">
          {/* Scrollable Conversation History */}
          <div className="flex-1 overflow-y-auto p-4 space-y-3">
            {chatLog.length === 0 ? (
              <div className="h-full flex flex-col items-center justify-center text-center p-6 space-y-3">
                <Sparkles className="w-10 h-10 text-amber-500/40 animate-pulse" />
                <div className="space-y-1">
                  <h4 className="text-xs font-mono font-bold text-slate-300 uppercase">Consult the Botanical Sage</h4>
                  <p className="text-[11px] text-slate-500 max-w-xs leading-relaxed">
                    Ask any question about Scottish flora, clan uses, Highland foraging, and plant evolutionary ancestry. 
                    The Sage will use its context on your active node to answer!
                  </p>
                </div>
                <div className="flex flex-wrap justify-center gap-1.5 text-[9px] font-mono text-slate-400 pt-2">
                  <button
                    onClick={() => setQuestion("Why did clans plant rowan trees by blackhouses?")}
                    className="px-2 py-1 rounded border border-slate-800 hover:border-amber-500/40 hover:bg-slate-900 transition cursor-pointer"
                  >
                    "Why plant rowan?"
                  </button>
                  <button
                    onClick={() => setQuestion("Tell me about the medicinal uses of sphagnum moss.")}
                    className="px-2 py-1 rounded border border-slate-800 hover:border-amber-500/40 hover:bg-slate-900 transition cursor-pointer"
                  >
                    "Sphagnum healing uses"
                  </button>
                  <button
                    onClick={() => setQuestion("How old is the Fortingall Yew in Perthshire?")}
                    className="px-2 py-1 rounded border border-slate-800 hover:border-amber-500/40 hover:bg-slate-900 transition cursor-pointer"
                  >
                    "The Fortingall Yew age"
                  </button>
                </div>
              </div>
            ) : (
              chatLog.map((chat, idx) => (
                <div
                  key={idx}
                  className={`p-3 rounded-lg text-xs leading-relaxed ${
                    chat.role === "user"
                      ? "bg-slate-800/40 border border-slate-800/60 ml-8 text-slate-200"
                      : "bg-amber-500/5 border border-amber-500/15 mr-8 text-amber-100"
                  }`}
                >
                  <div className="flex items-center gap-1.5 font-mono text-[9px] uppercase font-bold mb-1.5 select-none">
                    <span className={chat.role === "user" ? "text-emerald-400" : "text-amber-400"}>
                      {chat.role === "user" ? "Guest Scholar" : "Caledonian Botanist Sage"}
                    </span>
                  </div>
                  <p className="font-sans whitespace-pre-wrap">{chat.text}</p>
                </div>
              ))
            )}

            {isSending && (
              <div className="p-3 rounded-lg text-xs leading-relaxed bg-amber-500/5 border border-amber-500/15 mr-8 text-amber-100/60 animate-pulse flex items-center gap-2">
                <Sparkles className="w-3.5 h-3.5 animate-spin text-amber-400" />
                <span className="font-mono text-[10px] uppercase font-semibold">The Sage is consulting records...</span>
              </div>
            )}
          </div>

          {/* Chat Input Bar */}
          <form
            onSubmit={handleSendQuestion}
            className="p-3 bg-slate-950 border-t border-slate-800 flex gap-2 items-center"
          >
            <input
              type="text"
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              disabled={isSending}
              placeholder="Ask the Scottish botanist a question..."
              className="flex-1 bg-slate-900 border border-slate-800 rounded px-3 py-1.5 text-xs font-sans text-slate-200 focus:outline-none focus:border-amber-500 transition"
            />
            <button
              type="submit"
              disabled={isSending || !question.trim()}
              className="bg-amber-500 text-slate-950 hover:bg-amber-400 disabled:bg-slate-800 disabled:text-slate-600 transition px-3 py-1.5 rounded flex items-center gap-1 cursor-pointer"
            >
              <Send className="w-3.5 h-3.5" />
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
