import { Image, StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import { Star } from 'lucide-react-native';
import { colors, font, radius, spacing } from '../theme';

export function ChatHeader({ onFavorite }: { onFavorite?: () => void }) {
  return (
    <View style={styles.container}>
      <View style={styles.identity}>
        <Image
          source={require('../../assets/ayesha-bot.png')}
          style={styles.avatar}
          resizeMode="cover"
        />
        <View style={styles.namePlate}>
          <Text style={styles.name}>Ayesha bot</Text>
        </View>
      </View>

      <TouchableOpacity
        activeOpacity={0.7}
        onPress={onFavorite}
        style={styles.favButton}
        accessibilityRole="button"
        accessibilityLabel="Favorite this chat"
      >
        <Star size={24} color={colors.card} fill={colors.card} />
      </TouchableOpacity>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
    paddingHorizontal: spacing.xl,
    paddingTop: spacing.lg,
    paddingBottom: spacing.md,
  },
  identity: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.md,
  },
  avatar: {
    width: 64,
    height: 64,
    borderRadius: radius.lg,
    backgroundColor: colors.card,
  },
  namePlate: {
    backgroundColor: colors.card,
    borderRadius: radius.lg,
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.sm,
  },
  name: {
    fontFamily: font.semibold,
    fontSize: 18,
    color: colors.foreground,
  },
  favButton: {
    width: 48,
    height: 48,
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: radius.lg,
    backgroundColor: colors.accent,
  },
});
