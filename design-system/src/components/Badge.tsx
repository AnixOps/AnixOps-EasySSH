import React, { forwardRef } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../utils';

/**
 * Badge Component
 *
 * Small status indicator for UI elements.
 */

const badgeVariants = cva(
  'inline-flex items-center gap-1 font-medium rounded-full whitespace-nowrap transition-colors',
  {
    variants: {
      variant: {
        default: 'bg-[var(--easyssh-bg-tertiary)] text-[var(--easyssh-text-secondary)]',
        primary: 'bg-[var(--easyssh-interactive-primary)] text-white',
        success: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
        warning: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
        danger: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
        info: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
        outline: 'border border-[var(--easyssh-border-default)] text-[var(--easyssh-text-secondary)]',
        ghost: 'bg-transparent text-[var(--easyssh-text-secondary)]',
      },
      size: {
        sm: 'px-2 py-0.5 text-xs',
        md: 'px-2.5 py-1 text-sm',
        lg: 'px-3 py-1.5 text-base',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'sm',
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {
  /** Badge text */
  children: React.ReactNode;
  /** Show dot indicator */
  dot?: boolean;
  /** Dot color (when dot is true) */
  dotColor?: string;
  /** Remove button */
  onRemove?: () => void;
}

const Badge = forwardRef<HTMLSpanElement, BadgeProps>(
  (
    { children, className, variant, size, dot, dotColor, onRemove, ...props },
    ref
  ) => {
    return (
      <span
        ref={ref}
        className={cn(badgeVariants({ variant, size }), className)}
        {...props}
      >
        {dot && (
          <span
            className={cn('w-1.5 h-1.5 rounded-full', dotColor || 'bg-current')}
          />
        )}
        {children}
        {onRemove && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onRemove();
            }}
            className="ml-1 p-0.5 rounded-full hover:bg-black/10 dark:hover:bg-white/10 transition-colors"
            aria-label="Remove"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        )}
      </span>
    );
  }
);

Badge.displayName = 'Badge';

/**
 * Status Badge - Specialized badge for status indicators
 */
export interface StatusBadgeProps extends Omit<BadgeProps, 'variant' | 'dot'> {
  status: 'online' | 'offline' | 'connecting' | 'warning' | 'error' | 'info';
}

export const StatusBadge: React.FC<StatusBadgeProps> = ({ status, ...props }) => {
  const statusConfig = {
    online: { variant: 'success' as const, label: 'Online', dotColor: 'bg-green-500' },
    offline: { variant: 'default' as const, label: 'Offline', dotColor: 'bg-gray-400' },
    connecting: { variant: 'warning' as const, label: 'Connecting', dotColor: 'bg-yellow-500 animate-pulse' },
    warning: { variant: 'warning' as const, label: 'Warning', dotColor: 'bg-yellow-500' },
    error: { variant: 'danger' as const, label: 'Error', dotColor: 'bg-red-500' },
    info: { variant: 'info' as const, label: 'Info', dotColor: 'bg-blue-500' },
  };

  const config = statusConfig[status];

  return (
    <Badge
      variant={config.variant}
      dot
      dotColor={config.dotColor}
      {...props}
    >
      {props.children || config.label}
    </Badge>
  );
};

/**
 * Count Badge - Numeric badge (often used on icons/buttons)
 */
export interface CountBadgeProps {
  count: number;
  max?: number;
  className?: string;
}

export const CountBadge: React.FC<CountBadgeProps> = ({
  count,
  max = 99,
  className,
}) => {
  if (count <= 0) return null;

  const display = count > max ? `${max}+` : count;

  return (
    <span
      className={cn(
        'inline-flex items-center justify-center min-w-[18px] h-[18px] px-1.5',
        'text-xs font-bold text-white bg-red-500 rounded-full',
        className
      )}
    >
      {display}
    </span>
  );
};

export { Badge, badgeVariants };
export default Badge;
