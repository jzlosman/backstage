## Context

Backstage persists approved roots and index snapshots in app-owned SQLite. Root-removal commands already exist through storage, Tauri, and the frontend API, but the React application exposes no removal action and renders every approved path in an accumulating footer inside the project rail.

Planning candidates are currently six hard-coded basenames in the catalog adapter. OpenSpec recognition accepts only current changes under `openspec/changes/<change>/`, so archived changes under `openspec/changes/archive/YYYY-MM-DD-<change>/` lose their bundle identity and structured reader. The ledger exposes task completion and source time as secondary strings, retains catalog ordering, and has no explicit lifecycle model.

This change crosses configuration, scanning, classification, persistence, and the desktop shell. Repository content remains untrusted and immutable. Settings, lifecycle, task progress, observed modification time, generated-view freshness, and relative recency must remain distinct facts or deterministic assessments.

## Goals / Non-Goals

**Goals:**

- Give approved roots and planning conventions a dedicated, keyboard-accessible Settings surface.
- Make root removal complete across persisted, in-flight, cached, and visible app state without touching the filesystem root.
- Let users replace Backstage's seeded planning conventions with bounded global regex rules.
- Preserve OpenSpec identity and the same Overview, Tasks, and Source reader before and after archival.
- Model archival custody independently from task progress and expose a clear Active, Done, or Archived work label.
- Sort every visible work record newest first, group it by useful date ranges, and make source time a primary scan cue.
- Preserve the Accession Desk visual world, three-pane work mode, keyboard flow, responsive behavior, and local-only privacy boundary.

**Non-Goals:**

- Editing or moving repository files, invoking `openspec archive`, or claiming that checked tasks prove archival.
- Per-root or per-project pattern overrides.
- User-authored detectors for non-Markdown formats or OpenSpec schema variants.
- Replacing filesystem modification time with Git history or inferred activity.
- Redesigning the reading desk, generated-summary lifecycle, or project discovery model beyond the lifecycle additions.

## Decisions

### Use an explicit top-level Work or Settings application mode

The titlebar gains a named Settings control and the command palette gains an equivalent action. Settings replaces the pane workspace below the titlebar rather than appearing inside the registry or as a detached generic modal. Closing Settings restores the prior work selection and focus.

The Settings surface uses two broad ruled sections on one paper field:

1. **Approved roots** — path, scan/cached-state summary when available, Add root, and a row-level Remove action.
2. **Planning patterns** — explanatory copy, ordered expressions, provenance label (`Default` or `Custom`), row-level Remove, Add pattern, and Restore defaults.

A removal confirmation states exactly what changes: Backstage forgets the approval, index, and unreachable generated summaries; repository files stay untouched. The project rail no longer contains an Approved Roots footer.

**Alternative considered:** Put remove icons beside roots in the existing footer. Rejected because the footer already competes with project navigation, cannot scale, and gives configuration no safe home for validation and recovery states.

### Persist planning patterns as first-class app configuration

Add an app-owned `planning_patterns` table with stable ID, expression, insertion order, and provenance. A migration seeds three canonical default expressions exactly once for the existing PLAN, TDD, and ROADMAP conventions. Removing every default remains a valid durable configuration; startup never silently re-adds deleted rows. Restore defaults adds missing canonical defaults without deleting custom patterns.

Patterns apply globally and match normalized project-relative Markdown paths. Matching paths rather than basenames supports conventions such as `docs/plans/.*\.md` without granting access outside the already-contained scan. Multiple matching patterns still produce one possible-artifact bundle, with deterministic evidence naming the accepted pattern.

Use Rust's linear-time regex engine. A smart constructor enforces non-empty expressions, a per-pattern byte limit, a total pattern-count limit, successful compilation, and a Markdown-path scope. Universal expressions remain valid because choosing every Markdown file as planning work can be intentional; the Settings copy previews that consequence instead of inventing a hidden prohibition.

A successful add, remove, or restore transaction advances the configuration revision and requests bounded concurrent rescans for every approved root. Invalid input changes neither configuration nor indexes. Existing snapshots remain usable while rescans run; scan admission binds revision and cancellation ownership, and transactional cache writes reject an older revision or generation that finishes late.

**Alternative considered:** Store per-root patterns. Rejected for this iteration because it adds inheritance and override states before the global workflow is proven.

### Remove a root through one coordinated command boundary

Extend the current removal path into one orchestrated operation:

1. Resolve the approved-root identity.
2. Cancel and forget any scan generation for that root.
3. Fence generated-view publication and cancel root-owned Pi generation requests.
4. Transactionally delete the approval and its cascading index snapshot.
5. Merge retained in-memory snapshots with persisted snapshots, then prune generated views whose bundle identities are no longer reachable.
6. Return the new roots/index inventory so the frontend replaces state atomically.

Generated-view persistence and in-memory publication recheck bundle reachability beneath the same publication fence. A delayed Pi result therefore cannot recreate private generated content after its final approved root is removed.

Removing an unknown or already-removed root is an explicit not-found outcome, not a filesystem error. Removing one of two overlapping roots preserves projects and generated views still reachable through the remaining approval. The frontend clears a selected record only when that record is no longer present, then falls back to the first current visible record or the appropriate empty state.

**Alternative considered:** Optimistically splice the root from React state and let startup repair persistence later. Rejected because stale scans, overlapping roots, and cached summaries would leave contradictory app state.

### Represent OpenSpec custody and task progress as orthogonal facts

Extend OpenSpec path classification to recognize both:

- current: `openspec/changes/<change>/...`
- archived: `openspec/changes/archive/YYYY-MM-DD-<change>/...`

The classifier stores an explicit custody value: `Current` or `Archived { archived_on? }`. A valid date prefix becomes observed archive metadata; malformed manual archive names remain archived with an unavailable archive date. Archived bundle identity includes its full archive directory so a current and archived copy can coexist without collision, while display names strip a valid date prefix.

Task progress remains the existing deterministic parser result. A separate pure assessment derives the prominent label:

- **Active:** current custody with one or more open tasks, or unavailable progress.
- **Done:** current custody with available progress and zero open tasks.
- **Archived:** archived custody regardless of task progress.

Rows and the reading desk show the primary label plus separate `N open · M done` facts when available. An archived change with open tasks therefore reads `Archived · 3 open · 8 done`; it is never relabeled Done. Archived bundles use the same Overview, Tasks, and Source builder as current bundles and remain searchable. The default Current filter excludes archived records; Archived reveals them.

**Alternative considered:** Infer Archived from zero remaining tasks. Rejected because completion is content state while archival is repository custody, and either can exist without the other.

### Make recency the ledger's primary ordering and grouping rule

Compose planning bundles and ordinary Markdown records, apply project/scope/status/search filters, then sort the full matching set by observed source modification time descending with stable path identity as the tie-breaker. Group the sorted result using the user's local calendar and one explicit clock input:

- **Today:** same local calendar date as now.
- **Past 7 days:** the seven preceding local calendar dates, excluding Today.
- **Older:** any earlier valid source time.
- **Date unavailable:** missing or invalid time, always last.

Group headings remain visible in the mounted ledger batch. Each row promotes a concise local date/time to body-sized, right-aligned metadata rather than a tiny footer. OpenSpec rows place lifecycle and `open · done` counts on the main metadata line. Source nanoseconds cross persistence and the Tauri/JavaScript bridge as decimal strings, preserving exact ordering beyond JavaScript's safe integer range while legacy numeric snapshots remain readable. The existing Recently changed filter is removed because recency is now the default order and visible structure. Current, Active, Done, Archived, Warning-bearing, and Stale controls retain deterministic filtering; Current is the launch default and includes non-archived documents plus current OpenSpec work.

**Alternative considered:** Preserve alphabetical ordering and strengthen only the date typography. Rejected because visual prominence cannot answer "what changed most recently" when recent records remain scattered.

### Keep settings mutations and lifecycle assessment behind pure-core seams

The target model follows the seven domain-pipeline traceability factors:

- `@edge-input`: filesystem roots, normalized relative paths, observed mtimes, local clock, persisted configuration revision, and user commands enter explicitly at adapters.
- `@domain-wrapper`: approved-root identity and validated planning-pattern expression protect real containment and compilation invariants; ordinary labels and timestamps remain primitive.
- `@rules`: pattern matching, restore-default behavior, root reachability, lifecycle assessment, and date-bucket boundaries are named deterministic rules.
- `@domain-state`: Settings mode, pattern configuration, OpenSpec custody, parsed progress, scan generation, and selected-record availability use explicit states rather than interacting booleans.
- `@domain-error`: invalid pattern, pattern limit, unknown root, and persistence rejection are expected typed outcomes; SQLite and filesystem faults remain operational failures.
- `@domain-effect`: store configuration, cancel scan, delete root state, prune unreachable generated views, and request rescans are explicit requested effects interpreted by adapters.
- `@pure-core-transition`: pattern mutations and root-removal reconciliation transition state and request effects without hidden I/O; OpenSpec status and recency grouping are pure functions of facts plus an explicit clock.

This keeps UI copy and adapters from inventing lifecycle or cleanup policy and gives focused normal, rejection, recovery, concurrency, and timing scenarios executable boundaries.

## Risks / Trade-offs

- [A broad regex promotes many Markdown files] → Preview the rule's scope, keep All Markdown distinct, bound result counts through existing scan budgets, and allow immediate removal.
- [A pattern change races an in-flight scan] → Bind configuration revision, generation, and cancellation ownership at admission; reject stale completion and stale cache writes.
- [Root removal races an in-flight Pi result] → Cancel root-owned requests and fence persistence/publication with a fresh retained-bundle reachability check.
- [Root removal leaves private generated text cached] → Merge current and persisted retained indexes before pruning summaries in the coordinated operation.
- [Overlapping roots share bundle identities] → Reconcile reachability across all retained snapshots before deleting summaries or visible records.
- [Archived directory names are manually malformed] → Preserve Archived custody and the structured reader while marking archive date unavailable.
- [Local date boundaries shift at midnight or daylight-saving transitions] → Group by local calendar dates using an explicit clock, not fixed nanoseconds, and test boundary cases.
- [More prominent metadata crowds narrow ledgers] → Keep one main metadata line, abbreviate dates without losing accessible labels, and let the ledger collapse before the registry at existing breakpoints.
- [Settings grows beyond two sections] → Keep this iteration flat; introduce settings navigation only when another independent category exists.

## Migration Plan

1. Add the app-owned pattern table and a one-time migration that seeds canonical defaults without changing repository state.
2. Keep index snapshot deserialization backward compatible by defaulting new custody and detector fields to current legacy behavior.
3. Ship Settings and root removal before removing the registry footer so every existing action retains a visible home.
4. Rescan approved roots after the migration to classify configured candidates and archived OpenSpec bundles.
5. Preserve previous snapshots if migration or rescan fails; surface recovery in Settings and allow retry.
6. Rollback leaves the additive settings table intact and older binaries ignore it; repository files remain unchanged.

## Open Questions

None. The confirmed product policy is global patterns, current-work default filtering, archived changes retained as OpenSpec, and filesystem modification time as the recency source.
