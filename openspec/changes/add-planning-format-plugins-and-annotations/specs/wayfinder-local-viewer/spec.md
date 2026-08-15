## ADDED Requirements

### Requirement: Local-Markdown Wayfinder recognition and grouping
The system SHALL recognize `.scratch/<effort>/map.md` as the root of a local-Markdown Wayfinder effort and SHALL group its contained Markdown sources as one Wayfinder Work Record.

#### Scenario: Canonical local map is discovered
- **WHEN** a safely indexed project contains `.scratch/<effort>/map.md`
- **THEN** the Wayfinder adapter emits one Work Record rooted at `.scratch/<effort>/`
- **AND** recognition identifies the local-Markdown Wayfinder adapter and version

#### Scenario: Effort contains decision tickets and related Markdown
- **WHEN** the effort root contains `issues/*.md` and other safely indexed descendant Markdown files
- **THEN** the Work Record includes those files as contained source members
- **AND** only canonical `issues/<NN>-<slug>.md` members are interpreted as decision tickets

#### Scenario: Similar map filename is outside the convention
- **WHEN** a project contains `map.md` outside `.scratch/<effort>/`
- **THEN** that filename alone does not produce a Wayfinder Work Record
- **AND** the document remains available through plain Markdown

#### Scenario: Effort members exceed scan or read bounds
- **WHEN** a descendant is excluded, unsafe, oversized, outside project containment, or reached only through an escaping link
- **THEN** the adapter does not include or read that source
- **AND** it reports available bounded warnings without weakening containment

### Requirement: Versioned local Wayfinder grammar
The system SHALL interpret local Wayfinder version 1 using exact case-sensitive paths, headings, metadata names, and supported values outside fenced Markdown.

#### Scenario: Canonical ticket filename is interpreted
- **WHEN** an effort contains `issues/<number>-<slug>.md`, where number has two or more ASCII digits with value greater than zero and slug contains lowercase ASCII alphanumeric words separated by single hyphens
- **THEN** the adapter interprets it as a decision ticket
- **AND** normalizes its numeric identity without leading zeroes for blocker resolution and duplicate detection

#### Scenario: Ticket filename is noncanonical
- **WHEN** an issue filename has a one-digit or zero number, uppercase or empty slug, repeated hyphen, unsupported extension, or nested directory
- **THEN** the file remains available in Source but is not interpreted as a decision ticket
- **AND** the adapter reports a recognition warning when it otherwise resembles a ticket

#### Scenario: Canonical map and ticket headings are parsed
- **WHEN** exact level-two headings name `Destination`, `Notes`, `Decisions so far`, `Not yet specified`, `Out of scope`, `Question`, or `Answer` outside fenced blocks in their corresponding source type
- **THEN** the parser recognizes those sections
- **AND** a heading with different case, level, or wording is not treated as canonical

#### Scenario: Canonical ticket metadata is parsed
- **WHEN** exact `Type:`, `Status:`, and `Blocked by:` lines appear before the first level-two heading
- **THEN** outer value whitespace is ignored
- **AND** Type accepts only `research`, `prototype`, `grilling`, or `task`
- **AND** Status accepts only `claimed` or `resolved`
- **AND** Blocked by accepts only a comma-separated list of two-or-more-digit positive ticket numbers

#### Scenario: Optional metadata is absent
- **WHEN** Status and Blocked by are absent
- **THEN** the ticket is open, unclaimed, and has no declared blockers
- **AND** absence does not produce a parser warning

#### Scenario: Canonical field is empty or duplicated
- **WHEN** a supported heading or metadata field is empty where a value is required or occurs more than once
- **THEN** that field is unavailable
- **AND** the parser reports the ambiguity instead of selecting one occurrence or treating empty Status as absent

### Requirement: Structured Wayfinder map overview
The system SHALL derive a local deterministic overview from the map's Destination, Notes, Decisions so far, Not yet specified, and Out of scope sections without AI interpretation.

#### Scenario: Canonical map sections exist
- **WHEN** the selected map contains one or more supported sections
- **THEN** the Overview presents those sections in source order with their source provenance
- **AND** Destination is the primary summary when available

#### Scenario: Canonical sections are incomplete
- **WHEN** the map omits or duplicates supported sections
- **THEN** the Overview presents every unambiguous section it can parse
- **AND** reports missing or ambiguous sections without inventing content

#### Scenario: Map contains unsafe Markdown
- **WHEN** a map section contains raw HTML, scripts, remote media, handlers, or unsafe links
- **THEN** the reader applies the existing sanitized and inert-link policy
- **AND** exact bounded source remains available in Source

### Requirement: Decision-ticket question and answer register
The system SHALL parse canonical local Wayfinder ticket files into a Questions view that preserves ticket order, type, status, blockers, question, answer, source identity, and parser warnings.

#### Scenario: Open unclaimed question is parsed
- **WHEN** an issue file has a supported `Type:` value, no claimed or resolved status, and a `## Question` section
- **THEN** Questions shows it as open and unclaimed with its question and source identity

#### Scenario: Claimed question is parsed
- **WHEN** an issue file contains `Status: claimed`
- **THEN** Questions shows it as open and claimed
- **AND** the ticket is excluded from the frontier

#### Scenario: Resolved question has an answer
- **WHEN** an issue file contains `Status: resolved` and an `## Answer` section
- **THEN** Questions shows the question and answer together as resolved
- **AND** preserves the answer's exact sanitized Markdown

#### Scenario: Ticket metadata is malformed
- **WHEN** a ticket has an unsupported type, status, blocker reference, duplicate canonical section, or missing question
- **THEN** the system keeps the ticket in Questions and Source when safely readable
- **AND** reports precise warnings instead of inferring unsupported values

### Requirement: Deterministic Wayfinder frontier
The system SHALL compute the frontier as open, unclaimed tickets whose declared blockers all resolve to tickets with `Status: resolved` in the same effort.

#### Scenario: Ticket has no blockers
- **WHEN** a ticket is open, unclaimed, and has no `Blocked by:` entries
- **THEN** it appears in the frontier

#### Scenario: Every blocker is resolved
- **WHEN** an open unclaimed ticket names blockers and every referenced ticket is resolved
- **THEN** it appears in the frontier
- **AND** its resolved blocker relationships remain visible

#### Scenario: Blocker remains open
- **WHEN** an open unclaimed ticket names at least one blocker that is not resolved
- **THEN** it does not appear in the frontier
- **AND** the blocking relationship is shown in Questions

#### Scenario: Blocker reference cannot be resolved
- **WHEN** a ticket names a missing, malformed, duplicate, or ambiguous blocker number
- **THEN** the ticket does not appear in the frontier
- **AND** the system reports why frontier eligibility is unavailable

#### Scenario: Frontier has multiple tickets
- **WHEN** more than one ticket is open, unblocked, and unclaimed
- **THEN** the frontier lists all eligible tickets by numeric ticket order
- **AND** identifies the first ticket as the local convention's next candidate without claiming it

### Requirement: Wayfinder structured reader and fallback
The system SHALL present a recognized local Wayfinder Work Record through Overview, Questions, and Source views while preserving the generic fallback and handoff boundaries.

#### Scenario: User opens a recognized effort
- **WHEN** the user selects a local Wayfinder Work Record
- **THEN** Overview is the default view
- **AND** Questions and Source are available when their inputs exist

#### Scenario: User opens a source member
- **WHEN** the user chooses a map, ticket, or related member in Source
- **THEN** the reader renders that exact safely captured Markdown
- **AND** identifies its project-relative path

#### Scenario: User copies a Wayfinder handoff
- **WHEN** the user copies a path or continuation prompt for a Wayfinder Work Record
- **THEN** the handoff identifies the map path and the computed frontier when available
- **AND** it does not claim, resolve, or edit a ticket

#### Scenario: Structured parsing is unavailable
- **WHEN** no supported map sections or tickets can be parsed from a recognized effort
- **THEN** the Work Record remains readable through Source
- **AND** the reader reports that structured Wayfinder facts are unavailable

### Requirement: Local-only Wayfinder boundary
The system SHALL limit this capability to safely indexed local-Markdown Wayfinder efforts and SHALL not infer that remote issue-tracker maps have been discovered.

#### Scenario: Repository only references a remote map
- **WHEN** local Markdown links to a GitHub, GitLab, Linear, Jira, or other remote Wayfinder map without a canonical local map
- **THEN** the system does not create a Wayfinder Work Record from that link
- **AND** performs no background network fetch

#### Scenario: Local Wayfinder detail opens
- **WHEN** the user opens a recognized local effort
- **THEN** parsing and frontier computation stay local and deterministic
- **AND** no Pi request occurs unless the user invokes a separate existing generated-view action
