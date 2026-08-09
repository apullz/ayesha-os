import { useEffect } from 'react';
import { Animated, StyleSheet, Text, View, useAnimatedValue } from 'react-native';
import { colors, font, radius, spacing } from '../theme';
import type { ChatRole } from '../types';

export function ChatBubble({
  role,
  text,
  streaming,
}: {
  role: ChatRole;
  text: string;
  streaming?: boolean;
}) {
  const opacity = useAnimatedValue(0);
  const translateY = useAnimatedValue(8);

  useEffect(() => {
    Animated.parallel([
      Animated.timing(opacity, { toValue: 1, duration: 300, useNativeDriver: true }),
      Animated.timing(translateY, { toValue: 0, duration: 300, useNativeDriver: true }),
    ]).start();
  }, [opacity, translateY]);

  const isBot = role === 'assistant';
  const backgroundColor = isBot ? colors.primary : colors.secondary;
  const textColor = isBot ? colors.primaryForeground : colors.secondaryForeground;

  return (
    <Animated.View
      style={[
        styles.row,
        isBot ? styles.rowLeft : styles.rowRight,
        { opacity, transform: [{ translateY }] },
      ]}
    >
      <View
        style={[
          styles.bubble,
          { backgroundColor },
          isBot ? styles.bubbleBot : styles.bubbleUser,
        ]}
      >
        <View
          style={[
            styles.tail,
            { backgroundColor },
            isBot ? styles.tailBot : styles.tailUser,
          ]}
        />
        <Text style={[styles.text, { color: textColor }]}>
          {text}
          {streaming ? ' ✨' : ''}
        </Text>
      </View>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  row: {
    flexDirection: 'row',
    marginVertical: spacing.xs,
  },
  rowLeft: {
    justifyContent: 'flex-start',
  },
  rowRight: {
    justifyContent: 'flex-end',
  },
  bubble: {
    position: 'relative',
    maxWidth: '78%',
    paddingHorizontal: spacing.xl,
    paddingVertical: spacing.lg,
  },
  bubbleBot: {
    borderBottomLeftRadius: 8,
    borderTopRightRadius: radius.xxl,
    borderTopLeftRadius: radius.xxl,
    borderBottomRightRadius: radius.xxl,
  },
  bubbleUser: {
    borderBottomRightRadius: 8,
    borderTopRightRadius: radius.xxl,
    borderTopLeftRadius: radius.xxl,
    borderBottomLeftRadius: radius.xxl,
  },
  tail: {
    position: 'absolute',
    bottom: 0,
    width: 16,
    height: 16,
  },
  tailBot: {
    left: -4,
    borderBottomLeftRadius: 4,
    transform: [{ skewX: '35deg' }],
  },
  tailUser: {
    right: -4,
    borderBottomRightRadius: 4,
    transform: [{ skewX: '-35deg' }],
  },
  text: {
    fontFamily: font.medium,
    fontSize: 16,
    lineHeight: 22,
  },
});
