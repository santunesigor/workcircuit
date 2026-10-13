# task-rotation Specification

## Purpose
Manage a persistent ordered queue of tasks and rotate attention without losing task history.

## Requirements

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

### Requirement: Task completion
The app SHALL allow completing active tasks from the dashboard and completing the current task from the compact companion. It SHALL persist the completion timestamp separately from archival, remove completed tasks from the active rotation and picker, and retain their session history in reports and exports. Completed tasks SHALL be listed in a separate dashboard section with a Reopen action.

#### Scenario: Complete a running task
- **WHEN** the current running task is completed and another active task exists
- **THEN** its session ends and the next task in rotation starts at the same instant, with a reset visit timer

#### Scenario: Complete while paused
- **WHEN** the current task is completed while tracking is paused or a scheduled break starts
- **THEN** the next active task is selected and tracking remains paused

#### Scenario: Complete the last task
- **WHEN** the only active task is completed
- **THEN** its session ends, selection is cleared and tracking stops

#### Scenario: Complete another task
- **WHEN** a task other than the current one is completed
- **THEN** the current selection and running session remain unchanged

#### Scenario: Reopen completed work
- **WHEN** a completed task is reopened
- **THEN** it returns to the end of the active queue with the same identity and history without interrupting the current session or starting paused tracking

### Requirement: Paused selection
Changing selection while paused SHALL leave tracking paused. Starting or resuming SHALL require an active task.

#### Scenario: Empty queue
- **WHEN** no active task exists
- **THEN** Start and Next cannot create a session

#### Scenario: Restore a task
- **WHEN** an archived task is restored
- **THEN** it returns to the end of the active queue with its existing identity and history

### Requirement: Temporary interruptions
The app SHALL provide a temporary-task action in the dashboard and compact companion. It SHALL require only a nonblank name, start tracking immediately, and insert the temporary task immediately before the interrupted task without changing the relative order of existing tasks. Creating one during a scheduled break SHALL be rejected without creating a task or interrupting the session. Completed temporary tasks SHALL retain their time in reports and exports.

#### Scenario: Return from an unexpected meeting
- **WHEN** the user creates a temporary task while working on B in queue A, B, C and presses Next after the meeting
- **THEN** B ends and the temporary task starts at one instant; Next completes the temporary task and starts B at one instant, with regular rotation remaining A, B, C

#### Scenario: Start an interruption while paused
- **WHEN** the user submits a valid temporary-task name while tracking is paused outside a scheduled break
- **THEN** the new task starts tracking immediately

#### Scenario: Skip to another task
- **WHEN** the user selects another task while a temporary task is current
- **THEN** the temporary task is completed automatically, while a paused timer stays paused

#### Scenario: Only temporary work remains
- **WHEN** the only active task is temporary and the user presses Next
- **THEN** it is completed and tracking stops rather than repeating it

#### Scenario: A second interruption arrives
- **WHEN** the user starts a second temporary task while a temporary task is current
- **THEN** the first temporary task is completed and the second starts without overlapping sessions

#### Scenario: Pause or restart during a meeting
- **WHEN** temporary work is paused manually, by a scheduled break or suspend, or recovered after restart
- **THEN** it remains unfinished and retains its temporary behavior until the user completes it, advances or selects another task

#### Scenario: Cancel or submit an empty title
- **WHEN** the temporary-task dialog is cancelled or a blank name is submitted
- **THEN** no task is created and current work remains unchanged
