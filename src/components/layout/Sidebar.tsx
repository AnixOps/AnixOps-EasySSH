/**
 * Sidebar - Navigation Sidebar Component
 *
 * Implements Apple Design System sidebar patterns with support for
 * collapsible sections, drag-and-drop groups, and keyboard navigation.
 *
 * @module components/layout/Sidebar
 */

import React, {
  forwardRef,
  useState,
  useCallback,
  useMemo,
  useRef,
  useEffect,
} from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ChevronRight,
  Plus,
  Search,
  Folder,
  Server,
  Settings,
} from 'lucide-react';
import type { Server as ServerType, ServerGroup } from '../../types/index.js';

// =============================================================================
// Types
// =============================================================================

/**
 * Sidebar component props
 */
export interface SidebarProps {
  /** Server groups to display */
  groups: ServerGroup[];
  /** Servers to display */
  servers: ServerType[];
  /** Currently selected item ID */
  selectedId?: string | null;
  /** Callback when item is selected */
  onSelect?: (id: string, type: 'server' | 'group') => void;
  /** Callback when add server is clicked */
  onAddServer?: () => void;
  /** Callback when add group is clicked */
  onAddGroup?: () => void;
  /** Callback when settings is clicked */
  onSettings?: () => void;
  /** Search query */
  searchQuery?: string;
  /** Callback when search changes */
  onSearch?: (query: string) => void;
  /** Whether sidebar is collapsed */
  collapsed?: boolean;
  /** Custom class name */
  className?: string;
}

/**
 * Tree node for recursive rendering
 */
interface TreeNode {
  id: string;
  type: 'server' | 'group';
  data: ServerType | ServerGroup;
  children: TreeNode[];
  level: number;
}

// =============================================================================
// Constants
// =============================================================================

const INDENT_SIZE = 12;

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Build tree structure from groups and servers
 */
function buildTree(
  groups: ServerGroup[],
  servers: ServerType[],
  parentId: string | null = null,
  level = 0
): TreeNode[] {
  const groupNodes: TreeNode[] = groups
    .filter((g) => g.parentId === parentId)
    .sort((a, b) => a.order - b.order)
    .map((group) => ({
      id: group.id,
      type: 'group' as const,
      data: group,
      children: buildTree(groups, servers, group.id, level + 1),
      level,
    }));

  const serverNodes: TreeNode[] = servers
    .filter((s) => s.groupId === parentId)
    .map((server) => ({
      id: server.id,
      type: 'server' as const,
      data: server,
      children: [],
      level,
    }));

  return [...groupNodes, ...serverNodes];
}

// =============================================================================
// Sub-components
// =============================================================================

/**
 * Sidebar Item Component
 */
interface SidebarItemProps {
  node: TreeNode;
  isSelected: boolean;
  isExpanded: boolean;
  collapsed: boolean;
  onSelect: (id: string, type: 'server' | 'group') => void;
  onToggleExpand: (id: string) => void;
}

const SidebarItemComponent = React.memo(
  ({
    node,
    isSelected,
    isExpanded,
    collapsed,
    onSelect,
    onToggleExpand,
  }: SidebarItemProps) => {
    const isGroup = node.type === 'group';
    const hasChildren = node.children.length > 0;
    const groupData = isGroup ? (node.data as ServerGroup) : null;
    const serverData = !isGroup ? (node.data as ServerType) : null;
    const color = groupData?.color || serverData?.color;

    const handleClick = useCallback(() => {
      if (isGroup && hasChildren) {
        onToggleExpand(node.id);
      }
      onSelect(node.id, node.type);
    }, [node.id, node.type, isGroup, hasChildren, onSelect, onToggleExpand]);

    const handleKeyDown = useCallback(
      (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          handleClick();
        }
        if (e.key === 'ArrowRight' && isGroup && !isExpanded) {
          onToggleExpand(node.id);
        }
        if (e.key === 'ArrowLeft' && isGroup && isExpanded) {
          onToggleExpand(node.id);
        }
      },
      [handleClick, isGroup, isExpanded, onToggleExpand, node.id]
    );

    if (collapsed) {
      return (
        <motion.button
          className={`w-full aspect-square flex items-center justify-center rounded-apple-md transition-all duration-150 ${
            isSelected
              ? 'bg-apple-accent-blue text-white'
              : 'text-apple-text-secondary hover:bg-apple-bg-tertiary hover:text-apple-text-primary'
          }`}
          onClick={handleClick}
          whileTap={{ scale: 0.95 }}
          title={node.type === 'server' ? serverData?.name : groupData?.name}
        >
          {isGroup ? (
            <Folder className="w-5 h-5" style={{ color }} />
          ) : (
            <Server className="w-5 h-5" style={{ color }} />
          )}
        </motion.button>
      );
    }

    return (
      <div className="w-full">
        <motion.button
          className={`w-full flex items-center gap-apple-2 px-apple-3 py-apple-1.5 rounded-apple-md text-left transition-all duration-100 ${
            isSelected
              ? 'bg-apple-accent-blue/20 text-apple-text-primary'
              : 'text-apple-text-secondary hover:bg-apple-bg-tertiary hover:text-apple-text-primary'
          }`}
          style={{ paddingLeft: `${12 + node.level * INDENT_SIZE}px` }}
          onClick={handleClick}
          onKeyDown={handleKeyDown}
          whileTap={{ scale: 0.98 }}
          tabIndex={0}
          role={isGroup ? 'treeitem' : 'button'}
          aria-expanded={isGroup ? isExpanded : undefined}
          aria-selected={isSelected}
        >
          {/* Expand/Collapse Icon */}
          {isGroup && hasChildren && (
            <span
              className="flex-shrink-0 w-4 h-4 flex items-center justify-center"
              onClick={(e) => {
                e.stopPropagation();
                onToggleExpand(node.id);
              }}
            >
              <ChevronRight
                className={`w-3.5 h-3.5 transition-transform duration-200 ${
                  isExpanded ? 'rotate-90' : ''
                }`}
              />
            </span>
          )}

          {isGroup && !hasChildren && (
            <span className="flex-shrink-0 w-4" />
          )}

          {/* Icon */}
          <span className="flex-shrink-0">
            {isGroup ? (
              <Folder className="w-4 h-4" style={{ color }} />
            ) : (
              <Server className="w-4 h-4" style={{ color }} />
            )}
          </span>

          {/* Label */}
          <span className="flex-1 truncate text-apple-sm font-medium">
            {isGroup ? groupData?.name : serverData?.name}
          </span>

          {/* Connection indicator for servers */}
          {!isGroup && serverData && (
            <span
              className={`flex-shrink-0 w-2 h-2 rounded-full ${
                serverData.lastConnectedAt
                  ? 'bg-apple-accent-green'
                  : 'bg-apple-text-quaternary'
              }`}
              title={
                serverData.lastConnectedAt
                  ? 'Previously connected'
                  : 'Never connected'
              }
            />
          )}
        </motion.button>

        {/* Children */}
        <AnimatePresence initial={false}>
          {isGroup && hasChildren && isExpanded && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.2, ease: [0.4, 0, 0.2, 1] }}
              className="overflow-hidden"
            >
              {node.children.map((child) => (
                <SidebarItemComponent
                  key={child.id}
                  node={child}
                  isSelected={isSelected}
                  isExpanded={isExpanded}
                  collapsed={collapsed}
                  onSelect={onSelect}
                  onToggleExpand={onToggleExpand}
                />
              ))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  }
);

SidebarItemComponent.displayName = 'SidebarItemComponent';

// =============================================================================
// Main Component
// =============================================================================

/**
 * Sidebar - Navigation sidebar with groups and servers
 */
export const Sidebar = forwardRef<HTMLDivElement, SidebarProps>(
  (
    {
      groups,
      servers,
      selectedId,
      onSelect,
      onAddServer,
      onAddGroup,
      onSettings,
      searchQuery = '',
      onSearch,
      collapsed = false,
      className = '',
    },
    ref
  ) => {
    const [expandedGroups, setExpandedGroups] = useState<Set<string>>(() => {
      // Expand all groups by default
      return new Set(groups.map((g) => g.id));
    });

    const searchInputRef = useRef<HTMLInputElement>(null);

    // Build tree structure
    const treeData = useMemo(() => {
      if (searchQuery) {
        // Flat list when searching
        const filteredServers = servers.filter(
          (s) =>
            s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            s.host.toLowerCase().includes(searchQuery.toLowerCase())
        );
        return filteredServers.map((s) => ({
          id: s.id,
          type: 'server' as const,
          data: s,
          children: [],
          level: 0,
        }));
      }
      return buildTree(groups, servers);
    }, [groups, servers, searchQuery]);

    // Toggle group expansion
    const handleToggleExpand = useCallback((id: string) => {
      setExpandedGroups((prev) => {
        const next = new Set(prev);
        if (next.has(id)) {
          next.delete(id);
        } else {
          next.add(id);
        }
        return next;
      });
    }, []);

    // Keyboard shortcut: Cmd/Ctrl+F to focus search
    useEffect(() => {
      const handleKeyDown = (e: KeyboardEvent) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
          e.preventDefault();
          searchInputRef.current?.focus();
        }
      };
      document.addEventListener('keydown', handleKeyDown);
      return () => document.removeEventListener('keydown', handleKeyDown);
    }, []);

    if (collapsed) {
      return (
        <div
          ref={ref}
          className={`h-full flex flex-col bg-apple-bg-secondary ${className}`}
          data-testid="sidebar"
        >
          {/* Collapsed Header */}
          <div className="flex items-center justify-center p-apple-3 border-b border-apple-border">
            <div className="w-8 h-8 rounded-apple-md bg-apple-accent-blue/20 flex items-center justify-center">
              <Server className="w-5 h-5 text-apple-accent-blue" />
            </div>
          </div>

          {/* Collapsed Content */}
          <div className="flex-1 overflow-y-auto py-apple-2 px-apple-2 space-y-apple-1">
            {treeData.map((node) => (
              <SidebarItemComponent
                key={node.id}
                node={node}
                isSelected={selectedId === node.id}
                isExpanded={expandedGroups.has(node.id)}
                collapsed={true}
                onSelect={onSelect || (() => {})}
                onToggleExpand={handleToggleExpand}
              />
            ))}
          </div>

          {/* Collapsed Footer */}
          <div className="p-apple-2 border-t border-apple-border space-y-apple-1">
            {onAddServer && (
              <motion.button
                className="w-full aspect-square flex items-center justify-center rounded-apple-md text-apple-text-secondary hover:bg-apple-bg-tertiary hover:text-apple-text-primary transition-colors"
                onClick={onAddServer}
                whileTap={{ scale: 0.95 }}
                title="添加服务器"
              >
                <Plus className="w-5 h-5" />
              </motion.button>
            )}
            {onSettings && (
              <motion.button
                className="w-full aspect-square flex items-center justify-center rounded-apple-md text-apple-text-secondary hover:bg-apple-bg-tertiary hover:text-apple-text-primary transition-colors"
                onClick={onSettings}
                whileTap={{ scale: 0.95 }}
                title="设置"
              >
                <Settings className="w-5 h-5" />
              </motion.button>
            )}
          </div>
        </div>
      );
    }

    return (
      <div
        ref={ref}
        className={`h-full flex flex-col bg-apple-bg-secondary ${className}`}
        data-testid="sidebar"
      >
        {/* Header */}
        <div className="flex items-center gap-apple-2 p-apple-3 border-b border-apple-border">
          <div className="flex-1 flex items-center gap-apple-2">
            <div className="w-7 h-7 rounded-apple-md bg-apple-accent-blue/20 flex items-center justify-center flex-shrink-0">
              <Server className="w-4 h-4 text-apple-accent-blue" />
            </div>
            <span className="font-semibold text-apple-text-primary">
              EasySSH
            </span>
          </div>
        </div>

        {/* Search */}
        {onSearch && (
          <div className="px-apple-3 py-apple-2">
            <div className="relative">
              <Search className="absolute left-apple-2 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text-tertiary" />
              <input
                ref={searchInputRef}
                type="text"
                value={searchQuery}
                onChange={(e) => onSearch(e.target.value)}
                placeholder="搜索服务器..."
                className="w-full pl-apple-8 pr-apple-3 py-apple-1.5 bg-apple-bg-tertiary border border-apple-border rounded-apple-md text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:outline-none focus:border-apple-accent-blue/50 focus:ring-2 focus:ring-apple-accent-blue/20 transition-all"
              />
            </div>
          </div>
        )}

        {/* Content */}
        <div className="flex-1 overflow-y-auto apple-scrollbar py-apple-2 px-apple-2 space-y-apple-0.5">
          {searchQuery && treeData.length === 0 ? (
            <div className="px-apple-3 py-apple-4 text-center text-apple-text-tertiary text-apple-sm">
              未找到匹配的服务器
            </div>
          ) : (
            treeData.map((node) => (
              <SidebarItemComponent
                key={node.id}
                node={node}
                isSelected={selectedId === node.id}
                isExpanded={expandedGroups.has(node.id)}
                collapsed={false}
                onSelect={onSelect || (() => {})}
                onToggleExpand={handleToggleExpand}
              />
            ))
          )}
        </div>

        {/* Footer */}
        <div className="p-apple-2 border-t border-apple-border space-y-apple-1">
          {onAddServer && (
            <motion.button
              className="w-full flex items-center gap-apple-2 px-apple-3 py-apple-2 rounded-apple-md text-apple-text-secondary hover:bg-apple-bg-tertiary hover:text-apple-text-primary transition-colors text-apple-sm"
              onClick={onAddServer}
              whileTap={{ scale: 0.98 }}
            >
              <Plus className="w-4 h-4" />
              <span>添加服务器</span>
            </motion.button>
          )}
          {onAddGroup && (
            <motion.button
              className="w-full flex items-center gap-apple-2 px-apple-3 py-apple-2 rounded-apple-md text-apple-text-secondary hover:bg-apple-bg-tertiary hover:text-apple-text-primary transition-colors text-apple-sm"
              onClick={onAddGroup}
              whileTap={{ scale: 0.98 }}
            >
              <Folder className="w-4 h-4" />
              <span>新建分组</span>
            </motion.button>
          )}
          {onSettings && (
            <motion.button
              className="w-full flex items-center gap-apple-2 px-apple-3 py-apple-2 rounded-apple-md text-apple-text-secondary hover:bg-apple-bg-tertiary hover:text-apple-text-primary transition-colors text-apple-sm"
              onClick={onSettings}
              whileTap={{ scale: 0.98 }}
            >
              <Settings className="w-4 h-4" />
              <span>设置</span>
            </motion.button>
          )}
        </div>
      </div>
    );
  }
);

Sidebar.displayName = 'Sidebar';

export default Sidebar;
