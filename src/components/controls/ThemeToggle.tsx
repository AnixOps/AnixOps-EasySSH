import { useUiStore } from '../../stores/uiStore';

export function ThemeToggle() {
  const { theme, toggleTheme } = useUiStore();

  return (
    <button
      type="button"
      onClick={toggleTheme}
      className="rounded-xl border border-slate-800 bg-slate-900 px-3 py-2 text-xs font-medium text-slate-300 transition hover:border-slate-700 hover:bg-slate-800 hover:text-white"
      title="Toggle theme"
    >
      {theme === 'dark' ? '☀️' : '🌙'}
    </button>
  );
}
