export function ActionCard({
  title,
  body,
  tone = 'slate',
}: {
  title: string;
  body: string;
  tone?: 'cyan' | 'slate' | 'violet';
}) {
  const toneClasses = {
    cyan: 'border-cyan-500/20 bg-cyan-500/5',
    slate: 'border-slate-800 bg-slate-950/50',
    violet: 'border-violet-500/20 bg-violet-500/5',
  };

  return (
    <div className={`rounded-2xl border p-4 ${toneClasses[tone]}`}>
      <h3 className="text-sm font-semibold text-slate-50">{title}</h3>
      <p className="mt-2 text-sm leading-6 text-slate-400">{body}</p>
    </div>
  );
}
