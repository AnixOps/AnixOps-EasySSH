# UI Reference - React Frontend Archive

## Status: ARCHIVED

This directory contains the **archived React frontend code** from the original EasySSH Web UI implementation.

**Important**: This code is for **reference only** and is not part of the active codebase. The project has migrated to native UI implementations:

| Platform | UI Framework | Location |
|----------|--------------|----------|
| Windows | egui | `platforms/windows/easyssh-winui/` |
| Linux | GTK4 | `platforms/linux/easyssh-gtk4/` |
| TUI/CLI | Rust TUI | `tui/` |

## What's Preserved

### `stores/`
Zustand state management logic - useful for understanding the data model when migrating to Rust:
- `serverStore.ts` - Server and session state management
- `uiStore.ts` - UI state (sidebar, toasts, theme)
- `i18nStore.ts` - Internationalization state
- `k8sStore.ts` - Kubernetes cluster state
- `aiAssistantStore.ts` - AI assistant conversations

### `types/`
TypeScript type definitions - serves as reference for Rust type design:
- Server, Session, ServerGroup types
- Kubernetes types (K8sCluster, K8sPod, K8sNode, etc.)
- Settings and configuration types
- AI Assistant types

### `utils/`
Utility functions - algorithm reference:
- Date/time formatting
- ID generation
- Keyboard shortcut parsing
- Validation functions
- Color utilities

### `styles/`
CSS/Tailwind styles - design token reference:
- Apple Design System inspired tokens
- RTL support styles

## Migration Notes

When porting logic to Rust:

1. **State Management**: Zustand stores → Rust state machines with egui/GTK4
2. **Types**: TypeScript interfaces → Rust structs with serde
3. **Utils**: Pure functions can be directly ported to Rust
4. **Styles**: CSS tokens → Design system constants in Rust

## Original Tech Stack

- **Framework**: React 18 + TypeScript
- **State**: Zustand
- **Styling**: Tailwind CSS with custom Apple Design System
- **Terminal**: xterm.js (planned, never implemented)
- **Build**: Vite
- **Testing**: Playwright

## Deletion Date

2026-04-02 - Moved from `src/` to `ui-reference/`

## Rationale

The Web UI (Tauri-based) was abandoned in favor of:
- **Better native integration** - Native UI provides better OS integration
- **Smaller bundle size** - No WebView overhead
- **Better performance** - Native rendering vs WebView
- **Simpler architecture** - Direct Rust UI instead of Rust + WebView + React
