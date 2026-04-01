/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    './index.html',
    './src/**/*.{js,ts,jsx,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        // Apple Design System Colors (Dark Mode Focus)
        apple: {
          // Backgrounds
          bg: {
            primary: '#000000',
            secondary: '#1c1c1e',
            tertiary: '#2c2c2e',
            quaternary: '#3a3a3c',
            elevated: '#1c1c1e',
          },
          // Text
          text: {
            primary: '#ffffff',
            secondary: 'rgba(255, 255, 255, 0.55)',
            tertiary: 'rgba(255, 255, 255, 0.25)',
            quaternary: 'rgba(255, 255, 255, 0.10)',
          },
          // Accents
          accent: {
            blue: '#0a84ff',
            green: '#30d158',
            indigo: '#5e5ce6',
            orange: '#ff9f0a',
            pink: '#ff375f',
            purple: '#bf5af2',
            red: '#ff453a',
            teal: '#64d2ff',
            yellow: '#ffd60a',
          },
          // Grays
          gray: {
            1: '#8e8e93',
            2: '#636366',
            3: '#48484a',
            4: '#3a3a3c',
            5: '#2c2c2e',
            6: '#1c1c1e',
          },
          // UI Elements
          border: 'rgba(255, 255, 255, 0.08)',
          separator: 'rgba(255, 255, 255, 0.08)',
          overlay: 'rgba(0, 0, 0, 0.7)',
        },
      },
      fontFamily: {
        sans: [
          '-apple-system',
          'BlinkMacSystemFont',
          'SF Pro Display',
          'SF Pro Text',
          'Segoe UI',
          'Roboto',
          'Helvetica Neue',
          'Arial',
          'sans-serif',
        ],
        mono: [
          'SF Mono',
          'SFMono-Regular',
          'Menlo',
          'Monaco',
          'Consolas',
          'Liberation Mono',
          'Courier New',
          'monospace',
        ],
      },
      fontSize: {
        'apple-xs': ['11px', { lineHeight: '13px', letterSpacing: '-0.01em' }],
        'apple-sm': ['12px', { lineHeight: '16px', letterSpacing: '-0.01em' }],
        'apple-base': ['13px', { lineHeight: '20px', letterSpacing: '-0.01em' }],
        'apple-md': ['15px', { lineHeight: '22px', letterSpacing: '-0.01em' }],
        'apple-lg': ['17px', { lineHeight: '24px', letterSpacing: '-0.02em' }],
        'apple-xl': ['20px', { lineHeight: '28px', letterSpacing: '-0.02em' }],
        'apple-2xl': ['24px', { lineHeight: '32px', letterSpacing: '-0.02em' }],
      },
      spacing: {
        'apple-1': '4px',
        'apple-2': '8px',
        'apple-3': '12px',
        'apple-4': '16px',
        'apple-5': '20px',
        'apple-6': '24px',
        'apple-8': '32px',
        'apple-10': '40px',
        'apple-12': '48px',
      },
      borderRadius: {
        'apple-sm': '6px',
        'apple-md': '8px',
        'apple-lg': '12px',
        'apple-xl': '16px',
        'apple-2xl': '20px',
      },
      boxShadow: {
        'apple-sm': '0 1px 2px rgba(0, 0, 0, 0.3)',
        'apple-md': '0 4px 12px rgba(0, 0, 0, 0.4)',
        'apple-lg': '0 8px 24px rgba(0, 0, 0, 0.5)',
        'apple-xl': '0 16px 48px rgba(0, 0, 0, 0.6)',
        'apple-glow': '0 0 20px rgba(10, 132, 255, 0.3)',
      },
      transitionTimingFunction: {
        'apple-ease': 'cubic-bezier(0.4, 0.0, 0.2, 1)',
        'apple-spring': 'cubic-bezier(0.175, 0.885, 0.32, 1.275)',
      },
      animation: {
        'apple-fade-in': 'fadeIn 0.2s ease-out',
        'apple-slide-up': 'slideUp 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
        'apple-slide-in': 'slideIn 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(10px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        slideIn: {
          '0%': { opacity: '0', transform: 'translateX(-10px)' },
          '100%': { opacity: '1', transform: 'translateX(0)' },
        },
      },
    },
  },
  plugins: [
    function({ addUtilities }) {
      addUtilities({
        '.apple-contain': {
          contain: 'layout style paint',
        },
        '.apple-blur': {
          backdropFilter: 'blur(20px) saturate(180%)',
          WebkitBackdropFilter: 'blur(20px) saturate(180%)',
        },
        '.apple-glass': {
          backgroundColor: 'rgba(28, 28, 30, 0.72)',
          backdropFilter: 'blur(20px) saturate(180%)',
          WebkitBackdropFilter: 'blur(20px) saturate(180%)',
        },
        '.apple-scrollbar': {
          scrollbarWidth: 'thin',
          scrollbarColor: 'rgba(255, 255, 255, 0.2) transparent',
          '&::-webkit-scrollbar': {
            width: '8px',
            height: '8px',
          },
          '&::-webkit-scrollbar-track': {
            background: 'transparent',
          },
          '&::-webkit-scrollbar-thumb': {
            backgroundColor: 'rgba(255, 255, 255, 0.2)',
            borderRadius: '4px',
            border: '2px solid transparent',
            backgroundClip: 'padding-box',
          },
          '&::-webkit-scrollbar-thumb:hover': {
            backgroundColor: 'rgba(255, 255, 255, 0.3)',
          },
        },
        '.apple-line-clamp-1': {
          display: '-webkit-box',
          WebkitLineClamp: '1',
          WebkitBoxOrient: 'vertical',
          overflow: 'hidden',
        },
        '.apple-line-clamp-2': {
          display: '-webkit-box',
          WebkitLineClamp: '2',
          WebkitBoxOrient: 'vertical',
          overflow: 'hidden',
        },
      })
    },
  ],
}
