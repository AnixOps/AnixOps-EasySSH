# EasySSH UI / UX Redesign Plan

This document is the implementation checklist. No SSH invocation, configuration format, Git sync whitelist, or credential boundary may change.

## 1. Foundation and architecture

- [ ] Split desktop presentation into `ui/tokens`, `ui/theme`, `ui/components`, `ui/feedback`, `ui/shell`, `ui/dialogs`, and `ui/workspaces`.
- [ ] Move transient UI state out of rendering functions and introduce explicit view/form state for dialogs and transfers.
- [ ] Preserve `AppConfig`, OpenSSH invocation arguments, external terminals, Git/GCM authentication and non-persistent transfer records.

Acceptance: `main.rs` is an application coordinator, and shared components own common visuals and interaction rules.

## 2. Design system

- [x] Define semantic light/dark tokens for canvas, surfaces, text, border, focus, primary, success, warning, danger and muted states.
- [ ] Define density-scaled type, spacing, control heights, icon sizes, 0-8px rounding, borders, elevation and z-order.
- [ ] Build Button, Input, Select, Card, Dialog, Toast, Tooltip, Tabs, Sidebar, EmptyState, Skeleton, StatusBadge and validated form-field APIs.
- [ ] Apply the system to every workspace and dialog; remove local one-off styling.

Acceptance: theme/density changes affect every screen through tokens, and all interactive controls have consistent hover, focus, disabled and danger treatment.

## 3. Shell and workspace UX

- [ ] Rebuild top bar, session strip and sidebar with responsive overflow behavior and useful disabled reasons.
- [ ] Rebuild Hosts with grouped/filterable two-line rows, explicit empty/loading/error states and responsive inspector drawer below the safe three-column width.
- [ ] Rebuild inspector into Overview, Connection, Forwarding, Notes and Activity tabs, without implying remote terminal status.
- [ ] Rebuild host/snippet/quick-connect forms with draft state, validation, dirty close confirmation and consistent destructive dialogs.
- [ ] Rebuild Snippets and Forwarding using common lists, empty states and context actions while retaining copy-only and connection-only semantics.
- [ ] Complete Transfers with safe form validation, persistent retry payload, queued/cancelled/failure states and non-sensitive summaries.
- [ ] Make Git sync and diagnostics present loading, success, error, retry and disabled states without exposing credentials or terminal output.
- [ ] Expand command palette to all workspaces/actions and define Arrow, Enter and Escape behavior.

Acceptance: all workflows are reachable by keyboard and mouse, resize without overlapping content, and expose a clear outcome for every action.

## 4. Motion and accessibility

- [ ] Define 150-200ms opacity/transform motion tokens and apply them to dialogs, drawers, menus, tabs, toasts and list insertion/removal only.
- [ ] Add a local reduced-motion override, with system preference detection where the platform exposes it.
- [ ] Add visible focus treatment, dialog/palette focus lifecycle, Escape close behavior, accessible labels/tooltips and disabled reasons.

Acceptance: reduced motion eliminates non-essential animation; terminal I/O, transfer polling and large-list scrolling remain unanimated.

## 5. Validation

- [ ] Add unit tests for visual/status mappings, validation, command-palette navigation, session hiding, transfer cancel/retry and sync safety DTOs.
- [ ] Run `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, and `cargo build --release --workspace`.
- [ ] Manually inspect Hosts, Snippets, Forwarding, Transfers, Sync, Diagnostics and every modal under light/dark, Compact/Comfortable/Large, minimum and narrow widths, keyboard-only navigation and reduced motion.

Acceptance: no build warnings, no test failures, and no overlap, truncation or inaccessible control in manual desktop QA.
