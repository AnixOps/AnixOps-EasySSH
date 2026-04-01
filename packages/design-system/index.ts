/**
 * EasySSH Design System
 * Apple-level native UI design system for SSH client product line
 *
 * @package @easyssh/design-system
 * @version 1.0.0
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
// Hook Exports
// ============================================================================

export {
  useTheme,
  initTheme,
  type UseThemeReturn,
  type Theme as ThemePreference,
} from './src/hooks/useTheme';

// ============================================================================
// Utility Exports
// ============================================================================

export {
  utils,
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
} from './src/utils';

// ============================================================================
// Version
// ============================================================================

export const VERSION = '1.0.0';

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
