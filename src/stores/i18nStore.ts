/**
 * Internationalization (i18n) Store
 *
 * Manages language state, translations, and RTL support for the EasySSH frontend.
 *
 * @module i18nStore
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

// =============================================================================
// Types
// =============================================================================

export type LanguageCode =
  | 'en'
  | 'zh-CN'
  | 'zh-TW'
  | 'ja'
  | 'ko'
  | 'de'
  | 'fr'
  | 'es'
  | 'ru'
  | 'ar'
  | 'he';

export interface Language {
  code: LanguageCode;
  name: string;
  nativeName: string;
  isRTL: boolean;
}

export interface I18nState {
  // Current language
  currentLanguage: LanguageCode;

  // Available languages
  languages: Language[];

  // RTL state
  isRTL: boolean;

  // Translation dictionary (loaded dynamically)
  translations: Record<string, string>;

  // Loading state
  isLoading: boolean;

  // Error state
  error: string | null;

  // Actions
  setLanguage: (lang: LanguageCode) => Promise<void>;
  loadTranslations: (lang: LanguageCode) => Promise<void>;
  t: (key: string, args?: Record<string, string | number>) => string;
  formatNumber: (num: number) => string;
  formatDate: (date: Date | number) => string;
  formatDateTime: (date: Date | number) => string;
  getTextDirection: () => 'ltr' | 'rtl';
  getLanguageByCode: (code: LanguageCode) => Language | undefined;
}

// =============================================================================
// Supported Languages Definition
// =============================================================================

export const SUPPORTED_LANGUAGES: Language[] = [
  { code: 'en', name: 'English', nativeName: 'English', isRTL: false },
  { code: 'zh-CN', name: 'Chinese (Simplified)', nativeName: '中文（简体）', isRTL: false },
  { code: 'zh-TW', name: 'Chinese (Traditional)', nativeName: '中文（繁體）', isRTL: false },
  { code: 'ja', name: 'Japanese', nativeName: '日本語', isRTL: false },
  { code: 'ko', name: 'Korean', nativeName: '한국어', isRTL: false },
  { code: 'de', name: 'German', nativeName: 'Deutsch', isRTL: false },
  { code: 'fr', name: 'French', nativeName: 'Français', isRTL: false },
  { code: 'es', name: 'Spanish', nativeName: 'Español', isRTL: false },
  { code: 'ru', name: 'Russian', nativeName: 'Русский', isRTL: false },
  { code: 'ar', name: 'Arabic', nativeName: 'العربية', isRTL: true },
  { code: 'he', name: 'Hebrew', nativeName: 'עברית', isRTL: true },
];

// RTL Languages
export const RTL_LANGUAGES: LanguageCode[] = ['ar', 'he'];

// Default language
export const DEFAULT_LANGUAGE: LanguageCode = 'en';

// =============================================================================
// Translation Loading
// =============================================================================

/**
 * Load translation file from the locales directory
 */
async function loadTranslationFile(lang: LanguageCode): Promise<Record<string, string>> {
  try {
    // In a real app, this would fetch from the backend or a JSON file
    // For now, we'll use a simple fetch from the public directory
    const response = await fetch(`/locales/${lang}.json`);
    if (!response.ok) {
      throw new Error(`Failed to load translations for ${lang}`);
    }
    return await response.json();
  } catch (error) {
    console.warn(`Failed to load translations for ${lang}, falling back to English`);
    // Fallback to English if loading fails
    if (lang !== 'en') {
      return loadTranslationFile('en');
    }
    return {};
  }
}

// =============================================================================
// Formatting Helpers
// =============================================================================

/**
 * Format a number according to locale
 */
function formatNumberForLocale(num: number, lang: LanguageCode): string {
  const locale = getLocaleForLanguage(lang);
  return new Intl.NumberFormat(locale).format(num);
}

/**
 * Format a date according to locale
 */
function formatDateForLocale(date: Date | number, lang: LanguageCode): string {
  const d = typeof date === 'number' ? new Date(date) : date;
  const locale = getLocaleForLanguage(lang);
  return new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  }).format(d);
}

/**
 * Format a datetime according to locale
 */
function formatDateTimeForLocale(date: Date | number, lang: LanguageCode): string {
  const d = typeof date === 'number' ? new Date(date) : date;
  const locale = getLocaleForLanguage(lang);
  return new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(d);
}

/**
 * Get BCP 47 locale tag for a language code
 */
function getLocaleForLanguage(lang: LanguageCode): string {
  const localeMap: Record<LanguageCode, string> = {
    'en': 'en-US',
    'zh-CN': 'zh-CN',
    'zh-TW': 'zh-TW',
    'ja': 'ja-JP',
    'ko': 'ko-KR',
    'de': 'de-DE',
    'fr': 'fr-FR',
    'es': 'es-ES',
    'ru': 'ru-RU',
    'ar': 'ar-SA',
    'he': 'he-IL',
  };
  return localeMap[lang] || 'en-US';
}

// =============================================================================
// Translation Helpers
// =============================================================================

/**
 * Get a translation string with optional argument interpolation
 * Supports Fluent-style placeholders: { $argName }
 */
function translate(
  translations: Record<string, string>,
  key: string,
  args?: Record<string, string | number>
): string {
  let text = translations[key] || key;

  if (args) {
    Object.entries(args).forEach(([argKey, value]) => {
      // Replace Fluent-style placeholders: { $argName }
      text = text.replace(new RegExp(`\\{\\s*\\$${argKey}\\s*\\}`, 'g'), String(value));
      // Also support simple placeholders: { argName }
      text = text.replace(new RegExp(`\\{\\s*${argKey}\\s*\\}`, 'g'), String(value));
    });
  }

  return text;
}

// =============================================================================
// Store Creation
// =============================================================================

export const useI18nStore = create<I18nState>()(
  persist(
    (set, get) => ({
      // Initial state
      currentLanguage: DEFAULT_LANGUAGE,
      languages: SUPPORTED_LANGUAGES,
      isRTL: false,
      translations: {},
      isLoading: false,
      error: null,

      // Set language and load translations
      setLanguage: async (lang: LanguageCode) => {
        const language = SUPPORTED_LANGUAGES.find(l => l.code === lang);
        if (!language) {
          set({ error: `Unsupported language: ${lang}` });
          return;
        }

        set({ isLoading: true, error: null });

        try {
          const translations = await loadTranslationFile(lang);
          set({
            currentLanguage: lang,
            isRTL: language.isRTL,
            translations,
            isLoading: false,
          });

          // Update document direction for RTL support
          document.documentElement.dir = language.isRTL ? 'rtl' : 'ltr';
          document.documentElement.lang = lang;

          // Update CSS class for RTL styling
          if (language.isRTL) {
            document.body.classList.add('rtl');
          } else {
            document.body.classList.remove('rtl');
          }
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : 'Failed to load translations',
            isLoading: false,
          });
        }
      },

      // Load translations for a language
      loadTranslations: async (lang: LanguageCode) => {
        set({ isLoading: true });
        try {
          const translations = await loadTranslationFile(lang);
          set({ translations, isLoading: false });
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : 'Failed to load translations',
            isLoading: false,
          });
        }
      },

      // Translate a key
      t: (key: string, args?: Record<string, string | number>) => {
        return translate(get().translations, key, args);
      },

      // Format number
      formatNumber: (num: number) => {
        return formatNumberForLocale(num, get().currentLanguage);
      },

      // Format date
      formatDate: (date: Date | number) => {
        return formatDateForLocale(date, get().currentLanguage);
      },

      // Format datetime
      formatDateTime: (date: Date | number) => {
        return formatDateTimeForLocale(date, get().currentLanguage);
      },

      // Get text direction
      getTextDirection: () => {
        return get().isRTL ? 'rtl' : 'ltr';
      },

      // Get language info by code
      getLanguageByCode: (code: LanguageCode) => {
        return SUPPORTED_LANGUAGES.find(l => l.code === code);
      },
    }),
    {
      name: 'easyssh-i18n',
      partialize: (state) => ({
        currentLanguage: state.currentLanguage,
      }),
    }
  )
);

// =============================================================================
// React Hook Helpers
// =============================================================================

/**
 * Hook to get the current translation function
 */
export function useTranslation() {
  const t = useI18nStore(state => state.t);
  const currentLanguage = useI18nStore(state => state.currentLanguage);
  const isRTL = useI18nStore(state => state.isRTL);
  const isLoading = useI18nStore(state => state.isLoading);

  return { t, currentLanguage, isRTL, isLoading };
}

/**
 * Hook to get formatting functions
 */
export function useFormat() {
  const formatNumber = useI18nStore(state => state.formatNumber);
  const formatDate = useI18nStore(state => state.formatDate);
  const formatDateTime = useI18nStore(state => state.formatDateTime);
  const getTextDirection = useI18nStore(state => state.getTextDirection);

  return { formatNumber, formatDate, formatDateTime, getTextDirection };
}

/**
 * Hook to get language management functions
 */
export function useLanguage() {
  const languages = useI18nStore(state => state.languages);
  const currentLanguage = useI18nStore(state => state.currentLanguage);
  const setLanguage = useI18nStore(state => state.setLanguage);
  const getLanguageByCode = useI18nStore(state => state.getLanguageByCode);
  const isRTL = useI18nStore(state => state.isRTL);

  return {
    languages,
    currentLanguage,
    setLanguage,
    getLanguageByCode,
    isRTL,
    currentLanguageInfo: getLanguageByCode(currentLanguage),
  };
}

// =============================================================================
// Initialization
// =============================================================================

/**
 * Initialize i18n system
 * Should be called on app startup
 */
export async function initializeI18n() {
  const store = useI18nStore.getState();

  // Detect system language if no preference saved
  const savedLang = store.currentLanguage;
  if (!savedLang || savedLang === DEFAULT_LANGUAGE) {
    // Try to detect system language
    const systemLang = detectSystemLanguage();
    if (systemLang && systemLang !== DEFAULT_LANGUAGE) {
      await store.setLanguage(systemLang);
      return;
    }
  }

  // Load translations for current language
  await store.setLanguage(store.currentLanguage);
}

/**
 * Detect system language from browser
 */
function detectSystemLanguage(): LanguageCode | null {
  const browserLang = navigator.language || (navigator as any).userLanguage;

  if (!browserLang) return null;

  // Try exact match first
  const exactMatch = SUPPORTED_LANGUAGES.find(l => l.code === browserLang);
  if (exactMatch) return exactMatch.code;

  // Try matching base language (e.g., "zh" for "zh-CN")
  const baseLang = browserLang.split('-')[0];
  const baseMatch = SUPPORTED_LANGUAGES.find(l => l.code.startsWith(baseLang));
  if (baseMatch) return baseMatch.code;

  return null;
}

// =============================================================================
// Utility Functions
// =============================================================================

/**
 * Check if a language code is RTL
 */
export function isRTLLanguage(code: LanguageCode): boolean {
  return RTL_LANGUAGES.includes(code);
}

/**
 * Get CSS class for RTL layouts
 */
export function getRTLDirectionClass(isRTL: boolean): string {
  return isRTL ? 'rtl' : 'ltr';
}

/**
 * Get language display name
 */
export function getLanguageDisplayName(code: LanguageCode, native = true): string {
  const lang = SUPPORTED_LANGUAGES.find(l => l.code === code);
  if (!lang) return code;
  return native ? lang.nativeName : lang.name;
}

// Default export for convenience
export default useI18nStore;
