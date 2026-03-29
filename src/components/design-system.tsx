import type { ReactNode } from 'react';

export function Surface({ children, className = '' }: { children: ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl border border-slate-800 bg-slate-900/70 shadow-lg shadow-black/20 ${className}`}>
      {children}
    </div>
  );
}

export function SectionLabel({ children }: { children: ReactNode }) {
  return <p className="text-xs uppercase tracking-[0.2em] text-slate-500">{children}</p>;
}

export function StatCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: number | string;
  hint: string;
}) {
  return (
    <div className="rounded-2xl border border-slate-800 bg-slate-950/60 p-4">
      <p className="text-xs uppercase tracking-[0.2em] text-slate-500">{label}</p>
      <div className="mt-2 text-2xl font-semibold text-slate-50">{value}</div>
      <p className="mt-1 text-xs text-slate-500">{hint}</p>
    </div>
  );
}

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

export function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-slate-800/70 pb-2 last:border-b-0 last:pb-0">
      <span className="text-slate-500">{label}</span>
      <span className="font-medium text-slate-200">{value}</span>
    </div>
  );
}

// Design token for status colors
const STATUS_COLORS = {
  online: 'bg-green-500',
  offline: 'bg-red-500',
  warning: 'bg-yellow-500',
  unknown: 'bg-slate-500',
} as const;

export type StatusDotVariant = keyof typeof STATUS_COLORS;

export function StatusDot({ variant = 'unknown' }: { variant?: StatusDotVariant }) {
  return <span className={`inline-block w-2 h-2 rounded-full ${STATUS_COLORS[variant]}`} />;
}

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

export function RadioButton({
  label,
  description,
  checked,
  onChange,
  value,
  name,
  disabled = false,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange?: (checked: boolean) => void;
  value?: string;
  name?: string;
  disabled?: boolean;
}) {
  return (
    <label className={`flex items-center gap-3 p-3 rounded-xl cursor-pointer transition border border-slate-700 ${
      disabled
        ? 'opacity-50 cursor-not-allowed bg-slate-800/50'
        : 'hover:bg-slate-700 bg-slate-800'
    }`}>
      <input
        type="radio"
        name={name}
        checked={checked}
        onChange={(e) => !disabled && onChange?.(e.target.checked)}
        value={value}
        disabled={disabled}
        aria-label={label}
        className="w-4 h-4 text-cyan-500 bg-slate-900 border-slate-700 focus:ring-cyan-500/50 focus:ring-offset-0"
      />
      <div>
        <span className="text-slate-100 font-medium">{label}</span>
        {description && <p className="text-xs text-slate-500">{description}</p>}
      </div>
    </label>
  );
}

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
export type ButtonSize = 'sm' | 'md' | 'lg';

export function Button({
  children,
  variant = 'secondary',
  size = 'md',
  className = '',
  disabled = false,
  ...props
}: {
  children: React.ReactNode;
  variant?: ButtonVariant;
  size?: ButtonSize;
  className?: string;
  disabled?: boolean;
} & React.ButtonHTMLAttributes<HTMLButtonElement>) {
  const baseClasses = 'rounded-xl font-medium transition focus:outline-none focus:ring-2 focus:ring-cyan-500/50 disabled:opacity-50 disabled:cursor-not-allowed';

  const variantClasses: Record<ButtonVariant, string> = {
    primary: 'bg-cyan-500 text-slate-950 hover:bg-cyan-400',
    secondary: 'border border-slate-800 bg-slate-900 text-slate-300 hover:border-slate-700 hover:bg-slate-800 hover:text-white',
    ghost: 'text-slate-400 hover:bg-slate-800 hover:text-white',
    danger: 'bg-red-600 text-white hover:bg-red-500',
  };

  const sizeClasses: Record<ButtonSize, string> = {
    sm: 'px-3 py-1.5 text-xs',
    md: 'px-4 py-2 text-sm',
    lg: 'px-5 py-2.5 text-base',
  };

  return (
    <button
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      disabled={disabled}
      {...props}
    >
      {children}
    </button>
  );
}

export type InputSize = 'sm' | 'md' | 'lg';

export function Input({
  inputSize = 'md',
  className = '',
  ...props
}: {
  inputSize?: InputSize;
  className?: string;
} & Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'>) {
  const sizeClasses: Record<InputSize, string> = {
    sm: 'px-3 py-1.5 text-xs',
    md: 'px-3 py-2 text-sm',
    lg: 'px-4 py-2.5 text-base',
  };

  return (
    <input
      className={`w-full rounded-xl border border-slate-800 bg-slate-900 text-slate-100 placeholder:text-slate-500 focus:border-cyan-500 focus:outline-none transition ${sizeClasses[inputSize]} ${className}`}
      {...props}
    />
  );
}

export function Modal({
  children,
  title,
  onClose,
}: {
  children: React.ReactNode;
  title: string;
  onClose: () => void;
}) {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={onClose}>
      <div
        className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-md p-6 shadow-xl shadow-black/30"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-slate-50">{title}</h2>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-slate-100 transition text-sm"
          >
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
