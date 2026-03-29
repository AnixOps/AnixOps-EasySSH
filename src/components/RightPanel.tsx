import { useUiStore } from '../stores/uiStore';
import { useServerStore } from '../stores/serverStore';
import { useSessionStore } from '../stores/sessionStore';
import { getProductModeMeta } from '../productModes';
import { SectionLabel, Surface } from './design-system';

export function RightPanel() {
  const { productMode } = useUiStore();
  const { servers, groups } = useServerStore();
  const { terminalSessions } = useSessionStore();

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
            <span className="font-medium text-slate-200">{servers.length}</span>
          </div>
          <div className="flex items-center justify-between gap-4 border-b border-slate-800/70 pb-2 last:border-b-0 last:pb-0">
            <span className="text-slate-500">Groups</span>
            <span className="font-medium text-slate-200">{groups.length}</span>
          </div>
          <div className="flex items-center justify-between gap-4 border-b border-slate-800/70 pb-2 last:border-b-0 last:pb-0">
            <span className="text-slate-500">Sessions</span>
            <span className="font-medium text-slate-200">{terminalSessions.length}</span>
          </div>
        </div>
      </Surface>

      <div className="rounded-2xl border border-dashed border-slate-700 bg-slate-950 p-4 text-sm text-slate-500">
        This panel will eventually host selected server details, connection actions, and contextual tools.
      </div>
    </div>
  );
}
