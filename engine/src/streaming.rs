//! shared streaming machinery — one decoder + pipeline both backends consume.
//!
//! previously ollama (NDJSON) and cloud (SSE) each had their own streaming
//! loop, their own lowercase/think handling, and their own (slightly
//! different) active tool-call reconstruction. this module is the single
//! home for all of it:
//!
//! - `StreamDecoder` implementations are chunk-boundary-safe: they buffer raw
//!   bytes and only emit events for COMPLETE records, so a `data:` line, an
//!   NDJSON line, or a multi-byte character split across network chunks is
//!   never lost, truncated, or corrupted (the old code ran
//!   `from_utf8_lossy` over whole chunks, which turned split multibyte
//!   characters into U+FFFD, and a split `data:` line could lose an event).
//! - `StreamPipeline` owns the shared post-decode logic: lowercase
//!   enforcement, think-block tracking, active tool-call reconstruction, and
//!   the non-blocking steer hook.
//! - `stream_to_result` is the one streaming loop both backends call, so the
//!   visible and collect paths can never drift apart again.

use std::sync::mpsc;

use serde_json::{json, Value};

use crate::format::LowercaseStreamer;
use crate::ollama::{StreamResult, ToolCall, ToolFunction};

/// One decoded record from a transport stream.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// raw content delta (not yet enforced or think-tracked)
    Chunk(String),
    /// one OpenAI-shaped tool-call delta: `{index, id, type, function:{name, arguments}}`
    ToolDelta(Value),
    /// a think block opened in the rolling content tail
    ThinkOpen,
    /// a think block closed in the rolling content tail
    ThinkClose,
    /// terminal marker: ollama `done:true` or SSE `data: [DONE]`
    Done,
}

/// Converts raw transport bytes into complete-record events. Implementations
/// MUST hold partial data in an internal buffer and only emit events once a
/// full record (NDJSON line / SSE block terminated by a blank line) has
/// arrived — this is what makes decoding safe across arbitrary chunk splits.
pub trait StreamDecoder {
    fn feed(&mut self, bytes: &[u8]) -> Vec<StreamEvent>;
    /// Flush a trailing partial record at end of stream.
    fn finish(&mut self) -> Vec<StreamEvent>;
}

/// NDJSON decoder: one JSON object per line (ollama `/api/chat` stream).
pub struct OllamaDecoder {
    buf: Vec<u8>,
}

impl OllamaDecoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn decode_line(line: &[u8]) -> Vec<StreamEvent> {
        let line = String::from_utf8_lossy(line);
        let line = line.trim();
        if line.is_empty() {
            return Vec::new();
        }

        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                let preview: String = line.chars().take(80).collect();
                eprintln!("ollama stream parse error: {} near: {}", e, preview);
                return Vec::new();
            }
        };

        let mut events = Vec::new();

        // content deltas
        if let Some(c) = v["message"]["content"].as_str() {
            if !c.is_empty() {
                events.push(StreamEvent::Chunk(c.to_string()));
            }
        }

        // tool calls (ollama usually sends them complete in the done chunk;
        // emit them as deltas anyway so the shared reconstruction handles them)
        if let Some(tcs) = v
            .get("message")
            .and_then(|m| m.get("tool_calls"))
            .and_then(|t| t.as_array())
        {
            for (idx, tc) in tcs.iter().enumerate() {
                // only complete-looking calls (same shape the legacy
                // Vec<ToolCall> parse required)
                if tc.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .is_none()
                {
                    continue;
                }
                let mut delta = tc.clone();
                if let Some(obj) = delta.as_object_mut() {
                    obj.entry("index").or_insert(json!(idx));
                    // normalize arguments to the string form the shared
                    // accumulator expects (round-trips back to the same value)
                    if let Some(f) = obj.get_mut("function") {
                        if let Some(args) = f.get_mut("arguments") {
                            if !args.is_string() {
                                *args = Value::String(serde_json::to_string(args).unwrap_or_default());
                            }
                        }
                    }
                }
                events.push(StreamEvent::ToolDelta(delta));
            }
        }

        if v.get("done").and_then(|d| d.as_bool()) == Some(true) {
            events.push(StreamEvent::Done);
        }

        events
    }
}

impl StreamDecoder for OllamaDecoder {
    fn feed(&mut self, bytes: &[u8]) -> Vec<StreamEvent> {
        self.buf.extend_from_slice(bytes);
        let mut events = Vec::new();
        while let Some(nl) = self.buf.iter().position(|&b| b == b'\n') {
            // drain through the newline, then strip it before decoding
            let line: Vec<u8> = self.buf.drain(..=nl).collect();
            events.extend(Self::decode_line(&line[..line.len() - 1]));
        }
        events
    }

    fn finish(&mut self) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        if !self.buf.is_empty() {
            events.extend(Self::decode_line(&self.buf));
            self.buf.clear();
        }
        events
    }
}

/// SSE decoder: blocks of `data:` lines terminated by a blank line
/// (OpenAI-compatible chat/completions streams). A `data:` line split across
/// network chunks is held in the buffer until the blank-line terminator
/// arrives, so the event is never lost or truncated.
pub struct SseDecoder {
    buf: Vec<u8>,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Byte index just past the blank line that terminates a complete SSE
    /// block (`\n\n` or `\r\n\r\n`).
    fn block_end(buf: &[u8]) -> Option<usize> {
        for i in 0..buf.len().saturating_sub(1) {
            if buf[i] == b'\n' {
                if buf[i + 1] == b'\n' {
                    return Some(i + 2);
                }
                if buf[i + 1] == b'\r' && buf.get(i + 2) == Some(&b'\n') {
                    return Some(i + 3);
                }
            }
        }
        None
    }

    fn decode_block(block: &[u8]) -> Vec<StreamEvent> {
        let text = String::from_utf8_lossy(block);
        let mut data_parts: Vec<String> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("data:") {
                data_parts.push(rest.trim().to_string());
            }
        }
        let data = data_parts.join("\n");

        if data == "[DONE]" {
            return vec![StreamEvent::Done];
        }
        if data.is_empty() {
            return Vec::new();
        }

        // Normal path: the whole accumulated `data:` payload is one JSON
        // document (OpenAI-compatible multi-line events).
        match serde_json::from_str::<Value>(&data) {
            Ok(v) => Self::decode_value(&v),
            Err(e) => {
                // Fallback for line-per-event streams: some providers separate
                // events with a bare `\n` instead of a blank line, so the
                // whole accumulation never parses as one document. Re-decode
                // each `data:` line individually so those streams still emit.
                let mut events = Vec::new();
                for line in &data_parts {
                    if line == "[DONE]" {
                        events.push(StreamEvent::Done);
                        continue;
                    }
                    match serde_json::from_str::<Value>(line) {
                        Ok(v) => events.extend(Self::decode_value(&v)),
                        Err(e2) => {
                            let preview = crate::util::truncate_chars(line, 80);
                            eprintln!("cloud stream parse error: {} near: {}", e2, preview);
                        }
                    }
                }
                if events.is_empty() {
                    let preview = crate::util::truncate_chars(&data, 80);
                    eprintln!("cloud stream parse error: {} near: {}", e, preview);
                }
                events
            }
        }
    }

    /// Decode a single OpenAI-compatible event JSON value into stream events
    /// (content chunks + tool deltas).
    fn decode_value(v: &Value) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(delta) = choice.get("delta") {
                    if let Some(c) = delta.get("content").and_then(|c| c.as_str()) {
                        if !c.is_empty() {
                            events.push(StreamEvent::Chunk(c.to_string()));
                        }
                    }
                    if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                        for tc in tcs {
                            events.push(StreamEvent::ToolDelta(tc.clone()));
                        }
                    }
                }
            }
        }
        events
    }
}

impl StreamDecoder for SseDecoder {
    fn feed(&mut self, bytes: &[u8]) -> Vec<StreamEvent> {
        self.buf.extend_from_slice(bytes);
        let mut events = Vec::new();
        while let Some(end) = Self::block_end(&self.buf) {
            let block: Vec<u8> = self.buf.drain(..end).collect();
            events.extend(Self::decode_block(&block));
        }
        events
    }

    fn finish(&mut self) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        if !self.buf.is_empty() {
            events.extend(Self::decode_block(&self.buf));
            self.buf.clear();
        }
        events
    }
}

/// One in-flight tool call being reconstructed from streamed deltas.
#[derive(Default)]
struct ActiveToolCall {
    id: String,
    call_type: String,
    name: String,
    args: String,
}

/// Shared post-decode logic for both backends: lowercase enforcement, think
/// tracking, and active tool-call reconstruction. Also owns the steer hook —
/// `poll_steer` checks the typed-input channel without blocking, so the
/// caller can react to user input at every chunk boundary.
pub struct StreamPipeline<'a> {
    formatter: LowercaseStreamer,
    in_think: bool,
    recent: String,
    active: Vec<ActiveToolCall>,
    steer_rx: &'a mpsc::Receiver<String>,
    /// fully enforced content accumulated so far (the formatter's held-back
    /// tail is only flushed into this by finish()/finish_steered())
    pub content: String,
}

impl<'a> StreamPipeline<'a> {
    const RECENT_MAX: usize = 20;

    pub fn new(steer_rx: &'a mpsc::Receiver<String>) -> Self {
        Self {
            formatter: LowercaseStreamer::new(),
            in_think: false,
            recent: String::new(),
            active: Vec::new(),
            steer_rx,
            content: String::new(),
        }
    }

    pub fn is_thinking(&self) -> bool {
        self.in_think
    }

    /// Feed decoded events; returns the displayable events (enforced Chunks,
    /// think transitions, Done). ToolDelta events are consumed internally for
    /// tool-call reconstruction and are not re-emitted.
    pub fn feed_events(&mut self, events: Vec<StreamEvent>) -> Vec<StreamEvent> {
        let mut out = Vec::new();
        for ev in events {
            match ev {
                StreamEvent::Chunk(raw) => {
                    let enforced = self.formatter.feed(&raw);
                    if !enforced.is_empty() {
                        self.content.push_str(&enforced);
                        self.recent.push_str(&enforced);
                        let n = self.recent.chars().count();
                        if n > Self::RECENT_MAX {
                            let skip = n - Self::RECENT_MAX;
                            self.recent = self.recent.chars().skip(skip).collect();
                        }

                        // detect think-block transitions on the recent tail so
                        // tags split across chunk boundaries (<th / ink>)
                        // still register, but already-closed blocks never
                        // re-trigger on full content
                        if !self.in_think
                            && (self.recent.contains("<think>") || self.recent.contains("[think]"))
                        {
                            self.in_think = true;
                            out.push(StreamEvent::ThinkOpen);
                        }
                        if self.in_think
                            && (self.recent.contains("</think>") || self.recent.contains("[/think]"))
                        {
                            self.in_think = false;
                            out.push(StreamEvent::ThinkClose);
                        }
                    }
                    out.push(StreamEvent::Chunk(enforced));
                }
                StreamEvent::ToolDelta(delta) => self.accumulate_tool_delta(delta),
                StreamEvent::ThinkOpen | StreamEvent::ThinkClose => {}
                StreamEvent::Done => out.push(StreamEvent::Done),
            }
        }
        out
    }

    fn accumulate_tool_delta(&mut self, delta: Value) {
        let index = delta.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
        while self.active.len() <= index {
            self.active.push(ActiveToolCall::default());
        }
        let active = &mut self.active[index];

        // Gate B N3: a delta that carries a NEW id/name for a slot which
        // already holds accumulated state means the provider re-emitted the
        // call from scratch (some providers send the full call twice — once in
        // a `done:false` chunk, once in `done:true`). Reset the slot so
        // arguments can't double-append.
        let new_id = delta.get("id").and_then(|i| i.as_str());
        let new_name = delta
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str());
        let identity_changed = (new_id.is_some()
            && !active.id.is_empty()
            && new_id != Some(active.id.as_str()))
            || (new_name.is_some()
                && !active.name.is_empty()
                && new_name != Some(active.name.as_str()));
        if identity_changed {
            *active = ActiveToolCall::default();
        }

        // Gate C N3 residual: some providers re-emit the full call under the
        // SAME id+name (a `done:false` chunk followed by a `done:true` chunk).
        // Gate B can't catch that (the identity didn't change), so without a
        // guard the arguments would double-append and corrupt the JSON. Reset
        // only when the slot's accumulated arguments ALREADY parse as complete
        // JSON AND the incoming delta carries a call identity (id/name — the
        // signature of a call start) AND the incoming delta's own arguments
        // parse as complete JSON too. That last check is what distinguishes a
        // genuine re-emission (which always carries complete args) from a
        // provider that merely echoes id/name on continuation deltas while the
        // accumulated args happen to cross a parseable boundary mid-call — the
        // echo case must keep appending, never reset.
        let carries_identity = new_id.is_some() || new_name.is_some();
        let incoming_args_complete = match delta
            .get("function")
            .and_then(|f| f.get("arguments"))
        {
            Some(Value::String(s)) => serde_json::from_str::<Value>(s).is_ok(),
            Some(other) => other.is_object() || other.is_array(),
            None => false,
        };
        if carries_identity
            && !active.args.is_empty()
            && serde_json::from_str::<Value>(&active.args).is_ok()
            && incoming_args_complete
        {
            *active = ActiveToolCall::default();
        }

        if let Some(id) = delta.get("id").and_then(|i| i.as_str()) {
            active.id = id.to_string();
        }
        if let Some(t) = delta.get("type").and_then(|t| t.as_str()) {
            active.call_type = t.to_string();
        }
        if let Some(f) = delta.get("function") {
            if let Some(name) = f.get("name").and_then(|n| n.as_str()) {
                active.name = name.to_string();
            }
            if let Some(args) = f.get("arguments") {
                match args {
                    Value::String(s) => active.args.push_str(s),
                    other => {
                        if let Ok(s) = serde_json::to_string(other) {
                            active.args.push_str(&s);
                        }
                    }
                }
            }
        }
    }

    /// Rebuild final ToolCalls from the accumulated deltas (arguments strings
    /// parsed back to JSON, falling back to `{}` for malformed fragments).
    fn finalize(&mut self) -> Vec<ToolCall> {
        let mut calls = Vec::with_capacity(self.active.len());
        for a in &self.active {
            let args_val = if a.args.is_empty() {
                json!({})
            } else {
                serde_json::from_str(&a.args).unwrap_or(json!({}))
            };
            calls.push(ToolCall {
                id: a.id.clone(),
                call_type: if a.call_type.is_empty() {
                    "function".to_string()
                } else {
                    a.call_type.clone()
                },
                function: ToolFunction {
                    name: a.name.clone(),
                    arguments: args_val,
                },
            });
        }
        calls
    }

    /// Non-blocking steer check: the newest typed input (or control code) if
    /// the user typed during generation — never blocks.
    pub fn poll_steer(&mut self) -> Option<String> {
        match self.steer_rx.try_recv() {
            Ok(input) => Some(input),
            Err(_) => None,
        }
    }

    fn flush_tail(&mut self) {
        let tail = self.formatter.finish();
        if !tail.is_empty() {
            self.content.push_str(&tail);
        }
    }

    /// Normal completion: flush the formatter tail and pair everything up.
    pub fn finish(mut self) -> StreamResult {
        self.flush_tail();
        let tool_calls = self.finalize();
        StreamResult {
            content: self.content,
            tool_calls,
            steering: None,
        }
    }

    /// Early-exit completion (steering): flush the formatter tail, pair up
    /// whatever tool calls were reconstructed so far, and attach the steer.
    pub fn finish_steered(mut self, steering: String) -> StreamResult {
        self.flush_tail();
        let tool_calls = self.finalize();
        StreamResult {
            content: self.content,
            tool_calls,
            steering: Some(steering),
        }
    }
}

/// The single streaming loop both backends consume. Feeds network bytes
/// through `decoder` into a `StreamPipeline`, renders chunks when `visible`,
/// checks for steering between chunks (so typed input redirects the agent
/// mid-generation), and returns a complete StreamResult.
pub async fn stream_to_result(
    resp: &mut reqwest::Response,
    steer_rx: &mpsc::Receiver<String>,
    visible: bool,
    decoder: &mut dyn StreamDecoder,
) -> anyhow::Result<StreamResult> {
    let mut pipeline = StreamPipeline::new(steer_rx);
    let mut code = crate::render::CodeStream::new();

    loop {
        let chunk = resp.chunk().await?;
        let events = match &chunk {
            Some(bytes) => pipeline.feed_events(decoder.feed(bytes)),
            None => pipeline.feed_events(decoder.finish()),
        };

        let mut done = false;
        for ev in events {
            match ev {
                StreamEvent::Chunk(text) => {
                    if visible {
                        if pipeline.is_thinking() {
                            crate::ui::convo_write(&crate::theme::paint(crate::theme::Role::Dim, &text));
                        } else {
                            crate::ui::convo_write(&code.feed(&text));
                        }
                    }
                }
                StreamEvent::Done => done = true,
                StreamEvent::ThinkOpen | StreamEvent::ThinkClose | StreamEvent::ToolDelta(_) => {}
            }
        }

        if done {
            if visible {
                crate::ui::convo_write(&code.finish());
                if !pipeline.content.is_empty() {
                    crate::ui::convo_write("\n");
                }
            }
            return Ok(pipeline.finish());
        }

        // live steering: drain typed input at every chunk boundary
        if let Some(input) = pipeline.poll_steer() {
            if visible {
                crate::ui::convo_write(&code.finish());
                crate::ui::convo_write("\n");
            }
            return Ok(pipeline.finish_steered(input));
        }

        if chunk.is_none() {
            break;
        }
    }

    if visible {
        crate::ui::convo_write(&code.finish());
        crate::ui::convo_write("\n");
    }
    Ok(pipeline.finish())
}

/// The five call sites that pass tool definitions to chat calls share this
/// exact slice dance: definitions are a JSON array, the chat APIs take
/// `&[Value]`.
pub fn tool_payload_slice(v: &Value) -> &[Value] {
    v.as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_events(decoder: &mut dyn StreamDecoder, chunks: &[&[u8]]) -> Vec<StreamEvent> {
        let mut out = Vec::new();
        for c in chunks {
            out.extend(decoder.feed(c));
        }
        out.extend(decoder.finish());
        out
    }

    fn channel() -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel::<String>();
        drop(tx);
        rx
    }

    // ── OllamaDecoder ──

    #[test]
    fn ollama_decoder_chunk_split_produces_identical_events() {
        let payload = br#"{"message":{"content":"hi "},"done":false}
{"message":{"content":"there"},"done":false}
{"message":{"content":""},"done":true}
"#;
        // byte-by-byte feeds must produce exactly the same events as one feed
        let split: Vec<&[u8]> = payload.chunks(1).collect();
        let split_events = all_events(&mut OllamaDecoder::new(), &split);
        let whole_events = all_events(&mut OllamaDecoder::new(), &[payload]);
        assert_eq!(split_events, whole_events);
        assert!(split_events.contains(&StreamEvent::Done));
        let chunks: Vec<StreamEvent> = split_events
            .iter()
            .filter(|e| matches!(e, StreamEvent::Chunk(_)))
            .cloned()
            .collect();
        assert_eq!(
            chunks,
            vec![
                StreamEvent::Chunk("hi ".to_string()),
                StreamEvent::Chunk("there".to_string())
            ]
        );
    }

    #[test]
    fn ollama_decoder_multibyte_split_is_not_corrupted() {
        // a kaomoji whose bytes span a chunk boundary must survive intact
        // (the old per-chunk from_utf8_lossy turned it into U+FFFD)
        let payload = "{\"message\":{\"content\":\"(╯°□°)\"},".to_string() + "\"done\":false}\n";
        let bytes = payload.as_bytes();
        let mut d = OllamaDecoder::new();
        let mut events = Vec::new();
        for b in bytes {
            events.extend(d.feed(&[*b]));
        }
        events.extend(d.finish());
        let chunks: String = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::Chunk(c) => Some(c.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(chunks, "(╯°□°)");
        assert!(!chunks.contains('\u{fffd}'));
    }

    #[test]
    fn ollama_decoder_tool_calls_normalized_and_done() {
        let mut d = OllamaDecoder::new();
        let line = r#"{"message":{"content":"","tool_calls":[{"function":{"name":"write_file","arguments":{"path":"C:/x/y.txt","content":"hello"}}}],"role":"assistant"},"done":true}
"#;
        let events = d.feed(line.as_bytes());
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Done)));
        let delta = events
            .iter()
            .find_map(|e| match e {
                StreamEvent::ToolDelta(v) => Some(v),
                _ => None,
            })
            .expect("tool delta");
        assert_eq!(delta["function"]["name"], "write_file");
        assert_eq!(delta["index"], 0);
        // arguments normalized to the string form the accumulator expects
        assert!(delta["function"]["arguments"].is_string());
    }

    #[test]
    fn malformed_ollama_line_is_skipped() {
        let mut d = OllamaDecoder::new();
        assert!(d.feed(b"this is not json at all\n").is_empty());
        assert!(d.feed(b"{\"message\":{\"content\":\"hi\"},\"done\":false}\n").len() == 1);
    }

    // ── SseDecoder ──

    #[test]
    fn sse_decoder_data_line_split_across_chunks_still_emits_event() {
        // the reported bug class: a `data:` line chopped mid-way by a network
        // chunk must still produce exactly one complete event.
        let block = "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"world\"}}]}\n\ndata: [DONE]\n\n";
        let bytes = block.as_bytes();
        let mut d = SseDecoder::new();
        let mut events = Vec::new();
        // uneven slice sizes, several of them splitting inside a data: line
        let cuts = [3usize, 11, 5, 29, 7, 13, 2, 31, 1, 5, 17, 8];
        let mut pos = 0;
        for c in cuts {
            let end = (pos + c).min(bytes.len());
            events.extend(d.feed(&bytes[pos..end]));
            pos = end;
        }
        events.extend(d.finish());
        let chunks: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::Chunk(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(chunks, vec!["hello ".to_string(), "world".to_string()]);
        assert!(events.contains(&StreamEvent::Done));
    }

    #[test]
    fn sse_decoder_byte_by_byte_matches_whole_feed() {
        let block = "data: {\"choices\":[{\"delta\":{\"content\":\"(╯°□°)\"}}]}\n\ndata: [DONE]\n\n";
        let bytes = block.as_bytes();
        let split: Vec<&[u8]> = bytes.chunks(1).collect();
        let split_events = all_events(&mut SseDecoder::new(), &split);
        let whole_events = all_events(&mut SseDecoder::new(), &[bytes]);
        assert_eq!(split_events, whole_events);
        assert!(split_events.contains(&StreamEvent::Done));
    }

    #[test]
    fn sse_decoder_handles_crlf() {
        let block = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\r\n\r\n";
        let mut d = SseDecoder::new();
        let events = d.feed(block.as_bytes());
        let chunks: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::Chunk(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(chunks, vec!["hi".to_string()]);
    }

    #[test]
    fn sse_decoder_tool_deltas_and_finish_flush() {
        let mut d = SseDecoder::new();
        // last block has NO trailing blank line — finish() must flush it
        let events = d.feed(
            b"data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"grep\",\"arguments\":\"{\\\"pattern\\\":\\\"x\\\"}\"}}]}}]}\n\n",
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], StreamEvent::ToolDelta(_)));
        let tail = d.finish();
        assert!(tail.is_empty(), "complete block must not be re-emitted");
        assert!(d.finish().is_empty());
    }

    #[test]
    fn sse_decoder_bare_newline_separated_events_still_emit() {
        // Gate B N1: some providers separate `data:` events with a bare `\n`
        // instead of a blank line; decode_block must fall back to line-per-
        // event parsing instead of dropping the whole block.
        let block = "data: {\"choices\":[{\"delta\":{\"content\":\"one\"}}]}\ndata: {\"choices\":[{\"delta\":{\"content\":\"two\"}}]}\ndata: [DONE]\n";
        let mut d = SseDecoder::new();
        // no blank line -> nothing decodes during feed, everything flushes at
        // finish() where the per-line fallback kicks in
        assert!(d.feed(block.as_bytes()).is_empty());
        let events = d.finish();
        let chunks: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::Chunk(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(chunks, vec!["one".to_string(), "two".to_string()]);
        assert!(events.contains(&StreamEvent::Done));
    }

    #[test]
    fn sse_decoder_mixed_blank_and_bare_newlines_decode() {
        // A stream where SOME events are blank-line terminated and the tail
        // ends bare must still produce every chunk exactly once.
        let block = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"b\"}}]}\ndata: {\"choices\":[{\"delta\":{\"content\":\"c\"}}]}\n";
        let mut d = SseDecoder::new();
        let mut events = d.feed(block.as_bytes());
        events.extend(d.finish());
        let chunks: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                StreamEvent::Chunk(c) => Some(c.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(chunks, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    // ── StreamPipeline ──

    #[test]
    fn tool_delta_new_id_resets_active_slot() {
        // Gate B N3: a delta carrying a NEW id for a slot with accumulated
        // arguments means the provider re-emitted the whole call — the slot
        // must reset so arguments don't double-append.
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        // first call: partial arguments arrive over two deltas
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "id": "call_1",
            "type": "function",
            "function": { "name": "read_file", "arguments": "{\"path\": \"src/main" }
        }))]);
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "function": { "arguments": ".rs\"}" }
        }))]);
        // provider re-emits the full call under a NEW id — must reset, not
        // append to the accumulated arguments
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "id": "call_2",
            "type": "function",
            "function": { "name": "grep", "arguments": "{\"pattern\":\"fn\"}" }
        }))]);
        let result = p.finish();
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_2");
        assert_eq!(result.tool_calls[0].function.name, "grep");
        assert_eq!(result.tool_calls[0].function.arguments, json!({"pattern": "fn"}));
    }

    #[test]
    fn tool_delta_same_id_continues_accumulating() {
        // The normal streaming case: same id/name across deltas keeps
        // appending arguments until the call completes.
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "id": "call_1",
            "type": "function",
            "function": { "name": "read_file", "arguments": "{\"path\": \"" }
        }))]);
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "function": { "arguments": "src/main.rs\"}" }
        }))]);
        let result = p.finish();
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_1");
        assert_eq!(result.tool_calls[0].function.name, "read_file");
        assert_eq!(result.tool_calls[0].function.arguments, json!({"path": "src/main.rs"}));
    }

    #[test]
    fn tool_delta_same_id_full_reemission_replaces_not_appends() {
        // Gate C N3 residual: a provider may re-emit the SAME id+name call in
        // full (a `done:false` chunk followed by a `done:true` chunk). The
        // identity doesn't change, so gate B alone can't catch it — the
        // arguments must replace the previous emission instead of
        // double-appending and corrupting the JSON.
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "id": "call_1",
            "type": "function",
            "function": { "name": "read_file", "arguments": "{\"path\": \"src/main.rs\"}" }
        }))]);
        // full re-emission under the identical id+name
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "id": "call_1",
            "type": "function",
            "function": { "name": "read_file", "arguments": "{\"path\": \"src/grep.rs\"}" }
        }))]);
        let result = p.finish();
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_1");
        assert_eq!(result.tool_calls[0].function.name, "read_file");
        assert_eq!(
            result.tool_calls[0].function.arguments,
            json!({"path": "src/grep.rs"}),
            "re-emission must replace, not double-append, the arguments"
        );
    }

    #[test]
    fn tool_delta_continuation_echoing_id_does_not_reset() {
        // Gate C false-positive guard: a provider that echoes id/name on
        // CONTINUATION deltas must not trigger the re-emission reset just
        // because the accumulated arguments happened to cross a parseable
        // boundary mid-call. Only a re-emission delta whose OWN arguments are
        // complete JSON may reset the slot.
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        // first fragment happens to be complete JSON (parseable boundary)
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "id": "call_1",
            "type": "function",
            "function": { "name": "read_file", "arguments": "{\"a\": 1}" }
        }))]);
        // continuation delta echoes the same id+name but carries only a
        // fragment — NOT complete JSON on its own — so it must NOT reset;
        // the slot keeps concatenating
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "id": "call_1",
            "type": "function",
            "function": { "name": "read_file", "arguments": ",\"b\": 2}" }
        }))]);
        assert_eq!(
            p.active[0].args,
            "{\"a\": 1},\"b\": 2}",
            "continuation echo must append, not reset, the accumulated args"
        );
        let result = p.finish();
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].id, "call_1");
        assert_eq!(result.tool_calls[0].function.name, "read_file");
    }

    #[test]
    fn tool_delta_unparseable_args_fall_back_to_empty_object() {
        // Document the safe-fallback in finalize(): if accumulated arguments
        // are malformed (a continuation can't be reconstructed into JSON),
        // the call still goes through with `{}` rather than panicking — the
        // tool layer then reports a missing-argument error instead of
        // crashing the turn.
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "id": "call_1",
            "type": "function",
            "function": { "name": "read_file", "arguments": "{\"path\": \"a\"}" }
        }))]);
        // continuation WITHOUT identity: gate C intentionally doesn't reset
        // here (no call start), so the fragments concatenate into garbage
        p.feed_events(vec![StreamEvent::ToolDelta(json!({
            "index": 0,
            "function": { "arguments": "{\"path\": \"b\"}" }
        }))]);
        let result = p.finish();
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(
            result.tool_calls[0].function.arguments,
            json!({}),
            "malformed accumulated args must fall back to {{}}"
        );
    }

    #[test]
    fn pipeline_enforces_and_tracks_think() {
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);

        let ev1 = p.feed_events(vec![StreamEvent::Chunk("OK, let me ".to_string())]);
        assert_eq!(ev1, vec![StreamEvent::Chunk("ok, let me ".to_string())]);
        assert!(!p.is_thinking());

        let ev2 = p.feed_events(vec![StreamEvent::Chunk("<think>hmm".to_string())]);
        assert!(ev2.contains(&StreamEvent::ThinkOpen));
        assert!(p.is_thinking());

        let ev3 = p.feed_events(vec![StreamEvent::Chunk(" the user wants tools</think>".to_string())]);
        assert!(ev3.contains(&StreamEvent::ThinkClose));
        assert!(!p.is_thinking());
        assert_eq!(p.content, "ok, let me <think>hmm the user wants tools</think>");
    }

    #[test]
    fn pipeline_tracks_think_across_tag_splits() {
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        p.feed_events(vec![StreamEvent::Chunk("a <th".to_string())]);
        assert!(!p.is_thinking());
        p.feed_events(vec![StreamEvent::Chunk("ink>b".to_string())]);
        assert!(p.is_thinking(), "think block must register after boundary split");
    }

    #[test]
    fn pipeline_pairs_tool_call_deltas() {
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        p.feed_events(vec![
            StreamEvent::ToolDelta(json!({"index": 0, "id": "call_1", "type": "function", "function": {"name": "write_file", "arguments": ""}})),
            StreamEvent::ToolDelta(json!({"index": 0, "function": {"arguments": "{\"path\":\"C:/x"}})),
            StreamEvent::ToolDelta(json!({"index": 0, "function": {"arguments": "/y.txt\",\"content\":\"hi\"}"}})),
        ]);
        let r = p.finish();
        assert_eq!(r.tool_calls.len(), 1);
        let tc = &r.tool_calls[0];
        assert_eq!(tc.id, "call_1");
        assert_eq!(tc.call_type, "function");
        assert_eq!(tc.function.name, "write_file");
        assert_eq!(tc.function.arguments["path"], "C:/x/y.txt");
        assert_eq!(tc.function.arguments["content"], "hi");
        assert!(r.steering.is_none());
    }

    #[test]
    fn pipeline_finish_flushes_formatter_tail() {
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        // trailing backticks are held back by the enforcer until finish
        p.feed_events(vec![StreamEvent::Chunk("here's code: ``".to_string())]);
        assert_eq!(p.content, "here's code: ");
        let r = p.finish();
        assert_eq!(r.content, "here's code: ``");
    }

    #[test]
    fn empty_content_streams_yield_empty_result() {
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        for _ in 0..200 {
            p.feed_events(vec![StreamEvent::Chunk(String::new())]);
        }
        let mut d = OllamaDecoder::new();
        p.feed_events(d.feed(br#"{"message":{"content":""},"done":true}
"#));
        let r = p.finish();
        assert!(r.content.is_empty());
        assert!(r.tool_calls.is_empty());
    }

    #[test]
    fn pipeline_steer_drains_typed_input() {
        let (tx, rx) = mpsc::channel::<String>();
        let mut p = StreamPipeline::new(&rx);
        assert!(p.poll_steer().is_none());
        tx.send("stop that".to_string()).unwrap();
        assert_eq!(p.poll_steer(), Some("stop that".to_string()));
        assert!(p.poll_steer().is_none());
    }

    #[test]
    fn pipeline_steered_finish_preserves_partial_content_and_calls() {
        let rx = channel();
        let mut p = StreamPipeline::new(&rx);
        p.feed_events(vec![
            StreamEvent::Chunk("partial ".to_string()),
            StreamEvent::ToolDelta(json!({"index": 0, "function": {"name": "grep", "arguments": "{\"pattern\":"}})),
        ]);
        let r = p.finish_steered("go left".to_string());
        assert_eq!(r.content, "partial ");
        assert_eq!(r.steering.as_deref(), Some("go left"));
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].function.name, "grep");
    }

    #[test]
    fn tool_payload_slice_helper() {
        let v = json!([{"name": "read_file"}, {"name": "grep"}]);
        assert_eq!(tool_payload_slice(&v).len(), 2);
        let not_array = json!({"nope": true});
        assert_eq!(tool_payload_slice(&not_array).len(), 0);
    }
}
