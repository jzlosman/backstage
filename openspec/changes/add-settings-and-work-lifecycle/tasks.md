## 1. Establish explicit domain facts and transitions

- [x] 1.1 Add failing pure-core tests for validated planning patterns, removable/default restoration rules, configuration revisions, and pattern-match deduplication.
- [x] 1.2 Add failing pure-core tests for current versus archived OpenSpec custody, valid and malformed archive dates, Active/Done/Archived assessment, and progress-unavailable cases.
- [x] 1.3 Add failing pure-core tests for newest-first stable ordering and Today/Past 7 days/Older/Date unavailable grouping across midnight and daylight-saving boundaries.
- [x] 1.4 Implement the smallest Rust domain types and pure transitions for the seven traceability factors, keeping clock, filesystem, SQLite, regex compilation, and scan execution at adapters.
- [x] 1.5 Update index/API serialization with backward-compatible defaults and prove legacy snapshots load as current-custody records.

## 2. Deliver archived OpenSpec as a complete vertical slice

- [x] 2.1 Add catalog/classification failures for standard archive paths, current/archive name collisions, malformed archive names, supported member bounds, and ordinary Markdown deduplication.
- [x] 2.2 Recognize archived OpenSpec members, preserve full-path identity, strip valid date prefixes only for display, and route archived bundles through the existing Overview/Tasks/Source builder.
- [x] 2.3 Carry custody, archive date, primary status, and separate open/done facts through persisted indexes, Tauri detail responses, handoffs, and frontend API types.
- [x] 2.4 Add frontend failures for Current default filtering, Archived navigation, Active/Done/Archived row and reader labels, open/done counts, progress unavailable, selection fallback, and stale-response suppression.
- [x] 2.5 Implement lifecycle-aware filters and uniform archived reading without allowing Backstage to edit, move, close, or archive repository files.

## 3. Deliver approved-root Settings and coordinated removal

- [x] 3.1 Add storage/coordinator failures for successful removal, unknown roots, in-flight scan cancellation, index cascade, overlapping-root reachability, generated-view pruning, and transaction rollback.
- [x] 3.2 Extend the existing remove-root command into one coordinated adapter flow that returns authoritative retained roots and indexes while preserving repository immutability.
- [x] 3.3 Add frontend failures for opening/closing Settings from titlebar and command palette, focus restoration, root rows, add/cancel/duplicate behavior, confirmation copy, removal fallback, and recoverable failures.
- [x] 3.4 Build the Accession Desk Settings surface with ruled Approved roots and Planning patterns sections, then remove the Approved Roots footer from the project registry.
- [x] 3.5 Verify removal during scanning and delayed detail responses cannot resurrect removed roots or selections, and that retained roots remain operable.

## 4. Deliver configurable planning patterns end to end

- [x] 4.1 Add a SQLite migration and persistence failures proving canonical defaults seed once, deleted defaults stay deleted, an empty set is valid, custom patterns survive restart, and Restore defaults preserves custom rows.
- [x] 4.2 Add the bounded Rust regex dependency and adapter validation for expression bytes, total count, compilation, provenance, and normalized project-relative Markdown matching.
- [x] 4.3 Replace hard-coded candidate basenames with persisted pattern evaluation while preserving one artifact identity for multiple matches and keeping OpenSpec recognition independent.
- [x] 4.4 Add scan-coordination failures proving successful mutations rescan every root with a new configuration revision, older results are discarded, and failed rescans preserve last successful snapshots.
- [x] 4.5 Add Settings interaction failures for add, invalid input, remove, remove-all, broad-pattern explanation, Restore defaults, saving/scanning states, and keyboard operation.
- [x] 4.6 Implement pattern controls and accessible validation/recovery copy without writing configuration into approved repositories or invoking Pi.

## 5. Make recency and status the ledger hierarchy

- [x] 5.1 Add frontend/domain failures for cross-project newest-first ordering, stable equal-time ties, all record kinds, every recency group, omitted empty groups, and Date unavailable last.
- [x] 5.2 Compose filters and search before ordering/grouping, inject the current clock explicitly, and remove the redundant Recently changed filter.
- [x] 5.3 Redesign bundle and document rows so body-sized date/time, Active/Done/Archived status, and `N open · M done` facts form the primary metadata line without color-only meaning.
- [x] 5.4 Add grouped keyboard navigation, bounded batch continuation, nonduplicated group headings, complete counts, and selection preservation across refresh/regrouping.
- [x] 5.5 Verify narrow-window truncation/full accessible labels, 200% zoom, reduced motion, local midnight regrouping, and daylight-saving behavior.

## 6. Harden the integrated change

- [x] 6.1 Add rejection and recovery coverage for invalid patterns, pattern limits, malformed archive paths, unavailable task progress, missing roots, persistence faults, overlapping roots, and partial rescan failures.
- [x] 6.2 Extend safety and vertical smoke tests to prove path containment, symlink rejection, bounded scanning, repository before/after immutability, and no background Pi or network invocation.
- [x] 6.3 Exercise migrations from a legacy database and index snapshot, then verify rollback leaves additive app-owned settings harmless and repositories untouched.
- [x] 6.4 Run one bounded Impeccable visual inspection at representative desktop and narrow widths, fix the observed hierarchy/accessibility defects in one batch, and run the detector once over changed UI targets.
- [x] 6.5 Run a domain-pipeline branch review over the complete committed/staged/unstaged/untracked change surface and record all seven tags as finding, checked, or not applicable; resolve every material finding.
- [x] 6.6 Run focused and full Rust/frontend tests, formatting, lint, typecheck, Clippy, production build, strict OpenSpec validation, and the packaged macOS smoke flow.
