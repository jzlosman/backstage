## ADDED Requirements

### Requirement: Bounded Markdown discovery
The system SHALL record every regular file with a case-insensitive `.md` extension reached during the existing project-scoped, contained, budgeted catalog walk. Ordinary Markdown MUST remain distinct from recognized or candidate planning evidence.

#### Scenario: Ordinary Markdown is indexed without becoming planning work
- **WHEN** an approved project contains `README.md` and no recognized planning files
- **THEN** the index records `README.md` as a Markdown document
- **AND** the project has no planning bundle solely because that document exists

#### Scenario: Existing scan limits still apply
- **WHEN** a Markdown file is excluded, beyond the configured depth or entry budget, outside the project boundary, or reached only through an escaping symlink
- **THEN** the system does not index it
- **AND** the system does not weaken containment or mutate the repository

#### Scenario: Markdown identity is deterministic
- **WHEN** the same project-relative Markdown path is scanned again without a project identity change
- **THEN** the document receives the same stable identity and deterministic ordering

### Requirement: Planning-focused default scope
The registry SHALL start in a scope labeled `Plan files` and SHALL preserve the existing planning-only project and bundle presentation until the user explicitly chooses `All Markdown`.

#### Scenario: App opens in the planning scope
- **WHEN** the application starts with projects containing planning artifacts and ordinary Markdown
- **THEN** `Plan files` is selected
- **AND** ordinary standalone Markdown and Markdown-only projects are absent from the registry and ledger

#### Scenario: Scope is not persisted
- **WHEN** the user selects `All Markdown` and later starts a new application session
- **THEN** the registry starts in `Plan files`

### Requirement: Complete All Markdown scope
The user SHALL be able to select `All Markdown` and access every indexed Markdown document exactly once, either through its recognized planning bundle or as a standalone Markdown row.

#### Scenario: Broader scope reveals ordinary Markdown
- **WHEN** the user selects `All Markdown`
- **THEN** Markdown-only projects become visible
- **AND** ordinary indexed Markdown appears in the ledger with document-specific labeling and provenance

#### Scenario: OpenSpec members are not duplicated
- **WHEN** an indexed Markdown file is already a member of a visible recognized OpenSpec bundle
- **THEN** `All Markdown` keeps the bundle as the single ledger entry representing that member
- **AND** the file remains accessible through the bundle Source view

#### Scenario: Counts use unique files
- **WHEN** `All Markdown` contains planning bundles and standalone documents
- **THEN** project and aggregate file counts count each indexed Markdown document once

### Requirement: Scope-aware navigation and filtering
The system SHALL derive project visibility, ledger rows, search results, counts, selection, and empty states from the active registry scope without assigning planning states to ordinary Markdown.

#### Scenario: Search includes ordinary Markdown only in broad scope
- **WHEN** an ordinary Markdown filename or path matches the search query
- **THEN** it appears in `All Markdown`
- **AND** it remains absent from `Plan files`

#### Scenario: Broad result set remains fully reachable
- **WHEN** `All Markdown` matches more rows than the bounded initial ledger batch
- **THEN** the ledger reports counts from the complete matching set while mounting only the bounded batch
- **AND** the user can reveal additional batches or search directly for any indexed filename or path

#### Scenario: Planning state filter does not reclassify documents
- **WHEN** `All Markdown` is active and the user chooses a planning-only state filter such as `Unfinished`
- **THEN** standalone Markdown documents are excluded from that filtered result
- **AND** they are not assigned invented progress, warning, stale, or completion state

#### Scenario: Scope switch removes the current row
- **WHEN** the selected standalone document is visible in `All Markdown` and the user switches to `Plan files`
- **THEN** the app replaces it with the first visible planning selection or the planning empty state
- **AND** a delayed document response cannot restore the hidden selection

### Requirement: Safe generic Markdown reading
The system SHALL resolve standalone Markdown only by a current indexed document identity and SHALL read it through the contained immutable snapshot boundary before rendering its exact bounded UTF-8 source in the generic Markdown reader.

#### Scenario: Ordinary Markdown opens in the generic reader
- **WHEN** the user selects an indexed standalone Markdown row
- **THEN** the app displays its rendered Markdown and source provenance
- **AND** it does not display OpenSpec Overview or Tasks navigation

#### Scenario: Unsafe or unavailable source fails visibly
- **WHEN** the selected source is missing, oversized, non-UTF-8, unstable during capture, or no longer resolves beneath the approved root
- **THEN** the read is rejected with a recoverable visible error
- **AND** repository content is not mutated

#### Scenario: Delayed response cannot replace a newer selection
- **WHEN** a document read completes after the user has selected a different bundle or document
- **THEN** the stale response is ignored
- **AND** the newer selection remains displayed

### Requirement: Local deterministic browsing
Indexing and reading ordinary Markdown SHALL remain local and deterministic. Selecting a document or enabling `All Markdown` MUST NOT invoke Pi or expose repository content to a generated-view workflow.

#### Scenario: Broad browsing does not generate content
- **WHEN** the user enables `All Markdown` or opens an ordinary Markdown document
- **THEN** no Pi generation request is made
- **AND** the document receives no generated summary or continuation prompt by default
