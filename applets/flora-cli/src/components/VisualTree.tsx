import React, { useRef, useEffect } from "react";
import { TreePine, Calendar, Eye, GitBranch } from "lucide-react";
import { Rank } from "../types";

interface VisualNode {
  id: string;
  name: string;
  commonName?: string;
  rank: Rank;
  x: number;
  y: number;
  path: string[];
}

interface VisualLink {
  from: string;
  to: string;
}

const nodes: VisualNode[] = [
  { id: "/", name: "Plantae", rank: "clade", x: 40, y: 390, path: [] },
  
  // Clades
  { id: "/bryophytes", name: "Bryophytes", rank: "clade", x: 160, y: 130, path: ["bryophytes"] },
  { id: "/pteridophytes", name: "Pteridophytes", rank: "clade", x: 160, y: 260, path: ["pteridophytes"] },
  { id: "/gymnosperms", name: "Gymnosperms", rank: "clade", x: 160, y: 410, path: ["gymnosperms"] },
  { id: "/angiosperms", name: "Angiosperms", rank: "clade", x: 160, y: 640, path: ["angiosperms"] },

  // Classes/divisions
  { id: "/bryophytes/mosses", name: "Bryophyta (Mosses)", rank: "class", x: 280, y: 90, path: ["bryophytes", "mosses"] },
  { id: "/bryophytes/liverworts", name: "Liverworts", rank: "class", x: 280, y: 170, path: ["bryophytes", "liverworts"] },
  { id: "/pteridophytes/horsetails", name: "Horsetails", rank: "class", x: 280, y: 230, path: ["pteridophytes", "horsetails"] },
  { id: "/pteridophytes/ferns", name: "Ferns", rank: "class", x: 280, y: 290, path: ["pteridophytes", "ferns"] },
  { id: "/gymnosperms/pinaceae", name: "Pinaceae (Pine)", rank: "family", x: 280, y: 350, path: ["gymnosperms", "pinaceae"] },
  { id: "/gymnosperms/cupressaceae", name: "Cupressaceae", rank: "family", x: 280, y: 410, path: ["gymnosperms", "cupressaceae"] },
  { id: "/gymnosperms/taxaceae", name: "Taxaceae", rank: "family", x: 280, y: 470, path: ["gymnosperms", "taxaceae"] },
  { id: "/angiosperms/monocots", name: "Monocots", rank: "clade", x: 280, y: 550, path: ["angiosperms", "monocots"] },
  { id: "/angiosperms/eudicots", name: "Eudicots", rank: "clade", x: 280, y: 690, path: ["angiosperms", "eudicots"] },

  // Families under classes
  { id: "/bryophytes/mosses/sphagnaceae", name: "Sphagnaceae", rank: "family", x: 410, y: 60, path: ["bryophytes", "mosses", "sphagnaceae"] },
  { id: "/bryophytes/mosses/polytrichaceae", name: "Polytrichaceae", rank: "family", x: 410, y: 120, path: ["bryophytes", "mosses", "polytrichaceae"] },
  { id: "/bryophytes/liverworts/herbertaceae", name: "Herbertaceae", rank: "family", x: 410, y: 170, path: ["bryophytes", "liverworts", "herbertaceae"] },
  { id: "/pteridophytes/horsetails/equisetaceae", name: "Equisetaceae", rank: "family", x: 410, y: 230, path: ["pteridophytes", "horsetails", "equisetaceae"] },
  { id: "/pteridophytes/ferns/dennstaedtiaceae", name: "Dennstaedtiaceae", rank: "family", x: 410, y: 290, path: ["pteridophytes", "ferns", "dennstaedtiaceae"] },
  { id: "/gymnosperms/pinaceae/pinus", name: "Pinus (Genus)", rank: "genus", x: 410, y: 350, path: ["gymnosperms", "pinaceae", "pinus"] },
  { id: "/gymnosperms/cupressaceae/juniperus", name: "Juniperus (Genus)", rank: "genus", x: 410, y: 410, path: ["gymnosperms", "cupressaceae", "juniperus"] },
  { id: "/gymnosperms/taxaceae/taxus", name: "Taxus (Genus)", rank: "genus", x: 410, y: 470, path: ["gymnosperms", "taxaceae", "taxus"] },
  { id: "/angiosperms/monocots/orchidaceae", name: "Orchidaceae", rank: "family", x: 410, y: 520, path: ["angiosperms", "monocots", "orchidaceae"] },
  { id: "/angiosperms/monocots/poaceae", name: "Poaceae (Grasses)", rank: "family", x: 410, y: 580, path: ["angiosperms", "monocots", "poaceae"] },
  { id: "/angiosperms/eudicots/ericaceae", name: "Ericaceae", rank: "family", x: 410, y: 630, path: ["angiosperms", "eudicots", "ericaceae"] },
  { id: "/angiosperms/eudicots/rosaceae", name: "Rosaceae (Rose)", rank: "family", x: 410, y: 710, path: ["angiosperms", "eudicots", "rosaceae"] },
  { id: "/angiosperms/eudicots/campanulaceae", name: "Campanulaceae", rank: "family", x: 410, y: 770, path: ["angiosperms", "eudicots", "campanulaceae"] },
  { id: "/angiosperms/eudicots/asteraceae", name: "Asteraceae", rank: "family", x: 410, y: 830, path: ["angiosperms", "eudicots", "asteraceae"] },

  // Genera levels
  { id: "/bryophytes/mosses/sphagnaceae/sphagnum", name: "Sphagnum", rank: "genus", x: 530, y: 60, path: ["bryophytes", "mosses", "sphagnaceae", "sphagnum"] },
  { id: "/bryophytes/mosses/polytrichaceae/polytrichum", name: "Polytrichum", rank: "genus", x: 530, y: 120, path: ["bryophytes", "mosses", "polytrichaceae", "polytrichum"] },
  { id: "/bryophytes/liverworts/herbertaceae/herbertus", name: "Herbertus", rank: "genus", x: 530, y: 170, path: ["bryophytes", "liverworts", "herbertaceae", "herbertus"] },
  { id: "/pteridophytes/horsetails/equisetaceae/equisetum", name: "Equisetum", rank: "genus", x: 530, y: 230, path: ["pteridophytes", "horsetails", "equisetaceae", "equisetum"] },
  { id: "/pteridophytes/ferns/dennstaedtiaceae/pteridium", name: "Pteridium", rank: "genus", x: 530, y: 290, path: ["pteridophytes", "ferns", "dennstaedtiaceae", "pteridium"] },
  { id: "/angiosperms/monocots/orchidaceae/dactylorhiza", name: "Dactylorhiza", rank: "genus", x: 530, y: 520, path: ["angiosperms", "monocots", "orchidaceae", "dactylorhiza"] },
  { id: "/angiosperms/monocots/poaceae/ammophila", name: "Ammophila", rank: "genus", x: 530, y: 580, path: ["angiosperms", "monocots", "poaceae", "ammophila"] },
  { id: "/angiosperms/eudicots/ericaceae/calluna", name: "Calluna", rank: "genus", x: 530, y: 615, path: ["angiosperms", "eudicots", "ericaceae", "calluna"] },
  { id: "/angiosperms/eudicots/ericaceae/erica", name: "Erica", rank: "genus", x: 530, y: 650, path: ["angiosperms", "eudicots", "ericaceae", "erica"] },
  { id: "/angiosperms/eudicots/rosaceae/sorbus", name: "Sorbus", rank: "genus", x: 530, y: 690, path: ["angiosperms", "eudicots", "rosaceae", "sorbus"] },
  { id: "/angiosperms/eudicots/rosaceae/rubus", name: "Rubus", rank: "genus", x: 530, y: 730, path: ["angiosperms", "eudicots", "rosaceae", "rubus"] },
  { id: "/angiosperms/eudicots/campanulaceae/campanula", name: "Campanula", rank: "genus", x: 530, y: 770, path: ["angiosperms", "eudicots", "campanulaceae", "campanula"] },
  { id: "/angiosperms/eudicots/asteraceae/cirsium", name: "Cirsium", rank: "genus", x: 530, y: 830, path: ["angiosperms", "eudicots", "asteraceae", "cirsium"] },

  // Species levels
  { id: "/bryophytes/mosses/sphagnaceae/sphagnum/capillifolium", name: "S. capillifolium", commonName: "Red Bog Moss", rank: "species", x: 670, y: 60, path: ["bryophytes", "mosses", "sphagnaceae", "sphagnum", "capillifolium"] },
  { id: "/bryophytes/mosses/polytrichaceae/polytrichum/commune", name: "P. commune", commonName: "Haircap Moss", rank: "species", x: 670, y: 120, path: ["bryophytes", "mosses", "polytrichaceae", "polytrichum", "commune"] },
  { id: "/bryophytes/liverworts/herbertaceae/herbertus/borealis", name: "H. borealis", commonName: "Northern Prongwort", rank: "species", x: 670, y: 170, path: ["bryophytes", "liverworts", "herbertaceae", "herbertus", "borealis"] },
  { id: "/pteridophytes/horsetails/equisetaceae/equisetum/sylvaticum", name: "E. sylvaticum", commonName: "Wood Horsetail", rank: "species", x: 670, y: 230, path: ["pteridophytes", "horsetails", "equisetaceae", "equisetum", "sylvaticum"] },
  { id: "/pteridophytes/ferns/dennstaedtiaceae/pteridium/aquilinum", name: "P. aquilinum", commonName: "Bracken Fern", rank: "species", x: 670, y: 290, path: ["pteridophytes", "ferns", "dennstaedtiaceae", "pteridium", "aquilinum"] },
  { id: "/gymnosperms/pinaceae/pinus/sylvestris", name: "P. sylvestris", commonName: "Scots Pine", rank: "species", x: 670, y: 350, path: ["gymnosperms", "pinaceae", "pinus", "sylvestris"] },
  { id: "/gymnosperms/cupressaceae/juniperus/communis", name: "J. communis", commonName: "Common Juniper", rank: "species", x: 670, y: 410, path: ["gymnosperms", "cupressaceae", "juniperus", "communis"] },
  { id: "/gymnosperms/taxaceae/taxus/baccata", name: "T. baccata", commonName: "European Yew", rank: "species", x: 670, y: 470, path: ["gymnosperms", "taxaceae", "taxus", "baccata"] },
  { id: "/angiosperms/monocots/orchidaceae/dactylorhiza/purpurella", name: "D. purpurella", commonName: "Marsh Orchid", rank: "species", x: 670, y: 520, path: ["angiosperms", "monocots", "orchidaceae", "dactylorhiza", "purpurella"] },
  { id: "/angiosperms/monocots/poaceae/ammophila/arenaria", name: "A. arenaria", commonName: "Marram Grass", rank: "species", x: 670, y: 580, path: ["angiosperms", "monocots", "poaceae", "ammophila", "arenaria"] },
  { id: "/angiosperms/eudicots/ericaceae/calluna/vulgaris", name: "C. vulgaris", commonName: "Heather / Ling", rank: "species", x: 670, y: 615, path: ["angiosperms", "eudicots", "ericaceae", "calluna", "vulgaris"] },
  { id: "/angiosperms/eudicots/ericaceae/erica/tetralix", name: "E. tetralix", commonName: "Cross-leaved Heath", rank: "species", x: 670, y: 650, path: ["angiosperms", "eudicots", "ericaceae", "erica", "tetralix"] },
  { id: "/angiosperms/eudicots/rosaceae/sorbus/aucuparia", name: "S. aucuparia", commonName: "Rowan Tree", rank: "species", x: 670, y: 690, path: ["angiosperms", "eudicots", "rosaceae", "sorbus", "aucuparia"] },
  { id: "/angiosperms/eudicots/rosaceae/rubus/chamaemorus", name: "R. chamaemorus", commonName: "Cloudberry", rank: "species", x: 670, y: 730, path: ["angiosperms", "eudicots", "rosaceae", "rubus", "chamaemorus"] },
  { id: "/angiosperms/eudicots/campanulaceae/campanula/rotundifolia", name: "C. rotundifolia", commonName: "Harebell", rank: "species", x: 670, y: 770, path: ["angiosperms", "eudicots", "campanulaceae", "campanula", "rotundifolia"] },
  { id: "/angiosperms/eudicots/asteraceae/cirsium/vulgare", name: "C. vulgare", commonName: "Spear Thistle", rank: "species", x: 670, y: 830, path: ["angiosperms", "eudicots", "asteraceae", "cirsium", "vulgare"] }
];

const links: VisualLink[] = [
  { from: "/", to: "/bryophytes" },
  { from: "/", to: "/pteridophytes" },
  { from: "/", to: "/gymnosperms" },
  { from: "/", to: "/angiosperms" },

  { from: "/bryophytes", to: "/bryophytes/mosses" },
  { from: "/bryophytes", to: "/bryophytes/liverworts" },

  { from: "/bryophytes/mosses", to: "/bryophytes/mosses/sphagnaceae" },
  { from: "/bryophytes/mosses", to: "/bryophytes/mosses/polytrichaceae" },
  { from: "/bryophytes/liverworts", to: "/bryophytes/liverworts/herbertaceae" },

  { from: "/bryophytes/mosses/sphagnaceae", to: "/bryophytes/mosses/sphagnaceae/sphagnum" },
  { from: "/bryophytes/mosses/polytrichaceae", to: "/bryophytes/mosses/polytrichaceae/polytrichum" },
  { from: "/bryophytes/liverworts/herbertaceae", to: "/bryophytes/liverworts/herbertaceae/herbertus" },

  { from: "/bryophytes/mosses/sphagnaceae/sphagnum", to: "/bryophytes/mosses/sphagnaceae/sphagnum/capillifolium" },
  { from: "/bryophytes/mosses/polytrichaceae/polytrichum", to: "/bryophytes/mosses/polytrichaceae/polytrichum/commune" },
  { from: "/bryophytes/liverworts/herbertaceae/herbertus", to: "/bryophytes/liverworts/herbertaceae/herbertus/borealis" },

  { from: "/pteridophytes", to: "/pteridophytes/horsetails" },
  { from: "/pteridophytes", to: "/pteridophytes/ferns" },

  { from: "/pteridophytes/horsetails", to: "/pteridophytes/horsetails/equisetaceae" },
  { from: "/pteridophytes/ferns", to: "/pteridophytes/ferns/dennstaedtiaceae" },

  { from: "/pteridophytes/horsetails/equisetaceae", to: "/pteridophytes/horsetails/equisetaceae/equisetum" },
  { from: "/pteridophytes/ferns/dennstaedtiaceae", to: "/pteridophytes/ferns/dennstaedtiaceae/pteridium" },

  { from: "/pteridophytes/horsetails/equisetaceae/equisetum", to: "/pteridophytes/horsetails/equisetaceae/equisetum/sylvaticum" },
  { from: "/pteridophytes/ferns/dennstaedtiaceae/pteridium", to: "/pteridophytes/ferns/dennstaedtiaceae/pteridium/aquilinum" },

  { from: "/gymnosperms", to: "/gymnosperms/pinaceae" },
  { from: "/gymnosperms", to: "/gymnosperms/cupressaceae" },
  { from: "/gymnosperms", to: "/gymnosperms/taxaceae" },

  { from: "/gymnosperms/pinaceae", to: "/gymnosperms/pinaceae/pinus" },
  { from: "/gymnosperms/cupressaceae", to: "/gymnosperms/cupressaceae/juniperus" },
  { from: "/gymnosperms/taxaceae", to: "/gymnosperms/taxaceae/taxus" },

  { from: "/gymnosperms/pinaceae/pinus", to: "/gymnosperms/pinaceae/pinus/sylvestris" },
  { from: "/gymnosperms/cupressaceae/juniperus", to: "/gymnosperms/cupressaceae/juniperus/communis" },
  { from: "/gymnosperms/taxaceae/taxus", to: "/gymnosperms/taxaceae/taxus/baccata" },

  { from: "/angiosperms", to: "/angiosperms/monocots" },
  { from: "/angiosperms", to: "/angiosperms/eudicots" },

  { from: "/angiosperms/monocots", to: "/angiosperms/monocots/orchidaceae" },
  { from: "/angiosperms/monocots", to: "/angiosperms/monocots/poaceae" },

  { from: "/angiosperms/monocots/orchidaceae", to: "/angiosperms/monocots/orchidaceae/dactylorhiza" },
  { from: "/angiosperms/monocots/poaceae", to: "/angiosperms/monocots/poaceae/ammophila" },

  { from: "/angiosperms/monocots/orchidaceae/dactylorhiza", to: "/angiosperms/monocots/orchidaceae/dactylorhiza/purpurella" },
  { from: "/angiosperms/monocots/poaceae/ammophila", to: "/angiosperms/monocots/poaceae/ammophila/arenaria" },

  { from: "/angiosperms/eudicots", to: "/angiosperms/eudicots/ericaceae" },
  { from: "/angiosperms/eudicots", to: "/angiosperms/eudicots/rosaceae" },
  { from: "/angiosperms/eudicots", to: "/angiosperms/eudicots/campanulaceae" },
  { from: "/angiosperms/eudicots", to: "/angiosperms/eudicots/asteraceae" },

  { from: "/angiosperms/eudicots/ericaceae", to: "/angiosperms/eudicots/ericaceae/calluna" },
  { from: "/angiosperms/eudicots/ericaceae", to: "/angiosperms/eudicots/ericaceae/erica" },
  { from: "/angiosperms/eudicots/rosaceae", to: "/angiosperms/eudicots/rosaceae/sorbus" },
  { from: "/angiosperms/eudicots/rosaceae", to: "/angiosperms/eudicots/rosaceae/rubus" },
  { from: "/angiosperms/eudicots/campanulaceae", to: "/angiosperms/eudicots/campanulaceae/campanula" },
  { from: "/angiosperms/eudicots/asteraceae", to: "/angiosperms/eudicots/asteraceae/cirsium" },

  { from: "/angiosperms/eudicots/ericaceae/calluna", to: "/angiosperms/eudicots/ericaceae/calluna/vulgaris" },
  { from: "/angiosperms/eudicots/ericaceae/erica", to: "/angiosperms/eudicots/ericaceae/erica/tetralix" },
  { from: "/angiosperms/eudicots/rosaceae/sorbus", to: "/angiosperms/eudicots/rosaceae/sorbus/aucuparia" },
  { from: "/angiosperms/eudicots/rosaceae/rubus", to: "/angiosperms/eudicots/rosaceae/rubus/chamaemorus" },
  { from: "/angiosperms/eudicots/campanulaceae/campanula", to: "/angiosperms/eudicots/campanulaceae/campanula/rotundifolia" },
  { from: "/angiosperms/eudicots/asteraceae/cirsium", to: "/angiosperms/eudicots/asteraceae/cirsium/vulgare" }
];

interface VisualTreeProps {
  pathSegments: string[];
  setPathSegments: (segments: string[]) => void;
}

export default function VisualTree({ pathSegments, setPathSegments }: VisualTreeProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  // Helper to check if a node ID matches or is a parent of the active path
  const isNodeActive = (id: string) => {
    if (id === "/") return true;
    const currentPathStr = "/" + pathSegments.join("/");
    return currentPathStr === id || currentPathStr.startsWith(id + "/");
  };

  // Helper to check if a link is completely on the active evolutionary path
  const isLinkActive = (from: string, to: string) => {
    return isNodeActive(from) && isNodeActive(to);
  };

  // Helper to check if a node is the EXACT active node
  const isExactActive = (id: string) => {
    const currentPathStr = "/" + pathSegments.join("/");
    return currentPathStr === id;
  };

  // Automatically scroll the tree view to keep the active node centered vertically
  useEffect(() => {
    if (!containerRef.current) return;
    const currentPathStr = "/" + pathSegments.join("/");
    const activeNodeObj = nodes.find(n => n.id === currentPathStr);
    if (activeNodeObj) {
      const scrollContainer = containerRef.current;
      const targetY = activeNodeObj.y - scrollContainer.clientHeight / 2;
      scrollContainer.scrollTo({
        top: Math.max(0, targetY),
        behavior: "smooth"
      });
    }
  }, [pathSegments]);

  const handleNodeClick = (node: VisualNode) => {
    setPathSegments(node.path);
  };

  return (
    <div className="flex flex-col h-[520px] md:h-[600px] rounded-xl bg-slate-900 border border-slate-800 shadow-xl overflow-hidden">
      {/* Panel Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-slate-950 border-b border-slate-800 select-none">
        <div className="flex items-center gap-2 text-emerald-400">
          <GitBranch className="w-4 h-4" />
          <span className="text-xs font-semibold text-slate-300 tracking-wider uppercase">Interactive Tree of Life</span>
        </div>
        <div className="text-[10px] text-slate-500 font-mono">
          Click any node to navigate
        </div>
      </div>

      {/* SVG Canvas Scroll Area */}
      <div
        ref={containerRef}
        className="flex-1 overflow-auto bg-gradient-to-br from-slate-950 to-slate-900 p-4 relative"
      >
        {/* Timeline Sidebar (Visual indicator only) */}
        <div className="absolute left-1 top-0 bottom-0 w-8 border-r border-slate-800/40 flex flex-col justify-between text-[8px] font-mono text-slate-600 py-12 pointer-events-none z-10 select-none">
          <div className="flex flex-col items-center rotate-270 leading-none">
            <Calendar className="w-2.5 h-2.5 mb-1" />
            <span>PRECAMBRIAN ~1.2 Bya</span>
          </div>
          <div className="flex flex-col items-center rotate-270 leading-none">
            <span>ORDOVICIAN ~470 Mya</span>
          </div>
          <div className="flex flex-col items-center rotate-270 leading-none">
            <span>CARBONIFEROUS ~350 Mya</span>
          </div>
          <div className="flex flex-col items-center rotate-270 leading-none">
            <span>CRETACEOUS ~140 Mya</span>
          </div>
          <div className="flex flex-col items-center rotate-270 leading-none">
            <span>MIOCENE ~15 Mya</span>
          </div>
        </div>

        {/* SVG Drawing */}
        <svg
          width="760"
          height="880"
          className="ml-8 z-0 overflow-visible"
        >
          {/* Defs for gradients, glows, and custom markers */}
          <defs>
            <filter id="active-glow" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="3" result="blur" />
              <feComposite in="SourceGraphic" in2="blur" operator="over" />
            </filter>
            <linearGradient id="active-grad" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stopColor="#10b981" />
              <stop offset="100%" stopColor="#34d399" />
            </linearGradient>
          </defs>

          {/* Render Connections (Links) */}
          {links.map((link, idx) => {
            const fromNode = nodes.find(n => n.id === link.from);
            const toNode = nodes.find(n => n.id === link.to);
            if (!fromNode || !toNode) return null;

            const isActive = isLinkActive(link.from, link.to);
            
            // Draw smooth bezier curves or clean step-lines
            // Using a step-horizontal path looks very structured and phylogenetically correct!
            const midX = (fromNode.x + toNode.x) / 2;
            const pathData = `M ${fromNode.x} ${fromNode.y} 
                             C ${midX} ${fromNode.y}, ${midX} ${toNode.y}, ${toNode.x} ${toNode.y}`;

            return (
              <g key={`link-${idx}`}>
                {/* Glow under active lines */}
                {isActive && (
                  <path
                    d={pathData}
                    fill="none"
                    stroke="#10b981"
                    strokeWidth="4"
                    strokeOpacity="0.4"
                    filter="url(#active-glow)"
                    className="animate-pulse"
                  />
                )}
                {/* Main line */}
                <path
                  d={pathData}
                  fill="none"
                  stroke={isActive ? "url(#active-grad)" : "#334155"}
                  strokeWidth={isActive ? "2" : "1"}
                  strokeOpacity={isActive ? "0.9" : "0.3"}
                  transition="stroke 0.3s, stroke-width 0.3s"
                />
              </g>
            );
          })}

          {/* Render Nodes */}
          {nodes.map((node) => {
            const isActive = isNodeActive(node.id);
            const isExact = isExactActive(node.id);
            const isSpecies = node.rank === "species";

            // Style definitions
            let nodeColor = "fill-slate-800 stroke-slate-600";
            let textColor = "fill-slate-400 font-normal";
            let radius = 4;

            if (isActive) {
              textColor = "fill-emerald-300 font-medium";
              radius = 5.5;
              if (node.rank === "clade") nodeColor = "fill-teal-500 stroke-teal-300";
              else if (node.rank === "class") nodeColor = "fill-purple-500 stroke-purple-300";
              else if (node.rank === "family") nodeColor = "fill-sky-500 stroke-sky-300";
              else if (node.rank === "genus") nodeColor = "fill-amber-500 stroke-amber-300";
              else if (node.rank === "species") nodeColor = "fill-green-400 stroke-green-200";
            }

            if (isExact) {
              radius = 7.5;
              nodeColor = "fill-emerald-400 stroke-white ring-4 ring-emerald-500/20";
              textColor = "fill-white font-bold";
            }

            return (
              <g
                key={node.id}
                className="cursor-pointer group select-none"
                onClick={() => handleNodeClick(node)}
              >
                {/* Hover Aura */}
                <circle
                  cx={node.x}
                  cy={node.y}
                  r={radius + 6}
                  className="fill-transparent group-hover:fill-emerald-500/10 transition-colors"
                />

                {/* Node Dot */}
                <circle
                  cx={node.x}
                  cy={node.y}
                  r={radius}
                  className={`${nodeColor} transition-all duration-300 shadow-lg`}
                  filter={isExact ? "url(#active-glow)" : ""}
                />

                {/* Glowing ring for active node */}
                {isExact && (
                  <circle
                    cx={node.x}
                    cy={node.y}
                    r={radius + 4}
                    fill="none"
                    stroke="#10b981"
                    strokeWidth="1.5"
                    className="animate-ping opacity-60"
                  />
                )}

                {/* Node Label Text */}
                <text
                  x={node.x + 12}
                  y={node.y + 3.5}
                  className={`${textColor} text-[9.5px] font-mono group-hover:fill-emerald-200 transition-colors`}
                >
                  {node.name}
                  {isSpecies && node.commonName && (
                    <tspan className="fill-slate-500 text-[8.5px] font-sans">
                      {" "}({node.commonName})
                    </tspan>
                  )}
                </text>
              </g>
            );
          })}
        </svg>
      </div>

      {/* Mini Legend Panel */}
      <div className="flex justify-around items-center px-4 py-2 bg-slate-950 border-t border-slate-800 text-[9px] font-mono text-slate-500 select-none">
        <div className="flex items-center gap-1">
          <span className="w-2.5 h-2.5 rounded-full bg-teal-500" />
          <span>Clade</span>
        </div>
        <div className="flex items-center gap-1">
          <span className="w-2.5 h-2.5 rounded-full bg-purple-500" />
          <span>Class</span>
        </div>
        <div className="flex items-center gap-1">
          <span className="w-2.5 h-2.5 rounded-full bg-sky-500" />
          <span>Family</span>
        </div>
        <div className="flex items-center gap-1">
          <span className="w-2.5 h-2.5 rounded-full bg-amber-500" />
          <span>Genus</span>
        </div>
        <div className="flex items-center gap-1">
          <span className="w-2.5 h-2.5 rounded-full bg-green-400" />
          <span>Species</span>
        </div>
      </div>
    </div>
  );
}
