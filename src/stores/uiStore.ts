import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { ProductMode } from '../productModes';

export interface UiState {
  searchQuery: string;
  productMode: ProductMode;
  theme: 'light' | 'dark';
}

interface UiActions {
  setSearchQuery: (query: string) => void;
  setProductMode: (mode: ProductMode) => void;
  setTheme: (theme: 'light' | 'dark') => void;
  toggleTheme: () => void;
}

export const useUiStore = create<UiState & UiActions>()(
  persist(
    (set, get) => ({
      searchQuery: '',
      productMode: 'lite',
      theme: 'dark',
      setSearchQuery: (query) => set({ searchQuery: query }),
      setProductMode: (productMode) => set({ productMode }),
      setTheme: (theme) => set({ theme }),
      toggleTheme: () => set({ theme: get().theme === 'dark' ? 'light' : 'dark' }),
    }),
    {
      name: 'easyssh-ui-state',
    }
  )
);
