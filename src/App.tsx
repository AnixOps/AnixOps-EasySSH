/**
 * Main Application Component
 *
 * Root component that initializes the EasySSH application with all
 * core layouts and providers.
 *
 * @module App
 */

import React, { useCallback, useMemo, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Plus,
  Settings,
  Search,
  Terminal,
  X,
  AlertCircle,
  CheckCircle,
  Info,
  Sparkles,
} from 'lucide-react';

// Components
import { AppShell, TopBar, Sidebar, AIAssistant } from './components/index.js';

// Stores
import {
  useSidebarState,
  useServerStore,
  useGroups,
  useServers,
  useSelection,
  useToastActions,
  useSessions,
  useTranslation,
} from './stores/index.js';

// Types
import type { QuickAction } from './components/index.js';
import type { Server } from './types/index.js';
import type { Toast } from './stores/uiStore.js';

// =============================================================================
// Mock Data for Development
// =============================================================================

const MOCK_GROUPS = [
  { id: 'group-1', name: '生产环境', parentId: null, order: 0, createdAt: Date.now(), color: '#ff453a' },
  { id: 'group-2', name: '测试环境', parentId: null, order: 1, createdAt: Date.now(), color: '#ff9f0a' },
  { id: 'group-3', name: '开发环境', parentId: null, order: 2, createdAt: Date.now(), color: '#30d158' },
];

const MOCK_SERVERS = [
  {
    id: 'server-1',
    name: 'Web Server 01',
    host: '192.168.1.10',
    port: 22,
    username: 'admin',
    authMethod: 'key' as const,
    groupId: 'group-1',
    tags: ['web', 'production'],
    color: '#0a84ff',
    createdAt: Date.now(),
    updatedAt: Date.now(),
    lastConnectedAt: Date.now() - 86400000,
  },
  {
    id: 'server-2',
    name: 'Database Master',
    host: 'db.production.local',
    port: 22,
    username: 'dbadmin',
    authMethod: 'key' as const,
    groupId: 'group-1',
    tags: ['database', 'production'],
    color: '#bf5af2',
    createdAt: Date.now(),
    updatedAt: Date.now(),
    lastConnectedAt: Date.now() - 172800000,
  },
  {
    id: 'server-3',
    name: 'Test API Server',
    host: 'api.test.local',
    port: 22,
    username: 'testuser',
    authMethod: 'password' as const,
    groupId: 'group-2',
    tags: ['api', 'testing'],
    color: '#64d2ff',
    createdAt: Date.now(),
    updatedAt: Date.now(),
    lastConnectedAt: undefined,
  },
  {
    id: 'server-4',
    name: 'Local Dev',
    host: 'localhost',
    port: 2222,
    username: 'developer',
    authMethod: 'agent' as const,
    groupId: 'group-3',
    tags: ['local', 'development'],
    color: '#30d158',
    createdAt: Date.now(),
    updatedAt: Date.now(),
    lastConnectedAt: Date.now(),
  },
];

// =============================================================================
// Toast Component
// =============================================================================

const ToastContainer = () => {
  const { toasts, remove } = useToastActions();

  return (
    <div className="fixed bottom-apple-6 right-apple-6 z-50 flex flex-col gap-apple-3 pointer-events-none">
      <AnimatePresence mode="popLayout">
        {toasts.map((toast: Toast) => {
          const Icon =
            toast.type === 'success'
              ? CheckCircle
              : toast.type === 'error'
              ? AlertCircle
              : Info;

          return (
            <motion.div
              key={toast.id}
              initial={{ opacity: 0, y: 20, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, x: 100 }}
              className="pointer-events-auto flex items-start gap-apple-3 px-apple-4 py-apple-3 bg-apple-bg-secondary/95 apple-blur border border-apple-border rounded-apple-lg shadow-apple-lg min-w-[300px] max-w-[400px]"
            >
              <Icon
                className={`w-5 h-5 flex-shrink-0 ${
                  toast.type === 'success'
                    ? 'text-apple-accent-green'
                    : toast.type === 'error'
                    ? 'text-apple-accent-red'
                    : 'text-apple-accent-blue'
                }`}
              />
              <div className="flex-1 min-w-0">
                <h4 className="text-apple-sm font-medium text-apple-text-primary">
                  {toast.title}
                </h4>
                {toast.message && (
                  <p className="mt-apple-1 text-apple-xs text-apple-text-secondary">
                    {toast.message}
                  </p>
                )}
              </div>
              <button
                onClick={() => remove(toast.id)}
                className="flex-shrink-0 text-apple-text-tertiary hover:text-apple-text-primary transition-colors"
              >
                <X className="w-4 h-4" />
              </button>
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
};

// =============================================================================
// Main App Component
// =============================================================================

function App() {
  // UI State
  const sidebarState = useSidebarState();
  const { add: addToast } = useToastActions();
  const { t } = useTranslation();
  const [showAIAssistant, setShowAIAssistant] = useState(false);

  // Server State (using mock data for now)
  const groups = useGroups();
  const servers = useServers();
  const selection = useSelection();

  // Initialize mock data if empty
  React.useEffect(() => {
    const store = useServerStore.getState();
    if (store.servers.length === 0) {
      MOCK_GROUPS.forEach((g) => store.addGroup(g));
      MOCK_SERVERS.forEach((s) => store.addServer(s));
    }
  }, []);

  // Actions
  const handleAddServer = useCallback(() => {
    addToast({
      type: 'info',
      title: t('server-add-title'),
      message: t('server-add-dialog-message'),
      duration: 3000,
    });
  }, [addToast, t]);

  const handleAddGroup = useCallback(() => {
    addToast({
      type: 'info',
      title: t('group-new-title'),
      message: t('group-add-dialog-message'),
      duration: 3000,
    });
  }, [addToast, t]);

  const handleSettings = useCallback(() => {
    addToast({
      type: 'info',
      title: t('settings-title'),
      message: t('settings-dialog-message'),
      duration: 3000,
    });
  }, [addToast, t]);

  const handleSelect = useCallback(
    (id: string, type: 'server' | 'group') => {
      if (type === 'server') {
        selection.setServer(id);
        addToast({
          type: 'info',
          title: t('server-select-title'),
          message: t('server-selected-message', { id }),
          duration: 2000,
        });
      }
    },
    [selection, addToast, t]
  );

  const handleSearch = useCallback(() => {
    addToast({
      type: 'info',
      title: t('command-palette-title'),
      message: t('command-palette-shortcut'),
      duration: 2000,
    });
  }, [addToast, t]);

  const handleAIAssistant = useCallback(() => {
    setShowAIAssistant(true);
    addToast({
      type: 'info',
      title: 'AI 助手',
      message: 'AI 智能助手已启动',
      duration: 2000,
    });
  }, [addToast]);

  // Quick actions
  const quickActions = useMemo<QuickAction[]>(
    () => [
      {
        id: 'ai-assistant',
        icon: Sparkles,
        label: 'AI 助手',
        onClick: handleAIAssistant,
        shortcut: 'Cmd+I',
      },
      {
        id: 'new-connection',
        icon: Plus,
        label: t('action-new-connection'),
        onClick: handleAddServer,
        shortcut: 'Cmd+N',
      },
      {
        id: 'terminal',
        icon: Terminal,
        label: t('action-terminal'),
        onClick: () => {},
        shortcut: 'Cmd+T',
      },
      {
        id: 'search',
        icon: Search,
        label: t('action-search'),
        onClick: handleSearch,
        shortcut: 'Cmd+F',
      },
      {
        id: 'settings',
        icon: Settings,
        label: t('settings-title'),
        onClick: handleSettings,
        shortcut: 'Cmd+,',
      },
    ],
    [handleAddServer, handleSearch, handleSettings, handleAIAssistant, t]
  );

  // Breadcrumbs
  const breadcrumbs = useMemo(() => {
    const items: Array<{ label: string; icon?: typeof Terminal }> = [{ label: 'EasySSH', icon: Terminal }];
    if (selection.selectedServer) {
      const server = servers.find((s: Server) => s.id === selection.selectedServer);
      if (server) {
        items.push({ label: server.name });
      }
    }
    return items;
  }, [selection.selectedServer, servers]);

  return (
    <>
      <AppShell
        sidebar={
          <Sidebar
            groups={groups}
            servers={servers}
            selectedId={selection.selectedServer}
            onSelect={handleSelect}
            onAddServer={handleAddServer}
            onAddGroup={handleAddGroup}
            onSettings={handleSettings}
            collapsed={sidebarState.collapsed}
          />
        }
        topBar={
          <TopBar
            breadcrumbs={breadcrumbs}
            actions={quickActions}
            onSearch={handleSearch}
            onToggleMenu={sidebarState.toggle}
            showSidebarToggle
            sidebarVisible={!sidebarState.collapsed}
          />
        }
        statusBar={
          <div className="flex items-center justify-between w-full text-apple-xs text-apple-text-secondary">
            <div className="flex items-center gap-apple-4">
              <span>
                {servers.length} {t('servers-label')} · {groups.length} {t('groups-label')}
              </span>
              {selection.selectedServer && (
                <span className="text-apple-accent-blue">
                  {t('selected-label')}: {servers.find((s: Server) => s.id === selection.selectedServer)?.name}
                </span>
              )}
            </div>
            <div className="flex items-center gap-apple-3">
              <span className="flex items-center gap-apple-1">
                <span className="w-2 h-2 rounded-full bg-apple-accent-green" />
                {t('status-ready')}
              </span>
            </div>
          </div>
        }
        sidebarCollapsed={sidebarState.collapsed}
        sidebarWidth={sidebarState.width}
        onSidebarResize={sidebarState.setWidth}
        toasts={<ToastContainer />}
      >
        {/* Main Workspace */}
        <div className="h-full flex flex-col items-center justify-center p-apple-8">
          <AnimatePresence mode="wait">
            {selection.selectedServer ? (
              <motion.div
                key="server-detail"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -20 }}
                className="w-full max-w-2xl"
              >
                <ServerDetail
                  server={servers.find((s: Server) => s.id === selection.selectedServer)!}
                />
              </motion.div>
            ) : (
              <motion.div
                key="empty-state"
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.95 }}
                className="text-center"
              >
                <EmptyState onAddServer={handleAddServer} />
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </AppShell>

      {/* AI Assistant Modal */}
      <AnimatePresence>
        {showAIAssistant && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-[100] bg-black/50 backdrop-blur-sm flex items-center justify-center p-apple-4"
          >
            <motion.div
              initial={{ scale: 0.95, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              className="w-full max-w-5xl h-[85vh]"
            >
              <AIAssistant fullscreen onClose={() => setShowAIAssistant(false)} />
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}

// =============================================================================
// Sub-components
// =============================================================================

function EmptyState({ onAddServer }: { onAddServer: () => void }) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center gap-apple-4">
      <div className="w-16 h-16 rounded-apple-xl bg-apple-accent-blue/10 flex items-center justify-center">
        <Terminal className="w-8 h-8 text-apple-accent-blue" />
      </div>
      <div>
        <h2 className="text-apple-xl font-semibold text-apple-text-primary">
          {t('welcome-title')}
        </h2>
        <p className="mt-apple-2 text-apple-text-secondary max-w-md">
          {t('welcome-message')}
        </p>
      </div>
      <motion.button
        className="btn-primary mt-apple-4"
        onClick={onAddServer}
        whileTap={{ scale: 0.98 }}
      >
        <Plus className="w-4 h-4 mr-apple-2" />
        {t('server-add-title')}
      </motion.button>
    </div>
  );
}

function ServerDetail({ server }: { server: ReturnType<typeof useServers>[0] }) {
  const { t } = useTranslation();
  const { create: createSession } = useSessions();
  const { add: addToast } = useToastActions();

  const handleConnect = () => {
    const sessionId = createSession(server.id);
    addToast({
      type: 'success',
      title: t('toast-connected-title'),
      message: t('toast-connected-message', { sessionId }),
      duration: 3000,
    });
  };

  const getAuthMethodLabel = (method: string) => {
    switch (method) {
      case 'password': return t('auth-password');
      case 'key': return t('auth-key');
      case 'agent': return t('auth-agent');
      default: return method;
    }
  };

  return (
    <div className="bg-apple-bg-secondary rounded-apple-xl border border-apple-border p-apple-6 shadow-apple-md">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-apple-4">
          <div
            className="w-12 h-12 rounded-apple-lg flex items-center justify-center"
            style={{ backgroundColor: `${server.color}20` }}
          >
            <Terminal className="w-6 h-6" style={{ color: server.color }} />
          </div>
          <div>
            <h2 className="text-apple-xl font-semibold text-apple-text-primary">
              {server.name}
            </h2>
            <p className="text-apple-sm text-apple-text-secondary mt-apple-1">
              {server.username}@{server.host}:{server.port}
            </p>
          </div>
        </div>
        <motion.button
          className="btn-primary"
          onClick={handleConnect}
          whileTap={{ scale: 0.98 }}
        >
          <Terminal className="w-4 h-4 mr-apple-2" />
          {t('server-detail-connect')}
        </motion.button>
      </div>

      <div className="mt-apple-6 grid grid-cols-3 gap-apple-4">
        <div className="p-apple-3 bg-apple-bg-tertiary rounded-apple-md">
          <p className="text-apple-xs text-apple-text-tertiary uppercase tracking-wide">
            {t('auth-method-label')}
          </p>
          <p className="mt-apple-1 text-apple-sm font-medium text-apple-text-primary capitalize">
            {getAuthMethodLabel(server.authMethod)}
          </p>
        </div>
        <div className="p-apple-3 bg-apple-bg-tertiary rounded-apple-md">
          <p className="text-apple-xs text-apple-text-tertiary uppercase tracking-wide">
            {t('last-connected-label')}
          </p>
          <p className="mt-apple-1 text-apple-sm font-medium text-apple-text-primary">
            {server.lastConnectedAt
              ? new Date(server.lastConnectedAt).toLocaleDateString()
              : t('never-connected')}
          </p>
        </div>
        <div className="p-apple-3 bg-apple-bg-tertiary rounded-apple-md">
          <p className="text-apple-xs text-apple-text-tertiary uppercase tracking-wide">
            {t('tags-label')}
          </p>
          <div className="mt-apple-1 flex flex-wrap gap-apple-1">
            {server.tags.map((tag: string) => (
              <span
                key={tag}
                className="px-apple-2 py-apple-0.5 rounded-apple-sm bg-apple-accent-blue/10 text-apple-accent-blue text-apple-xs"
              >
                {tag}
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
