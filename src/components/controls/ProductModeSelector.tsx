import { useUiStore } from '../../stores/uiStore';
import { PRODUCT_MODES } from '../../productModes';

export function ProductModeSelector() {
  const { productMode, setProductMode } = useUiStore();

  return (
    <div className="flex items-center gap-2 rounded-full border border-slate-800 bg-slate-900/80 p-1 shadow-inner shadow-black/20">
      {PRODUCT_MODES.map((meta) => {
        const active = productMode === meta.id;
        return (
          <button
            key={meta.id}
            onClick={() => setProductMode(meta.id)}
            className={`rounded-full px-3 py-1.5 text-xs font-medium transition ${
              active
                ? 'bg-cyan-500 text-slate-950'
                : 'text-slate-400 hover:bg-slate-800 hover:text-slate-100'
            }`}
          >
            {meta.subtitle}
          </button>
        );
      })}
    </div>
  );
}
