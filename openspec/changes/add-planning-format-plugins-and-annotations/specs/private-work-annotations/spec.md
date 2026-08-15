## ADDED Requirements

### Requirement: Private app-owned Work Record annotations
The system SHALL store user annotations in app-owned local storage against Work Record subjects and SHALL neither read nor write annotation frontmatter in repository files.

#### Scenario: User annotates a plain Markdown record
- **WHEN** the user changes an annotation on a plain-Markdown Work Record
- **THEN** the system persists the annotation outside the scanned repository
- **AND** the source Markdown remains byte-for-byte unchanged

#### Scenario: User annotates a grouped record
- **WHEN** the user changes an annotation on an OpenSpec or Wayfinder Work Record
- **THEN** the annotation applies to the grouped Work Record
- **AND** no member file, task, question, or heading is edited or independently annotated

#### Scenario: Record has no saved annotation
- **WHEN** a Work Record has no annotation row
- **THEN** its effective decision is Undecided and its disposition is Applicable
- **AND** favorite and todo are false and priority is unset

#### Scenario: Repository contains annotation-like frontmatter
- **WHEN** a source document contains `backstage`, approval, favorite, todo, priority, obsolete, supersession, or similar frontmatter fields
- **THEN** the annotation system ignores those fields as annotation state
- **AND** only app-owned annotation storage determines the effective private annotation

#### Scenario: Annotation operation runs
- **WHEN** the user creates, changes, or clears an annotation
- **THEN** the operation performs no network request and invokes no Pi generation

### Requirement: Explicit decision and disposition states
The system SHALL model user decision and disposition as independent annotations so private intent remains distinct from source-derived lifecycle and progress.

#### Scenario: Approved plan is not ready to build
- **WHEN** the user marks a Work Record Approved and Todo
- **THEN** both annotations are visible together
- **AND** the system does not change source task progress or claim that implementation has begun

#### Scenario: Alternative is rejected
- **WHEN** the user marks a Work Record Rejected
- **THEN** it remains searchable and readable with a Rejected annotation
- **AND** it is not relabeled Obsolete unless the user separately changes its disposition

#### Scenario: Previously approved record becomes obsolete
- **WHEN** the user changes an Approved Work Record's disposition to Obsolete
- **THEN** the system retains Approved as its decision annotation
- **AND** displays Obsolete independently of format-derived status

#### Scenario: OpenSpec lifecycle and annotation coexist
- **WHEN** an OpenSpec Work Record is source-derived as Done or Archived and privately marked Approved, Rejected, Obsolete, or Superseded
- **THEN** the interface presents both facts with distinct labels
- **AND** neither state overwrites or derives the other

### Requirement: Favorite, todo, and priority markers
The system SHALL let the user independently favorite a Work Record, mark it Todo, and assign no priority or Low, Medium, or High priority.

#### Scenario: Multiple markers coexist
- **WHEN** a Work Record is Approved, favorited, Todo, and High priority
- **THEN** all selected annotations remain stored and visible
- **AND** no marker silently clears another

#### Scenario: User clears a marker
- **WHEN** the user removes favorite, todo, or priority from a Work Record
- **THEN** only that marker returns to its default
- **AND** the record's other annotations remain unchanged

#### Scenario: Annotation filter is active
- **WHEN** the user filters by decision, disposition, favorite, todo, or priority
- **THEN** the ledger includes every matching visible Work Record regardless of format
- **AND** ordinary records are not assigned source-derived planning status to satisfy the filter

#### Scenario: No annotation filter is active
- **WHEN** the ledger uses its default scope and ordering
- **THEN** annotations remain visible but do not replace the existing source-recency ordering

### Requirement: Valid supersession relationship
The system SHALL represent Superseded as a disposition with one distinct replacement Work Record subject and SHALL reject self-reference and supersession cycles.

#### Scenario: User selects a valid replacement
- **WHEN** the user marks record A Superseded by visible record B
- **THEN** the system stores a typed relationship from A to B
- **AND** the UI links to B using its current display details

#### Scenario: User attempts self-supersession
- **WHEN** the user selects the same Work Record as both obsolete record and replacement
- **THEN** the system rejects the change without altering the existing annotation

#### Scenario: User attempts a supersession cycle
- **WHEN** a requested relationship would make a Work Record directly or transitively supersede itself
- **THEN** the system rejects the change and identifies the conflicting chain

#### Scenario: User chooses Obsolete without a replacement
- **WHEN** a Work Record is no longer authoritative but has no known replacement
- **THEN** the user can mark it Obsolete without creating a target relationship

### Requirement: Annotation durability independent of index snapshots
The system SHALL persist subjects and annotations separately from replaceable index snapshots and SHALL reconcile them by stable app-owned subject identity and exact adapter record locator.

#### Scenario: Record survives rescan
- **WHEN** a record with the same project identity, format adapter, and adapter record key is rediscovered
- **THEN** it resolves to the same app-owned subject
- **AND** its annotations survive the rescan

#### Scenario: Application restarts
- **WHEN** Backstage restarts and reloads or rebuilds indexes
- **THEN** annotations for rediscovered subjects remain available
- **AND** no repository read is treated as annotation authority

#### Scenario: Record temporarily disappears
- **WHEN** an annotated record is absent from a scan while its approved root remains registered
- **THEN** the system retains its subject and annotations as unavailable app-owned state
- **AND** restores them if the exact record locator reappears

#### Scenario: Record path or format identity changes
- **WHEN** a source move or format change produces a different exact adapter record locator
- **THEN** the system does not heuristically transfer private annotations to the new record
- **AND** the prior subject remains unavailable rather than silently pointing at unrelated work

#### Scenario: OpenSpec change moves into archival custody
- **WHEN** a current OpenSpec directory disappears and an archived directory is recognized at a different exact adapter record locator
- **THEN** the archived record receives a distinct subject without automatic annotation transfer
- **AND** current and archived copies with the same display name never share one subject

#### Scenario: Adapter implementation version changes
- **WHEN** a compiled adapter version changes but retains the same stable format identifier and adapter record-key semantics
- **THEN** the exact record resolves to the same subject
- **AND** its annotations remain available

### Requirement: Missing and forgotten replacement targets
The system SHALL preserve a supersession relationship when its target is temporarily unavailable and SHALL remove private knowledge when an approved root is explicitly forgotten.

#### Scenario: Replacement source temporarily disappears
- **WHEN** a replacement subject is not present in the current index but its approved root remains registered
- **THEN** the superseded record retains the relationship
- **AND** the UI marks the target unavailable and shows its last-known non-sensitive display details

#### Scenario: Root removal leaves no route to an annotated subject
- **WHEN** an approved root is removed and an annotated subject is unreachable through every retained approved root
- **THEN** the system deletes that subject's private annotations and last-known source details
- **AND** it does not modify the former repository

#### Scenario: Forgotten target has incoming supersession links
- **WHEN** a forgotten subject was the replacement for a retained superseded record
- **THEN** the retained record becomes Obsolete without a replacement
- **AND** the UI does not retain the forgotten target's name or path

#### Scenario: Overlapping root still reaches the subject
- **WHEN** one approved root is removed but another retained root still reaches the same Work Record subject
- **THEN** the subject, annotations, and valid replacement relationships remain intact

#### Scenario: Unavailable subject belongs to another retained root
- **WHEN** a subject is temporarily absent from the current index under retained root A and unrelated root B is removed
- **THEN** the historical route to retained root A preserves the subject and its annotations
- **AND** removing root B does not treat source absence under root A as an explicit forget command
