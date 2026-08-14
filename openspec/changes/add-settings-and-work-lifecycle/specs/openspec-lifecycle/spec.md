## ADDED Requirements

### Requirement: Current and archived OpenSpec recognition
The system SHALL recognize supported OpenSpec members in both current change directories and standard archived change directories and SHALL group either location as an OpenSpec change bundle.

#### Scenario: Current change is discovered
- **WHEN** supported members exist below `openspec/changes/<change>/`
- **THEN** the system groups them as one current OpenSpec change bundle

#### Scenario: Archived change is discovered
- **WHEN** supported members exist below `openspec/changes/archive/YYYY-MM-DD-<change>/`
- **THEN** the system groups them as one archived OpenSpec change bundle
- **AND** it does not demote those members to standalone Markdown records

#### Scenario: Current and archived copies coexist
- **WHEN** current and archived directories resolve to the same display change name
- **THEN** the system keeps two distinct bundle identities based on their full source directories
- **AND** each bundle opens its own contained source members

#### Scenario: Manual archive name has no valid date
- **WHEN** supported members exist under the archive directory but its folder lacks a valid date prefix
- **THEN** the system still recognizes Archived custody
- **AND** archive date is reported as unavailable rather than inferred

### Requirement: Archival custody and task progress remain distinct
The system SHALL preserve OpenSpec custody and parsed task progress as independent deterministic facts and SHALL derive the primary work status without equating completion with archival.

#### Scenario: Current change has open tasks
- **WHEN** a current OpenSpec change has available progress with at least one open task
- **THEN** its primary status is Active
- **AND** its open and done counts are displayed separately

#### Scenario: Current change has no open tasks
- **WHEN** a current OpenSpec change has available progress with zero open tasks
- **THEN** its primary status is Done
- **AND** it remains current rather than being labeled Archived

#### Scenario: Current progress is unavailable
- **WHEN** a current OpenSpec change has no safely parsed task progress
- **THEN** its primary status is Active
- **AND** the interface labels progress unavailable instead of inventing open or done counts

#### Scenario: Archived change has open tasks
- **WHEN** an archived OpenSpec change has available progress with open tasks
- **THEN** its primary status is Archived
- **AND** its open and done counts remain visible

#### Scenario: Archived change has every task done
- **WHEN** an archived OpenSpec change has available progress with zero open tasks
- **THEN** its primary status remains Archived
- **AND** the completed task facts remain visible

#### Scenario: Archived progress is unavailable
- **WHEN** an archived OpenSpec change has no safely parsed task progress
- **THEN** its primary status remains Archived
- **AND** progress unavailability is reported separately

### Requirement: Uniform OpenSpec reading experience
Current, Done, and Archived OpenSpec bundles SHALL use the same Overview, Tasks, and Source reader and the same contained read and provenance boundaries.

#### Scenario: Open an archived change
- **WHEN** the user selects an archived OpenSpec bundle
- **THEN** the reading desk opens its Overview, Tasks, and Source tabs
- **AND** all source reads remain contained beneath the approved root

#### Scenario: View archived tasks
- **WHEN** an archived change has a readable tasks file
- **THEN** the Tasks tab displays every parsed open and done task using the same layout as a current change
- **AND** Archived custody remains visible in the reading-desk header

#### Scenario: Copy an archived path
- **WHEN** the user copies a path or continuation prompt for an archived change
- **THEN** the handoff uses the exact archived source path
- **AND** it identifies Archived custody without claiming the change is active

#### Scenario: Archived source changes after indexing
- **WHEN** an archived source member changes before it is opened
- **THEN** the fresh contained snapshot is authoritative for the reader and progress
- **AND** a stale detail response cannot replace a newer selection

### Requirement: Lifecycle-aware navigation
The work ledger SHALL make Active, Done, and Archived statuses explicit and SHALL keep archived changes out of the default Current result while preserving direct access through the Archived filter and search.

#### Scenario: Application opens
- **WHEN** current and archived work has been indexed
- **THEN** the ledger starts in the Current filter
- **AND** current Active and Done work is visible while Archived work is excluded

#### Scenario: Archived filter is selected
- **WHEN** the user selects Archived
- **THEN** archived OpenSpec bundles appear with their archive status and task counts
- **AND** ordinary current documents are excluded

#### Scenario: Search archived work
- **WHEN** the Archived filter is active and search matches an archived change name or member path
- **THEN** the matching archived bundle is reachable through the ledger

#### Scenario: Switch away from selected archive
- **WHEN** an archived bundle is selected and the user returns to Current
- **THEN** the interface selects the first visible current record or shows the current empty state
- **AND** delayed archived responses cannot restore the hidden selection
