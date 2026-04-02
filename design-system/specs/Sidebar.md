# Sidebar Component Specification

> Primary navigation for server management and workspace switching

---

## Overview

The Sidebar provides persistent access to servers, groups, recent connections, and settings. It supports multiple modes and collapsible states to maximize workspace efficiency.

---

## Anatomy

```
┌────────────────────────────┐
│  Sidebar                   │
│  ┌────────────────────────┐│
│  │ Search                 ││
│  │ 🔍 Search servers...   ││
│  └────────────────────────┘│
│                            │
│  ┌────────────────────────┐│
│  │ Quick Actions          ││
│  │ [+] [⚡] [📤]         ││
│  └────────────────────────┘│
│                            │
│  ▼ Production (3)         ││
│  ├─ 🟢 web-server-01    ▶││
│  ├─ 🟢 web-server-02    ▶││
│  └─ 🟡 db-master        ▶││
│                            │
│  ▶ Staging (2)            ││
│                            │
│  ▶ Recent                 ││
│  ├─ ⚫ prod-cache        ▶││
│  └─ ⚫ 192.168.1.50      ▶││
│                            │
│  ┌────────────────────────┐│
│  │ Footer                 ││
│  │ [⚙] [🌙] [?]          ││
│  └────────────────────────┘│
└────────────────────────────┘
```

---

## Component Interface

```typescript
interface SidebarProps {
  /** Current mode affects available sections */
  mode: 'lite' | 'standard' | 'pro';

  /** Collapsed state */
  collapsed?: boolean;
  onToggle?: () => void;

  /** Navigation items */
  groups: ServerGroup[];
  recentConnections?: Server[];
  favorites?: Server[];

  /** Team section (Pro only) */
  team?: TeamInfo;

  /** Current selection */
  selectedServerId?: string;
  selectedGroupId?: string;

  /** Callbacks */
  onServerSelect: (server: Server) => void;
  onServerConnect: (server: Server) => void;
  onGroupToggle: (groupId: string) => void;
  onSearch: (query: string) => void;

  /** Quick actions */
  onAddServer?: () => void;
  onImportConfig?: () => void;
  onOpenSettings?: () => void;

  /** Drag and drop */
  onServerMove?: (serverId: string, targetGroupId: string) => void;
}

interface ServerGroup {
  id: string;
  name: string;
  servers: Server[];
  expanded?: boolean;
  icon?: string;
  color?: string;
}

interface Server {
  id: string;
  name: string;
  host: string;
  username: string;
  port: number;
  status: 'online' | 'offline' | 'connecting' | 'unknown';
  tags?: string[];
  lastConnected?: Date;
}
```

---

## Dimensions & Spacing

| Element | Height | Width | Padding |
|---------|--------|-------|---------|
| Sidebar (expanded) | 100% | 260px | 0 |
| Sidebar (collapsed) | 100% | 48px | 0 |
| Search Box | 36px | 100% | 0 12px |
| Quick Actions Row | 40px | 100% | 8px 12px |
| Group Header | 32px | 100% | 8px 12px |
| Server Item | 36px | 100% | 8px 12px |
| Footer | 48px | 100% | 8px 12px |

---

## Visual States

### Server Item States

| State | Background | Text | Icon | Indicator |
|-------|------------|------|------|-----------|
| Default | transparent | text-secondary | text-tertiary | status dot |
| Hover | interactive-ghost-hover | text-primary | text-primary | - |
| Selected | interactive-secondary | text-primary | brand-500 | - |
| Connecting | status-warning-bg | warning-main | warning-main | pulse |
| Online | status-success-bg | success-main | success-main | solid |
| Offline | transparent | text-quaternary | text-quaternary | solid |

### Group Header States

| State | Chevron | Background |
|-------|---------|------------|
| Collapsed | ▶ (right) | transparent |
| Expanded | ▼ (down) | transparent |
| Hover | - | interactive-ghost-hover |

---

## Design Tokens

### Colors
```
--sidebar-bg: var(--easyssh-bg-secondary)
--sidebar-border: var(--easyssh-border-subtle)
--sidebar-text: var(--easyssh-text-secondary)
--sidebar-text-active: var(--easyssh-text-primary)
--sidebar-item-hover: var(--easyssh-interactive-ghost-hover)
--sidebar-item-selected: var(--easyssh-interactive-secondary)
--sidebar-group-text: var(--easyssh-text-tertiary)
```

### Typography
```
Group Header: 11px, 500 weight, 0.02em letter-spacing, uppercase
Server Name: 13px, 500 weight
Server Details: 11px, 400 weight, text-tertiary
```

### Icons
```
Size: 16px (20px in collapsed mode)
Color: Inherit from parent
Status Dot: 8px circle
```

---

## Interaction Patterns

### Server Item
```
Single Click:    Select server (show in right panel)
Double Click:    Connect to server immediately
Right Click:     Context menu (Edit, Delete, Duplicate, Copy command)
Drag:            Move to another group
Middle Click:    Open in new tab (Standard/Pro)
```

### Group Header
```
Click:           Toggle expand/collapse
Double Click:    Edit group name
Right Click:     Context menu (Rename, Delete, Change color)
Drag Handle:     Reorder groups
```

### Search Box
```
Focus:           Expand with subtle shadow
Typing:          Real-time filter with highlight
Enter:           Connect to first result
Esc:             Clear search
Cmd+K:           Focus search from anywhere
```

---

## Motion & Animation

### Expand/Collapse Group
```
Duration: 200ms
Easing: cubic-bezier(0.4, 0, 0.2, 1)
Properties: max-height, opacity
Chevron rotation: 0deg → 90deg (200ms)
```

### Item Selection
```
Duration: 150ms
Easing: ease-out
Properties: background-color, color
```

### Sidebar Collapse (Standard/Pro)
```
Duration: 300ms
Easing: cubic-bezier(0.23, 1, 0.32, 1)
Properties: width (260px → 48px)
Icon/Text fade: opacity 1 → 0 (150ms, delay 0 for text)
```

### Server Status Indicator
```
Connecting: Pulse animation
  Duration: 1.5s
  Easing: ease-in-out
  Infinite

Online/Offline: Solid color
  Transition: background-color 200ms
```

---

## Collapsed Mode

When sidebar is collapsed to 48px:

```
┌────┐
│ 🔍 │  <- Search icon (click expands)
├────┤
│ ➕ │  <- Add server
├────┤
│ ⚡ │  <- Quick connect
├────┤
│ 📁 │  <- Groups (hover for tooltip)
├────┤
│ 🖥️ │  <- Server 1 (status color)
├────┤
│ 🖥️ │  <- Server 2
├────┤
│    │
│ ⚙️ │  <- Settings
└────┘
```

### Collapsed Behavior
- Icons only, no text
- Tooltip on hover: Full server name + host
- Click expands sidebar temporarily
- Drag and drop still works

---

## Pro Mode Extensions

### Team Section
```
┌────────────────────────────┐
│ 👥 Team: Acme Corp        │  <- Header with team name
├────────────────────────────┤
│ 🏢 Shared Servers          │  <- Read-only group
│ ├─ 🟢 production-db      │
│ └─ 🟢 production-web       │
├────────────────────────────┤
│ 👤 My Servers              │  <- Personal group
│ ├─ 🟢 dev-localhost        │
│ └─ 🟡 staging-test         │
└────────────────────────────┘
```

### Team Features
- Shared server groups (read-only badges)
- Member avatars in server items
- Sync indicators (cloud icon)
- Permission badges (Admin, Dev, Viewer)

---

## Accessibility

### Keyboard Navigation
```
↑ / ↓:           Navigate between items
→:               Expand group / Open right panel
←:               Collapse group
Enter:           Select / Connect
Space:           Toggle group
Cmd+F:           Focus search
Cmd+Shift+N:     Add new server
```

### ARIA Attributes
```html
<nav role="navigation" aria-label="Server groups">
  <div role="group" aria-label="Production">
    <button aria-expanded="true" aria-controls="group-1">
    <ul id="group-1" role="list">
      <li role="listitem">
        <button aria-current="true" aria-describedby="status">
```

### Screen Reader Announcements
- "3 servers in Production group"
- "web-server-01, online, selected"
- "Connecting to database-server..."

---

## Empty States

### No Servers
```
┌────────────────────────────┐
│                            │
│     ┌─────────────┐        │
│     │   🖥️ ➕    │        │
│     └─────────────┘        │
│                            │
│   No servers yet           │
│                            │
│   Add your first server    │
│   to get started           │
│                            │
│   [+ Add Server]           │
│   [Import from SSH config] │
│                            │
└────────────────────────────┘
```

### No Search Results
```
┌────────────────────────────┐
│ 🔍 "prod"                  │
├────────────────────────────┤
│                            │
│     ┌─────────────┐        │
│     │     🔍      │        │
│     └─────────────┘        │
│                            │
│   No matches found         │
│                            │
│   Try a different          │
│   search term              │
│                            │
│   [Clear Search]           │
│                            │
└────────────────────────────┘
```

---

## Implementation Notes

### Virtualization
For 100+ servers, use virtual scrolling:
- Window size: 20 items
- Overscan: 5 items
- Estimated height: 36px per item

### Drag and Drop
```typescript
interface DragState {
  draggedItem: Server | ServerGroup;
  dragType: 'server' | 'group';
  dropTarget: string | null;
  dropPosition: 'before' | 'after' | 'inside';
}
```

### Performance
- Memoize server items
- Debounce search input (150ms)
- Lazy load group content
- CSS containment: `contain: layout style paint`
