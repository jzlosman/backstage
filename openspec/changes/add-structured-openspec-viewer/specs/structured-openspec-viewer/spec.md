## ADDED Requirements

### Requirement: Deterministic OpenSpec overview
Backstage SHALL open recognized OpenSpec change bundles in an Overview that is derived locally from canonical proposal and design sections without AI interpretation.

#### Scenario: Recognized change with canonical sections
- **WHEN** a user selects a recognized OpenSpec bundle containing canonical `proposal.md` or `design.md` sections
- **THEN** Overview is the default reader mode and shows the available purpose, changes, goals, decisions, and risk excerpts with their source identity

#### Scenario: Generated Summary is available
- **WHEN** Overview contains a current, stale, generating, or failed Pi Summary state
- **THEN** Backstage presents it after and visually distinct from deterministic OpenSpec excerpts

#### Scenario: Canonical overview sections are missing
- **WHEN** a recognized change lacks all supported proposal and design sections
- **THEN** Overview states that no canonical overview sections were found and offers Source without inventing content

### Requirement: Complete grouped task register
Backstage SHALL present parsed OpenSpec tasks in a dedicated read-only Tasks mode, grouped by source heading and preserving source order, completion state, and source location.

#### Scenario: Mixed task completion
- **WHEN** `tasks.md` contains completed and remaining supported task markers under headings
- **THEN** Tasks shows every completed and remaining task under the corresponding heading with deterministic group and total progress

#### Scenario: All tasks are complete
- **WHEN** every parsed task is complete
- **THEN** Tasks keeps the completed tasks visible and reports that no tasks remain

#### Scenario: Tasks are unavailable
- **WHEN** no supported task markers can be parsed
- **THEN** Tasks explains that structured task facts are unavailable, preserves parser warnings, and keeps `tasks.md` accessible through Source

### Requirement: Exact source and progressive provenance
Backstage SHALL keep every indexed OpenSpec member available as sanitized source Markdown while progressively disclosing custody metadata that is not needed to understand the change.

#### Scenario: Switch to a source member
- **WHEN** a user enters Source and chooses a bundle member
- **THEN** Backstage renders that exact member, keeps Source active after loading, and identifies the selected file

#### Scenario: Inspect source details
- **WHEN** a user expands Source details
- **THEN** Backstage shows project, bundle, path, recognition, Git branch, modified time, parser, task facts, and fingerprint without duplicating them in the Overview body

#### Scenario: Source warning exists
- **WHEN** the live bundle or task parser reports a warning
- **THEN** Backstage keeps the warning visible without requiring Source details to be expanded

### Requirement: Safe viewer fallback
Backstage SHALL limit the structured viewer to recognized OpenSpec bundles and preserve usable reading paths for candidates and incomplete changes.

#### Scenario: Planning candidate selected
- **WHEN** a user selects a planning candidate rather than a recognized OpenSpec change
- **THEN** Backstage uses the existing single-document Markdown reader and does not imply structured OpenSpec recognition

#### Scenario: Specification file present
- **WHEN** a recognized change contains specification documents
- **THEN** those documents remain available in Source without creating a specialized requirements view

#### Scenario: Extracted Markdown contains unsafe content
- **WHEN** an extracted overview section contains raw HTML, scripts, remote media, handlers, or unsafe links
- **THEN** Backstage applies the same sanitization and inert-link policy used by the existing source renderer
