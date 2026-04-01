# ServerCard Component Specification

> Compact server connection card for Lite version

---

## Overview

The ServerCard displays server connection information in a compact, actionable format. It's the primary interaction element for EasySSH Lite, designed for quick scanning and one-click connections.

---

## Anatomy

```
┌─────────────────────────────────────┐
│ ServerCard                          │
│ ┌─────────────────────────────────┐ │
│ │ ┌─────┐                         │ │
│ │ │ 🖥️  │  Web Server 01      🟢  │ │  <- Header
│ │ └─────┘  ├─ Production        ⋮  │ │
│ │          └─ admin@192.168.1.10  │ │
│ ├─────────────────────────────────┤ │
│ │ 🔑 SSH Agent         [Connect ▶]│ │  <- Auth & Action
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘

Alternative Compact View:
┌───────────────────────────────┐
│ 🖥️ │ Web Server 01    │ 🟢 │ ▶│
│    │ admin@192.168.1.10│    │  │
└───────────────────────────────┘
```

---

## Component Interface

```typescript
interface ServerCardProps {
  /** Server data */
  server: {
    id: string;
    name: string;
    host: string;
    port: number;
    username: string;
    authType: 'password' | 'key' | 'agent';
    keyName?: string;
    group?: string;
    tags?: string[];
    notes?: string;
    lastConnected?: Date;
  };

  /** Connection status */
  status: 'online' | 'offline' | 'connecting' | 'unknown';

  /** Display variants */
  variant: 'default' | 'compact' | 'list';

  /** Selection state */
  selected?: boolean;
  onSelect?: () => void;

  /** Actions */
  onConnect: () => void;
  onEdit?: () => void;
  onDuplicate?: () => void;
  onDelete?: () => void;
  onCopyCommand?: () => void;

  /** Drag and drop */
  draggable?: boolean;
  onDragStart?: () => void;
  onDragEnd?: () => void;
}
```

---

## Dimensions & Spacing

### Default Variant
| Property | Value |
|----------|-------|
| Width | 280px (min) - 360px (max) |
| Height | auto (content-based) |
| Padding | 16px |
| Gap (internal) | 12px |
| Border radius | 8px |

### Compact Variant
| Property | Value |
|----------|-------|
| Width | 100% (flexible) |
| Height | 60px |
| Padding | 12px 16px |
| Border radius | 6px |

### List Variant
| Property | Value |
|----------|-------|
| Width | 100% |
| Height | 48px |
| Padding | 8px 12px |
| Border radius | 0 (in list context) |

---

## Visual States

### Default State
```
Background: bg-elevated (white/dark)
Border: 1px border-subtle
Shadow: shadow-sm
Status indicator: Right side, 8px circle
```

### Hover State
```
Background: bg-elevated
Border: 1px border-default
Shadow: shadow-md
Cursor: pointer
Transform: translateY(-1px)  // Subtle lift
```

### Selected State
```
Background: interactive-secondary
Border: 2px brand-500
Shadow: none
Ring: focus-ring
```

### Connecting State
```
Background: warning-bg (subtle)
Border: 1px warning-main
Status: Pulsing amber dot
Button: "Connecting..." with spinner
```

### Status Indicators

| Status | Color | Icon | Animation |
|--------|-------|------|-----------|
| Online | success-500 | Solid circle | None |
| Offline | danger-500 | Solid circle | None |
| Connecting | warning-500 | Circle | Pulse (1.5s) |
| Unknown | neutral-400 | Dashed circle | None |
| Maintenance | purple-500 | Wrench | None |

---

## Design Tokens

### Colors
```
--card-bg: var(--easyssh-bg-elevated)
--card-bg-hover: var(--easyssh-bg-elevated)
--card-bg-selected: var(--easyssh-interactive-secondary)
--card-border: var(--easyssh-border-subtle)
--card-border-hover: var(--easyssh-border-default)
--card-border-selected: var(--easyssh-primary-500)
--card-shadow: var(--easyssh-shadow-sm)
--card-shadow-hover: var(--easyssh-shadow-md)
```

### Typography
```
Server Name: 14px, font-weight 600, text-primary
Group Tag: 11px, font-weight 500, brand-500, uppercase
Connection String: 12px, font-weight 400, text-secondary
Auth Method: 12px, font-weight 400, text-tertiary
```

### Spacing
```
Internal padding: 16px
Icon-to-text gap: 12px
Text-to-status gap: 12px
Header-to-action gap: 12px
Action button height: 32px
```

---

## Variants

### Default (Card)
```
┌─────────────────────────────────┐
│  ┌──┐                          │
│  │🖥️│  Server Name        🟢  │
│  └──┘  ├─ Group                 │
│        └─ user@host:port        │
├─────────────────────────────────┤
│  🔑 Auth Method    [Connect ▶]  │
└─────────────────────────────────┘
```

### Compact
```
┌─────────────────────────────────┐
│ 🖥️ Server Name              🟢 ▶ │
│    user@host:port               │
└─────────────────────────────────┘
```

### List Item
```
│ 🖥️ │ Server Name    │ Group │ 🟢 │ ▶ │
│    │ user@host:port │       │    │    │
```

---

## Content Structure

### Header Section
- Platform icon (16px)
- Server name (truncated with ellipsis)
- Group tag (if applicable)
- Status indicator
- Context menu trigger (⋮)

### Info Section
- Connection string: `username@hostname:port`
- Tags (optional, max 2)
- Last connected (optional, relative time)

### Action Section
- Authentication badge (key icon + method)
- Connect button (primary CTA)

---

## Interaction Patterns

### Click Behaviors

| Action | Result |
|--------|--------|
| Single Click | Select card, show details in side panel |
| Double Click | Immediate connect |
| Right Click | Context menu |
| Long Press (mobile) | Context menu |

### Context Menu
```
├─ Connect          (Enter)
├─ Edit             (Cmd+E)
├─ Duplicate          (Cmd+D)
├─ Copy SSH Command (Cmd+Shift+C)
├────────────────────────
├─ Move to Group   →
│  ├─ Production
│  ├─ Staging
│  └─ Create New...
├────────────────────────
└─ Delete           (Cmd+Delete)
   └─ Confirm deletion
```

### Connect Button States

| State | Style | Text |
|-------|-------|------|
| Default | Primary button | "Connect" |
| Hover | Primary + glow | "Connect" |
| Connecting | Secondary + spinner | "Connecting..." |
| Connected | Success | "Connected" |
| Error | Danger | "Retry" |

---

## Group Organization

### Within Group Section
```
▼ Production (4)
┌─────────────────────┐
│ 🖥️ web-01      🟢 │
│    admin@10.0.1.1   │
├─────────────────────┤
│ 🖥️ web-02      🟢 │
│    admin@10.0.1.2   │
├─────────────────────┤
│ 🖥️ db-master   🟡 │
│    dba@10.0.1.10   │
└─────────────────────┘
```

### Drag and Drop
- Visual indicator: Dashed border during drag
- Drop target: Highlighted background
- Reorder: Within group or between groups

---

## Empty & Loading States

### Loading Skeleton
```
┌─────────────────────────────────┐
│ ┌──┐  ┌──────────────┐  ┌───┐  │
│ │░░│  │░░░░░░░░░░░░░░│  │░░░│  │
│ └──┘  ├──────────────┤  └───┘  │
│       └──────────────┘         │
├─────────────────────────────────┤
│ ┌───────────────┐  ┌───────────┐ │
│ │░░░░░░░░░░░░░░░│  │░░░░░░░░░░░│ │
│ └───────────────┘  └───────────┘ │
└─────────────────────────────────┘
```

### Connection Error State
```
┌─────────────────────────────────┐
│ 🖥️ Server Name            ⚠️     │
│    Connection failed            │
├─────────────────────────────────┤
│  🔴 Network timeout             │
│            [Retry]              │
└─────────────────────────────────┘
```

---

## Motion & Animation

### Hover Effect
```
Duration: 200ms
Easing: ease-out
Properties:
  - box-shadow: shadow-sm → shadow-md
  - transform: translateY(0) → translateY(-1px)
  - border-color: border-subtle → border-default
```

### Selection
```
Duration: 150ms
Easing: ease-out
Properties:
  - background: transparent → interactive-secondary
  - border-width: 1px → 2px
  - border-color: border-subtle → brand-500
```

### Connecting Pulse
```
@keyframes connecting-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
Duration: 1.5s
Easing: ease-in-out
Infinite loop
```

---

## Accessibility

### Keyboard Navigation
```
Tab:        Focus next card
Shift+Tab:  Focus previous card
Enter:      Connect to focused server
Space:      Select card
Cmd+E:      Edit server
Cmd+Delete: Delete server (with confirmation)
```

### ARIA Attributes
```html
<div role="listitem"
     aria-label="Web Server 01, Production, online"
     aria-selected="false"
     tabindex="0">
  <button aria-label="Connect to Web Server 01">
</div>
```

### Screen Reader
- Full announcement: "Web Server 01, Production group, user admin at 192.168.1.10 port 22, online, SSH agent authentication"
- Status change: "Connecting to Web Server 01"
- Connection result: "Connected to Web Server 01" or "Connection failed: authentication error"

### Focus Indicators
- Visible 2px focus ring with brand color
- Focused card has elevated shadow
- Tab order follows visual layout

---

## Responsive Behavior

| Container Width | Layout |
|-----------------|--------|
| < 400px | Single column, compact variant |
| 400px - 800px | 2 columns, default variant |
| > 800px | 3-4 columns, default variant |

---

## Implementation Notes

### Performance
- Use CSS containment: `contain: layout style`
- Lazy load status checks
- Debounce hover effects
- Virtual scroll for > 50 cards

### Security
- Never display actual passwords
- Mask sensitive info in screenshots
- Clear clipboard after copy command
