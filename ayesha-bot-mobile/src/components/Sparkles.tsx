import { useEffect, useMemo } from 'react';
import { Animated, StyleSheet, useAnimatedValue } from 'react-native';
import Svg, { Path } from 'react-native-svg';
import { colors } from '../theme';

interface SparkleConfig {
  left: number;
  top: number;
  size: number;
  delay: number;
  duration: number;
}

function Star({ size, color }: { size: number; color: string }) {
  return (
    <Svg width={size} height={size} viewBox="0 0 24 24" fill={color}>
      <Path d="M12 0c.6 4.9 2.1 6.4 7 7-4.9.6-6.4 2.1-7 7-.6-4.9-2.1-6.4-7-7 4.9-.6 6.4-2.1 7-7z" />
    </Svg>
  );
}

function generateSparkles(count: number): SparkleConfig[] {
  let seed = 7;
  const rand = () => {
    seed = (seed * 9301 + 49297) % 233280;
    return seed / 233280;
  };
  return Array.from({ length: count }, () => ({
    left: Math.round(rand() * 100),
    top: Math.round(rand() * 100),
    size: 8 + Math.round(rand() * 16),
    delay: rand() * 2.4,
    duration: 2 + rand() * 2,
  }));
}

function Sparkle({ config }: { config: SparkleConfig }) {
  const opacity = useAnimatedValue(0.2);
  const scale = useAnimatedValue(0.7);

  useEffect(() => {
    const loop = Animated.loop(
      Animated.parallel([
        Animated.sequence([
          Animated.timing(opacity, { toValue: 1, duration: config.duration * 0.5, useNativeDriver: true }),
          Animated.timing(opacity, { toValue: 0.2, duration: config.duration * 0.5, useNativeDriver: true }),
        ]),
        Animated.sequence([
          Animated.timing(scale, { toValue: 1, duration: config.duration * 0.5, useNativeDriver: true }),
          Animated.timing(scale, { toValue: 0.7, duration: config.duration * 0.5, useNativeDriver: true }),
        ]),
      ]),
    );
    const timer = setTimeout(() => loop.start(), config.delay * 1000);
    return () => {
      clearTimeout(timer);
      loop.stop();
    };
  }, [opacity, scale, config]);

  return (
    <Animated.View
      pointerEvents="none"
      style={[
        styles.sparkle,
        {
          left: `${config.left}%`,
          top: `${config.top}%`,
          opacity,
          transform: [{ scale }],
        },
      ]}
    >
      <Star size={config.size} color={colors.sparkle} />
    </Animated.View>
  );
}

export function Sparkles({ count = 14 }: { count?: number }) {
  const sparkles = useMemo(() => generateSparkles(count), [count]);

  return (
    <Animated.View pointerEvents="none" style={StyleSheet.absoluteFill}>
      {sparkles.map((s, i) => (
        <Sparkle key={i} config={s} />
      ))}
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  sparkle: {
    position: 'absolute',
  },
});
