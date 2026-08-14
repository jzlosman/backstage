## ADDED Requirements

### Requirement: Durable removable defaults
The system SHALL seed the existing PLAN, TDD, and ROADMAP conventions as ordinary app-owned planning patterns exactly once and SHALL allow every seeded pattern to be removed permanently.

#### Scenario: Existing installation migrates
- **WHEN** an installation without planning-pattern configuration starts after the upgrade
- **THEN** the system stores canonical default patterns for the supported PLAN, TDD, and ROADMAP names
- **AND** existing candidate behavior remains available after rescan

#### Scenario: Default is removed
- **WHEN** the user removes a seeded default pattern
- **THEN** the pattern no longer participates in later classification
- **AND** restarting the application does not silently restore it

#### Scenario: Every default is removed
- **WHEN** the user removes all seeded defaults and has no custom patterns
- **THEN** the empty planning-pattern set is persisted as valid configuration
- **AND** OpenSpec recognition and All Markdown indexing continue independently

#### Scenario: Restore defaults
- **WHEN** the user activates Restore defaults
- **THEN** the system adds any missing canonical defaults
- **AND** it preserves custom patterns and avoids duplicate canonical entries

### Requirement: Validated custom planning patterns
The system SHALL let the user add and remove bounded Rust-compatible regular expressions matched against normalized project-relative Markdown paths.

#### Scenario: Add a valid pattern
- **WHEN** the user submits a non-empty expression within pattern size and count limits that compiles successfully
- **THEN** the system persists one custom planning pattern and displays its expression and Custom provenance
- **AND** the accepted expression participates in the next scan

#### Scenario: Pattern matches a nested planning path
- **WHEN** a custom pattern matches `docs/plans/launch.md`
- **THEN** that Markdown file is classified as one possible planning artifact
- **AND** the evidence identifies the accepted planning pattern

#### Scenario: Multiple patterns match one path
- **WHEN** two accepted patterns match the same Markdown path
- **THEN** the system emits one possible-artifact work record for that path
- **AND** its stable artifact identity does not depend on which matching pattern runs first

#### Scenario: Pattern is invalid
- **WHEN** an expression is empty, oversized, over the configured count limit, or fails compilation
- **THEN** the system rejects it with a specific validation message
- **AND** persisted patterns and current indexes remain unchanged

#### Scenario: Broad pattern is accepted
- **WHEN** the user deliberately adds a valid expression that matches every in-scope Markdown path
- **THEN** the system accepts the expression and explains its broad planning scope
- **AND** existing containment, file-size, depth, entry, and timeout limits still apply

#### Scenario: Non-Markdown path matches
- **WHEN** an accepted expression would match a non-Markdown file
- **THEN** the file is not indexed or classified by the planning-pattern capability

### Requirement: Pattern-driven rescans are coherent
A successful planning-pattern mutation SHALL rescan all approved roots against one persisted configuration revision and SHALL prevent stale scan results from replacing newer classification.

#### Scenario: Add pattern triggers rescans
- **WHEN** a pattern addition commits successfully
- **THEN** every approved root is scheduled for rescan with the new configuration revision
- **AND** previous snapshots remain usable until replacement snapshots succeed

#### Scenario: Remove pattern drops candidates
- **WHEN** a removed pattern was the only pattern matching a possible planning artifact
- **THEN** the artifact disappears from Plan files after the replacement scan publishes
- **AND** the Markdown remains available in All Markdown

#### Scenario: Older scan starts or finishes late
- **WHEN** a scan using an older pattern revision starts or completes after a newer-revision scan is active
- **THEN** the older scan cannot cancel the newer scan or replace its runtime or persisted configuration state

#### Scenario: Rescan fails
- **WHEN** one root cannot be rescanned after a valid pattern mutation
- **THEN** the system preserves that root's last successful snapshot and identifies it as awaiting retry
- **AND** the accepted pattern configuration remains durable

### Requirement: Pattern configuration remains local
Planning patterns and their classification SHALL remain app-owned, local, deterministic, and read-only with respect to approved repositories.

#### Scenario: Pattern is added or removed
- **WHEN** the user changes planning patterns
- **THEN** only app-owned configuration and indexes are written
- **AND** no repository metadata or Markdown file is created, edited, or deleted

#### Scenario: Classification runs
- **WHEN** a scan evaluates planning patterns
- **THEN** no Pi request or network request is made
- **AND** pattern order cannot change the classification result
