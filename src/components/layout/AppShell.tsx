/**
 * AppShell - Main Application Layout Container
 *
 * Provides the structural foundation for the EasySSH application following
 * Apple Design System patterns. Handles responsive layout, window controls,
 * and overall app chrome.
 *
 * @module components/layout/AppShell
 */

import React, { forwardRef, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import type { AppSettings } from '../../types/index.js';

// =============================================================================
// Types
// =============================================================================

/**
 * AppShell component props
 */
export interface AppShellProps {
  /** Sidebar content (navigation) */
  sidebar?: React.ReactNode;
  /** Main content area */
  children: React.ReactNode;
  /** Top bar / title bar content */
  topBar?: React.ReactNode;
  /** Status bar content (bottom) */
  statusBar?: React.ReactNode;
  /** Command palette component */
  commandPalette?: React.ReactNode;
  /** Toast notifications container */
  toasts?: React.ReactNode;
  /** Modal dialogs */
  modals?: React.ReactNode;
  /** Whether sidebar is collapsed */
  sidebarCollapsed?: boolean;
  /** Sidebar width in pixels (when expanded) */
  sidebarWidth?: number;
  /** Callback when sidebar width changes */
  onSidebarResize?: (width: number) => void;
  /** App settings */
  settings?: AppSettings;
  /** Additional CSS classes */
  className?: string;
}

// =============================================================================
// Constants
// =============================================================================

const MIN_SIDEBAR_WIDTH = 180;
const MAX_SIDEBAR_WIDTH = 400;
const DEFAULT_SIDEBAR_WIDTH = 240;
const SIDEBAR_COLLAPSED_WIDTH = 64;

// =============================================================================
// Component
// =============================================================================

/**
 * AppShell - Main application layout container
 *
 * @example
 * ```tsx
 * <AppShell
 *   sidebar={<Sidebar />}
 *   topBar={<TopBar />}
 *   statusBar={<StatusBar />}
 * >
 *   <Workspace />
 * </AppShell>
 * ```
 */
export const AppShell = forwardRef<HTMLDivElement, AppShellProps>(
  (
    {
      sidebar,
      children,
      topBar,
      statusBar,
      commandPalette,
      toasts,
      modals,
      sidebarCollapsed = false,
      sidebarWidth = DEFAULT_SIDEBAR_WIDTH,
      onSidebarResize,
      className = '',
    },
    ref
  ) => {
    // Handle sidebar resize
    const handleResizeStart = useCallback(
      (e: React.MouseEvent) => {
        if (!onSidebarResize || sidebarCollapsed) return;

        e.preventDefault();
        const startX = e.clientX;
        const startWidth = sidebarWidth;

        const handleMouseMove = (moveEvent: MouseEvent) => {
          const delta = moveEvent.clientX - startX;
          const newWidth = Math.max(
            MIN_SIDEBAR_WIDTH,
            Math.min(MAX_SIDEBAR_WIDTH, startWidth + delta)
          );
          onSidebarResize(newWidth);
        };

        const handleMouseUp = () => {
          document.removeEventListener('mousemove', handleMouseMove);
          document.removeEventListener('mouseup', handleMouseUp);
          document.body.style.cursor = '';
          document.body.style.userSelect = '';
        };

        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none';
        document.addEventListener('mousemove', handleMouseMove);
        document.addEventListener('mouseup', handleMouseUp);
      },
      [sidebarWidth, sidebarCollapsed, onSidebarResize]
    );

    const effectiveSidebarWidth = sidebarCollapsed
      ? SIDEBAR_COLLAPSED_WIDTH
      : sidebarWidth;

    return (
      <div
        ref={ref}
        className={`h-screen w-screen overflow-hidden bg-apple-bg-primary flex flex-col ${className}`}
        data-testid="app-shell"
      >
        {/* Top Bar / Title Bar */}
        {topBar && (
          <header className="flex-shrink-0 h-11 bg-apple-bg-secondary/80 apple-blur border-b border-apple-border tauri-drag">
            {topBar}
          </header>
        )}

        {/* Main Content Area */}
        <div className="flex-1 flex overflow-hidden">
          {/* Sidebar */}
          {sidebar && (
            <>
              <motion.aside
                className="flex-shrink-0 h-full bg-apple-bg-secondary border-r border-apple-border flex flex-col overflow-hidden tauri-no-drag"
                initial={false}
                animate={{ width: effectiveSidebarWidth }}
                transition={{
                  type: 'spring',
                  stiffness: 400,
                  damping: 30,
                }}
                style={{ contain: 'layout size' }}
              >
                {sidebar}
              </motion.aside>

              {/* Resize Handle */}
              {!sidebarCollapsed && onSidebarResize && (
                <div
                  className="w-1 flex-shrink-0 cursor-col-resize hover:bg-apple-accent-blue/30 active:bg-apple-accent-blue/50 transition-colors relative z-10 group"
                  onMouseDown={handleResizeStart}
                  title="拖拽调整侧边栏宽度"
                  role="separator"
                  aria-orientation="vertical"
                  aria-label="调整侧边栏宽度"
                >
                  <div className="absolute inset-y-0 left-1/2 -translate-x-1/2 w-0.5 group-hover:w-1 bg-apple-accent-blue/0 group-hover:bg-apple-accent-blue/50 transition-all" />
                </div>
              )}
            </>
          )}

          {/* Workspace */}
          <main
            className="flex-1 flex flex-col overflow-hidden bg-apple-bg-primary"
            style={{ contain: 'layout style paint' }}
          >
            {children}
          </main>
        </div>

        {/* Status Bar */}
        {statusBar && (
          <footer className="flex-shrink-0 h-6 bg-apple-bg-secondary border-t border-apple-border flex items-center px-apple-3 text-apple-xs text-apple-text-secondary tauri-no-drag">
            {statusBar}
          </footer>
        )}

        {/* Overlays */}
        <AnimatePresence>
          {/* Command Palette */}
          {commandPalette}

          {/* Modals */}
          {modals}
        </AnimatePresence>

        {/* Toast Notifications */}
        {toasts}
      </div>
    );
  }
);

AppShell.displayName = 'AppShell';

// =============================================================================
// Sub-components for convenience
// =============================================================================

/**
 * AppShell Content - Scrollable content container within AppShell
 */
export const AppShellContent = forwardRef<
  HTMLDivElement,
  { children: React.ReactNode; className?: string }
>(({ children, className = '' }, ref) => (
  <div
    ref={ref}
    className={`flex-1 overflow-auto apple-scrollbar ${className}`}
    style={{ contain: 'layout style' }}
  >
    {children}
  </div>
));

AppShellContent.displayName = 'AppShellContent';

/**
 * AppShell Header - Sticky header within content area
 */
export const AppShellHeader = forwardRef<
  HTMLDivElement,
  { children: React.ReactNode; className?: string }
>(({ children, className = '' }, ref) => (
  <div
    ref={ref}
    className={`sticky top-0 z-10 bg-apple-bg-primary/95 apple-blur border-b border-apple-border px-apple-4 py-apple-3 ${className}`}
  >
    {children}
  </div>
));

AppShellHeader.displayName = 'AppShellHeader';

export default AppShell;
