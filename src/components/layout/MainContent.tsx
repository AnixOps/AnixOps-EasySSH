import { useServerStore } from '../../stores/serverStore';
import { useSessionStore } from '../../stores/sessionStore';
import { useUiStore } from '../../stores/uiStore';
import { getProductModeMeta } from '../../productModes';
import { ActionCard, InfoRow, SectionLabel, StatCard, Surface, StatusPill } from '../design-system';
import { SplitScreen } from '../SplitScreen';

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
          <Surface className="h-full min-h-0 overflow-hidden">
            <SplitScreen />
          </Surface>
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

function LiteWorkspace({ servers, groups }: { servers: number; groups: number }) {
  return (
    <div className="grid h-full min-h-0 gap-4 lg:grid-cols-2">
      <Surface className="p-5">
        <SectionLabel>Lite surface</SectionLabel>
        <h2 className="mt-2 text-xl font-semibold text-slate-50">SSH vault and quick launch</h2>
        <p className="mt-3 text-sm leading-6 text-slate-400">
          Lite is now a focused SSH vault. It should feel like a secure launcher instead of a half-finished terminal app.
        </p>
        <div className="mt-5 grid gap-3 sm:grid-cols-2">
          <StatCard label="Servers" value={servers} hint="Stored locally" />
          <StatCard label="Groups" value={groups} hint="Flat and simple" />
        </div>
      </Surface>

      <div className="grid gap-4">
        <ActionCard title="Native terminal launch" body="Open the system terminal with the selected SSH profile." tone="cyan" />
        <ActionCard title="Keychain first" body="Prefer system keychain and encrypted local storage." tone="slate" />
        <ActionCard title="Fast search" body="Find servers by name, host, or username without extra clutter." tone="slate" />
      </div>
    </div>
  );
}

function ProWorkspace({ serverCount, groupCount, sessionCount }: { serverCount: number; groupCount: number; sessionCount: number }) {
  return (
    <div className="grid h-full min-h-0 gap-4 xl:grid-cols-3">
      <Surface className="p-5 xl:col-span-2">
        <SectionLabel>Pro surface</SectionLabel>
        <h2 className="mt-2 text-xl font-semibold text-slate-50">Team control console</h2>
        <p className="mt-3 text-sm leading-6 text-slate-400">
          Pro should be a governance surface, not just more buttons. The focus is team access, auditability, and shared resources.
        </p>

        <div className="mt-5 grid gap-3 sm:grid-cols-3">
          <StatCard label="Servers" value={serverCount} hint="Shared assets" />
          <StatCard label="Groups" value={groupCount} hint="Team structure" />
          <StatCard label="Sessions" value={sessionCount} hint="Auditable work" />
        </div>

        <div className="mt-5 grid gap-3 md:grid-cols-3">
          <ActionCard title="RBAC" body="Define who can connect, manage, or audit." tone="violet" />
          <ActionCard title="SSO" body="Plug into enterprise identity providers." tone="violet" />
          <ActionCard title="Audit" body="Track session and command activity centrally." tone="violet" />
        </div>
      </Surface>

      <Surface className="p-5">
        <SectionLabel>Governance</SectionLabel>
        <div className="mt-4 space-y-3 text-sm text-slate-400">
          <InfoRow label="Team space" value="Enabled" />
          <InfoRow label="Audit log" value="Ready" />
          <InfoRow label="SSO" value="Planned" />
          <InfoRow label="Shared snippets" value="Planned" />
        </div>
      </Surface>
    </div>
  );
}
