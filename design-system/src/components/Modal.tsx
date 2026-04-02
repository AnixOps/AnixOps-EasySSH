import React, { useEffect, useState } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../utils';
import { Icon } from '../icons';
import { Button } from './Button';

/**
 * Modal Component
 *
 * Dialog overlay for important information or actions.
 */

const modalOverlayVariants = cva(
  'fixed inset-0 bg-black/50 backdrop-blur-sm transition-opacity duration-200 z-50',
  {
    variants: {
      open: {
        true: 'opacity-100',
        false: 'opacity-0 pointer-events-none',
      },
    },
  }
);

const modalContentVariants = cva(
  'fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-[var(--easyssh-bg-elevated)] rounded-lg shadow-2xl transition-all duration-200 z-50 overflow-hidden',
  {
    variants: {
      open: {
        true: 'opacity-100 scale-100',
        false: 'opacity-0 scale-95',
      },
      size: {
        sm: 'w-full max-w-md',
        md: 'w-full max-w-lg',
        lg: 'w-full max-w-2xl',
        xl: 'w-full max-w-4xl',
        fullscreen: 'inset-4 w-auto h-auto rounded-lg',
      },
    },
    defaultVariants: {
      open: true,
      size: 'md',
    },
  }
);

export interface ModalProps extends VariantProps<typeof modalContentVariants> {
  open: boolean;
  onClose: () => void;
  title?: string;
  description?: string;
  children?: React.ReactNode;
  footer?: React.ReactNode;
  closeOnOverlay?: boolean;
  closeOnEscape?: boolean;
  hideCloseButton?: boolean;
  className?: string;
}

const Modal: React.FC<ModalProps> = ({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  closeOnOverlay = true,
  closeOnEscape = true,
  hideCloseButton = false,
  size = 'md',
  className,
}) => {
  const [isMounted, setIsMounted] = useState(false);

  // Handle escape key
  useEffect(() => {
    if (!closeOnEscape) return;

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && open) {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [open, onClose, closeOnEscape]);

  // Handle mount/unmount for animations
  useEffect(() => {
    if (open) {
      setIsMounted(true);
      document.body.style.overflow = 'hidden';
    } else {
      const timer = setTimeout(() => {
        setIsMounted(false);
        document.body.style.overflow = '';
      }, 200);
      return () => clearTimeout(timer);
    }
  }, [open]);

  if (!isMounted && !open) return null;

  return (
    <>
      {/* Overlay */}
      <div
        className={cn(modalOverlayVariants({ open }))}
        onClick={closeOnOverlay ? onClose : undefined}
        aria-hidden="true"
      />

      {/* Modal Content */}
      <div
        className={cn(modalContentVariants({ open, size }), className)}
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? 'modal-title' : undefined}
        aria-describedby={description ? 'modal-description' : undefined}
      >
        {/* Header */}
        {(title || !hideCloseButton) && (
          <div className="flex items-start justify-between px-6 py-4 border-b border-[var(--easyssh-border-subtle)]">
            <div>
              {title && (
                <h2
                  id="modal-title"
                  className="text-lg font-semibold text-[var(--easyssh-text-primary)]"
                >
                  {title}
                </h2>
              )}
              {description && (
                <p
                  id="modal-description"
                  className="mt-1 text-sm text-[var(--easyssh-text-secondary)]"
                >
                  {description}
                </p>
              )}
            </div>
            {!hideCloseButton && (
              <button
                onClick={onClose}
                className="text-[var(--easyssh-text-tertiary)] hover:text-[var(--easyssh-text-secondary)] transition-colors rounded-full p-1 -mr-2 -mt-2"
                aria-label="Close"
              >
                <Icon name="x" size={20} />
              </button>
            )}
          </div>
        )}

        {/* Body */}
        <div className="px-6 py-4 max-h-[calc(100vh-200px)] overflow-y-auto">
          {children}
        </div>

        {/* Footer */}
        {footer && (
          <div className="flex items-center justify-end gap-2 px-6 py-4 border-t border-[var(--easyssh-border-subtle)] bg-[var(--easyssh-bg-secondary)]">
            {footer}
          </div>
        )}
      </div>
    </>
  );
};

/**
 * Confirmation Modal - Pre-built modal for confirmations
 */
export interface ConfirmModalProps extends Omit<ModalProps, 'footer' | 'children'> {
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void;
  onCancel?: () => void;
  variant?: 'default' | 'danger';
  loading?: boolean;
}

export const ConfirmModal: React.FC<ConfirmModalProps> = ({
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  onConfirm,
  onCancel,
  variant = 'default',
  loading = false,
  ...props
}) => {
  const handleCancel = () => {
    onCancel?.();
    props.onClose();
  };

  const handleConfirm = () => {
    onConfirm();
  };

  return (
    <Modal
      {...props}
      footer={
        <>
          <Button
            variant="secondary"
            onClick={handleCancel}
            disabled={loading}
          >
            {cancelLabel}
          </Button>
          <Button
            variant={variant === 'danger' ? 'danger' : 'primary'}
            onClick={handleConfirm}
            loading={loading}
          >
            {confirmLabel}
          </Button>
        </>
      }
    >
      <p className="text-[var(--easyssh-text-secondary)]">
        {props.description}
      </p>
    </Modal>
  );
};

/**
 * Alert Modal - Pre-built modal for alerts
 */
export interface AlertModalProps extends Omit<ModalProps, 'footer' | 'children'> {
  actionLabel?: string;
  onAction?: () => void;
}

export const AlertModal: React.FC<AlertModalProps> = ({
  actionLabel = 'OK',
  onAction,
  ...props
}) => {
  const handleAction = () => {
    onAction?.();
    props.onClose();
  };

  return (
    <Modal
      {...props}
      footer={
        <Button variant="primary" onClick={handleAction}>
          {actionLabel}
        </Button>
      }
    >
      <p className="text-[var(--easyssh-text-secondary)]">
        {props.description}
      </p>
    </Modal>
  );
};

export { Modal, modalOverlayVariants, modalContentVariants };
export default Modal;
