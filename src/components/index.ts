/**
 * Component exports index
 * @module components
 */

// Layout Components
export { AppShell, AppShellContent, AppShellHeader } from './layout/AppShell';
export type { AppShellProps } from './layout/AppShell';

export { Sidebar } from './layout/Sidebar';
export type { SidebarProps } from './layout/Sidebar';

// Navigation Components
export { TopBar } from './navigation/TopBar';
export type { TopBarProps, BreadcrumbItem, QuickAction } from './navigation/TopBar';

// AI Assistant Components
export {
  AIAssistant,
  ChatMessageList,
  ChatInput,
  ConversationSidebar,
  AISettingsPanel,
  QuickCommandsPanel,
} from './ai-assistant';
export type { AIAssistantProps } from './ai-assistant';
