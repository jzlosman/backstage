## 1. Deterministic change brief

- [x] 1.1 Add failing pure-core tests for canonical proposal/design section extraction, fenced headings, aliases, and missing sections
- [x] 1.2 Add serializable OpenSpec overview models and implement fence-aware canonical section extraction
- [x] 1.3 Add failing pure-core tests for source-ordered task grouping with completed tasks, ungrouped tasks, and unavailable progress
- [x] 1.4 Implement task grouping by existing task-fact source locations without reparsing completion markers

## 2. Live artifact-detail integration

- [x] 2.1 Add failing integration tests proving recognized nested-root changes receive a structured view while planning candidates do not
- [x] 2.2 Build the structured view from already-read live snapshots and add optional `openSpecView` data to artifact detail without changing persisted indexes
- [x] 2.3 Verify malformed or incomplete OpenSpec changes remain browsable through source with deterministic warnings

## 3. OpenSpec reader experience

- [x] 3.1 Add failing frontend tests for Overview default, canonical excerpts, generated-content separation, and missing-overview recovery
- [x] 3.2 Implement recognized-change Overview, Tasks, and Source navigation with bundle-change reset and Source-preserving member selection
- [x] 3.3 Add failing frontend tests for grouped completed and remaining tasks, task-unavailable recovery, exact source selection, and candidate fallback
- [x] 3.4 Implement the complete grouped task register, exact source reader, candidate reader fallback, visible warnings, handoffs, and progressive Source details
- [x] 3.5 Extend preview fixtures and apply Accession Desk layout, typography, responsive, focus, coarse-pointer, and reduced-motion treatment

## 4. Verification and delivery

- [x] 4.1 Run frontend tests, typecheck, lint, formatting, production build, Rust workspace tests, formatting, Clippy, and the pure-core boundary check
- [x] 4.2 Run the Impeccable detector once and perform one batched browser pass at desktop, 960 px, 680 px, and 320 px across Overview, Tasks, Source, long content, keyboard, and missing-content states
- [x] 4.3 Inspect the final diff, compare bundle size, and obtain a bounded code review against the OpenSpec requirements
- [x] 4.4 Build and smoke-test the packaged macOS app, relaunch it, select a real recognized change, and verify repository immutability and path containment
