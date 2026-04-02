import React, { forwardRef } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../utils';
import { Icon, type IconName } from '../icons';

/**
 * Card Component
 *
 * Flexible card container with support for headers, actions, and various elevations.
 */

const cardVariants = cva(
  'bg-[var(--easyssh-bg-elevated)] rounded-lg overflow-hidden transition-shadow duration-200',
  {
    variants: {
      elevation: {
        0: 'shadow-none',
        1: 'shadow-sm',
        2: 'shadow-md',
        3: 'shadow-lg',
        4: 'shadow-xl',
        5: 'shadow-2xl',
      },
      interactive: {
        true: 'cursor-pointer hover:shadow-md transition-all duration-200 active:scale-[0.99]',
        false: '',
      },
    },
    defaultVariants: {
      elevation: 1,
      interactive: false,
    },
  }
);

const cardPaddingSizes = {
  none: '',
  sm: 'p-3',
  md: 'p-4',
  lg: 'p-6',
  xl: 'p-8',
};

export interface CardProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof cardVariants> {
  /** Card content */
  children: React.ReactNode;
  /** Card title */
  title?: string;
  /** Card subtitle */
  subtitle?: string;
  /** Header icon */
  icon?: IconName;
  /** Header actions */
  actions?: React.ReactNode;
  /** Padding size */
  padding?: 'none' | 'sm' | 'md' | 'lg' | 'xl';
  /** Footer content */
  footer?: React.ReactNode;
  /** Click handler for interactive cards */
  onClick?: () => void;
  /** Disabled state */
  disabled?: boolean;
}

const Card = forwardRef<HTMLDivElement, CardProps>(
  (
    {
      children,
      className,
      elevation,
      interactive,
      title,
      subtitle,
      icon,
      actions,
      padding = 'md',
      footer,
      onClick,
      disabled = false,
      ...props
    },
    ref
  ) => {
    const isInteractive = interactive || !!onClick;

    return (
      <div
        ref={ref}
        className={cn(
          cardVariants({ elevation, interactive: isInteractive }),
          disabled && 'opacity-60 cursor-not-allowed',
          className
        )}
        onClick={disabled ? undefined : onClick}
        role={isInteractive ? 'button' : undefined}
        tabIndex={isInteractive ? 0 : undefined}
        {...props}
      >
        {/* Header */}
        {(title || icon || actions) && (
          <div
            className={cn(
              'flex items-start justify-between gap-4',
              padding !== 'none' && cardPaddingSizes[padding],
              'pb-0'
            )}
          >
            <div className="flex items-center gap-3 min-w-0 flex-1">
              {icon && (
                <div className="flex-shrink-0 p-2 rounded-md bg-[var(--easyssh-bg-tertiary)]">
                  <Icon name={icon} size={20} />
                </div>
              )}
              {(title || subtitle) && (
                <div className="min-w-0 flex-1">
                  {title && (
                    <h3 className="font-semibold text-[var(--easyssh-text-primary)] truncate">
                      {title}
                    </h3>
                  )}
                  {subtitle && (
                    <p className="text-sm text-[var(--easyssh-text-secondary)] truncate">
                      {subtitle}
                    </p>
                  )}
                </div>
              )}
            </div>
            {actions && (
              <div className="flex-shrink-0 flex items-center gap-1">
                {actions}
              </div>
            )}
          </div>
        )}

        {/* Content */}
        <div className={cn(padding !== 'none' && cardPaddingSizes[padding])}>
          {children}
        </div>

        {/* Footer */}
        {footer && (
          <div
            className={cn(
              'border-t border-[var(--easyssh-border-subtle)]',
              padding !== 'none' && cardPaddingSizes[padding]
            )}
          >
            {footer}
          </div>
        )}
      </div>
    );
  }
);

Card.displayName = 'Card';

/**
 * Card Grid - Layout multiple cards
 */
export interface CardGridProps {
  children: React.ReactNode;
  /** Columns at different breakpoints */
  columns?: {
    xs?: number;
    sm?: number;
    md?: number;
    lg?: number;
    xl?: number;
  };
  /** Gap between cards */
  gap?: 'sm' | 'md' | 'lg';
  className?: string;
}

export const CardGrid: React.FC<CardGridProps> = ({
  children,
  columns = { xs: 1, sm: 2, md: 2, lg: 3, xl: 4 },
  gap = 'md',
  className,
}) => {
  const gapClasses = {
    sm: 'gap-3',
    md: 'gap-4',
    lg: 'gap-6',
  };

  const colClasses = [
    columns.xs && `grid-cols-${columns.xs}`,
    columns.sm && `sm:grid-cols-${columns.sm}`,
    columns.md && `md:grid-cols-${columns.md}`,
    columns.lg && `lg:grid-cols-${columns.lg}`,
    columns.xl && `xl:grid-cols-${columns.xl}`,
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div className={cn('grid', gapClasses[gap], colClasses, className)}>
      {children}
    </div>
  );
};

/**
 * Stat Card - Card displaying a metric/statistic
 */
export interface StatCardProps extends Omit<CardProps, 'children'> {
  /** Stat label */
  label: string;
  /** Stat value */
  value: string | number;
  /** Previous value for comparison */
  previousValue?: string | number;
  /** Change indicator (positive/negative/neutral) */
  change?: 'positive' | 'negative' | 'neutral';
  /** Change percentage */
  changePercent?: number;
  /** Icon to display */
  icon: IconName;
  /** Icon background color */
  iconColor?: 'primary' | 'success' | 'warning' | 'danger' | 'info';
}

export const StatCard: React.FC<StatCardProps> = ({
  label,
  value,
  previousValue,
  change,
  changePercent,
  icon,
  iconColor = 'primary',
  ...props
}) => {
  const iconColorClasses = {
    primary: 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400',
    success: 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400',
    warning: 'bg-yellow-100 text-yellow-600 dark:bg-yellow-900/30 dark:text-yellow-400',
    danger: 'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400',
    info: 'bg-cyan-100 text-cyan-600 dark:bg-cyan-900/30 dark:text-cyan-400',
  };

  const changeIcons = {
    positive: 'trending-up',
    negative: 'trending-down',
    neutral: 'minus',
  };

  const changeColors = {
    positive: 'text-green-600',
    negative: 'text-red-600',
    neutral: 'text-gray-500',
  };

  return (
    <Card {...props} padding="lg">
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm font-medium text-[var(--easyssh-text-secondary)]">
            {label}
          </p>
          <p className="mt-1 text-2xl font-bold text-[var(--easyssh-text-primary)]">
            {value}
          </p>
          {change && changePercent !== undefined && (
            <div className={cn('mt-2 flex items-center gap-1 text-sm', changeColors[change])}>
              <Icon name={changeIcons[change] as IconName} size={16} />
              <span>{changePercent > 0 ? '+' : ''}{changePercent}%</span>
              {previousValue && (
                <span className="text-[var(--easyssh-text-tertiary)] ml-1">
                  from {previousValue}
                </span>
              )}
            </div>
          )}
        </div>
        <div className={cn('p-3 rounded-lg', iconColorClasses[iconColor])}>
          <Icon name={icon} size={24} />
        </div>
      </div>
    </Card>
  );
};

/**
 * Server Card - Specialized card for server display
 */
export interface ServerCardProps extends Omit<CardProps, 'children' | 'title' | 'subtitle'> {
  /** Server name */
  name: string;
  /** Server address/host */
  host: string;
  /** Server status */
  status: 'online' | 'offline' | 'connecting' | 'maintenance' | 'unknown';
  /** Connection type */
  type?: 'ssh' | 'sftp' | 'docker' | 'kubernetes';
  /** OS type */
  os?: 'linux' | 'macos' | 'windows';
  /** Last connection time */
  lastConnected?: string;
  /** Tags/labels */
  tags?: string[];
  /** Favorite status */
  isFavorite?: boolean;
  /** Quick connect action */
  onConnect?: () => void;
  /** Toggle favorite action */
  onToggleFavorite?: () => void;
}

export const ServerCard: React.FC<ServerCardProps> = ({
  name,
  host,
  status,
  type = 'ssh',
  os,
  lastConnected,
  tags = [],
  isFavorite = false,
  onConnect,
  onToggleFavorite,
  ...props
}) => {
  const statusColors = {
    online: 'bg-green-500',
    offline: 'bg-red-500',
    connecting: 'bg-yellow-500 animate-pulse',
    maintenance: 'bg-purple-500',
    unknown: 'bg-gray-400',
  };

  const statusLabels = {
    online: 'Online',
    offline: 'Offline',
    connecting: 'Connecting...',
    maintenance: 'Maintenance',
    unknown: 'Unknown',
  };

  const typeIcons: Record<string, IconName> = {
    ssh: 'terminal',
    sftp: 'folder',
    docker: 'container',
    kubernetes: 'grid',
  };

  const osIcons: Record<string, IconName> = {
    linux: 'monitor',
    macos: 'monitor',
    windows: 'monitor',
  };

  return (
    <Card
      {...props}
      title={name}
      subtitle={host}
      icon={typeIcons[type]}
      padding="md"
      interactive={!!onConnect}
      onClick={onConnect}
      actions={
        <>
          {onToggleFavorite && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onToggleFavorite();
              }}
              className={cn(
                'p-1.5 rounded-full transition-colors',
                isFavorite
                  ? 'text-yellow-500 hover:bg-yellow-100'
                  : 'text-[var(--easyssh-text-tertiary)] hover:bg-[var(--easyssh-bg-tertiary)]'
              )}
              aria-label={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
            >
              <Icon
                name={isFavorite ? 'star' : 'star'}
                size={18}
                className={cn(isFavorite && 'fill-current')}
              />
            </button>
          )}
        </>
      }
    >
      <div className="mt-3 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className={cn('w-2 h-2 rounded-full', statusColors[status])} />
          <span className="text-sm text-[var(--easyssh-text-secondary)]">
            {statusLabels[status]}
          </span>
        </div>
        {os && (
          <div className="flex items-center gap-1 text-[var(--easyssh-text-tertiary)]">
            <Icon name={osIcons[os]} size={14} />
            <span className="text-xs capitalize">{os}</span>
          </div>
        )}
      </div>

      {tags.length > 0 && (
        <div className="mt-3 flex flex-wrap gap-1.5">
          {tags.map((tag) => (
            <span
              key={tag}
              className="px-2 py-0.5 text-xs font-medium rounded-full bg-[var(--easyssh-bg-tertiary)] text-[var(--easyssh-text-secondary)]"
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {lastConnected && (
        <p className="mt-3 text-xs text-[var(--easyssh-text-tertiary)]">
          Last connected: {lastConnected}
        </p>
      )}
    </Card>
  );
};

export { Card, cardVariants };
export default Card;
