import { useState } from 'react';
import {
  StyleSheet,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import { Heart } from 'lucide-react-native';
import { LinearGradient } from 'expo-linear-gradient';
import * as Haptics from 'expo-haptics';
import { colors, font, radius, spacing } from '../theme';

export function ChatInput({
  onSend,
  disabled,
}: {
  onSend: (text: string) => void;
  disabled?: boolean;
}) {
  const [value, setValue] = useState('');

  function submit() {
    const text = value.trim();
    if (!text || disabled) return;
    Haptics.impactAsync(Haptics.ImpactFeedbackStyle.Light).catch(() => {});
    onSend(text);
    setValue('');
  }

  return (
    <View style={styles.container}>
      <View style={styles.row}>
        <TextInput
          value={value}
          onChangeText={setValue}
          placeholder="Type a magical message..."
          placeholderTextColor={colors.mutedForeground}
          multiline
          style={styles.input}
          returnKeyType="send"
          blurOnSubmit={false}
          onSubmitEditing={() => {
            if (!value.includes('\n')) submit();
          }}
          accessibilityLabel="Type a magical message"
        />
        <TouchableOpacity
          activeOpacity={0.8}
          onPress={submit}
          disabled={disabled}
          accessibilityRole="button"
          accessibilityLabel="Send message"
          style={styles.sendButton}
        >
          <LinearGradient
            colors={colors.glitter}
            start={{ x: 0, y: 0.5 }}
            end={{ x: 1, y: 0.5 }}
            style={styles.sendGradient}
          >
            <Heart size={28} color={colors.card} fill={colors.card} />
          </LinearGradient>
        </TouchableOpacity>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    paddingHorizontal: spacing.lg,
    paddingBottom: spacing.sm,
    paddingTop: spacing.xs,
  },
  row: {
    flexDirection: 'row',
    alignItems: 'flex-end',
    gap: spacing.sm,
  },
  input: {
    flex: 1,
    minHeight: 56,
    maxHeight: 140,
    paddingHorizontal: spacing.xl,
    paddingVertical: spacing.lg,
    borderRadius: radius.lg,
    backgroundColor: colors.card,
    color: colors.foreground,
    fontFamily: font.regular,
    fontSize: 16,
    borderWidth: 1,
    borderColor: colors.border,
  },
  sendButton: {
    width: 56,
    height: 56,
    borderRadius: radius.lg,
    overflow: 'hidden',
  },
  sendGradient: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
});
