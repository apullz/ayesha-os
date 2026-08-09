export type ChatRole = 'user' | 'assistant';

export interface Message {
  id: string;
  role: ChatRole;
  text: string;
  timestamp: number;
}

export interface ChatHistory {
  id: string;
  title: string;
  messages: Message[];
  createdAt: number;
  updatedAt: number;
}

export interface GradioTurn {
  role: ChatRole;
  content: string;
}

export type TabId = 'chat' | 'stars' | 'settings';
