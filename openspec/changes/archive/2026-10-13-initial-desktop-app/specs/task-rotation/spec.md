## Purpose

Manage a persistent ordered queue of tasks and rotate attention without losing task history.

## ADDED Requirements

### Requirement: Task management
The app SHALL require a nonblank title, accept an optional description, and allow editing, reordering, archiving and restoring tasks without deleting recorded sessions.

#### Scenario: Archive current task
- **WHEN** the running task is archived
- **THEN** its session ends and tracking pauses; archived tasks remain in reports

### Requirement: Circular rotation
Next SHALL move to the next active task, wrapping after the last. Selecting a task SHALL continue subsequent rotation from that position. Skipped tasks SHALL receive no session.

#### Scenario: Jump in queue
- **WHEN** the user jumps from A to C in queue A, B, C, D
- **THEN** C starts and the next task is D; B receives no time

### Requirement: Paused selection
Changing selection while paused SHALL leave tracking paused. Starting or resuming SHALL require an active task.

#### Scenario: Empty queue
- **WHEN** no active task exists
- **THEN** Start and Next cannot create a session

#### Scenario: Restore a task
- **WHEN** an archived task is restored
- **THEN** it returns to the end of the active queue with its existing identity and history
