/**
 * Design System Utilities
 * Helper functions and type guards for the design system
 */

import { tokens, type Theme } from '../tokens/design-tokens';

// ============================================================================
// Class Name Utilities (Tailwind Merge)
// ============================================================================

import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

/**
 * Merge Tailwind classes with proper precedence
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// ============================================================================
// Color Utilities
// ============================================================================

/**
 * Get color value from design tokens
 */
export function getColor(
  colorPath: string,
  theme: Theme = 'light'
): string {
  // Handle CSS variable references
  if (colorPath.startsWith('--')) {
    return `var(${colorPath})`;
  }

  // Handle semantic colors
  if (colorPath.includes('semantic')) {
    return `var(--easyssh-${colorPath.replace('.', '-')})`;
  }

  // Fallback
  return colorPath;
}

/**
 * Get terminal color
 */
export function getTerminalColor(colorName: string): string {
  const colors: Record<string, string> = {
    black: tokens.colors.terminal.black,
    red: tokens.colors.terminal.red,
    green: tokens.colors.terminal.green,
    yellow: tokens.colors.terminal.yellow,
    blue: tokens.colors.terminal.blue,
    magenta: tokens.colors.terminal.magenta,
    cyan: tokens.colors.terminal.cyan,
    white: tokens.colors.terminal.white,
    brightBlack: tokens.colors.terminal.brightBlack,
    brightRed: tokens.colors.terminal.brightRed,
    brightGreen: tokens.colors.terminal.brightGreen,
    brightYellow: tokens.colors.terminal.brightYellow,
    brightBlue: tokens.colors.terminal.brightBlue,
    brightMagenta: tokens.colors.terminal.brightMagenta,
    brightCyan: tokens.colors.terminal.brightCyan,
    brightWhite: tokens.colors.terminal.brightWhite,
  };

  return colors[colorName] || colors.white;
}

/**
 * Get status color
 */
export function getStatusColor(status: 'online' | 'offline' | 'connecting' | 'maintenance' | 'unknown'): string {
  const colors = {
    online: tokens.colors.status.online,
    offline: tokens.colors.status.offline,
    connecting: tokens.colors.status.connecting,
    maintenance: tokens.colors.status.maintenance,
    unknown: tokens.colors.status.unknown,
  };

  return colors[status] || colors.unknown;
}

// Color conversion utilities
export const colors = {
  /**
   * Convert hex to RGB
   */
  hexToRgb(hex: string): { r: number; g: number; b: number } | null {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result
      ? {
          r: parseInt(result[1], 16),
          g: parseInt(result[2], 16),
          b: parseInt(result[3], 16),
        }
      : null;
  },

  /**
   * Convert RGB to hex
   */
  rgbToHex(r: number, g: number, b: number): string {
    return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
  },

  /**
   * Calculate luminance
   */
  luminance(r: number, g: number, b: number): number {
    const [rs, gs, bs] = [r, g, b].map((c) => {
      c = c / 255;
      return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
    });
    return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
  },

  /**
   * Calculate contrast ratio
   */
  contrastRatio(color1: string, color2: string): number {
    const rgb1 = this.hexToRgb(color1);
    const rgb2 = this.hexToRgb(color2);

    if (!rgb1 || !rgb2) return 0;

    const lum1 = this.luminance(rgb1.r, rgb1.g, rgb1.b);
    const lum2 = this.luminance(rgb2.r, rgb2.g, rgb2.b);

    const lighter = Math.max(lum1, lum2);
    const darker = Math.min(lum1, lum2);

    return (lighter + 0.05) / (darker + 0.05);
  },

  /**
   * Check if color meets WCAG AA standard
   */
  meetsWCAGAA(foreground: string, background: string, largeText = false): boolean {
    const ratio = this.contrastRatio(foreground, background);
    return largeText ? ratio >= 3 : ratio >= 4.5;
  },
};

// ============================================================================
// Spacing Utilities
// ============================================================================

/**
 * Convert spacing token to pixels
 */
export function getSpacing(token: keyof typeof tokens.spacing): string {
  return tokens.spacing[token];
}

/**
 * Create spacing string (e.g., "16px 24px")
 */
export function spacing(
  vertical: keyof typeof tokens.spacing,
  horizontal?: keyof typeof tokens.spacing
): string {
  const v = tokens.spacing[vertical];
  const h = horizontal ? tokens.spacing[horizontal] : v;
  return `${v} ${h}`;
}

// ============================================================================
// Typography Utilities
// ============================================================================

/**
 * Get font stack
 */
export function getFontStack(type: 'sans' | 'mono' | 'display'): string {
  return tokens.typography.fontFamily[type].join(', ');
}

/**
 * Generate CSS for type style
 */
export function getTypeStyle(
  style: keyof typeof tokens.typography.styles,
  size: 'large' | 'medium' | 'small'
): string {
  const config = tokens.typography.styles[style][size];

  return `
    font-size: ${config.fontSize};
    font-weight: ${config.fontWeight};
    line-height: ${config.lineHeight};
    ${config.letterSpacing ? `letter-spacing: ${config.letterSpacing};` : ''}
  `;
}

// ============================================================================
// Shadow Utilities
// ============================================================================

/**
 * Get shadow value
 */
export function getShadow(shadowName: keyof typeof tokens.shadows): string {
  return tokens.shadows[shadowName];
}

/**
 * Combine multiple shadows
 */
export function combineShadows(...shadowNames: (keyof typeof tokens.shadows)[]): string {
  return shadowNames.map(name => tokens.shadows[name]).join(', ');
}

// ============================================================================
// Animation Utilities
// ============================================================================

/**
 * Generate CSS transition string
 */
export function transition(
  properties: string | string[],
  duration: keyof typeof tokens.motion.duration = 'normal',
  easing: keyof typeof tokens.motion.easing = 'ease'
): string {
  const props = Array.isArray(properties) ? properties : [properties];
  const dur = tokens.motion.duration[duration];
  const ease = tokens.motion.easing[easing];

  return props.map(prop => `${prop} ${dur} ${ease}`).join(', ');
}

/**
 * Get animation CSS
 */
export function getAnimation(
  animationName: keyof typeof tokens.motion.keyframes,
  duration: keyof typeof tokens.motion.duration = 'normal'
): string {
  const anim = tokens.motion.keyframes[animationName];
  const dur = tokens.motion.duration[duration];

  // Extract timing from animation string
  const timing = anim.replace(/^\S+\s+/, '').replace(/\s+infinite$/, '');

  return `${anim.replace(' infinite', '')} ${dur} ${timing}`;
}

// ============================================================================
// Breakpoint Utilities
// ============================================================================

/**
 * Get breakpoint value
 */
export function getBreakpoint(bp: keyof typeof tokens.breakpoints): string {
  return tokens.breakpoints[bp];
}

/**
 * Media query helper
 */
export function mediaQuery(bp: keyof typeof tokens.breakpoints, direction: 'up' | 'down' = 'up'): string {
  const value = tokens.breakpoints[bp];

  if (direction === 'up') {
    return `@media (min-width: ${value})`;
  }

  // For 'down', we need the previous breakpoint
  const bps = Object.keys(tokens.breakpoints) as Array<keyof typeof tokens.breakpoints>;
  const index = bps.indexOf(bp);
  if (index > 0) {
    return `@media (max-width: calc(${value} - 1px))`;
  }

  return '@media (max-width: 479px)';
}

// ============================================================================
// Z-Index Utilities
// ============================================================================

/**
 * Get z-index value
 */
export function getZIndex(layer: keyof typeof tokens.zIndex): number {
  return tokens.zIndex[layer];
}

/**
 * Create z-index CSS variable reference
 */
export function zIndex(layer: keyof typeof tokens.zIndex): string {
  return `var(--easyssh-z-${layer.replace(/([A-Z])/g, '-$1').toLowerCase()})`;
}

// ============================================================================
// Component Token Helpers
// ============================================================================

/**
 * Get component token value
 */
export function getComponentToken(
  component: keyof typeof tokens.component,
  token: string
): string | number {
  const comp = tokens.component[component];
  return (comp as Record<string, string | number>)[token];
}

/**
 * Get button height by size
 */
export function getButtonHeight(size: 'xs' | 'sm' | 'md' | 'lg' | 'xl'): string {
  return tokens.component.button.height[size];
}

/**
 * Get sidebar width
 */
export function getSidebarWidth(collapsed: boolean): string {
  return collapsed
    ? tokens.component.appShell.sidebarCollapsedWidth
    : tokens.component.appShell.sidebarWidth;
}

// ============================================================================
// Accessibility Utilities
// ============================================================================

/**
 * Generate focus ring styles
 */
export function focusRing(inset: boolean = false): string {
  return inset
    ? 'box-shadow: inset var(--easyssh-focus-ring)'
    : 'box-shadow: var(--easyssh-focus-ring)';
}

/**
 * Generate reduced motion styles
 */
export function reducedMotion(): string {
  return `
    @media (prefers-reduced-motion: reduce) {
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 0.01ms !important;
    }
  `;
}

// ============================================================================
// Formatting Utilities
// ============================================================================

/**
 * Format bytes to human readable string
 */
export function formatBytes(bytes: number, decimals = 2): string {
  if (bytes === 0) return '0 Bytes';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

/**
 * Format date to localized string
 */
export function formatDate(
  date: Date | number | string,
  options: Intl.DateTimeFormatOptions = {}
): string {
  const d = new Date(date);
  const defaultOptions: Intl.DateTimeFormatOptions = {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    ...options,
  };
  return d.toLocaleDateString(undefined, defaultOptions);
}

/**
 * Format time to localized string
 */
export function formatTime(
  date: Date | number | string,
  options: Intl.DateTimeFormatOptions = {}
): string {
  const d = new Date(date);
  const defaultOptions: Intl.DateTimeFormatOptions = {
    hour: 'numeric',
    minute: '2-digit',
    ...options,
  };
  return d.toLocaleTimeString(undefined, defaultOptions);
}

/**
 * Format relative time (e.g., "2 hours ago")
 */
export function formatRelativeTime(date: Date | number | string): string {
  const now = new Date();
  const d = new Date(date);
  const diff = now.getTime() - d.getTime();

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (seconds < 60) return 'just now';
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;

  return formatDate(date);
}

/**
 * Truncate text with ellipsis
 */
export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return `${text.slice(0, maxLength)}...`;
}

/**
 * Generate a unique ID
 */
export function generateId(prefix = 'id'): string {
  return `${prefix}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Copy text to clipboard
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Fallback
    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);
    textarea.select();

    try {
      document.execCommand('copy');
      return true;
    } catch {
      return false;
    } finally {
      document.body.removeChild(textarea);
    }
  }
}

// ============================================================================
// DOM Utilities
// ============================================================================

/**
 * Get CSS variable value
 */
export function getCSSVariable(name: string): string {
  if (typeof window === 'undefined') return '';
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/**
 * Set CSS variable
 */
export function setCSSVariable(name: string, value: string): void {
  if (typeof document === 'undefined') return;
  document.documentElement.style.setProperty(name, value);
}

/**
 * Check if value is in viewport
 */
export function isInViewport(element: HTMLElement): boolean {
  const rect = element.getBoundingClientRect();
  return (
    rect.top >= 0 &&
    rect.left >= 0 &&
    rect.bottom <= (window.innerHeight || document.documentElement.clientHeight) &&
    rect.right <= (window.innerWidth || document.documentElement.clientWidth)
  );
}

// ============================================================================
// Theme Utilities
// ============================================================================

/**
 * Check if current theme is dark
 */
export function isDarkTheme(): boolean {
  if (typeof document === 'undefined') return false;
  return document.documentElement.getAttribute('data-theme') === 'dark';
}

/**
 * Apply theme class to element
 */
export function applyThemeClass(element: HTMLElement, theme: Theme): void {
  element.setAttribute('data-theme', theme);
  if (theme === 'dark') {
    element.classList.add('dark');
  } else {
    element.classList.remove('dark');
  }
}

// ============================================================================
// CSS-in-JS Helpers
// ============================================================================

/**
 * Generate CSS custom properties from tokens
 */
export function generateCSSVariables(): string {
  let css = ':root {\n';

  // Add semantic colors
  Object.entries(tokens.semantic.light).forEach(([category, values]) => {
    if (typeof values === 'string') {
      css += `  --easyssh-${category}: ${values};\n`;
    } else if (typeof values === 'object') {
      Object.entries(values).forEach(([key, value]) => {
        css += `  --easyssh-${category}-${key}: ${value};\n`;
      });
    }
  });

  css += '}\n';

  // Add dark theme
  css += '\n[data-theme="dark"] {\n';
  Object.entries(tokens.semantic.dark).forEach(([category, values]) => {
    if (typeof values === 'string') {
      css += `  --easyssh-${category}: ${values};\n`;
    } else if (typeof values === 'object') {
      Object.entries(values).forEach(([key, value]) => {
        css += `  --easyssh-${category}-${key}: ${value};\n`;
      });
    }
  });
  css += '}\n';

  return css;
}

// ============================================================================
// Export all utilities
// ============================================================================

export const utils = {
  cn,
  color: { getColor, getTerminalColor, getStatusColor, ...colors },
  spacing: { getSpacing, spacing },
  typography: { getFontStack, getTypeStyle },
  shadow: { getShadow, combineShadows },
  motion: { transition, getAnimation },
  breakpoint: { getBreakpoint, mediaQuery },
  zIndex: { getZIndex, zIndex },
  component: { getComponentToken, getButtonHeight, getSidebarWidth },
  a11y: { focusRing, reducedMotion },
  format: { formatBytes, formatDate, formatTime, formatRelativeTime, truncate },
  dom: { getCSSVariable, setCSSVariable, isInViewport, copyToClipboard },
  theme: { isDarkTheme, applyThemeClass },
  css: { generateCSSVariables },
  id: { generateId },
};

export default utils;
