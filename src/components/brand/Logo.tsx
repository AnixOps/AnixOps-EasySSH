export function Logo() {
  return (
    <div className="flex items-center gap-3">
      <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-cyan-500/15 text-cyan-300 ring-1 ring-cyan-500/30">
        E
      </div>
      <div className="min-w-0">
        <p className="text-sm font-semibold text-slate-50">EasySSH</p>
        <p className="text-xs text-slate-500">Termius-style workspace rewrite</p>
      </div>
    </div>
  );
}
