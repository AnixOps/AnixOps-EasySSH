import React, { createContext, useContext, useEffect, useState } from 'react';
import { tokens, Theme as ThemeType } from '../tokens/design-tokens';

/**
 * Theme Context and Provider for React components
 * Manages theme state and provides theme variables to all children
 */

export interface ThemeContextValue {
  theme: ThemeType;
  setTheme: (theme: ThemeType) => void;
  toggleTheme: () => void;
  isDark: boolean;
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

export interface ThemeProviderProps {
  children: React.ReactNode;
  defaultTheme?: ThemeType;
  enableSystem?: boolean;
  disableTransitionOnChange?: boolean;
}

export const ThemeProvider: React.FC<ThemeProviderProps> = ({
  children,
  defaultTheme = 'system',
  enableSystem = true,
  disableTransitionOnChange = false,
}) => {
  const [theme, setThemeState] = useState<ThemeType>(defaultTheme);
  const [resolvedTheme, setResolvedTheme] = useState<'light' | 'dark'>('light');

  // Apply theme to document
  useEffect(() => {
    const root = document.documentElement;

    // Disable transitions if requested
    if (disableTransitionOnChange) {
      root.classList.add('disable-transitions');
      setTimeout(() => root.classList.remove('disable-transitions'), 0);
    }

    // Resolve theme
    let resolved: 'light' | 'dark';
    if (theme === 'system' && enableSystem) {
      resolved = window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light';
    } else {
      resolved = theme === 'dark' ? 'dark' : 'light';
    }

    setResolvedTheme(resolved);

    // Apply CSS variables
    root.setAttribute('data-theme', resolved);
    applyThemeVariables(resolved);

    // Listen for system theme changes
    if (theme === 'system' && enableSystem) {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const handler = (e: MediaQueryListEvent) => {
        const newTheme = e.matches ? 'dark' : 'light';
        setResolvedTheme(newTheme);
        root.setAttribute('data-theme', newTheme);
        applyThemeVariables(newTheme);
      };

      mediaQuery.addEventListener('change', handler);
      return () => mediaQuery.removeEventListener('change', handler);
    }
  }, [theme, enableSystem, disableTransitionOnChange]);

  const setTheme = (newTheme: ThemeType) => {
    setThemeState(newTheme);
    localStorage.setItem('easyssh-theme', newTheme);
  };

  const toggleTheme = () => {
    const newTheme = resolvedTheme === 'light' ? 'dark' : 'light';
    setTheme(newTheme);
  };

  // Load saved theme on mount
  useEffect(() => {
    const saved = localStorage.getItem('easyssh-theme') as ThemeType | null;
    if (saved && (saved === 'light' || saved === 'dark' || saved === 'system')) {
      setThemeState(saved);
    }
  }, []);

  const value: ThemeContextValue = {
    theme,
    setTheme,
    toggleTheme,
    isDark: resolvedTheme === 'dark',
  };

  return (
    <ThemeContext.Provider value={value}>
      {children}
    </ThemeContext.Provider>
  );
};

/**
 * Apply CSS custom properties for the theme
 */
function applyThemeVariables(theme: 'light' | 'dark') {
  const root = document.documentElement;
  const colors = tokens.semantic[theme];
  const base = tokens.colors;

  // Background colors
  root.style.setProperty('--easyssh-bg-primary', colors.bg.primary);
  root.style.setProperty('--easyssh-bg-secondary', colors.bg.secondary);
  root.style.setProperty('--easyssh-bg-tertiary', colors.bg.tertiary);
  root.style.setProperty('--easyssh-bg-elevated', colors.bg.elevated);
  root.style.setProperty('--easyssh-bg-overlay', colors.bg.overlay);
  root.style.setProperty('--easyssh-bg-terminal', colors.bg.terminal);

  // Text colors
  root.style.setProperty('--easyssh-text-primary', colors.text.primary);
  root.style.setProperty('--easyssh-text-secondary', colors.text.secondary);
  root.style.setProperty('--easyssh-text-tertiary', colors.text.tertiary);
  root.style.setProperty('--easyssh-text-inverted', colors.text.inverted);
  root.style.setProperty('--easyssh-text-terminal', colors.text.terminal);

  // Interactive colors
  root.style.setProperty('--easyssh-interactive-primary', colors.interactive.primary);
  root.style.setProperty('--easyssh-interactive-primary-hover', colors.interactive.primaryHover);
  root.style.setProperty('--easyssh-interactive-secondary', colors.interactive.secondary);
  root.style.setProperty('--easyssh-interactive-secondary-hover', colors.interactive.secondaryHover);
  root.style.setProperty('--easyssh-interactive-ghost-hover', colors.interactive.ghostHover);

  // Border colors
  root.style.setProperty('--easyssh-border-subtle', colors.border.subtle);
  root.style.setProperty('--easyssh-border-default', colors.border.default);
  root.style.setProperty('--easyssh-border-strong', colors.border.strong);

  // Focus ring
  root.style.setProperty('--easyssh-focus-color', colors.focus);
  root.style.setProperty('--easyssh-focus-ring', colors.focusRing);

  // Brand colors
  for (let i = 50; i <= 950; i += 50) {
    const key = i as keyof typeof base.brand;
    if (base.brand[key]) {
      root.style.setProperty(`--easyssh-primary-${i}`, base.brand[key]);
    }
  }

  // Status colors
  root.style.setProperty('--easyssh-status-online', base.status.online);
  root.style.setProperty('--easyssh-status-offline', base.status.offline);
  root.style.setProperty('--easyssh-status-connecting', base.status.connecting);

  // Terminal colors
  const term = tokens.colors.terminal;
  root.style.setProperty('--easyssh-terminal-black', term.black);
  root.style.setProperty('--easyssh-terminal-red', term.red);
  root.style.setProperty('--easyssh-terminal-green', term.green);
  root.style.setProperty('--easyssh-terminal-yellow', term.yellow);
  root.style.setProperty('--easyssh-terminal-blue', term.blue);
  root.style.setProperty('--easyssh-terminal-magenta', term.magenta);
  root.style.setProperty('--easyssh-terminal-cyan', term.cyan);
  root.style.setProperty('--easyssh-terminal-white', term.white);
  root.style.setProperty('--easyssh-terminal-cursor', term.cursor);
  root.style.setProperty('--easyssh-terminal-selection', term.selection);
}

/**
 * Hook to access theme context
 */
export const useTheme = (): ThemeContextValue => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};

/**
 * Hook to detect system color scheme
 */
export const useSystemTheme = (): 'light' | 'dark' => {
  const [systemTheme, setSystemTheme] = useState<'light' | 'dark'>('light');

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    setSystemTheme(mediaQuery.matches ? 'dark' : 'light');

    const handler = (e: MediaQueryListEvent) => {
      setSystemTheme(e.matches ? 'dark' : 'light');
    };

    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  return systemTheme;
};

/**
 * Hook to detect reduced motion preference
 */
export const useReducedMotion = (): boolean => {
  const [reducedMotion, setReducedMotion] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    setReducedMotion(mediaQuery.matches);

    const handler = (e: MediaQueryListEvent) => {
      setReducedMotion(e.matches);
    };

    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  return reducedMotion;
};

/**
 * Hook to detect high contrast preference
 */
export const useHighContrast = (): boolean => {
  const [highContrast, setHighContrast] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-contrast: high)');
    setHighContrast(mediaQuery.matches);

    const handler = (e: MediaQueryListEvent) => {
      setHighContrast(e.matches);
    };

    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  return highContrast;
};

/**
 * Theme-aware className generator
 */
export const themeClass = (baseClass: string, theme: 'light' | 'dark'): string => {
  return `${baseClass} ${baseClass}--${theme}`;
};

/**
 * Generate CSS for theme transitions
 */
export const themeTransitionStyles = `
  :root {
    transition: background-color 0.2s ease, color 0.2s ease;
  }

  .disable-transitions,
  .disable-transitions * {
    transition: none !important;
    animation: none !important;
  }
`;

export default ThemeProvider;
