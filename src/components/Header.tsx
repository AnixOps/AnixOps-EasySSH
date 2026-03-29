import { useUiStore } from '../stores/uiStore';
import { getProductModeMeta, PRODUCT_MODES } from '../productModes';
import { SectionLabel } from './design-system';

export function Header() {
  const { productMode, setProductMode, theme, toggleTheme } = useUiStore();

  const mode = getProductModeMeta(productMode);

  return (
    <header className="flex h-16 items-center justify-between border-b border-slate-800/80 bg-slate-950/95 px-4 backdrop-blur">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-cyan-500/15 text-cyan-300 ring-1 ring-cyan-500/30">
            E
          </div>
          <div className="min-w-0">
            <p className="text-sm font-semibold text-slate-50">EasySSH</p>
            <p className="text-xs text-slate-500">Termius-style workspace rewrite</p>
          </div>
        </div>

        <div className="hidden lg:block">
          <SectionLabel>Current Mode</SectionLabel>
          <p className="text-sm font-medium text-slate-200">{mode.title}</p>
          <p className="text-xs text-slate-500">{mode.description}</p>
        </div>
      </div>

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

      <div className="flex items-center gap-2">
        <button
          type="button"
          className="rounded-xl border border-slate-800 bg-slate-900 px-3 py-2 text-xs font-medium text-slate-300 transition hover:border-slate-700 hover:bg-slate-800 hover:text-white"
          title="Command palette"
        >
          Ctrl+K
        </button>
        <button
          type="button"
          onClick={toggleTheme}
          className="rounded-xl border border-slate-800 bg-slate-900 px-3 py-2 text-xs font-medium text-slate-300 transition hover:border-slate-700 hover:bg-slate-800 hover:text-white"
          title="Toggle theme"
        >
          {theme === 'dark' ? '☀️' : '🌙'}
        </button>
      </div>
    </header>
  );
}
