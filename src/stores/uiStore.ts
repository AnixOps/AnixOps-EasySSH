import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { ProductMode } from '../productModes';

export interface UiState {
  productMode: ProductMode;
  theme: 'light' | 'dark';
}

interface UiActions {
  setProductMode: (mode: ProductMode) => void;
  setTheme: (theme: 'light' | 'dark') => void;
  toggleTheme: () => void;
}

export const useUiStore = create<UiState & UiActions>()(
  persist(
    (set, get) => ({
      productMode: 'lite',
      theme: 'dark',
      setProductMode: (productMode) => set({ productMode }),
      setTheme: (theme) => set({ theme }),
      toggleTheme: () => set({ theme: get().theme === 'dark' ? 'light' : 'dark' }),
    }),
    {
      name: 'easyssh-ui-state',
    }
  )
);
