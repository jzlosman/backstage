## Why

Backstage currently keeps its registry focused by indexing only recognized planning work and planning candidates, which means ordinary repository Markdown cannot be opened from the app. Users need a deliberate way to broaden the same read-only workspace without losing the planning-focused default.

## What Changes

- Add a registry scope control with **Plan files** as the default and **All Markdown** as the opt-in broader view.
- Safely index every in-scope `.md` file discovered beneath an approved project root, while preserving scan budgets, containment, and repository immutability.
- Represent ordinary Markdown as a distinct generic document type rather than mislabeling it as a planning candidate.
- In **All Markdown**, include projects and generic Markdown documents omitted from the planning-focused view; avoid duplicate standalone entries for files already readable through a recognized OpenSpec bundle.
- Open ordinary Markdown in the existing generic Markdown reader while recognized OpenSpec bundles retain the structured `Overview / Tasks / Source` viewer.
- Recompute visible project, work, search, filter, and empty-state results from the selected scope without changing deterministic planning classifications.
- Keep the broader scope local and deterministic; selecting **All Markdown** does not send content to Pi.

Non-goals:
- Indexing non-Markdown documents.
- Editing repository files or adding repository-owned metadata.
- Applying OpenSpec structure to ordinary Markdown.
- Enabling AI generation automatically for newly visible documents.
- Persisting or restoring the broader scope across launches in this iteration.

## Capabilities

### New Capabilities
- `markdown-browsing`: Discover, distinguish, filter, and safely read ordinary Markdown alongside planning artifacts.

### Modified Capabilities

None.

## Impact

- Rust discovery, artifact classification, index models, snapshots, and persisted index serialization.
- Tauri catalog assembly and contained artifact reads.
- React project registry, bundle ledger, scope controls, counts, search/filter behavior, and generic reading desk.
- Catalog, persistence, safety, and frontend interaction tests. No new external dependency is expected.
