## ADDED Requirements

### Requirement: Explicit Pi invocation
The system SHALL invoke Pi only after an explicit user request for a supported generated-view mode and SHALL NOT use Pi during scanning, classification, or progress calculation.

#### Scenario: Request a summary
- **WHEN** the user explicitly requests a summary for a selected readable scope
- **THEN** the system prepares a bounded source snapshot and requests one Pi generation

#### Scenario: Browse without generation
- **WHEN** the user scans, filters, selects, or reads artifacts without requesting a generated view
- **THEN** the system performs no Pi invocation

### Requirement: Bounded untrusted source snapshot
The system SHALL include only approved-root artifact paths in a generation snapshot, SHALL enforce configured file and byte limits, and SHALL identify repository content to Pi as untrusted quoted source rather than executable instructions.

#### Scenario: Scope fits limits
- **WHEN** the selected files remain beneath approved roots and fit configured limits
- **THEN** the system creates an immutable snapshot containing the selected normalized paths, contents, and source fingerprint

#### Scenario: Scope exceeds limits
- **WHEN** the selected scope exceeds configured file or byte limits
- **THEN** the system rejects generation, asks the user to narrow the scope, and invokes no Pi process

#### Scenario: Selected path escapes approval
- **WHEN** any selected path resolves outside approved roots before snapshot creation
- **THEN** the system rejects generation and reads no escaped content

### Requirement: Read-only Pi process boundary
The system SHALL run Pi without repository-write authority and SHALL NOT use a scanned repository as Pi's working directory.

#### Scenario: Launch Pi generation
- **WHEN** a valid snapshot is ready
- **THEN** the Pi adapter invokes the configured noninteractive command in an app-owned or isolated location with only the bounded snapshot and generation prompt

#### Scenario: Safe invocation unavailable
- **WHEN** the configured Pi command cannot satisfy the read-only process boundary
- **THEN** the system disables generation, explains the configuration problem, and preserves deterministic artifact features

### Requirement: Generated-view provenance and cache
The system SHALL cache successful generated views in app-owned local storage with mode, included paths, source fingerprint, generation time, model, and prompt version.

#### Scenario: Generation succeeds
- **WHEN** Pi returns a valid result for the current source snapshot
- **THEN** the system stores the result and provenance and marks the view current

#### Scenario: Equivalent cached view exists
- **WHEN** a cached result has the same mode, source fingerprint, and prompt version as the requested view
- **THEN** the system presents that result as current and allows explicit regeneration

### Requirement: Fingerprint-based freshness
The system SHALL mark a cached generated view stale whenever its recorded source fingerprint differs from the current fingerprint for that scope and SHALL use changed paths and dates only to explain that decision.

#### Scenario: Source changes after generation
- **WHEN** an included file or bundle membership changes after a result was generated
- **THEN** the system keeps the result readable, marks it stale, identifies known changed inputs, and offers regeneration

#### Scenario: Source changes during generation
- **WHEN** the current source fingerprint changes after generation starts but before the result returns
- **THEN** the system may cache the returned result with its original provenance but marks it stale immediately

#### Scenario: Timestamp changes without content change
- **WHEN** source modification metadata changes but normalized content and bundle membership produce the same fingerprint
- **THEN** the system keeps the generated view current

### Requirement: Failure preserves prior output
The system SHALL preserve the previous generated view when a generation or cache operation fails and SHALL report failure separately from artifact state.

#### Scenario: Regeneration times out
- **WHEN** a stale view exists and Pi times out during explicit regeneration
- **THEN** the system marks generation failed, keeps the stale result readable, and permits another explicit attempt

#### Scenario: Pi response is malformed
- **WHEN** Pi returns an unusable response
- **THEN** the system stores no replacement result and keeps deterministic facts and any prior generated view unchanged

### Requirement: Generated output labeling
The system SHALL label generated text with its mode and provenance and SHALL prevent generated output from changing deterministic progress, artifact classification, or lifecycle status.

#### Scenario: Display a current summary
- **WHEN** a cached summary is shown
- **THEN** the interface labels it as Pi-generated and displays its generation time and current or stale state

#### Scenario: Summary contradicts parsed tasks
- **WHEN** generated text conflicts with deterministic OpenSpec task facts
- **THEN** deterministic facts remain authoritative and the generated text cannot alter them
