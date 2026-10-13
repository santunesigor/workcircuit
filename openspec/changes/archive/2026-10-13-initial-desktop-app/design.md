# Design

## Context
The repository is empty. The target is Linux, initially Ubuntu GNOME on Wayland. See proposal.md for motivation.

## Goals / Non-Goals
Deliver an offline personal task rotation tool. Keep business logic independent of GTK and SQLite. Cloud accounts, synchronization and team features are deferred.

## Decisions
- Rust with GTK4/libadwaita provides native GNOME controls instead of a web runtime.
- One crate with domain, storage, reporting and UI modules keeps boundaries explicit without premature services or a generic backend framework.
- SQLite with numbered transactional migrations replaces a local database server. Tasks, sessions and exclusion rules use UUIDs; UTC timestamps and soft archival preserve history. SQLite stays on each client if a future sync API and PostgreSQL server are introduced.
- A single controller owns queue selection, monotonic session duration and pause state. Switching commits the old session and the new one atomically at one wall-clock instant. While paused, selection changes do not start counting.
- A one-second UI tick checks exclusion boundaries and reminders. Five-second checkpoints bound crash recovery loss; recovery ends an interrupted session at its last checkpoint and starts paused. Clean exit closes the session. Suspend is observed via logind PrepareForSleep, with elapsed-clock gap detection as a fallback.
- Exclusion ranges use local weekdays and clock times, including overnight ranges. Overlapping ranges form a union. Crossing a boundary stops at the boundary, never at the next repaint, and always requires manual resume.
- Store measured duration separately from wall-clock timestamps so clock adjustments cannot create negative elapsed time. Reports clip session contributions proportionally to their wall-clock overlap with the requested local-date period. Calendar day boundaries use timezone-aware local conversion.
- The compact window and dashboard share state; closing one leaves the other operational. Last-window closure and Quit stop tracking. Only the compact companion uses a cached dedicated X11/XWayland display. EWMH above-state is set before mapping and requested again on mapping; the dashboard retains its native backend. Task selection lives in a popover to keep the fixed companion around 300×140 logical pixels.
- Reporting and CSV export use the same aggregation path. A native save dialog writes an escaped CSV for totals or sessions; text cells starting with spreadsheet formula prefixes are neutralized.

## Risks / Trade-offs
- GNOME Wayland restricts native topmost placement → use XWayland only for the companion and show an actionable error if unavailable. Fullscreen/system surfaces remain compositor-controlled; no shell extension.
- Persist `en` or `pt-BR` in preferences with a serde English default for older settings. A small translation catalog handles controls and canonical domain statuses. Rebuild open views on language changes while preserving the controller/session; callbacks from replaced windows must not clear the replacement or stop tracking.
- Sudden power loss → transactional checkpoints, at most five seconds of uncheckpointed time lost.
- Clock/timezone changes → monotonic durations, UTC instants, local display; no sync conflict policy is implied.
- Session volume → indexed period queries; no per-second rows. Future sync requires its own OpenSpec change covering identity, tombstones, conflict resolution and overlapping device sessions.

## Migration Plan
Create schema on first launch in XDG_DATA_HOME/workcircuit. Future migrations increment user_version transactionally. Distribution starts with a binary, desktop entry and icon; packaging and repositories come later.
