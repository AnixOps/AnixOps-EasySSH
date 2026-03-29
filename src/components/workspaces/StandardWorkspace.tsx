import { ActionCard, InfoRow, SectionLabel, Surface } from '../design-system';
import { SplitScreen } from '../SplitScreen';

interface StandardWorkspaceProps {
  serverCount: number;
  groupCount: number;
  sessionCount: number;
}

export function StandardWorkspace({ serverCount, groupCount, sessionCount }: StandardWorkspaceProps) {
  return (
    <div className="grid h-full min-h-0 gap-4 lg:grid-cols-3">
      <Surface className="h-full min-h-0 overflow-hidden lg:col-span-2">
        <SplitScreen />
      </Surface>

      <div className="grid gap-4">
        <Surface className="p-4">
          <SectionLabel>Activity</SectionLabel>
          <div className="mt-4 space-y-3 text-sm text-slate-300">
            <InfoRow label="Servers" value={String(serverCount)} />
            <InfoRow label="Groups" value={String(groupCount)} />
            <InfoRow label="Active Sessions" value={String(sessionCount)} />
          </div>
        </Surface>

        <ActionCard title="Split View" body="Use keyboard shortcuts to split terminals." tone="cyan" />
        <ActionCard title="SFTP" body="Drag and drop file transfers coming soon." tone="slate" />
      </div>
    </div>
  );
}
