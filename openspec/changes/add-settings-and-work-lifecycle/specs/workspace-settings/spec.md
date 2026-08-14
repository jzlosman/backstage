## ADDED Requirements

### Requirement: Dedicated settings surface
The system SHALL provide an in-app Settings surface for app-owned configuration and SHALL keep configuration controls out of the project registry.

#### Scenario: Open Settings from the titlebar
- **WHEN** the user activates the named Settings control
- **THEN** the pane workspace is replaced by the Settings surface
- **AND** approved roots and planning patterns are presented as separate ruled sections

#### Scenario: Open Settings from the command palette
- **WHEN** the user invokes Settings from the command palette
- **THEN** the same Settings surface opens
- **AND** keyboard focus moves to its heading or first actionable control

#### Scenario: Return to work
- **WHEN** the user closes Settings
- **THEN** the prior visible work selection is restored when it still exists
- **AND** focus returns to the control that opened Settings

#### Scenario: Registry remains work-focused
- **WHEN** the work workspace is visible
- **THEN** the project registry does not render an Approved Roots footer
- **AND** root configuration remains available through Settings

### Requirement: Approved-root management
The system SHALL list every approved root in Settings and SHALL provide explicit Add root and Remove actions without modifying the approved directory.

#### Scenario: Add a root
- **WHEN** the user selects a valid local directory through Add root
- **THEN** the system persists its approval and begins a contained scan
- **AND** the root appears once in Settings

#### Scenario: Add an already approved root
- **WHEN** the user selects a directory that is already approved
- **THEN** the system keeps one approval
- **AND** the interface reports that the root was already present without duplicating it

#### Scenario: Cancel root selection
- **WHEN** the user cancels the directory picker
- **THEN** settings and indexes remain unchanged

### Requirement: Coordinated root removal
The system SHALL remove an approval, its unreachable app-owned index and generated data, and its visible records as one coordinated operation while leaving repository contents unchanged.

#### Scenario: Confirm root removal
- **WHEN** the user activates Remove for an approved root
- **THEN** the interface explains that Backstage data will be removed and repository files will remain untouched
- **AND** no removal occurs until the user confirms

#### Scenario: Remove a root successfully
- **WHEN** the user confirms removal of an approved root
- **THEN** the system cancels and forgets scans for that root, deletes its approval and index, and removes records no longer reachable from another root
- **AND** the system does not write to, move, or delete the approved directory

#### Scenario: In-flight summary completes after removal
- **WHEN** summary generation is running for a bundle whose final approved root is removed
- **THEN** the request is cancelled and any delayed result is rejected before persistence or publication
- **AND** generated content cannot recreate app-owned data for the removed bundle

#### Scenario: Overlapping root remains approved
- **WHEN** a removed root and a retained root both reach the same project or bundle
- **THEN** records and generated views still reachable through the retained root remain available
- **AND** only the removed approval's unique app-owned state is deleted

#### Scenario: Selected record disappears
- **WHEN** root removal makes the selected record unreachable
- **THEN** the interface selects the first current visible record or shows the appropriate empty state
- **AND** a delayed scan or detail response cannot restore the removed selection

#### Scenario: Removal fails
- **WHEN** app-owned persistence fails before root removal commits
- **THEN** the prior approval, index, and visible state remain usable
- **AND** Settings shows a recoverable error without changing repository contents

#### Scenario: Root was already removed
- **WHEN** a removal command names an unknown approval
- **THEN** the system reports a not-found outcome
- **AND** it does not treat the repository path as a filesystem failure

### Requirement: Settings states remain operable
The Settings surface SHALL provide explicit loading, empty, validation, saving, scanning, failure, and success feedback without blocking unrelated retained roots.

#### Scenario: No roots are approved
- **WHEN** Settings opens with no approved roots
- **THEN** the roots section explains that no folders are being scanned
- **AND** Add root remains the primary action

#### Scenario: One retained root fails to scan
- **WHEN** a scan fails for one approved root
- **THEN** that root identifies the failure and offers retry
- **AND** other root rows and planning-pattern controls remain operable

#### Scenario: Narrow window
- **WHEN** Settings is shown below the existing narrow-window breakpoint
- **THEN** paths wrap or truncate with an accessible full value and row actions remain reachable
- **AND** no horizontal page scrolling is required
