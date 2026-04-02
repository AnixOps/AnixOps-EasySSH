import React, { forwardRef, useState } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../utils';
import { Icon, type IconName } from '../icons';

/**
 * Button Component
 *
 * A versatile button component with multiple variants, sizes, and states.
 * Supports icons, loading states, and full accessibility.
 */

const buttonVariants = cva(
  // Base styles
  'inline-flex items-center justify-center font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98]',
  {
    variants: {
      variant: {
        primary:
          'bg-[var(--easyssh-interactive-primary)] text-[var(--easyssh-text-inverted)] hover:bg-[var(--easyssh-interactive-primary-hover)] focus:ring-[var(--easyssh-interactive-primary)]',
        secondary:
          'bg-[var(--easyssh-interactive-secondary)] text-[var(--easyssh-text-primary)] border border-[var(--easyssh-border-default)] hover:bg-[var(--easyssh-interactive-secondary-hover)] focus:ring-[var(--easyssh-interactive-secondary)]',
        ghost:
          'bg-transparent text-[var(--easyssh-text-primary)] hover:bg-[var(--easyssh-interactive-ghost-hover)] focus:ring-[var(--easyssh-border-default)]',
        danger:
          'bg-red-500 text-white hover:bg-red-600 focus:ring-red-500',
        success:
          'bg-green-500 text-white hover:bg-green-600 focus:ring-green-500',
        icon:
          'p-2 bg-transparent text-[var(--easyssh-text-secondary)] hover:bg-[var(--easyssh-interactive-ghost-hover)] hover:text-[var(--easyssh-text-primary)] rounded-full',
      },
      size: {
        xs: 'h-6 px-2 text-xs rounded gap-1',
        sm: 'h-8 px-3 text-sm rounded-md gap-1.5',
        md: 'h-9 px-4 text-sm rounded-md gap-2',
        lg: 'h-11 px-5 text-base rounded-md gap-2',
        xl: 'h-13 px-6 text-base rounded-lg gap-2',
        icon: 'h-9 w-9 p-2 rounded-full',
      },
    },
    defaultVariants: {
      variant: 'primary',
      size: 'md',
    },
  }
);

const iconSizes = {
  xs: 12,
  sm: 14,
  md: 16,
  lg: 18,
  xl: 20,
  icon: 18,
};

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  /** Button label */
  children: React.ReactNode;
  /** Leading icon */
  icon?: IconName;
  /** Trailing icon */
  trailingIcon?: IconName;
  /** Loading state */
  loading?: boolean;
  /** Full width button */
  fullWidth?: boolean;
  /** Loading text (defaults to children) */
  loadingText?: string;
}

const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      children,
      className,
      variant,
      size,
      icon,
      trailingIcon,
      loading = false,
      fullWidth = false,
      loadingText,
      disabled,
      ...props
    },
    ref
  ) => {
    const [isPressed, setIsPressed] = useState(false);

    const handleMouseDown = () => setIsPressed(true);
    const handleMouseUp = () => setIsPressed(false);
    const handleMouseLeave = () => setIsPressed(false);

    const isDisabled = disabled || loading;
    const iconSize = iconSizes[size || 'md'];

    return (
      <button
        ref={ref}
        className={cn(
          buttonVariants({ variant, size }),
          fullWidth && 'w-full',
          isPressed && 'scale-[0.98]',
          className
        )}
        disabled={isDisabled}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        aria-busy={loading}
        {...props}
      >
        {loading ? (
          <>
            <svg
              className="animate-spin -ml-1 mr-2"
              width={iconSize}
              height={iconSize}
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
            <span>{loadingText || children}</span>
          </>
        ) : (
          <>
            {icon && (
              <Icon
                name={icon}
                size={iconSize}
                className={cn(
                  'flex-shrink-0',
                  !trailingIcon && children && 'mr-1'
                )}
              />
            )}
            {children && <span>{children}</span>}
            {trailingIcon && (
              <Icon
                name={trailingIcon}
                size={iconSize}
                className={cn(
                  'flex-shrink-0',
                  children && 'ml-1'
                )}
              />
            )}
          </>
        )}
      </button>
    );
  }
);

Button.displayName = 'Button';

/**
 * IconButton - Button with just an icon
 */
export interface IconButtonProps
  extends Omit<ButtonProps, 'icon' | 'trailingIcon' | 'children'> {
  /** Icon name */
  icon: IconName;
  /** Accessible label (required for icon-only buttons) */
  'aria-label': string;
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ icon, size = 'md', className, ...props }, ref) => {
    return (
      <Button
        ref={ref}
        variant="icon"
        size={size === 'icon' ? 'icon' : 'md'}
        className={cn('p-2', className)}
        {...props}
      >
        <Icon name={icon} size={iconSizes[size === 'icon' ? 'md' : size || 'md']} />
      </Button>
    );
  }
);

IconButton.displayName = 'IconButton';

/**
 * Button Group - Group related buttons
 */
export interface ButtonGroupProps {
  children: React.ReactNode;
  /** Join buttons together */
  joined?: boolean;
  /** Vertical layout */
  vertical?: boolean;
  className?: string;
}

export const ButtonGroup: React.FC<ButtonGroupProps> = ({
  children,
  joined = false,
  vertical = false,
  className,
}) => {
  return (
    <div
      className={cn(
        'flex',
        vertical ? 'flex-col' : 'flex-row',
        joined && !vertical && '[&>button:not(:first-child)]:-ml-px [&>button:not(:first-child)]:rounded-l-none [&>button:not(:last-child)]:rounded-r-none',
        joined && vertical && '[&>button:not(:first-child)]:-mt-px [&>button:not(:first-child)]:rounded-t-none [&>button:not(:last-child)]:rounded-b-none',
        className
      )}
      role="group"
    >
      {children}
    </div>
  );
};

/**
 * Split Button - Primary action with dropdown
 */
export interface SplitButtonProps extends ButtonProps {
  /** Dropdown button props */
  dropdownProps?: Omit<ButtonProps, 'children'>;
  /** Dropdown content */
  dropdownContent?: React.ReactNode;
}

export const SplitButton = forwardRef<HTMLButtonElement, SplitButtonProps>(
  ({ dropdownProps, dropdownContent, children, ...props }, ref) => {
    const [isOpen, setIsOpen] = React.useState(false);

    return (
      <div className="relative inline-flex">
        <Button ref={ref} {...props}>
          {children}
        </Button>
        <Button
          variant={props.variant}
          size={props.size}
          className="rounded-l-none border-l-0 px-2"
          onClick={() => setIsOpen(!isOpen)}
          aria-expanded={isOpen}
          aria-haspopup="menu"
          {...dropdownProps}
        >
          <Icon name="chevron-down" size={iconSizes[props.size || 'md']} />
        </Button>
        {isOpen && dropdownContent && (
          <div className="absolute top-full right-0 mt-1 z-50">
            {dropdownContent}
          </div>
        )}
      </div>
    );
  }
);

SplitButton.displayName = 'SplitButton';

export { Button, buttonVariants };
export default Button;
