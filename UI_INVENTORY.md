# EasySSH UI / UX Inventory

## Audit scope

EasySSH is a native Rust `eframe` / `egui` desktop application. It has no web frontend, router, DOM, or embedded terminal. The entire presentation and application state currently live in `crates/easyssh-desktop/src/main.rs`; `ui/theme.rs` and `ui/components.rs` contain only partial shared styling helpers. The release executable was launched successfully on 2026-07-28 as a process smoke check. Native-window screenshots and automated click-through were not available in this environment, so the visual acceptance items below require a manual desktop pass after implementation.

## Application surfaces

| Surface | Entry point | Current states | Audit notes |
| --- | --- | --- | --- |
| Top bar | Always visible | Theme, agent state, Git status, quick connect, command palette | Fixed controls can crowd at narrow widths; no overflow strategy. |
| Session strip | Always visible | Empty, launch success/failure, hidden, reconnect | Represents launch records only, correctly not terminal state; no explicit empty state. |
| Workspace sidebar | Hosts, Snippets, Forwarding, Transfers | Selected, disabled placeholder tools, status text | Uses ad-hoc selectable labels and disabled buttons with no explanation. |
| Hosts | List + inspector | Empty, search, selected, favorite, recent, context menu | Fixed three columns only; no responsive inspector drawer, no loading/skeleton, metadata color is absent. |
| Host inspector | Hosts | No selection, overview, forwarding, notes/activity, actions | Sections are static labels rather than tabs; focus and scan hierarchy are weak. |
| Host editor | Modal | New/edit, address/alias, invalid/unfilled, dirty | Mutates persisted model while editing, has no cancel/revert or field-level validation. |
| Quick connect | Modal | Empty host disabled, connect success/failure | Only host non-empty is validated before submission. |
| Snippets | List + editor | Empty implicit, search, copy success, delete | Copy-only safety boundary is present; no explicit empty state or toast confirmation. |
| Port forwarding | List | Empty implicit, navigate to host | Read-only list lacks filtering and a defined empty state. |
| Transfers | Form + queue | Authorizing, transferring, completed, failed, cancelled | Uses system `scp`; no file picker, transfer progress, host retained in retry record, or queued pending scheduling. |
| Command palette | Modal | Query, keyboard selection, Escape | Arrow/Enter/Escape exists, but no initial focus restoration, categories, empty result state, or accessible selected-state treatment. |
| Delete confirmations | Modals | Host/snippet confirm/cancel | Host and snippet vary in language and danger style; no default focus or Escape contract. |
| SSH agent diagnostics | Modal | Available/unavailable/not configured | Security boundary is sound; no refresh/loading/error presentation. |
| Git metadata sync | Modal | Unconfigured, clean, changes, failed, init/pull/push | Synchronous operations freeze the UI; status is text only and retry/cancel/progress are absent. |

## Current shared implementation

- Theme: one density-aware `apply` function with hard-coded colors and 4-6px rounding.
- Components: icon button and status label only.
- Helpers: icon buttons, section label, detail row, multi-line forward editor, target and time formatting, status mapping.
- State: one `EasySshApp` struct owns persistent config, modal booleans, form state, transfer children, clipboard and status text.

## UX and accessibility findings

1. The UI has no component contract for buttons, inputs, dialogs, cards, toasts, tooltips, tabs, empty states, skeletons or error help. Equivalent controls look and behave differently across surfaces.
2. All async feedback is inconsistent: external terminal and Git actions overwrite a passive sidebar string; transfers are polled but have no progress or robust retry payload; Git work runs on the UI thread.
3. Dialogs and command palette do not capture, restore, or consistently set focus. Escape behavior is partial and keyboard order is not specified.
4. The host editor writes directly to application state before Save, so closing it can retain unsaved changes. Validation occurs only inside OpenSSH invocation, after the form interaction.
5. A 1200px minimum viewport prevents overlap but makes compact displays inaccessible rather than adapting the inspector to a drawer.
6. Empty, loading, success, error, disabled, and destructive states are incomplete or implicit. Disabled tools do not communicate why they are unavailable.
7. There is no reduced-motion preference or motion system. There are no purposeful transitions, but future work must avoid animating terminal I/O, scroll position, or large list layout.
8. Colors, dimensions, typography and hierarchy are duplicated between the legacy app and partial theme module. There is no semantic color system for status, focus, danger, or elevation.
9. The app preserves key security boundaries: no embedded terminal, credential storage, private key access, or persisted transfer output. The redesign must retain these boundaries.
