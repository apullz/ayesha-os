import { useEffect, useState } from 'react';
import { FlatList, StyleSheet, Text, View } from 'react-native';
import { Star } from 'lucide-react-native';
import { LinearGradient } from 'expo-linear-gradient';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, font, radius, spacing } from '../theme';
import type { ChatHistory } from '../types';
import { loadChats } from '../lib/storage';

function formatDate(ts: number) {
  return new Date(ts).toLocaleDateString();
}

export function StarsScreen() {
  const insets = useSafeAreaInsets();
  const [chats, setChats] = useState<ChatHistory[]>([]);

  useEffect(() => {
    loadChats().then(setChats);
  }, []);

  return (
    <LinearGradient
      colors={colors.dreamGrad}
      start={{ x: 0, y: 0 }}
      end={{ x: 1, y: 1 }}
      style={styles.flex}
    >
      <View style={[styles.content, { paddingTop: insets.top }]}>
        <View style={styles.titleRow}>
          <Star size={20} color={colors.accentForeground} fill={colors.accent} />
          <Text style={styles.title}>Stars</Text>
        </View>
        <Text style={styles.subtitle}>
          Your past conversations, saved on this device.
        </Text>
        <FlatList
          data={chats}
          keyExtractor={(c) => c.id}
          contentContainerStyle={styles.list}
          ListEmptyComponent={
            <View style={styles.empty}>
              <Text style={styles.emptyText}>no chats saved yet ✨</Text>
            </View>
          }
          renderItem={({ item }) => (
            <View style={styles.card}>
              <Text style={styles.cardTitle}>{item.title}</Text>
              <Text style={styles.cardMeta}>
                {item.messages.length} messages · {formatDate(item.updatedAt)}
              </Text>
            </View>
          )}
        />
      </View>
    </LinearGradient>
  );
}

const styles = StyleSheet.create({
  flex: {
    flex: 1,
  },
  content: {
    flex: 1,
    paddingHorizontal: spacing.xl,
  },
  titleRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.sm,
    paddingTop: spacing.lg,
    paddingBottom: spacing.xs,
  },
  title: {
    fontFamily: font.bold,
    fontSize: 24,
    color: colors.foreground,
  },
  subtitle: {
    fontFamily: font.regular,
    fontSize: 14,
    color: colors.mutedForeground,
    marginBottom: spacing.lg,
  },
  list: {
    paddingBottom: spacing.xl,
  },
  card: {
    backgroundColor: colors.card,
    borderRadius: radius.lg,
    padding: spacing.lg,
    marginBottom: spacing.md,
    borderWidth: 1,
    borderColor: colors.border,
  },
  cardTitle: {
    fontFamily: font.semibold,
    fontSize: 16,
    color: colors.foreground,
    marginBottom: spacing.xs,
  },
  cardMeta: {
    fontFamily: font.regular,
    fontSize: 13,
    color: colors.mutedForeground,
  },
  empty: {
    paddingTop: spacing.xxl * 2,
    alignItems: 'center',
  },
  emptyText: {
    fontFamily: font.regular,
    fontStyle: 'italic',
    color: colors.mutedForeground,
  },
});
