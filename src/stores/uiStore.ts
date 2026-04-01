/**
 * UI State Management using Zustand
 * @module stores/uiStore
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Theme, AppSettings } from '../types/index.js';

// =============================================================================
// Types
// =============================================================================

/**
 * UI Store State
 */
interface UIState {
  // Theme
  theme: Theme;
  setTheme: (theme: Theme) => void;

  // Sidebar
  sidebarCollapsed: boolean;
  sidebarWidth: number;
  activeSidebarItem: string | null;
  toggleSidebar: () => void;
  setSidebarWidth: (width: number) => void;
  setActiveSidebarItem: (id: string | null) => void;

  // Command Palette
  commandPaletteOpen: boolean;
  commandPaletteQuery: string;
  openCommandPalette: () => void;
  closeCommandPalette: () => void;
  setCommandPaletteQuery: (query: string) => void;

  // Modal Stack
  activeModals: string[];
  openModal: (id: string) => void;
  closeModal: (id: string) => void;
  isModalOpen: (id: string) => boolean;

  // Toast Notifications
  toasts: Toast[];
  addToast: (toast: Omit<Toast, 'id'>) => void;
  removeToast: (id: string) => void;
  clearToasts: () => void;

  // Loading States
  loadingStates: Record<string, boolean>;
  setLoading: (key: string, loading: boolean) => void;
  isLoading: (key: string) => boolean;

  // Search
  searchQuery: string;
  searchResults: SearchResult[];
  isSearching: boolean;
  setSearchQuery: (query: string) => void;
  setSearchResults: (results: SearchResult[]) => void;
  setIsSearching: (searching: boolean) => void;
  clearSearch: () => void;

  // App Settings
  settings: AppSettings;
  updateSettings: (settings: Partial<AppSettings>) => void;
}

/**
 * Toast Notification
 */
export interface Toast {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  title: string;
  message?: string;
  duration?: number;
  action?: {
    label: string;
    onClick: () => void;
  };
}

/**
 * Search Result
 */
interface SearchResult {
  id: string;
  type: 'server' | 'group' | 'setting' | 'action';
  title: string;
  subtitle?: string;
  icon?: string;
  onClick: () => void;
}

// =============================================================================
// Default Settings
// =============================================================================

const defaultSettings: AppSettings = {
  theme: 'system',
  language: 'zh-CN',
  sidebarWidth: 240,
  sidebarCollapsed: false,
  terminal: {
    fontFamily: 'SF Mono, Menlo, Monaco, Consolas, monospace',
    fontSize: 14,
    lineHeight: 1.5,
    cursorStyle: 'block',
    cursorBlink: true,
    scrollback: 10000,
    useWebGL: true,
    copyOnSelect: true,
    rightClickBehavior: 'contextMenu',
  },
  security: {
    autoLockTimeout: 0,
    clearClipboardOnExit: true,
    confirmBulkOperations: true,
    showConnectionNotifications: true,
  },
};

// =============================================================================
// Store Creation
// =============================================================================

export const useUIStore = create<UIState>()(
  persist(
    (set, get) => ({
      // Theme
      theme: 'system',
      setTheme: (theme) => set({ theme }),

      // Sidebar
      sidebarCollapsed: false,
      sidebarWidth: 240,
      activeSidebarItem: null,
      toggleSidebar: () =>
        set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      setSidebarWidth: (width) => set({ sidebarWidth: width }),
      setActiveSidebarItem: (id) => set({ activeSidebarItem: id }),

      // Command Palette
      commandPaletteOpen: false,
      commandPaletteQuery: '',
      openCommandPalette: () => set({ commandPaletteOpen: true }),
      closeCommandPalette: () =>
        set({ commandPaletteOpen: false, commandPaletteQuery: '' }),
      setCommandPaletteQuery: (query) => set({ commandPaletteQuery: query }),

      // Modal Stack
      activeModals: [],
      openModal: (id) =>
        set((state) => ({ activeModals: [...state.activeModals, id] })),
      closeModal: (id) =>
        set((state) => ({
          activeModals: state.activeModals.filter((m) => m !== id),
        })),
      isModalOpen: (id) => get().activeModals.includes(id),

      // Toast Notifications
      toasts: [],
      addToast: (toast) => {
        const id = crypto.randomUUID();
        set((state) => ({
          toasts: [...state.toasts, { ...toast, id }],
        }));

        // Auto-dismiss after duration
        if (toast.duration !== Infinity) {
          setTimeout(() => {
            get().removeToast(id);
          }, toast.duration || 5000);
        }
      },
      removeToast: (id) =>
        set((state) => ({
          toasts: state.toasts.filter((t) => t.id !== id),
        })),
      clearToasts: () => set({ toasts: [] }),

      // Loading States
      loadingStates: {},
      setLoading: (key, loading) =>
        set((state) => ({
          loadingStates: { ...state.loadingStates, [key]: loading },
        })),
      isLoading: (key) => !!get().loadingStates[key],

      // Search
      searchQuery: '',
      searchResults: [],
      isSearching: false,
      setSearchQuery: (query) => set({ searchQuery: query }),
      setSearchResults: (results) => set({ searchResults: results }),
      setIsSearching: (searching) => set({ isSearching: searching }),
      clearSearch: () =>
        set({ searchQuery: '', searchResults: [], isSearching: false }),

      // Settings
      settings: defaultSettings,
      updateSettings: (newSettings) =>
        set((state) => ({
          settings: { ...state.settings, ...newSettings },
        })),
    }),
    {
      name: 'easyssh-ui',
      partialize: (state) => ({
        theme: state.theme,
        sidebarCollapsed: state.sidebarCollapsed,
        sidebarWidth: state.sidebarWidth,
        settings: state.settings,
      }),
    }
  )
);

// =============================================================================
// Selector Hooks
// =============================================================================

/**
 * Get sidebar state
 */
export const useSidebarState = () =>
  useUIStore((state) => ({
    collapsed: state.sidebarCollapsed,
    width: state.sidebarWidth,
    activeItem: state.activeSidebarItem,
    toggle: state.toggleSidebar,
    setWidth: state.setSidebarWidth,
    setActiveItem: state.setActiveSidebarItem,
  }));

/**
 * Get command palette state
 */
export const useCommandPalette = () =>
  useUIStore((state) => ({
    isOpen: state.commandPaletteOpen,
    query: state.commandPaletteQuery,
    open: state.openCommandPalette,
    close: state.closeCommandPalette,
    setQuery: state.setCommandPaletteQuery,
  }));

/**
 * Get toast actions
 */
export const useToastActions = () =>
  useUIStore((state) => ({
    toasts: state.toasts,
    add: state.addToast,
    remove: state.removeToast,
    clear: state.clearToasts,
  }));

/**
 * Get app settings
 */
export const useAppSettings = () =>
  useUIStore((state) => ({
    settings: state.settings,
    update: state.updateSettings,
    theme: state.settings.theme,
    setTheme: state.setTheme,
  }));
