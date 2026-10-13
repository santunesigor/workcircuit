# WorkCircuit desktop foundation

## Why
Switching between AI-assisted projects makes it easy to lose track of time and neglect the next task. WorkCircuit replaces a handwritten rotation log with a small offline desktop companion.

## What Changes
- Introduce a native Linux dashboard, circular task queue and compact companion window.
- Track one task at a time, support manual and scheduled pauses and optional reminders.
- Persist local history and provide period reports and CSV export.
- Establish a modular Rust foundation, stable identities and versioned storage for future personal multi-PC sync.

## Capabilities
### New Capabilities
- `task-rotation`: Manage tasks and move through a circular queue.
- `time-tracking`: Record sessions, pauses, exclusions and reminders reliably.
- `desktop-companion`: Operate the same timer from dashboard and compact window.
- `local-history`: Persist records and inspect/export period reports offline.
### Modified Capabilities
None.

## Impact
New Rust application using GTK4/libadwaita and SQLite. No public API, backend, authentication or synchronization in this change. The compact companion uses X11/XWayland with EWMH above-state for automatic topmost placement. Settings support English (default) and Brazilian Portuguese.
