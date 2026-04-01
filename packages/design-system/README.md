# EasySSH Design System

Apple-level native UI design system for the EasySSH product line (Lite, Standard, Pro).

## Overview

This design system provides a comprehensive set of design tokens, components, and utilities for building consistent, accessible, and beautiful native desktop interfaces.

## Features

- **Complete Design Tokens**: Colors, typography, spacing, shadows, borders, motion
- **Dark/Light Theme System**: Automatic system preference detection with manual override
- **RTL Support**: Full right-to-left layout support
- **Accessibility**: WCAG 2.1 AA compliant with reduced motion support
- **Tailwind Integration**: Pre-configured Tailwind CSS setup
- **TypeScript**: Full type definitions and IntelliSense support

## Installation

```bash
npm install @easyssh/design-system
# or
pnpm add @easyssh/design-system
# or
yarn add @easyssh/design-system
```

## Quick Start

### 1. Import CSS Variables

```tsx
import '@easyssh/design-system/styles';
```

### 2. Configure Tailwind

```js
// tailwind.config.ts
import designSystemConfig from '@easyssh/design-system/tailwind.config';
import { merge } from 'lodash';

export default merge(designSystemConfig, {
  // Your custom config
  content: ['./src/**/*.{ts,tsx}'],
});
```

### 3. Use the Theme Hook

```tsx
import { useTheme, initTheme } from '@easyssh/design-system/hooks';

// Initialize theme on app mount
initTheme();

function App() {
  const { theme, toggleTheme } = useTheme();

  return (
    <div data-theme={theme}>
      <button onClick={toggleTheme}>
        Toggle {theme === 'dark' ? 'Light' : 'Dark'} Mode
      </button>
    </div>
  );
}
```

## Design Tokens

### Colors

```tsx
import { tokens } from '@easyssh/design-system';

// Access colors
const primary = tokens.colors.brand[500];
const success = tokens.colors.success[500];
const terminalBg = tokens.colors.terminal.background;
```

### Typography

```tsx
import { tokens } from '@easyssh/design-system';

// Font families
const sans = tokens.typography.fontFamily.sans;  // Inter
const mono = tokens.typography.fontFamily.mono;  // JetBrains Mono

// Type styles
const headline = tokens.typography.styles.headline.large;
// { fontSize: '20px', fontWeight: '600', lineHeight: '1.4' }
```

### Spacing

```tsx
import { tokens } from '@easyssh/design-system';

// 4px grid system
const small = tokens.spacing[2];   // 8px
const medium = tokens.spacing[4];  // 16px
const large = tokens.spacing[6];   // 24px
```

## Component Specifications

The design system includes detailed specifications for core components:

- **AppShell**: Main application layout container
- **Sidebar**: Server navigation and management
- **TerminalPanel**: Multi-session terminal workspace
- **ServerCard**: Server connection cards
- **CommandPalette**: Global command search interface

See `/specs` directory for detailed component documentation.

## Tailwind Classes

### Background Colors

```html
<div class="bg-background-primary">
<div class="bg-background-secondary">
<div class="bg-background-tertiary">
<div class="bg-background-elevated">
```

### Text Colors

```html
<span class="text-foreground-primary">
<span class="text-foreground-secondary">
<span class="text-foreground-tertiary">
<span class="text-brand-500">
```

### Interactive States

```html
<button class="bg-interactive-primary text-interactive-primary-text hover:bg-interactive-primary-hover">
<button class="bg-interactive-secondary hover:bg-interactive-secondary-hover">
<button class="hover:bg-interactive-ghost-hover">
```

### Status Colors

```html
<span class="text-status-online">
<span class="text-status-offline">
<span class="text-status-connecting">
```

### Shadows

```html
<div class="shadow-sm">   <!-- Small elevation -->
<div class="shadow-md">   <!-- Medium elevation -->
<div class="shadow-lg">   <!-- Large elevation -->
<div class="shadow-card"> <!-- Card shadow -->
<div class="shadow-modal">  <!-- Modal shadow -->
```

### Animations

```html
<div class="animate-fade-in">
<div class="animate-slide-in-up">
<div class="animate-scale-in">
<div class="animate-pulse">     <!-- Connecting indicator -->
<div class="animate-spin">      <!-- Loading spinner -->
```

## Animation System

### Duration Classes

```html
<div class="duration-instant"> <!-- 50ms -->
<div class="duration-fast">    <!-- 100ms -->
<div class="duration-normal">  <!-- 200ms -->
<div class="duration-slow">    <!-- 300ms -->
```

### Easing Functions

```html
<div class="ease-ease">        <!-- Standard -->
<div class="ease-spring">      <!-- Bouncy -->
<div class="ease-smooth">      <!-- Layout changes -->
```

## Accessibility

### Reduced Motion

```css
/* Automatically applied for users who prefer reduced motion */
@media (prefers-reduced-motion: reduce) {
  /* All animations reduced to 0.01ms */
}
```

### Focus Rings

```html
<button class="focus-ring">Focusable element</button>
<button class="focus-ring-inset">Inset focus ring</button>
```

### RTL Support

```html
<div class="rtl">
  <span class="me-2">Right margin in RTL</span>
  <span class="ms-2">Left margin in RTL</span>
</div>
```

## Browser Support

- Chrome/Edge 88+
- Firefox 78+
- Safari 14+
- Electron 12+

## Contributing

Please see the main EasySSH repository for contribution guidelines.

## License

MIT © EasySSH Team
