## ADDED Requirements

### Requirement: Ordinary Markdown exposes a copy-path handoff
The system SHALL provide a `Copy path` action for the selected ordinary indexed Markdown document.

#### Scenario: User copies an ordinary Markdown path
- **WHEN** the user selects an ordinary Markdown document and activates `Copy path`
- **THEN** the system writes that document's canonical absolute path to the clipboard
- **AND** shows concise success feedback

#### Scenario: Copy action is accessible
- **WHEN** the Markdown reading desk is used with a keyboard or coarse pointer
- **THEN** `Copy path` remains visibly labeled, focusable, and large enough to activate

### Requirement: Markdown path handoff revalidates repository containment
The system MUST resolve the selected document from its stable document ID, the current index, and the approved-root containment boundary immediately before returning a path to the clipboard adapter.

#### Scenario: Indexed document remains safely contained
- **WHEN** the selected document still resolves to a regular file inside its approved root
- **THEN** the backend returns its canonical absolute path

#### Scenario: Document is no longer available
- **WHEN** the document was removed, replaced by a non-regular source, or no longer resolves safely after selection
- **THEN** the backend rejects the copy request
- **AND** the frontend shows the failure without replacing clipboard contents

#### Scenario: Safely contained content changes
- **WHEN** the regular Markdown content changes at the same indexed project-relative path
- **THEN** the backend still returns that path because document identity is path-based
- **AND** the returned path is canonicalized through the approved-root reader

#### Scenario: Escaping symlink is rejected
- **WHEN** an indexed Markdown path is replaced by a symlink that resolves outside the approved root
- **THEN** the backend rejects the path handoff
- **AND** no outside path or content is copied

### Requirement: Markdown path copy remains local and read-only
The system MUST NOT modify repository contents or trigger Pi generation when copying an ordinary Markdown path.

#### Scenario: Copy completes without repository mutation
- **WHEN** the user copies a Markdown path
- **THEN** scanned repository contents remain byte-for-byte unchanged
- **AND** no Pi request is started
