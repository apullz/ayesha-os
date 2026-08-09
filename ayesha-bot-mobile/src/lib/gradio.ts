import type { GradioTurn } from '../types';

export const SPACE_URL = 'https://apullz-ayesha-bot.hf.space';
const API_PREFIX = '/gradio_api';
const FN_NAME = 'respond';

export interface StreamHandlers {
  onChunk: (text: string) => void;
  onDone: (finalText: string) => void;
  onError: (error: Error) => void;
}

interface InitResponse {
  event_id: string;
}

function ssePayloadToText(payload: unknown): string {
  if (Array.isArray(payload) && typeof payload[0] === 'string') {
    return payload[0];
  }
  if (payload && typeof payload === 'object') {
    const maybe = payload as Record<string, unknown>;
    if (typeof maybe.data === 'string') return maybe.data;
  }
  return '';
}

/**
 * Streams a chat completion from the apullz/ayesha-bot HF Space (Gradio SSE v3).
 *
 * 1. POST /gradio_api/call/respond with { data: [message, history] } -> { event_id }
 * 2. GET  /gradio_api/call/respond/{event_id} as an SSE stream.
 *
 * Each `event: generating` carries the FULL accumulated assistant text (not a
 * delta), so the UI replaces the bot bubble on every chunk. A final
 * `event: complete` signals the end.
 *
 * Uses XMLHttpRequest on purpose: it delivers incremental responseText on
 * both native (iOS/Android) and web, the most reliable way to consume SSE in
 * React Native.
 */
export function streamChat(
  message: string,
  history: GradioTurn[],
  handlers: StreamHandlers,
  signal?: AbortSignal,
): void {
  let aborted = false;
  let done = false;
  let xhr: XMLHttpRequest | null = null;

  const abort = () => {
    aborted = true;
    xhr?.abort();
  };
  if (signal) {
    if (signal.aborted) abort();
    else signal.addEventListener('abort', abort, { once: true });
  }

  fetch(`${SPACE_URL}${API_PREFIX}/call/${FN_NAME}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ data: [message, history] }),
    signal,
  })
    .then((res) => {
      if (!res.ok) throw new Error(`API responded ${res.status}`);
      return res.json() as Promise<InitResponse>;
    })
    .then(({ event_id }) => {
      if (aborted) return;
      if (!event_id) throw new Error('No event_id from API');

      xhr = new XMLHttpRequest();
      xhr.open('GET', `${SPACE_URL}${API_PREFIX}/call/${FN_NAME}/${event_id}`);

      let lastIndex = 0;
      let pendingEvent = '';
      let dataLines: string[] = [];

      const dispatchEvent = () => {
        if (!pendingEvent || done) return;
        const name = pendingEvent;
        const payload = dataLines.join('\n');
        dataLines = [];
        pendingEvent = '';

        if (name === 'generating') {
          const text = ssePayloadToText(payload);
          if (text) handlers.onChunk(text);
        } else if (name === 'complete') {
          done = true;
          handlers.onDone(ssePayloadToText(payload));
        }
      };

      xhr.onreadystatechange = () => {
        if (xhr!.readyState < 3) return;
        const text = xhr!.responseText as string;
        const chunk = text.slice(lastIndex);
        lastIndex = text.length;

        const lines = chunk.split('\n');
        for (const line of lines) {
          if (line === '') {
            dispatchEvent();
          } else if (line.startsWith('event:')) {
            pendingEvent = line.slice(6).trim();
          } else if (line.startsWith('data:')) {
            dataLines.push(line.slice(5).trim());
          }
        }
      };

      xhr.onloadend = () => {
        if (aborted || done) return;
        done = true;
        if (pendingEvent === 'generating') {
          handlers.onDone(ssePayloadToText(dataLines.join('\n')));
        } else {
          dispatchEvent();
        }
      };

      xhr.onerror = () => {
        if (aborted || done) return;
        done = true;
        handlers.onError(new Error('Stream connection failed'));
      };

      xhr.send();
    })
    .catch((err: unknown) => {
      if (aborted || done) return;
      done = true;
      handlers.onError(err instanceof Error ? err : new Error(String(err)));
    });
}
