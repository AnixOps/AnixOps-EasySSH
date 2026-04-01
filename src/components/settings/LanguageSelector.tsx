/**
 * Language Selector Component
 *
 * Dropdown for selecting the application language with RTL indicators.
 *
 * @module LanguageSelector
 */

import React, { useCallback } from 'react';
import { Globe, Check } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  useLanguage,
  SUPPORTED_LANGUAGES,
} from '../../stores/i18nStore';
import type { Language, LanguageCode } from '../../stores/i18nStore';

interface LanguageSelectorProps {
  /** Optional className for styling */
  className?: string;
  /** Size variant */
  size?: 'sm' | 'md' | 'lg';
  /** Show native name or English name */
  showNativeName?: boolean;
  /** Include RTL indicator */
  showRTLIndicator?: boolean;
}

const SIZE_CONFIG = {
  sm: {
    button: 'px-2 py-1 text-xs',
    icon: 'w-3 h-3',
    dropdown: 'w-48',
    item: 'px-2 py-1.5 text-xs',
  },
  md: {
    button: 'px-3 py-2 text-sm',
    icon: 'w-4 h-4',
    dropdown: 'w-56',
    item: 'px-3 py-2 text-sm',
  },
  lg: {
    button: 'px-4 py-2.5 text-base',
    icon: 'w-5 h-5',
    dropdown: 'w-64',
    item: 'px-4 py-3 text-base',
  },
};

export const LanguageSelector: React.FC<LanguageSelectorProps> = ({
  className = '',
  size = 'md',
  showNativeName = true,
  showRTLIndicator = true,
}) => {
  const { currentLanguage, setLanguage, currentLanguageInfo } = useLanguage();
  const [isOpen, setIsOpen] = React.useState(false);
  const dropdownRef = React.useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  React.useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSelect = useCallback(async (code: LanguageCode) => {
    await setLanguage(code);
    setIsOpen(false);
  }, [setLanguage]);

  const config = SIZE_CONFIG[size];

  return (
    <div ref={dropdownRef} className={`relative ${className}`}>
      {/* Trigger Button */}
      <motion.button
        onClick={() => setIsOpen(!isOpen)}
        className={`
          flex items-center gap-apple-2 rounded-apple-md
          bg-apple-bg-secondary hover:bg-apple-bg-tertiary
          border border-apple-border
          text-apple-text-primary
          transition-colors
          ${config.button}
        `}
        whileTap={{ scale: 0.98 }}
        aria-expanded={isOpen}
        aria-haspopup="listbox"
      >
        <Globe className={config.icon} />
        <span className="font-medium">
          {showNativeName
            ? currentLanguageInfo?.nativeName
            : currentLanguageInfo?.name}
        </span>
        {showRTLIndicator && currentLanguageInfo?.isRTL && (
          <span className="text-apple-text-tertiary text-xs">(RTL)</span>
        )}
      </motion.button>

      {/* Dropdown Menu */}
      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, y: -8, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -8, scale: 0.95 }}
            transition={{ duration: 0.15 }}
            className={`
              absolute right-0 mt-apple-2 z-50
              ${config.dropdown}
              bg-apple-bg-secondary/95 backdrop-blur-apple-md
              border border-apple-border
              rounded-apple-lg shadow-apple-lg
              overflow-hidden
            `}
            role="listbox"
            aria-label="Select language"
          >
            <div className="max-h-80 overflow-y-auto py-apple-2">
              {SUPPORTED_LANGUAGES.map((lang: Language) => (
                <motion.button
                  key={lang.code}
                  onClick={() => handleSelect(lang.code)}
                  className={`
                    w-full flex items-center justify-between
                    ${config.item}
                    text-apple-text-primary
                    hover:bg-apple-accent-blue/10
                    transition-colors
                    ${currentLanguage === lang.code ? 'bg-apple-accent-blue/10' : ''}
                  `}
                  role="option"
                  aria-selected={currentLanguage === lang.code}
                  whileTap={{ scale: 0.98 }}
                >
                  <div className="flex items-center gap-apple-2">
                    {/* Flag placeholder or language code */}
                    <span className="text-apple-text-tertiary font-mono text-xs">
                      {lang.code.toUpperCase()}
                    </span>

                    <div className="flex flex-col items-start">
                      <span className="font-medium">{lang.nativeName}</span>
                      <span className="text-apple-text-tertiary text-xs">
                        {lang.name}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-apple-2">
                    {lang.isRTL && (
                      <span className="text-apple-accent-orange text-xs">RTL</span>
                    )}
                    {currentLanguage === lang.code && (
                      <Check className="w-4 h-4 text-apple-accent-blue" />
                    )}
                  </div>
                </motion.button>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export default LanguageSelector;
