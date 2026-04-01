import type { Config } from 'tailwindcss';
import * as tokens from './src/tokens/design-tokens';

/**
 * EasySSH Tailwind Configuration
 * Integrates design tokens with Tailwind CSS
 */

const config: Config = {
  content: [
    './src/**/*.{js,ts,jsx,tsx}',
    './packages/**/*.{js,ts,jsx,tsx}',
    './apps/**/*.{js,ts,jsx,tsx}',
  ],
  darkMode: ['class', '[data-theme="dark"]'],
  theme: {
    extend: {
      // ==========================================================================
      // Colors - Semantic color tokens
      // ==========================================================================
      colors: {
        // Brand colors
        brand: {
          50: 'var(--easyssh-primary-50)',
          100: 'var(--easyssh-primary-100)',
          200: 'var(--easyssh-primary-200)',
          300: 'var(--easyssh-primary-300)',
          400: 'var(--easyssh-primary-400)',
          500: 'var(--easyssh-primary-500)',
          600: 'var(--easyssh-primary-600)',
          700: 'var(--easyssh-primary-700)',
          800: 'var(--easyssh-primary-800)',
          900: 'var(--easyssh-primary-900)',
        },

        // Background colors
        background: {
          primary: 'var(--easyssh-bg-primary)',
          secondary: 'var(--easyssh-bg-secondary)',
          tertiary: 'var(--easyssh-bg-tertiary)',
          quaternary: 'var(--easyssh-bg-quaternary)',
          elevated: 'var(--easyssh-bg-elevated)',
          overlay: 'var(--easyssh-bg-overlay)',
          terminal: 'var(--easyssh-bg-terminal)',
        },

        // Text colors
        foreground: {
          primary: 'var(--easyssh-text-primary)',
          secondary: 'var(--easyssh-text-secondary)',
          tertiary: 'var(--easyssh-text-tertiary)',
          quaternary: 'var(--easyssh-text-quaternary)',
          inverted: 'var(--easyssh-text-inverted)',
          terminal: 'var(--easyssh-text-terminal)',
        },

        // Border colors
        border: {
          subtle: 'var(--easyssh-border-subtle)',
          DEFAULT: 'var(--easyssh-border-default)',
          strong: 'var(--easyssh-border-strong)',
        },

        // Interactive colors
        interactive: {
          primary: 'var(--easyssh-interactive-primary)',
          'primary-hover': 'var(--easyssh-interactive-primary-hover)',
          'primary-active': 'var(--easyssh-interactive-primary-active)',
          'primary-text': 'var(--easyssh-interactive-primary-text)',
          secondary: 'var(--easyssh-interactive-secondary)',
          'secondary-hover': 'var(--easyssh-interactive-secondary-hover)',
          'secondary-active': 'var(--easyssh-interactive-secondary-active)',
          'secondary-text': 'var(--easyssh-interactive-secondary-text)',
          ghost: 'var(--easyssh-interactive-ghost)',
          'ghost-hover': 'var(--easyssh-interactive-ghost-hover)',
          'ghost-active': 'var(--easyssh-interactive-ghost-active)',
        },

        // Semantic colors
        success: {
          DEFAULT: 'var(--easyssh-success-main)',
          bg: 'var(--easyssh-success-bg)',
          border: 'var(--easyssh-success-border)',
        },
        warning: {
          DEFAULT: 'var(--easyssh-warning-main)',
          bg: 'var(--easyssh-warning-bg)',
          border: 'var(--easyssh-warning-border)',
        },
        danger: {
          DEFAULT: 'var(--easyssh-danger-main)',
          bg: 'var(--easyssh-danger-bg)',
          border: 'var(--easyssh-danger-border)',
        },
        info: {
          DEFAULT: 'var(--easyssh-info-main)',
          bg: 'var(--easyssh-info-bg)',
          border: 'var(--easyssh-info-border)',
        },

        // Status colors
        status: {
          online: 'var(--easyssh-status-online)',
          offline: 'var(--easyssh-status-offline)',
          connecting: 'var(--easyssh-status-connecting)',
          maintenance: 'var(--easyssh-status-maintenance)',
          unknown: 'var(--easyssh-status-unknown)',
        },

        // Terminal colors
        terminal: {
          black: 'var(--easyssh-terminal-black)',
          red: 'var(--easyssh-terminal-red)',
          green: 'var(--easyssh-terminal-green)',
          yellow: 'var(--easyssh-terminal-yellow)',
          blue: 'var(--easyssh-terminal-blue)',
          magenta: 'var(--easyssh-terminal-magenta)',
          cyan: 'var(--easyssh-terminal-cyan)',
          white: 'var(--easyssh-terminal-white)',
          'bright-black': 'var(--easyssh-terminal-bright-black)',
          'bright-red': 'var(--easyssh-terminal-bright-red)',
          'bright-green': 'var(--easyssh-terminal-bright-green)',
          'bright-yellow': 'var(--easyssh-terminal-bright-yellow)',
          'bright-blue': 'var(--easyssh-terminal-bright-blue)',
          'bright-magenta': 'var(--easyssh-terminal-bright-magenta)',
          'bright-cyan': 'var(--easyssh-terminal-bright-cyan)',
          'bright-white': 'var(--easyssh-terminal-bright-white)',
          cursor: 'var(--easyssh-terminal-cursor)',
          selection: 'var(--easyssh-terminal-selection)',
        },

        // Focus ring
        focus: 'var(--easyssh-focus-color)',
      },

      // ==========================================================================
      // Typography
      // ==========================================================================
      fontFamily: {
        sans: tokens.typography.fontFamily.sans,
        mono: tokens.typography.fontFamily.mono,
        display: tokens.typography.fontFamily.display,
      },

      fontSize: {
        '2xs': tokens.typography.fontSize['2xs'],
        xs: tokens.typography.fontSize.xs,
        sm: tokens.typography.fontSize.sm,
        base: tokens.typography.fontSize.base,
        md: tokens.typography.fontSize.md,
        lg: tokens.typography.fontSize.lg,
        xl: tokens.typography.fontSize.xl,
        '2xl': tokens.typography.fontSize['2xl'],
        '3xl': tokens.typography.fontSize['3xl'],
        '4xl': tokens.typography.fontSize['4xl'],
      },

      fontWeight: {
        normal: tokens.typography.fontWeight.normal,
        medium: tokens.typography.fontWeight.medium,
        semibold: tokens.typography.fontWeight.semibold,
        bold: tokens.typography.fontWeight.bold,
      },

      lineHeight: {
        none: tokens.typography.lineHeight.none,
        tight: tokens.typography.lineHeight.tight,
        snug: tokens.typography.lineHeight.snug,
        normal: tokens.typography.lineHeight.normal,
        relaxed: tokens.typography.lineHeight.relaxed,
        loose: tokens.typography.lineHeight.loose,
      },

      letterSpacing: {
        tighter: tokens.typography.letterSpacing.tighter,
        tight: tokens.typography.letterSpacing.tight,
        normal: tokens.typography.letterSpacing.normal,
        wide: tokens.typography.letterSpacing.wide,
        wider: tokens.typography.letterSpacing.wider,
        widest: tokens.typography.letterSpacing.widest,
      },

      // ==========================================================================
      // Spacing
      // ==========================================================================
      spacing: tokens.spacing,

      // ==========================================================================
      // Border Radius
      // ==========================================================================
      borderRadius: {
        none: tokens.borders.radius.none,
        xs: tokens.borders.radius.xs,
        sm: tokens.borders.radius.sm,
        md: tokens.borders.radius.md,
        lg: tokens.borders.radius.lg,
        xl: tokens.borders.radius.xl,
        '2xl': tokens.borders.radius['2xl'],
        '3xl': tokens.borders.radius['3xl'],
        full: tokens.borders.radius.full,
      },

      // ==========================================================================
      // Border Width
      // ==========================================================================
      borderWidth: {
        DEFAULT: tokens.borders.width.thin,
        0: tokens.borders.width.none,
        2: tokens.borders.width.medium,
        4: tokens.borders.width.thick,
      },

      // ==========================================================================
      // Shadows
      // ==========================================================================
      boxShadow: {
        none: tokens.shadows.none,
        xs: tokens.shadows.xs,
        sm: tokens.shadows.sm,
        md: tokens.shadows.md,
        lg: tokens.shadows.lg,
        xl: tokens.shadows.xl,
        '2xl': tokens.shadows['2xl'],
        inner: tokens.shadows.inner,
        terminal: tokens.shadows.terminal,
        card: tokens.shadows.card,
        popover: tokens.shadows.popover,
        dropdown: tokens.shadows.dropdown,
        modal: tokens.shadows.modal,
        tooltip: tokens.shadows.tooltip,
      },

      // ==========================================================================
      // Z-Index
      // ==========================================================================
      zIndex: {
        base: tokens.zIndex.base,
        dropdown: tokens.zIndex.dropdown,
        sticky: tokens.zIndex.sticky,
        fixed: tokens.zIndex.fixed,
        overlay: tokens.zIndex.overlay,
        'modal-backdrop': tokens.zIndex.modalBackdrop,
        modal: tokens.zIndex.modal,
        popover: tokens.zIndex.popover,
        tooltip: tokens.zIndex.tooltip,
        toast: tokens.zIndex.toast,
        'command-palette': tokens.zIndex.commandPalette,
      },

      // ==========================================================================
      // Transitions
      // ==========================================================================
      transitionDuration: {
        instant: tokens.motion.duration.instant,
        fast: tokens.motion.duration.fast,
        DEFAULT: tokens.motion.duration.normal,
        slow: tokens.motion.duration.slow,
        slower: tokens.motion.duration.slower,
        slowest: tokens.motion.duration.slowest,
      },

      transitionTimingFunction: {
        ease: tokens.motion.easing.ease,
        'ease-in': tokens.motion.easing.easeIn,
        'ease-out': tokens.motion.easing.easeOut,
        spring: tokens.motion.easing.spring,
        smooth: tokens.motion.easing.smooth,
        snappy: tokens.motion.easing.snappy,
      },

      // ==========================================================================
      // Animations
      // ==========================================================================
      animation: {
        'fade-in': 'fadeIn 200ms ease-out',
        'fade-out': 'fadeOut 200ms ease-in',
        'slide-in-up': 'slideInUp 300ms smooth',
        'slide-in-down': 'slideInDown 300ms smooth',
        'slide-in-left': 'slideInLeft 300ms smooth',
        'slide-in-right': 'slideInRight 300ms smooth',
        'scale-in': 'scaleIn 200ms spring',
        'scale-out': 'scaleOut 150ms ease-in',
        pulse: 'pulse 2s ease-in-out infinite',
        spin: 'spin 1s linear infinite',
        bounce: 'bounce 1s ease-in-out',
        shimmer: 'shimmer 2s linear infinite',
        blink: 'blink 1s step-end infinite',
      },

      keyframes: {
        fadeIn: {
          from: { opacity: '0' },
          to: { opacity: '1' },
        },
        fadeOut: {
          from: { opacity: '1' },
          to: { opacity: '0' },
        },
        slideInUp: {
          from: { opacity: '0', transform: 'translateY(8px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        slideInDown: {
          from: { opacity: '0', transform: 'translateY(-8px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        slideInLeft: {
          from: { opacity: '0', transform: 'translateX(-16px)' },
          to: { opacity: '1', transform: 'translateX(0)' },
        },
        slideInRight: {
          from: { opacity: '0', transform: 'translateX(16px)' },
          to: { opacity: '1', transform: 'translateX(0)' },
        },
        scaleIn: {
          from: { opacity: '0', transform: 'scale(0.95)' },
          to: { opacity: '1', transform: 'scale(1)' },
        },
        scaleOut: {
          from: { opacity: '1', transform: 'scale(1)' },
          to: { opacity: '0', transform: 'scale(0.95)' },
        },
        pulse: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0.5' },
        },
        spin: {
          from: { transform: 'rotate(0deg)' },
          to: { transform: 'rotate(360deg)' },
        },
        bounce: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-4px)' },
        },
        shimmer: {
          '0%': { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
        blink: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0' },
        },
      },

      // ==========================================================================
      // Breakpoints
      // ==========================================================================
      screens: tokens.breakpoints,
    },
  },
  plugins: [
    // Custom plugin for focus-visible styles
    function({ addUtilities }) {
      addUtilities({
        '.focus-ring': {
          outline: 'none',
          boxShadow: 'var(--easyssh-focus-ring)',
        },
        '.focus-ring-inset': {
          outline: 'none',
          boxShadow: 'inset var(--easyssh-focus-ring)',
        },
      });
    },

    // Custom plugin for terminal text colors
    function({ addUtilities }) {
      const terminalColors: Record<string, { color: string }> = {};
      const colors = [
        'black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white',
        'bright-black', 'bright-red', 'bright-green', 'bright-yellow',
        'bright-blue', 'bright-magenta', 'bright-cyan', 'bright-white',
      ];

      colors.forEach((color) => {
        terminalColors[`.text-terminal-${color}`] = {
          color: `var(--easyssh-terminal-${color})`,
        };
        terminalColors[`.bg-terminal-${color}`] = {
          backgroundColor: `var(--easyssh-terminal-${color})`,
        };
      });

      addUtilities(terminalColors);
    },

    // RTL support utilities
    function({ addUtilities }) {
      addUtilities({
        '.rtl': {
          direction: 'rtl',
        },
        '.ltr': {
          direction: 'ltr',
        },
        '.start-0': {
          insetInlineStart: '0',
        },
        '.end-0': {
          insetInlineEnd: '0',
        },
        '.me-2': {
          marginInlineEnd: '0.5rem',
        },
        '.ms-2': {
          marginInlineStart: '0.5rem',
        },
        '.pe-2': {
          paddingInlineEnd: '0.5rem',
        },
        '.ps-2': {
          paddingInlineStart: '0.5rem',
        },
      });
    },
  ],
};

export default config;
