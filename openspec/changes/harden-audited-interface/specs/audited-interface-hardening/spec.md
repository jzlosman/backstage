## ADDED Requirements

### Requirement: Recoverable workspace navigation
The workspace SHALL keep the bundle ledger reachable whenever no artifact is selected and SHALL expose an explicit, named control for showing or hiding the ledger.

#### Scenario: Relaunch after a collapsed detail view
- **WHEN** Backstage starts with a saved collapsed-ledger preference but no selected artifact
- **THEN** the bundle ledger is visible so the user can select work

#### Scenario: Narrow master-detail transition
- **WHEN** a user selects a bundle on a viewport at or below the detail-collapse breakpoint
- **THEN** Backstage shows the selected artifact and preserves an explicit control that returns to the bundle ledger

### Requirement: Keyboard-complete shell
The workspace SHALL support keyboard-only use of dialogs, pane resizing, global refresh, pane focus, filters, and handoff controls.

#### Scenario: Command palette traversal
- **WHEN** the command palette is open and the user tabs past its last control or before its first control
- **THEN** focus remains within the palette until the palette closes

#### Scenario: Keyboard pane resize
- **WHEN** a focused pane separator receives an appropriate arrow key
- **THEN** the pane width changes within its documented minimum and maximum and exposes its current value to assistive technology

#### Scenario: Refresh shortcut
- **WHEN** the user presses Command-R or Control-R outside an editable field and approved roots are available
- **THEN** Backstage refreshes the approved roots without invoking browser reload behavior

### Requirement: Identifiable projects and indexed-work counts
Each project row SHALL remain identifiable at supported viewport widths and SHALL show a deterministic count of indexed planning files derived from the current index. The work registry and its aggregate counts SHALL include only projects with at least one indexed planning file.

#### Scenario: Project with indexed work
- **WHEN** a project has indexed bundle members
- **THEN** its row shows the project identity and the total member count without a backend request

#### Scenario: Discovered project without indexed work
- **WHEN** a discovered project has no indexed bundle members
- **THEN** Backstage omits it from the project registry and excludes it from aggregate work-project counts

#### Scenario: Compact project rail
- **WHEN** the viewport uses the compact project rail
- **THEN** each project retains a visible distinguishing label and a complete accessible name containing its project name and file count

### Requirement: Responsive operational hierarchy
Backstage SHALL preserve the current project context, the primary workspace action, and readable artifact content from 320 px through desktop widths without horizontal page overflow.

#### Scenario: Minimum supported width
- **WHEN** the viewport is 320 px wide
- **THEN** the titlebar, compact project rail, workspace control, and reading content remain visible and operable without clipping the product name or page-level horizontal scrolling

#### Scenario: Coarse pointer input
- **WHEN** the active input reports a coarse pointer
- **THEN** primary buttons, icon controls, filters, and file selectors expose at least 44 by 44 CSS pixels of interactive area

### Requirement: Clear artifact classification language
Backstage SHALL label recognized and candidate artifacts with factual user-facing terms while preserving the distinction between deterministic recognition and filename-based candidate evidence.

#### Scenario: Candidate planning file
- **WHEN** a file is indexed because its filename matches configured planning evidence
- **THEN** the ledger labels it as a planning candidate and explains that filename match without presenting it as recognized OpenSpec material

### Requirement: Stable and efficient reading interactions
Backstage SHALL avoid reparsing unchanged Markdown during unrelated shell updates and SHALL bound pointer-driven pane updates to animation frames.

#### Scenario: Resize with selected Markdown
- **WHEN** the user resizes a pane while the selected artifact source is unchanged
- **THEN** the rendered Markdown result is reused and pane updates occur at most once per animation frame

### Requirement: Coherent accessible visual system
Backstage SHALL use one consistent interface icon family, semantic color tokens, visible non-text contrast, and intentional reduced-motion alternatives while preserving the Accession Desk direction.

#### Scenario: Icon-only titlebar control
- **WHEN** an icon-only control is rendered
- **THEN** it uses the shared Phosphor icon system, has a visible tooltip or equivalent title, and exposes a descriptive accessible name

#### Scenario: Reduced motion
- **WHEN** the user requests reduced motion
- **THEN** repeating sweep and pulse effects stop while static labels and surfaces continue to communicate scanning and state

#### Scenario: Focus on a light surface
- **WHEN** a keyboard user focuses an interactive element on a light surface
- **THEN** its focus indicator and necessary control boundary meet WCAG non-text contrast requirements
