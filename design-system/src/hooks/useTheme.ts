/**
 * Theme Hook
 * React hook for theme management with system preference detection
 */

import { useState, useEffect, useCallback } from 'react';

export type Theme = 'light' | 'dark' | 'system';

interface UseThemeReturn {
  /** Current active theme (resolved to light or dark) */
  theme: 'light' | 'dark';
  /** User's theme preference */
  preference: Theme;
  /** Set theme preference */
  setTheme: (theme: Theme) => void;
  /** Toggle between light and dark */
  toggleTheme: () => void;
  /** Whether theme is currently being set by system */
  isSystem: boolean;
}

const STORAGE_KEY = 'easyssh-theme-preference';

/**
 * Get initial theme from localStorage or system preference
 */
function getInitialTheme(): Theme {
  if (typeof window === 'undefined') return 'system';

  try {
    const stored = localStorage.getItem(STORAGE_KEY) as Theme;
    if (stored && ['light', 'dark', 'system'].includes(stored)) {
      return stored;
    }
  } catch {
    // Ignore localStorage errors
  }

  return 'system';
}

/**
 * Get actual theme value based on preference
 */
function resolveTheme(preference: Theme): 'light' | 'dark' {
  if (preference === 'system') {
    if (typeof window === 'undefined') return 'dark';
    return window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light';
  }
  return preference;
}

/**
 * Apply theme to document
 */
function applyTheme(theme: 'light' | 'dark'): void {
  if (typeof document === 'undefined') return;

  const root = document.documentElement;
  root.setAttribute('data-theme', theme);

  // Also set class for Tailwind dark mode
  if (theme === 'dark') {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }
}

/**
 * Hook for theme management
 */
export function useTheme(): UseThemeReturn {
  const [preference, setPreferenceState] = useState<Theme>(getInitialTheme);
  const [theme, setThemeState] = useState<'light' | 'dark'>(() =>
    resolveTheme(getInitialTheme())
  );

  // Apply theme when preference changes
  useEffect(() => {
    const resolved = resolveTheme(preference);
    setThemeState(resolved);
    applyTheme(resolved);

    // Store preference
    try {
      localStorage.setItem(STORAGE_KEY, preference);
    } catch {
      // Ignore localStorage errors
    }
  }, [preference]);

  // Listen for system theme changes
  useEffect(() => {
    if (preference !== 'system') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleChange = (e: MediaQueryListEvent) => {
      const newTheme = e.matches ? 'dark' : 'light';
      setThemeState(newTheme);
      applyTheme(newTheme);
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [preference]);

  const setTheme = useCallback((newTheme: Theme) => {
    setPreferenceState(newTheme);
  }, []);

  const toggleTheme = useCallback(() => {
    setPreferenceState((prev) => {
      if (prev === 'system') {
        // If system, toggle to opposite of current resolved theme
        return theme === 'dark' ? 'light' : 'dark';
      }
      // Toggle between light and dark
      return prev === 'dark' ? 'light' : 'dark';
    });
  }, [theme]);

  return {
    theme,
    preference,
    setTheme,
    toggleTheme,
    isSystem: preference === 'system',
  };
}

/**
 * Initialize theme on app mount
 * Call this once in your app root
 */
export function initTheme(): void {
  const preference = getInitialTheme();
  const theme = resolveTheme(preference);
  applyTheme(theme);
}

export default useTheme;
