# Animation & Motion System Specification

> Apple-quality motion design for EasySSH

---

## Philosophy

### Design Principles

1. **Purposeful**: Every animation serves a functional purpose (feedback, orientation, delight)
2. **Restrained**: Subtle, not flashy. Motion should not distract from the task
3. **Consistent**: Same easing curves and durations across the application
4. **Performant**: 60fps animations using only transform and opacity
5. **Accessible**: Respect `prefers-reduced-motion` settings

### Motion Language

| Property | Value | Usage |
|----------|-------|-------|
| **Primary Easing** | `cubic-bezier(0.4, 0, 0.2, 1)` | Standard transitions |
| **Enter Easing** | `cubic-bezier(0, 0, 0.2, 1)` | Elements appearing |
| **Exit Easing** | `cubic-bezier(0.4, 0, 1, 1)` | Elements disappearing |
| **Spring Easing** | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Playful interactions |
| **Smooth Easing** | `cubic-bezier(0.23, 1, 0.32, 1)` | Layout changes |

---

## Duration Scale

| Duration | Value | Usage |
|----------|-------|-------|
| **Instant** | 50ms | Micro-feedback (button active state) |
| **Fast** | 100ms | Hover states, small UI changes |
| **Normal** | 200ms | Standard transitions, color changes |
| **Slow** | 300ms | Layout changes, panel slides |
| **Slower** | 400ms | Complex transitions, page transitions |
| **Slowest** | 500ms | Emphasis animations, notifications |

---

## Animation Categories

### 1. Micro-interactions (0-150ms)

#### Button States
```css
/* Hover */
.button {
  transition: background-color 100ms ease,
              transform 100ms ease,
              box-shadow 100ms ease;
}

.button:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

/* Active/Press */
.button:active {
  transform: scale(0.98);
  transition-duration: 50ms;
}
```

#### Input Focus
```css
.input {
  transition: border-color 200ms ease,
              box-shadow 200ms ease;
}

.input:focus {
  border-color: var(--brand-500);
  box-shadow: 0 0 0 3px var(--brand-200);
}
```

#### Checkbox Toggle
```css
.checkbox {
  transition: background-color 100ms ease,
              border-color 100ms ease;
}

.checkbox::after {
  transition: transform 150ms spring;
}

.checkbox:checked::after {
  transform: scale(1);
}
```

#### Switch Toggle
```css
@keyframes switch-slide {
  0% { transform: translateX(0); }
  100% { transform: translateX(20px); }
}

.switch-knob {
  animation: switch-slide 200ms spring;
}
```

### 2. Component Transitions (150-300ms)

#### Sidebar Collapse/Expand
```css
.sidebar {
  transition: width 300ms smooth,
              opacity 200ms ease;
}

.sidebar.collapsed {
  width: 48px;
}

/* Text fade within sidebar */
.sidebar-text {
  transition: opacity 150ms ease;
  transition-delay: 0ms; /* Fade out immediately */
}

.sidebar.collapsed .sidebar-text {
  opacity: 0;
  pointer-events: none;
}
```

#### Panel Slide
```css
.panel {
  transition: transform 250ms smooth,
              opacity 200ms ease;
}

.panel.hidden {
  transform: translateX(100%);
  opacity: 0;
}
```

#### Card Hover Lift
```css
.card {
  transition: transform 200ms ease,
              box-shadow 200ms ease;
}

.card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-lg);
}
```

#### Modal Dialog
```css
/* Backdrop */
.modal-backdrop {
  animation: fade-in 200ms ease-out;
}

/* Content */
.modal-content {
  animation: scale-in 200ms spring;
}

@keyframes scale-in {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
```

### 3. Page/Workspace Transitions (300-500ms)

#### Workspace Switch
```css
.workspace-enter {
  animation: workspace-enter 300ms smooth;
}

.workspace-exit {
  animation: workspace-exit 200ms ease-in;
}

@keyframes workspace-enter {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes workspace-exit {
  from {
    opacity: 1;
    transform: translateY(0);
  }
  to {
    opacity: 0;
    transform: translateY(-8px);
  }
}
```

#### Tab Switch
```css
.tab-content {
  transition: opacity 150ms ease;
}

.tab-content.switching {
  opacity: 0;
}
```

### 4. Status & Feedback Animations

#### Loading/Connecting Pulse
```css
@keyframes connecting-pulse {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.5;
    transform: scale(0.95);
  }
}

.status-connecting {
  animation: connecting-pulse 1.5s ease-in-out infinite;
}
```

#### Success Checkmark
```css
@keyframes checkmark-draw {
  0% {
    stroke-dashoffset: 100;
  }
  100% {
    stroke-dashoffset: 0;
  }
}

.checkmark path {
  stroke-dasharray: 100;
  animation: checkmark-draw 300ms ease-out;
}

/* Scale pop */
@keyframes success-pop {
  0% { transform: scale(0); }
  70% { transform: scale(1.1); }
  100% { transform: scale(1); }
}

.success-icon {
  animation: success-pop 400ms spring;
}
```

#### Error Shake
```css
@keyframes error-shake {
  0%, 100% { transform: translateX(0); }
  20% { transform: translateX(-4px); }
  40% { transform: translateX(4px); }
  60% { transform: translateX(-4px); }
  80% { transform: translateX(4px); }
}

.error-shake {
  animation: error-shake 300ms ease-in-out;
}
```

#### Toast Notification
```css
@keyframes toast-enter {
  from {
    opacity: 0;
    transform: translateY(-16px) scale(0.95);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

@keyframes toast-exit {
  from {
    opacity: 1;
    transform: translateY(0);
  }
  to {
    opacity: 0;
    transform: translateY(-8px);
  }
}

.toast {
  animation: toast-enter 300ms spring;
}

.toast.exiting {
  animation: toast-exit 200ms ease-in;
}
```

### 5. Terminal Animations

#### Cursor Blink
```css
@keyframes cursor-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.terminal-cursor {
  animation: cursor-blink 1s step-end infinite;
}

/* Smooth cursor for block style */
.terminal-cursor-smooth {
  animation: cursor-blink 1s ease-in-out infinite;
}
```

#### Scroll
```css
.terminal-scroll {
  scroll-behavior: smooth;
  scroll-duration: 100ms;
}
```

#### Output Stream
```css
/* New lines appearing */
.terminal-line {
  animation: line-appear 50ms ease-out;
}

@keyframes line-appear {
  from {
    opacity: 0;
    transform: translateY(-2px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

### 6. Special Effects

#### Command Palette Open
```css
@keyframes palette-enter {
  0% {
    opacity: 0;
    transform: scale(0.95) translateY(-10px);
  }
  100% {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.command-palette {
  animation: palette-enter 200ms spring;
}
```

#### Spotlight/Search Highlight
```css
@keyframes spotlight-pulse {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.4);
  }
  50% {
    box-shadow: 0 0 0 8px rgba(59, 130, 246, 0);
  }
}

.search-highlight {
  animation: spotlight-pulse 1.5s ease-in-out;
}
```

#### Drag and Drop
```css
/* Drag start */
.dragging {
  opacity: 0.5;
  transform: scale(1.02);
  transition: transform 200ms spring,
              opacity 200ms ease;
}

/* Drop target */
.drop-target {
  background: var(--brand-100);
  border: 2px dashed var(--brand-400);
  transition: all 150ms ease;
}

/* Drop success */
@keyframes drop-success {
  0% { transform: scale(1); }
  50% { transform: scale(1.05); }
  100% { transform: scale(1); }
}

.drop-success {
  animation: drop-success 200ms spring;
}
```

---

## Stagger Animations

### List Items
```css
.list-item {
  animation: slide-in 200ms ease-out;
  animation-fill-mode: both;
}

/* Stagger delay */
.list-item:nth-child(1) { animation-delay: 0ms; }
.list-item:nth-child(2) { animation-delay: 50ms; }
.list-item:nth-child(3) { animation-delay: 100ms; }
.list-item:nth-child(4) { animation-delay: 150ms; }
/* ... and so on, or use JS to calculate */

@keyframes slide-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

### Grid Items
```css
.grid-item {
  animation: scale-fade-in 200ms ease-out;
  animation-fill-mode: both;
}

/* Row-based stagger */
.grid-item:nth-child(3n+1) { animation-delay: 0ms; }
.grid-item:nth-child(3n+2) { animation-delay: 50ms; }
.grid-item:nth-child(3n+3) { animation-delay: 100ms; }

@keyframes scale-fade-in {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
```

---

## Performance Guidelines

### Animatable Properties (60fps)

✅ **Safe to animate:**
- `transform` (translate, scale, rotate)
- `opacity`
- `filter` (sparingly, causes repaint)

❌ **Avoid animating:**
- `width`, `height` (causes layout)
- `top`, `left`, `right`, `bottom`
- `margin`, `padding`
- `border-width`
- `box-shadow` (can be expensive with large blur)

### Compositing Hints

```css
/* Promote to GPU layer for complex animations */
.animated-element {
  will-change: transform, opacity;
}

/* Remove after animation completes */
.animation-complete {
  will-change: auto;
}
```

### CSS Containment

```css
/* Isolate animation effects */
.animation-container {
  contain: layout style paint;
}
```

### Reduced Motion Support

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

### JavaScript Animation Best Practices

```typescript
// Use requestAnimationFrame
function animateValue(start: number, end: number, duration: number) {
  const startTime = performance.now();

  function update(currentTime: number) {
    const elapsed = currentTime - startTime;
    const progress = Math.min(elapsed / duration, 1);

    // Easing function
    const eased = 1 - Math.pow(1 - progress, 3); // ease-out-cubic
    const current = start + (end - start) * eased;

    element.style.transform = `translateX(${current}px)`;

    if (progress < 1) {
      requestAnimationFrame(update);
    }
  }

  requestAnimationFrame(update);
}

// Cancel animations properly
def handleUnmount() {
  if (animationRef.current) {
    cancelAnimationFrame(animationRef.current);
  }
}
```

---

## Component-Specific Animations

### Server Card

```css
/* Hover lift */
.server-card {
  transition: transform 200ms ease,
              box-shadow 200ms ease;
}

.server-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-lg);
}

/* Selection pulse */
@keyframes selection-pulse {
  0% { box-shadow: 0 0 0 0 var(--brand-300); }
  100% { box-shadow: 0 0 0 8px transparent; }
}

.server-card.selected {
  animation: selection-pulse 400ms ease-out;
}

/* Status change */
.server-card.status-changing .status-dot {
  animation: status-blink 300ms ease 2;
}

@keyframes status-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}
```

### Terminal Panel

```css
/* Pane split */
.terminal-pane {
  transition: flex 300ms smooth;
}

/* Tab switch */
.terminal-tab-content {
  transition: opacity 150ms ease;
}

/* Resize handle */
.resize-handle {
  transition: background-color 100ms ease,
              width 100ms ease;
}

.resize-handle:hover,
.resize-handle.active {
  background-color: var(--brand-500);
  width: 4px;
}
```

### Command Palette

```css
/* Modal backdrop */
.palette-backdrop {
  animation: fade-in 200ms ease-out;
}

/* Content entrance */
.palette-content {
  animation: scale-in-up 200ms spring;
}

/* Result items */
.palette-item {
  transition: background-color 100ms ease;
}

/* Selected item indicator */
.palette-item.selected::before {
  animation: slide-in-left 150ms ease-out;
}

@keyframes slide-in-left {
  from { transform: scaleY(0); }
  to { transform: scaleY(1); }
}
```

---

## Animation Keyframe Library

```css
/* Essential keyframes - included in theme.css */

@keyframes fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes fade-out {
  from { opacity: 1; }
  to { opacity: 0; }
}

@keyframes slide-in-up {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes slide-in-down {
  from {
    opacity: 0;
    transform: translateY(-8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes slide-in-left {
  from {
    opacity: 0;
    transform: translateX(16px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

@keyframes slide-in-right {
  from {
    opacity: 0;
    transform: translateX(-16px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

@keyframes scale-in {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes scale-out {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.95);
  }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes bounce {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-4px); }
}

@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

/* Utility classes */
.animate-fade-in {
  animation: fade-in var(--duration-normal) ease-out;
}

.animate-fade-out {
  animation: fade-out var(--duration-fast) ease-in;
}

.animate-slide-in-up {
  animation: slide-in-up var(--duration-slow) smooth;
}

.animate-scale-in {
  animation: scale-in var(--duration-normal) spring;
}

.animate-pulse {
  animation: pulse 2s ease-in-out infinite;
}

.animate-spin {
  animation: spin 1s linear infinite;
}

.animate-bounce {
  animation: bounce 1s ease-in-out;
}
```

---

## Usage Guidelines

### Do's

✅ Use 200ms for hover states
✅ Use 300ms for layout changes
✅ Add subtle transform to hover effects
✅ Respect reduced motion preferences
✅ Test animations at 60fps
✅ Use spring easing for playful elements
✅ Provide immediate visual feedback on click

### Don'ts

❌ Animate layout properties (width, height, top, left)
❌ Use animations longer than 500ms
❌ Chain too many animations
❌ Animate multiple properties simultaneously
❌ Forget to handle reduced motion
❌ Use bounce/spring for serious actions (delete, error)
❌ Block user interaction during animations

---

## Testing

### Visual Regression
```typescript
// Screenshot tests for animations
test('sidebar collapse animation', async () => {
  await page.click('[data-testid="sidebar-toggle"]');

  // Capture mid-animation
  await page.waitForTimeout(150);
  await expect(page).toHaveScreenshot('sidebar-mid-collapse');

  // Capture completed
  await page.waitForTimeout(300);
  await expect(page).toHaveScreenshot('sidebar-collapsed');
});
```

### Performance Testing
```typescript
test('animation maintains 60fps', async () => {
  const metrics = await page.evaluate(() => {
    return new Promise((resolve) => {
      let frames = 0;
      const startTime = performance.now();

      function countFrames() {
        frames++;
        if (performance.now() - startTime < 1000) {
          requestAnimationFrame(countFrames);
        } else {
          resolve(frames);
        }
      }

      // Trigger animation
      document.querySelector('.sidebar').classList.toggle('collapsed');
      requestAnimationFrame(countFrames);
    });
  });

  expect(metrics).toBeGreaterThan(55); // Allow some variance
});
```
