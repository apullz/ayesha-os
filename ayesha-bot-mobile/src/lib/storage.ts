import AsyncStorage from '@react-native-async-storage/async-storage';
import type { ChatHistory, Message } from '../types';

const CHATS_KEY = '@ayesha/chats';

export async function loadChats(): Promise<ChatHistory[]> {
  try {
    const raw = await AsyncStorage.getItem(CHATS_KEY);
    return raw ? (JSON.parse(raw) as ChatHistory[]) : [];
  } catch {
    return [];
  }
}

export async function saveChats(chats: ChatHistory[]): Promise<void> {
  try {
    await AsyncStorage.setItem(CHATS_KEY, JSON.stringify(chats));
  } catch {
    // ignore persistence failures
  }
}

export async function loadOrCreateChat(): Promise<ChatHistory> {
  const chats = await loadChats();
  if (chats.length > 0) {
    return chats[0];
  }
  const chat = createChat();
  await saveChats([chat]);
  return chat;
}

export function createChat(): ChatHistory {
  const now = Date.now();
  return {
    id: `chat-${now}`,
    title: 'Magical Chat',
    messages: [],
    createdAt: now,
    updatedAt: now,
  };
}

export function makeMessage(role: Message['role'], text: string): Message {
  return {
    id: `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    role,
    text,
    timestamp: Date.now(),
  };
}

export async function upsertMessages(
  chatId: string,
  messages: Message[],
): Promise<void> {
  const chats = await loadChats();
  const idx = chats.findIndex((c) => c.id === chatId);
  if (idx >= 0) {
    chats[idx].messages = messages;
    chats[idx].updatedAt = Date.now();
  } else {
    chats.push({
      ...createChat(),
      id: chatId,
      messages,
    });
  }
  await saveChats(chats);
}
