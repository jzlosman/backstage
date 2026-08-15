## Why

Backstage treats OpenSpec as privileged application behavior, so every additional planning format would spread new branches across discovery, indexing, API contracts, and the reader. Users also cannot privately distinguish approved plans from rejected, obsolete, or superseded work without changing repository files.

## What Changes

- Introduce a compiled-in planning-format adapter registry with plain Markdown as the guaranteed fallback, existing planning-pattern candidates preserved through a low-precedence adapter, and OpenSpec as the first recognized specialized adapter.
- Model specialized records as structured projections over safely indexed sources so adapter failure never hides readable Markdown.
- Replace OpenSpec-specific index and detail seams with neutral Work Record, derived-fact, warning, source, and structured-view contracts while preserving current OpenSpec behavior.
- Add a local-Markdown Wayfinder adapter for `.scratch/<effort>/map.md` and its `issues/` children, including Destination, Decisions so far, Not yet specified, Out of scope, questions, answers, blockers, and a deterministically computed frontier.
- Add private, app-owned Work Record annotations for decision (`Undecided`, `Approved`, `Rejected`), disposition (`Applicable`, `Obsolete`, `Superseded`), favorite, todo, and optional priority.
- Store annotations separately from index snapshots, keep them across rescans, and represent supersession as a typed relationship to another Work Record with last-known target details.
- Make annotation state visible and filterable without conflating it with source-derived facts such as OpenSpec custody, task progress, or Wayfinder ticket state.

Non-goals:

- Loading executable third-party plugins or arbitrary plugin-provided frontend code.
- Writing, moving, editing, or adding frontmatter to repository files.
- Reading annotation state from repository frontmatter.
- Synchronizing annotations between Backstage installations or users.
- Supporting remote GitHub, GitLab, Linear, or Jira Wayfinder maps in this iteration.
- Annotating individual OpenSpec tasks, Wayfinder questions, headings, or source members independently of their containing Work Record.
- Using Pi to recognize formats, compute lifecycle, or assign annotations.

## Capabilities

### New Capabilities
- `planning-format-adapters`: Recognize, group, derive facts from, and render planning formats through a pure compiled-in adapter registry while preserving plain-Markdown fallback.
- `private-work-annotations`: Persist and operate private Work Record decision, disposition, attention, priority, and supersession annotations outside repositories and index snapshots.
- `wayfinder-local-viewer`: Recognize local-Markdown Wayfinder efforts, compute their frontier, and present their map and decision tickets as one structured Work Record.

### Modified Capabilities

None. The project currently has no canonical specifications under `openspec/specs/`; existing behavior is carried forward explicitly by the new capability specifications.

## Impact

- Rust core artifact classification, identities, adapter contracts, OpenSpec parsing, neutral structured views, and deterministic Wayfinder parsing.
- Tauri catalog orchestration, contained source reads, index snapshot compatibility, SQLite migrations, annotation commands, and replacement-target reconciliation.
- TypeScript API models, work-ledger composition and filtering, structured-reader rendering, annotation controls, badges, and missing-target states.
- Existing OpenSpec and generic Markdown behavior must migrate without regressions; current cached indexes require backward-compatible deserialization or safe rebuild.
- No new network dependency, repository write path, background model invocation, or executable plugin-loading boundary is introduced.
