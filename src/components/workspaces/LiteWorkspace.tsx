import { ActionCard, SectionLabel, StatCard, Surface } from '../design-system';

interface LiteWorkspaceProps {
  servers: number;
  groups: number;
}

export function LiteWorkspace({ servers, groups }: LiteWorkspaceProps) {
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
