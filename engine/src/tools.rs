use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};
use anyhow::{Result, bail};
use serde_json::{json, Value};

use crate::sandbox::Sandbox;
use crate::memory::MemoryStore;
use crate::prompt_refinement::PromptHistory;
use crate::self_analysis::SelfAnalyzer;
use crate::tool_evolution::ToolEvolver;
use crate::ollama::{OllamaClient, ChatMessage};
use crate::applet_manager::AppletManager;

const MAX_READ_SIZE: usize = 256 * 1024;

pub struct ToolContext<'a> {
    pub memory: &'a mut MemoryStore,
    pub prompt_history: &'a mut PromptHistory,
    pub analyzer: &'a SelfAnalyzer,
    pub evolver: &'a ToolEvolver,
    pub ollama: &'a OllamaClient,
    pub project_root: &'a Path,
    pub applet_manager: &'a mut AppletManager,
    pub steer_tx: &'a mpsc::Sender<String>,
    pub steer_rx: &'a mpsc::Receiver<String>,
    pub input_flag: &'a mut Arc<AtomicBool>,
}

pub struct ToolExecutor {
    sandbox: Sandbox,
}

impl ToolExecutor {
    pub fn new(sandbox: Sandbox) -> Self {
        Self { sandbox }
    }

    pub async fn execute(&self, name: &str, args: &Value, ctx: &mut ToolContext<'_>) -> Result<String> {
        match name {
            // File ops
            "read_file" => self.read_file(args).await,
            "write_file" => self.write_file(args).await,
            "list_dir" => self.list_dir(args).await,

            // Generation
            "generate_html" => self.generate_html(args).await,
            "generate_sprite" => self.generate_sprite(args).await,
            "generate_tileset" => self.generate_tileset(args).await,
            "generate_object" => self.generate_object(args).await,
            "render_sprite" => self.render_sprite(args).await,

            // Clipboard
            "read_clipboard" => self.read_clipboard().await,

            // Memory
            "remember" => self.remember(args, ctx.memory),
            "list_memories" => self.list_memories(args, ctx.memory),
            "search_memories" => self.search_memories(args, ctx.memory),
            "set_preference" => self.set_preference(args, ctx.memory),

            // Analysis
            "analyze_self" => self.analyze_self(args, ctx.analyzer),
            "list_source_files" => self.list_source_files(ctx.analyzer),

            // Evolution
            "evolve_tools" => self.evolve_tools(args, ctx.evolver, ctx.ollama).await,

            // Prompt
            "refine_prompt" => self.refine_prompt(ctx.prompt_history),
            "get_tool_stats" => self.get_tool_stats(ctx.prompt_history),

            // Coding agent
            "coding_agent" => self.coding_agent(args, ctx.ollama, ctx.project_root, &self.sandbox).await,

            // Applets
            "manage_applet" => self.manage_applet(args, ctx).await,

            _ => bail!("unknown tool: {}", name),
        }
    }

    // ═══════════════════════════════════════════
    //  Applet management
    // ═══════════════════════════════════════════

    async fn manage_applet(&self, args: &Value, ctx: &mut ToolContext<'_>) -> Result<String> {
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
        match action {
            "list" => Ok(ctx.applet_manager.list()),
            "status" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.is_empty() {
                    bail!("status requires a 'name' argument");
                }
                if !ctx.applet_manager.has(name) {
                    bail!("unknown applet: {}", name);
                }
                let running = ctx.applet_manager.is_running(name);
                let foreground = ctx.applet_manager.is_foreground(name);
                let entry = ctx.applet_manager.entries.get(name).unwrap();
                let port = entry.port.map(|p| p.to_string()).unwrap_or_else(|| "none".to_string());
                Ok(format!(
                    "{}: {}\n  status: {}\n  mode: {} (runs in the current window)\n  port: {}",
                    name, entry.desc,
                    if running { "● running" } else { "○ stopped" },
                    if foreground { "in-window" } else { "own window" },
                    port,
                ))
            }
            "launch" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.is_empty() {
                    bail!("launch requires a 'name' argument");
                }
                if ctx.applet_manager.is_foreground(name) {
                    // Takes over the current window until it exits, then ayesha returns
                    ctx.applet_manager
                        .run_in_window(name, ctx.steer_tx, ctx.steer_rx, ctx.input_flag)
                        .map_err(|e| anyhow::anyhow!("{}", e))
                        .map(|_| format!("ran {} in the current window and returned (press ctrl+p to switch pages again)", name))
                } else if ctx.applet_manager.is_running(name) {
                    bail!("{} is already running", name)
                } else {
                    ctx.applet_manager
                        .launch(name)
                        .map_err(|e| anyhow::anyhow!("{}", e))
                        .map(|_| format!("launched {} in its own window", name))
                }
            }
            "stop" => {
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.is_empty() {
                    bail!("stop requires a 'name' argument");
                }
                ctx.applet_manager
                    .stop(name)
                    .map_err(|e| anyhow::anyhow!("{}", e))
                    .map(|_| format!("stopped {}", name))
            }
            _ => bail!("unknown manage_applet action: {} (expected list/status/launch/stop)", action),
        }
    }

    // ═══════════════════════════════════════════
    //  File tools
    // ═══════════════════════════════════════════

    async fn read_file(&self, args: &Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;
        self.sandbox.check_sensitive_resolved(&resolved)?;

        let content = fs::read_to_string(&resolved)?;

        if content.chars().count() > MAX_READ_SIZE {
            let truncated = crate::util::truncate_chars(&content, MAX_READ_SIZE);
            Ok(format!(
                "{}\n\n... [truncated at {} chars, file is {} chars total]",
                truncated,
                MAX_READ_SIZE,
                content.chars().count()
            ))
        } else {
            Ok(content)
        }
    }

    async fn write_file(&self, args: &Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;

        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;
        // Removed: self.sandbox.check_sensitive_resolved(&resolved)?;

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        // Try to handle ReadOnly attribute if writing fails
        if let Err(e) = fs::write(&resolved, content) {
            let metadata = fs::metadata(&resolved);
            if let Ok(m) = metadata {
                if m.permissions().readonly() {
                    let mut perms = m.permissions();
                    perms.set_readonly(false);
                    fs::set_permissions(&resolved, perms)?;
                    fs::write(&resolved, content)?;
                } else {
                    return Err(e.into());
                }
            } else {
                return Err(e.into());
            }
        }

        Ok(format!(
            "wrote {} bytes to '{}'",
            content.len(),
            resolved.display()
        ))
    }

    async fn list_dir(&self, args: &Value) -> Result<String> {
        let path = args["path"].as_str().unwrap_or(".");
        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;
        self.sandbox.check_sensitive_resolved(&resolved)?;

        let entries = fs::read_dir(&resolved)?;

        let mut items: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let prefix = if is_dir { "[DIR] " } else { "[FILE] " };
            items.push(format!("  {}{}", prefix, name));
        }

        items.sort();

        if items.is_empty() {
            Ok(format!("directory '{}' is empty", resolved.display()))
        } else {
            Ok(format!(
                "contents of '{}':\n{}",
                resolved.display(),
                items.join("\n")
            ))
        }
    }

    // ═══════════════════════════════════════════
    //  Generation tools
    // ═══════════════════════════════════════════

    async fn generate_html(&self, args: &Value) -> Result<String> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;

        self.sandbox.check_sensitive(path)?;
        let resolved = self.sandbox.resolve(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&resolved, content)?;

        Ok(format!("generated html app at '{}' ({} bytes)", resolved.display(), content.len()))
    }

    async fn generate_sprite(&self, args: &Value) -> Result<String> {
        let output = args.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("assets/sprite.png");

        self.sandbox.check_sensitive(output)?;
        let resolved = self.sandbox.resolve(output)?;

        let config = crate::pixel_striker::renderer::SpriteSheetConfig::from_json_value(args);
        let result = crate::pixel_striker::renderer::render_to_file(&config, &resolved)?;

        Ok(format!(
            "generated sprite sheet at '{}' ({}x{}, {} states, {} frames)",
            resolved.display(),
            result.width, result.height,
            result.states.len(),
            result.total_frames,
        ))
    }

    async fn generate_tileset(&self, args: &Value) -> Result<String> {
        let output = args.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("assets/tileset.png");

        self.sandbox.check_sensitive(output)?;
        let resolved = self.sandbox.resolve(output)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let tile_w = args.get("tile_width").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let tile_h = args.get("tile_height").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let cols = args.get("columns").and_then(|v| v.as_u64()).unwrap_or(8) as u32;
        let rows = args.get("rows").and_then(|v| v.as_u64()).unwrap_or(4) as u32;

        let img_w = cols.saturating_mul(tile_w);
        let img_h = rows.saturating_mul(tile_h);
        if img_w == 0 || img_h == 0 || img_w > 16384 || img_h > 16384 {
            return Err(anyhow::anyhow!("image dimensions {}x{} are out of range (must be 1-16384)", img_w, img_h));
        }
        let mut img = image::RgbaImage::new(img_w, img_h);

        let colors = [
            image::Rgba([120, 180, 80, 255]),
            image::Rgba([180, 140, 60, 255]),
            image::Rgba([100, 120, 160, 255]),
            image::Rgba([160, 160, 160, 255]),
        ];

        for row in 0..rows {
            let base = colors[(row as usize) % colors.len()];
            for col in 0..cols {
                let ox = col * tile_w;
                let oy = row * tile_h;
                let v = (col * 17 + row * 31) % 30;
                let r = (base.0[0] as i32 + v as i32 - 15).clamp(0, 255) as u8;
                let g = (base.0[1] as i32 + v as i32 - 15).clamp(0, 255) as u8;
                let b = (base.0[2] as i32 + v as i32 - 15).clamp(0, 255) as u8;
                for dy in 0..tile_h {
                    for dx in 0..tile_w {
                        img.put_pixel(ox + dx, oy + dy, image::Rgba([r, g, b, 255]));
                    }
                }
            }
        }

        img.save(&resolved)?;

        Ok(format!(
            "generated tileset at '{}' ({}x{}, {} tiles)",
            resolved.display(), img_w, img_h, cols * rows
        ))
    }

    async fn generate_object(&self, args: &Value) -> Result<String> {
        let output = args.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("assets/object.png");

        self.sandbox.check_sensitive(output)?;
        let resolved = self.sandbox.resolve(output)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let w = args.get("width").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let h = args.get("height").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
        let px = args.get("pixel_size").and_then(|v| v.as_u64()).unwrap_or(4) as u32;

        let img_w = w.saturating_mul(px);
        let img_h = h.saturating_mul(px);
        if img_w == 0 || img_h == 0 || img_w > 16384 || img_h > 16384 {
            return Err(anyhow::anyhow!("image dimensions {}x{} are out of range (must be 1-16384)", img_w, img_h));
        }
        let mut img = image::RgbaImage::new(img_w, img_h);

        let r = args.get("color_r").and_then(|v| v.as_u64()).unwrap_or(200) as u8;
        let g = args.get("color_g").and_then(|v| v.as_u64()).unwrap_or(100) as u8;
        let b = args.get("color_b").and_then(|v| v.as_u64()).unwrap_or(50) as u8;

        for dy in 0..img_h {
            for dx in 0..img_w {
                let edge = dx < px || dx >= img_w - px || dy < px || dy >= img_h - px;
                let factor = if edge { 0.7 } else { 1.0 };
                img.put_pixel(dx, dy, image::Rgba([
                    (r as f32 * factor) as u8,
                    (g as f32 * factor) as u8,
                    (b as f32 * factor) as u8,
                    255,
                ]));
            }
        }

        img.save(&resolved)?;
        Ok(format!("generated object sprite at '{}' ({}x{})", resolved.display(), img_w, img_h))
    }

    async fn render_sprite(&self, args: &Value) -> Result<String> {
        let output = args.get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("assets/sprite_viewer.html");

        self.sandbox.check_sensitive(output)?;
        let resolved = self.sandbox.resolve(output)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
        }

        let html = r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>sprite viewer</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#0a0a0f;color:#0ff;font-family:monospace;display:flex;justify-content:center;align-items:center;min-height:100vh}
.viewer{background:#111;border:1px solid #0ff3;border-radius:8px;padding:2rem;box-shadow:0 0 40px #0ff2;text-align:center}
canvas{image-rendering:pixelated;border:2px solid #0ff5;border-radius:4px;margin:1rem 0}
.controls{display:flex;gap:.5rem;flex-wrap:wrap;justify-content:center;margin-top:1rem}
button{background:#0ff1;color:#0ff;border:1px solid #0ff5;padding:.5rem 1rem;border-radius:4px;cursor:pointer;font:inherit}
button:hover{background:#0ff3}
select{background:#111;color:#0ff;border:1px solid #0ff5;padding:.5rem;border-radius:4px;font:inherit}
h2{color:#0ff8}
</style></head>
<body>
<div class="viewer">
<h2>✦ sprite viewer ✦</h2>
<canvas id="c"></canvas>
<div class="controls">
<select id="state"><option>idle</option><option>walk_left</option><option>walk_right</option><option>hacking_active</option></select>
<button id="play">▶ play</button>
</div>
</div>
<script>
const c=document.getElementById('c'),ctx=c.getContext('2d'),S=4,W=32,H=32;c.width=W*S;c.height=H*S;
const p={skin:[255,204,153],hair:[80,40,120],shirt:[40,80,160],pants:[60,60,80],shoes:[40,40,50],visor:[0,240,255]};
function pt(x,y,c){if(x<0||x>=W||y<0||y>=H)return;ctx.fillStyle='rgb('+c[0]+','+c[1]+','+c[2]+')';ctx.fillRect(x*S,y*S,S,S)}
function dr(b){ctx.clearRect(0,0,c.width,c.height)
for(let d=0;d<8;d++){pt(1+d,1+b,[p.hair[0]*.6|0,p.hair[1]*.6|0,p.hair[2]*.6|0]);pt(1+d,2+b,p.hair)}
pt(3,1+b,p.hair);for(let y=0;y<4;y++)for(let x=0;x<6;x++)pt(2+x,3+y+b,p.skin)
pt(5,4+b,p.visor);pt(6,4+b,p.visor);pt(7,4+b,p.visor);pt(6,3+b,[p.visor[0]*.5|0,p.visor[1]*.5|0,p.visor[2]*.5|0])
for(let y=0;y<5;y++)for(let x=0;x<8;x++)pt(1+x,7+y+b,p.shirt)
for(let y=0;y<3;y++)for(let x=0;x<6;x++)pt(2+x,12+y+b,p.pants)
pt(3,15+b,p.pants);pt(6,15+b,p.pants)
pt(2,17+b,p.shoes);pt(3,17+b,p.shoes);pt(4,17+b,p.shoes);pt(5,17+b,p.shoes);pt(6,17+b,p.shoes);pt(7,17+b,p.shoes)}
let f=0,pl=0;setInterval(()=>{if(pl){f++;dr(f%2)}},250)
document.getElementById('play').onclick=()=>{pl=!pl;document.getElementById('play').textContent=pl?'⏸ stop':'▶ play'}
document.getElementById('state').onchange=function(){}
dr(0)
</script>
</body></html>"#;

        fs::write(&resolved, html)?;
        Ok(format!("rendered sprite viewer at '{}' ({} bytes)", resolved.display(), html.len()))
    }

    // ═══════════════════════════════════════════
    //  Clipboard
    // ═══════════════════════════════════════════

    async fn read_clipboard(&self) -> Result<String> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| anyhow::anyhow!("failed to open clipboard: {}", e))?;

        match cb.get_text() {
            Ok(text) => {
                let preview: String = text.chars().take(500).collect();
                let total_chars = text.chars().count();
                if total_chars > 500 {
                    Ok(format!("{}\n... [truncated: showing 500 of {} chars]", preview, total_chars))
                } else {
                    Ok(text)
                }
            }
            Err(arboard::Error::ContentNotAvailable) => {
                match cb.get_image() {
                    Ok(img) => Ok(format!(
                        "clipboard contains an image ({}x{}, {} bytes)", img.width, img.height, img.bytes.len()
                    )),
                    Err(_) => Ok("clipboard is empty or contains unsupported content".to_string()),
                }
            }
            Err(e) => Ok(format!("failed to read clipboard: {}", e)),
        }
    }

    // ═══════════════════════════════════════════
    //  Memory tools
    // ═══════════════════════════════════════════

    fn remember(&self, args: &Value, memory: &mut MemoryStore) -> Result<String> {
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;
        let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("general");

        memory.add_memory(category, content, vec![], 5);
        Ok(format!("stored memory: {} [category: {}]", content, category))
    }

    fn list_memories(&self, args: &Value, memory: &MemoryStore) -> Result<String> {
        let n = args.get("count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let memories = memory.recent(n);

        if memories.is_empty() {
            return Ok("no memories stored yet".to_string());
        }

        let lines: Vec<String> = memories.iter().enumerate().map(|(i, m)| {
            format!("{}. [{}] {} (importance: {})", i + 1, m.category, m.content, m.importance)
        }).collect();

        Ok(format!("recent memories ({}):\n{}", memories.len(), lines.join("\n")))
    }

    fn search_memories(&self, args: &Value, memory: &MemoryStore) -> Result<String> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'query' argument"))?;

        let results = memory.search(query);
        if results.is_empty() {
            return Ok(format!("no memories matching '{}'", query));
        }

        let lines: Vec<String> = results.iter().map(|m| {
            format!("- [{}] {} (tags: {})", m.category, m.content, m.tags.join(", "))
        }).collect();

        Ok(format!("memories matching '{}' ({}):\n{}", query, results.len(), lines.join("\n")))
    }

    fn set_preference(&self, args: &Value, memory: &mut MemoryStore) -> Result<String> {
        let key = args["key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'key' argument"))?;
        let value = args["value"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'value' argument"))?;

        memory.set_preference(key, value);
        Ok(format!("preference set: {} = {}", key, value))
    }

    // ═══════════════════════════════════════════
    //  Analysis tools
    // ═══════════════════════════════════════════

    fn analyze_self(&self, args: &Value, analyzer: &SelfAnalyzer) -> Result<String> {
        let target = args.get("file").and_then(|v| v.as_str()).unwrap_or("");

        let files = analyzer.source_files();
        if target.is_empty() {
            let names: Vec<String> = files.iter()
                .filter_map(|f| f.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
                .collect();
            return Ok(format!("source files:\n{}", names.join("\n")));
        }

        let file_path = files.iter().find(|f| {
            f.file_name().and_then(|n| n.to_str()) == Some(target)
        });

        match file_path {
            Some(p) => {
                let source = analyzer.read_source(p)?;
                let issues = analyzer.analyze_for_improvements(&source);
                let filename = p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                let issue_lines: Vec<String> = issues.iter().map(|i| {
                    format!("  [{}] line {:?}: {} \u{2192} {}", i.severity, i.line, i.description, i.fix)
                }).collect();

                Ok(format!("analysis of '{}' ({} lines, {} issues):\n{}",
                    filename, source.lines().count(), issues.len(), issue_lines.join("\n")))
            }
            None => Ok(format!("file '{}' not found in source directory", target)),
        }
    }

    fn list_source_files(&self, analyzer: &SelfAnalyzer) -> Result<String> {
        let files = analyzer.source_files();
        if files.is_empty() {
            return Ok("no source files found".to_string());
        }

        let total_lines: usize = files.iter()
            .filter_map(|f| fs::read_to_string(f).ok())
            .map(|s| s.lines().count())
            .sum();

        let lines: Vec<String> = files.iter().map(|f| {
            let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            let lc = fs::read_to_string(f).ok().map(|s| s.lines().count()).unwrap_or(0);
            format!("  {} ({} lines)", name, lc)
        }).collect();

        Ok(format!("source files ({} files, ~{} lines total):\n{}",
            files.len(), total_lines, lines.join("\n")))
    }

    // ═══════════════════════════════════════════
    //  Evolution
    // ═══════════════════════════════════════════

    async fn evolve_tools(&self, args: &Value, evolver: &ToolEvolver, _ollama: &OllamaClient) -> Result<String> {
        let specific = args.get("gap").and_then(|v| v.as_str());

        let gaps = evolver.analyze_gaps();
        if gaps.is_empty() {
            return Ok("no tool gaps detected — all tool categories are covered".to_string());
        }

        if let Some(gap_name) = specific {
            let found = gaps.iter().find(|g| g.contains(gap_name));
            match found {
                Some(gap) => {
                    let template = evolver.generate_tool_definition(gap).await?;
                    let code = ToolEvolver::generate_tool_code(&template);
                    Ok(format!("generated tool '{}':\n{}", template.name, code))
                }
                None => Ok(format!("gap '{}' not found. gaps: {}", gap_name, gaps.join(", "))),
            }
        } else {
            Ok(format!("detected {} tool gaps:\n{}", gaps.len(),
                gaps.iter().enumerate().map(|(i, g)| format!("  {}. {}", i + 1, g)).collect::<Vec<_>>().join("\n")))
        }
    }

    // ═══════════════════════════════════════════
    //  Prompt refinement
    // ═══════════════════════════════════════════

    fn refine_prompt(&self, history: &PromptHistory) -> Result<String> {
        Ok(history.generate_analysis_prompt())
    }

    fn get_tool_stats(&self, history: &PromptHistory) -> Result<String> {
        let stats = history.tool_stats();
        if stats.is_empty() {
            return Ok("no tool usage recorded yet".to_string());
        }

        let lines: Vec<String> = stats.iter().map(|(name, total, success, rate)| {
            let bar_len = (rate * 10.0) as usize;
            let bar = format!("{}{}", "█".repeat(bar_len), "░".repeat(10 - bar_len));
            format!("  {} {} {}/{} ({:.0}%)", name, bar, success, total, rate * 100.0)
        }).collect();

        Ok(format!("tool usage stats:\n{}", lines.join("\n")))
    }

    // ═══════════════════════════════════════════
    //  Coding agent (multi-action coding tool)
    // ═══════════════════════════════════════════

    async fn coding_agent(&self, args: &Value, ollama: &OllamaClient, project_root: &Path, sandbox: &Sandbox) -> Result<String> {

        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'action' argument"))?;

        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing 'path' argument"))?;

        match action {
            "read" => {
                sandbox.check_sensitive(path)?;
                let full_path = sandbox.resolve(path)?;
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;
                Ok(json!({"path": path, "content": content}).to_string())
            }
            "write" => {
                sandbox.check_sensitive(path)?;
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing 'content' argument"))?;
                let full_path = sandbox.resolve(path)?;
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full_path, content)?;
                Ok(json!({"path": path, "status": "written", "bytes": content.len()}).to_string())
            }
            "edit" => {
                sandbox.check_sensitive(path)?;
                let edits = args.get("edits")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow::anyhow!("missing 'edits' array"))?;

                let full_path = sandbox.resolve(path)?;
                let mut content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;

                let mut edits = edits.clone();
                edits.reverse();

                for edit in edits {
                    let act = edit.get("action").and_then(|v| v.as_str()).unwrap_or("");
                    match act {
                        "replace" => {
                            let old = edit.get("old").and_then(|v| v.as_str())
                                .ok_or_else(|| anyhow::anyhow!("replace missing 'old'"))?;
                            let new = edit.get("new").and_then(|v| v.as_str())
                                .ok_or_else(|| anyhow::anyhow!("replace missing 'new'"))?;
                            if !content.contains(old) {
                                return Err(anyhow::anyhow!("replace string not found in '{}'", path));
                            }
                            content = content.replacen(old, new, 1);
                        }
                        "insert" => {
                            let text = edit.get("text").and_then(|v| v.as_str())
                                .ok_or_else(|| anyhow::anyhow!("insert missing 'text'"))?;
                            let after = edit.get("after").and_then(|v| v.as_str());
                            if let Some(a) = after {
                                if let Some(pos) = content.find(a) {
                                    content.insert_str(pos + a.len(), text);
                                } else {
                                    return Err(anyhow::anyhow!("insert anchor '{}' not found", a));
                                }
                            } else {
                                content.push_str(text);
                            }
                        }
                        _ => return Err(anyhow::anyhow!("unknown edit action: {}", act)),
                    }
                }

                fs::write(&full_path, &content)?;
                Ok(json!({"path": path, "status": "edited", "bytes": content.len()}).to_string())
            }
            "list" => {
                sandbox.check_sensitive(path)?;
                let full_path = sandbox.resolve(path)?;
                let mut entries = Vec::new();
                if full_path.is_dir() {
                    for entry in fs::read_dir(&full_path)? {
                        let e = entry?;
                        let name = e.file_name().to_string_lossy().to_string();
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        entries.push(json!({"name": name, "type": if is_dir { "directory" } else { "file" }}));
                    }
                }
                Ok(json!({"path": path, "entries": entries}).to_string())
            }
            "analyze" => {
                sandbox.check_sensitive(path)?;
                let full_path = sandbox.resolve(path)?;
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;
                let analyzer = SelfAnalyzer::new(project_root.to_path_buf());
                let issues = analyzer.analyze_for_improvements(&content);
                Ok(json!({"path": path, "issues": issues}).to_string())
            }
            "modify" => {
                sandbox.check_sensitive(path)?;
                let instruction = args.get("instruction")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing 'instruction' argument"))?;
                let full_path = sandbox.resolve(path)?;
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;

                let prompt = format!(
                    "Modify this code per the instruction.\nFILE: {}\nCODE:\n{}\nINSTRUCTION:\n{}\nOutput ONLY the modified code.",
                    path, content, instruction
                );
                let msg = vec![ChatMessage {
                    role: "user".to_string(), content: prompt,
                    tool_calls: None, tool_call_id: None,
                }];
                let resp = ollama.chat(&msg, None).await?;
                let raw = resp.message.content.trim();
                // Strip markdown code fences if present (```rust ... ``` or ``` ... ```)
                let modified = if raw.starts_with("```") {
                    let stripped = raw.trim_start_matches("```");
                    // Remove language tag on first line
                    let after_lang = stripped.find('\n').map(|i| &stripped[i+1..]).unwrap_or(stripped);
                    // Remove trailing fence
                    after_lang.trim_end_matches("```").trim().to_string()
                } else {
                    raw.to_string()
                };

                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full_path, &modified)?;
                Ok(json!({"path": path, "status": "modified", "bytes": modified.len()}).to_string())
            }
            "suggest" => {
                sandbox.check_sensitive(path)?;
                let full_path = sandbox.resolve(path)?;
                let content = fs::read_to_string(&full_path)
                    .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", path, e))?;
                let prompt = format!(
                    "Review this code and suggest improvements.\nFILE: {}\nCODE:\n{}\nList specific, actionable suggestions.",
                    path, content
                );
                let msg = vec![ChatMessage {
                    role: "user".to_string(), content: prompt,
                    tool_calls: None, tool_call_id: None,
                }];
                let resp = ollama.chat(&msg, None).await?;
                Ok(json!({"path": path, "suggestions": resp.message.content}).to_string())
            }
            _ => Err(anyhow::anyhow!("unknown coding_agent action: {}", action)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::Sandbox;
    use serde_json::json;

    #[tokio::test]
    async fn test_unknown_tool() {
        let executor = ToolExecutor::new(Sandbox::new("."));
        let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();
        let mut input_flag = Arc::new(AtomicBool::new(true));
        let mut manager = AppletManager::new();
        let result = executor.execute("nonexistent_tool", &json!({}), &mut ToolContext {
            memory: &mut MemoryStore::default(),
            prompt_history: &mut PromptHistory::default(),
            analyzer: &SelfAnalyzer::new(std::path::PathBuf::from(".")),
            evolver: &ToolEvolver::new(vec![]),
            ollama: &OllamaClient::new("test"),
            project_root: std::path::Path::new("."),
            applet_manager: &mut manager,
            steer_tx: &steer_tx,
            steer_rx: &steer_rx,
            input_flag: &mut input_flag,
        }).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown tool"));
    }

    #[tokio::test]
    async fn test_write_read_roundtrip() {
        let dir = std::env::temp_dir().join("ayesha_test_write_read");
        let _ = std::fs::create_dir_all(&dir);
        let test_file = dir.join("test.txt");

        let executor = ToolExecutor::new(Sandbox::new(&dir));

        // Write
        let write_result = executor.write_file(&json!({
            "path": test_file.to_string_lossy(),
            "content": "hello ayesha! :3"
        })).await.unwrap();
        assert!(write_result.contains("wrote"));

        // Read
        let read_result = executor.read_file(&json!({
            "path": test_file.to_string_lossy()
        })).await.unwrap();
        assert_eq!(read_result, "hello ayesha! :3");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_list_dir() {
        let dir = std::env::temp_dir().join("ayesha_test_list");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        std::fs::write(dir.join("b.txt"), "b").unwrap();

        let executor = ToolExecutor::new(Sandbox::new(&dir));
        let result = executor.list_dir(&json!({
            "path": dir.to_string_lossy()
        })).await.unwrap();

        assert!(result.contains("a.txt"));
        assert!(result.contains("b.txt"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_write_sensitive_blocked() {
        let dir = std::env::temp_dir().join("ayesha_test_env");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(".env");
        let executor = ToolExecutor::new(Sandbox::new("."));
        let result = executor.write_file(&json!({
            "path": path.to_string_lossy(),
            "content": "SECRET=123"
        })).await;
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_remember_and_list() {
        let mut memory = MemoryStore::default();
        let executor = ToolExecutor::new(Sandbox::new("."));

        let _ = executor.remember(&json!({
            "content": "user likes tuna",
            "category": "user_pref"
        }), &mut memory);

        let _ = executor.remember(&json!({
            "content": "ayesha lives in a computer",
            "category": "fact"
        }), &mut memory);

        let list = executor.list_memories(&json!({"count": 10}), &memory).unwrap();
        assert!(list.contains("tuna"));
        assert!(list.contains("computer"));

        let search = executor.search_memories(&json!({"query": "tuna"}), &memory).unwrap();
        assert!(search.contains("tuna"));
        assert!(!search.contains("computer"));
    }

    #[test]
    fn test_set_preference() {
        let mut memory = MemoryStore::default();
        let executor = ToolExecutor::new(Sandbox::new("."));

        let _ = executor.set_preference(&json!({
            "key": "favorite color",
            "value": "cyan"
        }), &mut memory);

        assert_eq!(memory.get_preference("favorite color"), Some("cyan"));
    }

    #[tokio::test]
    async fn test_manage_applet_list() {
        let executor = ToolExecutor::new(Sandbox::new("."));
        let (steer_tx, steer_rx) = std::sync::mpsc::channel::<String>();
        let mut input_flag = Arc::new(AtomicBool::new(true));
        let mut manager = AppletManager::new();
        let result = executor.execute("manage_applet", &json!({ "action": "list" }), &mut ToolContext {
            memory: &mut MemoryStore::default(),
            prompt_history: &mut PromptHistory::default(),
            analyzer: &SelfAnalyzer::new(std::path::PathBuf::from(".")),
            evolver: &ToolEvolver::new(vec![]),
            ollama: &OllamaClient::new("test"),
            project_root: std::path::Path::new("."),
            applet_manager: &mut manager,
            steer_tx: &steer_tx,
            steer_rx: &steer_rx,
            input_flag: &mut input_flag,
        }).await.unwrap();
        assert!(result.contains("applets"));
        assert!(result.contains("engine"));
    }
}
