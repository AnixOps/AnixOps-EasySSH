/**
 * EasySSH Design Tokens
 * Apple-level native UI design system for SSH client product line
 *
 * @version 1.0.0
 * @author EasySSH Design System
 */

// ============================================================================
// BASE TOKENS - Primitive values
// ============================================================================

export const baseColors = {
  // Neutral Scale (Apple-style neutral with slight warmth)
  neutral: {
    0: '#FFFFFF',
    50: '#FAFAFA',
    100: '#F5F5F5',
    200: '#EBEBEB',
    300: '#E0E0E0',
    400: '#C4C4C4',
    500: '#9E9E9E',
    600: '#757575',
    700: '#525252',
    800: '#363636',
    900: '#1A1A1A',
    950: '#0D0D0D',
    1000: '#000000',
  },

  // Brand Colors - Professional, trustworthy blue
  brand: {
    50: '#EFF6FF',
    100: '#DBEAFE',
    200: '#BFDBFE',
    300: '#93C5FD',
    400: '#60A5FA',
    500: '#3B82F6',
    600: '#2563EB',
    700: '#1D4ED8',
    800: '#1E40AF',
    900: '#1E3A8A',
    950: '#172554',
  },

  // Semantic Colors
  success: {
    50: '#F0FDF4',
    100: '#DCFCE7',
    200: '#BBF7D0',
    300: '#86EFAC',
    400: '#4ADE80',
    500: '#22C55E',
    600: '#16A34A',
    700: '#15803D',
    800: '#166534',
    900: '#14532D',
  },

  warning: {
    50: '#FFFBEB',
    100: '#FEF3C7',
    200: '#FDE68A',
    300: '#FCD34D',
    400: '#FBBF24',
    500: '#F59E0B',
    600: '#D97706',
    700: '#B45309',
    800: '#92400E',
    900: '#78350F',
  },

  danger: {
    50: '#FEF2F2',
    100: '#FEE2E2',
    200: '#FECACA',
    300: '#FCA5A5',
    400: '#F87171',
    500: '#EF4444',
    600: '#DC2626',
    700: '#B91C1C',
    800: '#991B1B',
    900: '#7F1D1D',
  },

  // Terminal-specific colors (Xterm 256 compatible)
  terminal: {
    black: '#1E1E1E',
    red: '#E06C75',
    green: '#98C379',
    yellow: '#E5C07B',
    blue: '#61AFEF',
    magenta: '#C678DD',
    cyan: '#56B6C2',
    white: '#DCDCDC',
    brightBlack: '#5C6370',
    brightRed: '#FF6B7A',
    brightGreen: '#B5E08D',
    brightYellow: '#F0D58A',
    brightBlue: '#7BC3FF',
    brightMagenta: '#D78FE6',
    brightCyan: '#6ED4E0',
    brightWhite: '#FFFFFF',
    background: '#1E1E1E',
    foreground: '#DCDCDC',
    cursor: '#528BFF',
    selection: '#264F78',
  },

  // Accent colors for status indicators
  status: {
    online: '#22C55E',
    offline: '#EF4444',
    connecting: '#F59E0B',
    maintenance: '#8B5CF6',
    unknown: '#9CA3AF',
  },
} as const;

// ============================================================================
// TYPOGRAPHY - Inter for UI, JetBrains Mono for code
// ============================================================================

export const typography = {
  fontFamily: {
    sans: ['Inter', 'system-ui', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
    mono: ['JetBrains Mono', 'Fira Code', 'SF Mono', 'Menlo', 'Monaco', 'monospace'],
    display: ['Inter', 'system-ui', 'sans-serif'],
  },

  fontSize: {
    '2xs': '10px',
    xs: '12px',
    sm: '13px',
    base: '14px',
    md: '16px',
    lg: '18px',
    xl: '20px',
    '2xl': '24px',
    '3xl': '30px',
    '4xl': '36px',
  },

  fontWeight: {
    normal: '400',
    medium: '500',
    semibold: '600',
    bold: '700',
  },

  lineHeight: {
    none: '1',
    tight: '1.25',
    snug: '1.375',
    normal: '1.5',
    relaxed: '1.625',
    loose: '2',
  },

  letterSpacing: {
    tighter: '-0.05em',
    tight: '-0.025em',
    normal: '0',
    wide: '0.025em',
    wider: '0.05em',
    widest: '0.1em',
  },

  // Predefined type styles
  styles: {
    display: {
      large: { fontSize: '36px', fontWeight: '600', lineHeight: '1.2', letterSpacing: '-0.02em' },
      medium: { fontSize: '30px', fontWeight: '600', lineHeight: '1.25', letterSpacing: '-0.02em' },
      small: { fontSize: '24px', fontWeight: '600', lineHeight: '1.3', letterSpacing: '-0.01em' },
    },
    headline: {
      large: { fontSize: '20px', fontWeight: '600', lineHeight: '1.4' },
      medium: { fontSize: '18px', fontWeight: '600', lineHeight: '1.4' },
      small: { fontSize: '16px', fontWeight: '600', lineHeight: '1.5' },
    },
    body: {
      large: { fontSize: '16px', fontWeight: '400', lineHeight: '1.6' },
      medium: { fontSize: '14px', fontWeight: '400', lineHeight: '1.5' },
      small: { fontSize: '13px', fontWeight: '400', lineHeight: '1.5' },
    },
    label: {
      large: { fontSize: '14px', fontWeight: '500', lineHeight: '1.4', letterSpacing: '0.01em' },
      medium: { fontSize: '13px', fontWeight: '500', lineHeight: '1.4', letterSpacing: '0.01em' },
      small: { fontSize: '12px', fontWeight: '500', lineHeight: '1.4', letterSpacing: '0.02em' },
    },
    code: {
      regular: { fontSize: '13px', fontWeight: '400', lineHeight: '1.6', fontFamily: 'mono' },
      small: { fontSize: '12px', fontWeight: '400', lineHeight: '1.5', fontFamily: 'mono' },
    },
  },
} as const;

// ============================================================================
// SPACING - 4px grid system
// ============================================================================

export const spacing = {
  0: '0',
  0.5: '2px',
  1: '4px',
  1.5: '6px',
  2: '8px',
  2.5: '10px',
  3: '12px',
  3.5: '14px',
  4: '16px',
  5: '20px',
  6: '24px',
  7: '28px',
  8: '32px',
  9: '36px',
  10: '40px',
  12: '48px',
  14: '56px',
  16: '64px',
  20: '80px',
  24: '96px',
  28: '112px',
  32: '128px',
  36: '144px',
  40: '160px',
  44: '176px',
  48: '192px',
  52: '208px',
  56: '224px',
  60: '240px',
  64: '256px',
  72: '288px',
  80: '320px',
  96: '384px',
} as const;

// ============================================================================
// SHADOWS - Subtle, purposeful depth
// ============================================================================

export const shadows = {
  // Elevation levels
  none: 'none',
  xs: '0 1px 2px 0 rgba(0, 0, 0, 0.03)',
  sm: '0 1px 3px 0 rgba(0, 0, 0, 0.06), 0 1px 2px -1px rgba(0, 0, 0, 0.06)',
  md: '0 4px 6px -1px rgba(0, 0, 0, 0.06), 0 2px 4px -2px rgba(0, 0, 0, 0.06)',
  lg: '0 10px 15px -3px rgba(0, 0, 0, 0.06), 0 4px 6px -4px rgba(0, 0, 0, 0.06)',
  xl: '0 20px 25px -5px rgba(0, 0, 0, 0.06), 0 8px 10px -6px rgba(0, 0, 0, 0.06)',
  '2xl': '0 25px 50px -12px rgba(0, 0, 0, 0.15)',

  // Special shadows
  inner: 'inset 0 2px 4px 0 rgba(0, 0, 0, 0.03)',
  terminal: '0 4px 20px rgba(0, 0, 0, 0.4)',
  card: '0 2px 8px rgba(0, 0, 0, 0.08)',
  popover: '0 4px 16px rgba(0, 0, 0, 0.12)',
  dropdown: '0 8px 24px rgba(0, 0, 0, 0.12)',
  modal: '0 16px 48px rgba(0, 0, 0, 0.18)',
  tooltip: '0 4px 12px rgba(0, 0, 0, 0.15)',
} as const;

// ============================================================================
// BORDERS - Refined, minimalist
// ============================================================================

export const borders = {
  width: {
    none: '0',
    thin: '1px',
    medium: '2px',
    thick: '4px',
  },
  radius: {
    none: '0',
    xs: '2px',
    sm: '4px',
    md: '6px',
    lg: '8px',
    xl: '12px',
    '2xl': '16px',
    '3xl': '24px',
    full: '9999px',
  },
  style: {
    solid: 'solid',
    dashed: 'dashed',
    dotted: 'dotted',
  },
} as const;

// ============================================================================
// MOTION - Apple-quality animations
// ============================================================================

export const motion = {
  // Duration tokens (in seconds)
  duration: {
    instant: '0.05s',
    fast: '0.1s',
    normal: '0.2s',
    slow: '0.3s',
    slower: '0.4s',
    slowest: '0.5s',
  },

  // Easing curves (Apple-style)
  easing: {
    // Standard
    ease: 'cubic-bezier(0.4, 0, 0.2, 1)',
    // Entering
    easeIn: 'cubic-bezier(0.4, 0, 1, 1)',
    // Exiting
    easeOut: 'cubic-bezier(0, 0, 0.2, 1)',
    // Spring (bouncy)
    spring: 'cubic-bezier(0.34, 1.56, 0.64, 1)',
    // Smooth
    smooth: 'cubic-bezier(0.23, 1, 0.32, 1)',
    // Snappy
    snappy: 'cubic-bezier(0.25, 0.46, 0.45, 0.94)',
  },

  // Predefined transitions
  transition: {
    default: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
    fast: 'all 0.1s cubic-bezier(0.4, 0, 0.2, 1)',
    slow: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
    spring: 'all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)',
    transform: 'transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
    opacity: 'opacity 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
    colors: 'background-color 0.2s, border-color 0.2s, color 0.2s',
    shadow: 'box-shadow 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
  },

  // Animation keyframes (CSS variable references)
  keyframes: {
    fadeIn: 'fadeIn 0.2s ease-out',
    fadeOut: 'fadeOut 0.2s ease-in',
    slideIn: 'slideIn 0.3s cubic-bezier(0.23, 1, 0.32, 1)',
    slideOut: 'slideOut 0.2s cubic-bezier(0.4, 0, 1, 1)',
    scaleIn: 'scaleIn 0.2s cubic-bezier(0.34, 1.56, 0.64, 1)',
    scaleOut: 'scaleOut 0.15s cubic-bezier(0.4, 0, 1, 1)',
    pulse: 'pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
    spin: 'spin 1s linear infinite',
    bounce: 'bounce 1s ease-in-out',
    shimmer: 'shimmer 2s linear infinite',
    blink: 'blink 1s step-end infinite',
  },
} as const;

// ============================================================================
// SEMANTIC TOKENS - Theme-aware colors
// ============================================================================

export const semanticColors = {
  // Light theme (default)
  light: {
    // Background colors
    bg: {
      primary: baseColors.neutral[0],      // Main app background
      secondary: baseColors.neutral[50],   // Secondary surfaces
      tertiary: baseColors.neutral[100],   // Input backgrounds, cards
      quaternary: baseColors.neutral[200], // Disabled, subtle backgrounds
      elevated: baseColors.neutral[0],     // Cards, popovers (with shadow)
      overlay: 'rgba(0, 0, 0, 0.4)',      // Modal backdrops
      terminal: baseColors.terminal.background,
    },

    // Text colors
    text: {
      primary: baseColors.neutral[900],   // Headlines, important text
      secondary: baseColors.neutral[700], // Body text
      tertiary: baseColors.neutral[600],  // Captions, hints
      quaternary: baseColors.neutral[400],// Disabled, placeholders
      inverted: baseColors.neutral[0],    // Text on dark backgrounds
      terminal: baseColors.terminal.foreground,
    },

    // Border colors
    border: {
      subtle: baseColors.neutral[200],     // Dividers, subtle borders
      default: baseColors.neutral[300],   // Input borders
      strong: baseColors.neutral[400],    // Focus states
    },

    // Interactive colors
    interactive: {
      primary: baseColors.brand[600],
      primaryHover: baseColors.brand[700],
      primaryActive: baseColors.brand[800],
      secondary: baseColors.neutral[100],
      secondaryHover: baseColors.neutral[200],
      secondaryActive: baseColors.neutral[300],
      ghost: 'transparent',
      ghostHover: baseColors.neutral[100],
    },

    // Status backgrounds
    status: {
      online: 'rgba(34, 197, 94, 0.1)',
      offline: 'rgba(239, 68, 68, 0.1)',
      warning: 'rgba(245, 158, 11, 0.1)',
      info: 'rgba(59, 130, 246, 0.1)',
    },

    // Focus ring
    focus: baseColors.brand[500],
    focusRing: `0 0 0 3px ${baseColors.brand[200]}`,
  },

  // Dark theme
  dark: {
    // Background colors
    bg: {
      primary: baseColors.neutral[950],      // Main app background
      secondary: baseColors.neutral[900],   // Secondary surfaces
      tertiary: baseColors.neutral[800],    // Input backgrounds, cards
      quaternary: baseColors.neutral[700],  // Disabled, subtle backgrounds
      elevated: baseColors.neutral[900],    // Cards, popovers
      overlay: 'rgba(0, 0, 0, 0.7)',        // Modal backdrops
      terminal: baseColors.terminal.background,
    },

    // Text colors
    text: {
      primary: baseColors.neutral[100],      // Headlines
      secondary: baseColors.neutral[300],   // Body text
      tertiary: baseColors.neutral[400],    // Captions
      quaternary: baseColors.neutral[500],  // Disabled
      inverted: baseColors.neutral[900],    // Text on light backgrounds
      terminal: baseColors.terminal.foreground,
    },

    // Border colors
    border: {
      subtle: baseColors.neutral[800],      // Dividers
      default: baseColors.neutral[700],     // Input borders
      strong: baseColors.neutral[500],      // Focus states
    },

    // Interactive colors
    interactive: {
      primary: baseColors.brand[500],
      primaryHover: baseColors.brand[400],
      primaryActive: baseColors.brand[600],
      secondary: baseColors.neutral[800],
      secondaryHover: baseColors.neutral[700],
      secondaryActive: baseColors.neutral[600],
      ghost: 'transparent',
      ghostHover: baseColors.neutral[800],
    },

    // Status backgrounds
    status: {
      online: 'rgba(34, 197, 94, 0.15)',
      offline: 'rgba(239, 68, 68, 0.15)',
      warning: 'rgba(245, 158, 11, 0.15)',
      info: 'rgba(59, 130, 246, 0.15)',
    },

    // Focus ring
    focus: baseColors.brand[400],
    focusRing: `0 0 0 3px ${baseColors.brand[900]}`,
  },
} as const;

// ============================================================================
// COMPONENT TOKENS - Specific component styling
// ============================================================================

export const componentTokens = {
  // App Shell
  appShell: {
    headerHeight: '48px',
    sidebarWidth: '260px',
    sidebarCollapsedWidth: '48px',
    rightPanelWidth: '320px',
    bottomPanelHeight: '200px',
  },

  // Terminal
  terminal: {
    minHeight: '200px',
    padding: '8px 12px',
    fontSize: '13px',
    lineHeight: '1.4',
    cursorBlink: true,
    cursorStyle: 'block', // 'block' | 'line' | 'bar'
    scrollback: 10000,
  },

  // Sidebar
  sidebar: {
    itemHeight: '36px',
    itemPadding: '8px 12px',
    groupPadding: '16px 12px 8px',
    iconSize: '16px',
    indentSize: '16px',
  },

  // Server Card
  serverCard: {
    width: '280px',
    height: 'auto',
    padding: '16px',
    gap: '12px',
    borderRadius: borders.radius.lg,
  },

  // Command Palette
  commandPalette: {
    width: '640px',
    maxHeight: '480px',
    itemHeight: '44px',
    inputHeight: '56px',
    sectionGap: '8px',
  },

  // Buttons
  button: {
    height: {
      xs: '24px',
      sm: '32px',
      md: '36px',
      lg: '44px',
      xl: '52px',
    },
    padding: {
      xs: '0 8px',
      sm: '0 12px',
      md: '0 16px',
      lg: '0 20px',
      xl: '0 24px',
    },
    borderRadius: borders.radius.md,
    fontSize: {
      xs: typography.fontSize['2xs'],
      sm: typography.fontSize.xs,
      md: typography.fontSize.sm,
      lg: typography.fontSize.base,
      xl: typography.fontSize.md,
    },
  },

  // Inputs
  input: {
    height: {
      sm: '32px',
      md: '36px',
      lg: '44px',
    },
    padding: '0 12px',
    borderRadius: borders.radius.md,
    fontSize: typography.fontSize.sm,
  },

  // Tooltips
  tooltip: {
    padding: '6px 10px',
    borderRadius: borders.radius.sm,
    fontSize: typography.fontSize.xs,
    maxWidth: '240px',
  },

  // Toast notifications
  toast: {
    padding: '12px 16px',
    borderRadius: borders.radius.lg,
    maxWidth: '400px',
    duration: 4000,
  },
} as const;

// ============================================================================
// Z-INDEX SCALE
// ============================================================================

export const zIndex = {
  base: 0,
  dropdown: 100,
  sticky: 200,
  fixed: 300,
  overlay: 400,
  modalBackdrop: 500,
  modal: 510,
  popover: 600,
  tooltip: 700,
  toast: 800,
  commandPalette: 900,
} as const;

// ============================================================================
// BREAKPOINTS
// ============================================================================

export const breakpoints = {
  xs: '480px',
  sm: '640px',
  md: '768px',
  lg: '1024px',
  xl: '1280px',
  '2xl': '1536px',
} as const;

// ============================================================================
// OPACITY SCALE
// ============================================================================

export const opacity = {
  0: '0',
  5: '0.05',
  10: '0.1',
  20: '0.2',
  30: '0.3',
  40: '0.4',
  50: '0.5',
  60: '0.6',
  70: '0.7',
  80: '0.8',
  90: '0.9',
  100: '1',
} as const;

// ============================================================================
// EXPORT COMPLETE TOKENS OBJECT
// ============================================================================

export const tokens = {
  colors: baseColors,
  semantic: semanticColors,
  typography,
  spacing,
  shadows,
  borders,
  motion,
  component: componentTokens,
  zIndex,
  breakpoints,
  opacity,
} as const;

// Type exports for TypeScript
export type Tokens = typeof tokens;
export type Theme = 'light' | 'dark';
export type ColorScheme = keyof typeof baseColors;

export default tokens;
