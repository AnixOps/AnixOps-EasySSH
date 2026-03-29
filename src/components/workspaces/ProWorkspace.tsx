import { ActionCard, InfoRow, SectionLabel, StatCard, Surface } from '../design-system';

interface ProWorkspaceProps {
  serverCount: number;
  groupCount: number;
  sessionCount: number;
}

export function ProWorkspace({ serverCount, groupCount, sessionCount }: ProWorkspaceProps) {
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
