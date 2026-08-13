## ADDED Requirements

### Requirement: Explicit root approval
The system SHALL scan only absolute local directories the user has explicitly approved and SHALL store approval in app-owned configuration outside scanned repositories.

#### Scenario: Add a valid root
- **WHEN** the user selects an existing local directory and confirms approval
- **THEN** the system records the normalized directory as an approved root and makes it available for scanning

#### Scenario: Reject an invalid root
- **WHEN** a proposed root is relative, is not a directory, or cannot be normalized safely
- **THEN** the system rejects it without scanning any content

### Requirement: Contained read-only traversal
The system SHALL read only paths whose resolved targets remain beneath an approved root and SHALL NOT create, change, move, or delete content within a scanned repository.

#### Scenario: Discover contained files
- **WHEN** a scan encounters regular directories and readable candidate files beneath an approved root
- **THEN** the system reads only the metadata or contents required for discovery and reports candidates without changing them

#### Scenario: Reject symlink escape
- **WHEN** a symlink or path traversal resolves outside all approved roots
- **THEN** the system skips the escaped target, emits a warning, and performs no read against that target

#### Scenario: Preserve repository bytes
- **WHEN** a scan completes successfully or with warnings
- **THEN** repository files and directories remain byte-for-byte and structurally unchanged

### Requirement: Project discovery
The system SHALL treat discovered Git working trees as project boundaries and SHALL permit explicit project folders when a scanned location is not a Git working tree.

#### Scenario: Discover Git projects
- **WHEN** an approved root contains one or more Git working trees
- **THEN** the system indexes each working tree as a project with its normalized root and available Git context

#### Scenario: Git inspection fails
- **WHEN** project files are readable but Git metadata is unavailable or malformed
- **THEN** the system keeps the project and its artifacts visible with a Git warning

### Requirement: Bounded and recoverable scans
The system SHALL bound scans by configurable exclusions, depth, file size, cancellation, and timeouts, and SHALL preserve the last usable index until a replacement scan succeeds.

#### Scenario: Initial scan succeeds with warnings
- **WHEN** some contained paths are unreadable but other projects and artifacts are discoverable
- **THEN** the system publishes the discoverable results with path-specific warnings

#### Scenario: Refresh fails after prior success
- **WHEN** a refresh encounters an unavailable root or operational failure and a previous index exists
- **THEN** the system marks the root unavailable while preserving the previous index for inspection

#### Scenario: Project returns
- **WHEN** an unavailable project becomes readable and a later refresh succeeds
- **THEN** the system replaces the unavailable state with the newly discovered project state

### Requirement: Local index persistence
The system SHALL store discovery results only in app-owned local storage and SHALL retain enough source metadata to determine whether an indexed artifact needs reclassification.

#### Scenario: Restart with cached index
- **WHEN** the application restarts after a successful scan
- **THEN** it can present the last usable local index while a user-requested refresh is pending

#### Scenario: Cache write fails
- **WHEN** the app-owned index cannot be persisted
- **THEN** the in-memory results remain usable for the current session and the system reports an operational warning
