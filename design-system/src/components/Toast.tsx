import React, { useEffect, useState } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../utils';
import { Icon, type IconName } from '../icons';
import { Button } from './Button';

/**
 * Toast Component
 *
 * Toast notifications for displaying temporary messages.
 */

const toastVariants = cva(
  'relative flex w-full max-w-sm items-start gap-3 rounded-lg p-4 shadow-lg border',
  {
    variants: {
      variant: {
        info: 'bg-[var(--easyssh-bg-elevated)] border-[var(--easyssh-border-default)]',
        success: 'bg-green-50 border-green-200 dark:bg-green-900/20 dark:border-green-800',
        warning: 'bg-yellow-50 border-yellow-200 dark:bg-yellow-900/20 dark:border-yellow-800',
        error: 'bg-red-50 border-red-200 dark:bg-red-900/20 dark:border-red-800',
      },
    },
    defaultVariants: {
      variant: 'info',
    },
  }
);

const iconMap: Record<string, { icon: IconName; color: string }> = {
  info: { icon: 'info', color: 'text-blue-500' },
  success: { icon: 'check-circle', color: 'text-green-500' },
  warning: { icon: 'alert-triangle', color: 'text-yellow-500' },
  error: { icon: 'alert-circle', color: 'text-red-500' },
};

export interface ToastProps extends VariantProps<typeof toastVariants> {
  id?: string;
  title: string;
  message?: string;
  variant?: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
  onClose?: () => void;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export const Toast: React.FC<ToastProps> = ({
  title,
  message,
  variant = 'info',
  duration = 4000,
  onClose,
  action,
}) => {
  const [progress, setProgress] = useState(100);
  const { icon, color } = iconMap[variant];

  useEffect(() => {
    if (duration === Infinity) return;

    const startTime = Date.now();
    const interval = setInterval(() => {
      const elapsed = Date.now() - startTime;
      const remaining = Math.max(0, duration - elapsed);
      setProgress((remaining / duration) * 100);

      if (remaining <= 0) {
        clearInterval(interval);
        onClose?.();
      }
    }, 50);

    return () => clearInterval(interval);
  }, [duration, onClose]);

  return (
    <div className={cn(toastVariants({ variant }))} role="alert">
      <div className={cn('flex-shrink-0 mt-0.5', color)}>
        <Icon name={icon} size={20} />
      </div>
      <div className="flex-1 min-w-0">
        <h4 className="text-sm font-medium text-[var(--easyssh-text-primary)]">
          {title}
        </h4>
        {message && (
          <p className="mt-1 text-sm text-[var(--easyssh-text-secondary)]">
            {message}
          </p>
        )}
        {action && (
          <div className="mt-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={action.onClick}
            >
              {action.label}
            </Button>
          </div>
        )}
      </div>
      {onClose && (
        <button
          onClick={onClose}
          className="flex-shrink-0 text-[var(--easyssh-text-tertiary)] hover:text-[var(--easyssh-text-secondary)] transition-colors"
          aria-label="Close"
        >
          <Icon name="x" size={16} />
        </button>
      )}
      {duration !== Infinity && (
        <div
          className="absolute bottom-0 left-0 h-0.5 bg-current opacity-30 transition-all duration-100"
          style={{ width: `${progress}%` }}
        />
      )}
    </div>
  );
};

/**
 * Toast Container - Manages multiple toasts
 */
export interface ToastContainerProps {
  toasts: ToastProps[];
  position?: 'top-left' | 'top-right' | 'top-center' | 'bottom-left' | 'bottom-right' | 'bottom-center';
  onRemove: (id: string) => void;
}

export const ToastContainer: React.FC<ToastContainerProps> = ({
  toasts,
  position = 'bottom-right',
  onRemove,
}) => {
  const positionClasses = {
    'top-left': 'top-4 left-4',
    'top-right': 'top-4 right-4',
    'top-center': 'top-4 left-1/2 -translate-x-1/2',
    'bottom-left': 'bottom-4 left-4',
    'bottom-right': 'bottom-4 right-4',
    'bottom-center': 'bottom-4 left-1/2 -translate-x-1/2',
  };

  return (
    <div
      className={cn(
        'fixed z-[9999] flex flex-col gap-2 pointer-events-none',
        positionClasses[position]
      )}
    >
      {toasts.map((toast) => (
        <div key={toast.id} className="pointer-events-auto animate-slide-in-right">
          <Toast
            {...toast}
            onClose={() => toast.id && onRemove(toast.id)}
          />
        </div>
      ))}
    </div>
  );
};

/**
 * useToast Hook
 */
export interface ToastOptions extends Omit<ToastProps, 'id'> {}

export const useToast = () => {
  const [toasts, setToasts] = useState<(ToastProps & { id: string })[]>([]);

  const addToast = (options: ToastOptions) => {
    const id = Math.random().toString(36).substr(2, 9);
    const newToast = { ...options, id };
    setToasts((prev) => [...prev, newToast]);
    return id;
  };

  const removeToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  const success = (title: string, message?: string, options?: Partial<ToastOptions>) => {
    return addToast({ title, message, variant: 'success', ...options });
  };

  const error = (title: string, message?: string, options?: Partial<ToastOptions>) => {
    return addToast({ title, message, variant: 'error', ...options });
  };

  const warning = (title: string, message?: string, options?: Partial<ToastOptions>) => {
    return addToast({ title, message, variant: 'warning', ...options });
  };

  const info = (title: string, message?: string, options?: Partial<ToastOptions>) => {
    return addToast({ title, message, variant: 'info', ...options });
  };

  return {
    toasts,
    addToast,
    removeToast,
    success,
    error,
    warning,
    info,
  };
};

export { Toast, toastVariants };
export default Toast;
