# CommandPalette Component Specification

> Global command search and quick action interface

---

## Overview

The CommandPalette provides universal access to all application features through keyboard-driven search. Inspired by VS Code and Raycast, it enables power users to work efficiently without mouse interaction.

---

## Anatomy

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Command Palette (Modal Overlay)                                         │
│ ┌─────────────────────────────────────────────────────────────────────┐ │
│ │ Input Section (56px)                                                │ │
│ │ ┌─────────────────────────────────────────────────────────────────┐ │ │
│ │ │ ⌘ │ Search commands or servers...              │ 🔍 │ ✕ │    │ │ │
│ │ └─────────────────────────────────────────────────────────────────┘ │ │
│ ├─────────────────────────────────────────────────────────────────────┤ │
│ │ Results Section (max 424px)                                         │ │
│ │ ┌─────────────────────────────────────────────────────────────────┐ │ │
│ │ │ 🕐 Recent                                                       │ │ │
│ │ │    🖥️  Connect to web-server-01              Enter            │ │ │
│ │ │    📁  Open Production group                  →               │ │ │
│ │ │                                                               │ │ │
│ │ │ 🖥️ Servers (3)                                                │ │ │
│ │ │    🟢 web-server-01       admin@10.0.0.1                      │ │ │
│ │ │    🟢 web-server-02       admin@10.0.0.2                      │ │ │
│ │ │    🟡 db-master           dba@10.0.0.10                       │ │ │
│ │ │                                                               │ │ │
│ │ │ ⚡ Quick Actions                                              │ │ │
│ │ │    ➕ Add New Server...                      Cmd+Shift+N     │ │ │
│ │ │    📤 Import from SSH config                 Cmd+Shift+I     │ │ │
│ │ │    ⚙️  Open Settings                         Cmd+,           │ │ │
│ │ │                                                               │ │ │
│ │ │ 🔧 Commands                                                    │ │ │
│ │ │    🌙 Toggle Dark Mode                         Cmd+Shift+L   │ │ │
│ │ │    📋 Copy Last Command                        Cmd+Shift+C   │ │ │
│ │ │    🔍 Find in Terminal                         Cmd+F         │ │ │
│ │ │                                                               │ │ │
│ │ ├─────────────────────────────────────────────────────────────────┤ │ │
│ │ │ Footer (24px)                                                   │ │ │
│ │ │  ↑↓ Navigate  │  ↵ Select  │  ⌘+K Open  │  ⌘+Number Quick    │ │ │
│ │ └─────────────────────────────────────────────────────────────────┘ │ │
│ └─────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Component Interface

```typescript
interface CommandPaletteProps {
  /** Visibility */
  isOpen: boolean;
  onClose: () => void;

  /** Search state */
  searchQuery: string;
  onSearchChange: (query: string) => void;

  /** Results */
  results: CommandPaletteItem[];
  selectedIndex: number;
  onSelect: (item: CommandPaletteItem) => void;

  /** Recent items */
  recentItems: CommandPaletteItem[];

  /** Callbacks */
  onExecute: (item: CommandPaletteItem) => void;

  /** Sections to display */
  enabledSections: CommandSection[];

  /** Theme */
  theme?: 'light' | 'dark';
}

interface CommandPaletteItem {
  id: string;
  type: 'server' | 'command' | 'action' | 'setting' | 'group';
  title: string;
  subtitle?: string;
  icon?: string;
  badge?: string;
  shortcut?: string;
  keywords?: string[];
  action: () => void;
  disabled?: boolean;
}

type CommandSection =
  | 'recent'
  | 'servers'
  | 'groups'
  | 'actions'
  | 'commands'
  | 'settings'
  | 'help';

interface CommandShortcut {
  key: string;
  modifiers?: ('cmd' | 'shift' | 'alt' | 'ctrl')[];
  action: () => void;
}
```

---

## Dimensions & Spacing

| Element | Height | Width | Padding |
|---------|--------|-------|---------|
| Modal | auto (max 480px) | 640px (max) | 0 |
| Input Section | 56px | 100% | 0 16px |
| Results Area | auto (max 424px) | 100% | 8px 0 |
| Section Header | 28px | 100% | 4px 16px |
| Item Row | 44px | 100% | 0 16px |
| Footer | 24px | 100% | 4px 16px |
| Icon | 20px × 20px | - | 0 |
| Shortcut Badge | 20px | auto | 2px 6px |

---

## Design Tokens

### Modal
```
--palette-bg: var(--easyssh-bg-elevated)
--palette-border-radius: var(--easyssh-radius-xl)  // 12px
--palette-shadow: var(--easyssh-shadow-modal)        // Large shadow
--palette-backdrop: var(--easyssh-bg-overlay)      // Semi-transparent
```

### Input
```
--palette-input-bg: transparent
--palette-input-text: var(--easyssh-text-primary)
--palette-input-placeholder: var(--easyssh-text-tertiary)
--palette-input-icon: var(--easyssh-text-tertiary)
```

### Results
```
--palette-section-header: var(--easyssh-text-tertiary)
--palette-item-text: var(--easyssh-text-primary)
--palette-item-subtext: var(--easyssh-text-secondary)
--palette-item-bg-hover: var(--easyssh-interactive-ghost-hover)
--palette-item-bg-selected: var(--easyssh-interactive-secondary)
--palette-shortcut-bg: var(--easyssh-bg-tertiary)
--palette-shortcut-text: var(--easyssh-text-tertiary)
```

---

## Visual States

### Input Section
| State | Description |
|-------|-------------|
| Default | Transparent background, placeholder visible |
| Typing | Text appears, clear button visible |
| No Results | "No matches found" appears below |

### Result Items
| State | Background | Text |
|-------|------------|------|
| Default | transparent | text-primary |
| Hover | interactive-ghost-hover | text-primary |
| Selected | interactive-secondary | text-primary |
| Disabled | transparent | text-quaternary |

### Section Headers
```
┌─────────────────────────────────────────────────────────┐
│ 🕐 RECENT                                          2    │
└─────────────────────────────────────────────────────────┘
```
- Icon (16px)
- Label: UPPERCASE, 11px, font-weight 600
- Count badge (optional): Right aligned

---

## Command Categories

### 1. Server Actions
```
🖥️  Connect to {server-name}
    user@host:port                      Enter

🖥️  Open {server-name} in new tab
    (Standard/Pro only)                 Cmd+T

📋  Copy SSH command for {server-name}
    ssh user@host -p port               Cmd+Shift+C
```

### 2. Quick Actions
```
➕  Add New Server...
    Create a new server connection      Cmd+Shift+N

📤  Import from SSH config
    Import ~/.ssh/config entries        Cmd+Shift+I

📁  Create New Group
    Organize servers into groups        Cmd+Shift+G
```

### 3. Navigation
```
📂  Open {group-name} group
    View servers in this group          →

⚙️  Open Settings
    Application preferences             Cmd+,

🗄️  Open Server Vault
    (Lite mode main view)               Cmd+1
```

### 4. View Commands (Standard/Pro)
```
🖥️  Toggle Terminal Panel
    Show/hide terminal workspace        Cmd+J

📊  Toggle Right Panel
    Show/hide details panel               Cmd+Shift+J

📑  Toggle Sidebar
    Collapse/expand server list          Cmd+B

⬌   Split Terminal Vertically
    Create side-by-side panes           Cmd+D

⬍   Split Terminal Horizontally
    Create stacked panes                Cmd+Shift+D
```

### 5. Terminal Commands (Active Session)
```
🔍  Find in Terminal
    Search terminal output              Cmd+F

📋  Copy Selection
    Copy selected text                  Cmd+C

📄  Paste to Terminal
    Paste clipboard content             Cmd+V

❌  Clear Terminal
    Clear screen and scrollback         Cmd+K

🔄  Reconnect Session
    Re-establish connection             Cmd+R
```

### 6. Appearance
```
🌙  Toggle Dark Mode
    Switch light/dark theme             Cmd+Shift+L

🔤  Increase Font Size
    Make text larger                    Cmd++

🔤  Decrease Font Size
    Make text smaller                   Cmd+-
```

### 7. Help
```
⌨️  Keyboard Shortcuts
    View all available shortcuts        Cmd+Shift+?

📖  Documentation
    Open help documentation             F1

🐛  Report Issue
    Open GitHub issues                  -
```

---

## Interaction Patterns

### Opening the Palette
```
Default:        Cmd+K (macOS) / Ctrl+K (Windows/Linux)
Alternative:    Cmd+Shift+P (VS Code style)
From menu:      Help → Command Palette
```

### Keyboard Navigation
```
↓ (Arrow Down):    Next item
↑ (Arrow Up):      Previous item
→ (Arrow Right):   Expand group / Enter submenu
← (Arrow Left):    Back to parent / Close submenu
Enter:             Execute selected command
Cmd+1-9:           Quick select (first 9 items)
Esc:               Close palette
Cmd+K:             Toggle between server/command mode
```

### Search Behavior
```
Empty query:       Show recent + common actions
"web":             Filter servers/groups containing "web"
"connect web":     Prioritize "connect" commands for "web"
"> theme":         Only search commands (prefixed with >)
"@production":     Only search in "production" group
"!offline":        Filter by status (! = filter syntax)
```

### Fuzzy Search Algorithm
```typescript
// Matching priority (highest first)
1. Exact title match
2. Title starts with query
3. Title contains word starting with query
4. Title contains query
5. Subtitle match
6. Keyword match
7. Fuzzy match (character sequence)
```

---

## Motion & Animation

### Open Animation
```
Duration: 200ms
Easing: cubic-bezier(0.23, 1, 0.32, 1)
Backdrop: Fade in (opacity 0 → 1)
Modal: Scale in (0.95 → 1) + fade
Input: Cursor blink starts
```

### Close Animation
```
Duration: 150ms
Easing: ease-in
Backdrop: Fade out
Modal: Scale down (1 → 0.98) + fade
```

### List Transitions
```
Duration: 150ms
Easing: ease-out
Item enter: Slide in from top + fade
Item leave: Fade out quickly
Selection change: Background color transition
```

### Search Feedback
```
Typing: Immediate filter (debounced 50ms)
No results: Shake animation on input (300ms)
Results appear: Staggered fade-in (50ms delay between items)
```

---

## Accessibility

### Keyboard Shortcuts
```
Cmd+K:           Open/close palette (primary)
Cmd+Shift+P:   Open/close palette (alternative)
Cmd+/:          Open with "?" pre-filled (help mode)
Esc:             Close palette
Tab:             Navigate between sections (skipping)
Shift+Tab:       Navigate backwards
Home:            Jump to first item
End:             Jump to last item
PageUp:          Scroll up 10 items
PageDown:        Scroll down 10 items
```

### ARIA
```html
<div role="dialog"
     aria-label="Command Palette"
     aria-modal="true">
  <input role="combobox"
         aria-autocomplete="list"
         aria-controls="command-list"
         aria-activedescendant="command-1">
  <div role="listbox" id="command-list">
    <div role="group" aria-label="Recent">
      <div role="option"
           id="command-1"
           aria-selected="true">
```

### Screen Reader
- Open: "Command palette, search field"
- Search: "3 results for 'web'"
- Navigation: "Connect to web-server-01, option 1 of 3"
- Execute: "Connecting to web-server-01"
- Empty: "No results, try a different search"

### Focus Management
- Auto-focus input on open
- Trap focus within modal
- Restore focus on close
- Focus indicator on all items

---

## Empty States

### No Recent Items
```
┌─────────────────────────────────────────────────────────┐
│ ⌘ │                                           │ ✕ │     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│   🕐 RECENT                                             │
│      No recent commands                                 │
│      Your recent actions will appear here               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### No Search Results
```
┌─────────────────────────────────────────────────────────┐
│ ⌘ │ xyz                                        │ ✕ │     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│   🔍 No matches found for "xyz"                       │
│                                                         │
│   Try:                                                  │
│   • Checking your spelling                              │
│   • Using different keywords                            │
│   • Searching for server names or commands             │
│                                                         │
│   [Clear Search]                                        │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## Submenus & Workflows

### Server Connection Workflow
```
Step 1: Search "web"
   └─ Results: Servers matching "web"

Step 2: Select server
   └─ Submenu appears:
      ├─ 🚀 Quick Connect
      ├─ 📋 Copy SSH Command
      ├─ ✏️  Edit Server
      ├─ 📊 View Details
      └─ 🗑️  Remove
```

### Theme Selection Workflow
```
Step 1: Type "> theme"
   └─ Filter to theme commands

Step 2: Select "Change Theme"
   └─ Submenu:
      ├─ 🌙 Dark (current)
      ├─ ☀️  Light
      ├─ 🖥️  System
      └─ 🎨 Custom...
```

---

## Footer Shortcuts Reference

```
┌─────────────────────────────────────────────────────────┐
│ ↑↓ Navigate  │  ↵ Select  │  ⌘+K Open  │  ? Shortcuts  │
└─────────────────────────────────────────────────────────┘
```

Always visible, updates based on context:
- Submenu active: Shows "← Back"
- Editable item: Shows "↵ Edit"
- Dangerous action: Shows "⇧+↵ Confirm"

---

## Implementation Notes

### Performance
```typescript
interface PerformanceConfig {
  // Debounce search input
  searchDebounce: 50ms;

  // Virtualize long lists
  virtualizeThreshold: 50;

  // Cache recent items
  recentCacheSize: 20;

  // Preload common commands
  preload: true;
}
```

### State Management
```typescript
interface PaletteState {
  isOpen: boolean;
  searchQuery: string;
  selectedIndex: number;
  selectedSection: string | null;
  history: string[];  // Recent searches
  results: {
    servers: Server[];
    commands: Command[];
    actions: Action[];
  };
}
```

### Integration Points
- Register commands from plugins
- Expose API for custom sections
- Theme-aware rendering
- Keyboard shortcut registry
