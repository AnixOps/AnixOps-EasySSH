/**
 * I18n Provider Component
 *
 * Wraps the application with i18n initialization and RTL support.
 *
 * @module I18nProvider
 */

import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Loader2 } from 'lucide-react';
import { initializeI18n, useI18nStore } from '../../stores/i18nStore';
import '../../styles/rtl.css';

interface I18nProviderProps {
  children: React.ReactNode;
}

export const I18nProvider: React.FC<I18nProviderProps> = ({ children }) => {
  const [isInitialized, setIsInitialized] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const isRTL = useI18nStore(state => state.isRTL);

  useEffect(() => {
    const init = async () => {
      try {
        await initializeI18n();
        setIsInitialized(true);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to initialize i18n');
        setIsInitialized(true); // Still show the app even if i18n fails
      }
    };

    init();
  }, []);

  // Update document direction when RTL changes
  useEffect(() => {
    document.documentElement.dir = isRTL ? 'rtl' : 'ltr';
    document.body.classList.toggle('rtl', isRTL);
  }, [isRTL]);

  if (!isInitialized) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-apple-bg-primary">
        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          className="flex flex-col items-center gap-apple-4"
        >
          <Loader2 className="w-8 h-8 text-apple-accent-blue animate-spin" />
          <p className="text-apple-text-secondary">Loading translations...</p>
        </motion.div>
      </div>
    );
  }

  return (
    <>
      <AnimatePresence mode="wait">
        {error && (
          <motion.div
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="fixed top-0 left-0 right-0 z-50 p-apple-4 bg-apple-accent-red/10 border-b border-apple-accent-red/20"
          >
            <p className="text-center text-apple-accent-red text-sm">
              Error loading translations: {error}
            </p>
          </motion.div>
        )}
      </AnimatePresence>
      {children}
    </>
  );
};

export default I18nProvider;
