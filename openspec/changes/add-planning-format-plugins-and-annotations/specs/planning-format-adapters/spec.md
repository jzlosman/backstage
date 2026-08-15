## ADDED Requirements

### Requirement: Guaranteed Markdown source fallback
The system SHALL keep every safely indexed Markdown source readable through the sanitized Markdown source renderer even when no specialized format recognizes it or a specialized adapter cannot parse it.

#### Scenario: Ordinary Markdown has no specialized format
- **WHEN** a safely indexed Markdown document is not a member of any recognized specialized Work Record
- **THEN** the system exposes it as one plain-Markdown Work Record
- **AND** the reader offers exact sanitized source without specialized sections or derived lifecycle

#### Scenario: Specialized parsing fails
- **WHEN** a specialized adapter recognizes a Work Record but cannot parse some or all of its structured content
- **THEN** the recognized record retains its claim and grouped identity
- **AND** every safely readable member remains available through its Source capability
- **AND** the system reports adapter warnings without inventing structured facts or emitting duplicate plain-Markdown records

#### Scenario: Recognized members appear once in the ledger
- **WHEN** specialized format members are included in the active ledger scope
- **THEN** the specialized Work Record represents those members as one ledger entry
- **AND** the same members do not also appear as standalone plain-Markdown ledger entries

### Requirement: Deterministic compiled-in format registry
The system SHALL run planning-format recognition and grouping through an ordered registry of compiled-in adapters using normalized, contained source observations and without model invocation.

#### Scenario: OpenSpec record is recognized through the registry
- **WHEN** a project contains supported current or archived OpenSpec members
- **THEN** the OpenSpec adapter emits one Work Record for each recognized change
- **AND** its recognition records the adapter identifier and version

#### Scenario: Existing planning pattern matches
- **WHEN** an unclaimed Markdown source matches a configured planning-path pattern
- **THEN** the planning-pattern adapter emits the existing one-source possible Work Record with deterministic evidence
- **AND** the pattern match does not override a recognized specialized record

#### Scenario: Adapter results are stable
- **WHEN** the same project identity, source paths, source metadata, adapter versions, and configuration are scanned again
- **THEN** the registry emits the same record keys, grouping, recognition provenance, and deterministic ordering

#### Scenario: Specialized adapters overlap
- **WHEN** more than one specialized adapter claims the same source document
- **THEN** the registry selects the winner by explicit adapter precedence
- **AND** it records a visible overlap warning naming the competing adapters
- **AND** the source remains readable

#### Scenario: Registry executes during background scan
- **WHEN** an approved root is scanned
- **THEN** format recognition performs no network request and invokes no Pi generation
- **AND** repository files remain unchanged

### Requirement: Neutral Work Record contract
The system SHALL represent plain Markdown and specialized planning formats through a format-neutral Work Record contract containing identity, display metadata, recognition, source membership, deterministic summary facets, warnings, and available view capabilities.

#### Scenario: OpenSpec behavior migrates to the neutral contract
- **WHEN** a current, done, or archived OpenSpec record is indexed and opened
- **THEN** its custody, task progress, primary status, Overview, Tasks, Source, warnings, and handoff behavior remain available
- **AND** those behaviors are supplied through the OpenSpec adapter rather than top-level OpenSpec branches

#### Scenario: Plain Markdown uses the same ledger contract
- **WHEN** an ordinary Markdown record is composed with specialized records
- **THEN** common search, recency, selection, provenance, and source-reading behavior use the same Work Record envelope
- **AND** absent specialized capabilities are represented as unavailable rather than as OpenSpec defaults

#### Scenario: Adapter exposes structured content
- **WHEN** a recognized adapter can derive structured content from a fresh contained source snapshot
- **THEN** it returns neutral summary blocks, collections, relationships, sources, and warnings supported by the compiled frontend renderer
- **AND** it does not provide executable frontend code

#### Scenario: Scan produces indexed facts
- **WHEN** a detected specialized record has safely captured bounded scan snapshots
- **THEN** its pure adapter summarizer derives the source fingerprint, progress, status, counts, warnings, and other supported ledger facts before the record opens
- **AND** the adapter performs no source I/O itself

#### Scenario: Scan snapshot is incomplete
- **WHEN** one or more detected members cannot be captured safely during indexing
- **THEN** the adapter summarizes only facts supported by the complete captured inputs
- **AND** reports unavailable facts and source warnings instead of inventing values

### Requirement: Generated-view continuity
The system SHALL key generated views to the neutral Work Record subject while preserving existing generated-view provenance, freshness, failure, and root-removal behavior for records that already support generation.

#### Scenario: Existing current summary is migrated
- **WHEN** an existing generated summary's legacy bundle owner maps unambiguously to a neutral Work Record subject
- **THEN** the summary remains available with its source fingerprint, included paths, prompt version, output, generation time, and model
- **AND** its current or stale state is assessed against the neutral record's fresh fingerprint

#### Scenario: Existing stale summary is migrated
- **WHEN** a mapped legacy summary fingerprint differs from the current neutral record fingerprint
- **THEN** the prior summary remains readable as stale
- **AND** regeneration remains an explicit user action

#### Scenario: Legacy cache owner cannot be mapped
- **WHEN** a legacy generated-view row cannot be mapped unambiguously from a translated index or accepted rescan
- **THEN** the system may discard that derived cache row
- **AND** it does not discard repository sources or private annotations

#### Scenario: Root removal prunes generated views
- **WHEN** a Work Record subject loses its final approved-root route
- **THEN** generated views owned by that subject are deleted with its other app-owned private state
- **AND** views remain when another approved-root route still reaches the subject

### Requirement: Fresh contained adapter details
The system SHALL build specialized detail views from a fresh bounded snapshot of the selected record's currently indexed source members and SHALL reject stale detail responses after selection or source membership changes.

#### Scenario: Source changes after indexing
- **WHEN** a recognized record member changes after indexing but before the record opens
- **THEN** the detail view uses the fresh contained snapshot as authoritative
- **AND** updated deterministic facts and warnings are shown without mutating the source

#### Scenario: Delayed adapter response loses selection race
- **WHEN** a detail response completes after the user selects another Work Record
- **THEN** the delayed response is ignored
- **AND** the newer selection remains visible

#### Scenario: Member no longer resolves safely
- **WHEN** a selected member disappears, exceeds read bounds, becomes non-UTF-8, or no longer resolves beneath an approved root
- **THEN** the system reports a recoverable source warning or unavailable detail
- **AND** the remaining safely captured members stay readable
