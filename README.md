# WorkCircuit

[English](README.md) · [Português (Brasil)](README.pt-BR.md)

A small offline Linux desktop companion for rotating between projects and knowing where your time went.

When an AI assistant is working on one project, move to the next without losing your place. WorkCircuit keeps an ordered task rotation, measures your work, and stays beside your other windows in a compact companion view.

**Latest release: 0.1.0** · [Releases](https://github.com/santunesigor/workcircuit/releases) · [Changelog](CHANGELOG.md)

Task completion and temporary tasks are currently available in the source build; they are not included in the 0.1.0 release archive.

Cloud synchronization and APT/Snap publishing are planned, not available.

## What it does

- Create tasks with a required title and optional description; edit, reorder, archive and restore them.
- Complete finished tasks, preserve their reports, and reopen them when needed.
- Start a temporary task with just a name for an unexpected meeting or interruption; it completes automatically when you move on.
- Start a task, advance through a circular queue, or jump directly to another task.
- Pause for a break and resume explicitly. Only one task counts time at a time.
- Configure recurring breaks by weekday and local time, including overnight ranges.
- Optionally receive a reminder after a configurable amount of work on one task visit.
- Use a compact **Overlay** window that automatically stays above ordinary windows, with a task picker and the same timer as the dashboard.
- Choose English or Português (Brasil) in Settings. New installations default to English; your choice is saved.
- View period totals and session history, and export task totals or sessions as CSV.
- Keep everything on your computer, without an account, internet connection or database server.

## Install a release

Download the Linux x86_64 archive and `SHA256SUMS` from [the latest release](https://github.com/santunesigor/workcircuit/releases/latest). The binary is built on Ubuntu 24.04 and requires GTK 4.12+, libadwaita 1.4+, and X11/XWayland. Other distributions with compatible libraries may work but are not release targets yet.

```sh
sudo apt install libgtk-4-1 libadwaita-1-0 xwayland
sha256sum --check SHA256SUMS
tar -xzf workcircuit-0.1.0-linux-x86_64.tar.gz
cd workcircuit-0.1.0-linux-x86_64
./install.sh
```

The installer uses `~/.local` by default. Search for **WorkCircuit** in the application launcher. To uninstall, remove `~/.local/bin/workcircuit`, `~/.local/share/applications/io.github.workcircuit.WorkCircuit.desktop` and the two `io.github.workcircuit.WorkCircuit*.svg` files in `~/.local/share/icons/hicolor/scalable/apps/`. Your database is retained.

## Requirements and build

Linux with GTK **4.12+** (including its X11 backend), libadwaita **1.4+**, and Rust **1.92+**. CI uses the current stable Rust toolchain. The overlay requires an X11 session or XWayland on Wayland. Ubuntu 24.04 or newer is a suitable baseline. The lockfile pins Rust dependencies; the first dependency download requires internet. Running the built application does not.

On Ubuntu/Debian with sufficiently recent GTK:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev xwayland
git clone https://github.com/santunesigor/workcircuit.git
cd workcircuit
cargo build --locked
./target/debug/workcircuit
```

For an optimized binary:

```sh
cargo build --release --locked
./target/release/workcircuit
```

SQLite is compiled into the application; PostgreSQL and a separate SQLite installation are unnecessary.

## Daily workflow

1. Add the projects you are working on and arrange their order with the up/down controls.
2. Select a task and press **Start**. Selection while paused does not begin counting.
3. Press **Next** to end the current session and start the next task at the same instant. After the last task, the queue wraps to the first. With one regular task, Next keeps the current visit running.
4. Use a task's selection button to jump to it. Skipped tasks receive no time, and the next advance continues from the selected task.
5. Press **Pause** for an unscheduled break, and **Resume** when you return.
6. Open **Reports**, enter an inclusive local-date range, and view or export the result. Press **Show** again to refresh a running session's contribution.

Press **Complete** in the dashboard, or the checkmark beside a task, when its work is finished. The companion has the same checkmark in its header. Completed tasks leave the rotation and task picker, and move to **Completed tasks** in the dashboard. Completing the current task starts the next task at the same instant if tracking was running; while paused, it selects the next task without starting it. Completing the last task stops tracking. **Reopen** returns a completed task to the end of the queue with its original identity and time history. Completion and archival are separate states; both preserve reports and CSV exports.

The main timer displays accumulated work during the current task visit, excluding pauses. Switching to a different task resets this visit timer. Reports retain all sessions, including those for archived tasks. Archiving the current task pauses tracking. Restore archived tasks from the dashboard to return them to the end of the queue.

### Temporary tasks for interruptions

Press **Temporary Task** in the dashboard or **+** in the companion, enter a name, and press **Start Now** (or Enter). Tracking starts immediately, even if you were paused, and the previous running session ends at the same instant. Closing the dialog without starting leaves your work unchanged.

The temporary task is inserted just before the interrupted task. **Next** completes it automatically and returns to that task, preserving the regular queue's order. Selecting another task also completes the temporary one. If no other active task exists, Next completes it and stops tracking. Starting a second temporary task completes the previous one.

Manual pauses, scheduled breaks, suspend and restart do not complete temporary work. Resume it when ready, then advance or switch tasks to finish. Creation is blocked during a scheduled break. Completed temporary tasks remain in **Completed tasks**, reports and CSV exports; reopening one retains its temporary behavior.

### Companion window on GNOME

Press **Overlay** to open the approximately 300 × 140 px companion. It requests **Always on Top** automatically, so no window-menu pinning is necessary. On GNOME/Wayland, only the companion uses XWayland and the standard EWMH above-state request; the dashboard stays native. This keeps it above ordinary windows, while fullscreen applications and system surfaces remain under the compositor's control.

Drag the task-name area to move it, use the list button to choose a task, and use the expand button to reopen the dashboard. The task list lives in a popover instead of taking up the companion's height. If XWayland is unavailable, an actionable error is shown instead of presenting an unpinned window.

### Interface language

Open **Settings → Language**, choose **Português (Brasil)** and press **Save Settings**. Both windows update immediately without interrupting the active session. The selection persists across restarts; fresh and older installations without a language preference start in English. Task titles/descriptions and CSV column names remain as entered or defined, independent of interface language.

Closing either dashboard or companion leaves the other operational. Closing the last application window ends tracking. The companion includes a button to reopen the dashboard.

### Scheduled breaks and reminders

In **Settings**, add breaks using weekdays and 24-hour local times. Overlapping breaks are treated as a union. For an overnight break, the selected weekday is the day it starts. Equal start/end times are rejected. During a break, starting or resuming is blocked; after it ends, resume manually.

The reminder is disabled by default. Set its minutes to receive one reminder per task visit. Manual pauses preserve accumulated work, and a reminder never switches your task automatically. Notifications depend on your desktop's notification settings; the app also shows a visible reminder message.

## Data and time accuracy

Data lives at:

```text
$XDG_DATA_HOME/workcircuit/workcircuit.db
# default: ~/.local/share/workcircuit/workcircuit.db
```

Tasks, sessions and break rules have stable UUIDs. Schema changes use numbered transactional migrations. Changes and task transitions are committed to SQLite, with active sessions checkpointed every five seconds. After a crash, the last checkpoint becomes the session end and the app reopens paused; up to five seconds of uncheckpointed work may be lost.

Computer-clock timestamps are stored in UTC and displayed in the current local timezone. Duration uses a monotonic clock. Suspension pauses tracking through logind, with clock-gap detection as a fallback. Large timer interruptions pause conservatively rather than counting unattended time.

Reports count only the part of a session inside the selected period. If the wall clock changes during a session, measured duration is apportioned by wall-clock overlap; a nonpositive wall interval is attributed to its start date. CSV exports use UTC timestamps, duration in seconds, standard quoting and spreadsheet-formula neutralization for text cells.

For a database backup, close WorkCircuit first and copy the database file. Do not share a live SQLite file between computers through a synced folder or network drive. CSV is a report format, not a complete application backup.

## Architecture and future sync

A single Rust crate keeps these boundaries:

| Module | Responsibility |
| --- | --- |
| `domain` | Rotation, sessions, exclusions, reminders and clock samples |
| `storage` | SQLite, migrations, atomic writes and recovery |
| `report` | Period clipping, aggregation and CSV |
| `ui` | Native GTK/libadwaita windows, notifications and suspend integration |
| `i18n` | Explicit English/Portuguese UI text selection |
| `overlay` | Dedicated X11 display and standard always-on-top window hints |

The domain has no GTK or SQL dependency. Both windows share one controller. Period queries use an index; there are no per-second database records.

The future direction is personal use across multiple PCs: retain a local SQLite database on each computer and add an authenticated synchronization API, potentially backed by PostgreSQL. Stable IDs and module boundaries help that evolution, but do not implement synchronization by themselves. Device identity, deleted-record handling, overlapping sessions and conflict resolution must be specified before adding the service.

## Development and specifications

The current requirements are in [`openspec/specs`](openspec/specs). The completed initial proposal, design and checklist are kept in [`openspec/changes/archive`](openspec/changes/archive). OpenSpec is a development tool, not a runtime dependency.

```sh
openspec validate --all --strict
cargo test --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
```

The domain/storage/report suite can run without GTK development packages:

```sh
cargo test --no-default-features --locked
```

The opt-in GUI smoke test needs a display (or Xvfb) and a session bus:

```sh
GDK_BACKEND=x11 GSK_RENDERER=cairo dbus-run-session -- xvfb-run -a cargo test --bin workcircuit --locked -- --ignored --test-threads=1
```

On GNOME/Wayland, the compositor integration check verifies the reported above-state even after the dashboard receives focus:

```sh
WORKCIRCUIT_WM_ONLY=1 GDK_BACKEND=wayland cargo test --bin workcircuit --locked -- --ignored --test-threads=1
```

Set `WORKCIRCUIT_CAPTURE_DIR=/tmp` when running the GUI test under Xvfb to save dashboard and companion screenshots. Native Wayland may skip rendering windows covered by other applications, so screenshot capture is opt-in.

## Local installation and roadmap

```sh
make install
```

This builds a release binary and installs it, a desktop entry and SVG icons under `~/.local`. The transparent double-chevron icon adapts to light and dark desktop surfaces. The installed launcher uses an absolute executable path. Search for **WorkCircuit** in GNOME's application list; right-click its dock icon and choose **Pin to Dash** / **Add to Favorites** to keep it visible when closed. Ensure `~/.local/bin` is on your PATH to launch it by name from a terminal. Packagers can set `PREFIX=/usr` and `DESTDIR` to stage the same files. The desktop ID remains `io.github.workcircuit.WorkCircuit` for compatibility with existing installations.

CI checks formatting, Clippy, core tests, the GUI smoke test under Xvfb, and an optimized build on Ubuntu 24.04. It produces a release archive with an installer, SHA-256 checksum and third-party notices. To package a local release after building:

```sh
python3 scripts/package.py
```

Use Ubuntu 24.04 when building the published Linux archive; a binary built on a newer distribution may require newer system libraries. Generated archives go in `dist/` and are ignored by Git.

Planned separately:

- Debian packages and APT repository publishing.
- Snap packaging, confinement checks and publishing.
- Optional personal cloud backup and multi-PC synchronization.

## License

No license is granted for WorkCircuit at this time. Public source availability does not grant permission to reuse, modify or redistribute this project's code. Third-party components retain their own licenses; release archives include their notices in `THIRD-PARTY-NOTICES.md` and `LICENSES/`.
