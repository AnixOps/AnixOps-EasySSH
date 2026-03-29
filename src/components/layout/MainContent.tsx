import { useServerStore } from '../../stores/serverStore';
import { useSessionStore } from '../../stores/sessionStore';
import { useUiStore } from '../../stores/uiStore';
import { getProductModeMeta } from '../../productModes';
import { SectionLabel, StatusPill } from '../design-system';
import { LiteWorkspace, ProWorkspace, StandardWorkspace } from '../workspaces';

export function MainContent() {
  const { productMode } = useUiStore();
  const { servers, groups } = useServerStore();
  const { terminalSessions } = useSessionStore();

  const mode = getProductModeMeta(productMode);

  return (
    <main className="flex min-w-0 flex-1 flex-col border-l border-slate-800/80 bg-slate-950">
      <section className="flex items-center justify-between border-b border-slate-800/80 px-4 py-3">
        <div>
          <SectionLabel>Workspace</SectionLabel>
          <h1 className="text-lg font-semibold text-slate-50">{mode.title}</h1>
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-400">
          <StatusPill label={`${servers.length} servers`} active />
          <StatusPill label={`${groups.length} groups`} />
          <StatusPill label={`${terminalSessions.length} sessions`} />
        </div>
      </section>

      <div className="min-h-0 flex-1 overflow-hidden p-4">
        {productMode === 'lite' && (
          <LiteWorkspace servers={servers.length} groups={groups.length} />
        )}

        {productMode === 'standard' && (
          <StandardWorkspace
            serverCount={servers.length}
            groupCount={groups.length}
            sessionCount={terminalSessions.length}
          />
        )}

        {productMode === 'pro' && (
          <ProWorkspace
            serverCount={servers.length}
            groupCount={groups.length}
            sessionCount={terminalSessions.length}
          />
        )}
      </div>
    </main>
  );
}
