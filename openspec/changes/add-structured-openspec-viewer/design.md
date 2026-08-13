## Context

Backstage’s current `ArtifactDetail` returns one selected member’s Markdown plus bundle-wide progress and provenance. `live_bundle_state` already reads every member in a recognized OpenSpec bundle, but the UI renders the selected source file first and places a large provenance grid before the document. OpenSpec changes use stable top-level headings in `proposal.md`, `design.md`, and `tasks.md`, so Backstage can derive a more useful local reading model without AI or repository writes.

The feature crosses the pure Rust core, Tauri artifact-detail adapter, React reader, and CSS. Existing cached index payloads must remain compatible, candidate planning files must keep their current reader, and repository content remains untrusted.

## Goals / Non-Goals

**Goals:**

- Make a deterministic Overview the default for recognized OpenSpec bundles.
- Extract only canonical source sections, preserving Markdown and source provenance.
- Show all parsed tasks, grouped by their source headings and in source order.
- Keep exact source documents one action away and retain warnings, handoffs, and optional Pi Summary.
- Reduce initial metadata load through progressive disclosure while strengthening the Accession Desk reading hierarchy.
- Preserve keyboard, narrow-layout, reduced-motion, and read-only behavior.

**Non-Goals:**

- Edit source or task state, infer completion, or synthesize missing sections.
- Add a specialized requirements viewer; specification documents remain ordinary source members.
- Change detectors, index persistence, Pi generation semantics, or repository permissions.
- Replace the established shell or visual world.

## Decisions

### Derive a live structured view in the pure core

Add serializable pure-core models for overview sections and task groups. A deterministic parser will identify canonical level-two Markdown headings outside fenced code, extract the section body as raw Markdown, and associate existing parsed task facts with the nearest task-section heading. It will expose:

- `Why` and `What Changes` from `proposal.md`;
- `Goals / Non-Goals`, `Decisions`, and `Risks / Trade-offs` from `design.md`;
- grouped task facts from `tasks.md`.

The parser will normalize heading case and punctuation only for matching, never rewrite source content. Empty and missing sections are omitted.

Alternative considered: parse headings in React with `marked`. Rejected because structured OpenSpec facts belong in the testable pure core and every client should receive the same deterministic result.

Alternative considered: add a full Markdown AST dependency. Deferred because canonical level-two section extraction and heading-to-line grouping can be implemented with a small fence-aware parser; a dependency is warranted only if real documents exceed that contract.

### Attach the view to live artifact detail, not persisted indexes

Add an optional `openSpecView` field to `ArtifactDetail`, populated from the already-read bundle snapshots only for recognized OpenSpec changes. Keep index models and SQLite payloads unchanged. Planning candidates receive no structured view.

Alternative considered: persist extracted sections in `IndexSnapshot`. Rejected because this creates a migration and stale duplicate content without improving navigation or classification.

### Use three reader modes with Overview as bundle default

Recognized changes use `Overview`, `Tasks`, and `Source` navigation. Selecting a new bundle resets to Overview. Selecting a source member keeps the reader in Source. The Tasks view shows every task—including completed tasks—grouped and ordered exactly as parsed. Requirements/spec files appear in Source only.

Planning candidates retain the current single-document reader. If a recognized change has no canonical overview sections or parsed tasks, the relevant mode explains what is unavailable and points to Source rather than inventing content.

### Turn provenance into progressive disclosure

Keep title, project/branch, deterministic progress, warnings, and handoffs immediately available. Move recognition rule, exact path, modified time, parser, fingerprint, and other custody metadata into a native `details` disclosure labeled `Source details`. Warnings remain visible and are not hidden in the disclosure.

### Extend the Accession Desk rather than add dashboard cards

The reading desk becomes a change dossier: a compact change masthead, sticky ruled mode navigation, a typographically strong purpose statement, a ruled change list, quiet design excerpts, and a grouped work register. Use the existing system sans, semantic tokens, Phosphor icons, 65–75ch prose measure, and current square control grammar. Avoid metric cards, decorative animation, and duplicated metadata.

Pi Summary remains in Overview after deterministic sections, explicitly labeled as generated. Source Markdown remains sanitized through the existing renderer; extracted section Markdown uses the same sanitization boundary.

## Risks / Trade-offs

- **[Risk] Real OpenSpec documents vary in heading spelling or omit canonical files** → Match a small documented alias set case-insensitively, omit missing sections, and keep Source complete.
- **[Risk] Task grouping diverges from task parsing** → Reuse existing `TaskFact` results and group by their source line locations rather than parsing markers twice.
- **[Risk] Adding view data increases artifact-detail payload size** → Return only extracted Markdown fragments and task facts; source content was already read and only the selected document remains returned in full.
- **[Risk] Tab state resets unexpectedly during member loading** → Reset only when `bundleId` changes and explicitly switch to Source before member selection.
- **[Risk] Extracted Markdown could bypass sanitization** → Return raw fragments as untrusted strings and sanitize every rendered fragment with the existing `renderMarkdown` boundary.
- **[Risk] Completed task lists become long** → Preserve all tasks as requested, use compact grouped rows, and rely on the reading-desk scroll rather than hiding facts.

## Migration Plan

1. Add pure-core parser models and failing tests for section extraction, fenced headings, grouped completed/remaining tasks, and missing sections.
2. Extend live artifact detail with an optional structured view and add nested-root integration coverage.
3. Add failing frontend interaction tests for default Overview, all-task visibility, Source switching, candidate fallback, and missing-section recovery.
4. Build the three-mode reader and progressive source details.
5. Apply Accession Desk layout and type treatment, then run one bounded desktop/narrow visual pass and one confirmation pass.
6. Run all frontend/Rust checks, package the macOS app, and verify real OpenSpec selection.

Rollback removes the optional detail field and viewer branch; no persisted data migration or repository cleanup is required.

## Open Questions

None. The user confirmed Overview as the default, completed tasks remaining visible, and no specialized requirements view in this version.
