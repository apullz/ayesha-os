export const colors = {
  // Magical pastel palette (ported from the web prototype's oklch values)
  background: '#F7EAF2',
  foreground: '#7A4252',
  card: '#F8EEF5',
  cardForeground: '#7A4252',
  primary: '#D5B6E4', // lavender = bot bubble
  primaryForeground: '#6C4B8E',
  secondary: '#E9B6BB', // pink = user bubble
  secondaryForeground: '#8E4A58',
  muted: '#EBD8E8',
  mutedForeground: '#A68A9E',
  accent: '#BFE3D9', // mint
  accentForeground: '#4C8A7A',
  border: '#DFCBDC',
  input: '#E6D3E0',
  ring: '#C9A8DB',
  sparkle: '#FFFFFF',
  dreamGrad: ['#FBE9F2', '#F7E7F5', '#E8F0F7', '#DFF2F0'],
  glitter: ['#F0A8C8', '#E6B8E6', '#C8D8F0', '#A8E0D8', '#E6B8E6', '#F0A8C8'],
} as const;

export const spacing = {
  xs: 4,
  sm: 8,
  md: 12,
  lg: 16,
  xl: 20,
  xxl: 24,
} as const;

export const radius = {
  sm: 8,
  md: 12,
  lg: 16,
  xl: 20,
  xxl: 28,
} as const;

export const font = {
  regular: 'Quicksand_400Regular',
  medium: 'Quicksand_500Medium',
  semibold: 'Quicksand_600SemiBold',
  bold: 'Quicksand_700Bold',
} as const;
