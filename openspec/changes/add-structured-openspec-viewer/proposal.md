## Why

Recognized OpenSpec changes currently open as metadata-heavy individual Markdown files, forcing users to reconstruct the purpose, design, and work state by switching among source documents. OpenSpec’s stable document structure makes it possible to present a deterministic, local change brief that answers what the change is and what work remains before exposing raw source.

## What Changes

- Open recognized OpenSpec bundles in a dedicated viewer whose default view is an extracted Overview rather than an individual source file.
- Extract canonical sections from `proposal.md` and `design.md` locally and present purpose, changes, goals, decisions, and risks without AI interpretation.
- Present `tasks.md` as a separate, read-only work plan grouped by source headings, with completed and remaining tasks both visible in source order.
- Keep every bundle member available in a Source view with exact sanitized Markdown rendering.
- Move parser, recognition, fingerprint, path, and timestamp metadata into progressively disclosed Source details while preserving warnings and read-only handoffs.
- Keep Pi-generated Summary visibly separate from deterministic OpenSpec content.
- Preserve the existing artifact reader for planning candidates and provide graceful fallbacks for incomplete or malformed OpenSpec changes.

Non-goals:

- No repository writes, task toggling, source editing, or task-state inference beyond existing deterministic markers.
- No specialized requirements/specification viewer in this version; spec files remain available as source documents.
- No AI-generated overview, automatic Pi request, new detector, routing system, or repository schema migration.
- No redesign of the Accession Desk shell, project registry, or bundle ledger.

## Capabilities

### New Capabilities
- `structured-openspec-viewer`: Deterministic Overview, Tasks, and Source experiences for recognized OpenSpec change bundles.

### Modified Capabilities

None.

## Impact

The change affects OpenSpec section/task parsing, artifact-detail response data, the React reading desk, frontend and Rust tests, and the existing Accession Desk layout/type styles. It may add a Markdown AST dependency if the current parser cannot safely expose section structure. Repository access remains read-only, cached indexes remain compatible, and Pi invocation behavior remains explicit and unchanged.
