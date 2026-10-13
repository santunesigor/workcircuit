## 1. Foundation
- [x] 1.1 Establish OpenSpec proposal, design and scenarios; validate with openspec validate --strict.
- [x] 1.2 Create Rust crate, native desktop assets and English README; verify build instructions against available tools.

## 2. Domain and storage
- [x] 2.1 Implement rotation, pauses, reminders and scheduled exclusions; verify deterministic unit tests.
- [x] 2.2 Implement migrated SQLite storage, atomic transitions and crash recovery; verify persistence tests.
- [x] 2.3 Implement clipped period reports and escaped CSV export; verify midnight and boundary tests.

## 3. Desktop
- [x] 3.1 Implement task dashboard, editing, reordering and archival; verify via GUI smoke checks.
- [x] 3.2 Implement shared compact controls, settings, reports and export dialogs; verify via GUI smoke checks.
- [x] 3.3 Implement suspend pause, notifications and graceful shutdown; verify integration checks where environment permits.

## 4. Integration
- [x] 4.1 Run cargo test, cargo fmt --check, cargo clippy and openspec validation; record results.
- [x] 4.2 Smoke-test GUI and offline persistence; document any manual checks still needed.

## 5. Companion, language and desktop integration
- [x] 5.1 Use a compact companion layout with a readable task picker and whole-row activation.
- [x] 5.2 Request topmost state over X11/XWayland and verify the compositor reports above-state after dashboard focus.
- [x] 5.3 Add persisted language selection with English defaults and live updates without interrupting tracking.
- [x] 5.4 Install the selected transparent double-chevron symbolic mark with a dedicated icon identity.
- [x] 5.5 Verify light/dark icon contrast through GTK and GNOME Shell, including regular-fallback lookups.

## Verification record
- 13 domain/storage/report tests passed, including on-disk restart, atomic write failure, overnight exclusions, recovery, date clipping and language preferences.
- The opt-in GTK smoke test passed under Xvfb. It exercises real timer ticks, task creation, shared controls, compact geometry, picker selection, language changes, settings, reports, restore and last-window shutdown.
- The GNOME/Wayland compositor check passed with a native dashboard and XWayland companion; above-state remains set after dashboard focus.
- GTK and GNOME Shell icon rendering checks passed for light/dark foregrounds and normal/FORCE_REGULAR lookups.
- Debug and optimized builds, staged installation, desktop-entry validation, formatting, Clippy with warnings denied and strict OpenSpec validation passed.

## Manual acceptance checks
- Check companion placement across workspaces/monitors and fullscreen applications.
- Physically suspend/resume the PC and check desktop notification presentation. Clock-gap behavior is unit-tested and logind subscription is exercised; physical suspension is not automated.
- Choose a destination in the native save dialog and open each CSV in a spreadsheet. CSV serialization itself is tested.
