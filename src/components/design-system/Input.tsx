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
