# time-tracking Specification

## Purpose
Measure work on one task at a time while excluding breaks and protecting against unattended time.

## Requirements

### Requirement: Session transitions
Only one task SHALL count time. A running task switch SHALL end the old session and start the new one at the same computer-clock instant. Manual pause SHALL stop counting.

#### Scenario: Switch tasks
- **WHEN** the user advances while tracking
- **THEN** the old end and new start timestamps match

### Requirement: Recurring exclusions
The app SHALL support local weekday/time exclusion ranges, merge overlaps, support overnight ranges, and stop at the boundary. Starting during an exclusion SHALL be blocked; ending it SHALL require manual resume.

#### Scenario: Lunch
- **WHEN** tracking crosses into a configured 12:00–13:00 break
- **THEN** the session ends at 12:00 and remains paused after 13:00

### Requirement: Reminder
An optional global threshold SHALL produce one visible and local notification per task visit without switching tasks. Pauses SHALL preserve accumulated visit time.

#### Scenario: Threshold reached
- **WHEN** a visit reaches its configured threshold
- **THEN** a reminder appears and the task keeps counting

### Requirement: Suspend and recovery
Suspension and last-window exit SHALL stop counting. Crash recovery SHALL end the interrupted session at its last checkpoint and reopen paused.

#### Scenario: Interrupted process
- **WHEN** the app reopens after an unclean exit
- **THEN** time since its last checkpoint is not counted

### Requirement: Clock accuracy
Durations SHALL use monotonic elapsed time during execution while timestamps use the computer clock. Negative durations SHALL never be reported.

#### Scenario: Clock moves backwards
- **WHEN** the system wall clock is adjusted backwards during work
- **THEN** elapsed work duration remains nonnegative
