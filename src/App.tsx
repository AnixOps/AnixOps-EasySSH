import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Header } from './components/Header';
import { MainContent } from './components/MainContent';
import { Sidebar } from './components/Sidebar';
import { SectionLabel, Surface } from './components/design-system';
import { getProductModeMeta, type ProductMode } from './productModes';
import { useServerStore } from './stores/serverStore';
import { useSessionStore } from './stores/sessionStore';
import { useUiStore } from './stores/uiStore';

function App() {
  const { productMode } = useUiStore();
  const { fetchServers, fetchGroups, isLoading, error, servers, groups } = useServerStore();
  const { terminalSessions } = useSessionStore();

  // Initialize database on app start
  useEffect(() => {
    const init = async () => {
      try {
        await invoke('init_database');
        await Promise.all([fetchServers(), fetchGroups()]);
      } catch (e) {
        console.error('Failed to initialize database:', e);
      }
    };
    void init();
  }, [fetchServers, fetchGroups]);

  const mode = getProductModeMeta(productMode);

  if (isLoading && servers.length === 0 && groups.length === 0) {
    return (
      <div className="flex h-full min-h-screen items-center justify-center bg-slate-950 text-slate-100">
        <div className="text-center">
          <div className="mb-4 text-2xl font-semibold">EasySSH</div>
          <p className="text-sm text-slate-400">Loading workspace...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full min-h-screen items-center justify-center bg-slate-950 text-red-300">
        <div className="max-w-md rounded-2xl border border-red-900/50 bg-red-950/40 p-6">
          <h2 className="text-xl font-semibold">Workspace failed to load</h2>
          <p className="mt-2 text-sm text-red-200/80">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-screen flex-col overflow-hidden bg-slate-950 text-slate-100">
      <Header />

      <div className="flex min-h-0 flex-1 overflow-hidden">
        <Sidebar />

        <MainContent />

        <aside className="hidden w-80 flex-col border-l border-slate-800/80 bg-slate-950/90 xl:flex">
          <div className="border-b border-slate-800 px-4 py-3">
            <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Details</p>
            <h2 className="text-sm font-semibold text-slate-50">{mode.subtitle}</h2>
          </div>
          <div className="flex-1 overflow-auto p-4">
            <RightPanel productMode={productMode} serverCount={servers.length} groupCount={groups.length} sessionCount={terminalSessions.length} />
          </div>
        </aside>
      </div>
    </div>
  );
}

function RightPanel({
  productMode,
  serverCount,
  groupCount,
  sessionCount,
}: {
  productMode: ProductMode;
  serverCount: number;
  groupCount: number;
  sessionCount: number;
}) {
  const mode = getProductModeMeta(productMode);

  return (
    <div className="space-y-4">
      <Surface className="p-4">
        <SectionLabel>Product mode</SectionLabel>
        <h3 className="mt-2 text-base font-semibold text-slate-50">{mode.title}</h3>
        <p className="mt-2 text-sm leading-6 text-slate-400">{mode.description}</p>
      </Surface>

      <Surface className="p-4">
        <SectionLabel>Activity</SectionLabel>
        <div className="mt-4 space-y-3 text-sm text-slate-300">
          <div className="flex items-center justify-between gap-4 border-b border-slate-800/70 pb-2 last:border-b-0 last:pb-0">
            <span className="text-slate-500">Servers</span>
            <span className="font-medium text-slate-200">{serverCount}</span>
          </div>
          <div className="flex items-center justify-between gap-4 border-b border-slate-800/70 pb-2 last:border-b-0 last:pb-0">
            <span className="text-slate-500">Groups</span>
            <span className="font-medium text-slate-200">{groupCount}</span>
          </div>
          <div className="flex items-center justify-between gap-4 border-b border-slate-800/70 pb-2 last:border-b-0 last:pb-0">
            <span className="text-slate-500">Sessions</span>
            <span className="font-medium text-slate-200">{sessionCount}</span>
          </div>
        </div>
      </Surface>

      <div className="rounded-2xl border border-dashed border-slate-700 bg-slate-950 p-4 text-sm text-slate-500">
        This panel will eventually host selected server details, connection actions, and contextual tools.
      </div>
    </div>
  );
}

export default App;
