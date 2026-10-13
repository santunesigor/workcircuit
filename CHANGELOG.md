# Changelog

## Unreleased

- Start a temporary task from just a name in the dashboard or companion. It runs immediately, completes on Next or a direct task switch, and returns to interrupted work without changing the regular rotation order.
- Persist temporary-task behavior across restarts and keep meeting time in reports and CSV exports.
- Add a Brazilian Portuguese README with language links and include it in release packages.
- Complete tasks from the dashboard, task rows or compact companion; keep them outside the active rotation with their existing time history.
- Reopen completed tasks at the end of the queue. Completing a running task continues with the next task; paused tracking stays paused and finishing the last task stops tracking.
- Migrate existing databases automatically to store task completion timestamps.

## 0.1.0 — 2026-10-13

First Linux desktop release.

- Ordered task rotation with direct switching, editing, reordering, archival and restoration.
- One active timer, manual pause/resume, recurring scheduled breaks and optional reminders.
- Offline SQLite persistence, crash recovery and suspend-aware tracking.
- Period reports and CSV exports for task totals and session history.
- Compact always-on-top companion using X11/XWayland alongside a native GNOME dashboard.
- English by default, with saved Brazilian Portuguese selection and live language updates.
- Minimal double-chevron icon that adapts to light and dark desktop surfaces.

The Linux x86_64 binary targets Ubuntu 24.04 with GTK 4.12+ and libadwaita 1.4+. APT/Snap packages and cloud synchronization are future work. Fullscreen window ordering is controlled by the compositor. Physical suspend/resume, notification presentation and native save-dialog interaction still require manual checks on each desktop environment.

No license is granted for the project's code at this time.
