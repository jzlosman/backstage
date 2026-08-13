## ADDED Requirements

### Requirement: Deterministic artifact recognition
The system SHALL recognize supported artifacts from deterministic path, filename, frontmatter, and structure rules and SHALL label uncertain Markdown as a possible artifact rather than a confirmed type.

#### Scenario: Recognize an OpenSpec change
- **WHEN** a project contains a supported OpenSpec change directory with expected proposal, design, spec, or task material
- **THEN** the system groups the related files into one recognized OpenSpec bundle

#### Scenario: Identify a possible artifact
- **WHEN** a Markdown file matches a candidate naming rule but not a supported artifact structure
- **THEN** the system presents it as a possible artifact with the deterministic reason for candidacy

#### Scenario: Ignore ordinary Markdown
- **WHEN** a Markdown file matches neither a supported artifact structure nor a configured deterministic candidate rule
- **THEN** the system excludes it from the artifact index without invoking Pi

### Requirement: OpenSpec task progress
The system SHALL derive OpenSpec completion from parseable task markers and SHALL expose total, completed, and remaining task facts without using AI interpretation.

#### Scenario: Parse mixed task states
- **WHEN** a supported OpenSpec task file contains completed and incomplete task markers
- **THEN** the system reports exact totals and identifies the remaining task text and source locations

#### Scenario: No supported task structure
- **WHEN** an OpenSpec bundle has no parseable supported task markers
- **THEN** the system reports progress as unavailable instead of inventing a percentage

### Requirement: Graceful parse degradation
The system SHALL preserve source access when artifact parsing is partial or fails and SHALL attach parse warnings to the affected file or bundle.

#### Scenario: Malformed OpenSpec content
- **WHEN** an expected OpenSpec file is malformed but readable
- **THEN** the system keeps the file in its bundle, renders its Markdown, and shows a parse warning

#### Scenario: File changes during parsing
- **WHEN** a file changes between metadata inspection and content parsing
- **THEN** the system discards the inconsistent parse result and requests re-indexing without hiding the previous usable result

### Requirement: Provenance and classification labels
The system SHALL expose artifact path, project, bundle membership, source modification time, available Git context, parser provenance, and whether each displayed statement is an observed fact, deterministic assessment, heuristic, or AI-generated output.

#### Scenario: Inspect a parsed artifact
- **WHEN** the user selects a recognized artifact
- **THEN** the detail surface shows its path, project, bundle, source dates, available Git context, parser status, and deterministic progress

#### Scenario: Show a staleness heuristic
- **WHEN** a deterministic age rule labels an artifact as possibly stale
- **THEN** the interface labels the result as a heuristic and does not describe the artifact as abandoned

### Requirement: Markdown rendering
The system SHALL render readable Markdown for selected artifact files without providing repository-editing controls.

#### Scenario: Render supported Markdown
- **WHEN** the user selects a readable Markdown artifact
- **THEN** the system renders headings, lists, task markers, tables, links, and code blocks while preserving access to the source path

#### Scenario: Unsafe embedded content
- **WHEN** Markdown contains raw HTML, scripts, external resources, or executable instructions
- **THEN** the renderer sanitizes or blocks active content and treats text instructions as untrusted repository content

### Requirement: Bundle source fingerprint
The system SHALL compute a deterministic fingerprint over the normalized included paths, file contents, and bundle membership used by downstream generated views.

#### Scenario: Included content changes
- **WHEN** the contents of an included file change
- **THEN** the bundle source fingerprint changes

#### Scenario: Bundle membership changes
- **WHEN** a relevant file is added to or removed from a bundle
- **THEN** the bundle source fingerprint changes even if remaining file contents are unchanged
