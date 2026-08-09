import { StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import { MessagesSquare, Settings, Sparkles } from 'lucide-react-native';
import { colors, font, spacing } from '../theme';
import type { TabId } from '../types';

const items: { id: TabId; label: string; icon: typeof MessagesSquare }[] = [
  { id: 'chat', label: 'Chat', icon: MessagesSquare },
  { id: 'stars', label: 'Stars', icon: Sparkles },
  { id: 'settings', label: 'Settings', icon: Settings },
];

export function BottomNav({
  active,
  onChange,
}: {
  active: TabId;
  onChange: (id: TabId) => void;
}) {
  return (
    <View style={styles.container}>
      {items.map(({ id, label, icon: Icon }) => {
        const isActive = active === id;
        return (
          <TouchableOpacity
            key={id}
            activeOpacity={0.7}
            onPress={() => onChange(id)}
            style={styles.item}
            accessibilityRole="tab"
            accessibilityState={{ selected: isActive }}
          >
            <Icon
              size={24}
              color={isActive ? colors.foreground : colors.mutedForeground}
              fill={isActive ? colors.accent : 'transparent'}
            />
            <Text
              style={[
                styles.label,
                isActive ? styles.labelActive : styles.labelInactive,
              ]}
            >
              {label}
            </Text>
          </TouchableOpacity>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-around',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm,
    borderTopWidth: 1,
    borderTopColor: colors.border,
    backgroundColor: colors.card,
  },
  item: {
    alignItems: 'center',
    gap: 2,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.xs,
  },
  label: {
    fontSize: 12,
  },
  labelActive: {
    fontFamily: font.bold,
    color: colors.foreground,
  },
  labelInactive: {
    fontFamily: font.regular,
    fontStyle: 'italic',
    color: colors.mutedForeground,
  },
});
