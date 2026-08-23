#!/usr/bin/env python3
"""import-takeout-memories.py — distill Google Takeout Gemini activity into ayesha-os memories.

Parses Takeout/My Activity/Gemini Apps/My Activity.html, extracts user's
interests, projects, hardware, location, and preferences as memory entries
in the exact schema that engine/src/memory.rs (MemoryStore) expects.

Writes to ~/.ayesha/memory.json, merging with any existing entries.
"""
import argparse
import html as html_mod
import json
import os
import re
import sys
import zipfile
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# heuristics — keyword banks for distillation
# ---------------------------------------------------------------------------

HARDWARE_KEYWORDS = {
    "gpu": ("user has a dedicated gpu for local model inference", ["hardware", "gpu"], 7),
    "raspberry pi": ("user has a raspberry pi — interested in low-power home labs", ["hardware", "home-lab"], 5),
    "miyoo mini": ("user has a retro handheld gaming device", ["hardware", "retro-gaming"], 5),
}

PROJECT_KEYWORDS = {
    "ayesha-os": ("user is building ayesha-os — a distributed self-improving ai ecosystem with kilo", ["project", "ai"], 9),
    "ayesha os": ("user is building ayesha-os — a distributed self-improving ai ecosystem with kilo", ["project", "ai"], 9),
    "zelda hacker": ("user asked gemini to make a zelda hacker pixel art game", ["project", "game", "pixel-art"], 5),
    "pixel link hacker": ("user asked gemini to make a pixel link hacker game", ["project", "game", "pixel-art"], 5),
    "toxic flora": ("user is working on a scottish flora phylogeny project", ["project", "flora", "scotland"], 5),
    "neural-strike": ("user has a mechanistic interpretability game project", ["project", "applet"], 5),
    "flora-cli": ("user has a scottish flora phylogeny explorer", ["project", "applet"], 5),
    "opencode": ("user uses an ai coding assistant", ["tool", "ai"], 5),
    "kilo": ("user uses kilo for local model inference", ["tool", "ai", "local-llm"], 5),
    "mistral": ("user is interested in mistral models", ["model", "ai"], 4),
    "modelfile": ("user creates kilo modelfiles for custom model personas", ["model", "ai"], 4),
    "email server": ("user wanted to set up their own email server", ["project", "self-hosting"], 3),
    "mail server": ("user wanted to set up their own email server", ["project", "self-hosting"], 3),
}

INTEREST_KEYWORDS = {
    "pixel art": ("user is into pixel art — sprite generation, character design, animation", ["interest", "art"], 5),
    "retro gaming": ("user loves retro gaming", ["interest", "gaming"], 4),
    "retro game": ("user loves retro gaming", ["interest", "gaming"], 4),
    "vocaloid": ("user likes vocaloid music", ["interest", "music"], 3),
    "hatsune miku": ("user likes hatsune miku / vocaloid", ["interest", "music"], 3),
    "breakcore": ("user likes breakcore / jungle music", ["interest", "music"], 3),
    "anime": ("user is into anime culture", ["interest", "anime"], 3),
    "scottish": ("user has connections to scotland — works on scottish flora projects", ["interest", "scotland"], 4),
    "scotland": ("user has connections to scotland — works on scottish flora projects", ["interest", "scotland"], 4),
    "toots cafe": ("user knows about toots cafe in rothes", ["location", "scotland"], 3),
    "rothes": ("user is familiar with rothes, scotland", ["location", "scotland"], 4),
    "open source": ("user cares about open source software", ["interest", "philosophy"], 3),
    "self-improvement": ("user is interested in self-improvement concepts", ["interest", "philosophy"], 3),
    "mechanistic interpretability": ("user is interested in mechanistic interpretability", ["interest", "ai-research"], 4),
    "umi": ("user knows about umi — possibly a character or tool", ["interest", "general"], 2),
}

PREFERENCE_KEYWORDS = {
    "favorite color": None,  # handled via [PREFERENCE] markers
    "i like": None,
    "i love": None,
    "i prefer": None,
    "i hate": None,
}

# ---------------------------------------------------------------------------
# HTML parsing
# ---------------------------------------------------------------------------

def parse_gemini_activity(html_text: str) -> list[dict]:
    """Parse the Gemini Apps My Activity.html into structured events."""
    events = []
    # split on outer-cell boundaries to get each conversation turn
    blocks = re.split(r'<div class="outer-cell mdl-cell mdl-cell--12-col', html_text)

    for block in blocks[1:]:  # skip first (before first outer-cell)
        # extract prompt text
        prompt_match = re.search(
            r'Prompted\s*(.*?)<br>(\d{1,2}\s+\w+\s+\d{4},\s+\d{1,2}:\d{2}:\d{2}\s+\w+)<br>',
            block, re.DOTALL
        )
        if not prompt_match:
            continue

        raw_prompt = prompt_match.group(1)
        date_str = prompt_match.group(2)

        # clean prompt text (strip HTML tags)
        prompt_text = re.sub(r'<[^>]+>', '', raw_prompt).strip()
        prompt_text = html_mod.unescape(prompt_text)

        # extract chat id from URL
        chat_match = re.search(r'gemini\.google\.com/app/([0-9a-f]{16})', block)
        chat_id = chat_match.group(1) if chat_match else "unknown"

        # parse timestamp
        ts = parse_date_to_epoch(date_str)

        # extract model response (first <p> in the response cell)
        resp_match = re.search(r'mdl-typography--text-right.*?<p>(.*?)</p>', block, re.DOTALL)
        response = ""
        if resp_match:
            response = re.sub(r'<[^>]+>', '', resp_match.group(1)).strip()
            response = html_mod.unescape(response)

        events.append({
            "prompt": prompt_text,
            "response": response,
            "chat_id": chat_id,
            "date": date_str,
            "timestamp": ts,
        })

    return events


MONTHS = {m: i + 1 for i, m in enumerate(
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
     "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"])}


def parse_date_to_epoch(date_str: str) -> float:
    """Parse Takeout date string to epoch seconds."""
    if not date_str:
        return 0.0
    s = date_str.strip()
    # format: "7 Aug 2026, 23:38:27 BST"
    m = re.match(
        r"(\d{1,2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4}),\s+(\d{1,2}):(\d{2}):(\d{2})\s+\w+",
        s
    )
    if not m:
        return 0.0
    day, mon, year, hh, mm, ss = m.groups()
    try:
        dt = datetime(int(year), MONTHS[mon], int(day), int(hh), int(mm), int(ss), tzinfo=timezone.utc)
        return dt.timestamp()
    except Exception:
        return 0.0


# ---------------------------------------------------------------------------
# memory distillation
# ---------------------------------------------------------------------------

def distill_memories(events: list[dict]) -> dict:
    """Distill high-value memories from conversation events.

    Returns dict with keys: memories, user_preferences, learned_facts
    """
    memories = []
    seen_content = set()  # for dedup
    pref_map = {}
    facts = []

    def add_memory(content: str, category: str, tags: list[str], importance: int):
        key = content.lower().strip()
        if key in seen_content:
            return
        seen_content.add(key)
        memories.append({
            "content": content,
            "category": category,
            "tags": tags,
            "importance": importance,
        })

    # scan all prompts and responses for keywords
    for ev in events:
        text = (ev["prompt"] + " " + ev.get("response", "")).lower()
        ts = ev.get("timestamp", 0.0)
        ts_str = f"{int(ts)}s" if ts else "unknown"

        # hardware
        for kw, (content, tags, imp) in HARDWARE_KEYWORDS.items():
            if kw in text:
                add_memory(content, "fact", tags, imp)

        # projects
        for kw, (content, tags, imp) in PROJECT_KEYWORDS.items():
            if kw in text:
                add_memory(content, "project", tags, imp)

        # interests
        for kw, (content, tags, imp) in INTEREST_KEYWORDS.items():
            if kw in text:
                add_memory(content, "interest", tags, imp)

        # extract explicit preference patterns from prompts (not responses)
        # "i like X" / "i prefer X" / "favorite X is Y" — only from user's messages
        prompt_text = ev["prompt"].lower()
        for pattern in [
            r"^(?:i |do )?(?:like|love|prefer) ([a-z0-9 ]{2,40})$",
            r"^(?:my )?(?:favorite|favourite) ([a-z ]{2,20}) (?:is|are) ([a-z0-9 ]{2,40})$",
        ]:
            for match in re.finditer(pattern, prompt_text):
                groups = match.groups()
                if len(groups) == 1:
                    pref_content = groups[0].strip()
                    # filter out junk
                    if pref_content and not any(skip in pref_content for skip in
                        ["this model", "this file", "this", "that", "it", "you", "him", "her",
                         "them", "so", "but", "and", "or", "because", "if", "when",
                         "parse", "whats", "fusion", "blood", "ana"]):
                        add_memory(f"user likes: {pref_content}", "user_pref", ["preference"], 5)
                elif len(groups) == 2:
                    key = groups[0].strip()
                    val = groups[1].strip()
                    if val and key:
                        pref_map[key] = val
                        add_memory(f"user's {key}: {val}", "user_pref", ["preference"], 6)

        # [PREFERENCE: key = value] marker (from ayesha's own memory system)
        for match in re.finditer(r'\[PREFERENCE:\s*(.+?)\s*=\s*(.+?)\]', ev.get("response", "")):
            k, v = match.groups()
            pref_map[k.strip()] = v.strip()
            add_memory(f"user's {k.strip()}: {v.strip()}", "user_pref", ["preference"], 7)

        # [REMEMBER: content] marker
        for match in re.finditer(r'\[REMEMBER:\s*(.+?)\]', ev.get("response", "")):
            content = match.group(1).strip()
            add_memory(content, "fact", ["ayesha-remembered"], 7)

        # [FACT: content] marker
        for match in re.finditer(r'\[FACT:\s*(.+?)\]', ev.get("response", "")):
            content = match.group(1).strip()
            add_memory(content, "fact", ["ayesha-fact"], 7)

    # add some meta-facts about user based on the overall pattern
    chat_ids = set(ev["chat_id"] for ev in events if ev["chat_id"] != "unknown")
    total_prompts = len(events)
    date_range = ""
    timestamps = [ev["timestamp"] for ev in events if ev["timestamp"] > 0]
    if timestamps:
        earliest = datetime.fromtimestamp(min(timestamps), tz=timezone.utc)
        latest = datetime.fromtimestamp(max(timestamps), tz=timezone.utc)
        date_range = f"{earliest.strftime('%Y-%m-%d')} to {latest.strftime('%Y-%m-%d')}"

    add_memory(
        f"user had {total_prompts} conversations with gemini across {len(chat_ids)} chats ({date_range})",
        "meta",
        ["meta", "usage"],
        3
    )
    add_memory(
        "user calls themselves 'user' or 'ayesha-os' — refers to ayesha as their ai companion",
        "user_pref",
        ["identity", "preference"],
        8
    )

    return {
        "memories": memories,
        "user_preferences": pref_map,
        "learned_facts": facts,
    }


# ---------------------------------------------------------------------------
# memory.json merge
# ---------------------------------------------------------------------------

def load_existing_memories(path: Path) -> dict:
    """Load existing memory.json or return default schema."""
    if path.exists():
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            # validate basic schema
            if "memories" in data:
                return data
        except Exception:
            pass
    return {"memories": [], "user_preferences": {}, "learned_facts": []}


def merge_memories(existing: dict, distilled: dict) -> dict:
    """Merge distilled memories into existing store, preserving existing entries."""
    # find max existing id number
    max_id = -1
    for m in existing.get("memories", []):
        mid = m.get("id", "mem_-1")
        try:
            n = int(mid.split("_")[1])
            max_id = max(max_id, n)
        except (IndexError, ValueError):
            pass

    # dedup against existing content
    existing_content = set(
        m.get("content", "").lower().strip()
        for m in existing.get("memories", [])
    )

    next_id = max_id + 1
    added = 0
    for mem in distilled.get("memories", []):
        key = mem["content"].lower().strip()
        if key in existing_content:
            continue
        existing_content.add(key)
        timestamp = "unknown"
        # use current time as import timestamp
        timestamp = f"{int(datetime.now(timezone.utc).timestamp())}s"
        existing.setdefault("memories", []).append({
            "id": f"mem_{next_id}",
            "category": mem["category"],
            "content": mem["content"],
            "tags": mem["tags"],
            "timestamp": timestamp,
            "importance": mem["importance"],
        })
        next_id += 1
        added += 1

    # merge user_preferences
    for k, v in distilled.get("user_preferences", {}).items():
        existing.setdefault("user_preferences", {})[k] = v

    # merge learned_facts
    existing_facts = set(existing.get("learned_facts", []))
    for fact in distilled.get("learned_facts", []):
        if fact not in existing_facts:
            existing.setdefault("learned_facts", []).append(fact)
            existing_facts.add(fact)

    print(f"[+] merged {added} new memories (total: {len(existing.get('memories', []))})")
    print(f"[+] preferences: {len(existing.get('user_preferences', {}))}")
    print(f"[+] facts: {len(existing.get('learned_facts', []))}")
    return existing


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="distill takeout gemini activity into ayesha-os memories")
    parser.add_argument("archive", help="path to takeout .zip")
    parser.add_argument("--out", default=str(Path.home() / ".ayesha" / "memory.json"),
                        help="output memory.json path (default: ~/.ayesha/memory.json)")
    parser.add_argument("--dry-run", action="store_true", help="print distilled memories without writing")
    args = parser.parse_args()

    archive = Path(args.archive)
    if not archive.is_file():
        print(f"[X] archive not found: {archive}", file=sys.stderr)
        sys.exit(1)

    out_path = Path(args.out)

    # find the gemini activity html in the zip
    target = None
    with zipfile.ZipFile(archive) as zf:
        for name in zf.namelist():
            if name.endswith("My Activity.html") and "Gemini Apps" in name:
                target = name
                break
        if not target:
            print("[X] no Gemini Apps/My Activity.html found in archive", file=sys.stderr)
            sys.exit(1)

        print(f"[+] parsing {target} ...")
        html_text = zf.read(target).decode("utf-8", errors="replace")

    events = parse_gemini_activity(html_text)
    print(f"[+] extracted {len(events)} prompt/response turns")

    if not events:
        print("[!] no events found — check the zip structure", file=sys.stderr)
        sys.exit(1)

    distilled = distill_memories(events)
    print(f"[+] distilled {len(distilled['memories'])} unique memories")

    if args.dry_run:
        print("\n--- dry run: distilled memories ---")
        for m in distilled["memories"]:
            print(f"  [{m['category']}] imp={m['importance']} | {m['content']}")
            print(f"    tags: {m['tags']}")
        if distilled["user_preferences"]:
            print("\n  user_preferences:")
            for k, v in distilled["user_preferences"].items():
                print(f"    {k} = {v}")
        return

    # load existing and merge
    existing = load_existing_memories(out_path)
    merged = merge_memories(existing, distilled)

    # write
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(merged, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"[+] wrote {out_path}")

    # summary
    n = len(merged.get("memories", []))
    cats = {}
    for m in merged.get("memories", []):
        c = m.get("category", "unknown")
        cats[c] = cats.get(c, 0) + 1
    print(f"[+] memory summary:")
    for c, count in sorted(cats.items(), key=lambda x: -x[1]):
        print(f"    {c}: {count}")


if __name__ == "__main__":
    main()
