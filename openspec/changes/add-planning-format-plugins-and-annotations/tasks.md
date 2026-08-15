## 1. Neutral Format Foundation

- [x] 1.1 Add failing core tests for stable record locators, subject IDs, adapter descriptors, source claims, neutral facts, capabilities, structured blocks, and deterministic serialization.
- [x] 1.2 Implement the neutral Work Record and structured-view domain types without filesystem, SQLite, clock, Pi, or frontend dependencies.
- [x] 1.3 Add failing registry tests for explicit precedence, recognized-over-possible claims, overlap warnings, unique ledger membership, deterministic ordering, and plain-Markdown fallback after adapter failure.
- [x] 1.4 Implement the pure compiled-in adapter registry, scan-time summarizer contract, and fake-adapter test seam.
- [x] 1.5 Add the plain-Markdown and planning-pattern adapters, preserving current pattern evidence, one-source possible records, generic source reading, and complete source counts.

## 2. OpenSpec Adapter Parity

- [x] 2.1 Convert existing current, done, archived, malformed, and mixed-task OpenSpec fixtures into adapter contract tests that capture recognition, custody, progress, summary, tasks, warnings, source ordering, handoff output, and stable format/version/record-key identity.
- [x] 2.2 Add identity tests proving current and archived copies remain distinct, archival movement does not transfer subjects, and adapter implementation upgrades preserve subjects when format ID and record-key semantics do not change.
- [x] 2.3 Implement `openspec-v1` detection and grouping behind the adapter contract using the existing pure path, progress, and status logic.
- [x] 2.4 Implement bounded scan-time OpenSpec summarization for fingerprints, progress, primary status, counts, and warnings, including incomplete snapshot tests before any record opens.
- [x] 2.5 Implement neutral OpenSpec Overview, Tasks, Source, summary-fact, warning, and handoff output and make the parity tests pass.
- [x] 2.6 Compose registry output into one neutral indexed Work Record collection while preserving complete source counts, unique ledger representation, source modification times, fingerprints, and stale-scan rejection.
- [x] 2.7 Add backward-compatible snapshot deserialization or safe cache-discard-and-rescan behavior for legacy bundle/document snapshots, with migration tests.
- [x] 2.8 Migrate generated views from reachable legacy bundle owners to SubjectId owners and test current summaries, stale summaries, regeneration, provenance, unmappable cache deletion, and route-based root-removal pruning.
- [x] 2.9 Replace OpenSpec-specific Tauri detail fields with neutral capability payloads and add command tests for fresh contained snapshots, partial member failure, and delayed-selection rejection.
- [x] 2.10 Add the compiled frontend capability renderer, migrate OpenSpec and plain-Markdown readers, and remove superseded top-level branches only after Rust, API, generated-view, sanitization, keyboard, and responsive parity tests pass.

## 3. Private Annotation Vertical Slice

- [x] 3.1 Add failing pure-core tests for annotation defaults, independent decision/disposition state, marker updates, valid supersession, self-reference rejection, cycle rejection, and Superseded-to-Obsolete transitions.
- [x] 3.2 Implement annotation commands, states, typed rejections, and bounded supersession-graph validation in the pure core.
- [x] 3.3 Add SQLite migration and storage tests for deterministic subjects, historical subject-root routes, sparse default annotations, atomic updates, restart durability, temporary disappearance, unrelated-root removal, and persistence failure.
- [x] 3.4 Implement `work_record_subjects`, `work_record_subject_roots`, and `work_record_annotations` separately from index snapshots; accepted scans add or refresh routes without deleting a route when source is merely absent.
- [x] 3.5 Add Tauri annotation query and mutation commands that resolve current subjects, invoke the pure transition, commit atomically, return authoritative state, and prove no repository, network, or Pi action occurs.
- [x] 3.6 Add tests proving annotation-like repository frontmatter is ignored and exact locator changes do not transfer annotations.
- [x] 3.7 Overlay annotations onto every neutral Work Record and add API contract tests proving that private annotations remain separate from OpenSpec and other adapter-derived facts.
- [x] 3.8 Add reading-desk controls and accessible ledger badges for Undecided, Approved, Rejected, Applicable, Obsolete, Superseded, favorite, todo, and Low/Medium/High priority.
- [x] 3.9 Render available and unavailable supersession targets with current or last-known details, navigation when reachable, and no silent relation loss.
- [x] 3.10 Add cross-format annotation filters while preserving source-recency default ordering, complete result counts, bounded row mounting, and selection-race protection.
- [x] 3.11 Extend root-removal tests for overlapping routes, unavailable subjects under an unrelated retained root, unreachable subject deletion, private-detail scrubbing, generated-view pruning, and incoming Superseded-to-Obsolete conversion.
- [x] 3.12 Implement transactional route-based annotation reconciliation inside coordinated root removal and verify temporary source absence does not trigger explicit-forget behavior.

## 4. Local Wayfinder Vertical Slice

- [x] 4.1 Add representative local Wayfinder fixtures for the exact versioned filename, heading, metadata, type, status, blocker, numbering, duplicate, empty-field, partial, unsafe-Markdown, and bounded-source grammar.
- [x] 4.2 Add failing core tests for exact `.scratch/<effort>/map.md` recognition, descendant grouping, canonical numbered issue interpretation, noncanonical Source-only files, similar-filename rejection, remote-link-only rejection, and deterministic record identity.
- [x] 4.3 Implement `wayfinder-local-v1` detection, source claims, and recognition warnings without filesystem or network access in the adapter.
- [x] 4.4 Add failing parser tests for exact case-sensitive map/ticket headings, pre-heading metadata, fenced Markdown, questions, answers, supported values, number normalization, duplicate fields, empty fields, and precise partial-parse warnings.
- [x] 4.5 Implement pure map and ticket parsers that retain safely readable partial records and source provenance.
- [x] 4.6 Add failing frontier tests for open, claimed, resolved, blocked, missing-blocker, duplicate-number, multiple-frontier, invalid-eligibility, and numeric-order scenarios.
- [x] 4.7 Implement pure frontier calculation plus bounded scan-time Wayfinder fingerprints, counts, frontier summary facts, and warnings before any record opens.
- [x] 4.8 Implement neutral Wayfinder Overview, Questions, Source, relationships, and empty states.
- [x] 4.9 Register the Wayfinder adapter with explicit precedence and add overlap tests against planning patterns and plain Markdown.
- [x] 4.10 Add Tauri detail and handoff tests proving fresh contained reads, no background network/Pi calls, exact map paths, computed frontier context, remote-link-only non-recognition, and no ticket mutation.
- [x] 4.11 Render Wayfinder Overview, Questions, Source, warnings, blockers, answers, and frontier through the neutral capability renderer with keyboard, responsive, accessibility, and sanitization tests.

## 5. Safety, Migration, and Completion Verification

- [x] 5.1 Add repository-immutability tests covering every annotation command, format scan, Wayfinder detail read, filter, and handoff path.
- [x] 5.2 Add containment and resource-bound tests for escaping links, traversal, oversized/non-UTF-8/unstable members, deep descendants, and partial specialized records.
- [x] 5.3 Add concurrency tests for stale scans, changing source membership, delayed detail responses, annotation writes racing root removal, and failed SQLite transactions.
- [x] 5.4 Add capability-boundary tests proving registry scans, scan summarization, annotation operations, and local Wayfinder parsing perform no network or Pi invocation.
- [x] 5.5 Update PRODUCT.md and `docs/v1-support.md` to describe compiled-in formats, local Wayfinder limits and grammar, private annotations, defaults, supersession behavior, identity limits, generated-view migration, and explicit non-support for frontmatter, sync, runtime plugins, and remote trackers.
- [x] 5.6 Run `pnpm format`, `pnpm lint`, `pnpm test`, `pnpm typecheck`, and `pnpm build`; resolve every failure without weakening existing safety or accessibility assertions.
- [x] 5.7 Run `openspec validate --all --strict` and review the final diff for OpenSpec and generated-view parity, complete source counts, unique ledger representation, local-only behavior, app-owned writes, and removal of obsolete format-specific branches.
