import { useCallback, useEffect, useRef, useState } from 'react';
import {
  ActivityIndicator,
  FlatList,
  KeyboardAvoidingView,
  Platform,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { LinearGradient } from 'expo-linear-gradient';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, font, spacing } from '../theme';
import type { ChatHistory, Message } from '../types';
import { ChatBubble } from '../components/ChatBubble';
import { ChatHeader } from '../components/ChatHeader';
import { ChatInput } from '../components/ChatInput';
import { Sparkles } from '../components/Sparkles';
import { streamChat } from '../lib/gradio';
import {
  loadOrCreateChat,
  makeMessage,
  upsertMessages,
} from '../lib/storage';

const BOT_WELCOME =
  'Hello Fox! ✨ What magical adventure should we dream up today?';

function toGradioHistory(messages: Message[]) {
  return messages.map((m) => ({ role: m.role, content: m.text }));
}

export function ChatScreen() {
  const insets = useSafeAreaInsets();
  const [chat, setChat] = useState<ChatHistory | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [loading, setLoading] = useState(true);
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const listRef = useRef<FlatList<Message>>(null);
  const chatRef = useRef<ChatHistory | null>(null);
  const messagesRef = useRef<Message[]>([]);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    chatRef.current = chat;
  }, [chat]);

  useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, []);

  useEffect(() => {
    let mounted = true;
    loadOrCreateChat().then((c) => {
      if (!mounted) return;
      setChat(c);
      if (c.messages.length === 0) {
        const welcome = makeMessage('assistant', BOT_WELCOME);
        const start = [welcome];
        messagesRef.current = start;
        setMessages(start);
        upsertMessages(c.id, start);
      } else {
        messagesRef.current = c.messages;
        setMessages(c.messages);
      }
      setLoading(false);
    });
    return () => {
      mounted = false;
    };
  }, []);

  const scrollToEnd = useCallback(() => {
    requestAnimationFrame(() => {
      listRef.current?.scrollToEnd({ animated: true });
    });
  }, []);

  useEffect(() => {
    scrollToEnd();
  }, [messages, scrollToEnd]);

  function handleSend(text: string) {
    if (streaming || !chatRef.current) return;

    const userMsg = makeMessage('user', text);
    const botMsg = makeMessage('assistant', '');
    const next = [...messages, userMsg, botMsg];
    messagesRef.current = next;
    setMessages(next);
    setError(null);
    setStreaming(true);

    const history = toGradioHistory(next.slice(0, -2));

    const controller = new AbortController();
    abortRef.current = controller;

    const update = (botText: string) => {
      const prev = messagesRef.current;
      const copy = [...prev];
      const idx = copy.length - 1;
      if (idx >= 0 && copy[idx].id === botMsg.id) {
        copy[idx] = { ...copy[idx], text: botText };
      }
      messagesRef.current = copy;
      setMessages(copy);
    };

    const persist = () => {
      const chatId = chatRef.current?.id;
      if (chatId) {
        const current = messagesRef.current;
        const finalIdx = current.findIndex((m) => m.id === botMsg.id);
        const snapshot =
          finalIdx >= 0 ? current.slice(0, finalIdx + 1) : current;
        upsertMessages(chatId, snapshot);
      }
    };

    streamChat(
      text,
      history,
      {
        onChunk: (chunk) => update(chunk),
        onDone: (finalText) => {
          update(finalText);
          setStreaming(false);
          persist();
        },
        onError: (e) => {
          update(
            'Hmm, the magic server drifted away. ✨ Try again in a moment — the sparkles are regrouping.',
          );
          setError(e.message);
          setStreaming(false);
          persist();
        },
      },
      controller.signal,
    );
  }

  return (
    <KeyboardAvoidingView
      style={styles.flex}
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      keyboardVerticalOffset={0}
    >
      <LinearGradient
        colors={colors.dreamGrad}
        start={{ x: 0, y: 0 }}
        end={{ x: 1, y: 1 }}
        style={styles.gradient}
      >
        <View style={[styles.content, { paddingTop: insets.top }]}>
          <Sparkles count={16} />
          <ChatHeader onFavorite={() => {}} />

          {loading ? (
            <View style={styles.center}>
              <ActivityIndicator size="large" color={colors.primaryForeground} />
            </View>
          ) : (
            <FlatList
              ref={listRef}
              data={messages}
              keyExtractor={(m) => m.id}
              renderItem={({ item, index }) => (
                <ChatBubble
                  role={item.role}
                  text={item.text}
                  streaming={streaming && index === messages.length - 1}
                />
              )}
              contentContainerStyle={styles.listContent}
              style={styles.flex}
              onContentSizeChange={scrollToEnd}
            />
          )}

          {streaming && (
            <View style={styles.thinkingRow}>
              <ActivityIndicator size="small" color={colors.mutedForeground} />
              <Text style={styles.thinkingText}>weaving magic…</Text>
            </View>
          )}
          {error && (
            <Text style={styles.errorText}>last attempt failed: {error}</Text>
          )}

          <ChatInput onSend={handleSend} disabled={loading || streaming} />
        </View>
      </LinearGradient>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  flex: {
    flex: 1,
  },
  gradient: {
    flex: 1,
  },
  content: {
    flex: 1,
    paddingBottom: spacing.sm,
  },
  center: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  listContent: {
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.lg,
  },
  thinkingRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.sm,
    paddingHorizontal: spacing.xl,
    paddingVertical: spacing.xs,
  },
  thinkingText: {
    fontFamily: font.regular,
    fontStyle: 'italic',
    color: colors.mutedForeground,
  },
  errorText: {
    fontFamily: font.regular,
    fontSize: 12,
    color: colors.secondaryForeground,
    paddingHorizontal: spacing.xl,
  },
});
