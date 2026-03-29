import type { ReactNode } from 'react';

export function Surface({ children, className = '' }: { children: ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl border border-slate-800 bg-slate-900/70 shadow-lg shadow-black/20 ${className}`}>
      {children}
    </div>
  );
}
