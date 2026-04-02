import React, { forwardRef, useState } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../utils';
import { Icon, type IconName } from '../icons';

/**
 * Input Component
 *
 * Form input field with support for icons, validation, and various states.
 */

const inputVariants = cva(
  'w-full bg-[var(--easyssh-bg-tertiary)] border border-[var(--easyssh-border-default)] rounded-md text-[var(--easyssh-text-primary)] placeholder:text-[var(--easyssh-text-quaternary)] transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-[var(--easyssh-focus-color)] focus:border-[var(--easyssh-interactive-primary)] disabled:opacity-50 disabled:cursor-not-allowed',
  {
    variants: {
      size: {
        sm: 'h-8 px-2.5 text-sm',
        md: 'h-9 px-3 text-sm',
        lg: 'h-11 px-4 text-base',
      },
      state: {
        default: '',
        error: 'border-red-500 focus:border-red-500 focus:ring-red-500/20',
        success: 'border-green-500 focus:border-green-500 focus:ring-green-500/20',
      },
    },
    defaultVariants: {
      size: 'md',
      state: 'default',
    },
  }
);

export interface InputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'>,
    VariantProps<typeof inputVariants> {
  /** Input label */
  label?: string;
  /** Helper text */
  helper?: string;
  /** Error message */
  error?: string;
  /** Leading icon */
  icon?: IconName;
  /** Trailing element */
  trailing?: React.ReactNode;
  /** Full width input */
  fullWidth?: boolean;
}

const Input = forwardRef<HTMLInputElement, InputProps>(
  (
    {
      className,
      label,
      helper,
      error,
      icon,
      trailing,
      size,
      state,
      fullWidth = true,
      type = 'text',
      disabled,
      ...props
    },
    ref
  ) => {
    const [showPassword, setShowPassword] = useState(false);
    const isPassword = type === 'password';
    const inputType = isPassword ? (showPassword ? 'text' : 'password') : type;

    return (
      <div className={cn(fullWidth && 'w-full')}>
        {label && (
          <label className="block mb-1.5 text-sm font-medium text-[var(--easyssh-text-secondary)]">
            {label}
            {props.required && (
              <span className="ml-1 text-red-500">*</span>
            )}
          </label>
        )}
        <div className="relative">
          {icon && (
            <div className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--easyssh-text-tertiary)]">
              <Icon name={icon} size={16} />
            </div>
          )}
          <input
            ref={ref}
            type={inputType}
            disabled={disabled}
            className={cn(
              inputVariants({ size, state: error ? 'error' : state }),
              icon && 'pl-9',
              (trailing || isPassword) && 'pr-9',
              className
            )}
            aria-invalid={error ? 'true' : 'false'}
            aria-describedby={error ? 'error-message' : helper ? 'helper-message' : undefined}
            {...props}
          />
          {isPassword && (
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-[var(--easyssh-text-tertiary)] hover:text-[var(--easyssh-text-secondary)] transition-colors"
              tabIndex={-1}
            >
              <Icon name={showPassword ? 'eye-off' : 'eye'} size={16} />
            </button>
          )}
          {!isPassword && trailing && (
            <div className="absolute right-3 top-1/2 -translate-y-1/2">
              {trailing}
            </div>
          )}
        </div>
        {(helper || error) && (
          <p
            id={error ? 'error-message' : 'helper-message'}
            className={cn(
              'mt-1.5 text-sm',
              error
                ? 'text-red-500'
                : 'text-[var(--easyssh-text-tertiary)]'
            )}
          >
            {error || helper}
          </p>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';

/**
 * TextArea Component
 */
export interface TextAreaProps
  extends Omit<React.TextareaHTMLAttributes<HTMLTextAreaElement>, 'size'> {
  label?: string;
  helper?: string;
  error?: string;
  fullWidth?: boolean;
  rows?: number;
}

export const TextArea = forwardRef<HTMLTextAreaElement, TextAreaProps>(
  ({ label, helper, error, fullWidth = true, rows = 4, className, ...props }, ref) => {
    return (
      <div className={cn(fullWidth && 'w-full')}>
        {label && (
          <label className="block mb-1.5 text-sm font-medium text-[var(--easyssh-text-secondary)]">
            {label}
            {props.required && <span className="ml-1 text-red-500">*</span>}
          </label>
        )}
        <textarea
          ref={ref}
          rows={rows}
          className={cn(
            'w-full bg-[var(--easyssh-bg-tertiary)] border border-[var(--easyssh-border-default)] rounded-md px-3 py-2',
            'text-[var(--easyssh-text-primary)] placeholder:text-[var(--easyssh-text-quaternary)]',
            'focus:outline-none focus:ring-2 focus:ring-[var(--easyssh-focus-color)] focus:border-[var(--easyssh-interactive-primary)]',
            'disabled:opacity-50 disabled:cursor-not-allowed resize-y min-h-[80px]',
            error && 'border-red-500 focus:border-red-500 focus:ring-red-500/20',
            className
          )}
          aria-invalid={error ? 'true' : 'false'}
          {...props}
        />
        {(helper || error) && (
          <p
            className={cn(
              'mt-1.5 text-sm',
              error ? 'text-red-500' : 'text-[var(--easyssh-text-tertiary)]'
            )}
          >
            {error || helper}
          </p>
        )}
      </div>
    );
  }
);

TextArea.displayName = 'TextArea';

/**
 * Select Component
 */
export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SelectProps
  extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'size'> {
  label?: string;
  helper?: string;
  error?: string;
  options: SelectOption[];
  fullWidth?: boolean;
  size?: 'sm' | 'md' | 'lg';
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ label, helper, error, options, fullWidth = true, size = 'md', className, ...props }, ref) => {
    const sizeClasses = {
      sm: 'h-8 px-2.5 text-sm',
      md: 'h-9 px-3 text-sm',
      lg: 'h-11 px-4 text-base',
    };

    return (
      <div className={cn(fullWidth && 'w-full')}>
        {label && (
          <label className="block mb-1.5 text-sm font-medium text-[var(--easyssh-text-secondary)]">
            {label}
            {props.required && <span className="ml-1 text-red-500">*</span>}
          </label>
        )}
        <div className="relative">
          <select
            ref={ref}
            className={cn(
              'w-full bg-[var(--easyssh-bg-tertiary)] border border-[var(--easyssh-border-default)] rounded-md',
              'text-[var(--easyssh-text-primary)]',
              'focus:outline-none focus:ring-2 focus:ring-[var(--easyssh-focus-color)] focus:border-[var(--easyssh-interactive-primary)]',
              'disabled:opacity-50 disabled:cursor-not-allowed appearance-none pr-8',
              sizeClasses[size],
              error && 'border-red-500 focus:border-red-500 focus:ring-red-500/20',
              className
            )}
            aria-invalid={error ? 'true' : 'false'}
            {...props}
          >
            {options.map((option) => (
              <option key={option.value} value={option.value} disabled={option.disabled}>
                {option.label}
              </option>
            ))}
          </select>
          <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-[var(--easyssh-text-tertiary)]">
            <Icon name="chevron-down" size={16} />
          </div>
        </div>
        {(helper || error) && (
          <p
            className={cn(
              'mt-1.5 text-sm',
              error ? 'text-red-500' : 'text-[var(--easyssh-text-tertiary)]'
            )}
          >
            {error || helper}
          </p>
        )}
      </div>
    );
  }
);

Select.displayName = 'Select';

/**
 * Checkbox Component
 */
export interface CheckboxProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type' | 'size'> {
  label?: string;
  indeterminate?: boolean;
  size?: 'sm' | 'md' | 'lg';
}

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ label, indeterminate, size = 'md', className, ...props }, ref) => {
    const sizeClasses = {
      sm: 'w-4 h-4',
      md: 'w-5 h-5',
      lg: 'w-6 h-6',
    };

    return (
      <label className="inline-flex items-center gap-2 cursor-pointer">
        <div className="relative">
          <input
            ref={ref}
            type="checkbox"
            className={cn(
              'appearance-none bg-[var(--easyssh-bg-tertiary)] border border-[var(--easyssh-border-default)] rounded',
              'checked:bg-[var(--easyssh-interactive-primary)] checked:border-[var(--easyssh-interactive-primary)]',
              'focus:outline-none focus:ring-2 focus:ring-[var(--easyssh-focus-color)]',
              'disabled:opacity-50 disabled:cursor-not-allowed',
              sizeClasses[size],
              className
            )}
            data-indeterminate={indeterminate}
            {...props}
          />
          {props.checked && !indeterminate && (
            <svg
              className="absolute inset-0 m-auto pointer-events-none text-white"
              width={size === 'sm' ? 10 : size === 'md' ? 12 : 14}
              height={size === 'sm' ? 10 : size === 'md' ? 12 : 14}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="3"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <polyline points="20 6 9 17 4 12" />
            </svg>
          )}
          {indeterminate && (
            <svg
              className="absolute inset-0 m-auto pointer-events-none text-white"
              width={size === 'sm' ? 10 : size === 'md' ? 12 : 14}
              height={size === 'sm' ? 10 : size === 'md' ? 12 : 14}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="3"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          )}
        </div>
        {label && (
          <span className="text-sm text-[var(--easyssh-text-secondary)]">
            {label}
          </span>
        )}
      </label>
    );
  }
);

Checkbox.displayName = 'Checkbox';

/**
 * Switch/Toggle Component
 */
export interface SwitchProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type' | 'size'> {
  label?: string;
  size?: 'sm' | 'md' | 'lg';
}

export const Switch = forwardRef<HTMLInputElement, SwitchProps>(
  ({ label, size = 'md', className, ...props }, ref) => {
    const sizeClasses = {
      sm: 'w-8 h-4',
      md: 'w-11 h-6',
      lg: 'w-14 h-7',
    };

    const thumbSize = {
      sm: 'w-3 h-3',
      md: 'w-5 h-5',
      lg: 'w-6 h-6',
    };

    const thumbTranslate = {
      sm: 'translate-x-4',
      md: 'translate-x-5',
      lg: 'translate-x-7',
    };

    return (
      <label className="inline-flex items-center gap-3 cursor-pointer">
        <div className="relative">
          <input
            ref={ref}
            type="checkbox"
            className="sr-only peer"
            {...props}
          />
          <div
            className={cn(
              'bg-[var(--easyssh-border-default)] rounded-full transition-colors duration-200',
              'peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-[var(--easyssh-focus-color)]',
              'peer-checked:bg-[var(--easyssh-interactive-primary)]',
              'peer-disabled:opacity-50 peer-disabled:cursor-not-allowed',
              sizeClasses[size],
              className
            )}
          >
            <div
              className={cn(
                'absolute top-0.5 left-0.5 bg-white rounded-full shadow-sm transition-transform duration-200',
                thumbSize[size],
                'peer-checked:' + thumbTranslate[size]
              )}
            />
          </div>
        </div>
        {label && (
          <span className="text-sm text-[var(--easyssh-text-secondary)]">
            {label}
          </span>
        )}
      </label>
    );
  }
);

Switch.displayName = 'Switch';

export { Input, inputVariants };
export default Input;
