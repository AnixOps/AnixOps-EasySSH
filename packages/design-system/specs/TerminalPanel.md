# TerminalPanel Component Specification

> Embedded terminal workspace for Standard and Pro versions

---

## Overview

The TerminalPanel provides a multi-session terminal interface with xterm.js rendering, supporting tabs, split panes, and session management. This component is only available in Standard and Pro versions.

---

## Anatomy

```
┌─────────────────────────────────────────────────────────────────────────┐
│ TerminalPanel                                                           │
│ ┌─────────────────────────────────────────────────────────────────────┐ │
│ │ Tab Bar (36px)                                                     │ │
│ │ ┌────────┬────────┬────────┬────────────────────────┬────────────┐ │ │
│ │ │ 🖥️ web-│ 🖥️ db- │ +      │                        │ 🔍  ⋮  │ │ │
│ │ │   01 ✕ │ master │        │                        │            │ │ │
│ │ └────────┴────────┴────────┴────────────────────────┴────────────┘ │ │
│ ├─────────────────────────────────────────────────────────────────────┤ │
│ │ Split Container                                                      │ │
│ │ ┌─────────────────────┬─────────────────────┐                      │ │
│ │ │                     │                     │                      │ │
│ │ │   Terminal Pane 1   │   Terminal Pane 2   │  (Vertical split)    │ │
│ │ │   ┌───────────────┐ │   ┌───────────────┐ │                      │ │
│ │ │   │$ ls -la       │ │   │$ htop         │ │                      │ │
│ │ │   │drwxr-xr-x ... │ │   │  CPU ▓▓▓▓░░   │ │                      │ │
│ │ │   │-rw-r--r-- ... │ │   │  Mem ▓▓▓░░░   │ │                      │ │
│ │ │   │               │ │   │               │ │                      │ │
│ │ │   │_              │ │   │_              │ │                      │ │
│ │ │   └───────────────┘ │   └───────────┘ │ │                      │ │
│ │ │                     │                     │                      │ │
│ │ │ [Status: Connected] │ [Status: Connecting]│                      │ │
│ │ ├─────────────────────┴─────────────────────┤                      │ │
│ │ │           Terminal Pane 3                 │  (Horizontal)        │ │
│ │ │           ┌─────────────────────────┐     │                      │ │
│ │ │           │$ tail -f /var/log/app.log│     │                      │ │
│ │ │           │[INFO] Server started     │     │                      │ │
│ │ │           │...                       │     │                      │ │
│ │ │           └─────────────────────────┘     │                      │ │
│ │ └───────────────────────────────────────────┘                      │ │
│ ├─────────────────────────────────────────────────────────────────────┤ │
│ │ Quick Actions (28px, optional)                                     │ │
│ │ [📋 Paste] [🔍 Find] [📝 Save] [⚡ Commands] [📤 Upload]            │ │
│ └─────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Component Interface

```typescript
interface TerminalPanelProps {
  /** Active sessions */
  sessions: TerminalSession[];
  activeSessionId: string;

  /** Tab management */
  onTabSelect: (sessionId: string) => void;
  onTabClose: (sessionId: string) => void;
  onTabReorder: (sessionIds: string[]) => void;

  /** Split management */
  layout: PaneLayout;
  onLayoutChange: (layout: PaneLayout) => void;
  onSplitVertical: (sessionId: string) => void;
  onSplitHorizontal: (sessionId: string) => void;
  onPaneClose: (paneId: string) => void;

  /** Terminal options */
  options: TerminalOptions;
  onOptionsChange: (options: TerminalOptions) => void;

  /** Actions */
  onSearch: (query: string) => void;
  onClear: () => void;
  onReconnect: (sessionId: string) => void;
}

interface TerminalSession {
  id: string;
  serverId: string;
  serverName: string;
  status: 'connecting' | 'connected' | 'disconnected' | 'error';
  paneId: string;
  createdAt: Date;
  lastActivity: Date;
  title?: string;
  hasUnviewedOutput?: boolean;
}

interface PaneLayout {
  type: 'leaf' | 'split';
  direction?: 'horizontal' | 'vertical';
  splitRatio?: number;
  paneId?: string;
  children?: PaneLayout[];
}

interface TerminalOptions {
  fontSize: number;
  fontFamily: string;
  theme: 'dark' | 'light' | 'custom';
  cursorStyle: 'block' | 'line' | 'bar';
  cursorBlink: boolean;
  scrollback: number;
  wordSeparator: string;
}
```

---

## Dimensions & Spacing

| Element | Height | Padding | Notes |
|---------|--------|---------|-------|
| Tab Bar | 36px | 0 8px | Scrollable if overflow |
| Tab Item | 32px | 0 12px | Close button on hover |
| Terminal Pane | auto | 8px 12px | Flexible height |
| Split Handle | 4px | 0 | Invisible hit area |
| Quick Actions | 28px | 0 12px | Optional, collapsible |
| Status Line | 20px | 0 12px | Per pane |

---

## Design Tokens

### Terminal Colors
```
/* Base16-inspired color scheme */
--terminal-bg: #1E1E1E
--terminal-fg: #DCDCDC
--terminal-cursor: #528BFF
--terminal-selection: #264F78
--terminal-black: #1E1E1E
--terminal-red: #E06C75
--terminal-green: #98C379
--terminal-yellow: #E5C07B
--terminal-blue: #61AFEF
--terminal-magenta: #C678DD
--terminal-cyan: #56B6C2
--terminal-white: #DCDCDC
--terminal-bright-black: #5C6370
--terminal-bright-red: #FF6B7A
--terminal-bright-green: #B5E08D
--terminal-bright-yellow: #F0D58A
--terminal-bright-blue: #7BC3FF
--terminal-bright-magenta: #D78FE6
--terminal-bright-cyan: #6ED4E0
--terminal-bright-white: #FFFFFF
```

### Panel Chrome
```
--terminal-panel-bg: var(--easyssh-bg-primary)
--terminal-panel-border: var(--easyssh-border-subtle)
--terminal-tab-bar-bg: var(--easyssh-bg-secondary)
--terminal-tab-bg: transparent
--terminal-tab-bg-active: var(--easyssh-bg-primary)
--terminal-tab-text: var(--easyssh-text-tertiary)
--terminal-tab-text-active: var(--easyssh-text-primary)
--terminal-status-line-bg: var(--easyssh-bg-secondary)
--terminal-status-line-text: var(--easyssh-text-tertiary)
```

---

## Tab Bar

### Tab States

| State | Background | Text | Border |
|-------|------------|------|--------|
| Inactive | transparent | text-tertiary | none |
| Hover | interactive-ghost-hover | text-secondary | none |
| Active | bg-primary | text-primary | top 2px brand-500 |
| Unviewed Output | bg-primary | text-primary | left 3px brand-500 |
| Error | bg-danger | danger-main | none |
| Disconnected | bg-quaternary | text-quaternary | none |

### Tab Content
```
┌────────────────────────────┐
│ 🖥️ Server Name    ✕ │
└────────────────────────────┘
```

- Icon: Platform indicator or custom
- Title: Server name (truncated with ellipsis)
- Status dot: Small indicator before title
- Close button: × appears on hover

### Tab Actions
```
Right Click Menu:
├─ Rename Tab
├─ Duplicate Tab
├─ Split →
│  ├─ Vertically
│  └─ Horizontally
├─ Close Tab        (Cmd+W)
├─ Close Other Tabs
├─ Close All Tabs
└─ Copy Tab URL
```

---

## Terminal Pane

### Pane Structure
```
┌─────────────────────────────────┐
│ Terminal (xterm.js canvas/DOM) │
├─────────────────────────────────┤
│ Status Line                     │
│ 🟢 user@host • 80×24 • UTF-8   │
└─────────────────────────────────┘
```

### Status Line Content
```
Left side:
  🟢 Connected / 🟡 Connecting / 🔴 Disconnected / ⚠️ Error
  user@hostname (if available)

Right side:
  80×24 (cols × rows)
  UTF-8 (encoding)
  📋 (if text selected)
```

### Pane Overlay States

#### Connecting
```
┌─────────────────────────────────┐
│                                 │
│        ⚡ Connecting...          │
│                                 │
│   Establishing SSH connection   │
│   to production-web-01...       │
│                                 │
│        [Cancel]                 │
│                                 │
└─────────────────────────────────┘
```

#### Disconnected
```
┌─────────────────────────────────┐
│                                 │
│         🔌 Disconnected         │
│                                 │
│   Connection closed by remote   │
│                                 │
│     [Reconnect]  [Close]        │
│                                 │
└─────────────────────────────────┘
```

#### Error
```
┌─────────────────────────────────┐
│                                 │
│      ⚠️ Connection Failed       │
│                                 │
│   Authentication failed:          │
│   Permission denied (publickey) │
│                                 │
│   [Retry]  [Edit Config]        │
│                                 │
└─────────────────────────────────┘
```

---

## Split Layout System

### Layout Types

#### Single Pane
```
┌─────────────────────┐
│                     │
│      Pane 1         │
│                     │
└─────────────────────┘
```

#### Vertical Split
```
┌──────────┬──────────┐
│          │          │
│ Pane 1   │ Pane 2   │
│          │          │
└──────────┴──────────┘
```

#### Horizontal Split
```
┌─────────────────────┐
│      Pane 1         │
├─────────────────────┤
│      Pane 2         │
└─────────────────────┘
```

#### Complex Layout
```
┌──────────┬──────────┐
│          │ Pane 2   │
│ Pane 1   ├──────────┤
│          │ Pane 3   │
└──────────┴──────────┘
```

### Resizing
- Drag handle between panes
- Minimum pane size: 200px × 100px
- Resize cursor: col-resize / row-resize
- Snap to grid: 10px increments

### Keyboard Shortcuts
```
Cmd+D:           Split vertical
Cmd+Shift+D:     Split horizontal
Cmd+W:           Close current pane
Cmd+Option+→:    Focus next pane
Cmd+Option+←:    Focus previous pane
Cmd+Option+↑:    Focus pane above
Cmd+Option+↓:    Focus pane below
```

---

## Context Menu

### Terminal Context Menu
```
├─ Copy              (Cmd+C)
├─ Paste             (Cmd+V)
├─ Paste as...      →
├─ Select All        (Cmd+A)
├─ Find...           (Cmd+F)
├──────────────────────
├─ Clear Terminal    (Cmd+K)
├─ Reset Terminal
├──────────────────────
├─ Send Command     →
│  ├─ Ctrl+C
│  ├─ Ctrl+D
│  ├─ Ctrl+L
│  └─ Ctrl+Z
├──────────────────────
├─ Split →
│  ├─ Vertically
│  └─ Horizontally
├──────────────────────
└─ Terminal Settings...
```

---

## Search Integration

### Find Bar
```
┌────────────────────────────────────────────────────────┐
│ Find: [search term                    ] [⬆] [⬇] [✕] │
│       Case sensitive [ ]  Whole word [ ]  Regex [ ]      │
│       3/12 matches                                       │
└────────────────────────────────────────────────────────┘
```

### Search Results
- Highlight all matches with yellow background
- Current match: orange border
- Navigate with arrows or Enter/Shift+Enter
- Close with Esc

---

## Motion & Animation

### Tab Switching
```
Duration: 150ms
Easing: ease-out
Effect: Cross-fade between terminals
```

### Pane Split
```
Duration: 300ms
Easing: cubic-bezier(0.23, 1, 0.32, 1)
Effect: Smooth resize with content fade-in
```

### Connection State Transition
```
Fade overlay: 200ms
Pulse connecting: 1.5s infinite
Error shake: 300ms
```

---

## Accessibility

### Keyboard Navigation
- All panes focusable with Tab
- Focus visible with brand-colored ring
- Screen reader: "Terminal connected to web-server-01, 80 columns by 24 rows"

### ARIA
```html
<div role="region" aria-label="Terminal panel">
  <div role="tablist" aria-label="Terminal sessions">
    <button role="tab" aria-selected="true" aria-controls="term-1">
  <div id="term-1" role="tabpanel" aria-label="Terminal content">
    <canvas aria-label="Terminal emulation, xterm.js">
```

### High Contrast Mode
- Increase terminal contrast
- Thicker cursor (3px)
- Bold text for selection

---

## Performance

### Rendering
- WebGL renderer for 60fps with large output
- DOM renderer fallback for accessibility
- Virtual scrollback buffer (configurable)

### Optimization
```typescript
interface PerformanceConfig {
  webgl: boolean;
  scrollback: number;      // default: 10000
  repaintInterval: number; // default: 16ms
  screenReaderMode: boolean;
}
```

### Memory Management
- Limit scrollback lines
- Dispose inactive session terminals
- Compress scrollback data after 5min inactivity
