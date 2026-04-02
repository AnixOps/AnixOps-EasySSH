/**
 * EasySSH Design System
 * Apple-level native UI design system for SSH client product line
 *
 * @package @easyssh/design-system
 * @version 1.1.0
 */

// ============================================================================
// Token Exports
// ============================================================================

export {
  tokens,
  baseColors,
  semanticColors,
  typography,
  spacing,
  shadows,
  borders,
  motion,
  componentTokens,
  zIndex,
  breakpoints,
  opacity,
} from './src/tokens/design-tokens';

export type {
  Tokens,
  Theme,
  ColorScheme,
} from './src/tokens/design-tokens';

// ============================================================================
// React Components
// ============================================================================

export { Button, IconButton, ButtonGroup, SplitButton, buttonVariants } from './src/components/Button';
export type { ButtonProps, IconButtonProps, ButtonGroupProps, SplitButtonProps } from './src/components/Button';

export { Card, CardGrid, StatCard, ServerCard, cardVariants } from './src/components/Card';
export type { CardProps, CardGridProps, StatCardProps, ServerCardProps } from './src/components/Card';

// ============================================================================
// Theme
// ============================================================================

export {
  ThemeProvider,
  useTheme,
  useSystemTheme,
  useReducedMotion,
  useHighContrast,
  themeClass,
  themeTransitionStyles,
} from './src/theme/ThemeProvider';
export type { ThemeProviderProps, ThemeContextValue } from './src/theme/ThemeProvider';

// ============================================================================
// Icons
// ============================================================================

export { Icon, IconWithLabel, iconPaths } from './src/icons';
export type { IconName, IconProps, IconWithLabelProps } from './src/icons';

// ============================================================================
// Animations
// ============================================================================

export {
  useReducedMotion as useReducedMotionHook,
  useFade,
  useSlide,
  useScale,
  useStagger,
  useSpring,
  useScrollReveal,
  useRipple,
  animationClasses,
  DURATIONS,
  easings,
  createTransition,
  debounce,
  throttle,
} from './src/animations';

// ============================================================================
// Utility Exports
// ============================================================================

export {
  utils,
  cn,
  getColor,
  getTerminalColor,
  getStatusColor,
  getSpacing,
  spacing,
  getFontStack,
  getTypeStyle,
  getShadow,
  combineShadows,
  transition,
  getAnimation,
  getBreakpoint,
  mediaQuery,
  getZIndex,
  zIndex as zIndexUtil,
  getComponentToken,
  getButtonHeight,
  getSidebarWidth,
  focusRing,
  reducedMotion,
  isDarkTheme,
  applyThemeClass,
  generateCSSVariables,
  formatBytes,
  formatDate,
  formatTime,
  formatRelativeTime,
  truncate,
  generateId,
  copyToClipboard,
  getCSSVariable,
  setCSSVariable,
  isInViewport,
  colors,
} from './src/utils';

export type { ClassValue } from 'clsx';

// ============================================================================
// Hooks (from existing useTheme)
// ============================================================================

export {
  useTheme as useThemeHook,
  initTheme,
  type UseThemeReturn,
  type Theme as ThemePreference,
} from './src/hooks/useTheme';

// ============================================================================
// Version
// ============================================================================

export const VERSION = '1.1.0';

// ============================================================================
// Default Export
// ============================================================================

import { tokens } from './src/tokens/design-tokens';
import { utils } from './src/utils';

export default {
  tokens,
  utils,
  version: VERSION,
};
