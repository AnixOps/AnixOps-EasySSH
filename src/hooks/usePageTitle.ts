import { useEffect } from 'react';
import { generatePageTitle } from '../utils';

/**
 * Hook to update document title based on current context
 */
export function usePageTitle(section?: string) {
  useEffect(() => {
    const previousTitle = document.title;
    document.title = generatePageTitle(section);

    return () => {
      document.title = previousTitle;
    };
  }, [section]);
}
