/**
 * i18n React Hook
 *
 * Provides convenient access to translations and formatting in React components.
 *
 * @module useI18n
 */

import { useCallback } from 'react';
import {
  useTranslation,
  useFormat,
  useLanguage,
} from '../stores/i18nStore';
import type { LanguageCode } from '../stores/i18nStore';

interface I18nHookResult {
  // Translation
  t: (key: string, args?: Record<string, string | number>) => string;

  // Formatting
  formatNumber: (num: number) => string;
  formatDate: (date: Date | number) => string;
  formatDateTime: (date: Date | number) => string;

  // Language
  currentLanguage: LanguageCode;
  setLanguage: (lang: LanguageCode) => Promise<void>;
  isRTL: boolean;

  // Utilities
  getTextDirection: () => 'ltr' | 'rtl';
  isLoading: boolean;
}

/**
 * Main i18n hook combining all translation functionality
 */
export function useI18n(): I18nHookResult {
  const { t, currentLanguage, isRTL, isLoading } = useTranslation();
  const { formatNumber, formatDate, formatDateTime, getTextDirection } = useFormat();
  const { setLanguage } = useLanguage();

  return {
    t,
    formatNumber,
    formatDate,
    formatDateTime,
    currentLanguage,
    setLanguage,
    isRTL,
    isLoading,
    getTextDirection,
  };
}

/**
 * Hook for RTL-aware styling
 */
export function useRTL() {
  const { isRTL } = useLanguage();
  const { getTextDirection } = useFormat();

  const rtlClass = isRTL ? 'rtl' : 'ltr';
  const textAlign = isRTL ? 'text-right' : 'text-left';
  const flexDirection = isRTL ? 'flex-row-reverse' : 'flex-row';
  const marginStart = isRTL ? 'ml-' : 'mr-';
  const marginEnd = isRTL ? 'mr-' : 'ml-';
  const paddingStart = isRTL ? 'pl-' : 'pr-';
  const paddingEnd = isRTL ? 'pr-' : 'pl-';

  return {
    isRTL,
    direction: getTextDirection(),
    rtlClass,
    textAlign,
    flexDirection,
    marginStart,
    marginEnd,
    paddingStart,
    paddingEnd,

    // Helper to conditionally apply RTL classes
    cn: (ltrClass: string, rtlClassName: string) =>
      isRTL ? rtlClassName : ltrClass,

    // Invert icon direction for RTL
    iconClass: isRTL ? '-scale-x-100' : '',
  };
}

/**
 * Hook for component-level translations with namespace support
 */
export function useNamespace(namespace: string) {
  const { t: baseT } = useTranslation();

  const t = useCallback(
    (key: string, args?: Record<string, string | number>) => {
      // Try namespaced key first
      const namespacedKey = `${namespace}-${key}`;
      const result = baseT(namespacedKey, args);

      // Fall back to plain key if namespaced version not found
      if (result === namespacedKey) {
        return baseT(key, args);
      }

      return result;
    },
    [baseT, namespace]
  );

  return { t };
}

// Common namespaces
export const NAMESPACES = {
  SERVER: 'server',
  TERMINAL: 'terminal',
  SFTP: 'sftp',
  SETTINGS: 'settings',
  ERROR: 'error',
  GENERAL: 'general',
  CONNECTION: 'connection',
} as const;

/**
 * Pre-defined namespace hooks
 */
export function useServerTranslations() {
  return useNamespace(NAMESPACES.SERVER);
}

export function useTerminalTranslations() {
  return useNamespace(NAMESPACES.TERMINAL);
}

export function useSFTPTranslations() {
  return useNamespace(NAMESPACES.SFTP);
}

export function useSettingsTranslations() {
  return useNamespace(NAMESPACES.SETTINGS);
}

export function useErrorTranslations() {
  return useNamespace(NAMESPACES.ERROR);
}

export default useI18n;
