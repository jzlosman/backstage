## ADDED Requirements

### Requirement: Persistent three-pane workspace
The system SHALL present a single desktop window with a permanent project rail, an artifact-bundle ledger, and a selected-artifact reading/detail pane.

#### Scenario: Open the application with indexed work
- **WHEN** at least one project and artifact bundle is indexed
- **THEN** the project rail remains anchored while project selection filters the ledger and artifact selection updates the detail pane

#### Scenario: Resize the desktop window
- **WHEN** the window becomes too narrow for all three panes
- **THEN** the system may collapse the bundle ledger behind a reversible control but keeps the project rail accessible and preserves selection

### Requirement: Cross-project All Work view
The system SHALL open to an All Work view that can surface unfinished, stale, warning-bearing, and recently changed artifacts across projects while retaining project filters.

#### Scenario: Open with mixed project states
- **WHEN** indexed projects contain current, stale, incomplete, and warning-bearing bundles
- **THEN** All Work presents those artifacts with explicit state labels and allows filtering by project and deterministic state

#### Scenario: No artifacts found
- **WHEN** approved roots contain projects but no recognized or possible artifacts
- **THEN** the workspace explains what was scanned, offers refresh and root controls, and does not imply scan failure

### Requirement: Structured artifact detail
The system SHALL show source provenance, deterministic status, OpenSpec progress when available, warnings, Markdown, generated views, and handoff actions without exposing a full source tree.

#### Scenario: Select an OpenSpec bundle
- **WHEN** the user selects a recognized OpenSpec bundle
- **THEN** the detail pane shows bundle files, task progress, remaining tasks, source provenance, warnings, and rendered Markdown

#### Scenario: Select a possible artifact
- **WHEN** the user selects a possible artifact
- **THEN** the detail pane shows its candidacy reason, source provenance, Markdown, and no unsupported structured progress

### Requirement: Keyboard-first navigation with pointer parity
The system SHALL provide visible keyboard focus, predictable navigation among projects, bundles, artifact content, actions, and a command palette, with equivalent pointer access.

#### Scenario: Navigate without a pointer
- **WHEN** the user operates the workspace using the keyboard
- **THEN** they can move between panes, change selections, search, open commands, trigger permitted actions, and return focus without a trap

#### Scenario: Invoke the command palette
- **WHEN** the user invokes the global command shortcut
- **THEN** the system opens a searchable list of currently valid commands and restores focus when it closes

#### Scenario: Use a pointer
- **WHEN** the user uses mouse or trackpad controls
- **THEN** all keyboard-accessible primary actions remain available through visible or discoverable pointer affordances

### Requirement: Honest loading, empty, warning, and failure states
The system SHALL distinguish first-run, scanning, ready, ready-with-warnings, unavailable, empty, generating, stale, and failed states while preserving prior usable data where available.

#### Scenario: First run
- **WHEN** no approved root exists
- **THEN** the workspace explains local read-only discovery and provides a clear action to approve a root

#### Scenario: Refresh with prior index
- **WHEN** a refresh is running and a previous index exists
- **THEN** the prior index remains usable with a non-blocking refresh indicator

#### Scenario: Generation with prior view
- **WHEN** regeneration is running and a previous view exists
- **THEN** the previous view remains readable and is labeled as the prior result until generation finishes

### Requirement: Read-only handoff actions
The system SHALL let the user copy a selected artifact path, copy a continuation prompt, open a terminal at the project root, and invoke a configured supported external target without mutating repository content.

#### Scenario: Copy artifact path
- **WHEN** the user requests the selected artifact path
- **THEN** the system copies its normalized local path and reports completion without changing the artifact

#### Scenario: Copy continuation prompt
- **WHEN** the user requests a continuation prompt
- **THEN** the system derives a prompt containing the project path, selected bundle, deterministic status, and explicit continuation instructions while labeling or omitting generated claims

#### Scenario: Open terminal
- **WHEN** the user requests a terminal for a selected project
- **THEN** the system asks the platform launcher to open at the project root without running a repository command

#### Scenario: Unsupported external integration
- **WHEN** the user requests an external target with no configured supported contract
- **THEN** the system explains that the integration is unavailable and offers copy-path or copy-prompt alternatives

### Requirement: Accessible state communication
The system SHALL communicate selection, progress, freshness, warning, failure, and focus through text or shape in addition to color and SHALL honor reduced-motion preferences.

#### Scenario: Review a stale summary without color
- **WHEN** color cues are unavailable or indistinguishable
- **THEN** the summary remains identifiable as stale through explicit text and an associated regenerate action

#### Scenario: Reduced motion enabled
- **WHEN** the operating system requests reduced motion
- **THEN** the interface avoids nonessential animation while preserving state-change feedback
