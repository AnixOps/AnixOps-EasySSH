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
