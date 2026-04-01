/**
 * TopBar - Application Header Component
 *
 * Provides window controls, navigation breadcrumbs, and quick actions
 * following Apple Design System patterns.
 *
 * @module components/navigation/TopBar
 */

import React, { forwardRef, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ChevronRight,
  Search,
  Menu,
  X,
  Minus,
  Square,
  MoreHorizontal,
  type LucideIcon,
} from 'lucide-react';

// =============================================================================
// Types
// =============================================================================

/**
 * Breadcrumb item
 */
export interface BreadcrumbItem {
  /** Item label */
  label: string;
  /** Click handler */
  onClick?: () => void;
  /** Item icon */
  icon?: LucideIcon;
}

/**
 * Quick action button
 */
export interface QuickAction {
  /** Action ID */
  id: string;
  /** Action icon */
  icon: LucideIcon;
  /** Action label */
  label: string;
  /** Click handler */
  onClick: () => void;
  /** Keyboard shortcut */
  shortcut?: string;
  /** Disabled state */
  disabled?: boolean;
  /** Badge count */
  badge?: number;
}

/**
 * TopBar component props
 */
export interface TopBarProps {
  /** Window title */
  title?: string;
  /** Breadcrumb items */
  breadcrumbs?: BreadcrumbItem[];
  /** Quick action buttons */
  actions?: QuickAction[];
  /** Search component or handler */
  onSearch?: () => void;
  /** Menu toggle handler */
  onToggleMenu?: () => void;
  /** Show/hide sidebar toggle */
  showSidebarToggle?: boolean;
  /** Whether sidebar is visible */
  sidebarVisible?: boolean;
  /** Custom class name */
  className?: string;
}

// =============================================================================
// Sub-components
// =============================================================================

/**
 * Window Controls (macOS style)
 */
const WindowControls = () => {
  const [hovered, setHovered] = useState<string | null>(null);

  return (
    <div
      className="flex items-center gap-apple-2 px-apple-3"
      onMouseLeave={() => setHovered(null)}
    >
      {/* Close */}
      <motion.button
        className="w-3 h-3 rounded-full bg-[#ff5f57] flex items-center justify-center"
        onHoverStart={() => setHovered('close')}
        whileHover={{ scale: 1.1 }}
        whileTap={{ scale: 0.9 }}
        onClick={() => {}}
        title="关闭"
      >
        {hovered === 'close' && (
          <X className="w-2 h-2 text-black/60" strokeWidth={3} />
        )}
      </motion.button>

      {/* Minimize */}
      <motion.button
        className="w-3 h-3 rounded-full bg-[#febc2e] flex items-center justify-center"
        onHoverStart={() => setHovered('minimize')}
        whileHover={{ scale: 1.1 }}
        whileTap={{ scale: 0.9 }}
        onClick={() => {}}
        title="最小化"
      >
        {hovered === 'minimize' && (
          <Minus className="w-2 h-2 text-black/60" strokeWidth={3} />
        )}
      </motion.button>

      {/* Maximize */}
      <motion.button
        className="w-3 h-3 rounded-full bg-[#28c840] flex items-center justify-center"
        onHoverStart={() => setHovered('maximize')}
        whileHover={{ scale: 1.1 }}
        whileTap={{ scale: 0.9 }}
        onClick={() => {}}
        title="最大化"
      >
        {hovered === 'maximize' && (
          <Square className="w-1.5 h-1.5 text-black/60" strokeWidth={3} />
        )}
      </motion.button>
    </div>
  );
};

/**
 * Breadcrumbs
 */
const Breadcrumbs = ({ items }: { items: BreadcrumbItem[] }) => {
  if (items.length === 0) return null;

  return (
    <nav
      className="flex items-center gap-apple-1 text-apple-sm"
      aria-label="Breadcrumb"
    >
      {items.map((item, index) => {
        const Icon = item.icon;
        const isLast = index === items.length - 1;

        return (
          <React.Fragment key={index}>
            {index > 0 && (
              <ChevronRight className="w-3.5 h-3.5 text-apple-text-tertiary flex-shrink-0" />
            )}
            <button
              onClick={item.onClick}
              disabled={!item.onClick || isLast}
              className={`flex items-center gap-apple-1 px-apple-1.5 py-apple-0.5 rounded-apple-sm transition-colors ${
                isLast
                  ? 'text-apple-text-primary font-medium cursor-default'
                  : 'text-apple-text-secondary hover:text-apple-text-primary hover:bg-apple-bg-tertiary'
              }`}
            >
              {Icon && <Icon className="w-3.5 h-3.5" />}
              <span className="truncate max-w-[200px]">{item.label}</span>
            </button>
          </React.Fragment>
        );
      })}
    </nav>
  );
};

/**
 * Quick Actions
 */
const QuickActions = ({ actions }: { actions: QuickAction[] }) => {
  const [showMore, setShowMore] = useState(false);

  // Separate visible and overflow actions
  const visibleActions = actions.slice(0, 4);
  const overflowActions = actions.slice(4);

  return (
    <div className="flex items-center gap-apple-1">
      {visibleActions.map((action) => {
        const Icon = action.icon;
        return (
          <motion.button
            key={action.id}
            className={`relative p-apple-2 rounded-apple-md transition-all duration-100 ${
              action.disabled
                ? 'text-apple-text-quaternary cursor-not-allowed'
                : 'text-apple-text-secondary hover:text-apple-text-primary hover:bg-apple-bg-tertiary'
            }`}
            onClick={action.onClick}
            disabled={action.disabled}
            whileTap={!action.disabled ? { scale: 0.9 } : undefined}
            title={action.shortcut ? `${action.label} (${action.shortcut})` : action.label}
          >
            <Icon className="w-4 h-4" />
            {action.badge !== undefined && action.badge > 0 && (
              <span className="absolute -top-0.5 -right-0.5 w-4 h-4 text-[10px] font-medium bg-apple-accent-red text-white rounded-full flex items-center justify-center">
                {action.badge > 99 ? '99+' : action.badge}
              </span>
            )}
          </motion.button>
        );
      })}

      {/* More Actions Dropdown */}
      {overflowActions.length > 0 && (
        <div className="relative">
          <motion.button
            className="p-apple-2 rounded-apple-md text-apple-text-secondary hover:text-apple-text-primary hover:bg-apple-bg-tertiary transition-all"
            onClick={() => setShowMore(!showMore)}
            whileTap={{ scale: 0.9 }}
          >
            <MoreHorizontal className="w-4 h-4" />
          </motion.button>

          <AnimatePresence>
            {showMore && (
              <>
                <motion.div
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 8 }}
                  transition={{ duration: 0.15 }}
                  className="absolute right-0 top-full mt-apple-1 w-48 bg-apple-bg-secondary/95 apple-blur border border-apple-border rounded-apple-lg shadow-apple-lg py-apple-1 z-50"
                >
                  {overflowActions.map((action) => {
                    const Icon = action.icon;
                    return (
                      <button
                        key={action.id}
                        className="w-full flex items-center gap-apple-3 px-apple-3 py-apple-2 text-apple-sm text-apple-text-primary hover:bg-apple-accent-blue/10 transition-colors text-left"
                        onClick={() => {
                          action.onClick();
                          setShowMore(false);
                        }}
                        disabled={action.disabled}
                      >
                        <Icon className="w-4 h-4 text-apple-text-secondary" />
                        <span>{action.label}</span>
                        {action.shortcut && (
                          <span className="ml-auto text-apple-text-tertiary text-apple-xs">
                            {action.shortcut}
                          </span>
                        )}
                      </button>
                    );
                  })}
                </motion.div>
                <div
                  className="fixed inset-0 z-40"
                  onClick={() => setShowMore(false)}
                />
              </>
            )}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
};

// =============================================================================
// Main Component
// =============================================================================

/**
 * TopBar - Application header with navigation and actions
 */
export const TopBar = forwardRef<HTMLElement, TopBarProps>(
  (
    {
      title,
      breadcrumbs,
      actions,
      onSearch,
      onToggleMenu,
      showSidebarToggle = true,
      sidebarVisible = true,
      className = '',
    },
    ref
  ) => {
    return (
      <nav
        ref={ref}
        className={`h-full flex items-center justify-between px-apple-2 ${className}`}
        data-testid="top-bar"
      >
        {/* Left Section */}
        <div className="flex items-center gap-apple-3 flex-1 min-w-0">
          {/* Window Controls (macOS style) */}
          <WindowControls />

          {/* Sidebar Toggle */}
          {showSidebarToggle && onToggleMenu && (
            <motion.button
              className={`p-apple-2 rounded-apple-md transition-all ${
                sidebarVisible
                  ? 'text-apple-accent-blue bg-apple-accent-blue/10'
                  : 'text-apple-text-secondary hover:text-apple-text-primary hover:bg-apple-bg-tertiary'
              }`}
              onClick={onToggleMenu}
              whileTap={{ scale: 0.9 }}
              title={sidebarVisible ? '隐藏侧边栏' : '显示侧边栏'}
            >
              <Menu className="w-4 h-4" />
            </motion.button>
          )}

          {/* Breadcrumbs */}
          {breadcrumbs && breadcrumbs.length > 0 ? (
            <Breadcrumbs items={breadcrumbs} />
          ) : title ? (
            <h1 className="text-apple-md font-semibold text-apple-text-primary truncate">
              {title}
            </h1>
          ) : null}
        </div>

        {/* Right Section */}
        <div className="flex items-center gap-apple-2">
          {/* Search */}
          {onSearch && (
            <motion.button
              className="flex items-center gap-apple-2 px-apple-3 py-apple-1.5 rounded-apple-md bg-apple-bg-tertiary text-apple-text-secondary hover:text-apple-text-primary transition-all text-apple-sm"
              onClick={onSearch}
              whileTap={{ scale: 0.98 }}
            >
              <Search className="w-3.5 h-3.5" />
              <span>搜索</span>
              <kbd className="px-apple-1.5 py-apple-0.5 rounded-apple-sm bg-apple-bg-quaternary text-apple-text-tertiary text-apple-xs">
                ⌘K
              </kbd>
            </motion.button>
          )}

          {/* Quick Actions */}
          {actions && actions.length > 0 && (
            <div className="w-px h-5 bg-apple-border mx-apple-1" />
          )}
          {actions && <QuickActions actions={actions} />}
        </div>
      </nav>
    );
  }
);

TopBar.displayName = 'TopBar';

export default TopBar;
