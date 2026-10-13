# local-history Specification

## Purpose
Retain private work history locally and provide accurate portable reports over selected calendar periods.

## Requirements

### Requirement: Offline persistence
Tasks, ordering, descriptions, sessions, exclusions and preferences SHALL persist locally without a network service. Records SHALL have stable identities and versioned schema migrations.

#### Scenario: Restart offline
- **WHEN** the app restarts with no network
- **THEN** tasks and settings are restored and tracking is paused

#### Scenario: Upgrade a database with existing history
- **WHEN** a database from version 0.1.0 is opened by a version supporting task completion
- **THEN** it is migrated transactionally, retaining tasks, archival state, preferences and sessions; existing tasks have no completion timestamp

#### Scenario: Upgrade a database with completed tasks
- **WHEN** a database supporting completion is opened by a version supporting temporary tasks
- **THEN** it is migrated transactionally with completion timestamps and history retained; existing tasks are regular tasks by default

### Requirement: Period reporting
Reports SHALL show totals by task, a grand total and session history for an inclusive local-date range, counting only the contribution inside that range.

#### Scenario: Midnight boundary
- **WHEN** a session crosses midnight and only the second date is requested
- **THEN** only the portion on the requested date contributes

### Requirement: CSV export
Reports SHALL export task totals and session details as separate CSV files with headers and escaped text.

#### Scenario: Special task title
- **WHEN** a task contains commas, quotes or line breaks
- **THEN** the exported CSV preserves cells and remains parseable

### Requirement: Storage failures
Failed writes SHALL be shown to the user and SHALL NOT silently acknowledge a task transition.

#### Scenario: Disk write fails
- **WHEN** a task switch cannot be saved
- **THEN** an error is visible and no successful transition is claimed
