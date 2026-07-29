# Transfers / SCP Redesign Checklist

## Audited baseline

- [x] Current page, transfer state, `ScpInvocation`, cancellation and configuration storage inspected.
- [x] Constraints recorded: `scp` provides no trustworthy byte progress through the current non-interactive child-process API; no remote browsing API exists.

## Implementation

- [ ] Add a safe system-OpenSSH `sftp` batch interface for remote listing; validate every path and never pass values to a shell.
- [ ] Add local filesystem enumeration, metadata formatting, path navigation, search and system file/directory picker.
- [ ] Replace path parameter form with a connected-host header and two-pane browser.
- [ ] Add multi-select transfer actions; infer recursion from selected local directory rather than exposing a recursive switch.
- [ ] Persist non-sensitive transfer history, marking active jobs interrupted at next launch.
- [ ] Implement queue state: waiting, running, paused, completed, failed, cancelled and interrupted.
- [ ] Add cancel/retry/clear and explicit overwrite, permission, connection and disk-space error states.
- [ ] Implement truthful progress only where the underlying OpenSSH process exposes measurable bytes; otherwise present indeterminate running state, never simulated values.
- [ ] Add keyboard selection/navigation, focus, tooltips and destructive confirmations.
- [ ] Apply shared tokens and reduced-motion-aware opacity/transform transitions.

## Validation

- [ ] Unit test SFTP command construction and parsing, local metadata and state transitions.
- [ ] Run fmt, clippy, workspace tests and release build.
- [ ] Manually verify local browsing, a permitted remote listing, denied access, failed transfer, cancellation and narrow/wide layouts.
