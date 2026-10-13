## Purpose

Provide native Linux desktop controls that remain usable beside other work without requiring an internet connection.

## ADDED Requirements

### Requirement: Native dashboard
The app SHALL provide a GNOME-style task dashboard, default to English and follow the system light/dark preference.

#### Scenario: New installation
- **WHEN** the app starts without stored tasks
- **THEN** an empty state explains how to add the first task

#### Scenario: Theme-aware app icon
- **WHEN** the application icon is rendered on light or dark GNOME surfaces
- **THEN** the transparent 10.08 chevrons use the surface foreground color, without a base or frame

#### Scenario: GNOME Shell prefers regular app icons
- **WHEN** GNOME Shell resolves the launcher icon with regular-icon preference
- **THEN** the dedicated symbolic asset is still selected and recolored, rather than falling back to a fixed black counterpart

### Requirement: Compact companion
Overlay SHALL open a compact window with current task, elapsed duration, next task, selection, Next and Pause/Resume. Both windows SHALL operate the same timer.

#### Scenario: Shared state
- **WHEN** a task is advanced in the compact window
- **THEN** the dashboard immediately reflects the same task and duration

### Requirement: Window lifecycle
Closing one window SHALL retain tracking while the other remains open. Closing the last window or Quit SHALL end tracking. The companion SHALL automatically request above-state on X11 and GNOME/XWayland.

#### Scenario: Close dashboard
- **WHEN** the compact window remains open when the dashboard closes
- **THEN** the timer continues and the dashboard can be reopened

### Requirement: Small always-on-top companion
The companion SHALL use a small fixed layout (approximately 300 by 140 logical pixels) with an ellipsized task title and task picker popover. It SHALL request above-state automatically, and display an actionable error if X11/XWayland is unavailable.

#### Scenario: Dashboard receives focus
- **WHEN** the user focuses the dashboard after opening the companion on GNOME/Wayland with XWayland
- **THEN** the compositor retains the companion above ordinary windows

#### Scenario: Long task title
- **WHEN** a task has a long title or many tasks exist
- **THEN** the companion stays compact and task selection is available in its popover

#### Scenario: Readable picker beside a narrow button
- **WHEN** the task picker is opened from the small overlay button
- **THEN** it has a readable minimum width, names occupy one line with ellipsis and a full-name tooltip, and the entire task row selects the task

### Requirement: Persisted interface language
Settings SHALL offer English and Português (Brasil). English SHALL be the default regardless of system locale. Saving the language SHALL update both windows without interrupting tracking, persist across restarts and preserve existing task text.

#### Scenario: Change language while working
- **WHEN** the user saves Português (Brasil) during an active session
- **THEN** both windows show Portuguese controls, the same session continues, and the choice is restored on restart

#### Scenario: Existing database
- **WHEN** preferences created before language selection are loaded
- **THEN** English is used without requiring a data reset
