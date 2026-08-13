## Why

Developers running many coding agents across many repositories lose track of plans, OpenSpec changes, and evidence after their originating sessions close or go stale. Backstage needs one local, read-only desktop surface that finds this durable work, reports what is actually known about it, and hands it back to an agent without modifying project repositories.

## What Changes

- Create a macOS-first Tauri desktop application with a Rust core and React/TypeScript interface.
- Let users approve local scan roots and discover Git working trees beneath them without following paths outside the approved boundary.
- Recognize OpenSpec change bundles and common planning-artifact candidates, parse deterministic OpenSpec task progress, and preserve readable Markdown when recognition or parsing is incomplete.
- Present 10–20 projects and 50–200 artifact bundles in a persistent three-pane, keyboard-first desktop shell.
- Render selected Markdown with provenance, Git context, warnings, progress, and freshness state.
- Generate summaries and continuation prompts through Pi only after explicit user action; cache results locally and mark them stale when their source fingerprint changes.
- Provide read-only handoffs for copying paths or continuation prompts and opening a terminal or configured external target.
- Keep observed facts, heuristics, and AI-generated output visibly and structurally distinct.

### Non-goals

- Editing, moving, archiving, deleting, or marking repository artifacts abandoned.
- Full source-code browsing or editing.
- Automatic AI discovery, classification, summarization, or work continuation.
- Pi session inventory or restoration.
- Guaranteed Superset deep linking before a supported integration contract is confirmed.
- Broad detector coverage beyond OpenSpec and a small deterministic candidate set in the initial vertical slice.

## Capabilities

### New Capabilities

- `approved-root-discovery`: Approve local roots, discover project boundaries and artifact candidates safely, and retain usable results through operational failures.
- `artifact-understanding`: Recognize artifact bundles, parse deterministic OpenSpec progress, expose provenance and warnings, and render source Markdown.
- `generated-views`: Invoke Pi on demand against bounded snapshots, cache generated views with provenance, and detect stale results.
- `desktop-workspace`: Navigate projects and artifacts in a persistent keyboard-first shell and perform read-only handoff actions.

### Modified Capabilities

None.

## Impact

- Introduces a Rust workspace, Tauri application shell, and React/TypeScript frontend.
- Adds local filesystem, Git inspection, Markdown parsing/rendering, app-owned persistence, source fingerprinting, Pi subprocess, clipboard, and external-launch adapters.
- Establishes typed Tauri commands/events between the Rust application boundary and frontend.
- Requires representative local filesystem and OpenSpec fixtures, pure-core tests, adapter integration tests, and UI interaction/accessibility checks.
- Reads approved repositories but never writes inside them; only application configuration and cache storage are mutable.
