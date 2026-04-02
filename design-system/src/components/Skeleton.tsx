import React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../utils';

/**
 * Skeleton Component
 *
 * Placeholder loading component with animated shimmer effect.
 */

const skeletonVariants = cva(
  'relative overflow-hidden bg-[var(--easyssh-bg-tertiary)]',
  {
    variants: {
      variant: {
        default: 'rounded',
        circle: 'rounded-full',
        card: 'rounded-lg',
        text: 'rounded',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  }
);

export interface SkeletonProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof skeletonVariants> {
  width?: string | number;
  height?: string | number;
  animate?: boolean;
}

const Skeleton: React.FC<SkeletonProps> = ({
  className,
  variant,
  width,
  height,
  animate = true,
  style,
  ...props
}) => {
  return (
    <div
      className={cn(
        skeletonVariants({ variant }),
        animate && 'animate-pulse',
        className
      )}
      style={{
        width: typeof width === 'number' ? `${width}px` : width,
        height: typeof height === 'number' ? `${height}px` : height,
        ...style,
      }}
      {...props}
    >
      {animate && (
        <div className="absolute inset-0 -translate-x-full animate-[shimmer_2s_infinite] bg-gradient-to-r from-transparent via-white/10 to-transparent" />
      )}
    </div>
  );
};

/**
 * Skeleton Text - Multiple lines of skeleton text
 */
export interface SkeletonTextProps {
  lines?: number;
  lineHeight?: number;
  gap?: number;
  className?: string;
  animate?: boolean;
}

export const SkeletonText: React.FC<SkeletonTextProps> = ({
  lines = 3,
  lineHeight = 16,
  gap = 8,
  className,
  animate = true,
}) => {
  return (
    <div className={cn('flex flex-col', className)} style={{ gap }}>
      {Array.from({ length: lines }).map((_, i) => (
        <Skeleton
          key={i}
          height={lineHeight}
          width={i === lines - 1 ? '80%' : '100%'}
          animate={animate}
        />
      ))}
    </div>
  );
};

/**
 * Skeleton Card - Card-shaped skeleton
 */
export interface SkeletonCardProps {
  header?: boolean;
  content?: boolean;
  footer?: boolean;
  className?: string;
}

export const SkeletonCard: React.FC<SkeletonCardProps> = ({
  header = true,
  content = true,
  footer = false,
  className,
}) => {
  return (
    <div
      className={cn(
        'rounded-lg border border-[var(--easyssh-border-subtle)] bg-[var(--easyssh-bg-elevated)] p-4',
        className
      )}
    >
      {header && (
        <div className="flex items-center gap-3 mb-4">
          <Skeleton variant="circle" width={40} height={40} />
          <div className="flex-1">
            <Skeleton height={16} width="60%" className="mb-2" />
            <Skeleton height={12} width="40%" />
          </div>
        </div>
      )}
      {content && (
        <div className="space-y-2">
          <Skeleton height={12} />
          <Skeleton height={12} />
          <Skeleton height={12} width="80%" />
        </div>
      )}
      {footer && (
        <div className="mt-4 pt-4 border-t border-[var(--easyssh-border-subtle)] flex gap-2">
          <Skeleton height={32} width={80} />
          <Skeleton height={32} width={80} />
        </div>
      )}
    </div>
  );
};

/**
 * Skeleton Avatar - Circular skeleton for avatars
 */
export interface SkeletonAvatarProps {
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
}

export const SkeletonAvatar: React.FC<SkeletonAvatarProps> = ({
  size = 'md',
  className,
}) => {
  const sizeMap = {
    xs: 24,
    sm: 32,
    md: 40,
    lg: 56,
    xl: 80,
  };

  return (
    <Skeleton
      variant="circle"
      width={sizeMap[size]}
      height={sizeMap[size]}
      className={className}
    />
  );
};

/**
 * Skeleton Stat Card - Specialized skeleton for stat cards
 */
export interface SkeletonStatCardProps {
  className?: string;
}

export const SkeletonStatCard: React.FC<SkeletonStatCardProps> = ({
  className,
}) => {
  return (
    <div
      className={cn(
        'rounded-lg border border-[var(--easyssh-border-subtle)] bg-[var(--easyssh-bg-elevated)] p-6',
        className
      )}
    >
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <Skeleton height={14} width="40%" className="mb-2" />
          <Skeleton height={32} width="60%" className="mb-2" />
          <Skeleton height={16} width="50%" />
        </div>
        <Skeleton variant="circle" width={48} height={48} />
      </div>
    </div>
  );
};

/**
 * Skeleton Table - Table skeleton with header and rows
 */
export interface SkeletonTableProps {
  columns?: number;
  rows?: number;
  className?: string;
}

export const SkeletonTable: React.FC<SkeletonTableProps> = ({
  columns = 4,
  rows = 5,
  className,
}) => {
  return (
    <div className={cn('w-full', className)}>
      {/* Header */}
      <div className="flex gap-4 mb-3 pb-3 border-b border-[var(--easyssh-border-subtle)]">
        {Array.from({ length: columns }).map((_, i) => (
          <Skeleton
            key={`header-${i}`}
            height={16}
            className={cn(
              'flex-1',
              i === columns - 1 && 'w-16 flex-none'
            )}
          />
        ))}
      </div>
      {/* Rows */}
      {Array.from({ length: rows }).map((_, rowIndex) => (
        <div key={`row-${rowIndex}`} className="flex gap-4 mb-3">
          {Array.from({ length: columns }).map((_, colIndex) => (
            <Skeleton
              key={`cell-${rowIndex}-${colIndex}`}
              height={12}
              className={cn(
                'flex-1',
                colIndex === columns - 1 && 'w-16 flex-none'
              )}
            />
          ))}
        </div>
      ))}
    </div>
  );
};

export { Skeleton, skeletonVariants };
export default Skeleton;
