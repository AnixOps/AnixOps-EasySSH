# AppShell Component Specification

> Main application container for EasySSH workspace layout

---

## Overview

The AppShell provides the foundational layout structure for all EasySSH versions (Lite, Standard, Pro). It manages the global navigation, workspace areas, and responsive behavior.

---

## Anatomy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  AppShell                                                                   │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │ Header (48px)                                                          │ │
│  │  ├─ Logo/Brand              ├─ Search ├─ Actions ├─ User             │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│  ┌──────────────────┬──────────────────────────────┬────────────────────┐ │
│  │                  │                              │                    │ │
│  │  Sidebar         │      Main Content Area       │   Right Panel      │ │
│  │  (260px/48px)    │      (flex: 1)               │   (320px, optional)│ │
│  │                  │                              │                    │ │
│  │  ├─ Server List  │      ├─ Lite: Server Cards   │   ├─ Properties    │ │
│  │  ├─ Groups       │      ├─ Standard: Terminal   │   ├─ Status        │ │
│  │  ├─ Sessions     │      ├─ Pro: Team Console    │   ├─ SFTP          │ │
│  │  └─ Settings     │                              │                    │ │
│  │                  │                              │                    │ │
│  └──────────────────┴──────────────────────────────┴────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────────────────┐ │
│  │ Status Bar (optional, 24px)                                            │ │
│  │  ├─ Connection Status ├─ Sync Status ├─ Version                        │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Component Interface

```typescript
interface AppShellProps {
  /** Current product mode */
  mode: 'lite' | 'standard' | 'pro';

  /** Current workspace view */
  workspace: 'vault' | 'terminal' | 'team' | 'settings';

  /** Sidebar expansion state */
  sidebarExpanded?: boolean;
  onSidebarToggle?: () => void;

  /** Right panel visibility */
  rightPanelOpen?: boolean;
  onRightPanelToggle?: () => void;

  /** Content for each area */
  headerContent?: React.ReactNode;
  sidebarContent: React.ReactNode;
  mainContent: React.ReactNode;
  rightPanelContent?: React.ReactNode;
  statusBarContent?: React.ReactNode;

  /** Responsive breakpoint */
  isMobile?: boolean;

  /** Theme */
  theme?: 'light' | 'dark';
}
```

---

## Dimensions & Spacing

| Element | Height | Width | Padding |
|---------|--------|-------|---------|
| Header | 48px | 100% | 0 16px |
| Sidebar (expanded) | calc(100vh - 48px) | 260px | 8px 0 |
| Sidebar (collapsed) | calc(100vh - 48px) | 48px | 8px 0 |
| Main Content | calc(100vh - 48px) | flex: 1 | 0 |
| Right Panel | calc(100vh - 48px) | 320px | 16px |
| Status Bar | 24px | 100% | 0 16px |

---

## Design Tokens

### Colors

| Token | Light Mode | Dark Mode | Usage |
|-------|------------|-----------|-------|
| `--app-bg` | neutral-0 | neutral-950 | App background |
| `--header-bg` | neutral-0 | neutral-950 | Header background |
| `--header-border` | neutral-200 | neutral-800 | Header bottom border |
| `--sidebar-bg` | neutral-50 | neutral-900 | Sidebar background |
| `--sidebar-border` | neutral-200 | neutral-800 | Sidebar right border |
| `--main-bg` | neutral-0 | neutral-950 | Main content background |
| `--panel-bg` | neutral-50 | neutral-900 | Right panel background |

### Z-Index

| Element | Z-Index |
|---------|---------|
| Header | 100 |
| Sidebar | 90 |
| Main Content | 0 |
| Right Panel | 80 |
| Modal Overlay | 500 |
| Command Palette | 900 |

---

## Variants

### Lite Mode
```
┌─────────────────────────────────────────────────┐
│ Header                                          │
├──────────────────┬──────────────────────────────┤
│ Sidebar          │  Server Cards Grid           │
│ (Server list)    │  ┌─────┐ ┌─────┐ ┌─────┐    │
│                  │  │Card │ │Card │ │Card │    │
│                  │  └─────┘ └─────┘ └─────┘    │
└──────────────────┴──────────────────────────────┘
```

### Standard Mode
```
┌─────────────────────────────────────────────────────────┐
│ Header                                                  │
├──────────────────┬───────────────────────┬──────────────┤
│ Sidebar          │ Terminal Workspace    │ Details      │
│                  │ ┌───────────────────┐ │              │
│                  │ │ Tab 1 │ Tab 2 │... │ │  Properties  │
│                  │ ├───────────────────┤ │              │
│                  │ │                   │ │  ├─ Status   │
│                  │ │   Terminal Area   │ │  ├─ CPU      │
│                  │ │   (xterm.js)      │ │  ├─ Memory   │
│                  │ │                   │ │  └─ SFTP     │
│                  │ │                   │ │              │
│                  │ └───────────────────┘ │              │
└──────────────────┴───────────────────────┴──────────────┘
```

### Pro Mode
```
┌─────────────────────────────────────────────────────────┐
│ Header                          [Team: Acme Corp]         │
├──────────────────┬───────────────────────┬──────────────┤
│ Sidebar          │ Team Console          │ Audit Log    │
│                  │ ┌───────────────────┐ │              │
│                  │ │ Members           │ │  ├─ Sessions │
│                  │ │ ├─ Alice (Admin)  │ │  ├─ Commands │
│                  │ │ ├─ Bob (Dev)      │ │  └─ Files    │
│                  │ │ └─ Carol (Viewer) │ │              │
│                  │ ├───────────────────┤ │              │
│                  │ │ Shared Resources  │ │              │
│                  │ └───────────────────┘ │              │
└──────────────────┴───────────────────────┴──────────────┘
```

---

## Motion & Animation

### Sidebar Collapse/Expand
```
Duration: 300ms
Easing: cubic-bezier(0.23, 1, 0.32, 1)  // Smooth
Properties: width, opacity
```

### Right Panel Slide
```
Duration: 250ms
Easing: cubic-bezier(0.4, 0, 0.2, 1)
Properties: transform (translateX), opacity
```

### Workspace Transition
```
Duration: 200ms
Easing: ease-out
Properties: opacity, transform (subtle scale)
```

---

## Accessibility

### Keyboard Navigation
- `F6` / `Cmd+1-4`: Navigate between regions (header, sidebar, main, panel)
- `Cmd+B`: Toggle sidebar
- `Cmd+J`: Toggle right panel
- `Esc`: Close panels / unfocus terminal

### Screen Reader
- Main landmarks: `role="banner"`, `role="navigation"`, `role="main"`, `role="complementary"`
- Sidebar: `aria-label="Server navigation"`
- Right Panel: `aria-label="Details panel"`

### Focus Management
- Visible focus ring on all interactive elements
- Tab order: Header → Sidebar → Main → Right Panel
- Focus trap in modals

---

## Responsive Behavior

| Breakpoint | Sidebar | Right Panel |
|------------|---------|-------------|
| < 768px (mobile) | Drawer overlay (slide in) | Hidden / Full screen modal |
| 768px - 1024px (tablet) | Collapsed icons only | Collapsible |
| > 1024px (desktop) | Full expanded | Expanded |

---

## Implementation Notes

### Performance
- Use CSS Grid for main layout (better than Flex for complex layouts)
- `contain: layout style paint` on sidebar and panels
- Lazy load right panel content

### State Management
```typescript
interface AppShellState {
  sidebarExpanded: boolean;
  rightPanelOpen: boolean;
  activeWorkspace: WorkspaceMode;
  isMobile: boolean;
}
```

### CSS Structure
```css
.app-shell {
  display: grid;
  grid-template-areas:
    "header header header"
    "sidebar main panel";
  grid-template-rows: 48px 1fr;
  grid-template-columns: 260px 1fr 320px;
  height: 100vh;
  overflow: hidden;
}
```
