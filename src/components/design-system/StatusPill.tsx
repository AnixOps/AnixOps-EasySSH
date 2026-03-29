export function StatusPill({ label, active = false }: { label: string; active?: boolean }) {
  return (
    <span
      className={`rounded-full border px-3 py-1 text-xs font-medium transition ${
        active
          ? 'border-cyan-500/30 bg-cyan-500 text-slate-950'
          : 'border-slate-800 bg-slate-900 text-slate-400'
      }`}
    >
      {label}
    </span>
  );
}
