import { useUiStore } from '../../stores/uiStore';
import { getProductModeMeta } from '../../productModes';
import { SectionLabel } from '../design-system';

export function ModeIndicator() {
  const { productMode } = useUiStore();
  const mode = getProductModeMeta(productMode);

  return (
    <div className="hidden lg:block">
      <SectionLabel>Current Mode</SectionLabel>
      <p className="text-sm font-medium text-slate-200">{mode.title}</p>
      <p className="text-xs text-slate-500">{mode.description}</p>
    </div>
  );
}
