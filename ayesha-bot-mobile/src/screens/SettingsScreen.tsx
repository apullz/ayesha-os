import { useState } from 'react';
import { Alert, StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import { Settings as SettingsIcon, Trash2 } from 'lucide-react-native';
import { LinearGradient } from 'expo-linear-gradient';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { colors, font, radius, spacing } from '../theme';
import { saveChats } from '../lib/storage';

export function SettingsScreen() {
  const insets = useSafeAreaInsets();
  const [cleared, setCleared] = useState(false);

  function clearHistory() {
    Alert.alert(
      'Clear chat history?',
      'This wipes all saved conversations on this device. The sparkles cannot bring them back.',
      [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Clear',
          style: 'destructive',
          onPress: () => {
            saveChats([]).then(() => setCleared(true));
          },
        },
      ],
    );
  }

  return (
    <LinearGradient
      colors={colors.dreamGrad}
      start={{ x: 0, y: 0 }}
      end={{ x: 1, y: 1 }}
      style={styles.flex}
    >
      <View style={[styles.content, { paddingTop: insets.top }]}>
        <View style={styles.titleRow}>
          <SettingsIcon size={20} color={colors.accentForeground} />
          <Text style={styles.title}>Settings</Text>
        </View>

        <View style={styles.section}>
          <Text style={styles.label}>Model</Text>
          <Text style={styles.value}>apullz/ayesha-bot · HF Space (Ollama)</Text>
          <Text style={styles.hint}>Responses stream straight from the cloud space.</Text>
        </View>

        <View style={styles.section}>
          <Text style={styles.label}>Storage</Text>
          <Text style={styles.value}>On-device only — no accounts.</Text>
          <Text style={styles.hint}>Conversations are stored locally and never leave your phone.</Text>
        </View>

        {cleared && <Text style={styles.cleared}>history cleared ✨</Text>}

        <TouchableOpacity
          activeOpacity={0.7}
          onPress={clearHistory}
          style={styles.dangerButton}
          accessibilityRole="button"
        >
          <Trash2 size={18} color={colors.secondaryForeground} />
          <Text style={styles.dangerText}>Clear chat history</Text>
        </TouchableOpacity>

        <Text style={styles.footer}>Ayesha Bot v1.0.0</Text>
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
    paddingBottom: spacing.lg,
  },
  title: {
    fontFamily: font.bold,
    fontSize: 24,
    color: colors.foreground,
  },
  section: {
    backgroundColor: colors.card,
    borderRadius: radius.lg,
    padding: spacing.lg,
    marginBottom: spacing.md,
    borderWidth: 1,
    borderColor: colors.border,
  },
  label: {
    fontFamily: font.semibold,
    fontSize: 14,
    color: colors.mutedForeground,
    textTransform: 'uppercase',
    letterSpacing: 1,
    marginBottom: spacing.xs,
  },
  value: {
    fontFamily: font.medium,
    fontSize: 16,
    color: colors.foreground,
    marginBottom: spacing.xs,
  },
  hint: {
    fontFamily: font.regular,
    fontSize: 13,
    color: colors.mutedForeground,
  },
  cleared: {
    fontFamily: font.regular,
    fontStyle: 'italic',
    color: colors.accentForeground,
    marginBottom: spacing.md,
  },
  dangerButton: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: spacing.sm,
    backgroundColor: colors.secondary,
    borderRadius: radius.lg,
    paddingVertical: spacing.lg,
  },
  dangerText: {
    fontFamily: font.semibold,
    fontSize: 16,
    color: colors.secondaryForeground,
  },
  footer: {
    fontFamily: font.regular,
    fontSize: 12,
    color: colors.mutedForeground,
    textAlign: 'center',
    marginTop: spacing.xxl,
  },
});
