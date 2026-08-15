## Context

Backstage currently performs one contained project walk and records every Markdown source, but OpenSpec recognition, grouping, progress, lifecycle, detail building, API types, filtering, and React reader branches are embedded across the core, Tauri catalog, index snapshot, and frontend. Planning-path matches form a second hard-coded branch, while ordinary Markdown uses a separate top-level detail path.

The desired extension model has two independent concerns:

1. **Format interpretation:** derive a Work Record and structured view from untrusted, safely observed sources.
2. **Private annotation:** record the user's decision and attention without changing source truth.

Repository contents remain immutable and untrusted. The filesystem, SQLite, clock, and future remote trackers are I/O edges; classification, grouping, parsing, overlap resolution, annotation validation, and frontier calculation belong in the pure Rust core. This iteration reads local Markdown only. It creates an architectural seam for future source adapters without implementing them.

The current Wayfinder contract uses an issue tracker canonically. Its local fallback stores a map at `.scratch/<effort>/map.md` and numbered ticket files under `.scratch/<effort>/issues/`. Ticket status and `Blocked by:` metadata determine the frontier. There is no canonical `frontier.md` in the current local convention.

## Goals / Non-Goals

**Goals:**

- Make plain Markdown the guaranteed fallback for every safely indexed Markdown source.
- Move current OpenSpec and planning-pattern behavior behind a deterministic compiled-in adapter registry without changing visible semantics.
- Give adapters a pure input and neutral output contract for recognition, grouping, summary facts, structured detail, handoff context, warnings, and source provenance.
- Add a local-Markdown Wayfinder adapter that groups one effort and derives its overview, questions, answers, blockers, and frontier.
- Persist private record-level decision, disposition, favorite, todo, priority, and supersession annotations outside index snapshots and repositories.
- Preserve annotations across rescans and restarts when the exact logical record locator remains the same.
- Keep adapter facts, private annotations, heuristics, and Pi-generated text distinct in storage, APIs, and presentation.
- Migrate cached indexes safely and preserve the existing containment, race, accessibility, keyboard, and responsive boundaries.

**Non-Goals:**

- Runtime loading, sandboxing, distribution, or configuration of third-party executable plugins.
- Arbitrary adapter-provided TypeScript, React, HTML, CSS, or scripts.
- Repository frontmatter for annotations or any repository write path.
- Annotation synchronization, export, import, or multi-user semantics.
- Remote issue-tracker discovery or authentication.
- Independent annotations on source members, tasks, questions, sections, or headings.
- Heuristic annotation transfer after a path, project identity, format identity, or adapter record key changes.
- Replacing existing generated-view behavior or using Pi for background classification.

## Decisions

### Separate source observation, format interpretation, annotation overlay, and rendering

The pipeline has four explicit stages:

```text
contained filesystem I/O
        |
        +--> metadata inventory for detection
        +--> bounded scan snapshots for summaries and fingerprints
        +--> fresh selected snapshots for detail
        |
        v
pure FormatRegistry
  OpenSpec -> Wayfinder -> Planning Pattern -> Plain Markdown
        |
        v
Detected Work Records + indexed facts + fresh structured details
        |
        +---- app-owned AnnotationStore overlay
        |
        v
neutral API contracts -> compiled frontend renderer
```

`ProjectSourceInventory` contains normalized project-relative paths and bounded observations needed for detection. It contains no open file handles and grants no adapter direct filesystem access. After detection, the Tauri catalog uses the existing contained reader to capture bounded source snapshots for each detected record. The adapter's pure scan-time summarizer derives fingerprints, progress, status, frontier counts, warnings, and other indexed facts from those immutable snapshots. Selection performs a second fresh contained capture and passes it to the pure detail builder, so indexed summaries are available before opening a record while detail remains current.

A future GitHub or Linear source adapter may produce another normalized inventory, but this change adds no network source type or remote identity contract.

**Alternative considered:** Give each format plugin filesystem and network callbacks. Rejected because plugin behavior would hide I/O, weaken containment, complicate deterministic tests, and couple Wayfinder's local and remote representations prematurely.

### Use a compiled-in registry before designing executable plugins

Define a Rust `PlanningFormatAdapter` contract with immutable descriptor data and pure operations equivalent to:

```text
descriptor() -> { format_id, version, precedence }
detect(project_inventory) -> detected records + claims + warnings
summarize(detected_record, bounded_scan_snapshots) -> fingerprint + indexed facts + warnings
build_detail(detected_record, fresh_source_snapshot) -> structured view + facts + warnings
build_handoff(detected_record, structured_detail) -> bounded handoff context
```

The first registry order is:

1. `openspec-v1`
2. `wayfinder-local-v1`
3. `planning-pattern-v1`
4. `markdown-v1`

OpenSpec and Wayfinder produce recognized records. Planning patterns preserve the current one-source possible-record behavior for sources not claimed by a recognized adapter. Plain Markdown runs last and represents every remaining unclaimed Markdown source one-to-one.

The registry accepts adapters through ordinary Rust construction so tests can supply fakes. It does not define a stable third-party ABI or load dynamic libraries.

**Alternative considered:** Start with WASM, subprocess, or dynamic-library plugins. Rejected because the first need is separation and testability, not untrusted code execution. Runtime plugins would require capability security, schema negotiation, distribution, crash isolation, and UI extension policy before a second built-in format has proven the contract.

### Treat specialized records as claims over an always-retained source inventory

The Markdown inventory remains complete. Specialized adapters propose records and claim source members for ledger composition; they do not delete or replace the underlying source observations. Each adapter record has a format-specific `adapter_record_key` and a sorted member list.

The registry resolves claims by explicit precedence. A source claimed by more than one specialized record is represented by the highest-precedence winner and emits an overlap warning naming every claimant. A recognized claim outranks a planning-pattern candidate. The plain fallback represents only unclaimed sources in the ledger. Source members remain available through the winning record's Source capability.

The All Markdown scope remains a complete, unique representation of the indexed Markdown inventory: each unclaimed source appears as one plain record, while each claimed source appears through exactly one grouped record and remains individually selectable in that record's Source view. Counts use the retained source inventory rather than the number of ledger rows.

This preserves unique ledger representation while making parser failure reversible: a recognized record may lose structured blocks, but never access to safely captured sources.

**Alternative considered:** Let one source appear in every matching record. Rejected because duplicate ledger rows, counts, annotations, and handoffs would make record identity ambiguous.

### Introduce a neutral Work Record envelope and capability view model

Replace the current bundle/document split at the API boundary with a common envelope:

```text
WorkRecord
  subject_id
  locator { project_id, format_id, adapter_record_key }
  display_name
  recognition { recognized | possible | plain, adapter_id, version, evidence }
  source_members
  summary_facts[]
  warnings[]
  capabilities[]
  source_modified_time
  fingerprint
```

`summary_facts` are deterministic, namespaced values with explicit provenance and typed scalar/count/date values. They carry OpenSpec custody, progress, and primary status without placing OpenSpec fields on every record. Wayfinder may expose frontier and resolved/open counts through the same fact contract. Common filtering uses declared fact keys and never fabricates absent values.

The structured detail contract uses a bounded set of neutral blocks and collections: Markdown section, fact register, progress, item collection, relationship list, empty state, warning, and source list. OpenSpec tasks and Wayfinder questions are item collections with different labels and fields. The frontend owns rendering and sanitization for these block kinds. Adapters cannot return executable UI code.

Capabilities have stable IDs and labels, such as Overview, Tasks, Questions, and Source, but their payloads use the neutral block schema. Source remains available for every record.

**Alternative considered:** Return opaque adapter JSON and select a format-specific React component. Rejected because it only moves the format switch from `App.tsx` into another switchboard, weakens API validation, and creates an implicit plugin UI ABI.

### Preserve OpenSpec behavior through an adapter-shaped strangler migration

Move path recognition, grouping, custody, progress, primary status, overview extraction, task grouping, source ordering, and continuation context into `openspec-v1` while keeping current outputs and test fixtures. The stable format identifier is `openspec`; adapter version `1` is recognition provenance and does not participate in subject identity. The adapter record key is the full normalized change directory, so current and archived copies with the same display name remain distinct. Moving a change into the archive therefore creates a new exact locator and does not transfer private annotations automatically; an adapter implementation upgrade that retains the `openspec` format identifier and record-key semantics preserves identity.

The first frontend migration renders the neutral output for OpenSpec before adding Wayfinder. Legacy `ArtifactBundle`, `MarkdownDocument`, `IndexedBundle`, `open_spec_view`, and OpenSpec-specific API fields remain only as temporary migration inputs. New snapshots serialize Work Records. Deserialization either translates a legacy snapshot into the neutral in-memory model or discards it and requests a safe rescan when translation cannot preserve meaning. Repository state makes rescanning recoverable.

Generated views adopt `SubjectId` as their neutral cache owner while retaining mode, source fingerprint, prompt version, included paths, output, time, and model. A one-time app-owned migration maps legacy OpenSpec and planning-candidate `bundle_id` values to neutral subjects from a translated index or accepted rescan. Mapped current and stale summaries remain available. Unmappable legacy cache rows may be discarded because they are derived cache data. Root removal prunes generated views by retained subject reachability after the migration.

**Alternative considered:** Add Wayfinder branches beside the current OpenSpec branches and refactor later. Rejected because it doubles the coupling this change is intended to remove.

### Model local Wayfinder as one map-rooted Work Record

`wayfinder-local-v1` recognizes only `.scratch/<effort>/map.md`. The adapter record key is the normalized effort root. It claims the map and safely indexed descendant Markdown. Canonical numbered files at `issues/<NN>-<slug>.md` become decision tickets; other descendant Markdown remains available in Source but is not interpreted as a ticket.

The accepted local grammar is deliberately exact and versioned:

- The effort root is `.scratch/<effort>/map.md`, where `<effort>` is one non-empty directory segment.
- A ticket filename matches `issues/<number>-<slug>.md`; `<number>` is two or more ASCII digits with numeric value greater than zero, and `<slug>` is lowercase ASCII alphanumeric words separated by single hyphens. Numeric identity ignores leading zeroes, so duplicate numeric identities are ambiguous.
- Map sections are exact, case-sensitive level-two headings outside fenced blocks: `## Destination`, `## Notes`, `## Decisions so far`, `## Not yet specified`, and `## Out of scope`.
- Ticket sections are exact, case-sensitive level-two headings outside fenced blocks: one `## Question` and optional one `## Answer`.
- Metadata fields are exact, case-sensitive lines before the first level-two heading. Outer whitespace around the value is ignored. `Type:` accepts exactly `research`, `prototype`, `grilling`, or `task`. `Status:` accepts exactly `claimed` or `resolved`; absence means open and unclaimed, while an empty or unsupported value is invalid. `Blocked by:` accepts a comma-separated list of ticket numbers using the same two-or-more-digit grammar; absence means no blockers, while an empty value is invalid.
- A duplicate canonical heading or metadata field makes that field unavailable and emits a warning rather than choosing one occurrence.

The map parser extracts canonical sections while respecting fenced Markdown. The ticket parser retains partially parsed tickets with precise warnings.

Frontier calculation is pure:

```text
open(ticket) && unclaimed(ticket) &&
all declared blockers resolve uniquely to resolved tickets in the same effort
```

No status means open and unclaimed. `claimed` means open and claimed. `resolved` means closed. Unknown status, malformed blocker, missing blocker, or duplicate ticket number makes eligibility unavailable and excludes the affected ticket from the frontier. The frontier includes every eligible ticket in numeric order and identifies the first as the convention's next candidate without changing source.

The reader uses Overview, Questions, and Source capabilities. Handoffs include the exact map path and computed frontier when available, but never claim or resolve tickets.

**Alternative considered:** Recognize every folder containing any `map.md`. Rejected because that creates false positives outside the documented local tracker convention.

### Store annotations against deterministic app-owned Work Record subjects

Define a `RecordLocator` from project identity, format identifier, and adapter record key. Derive `SubjectId` deterministically from the canonical locator using the existing stable-hash approach. The locator is exact, not heuristic: a rename or format change creates a new subject.

Add app-owned SQLite tables equivalent to:

```text
work_record_subjects(
  subject_id primary key,
  project_id,
  format_id,
  adapter_record_key,
  last_known_name,
  last_seen_at
)

work_record_subject_roots(
  subject_id references work_record_subjects,
  root_id references approved_roots,
  primary key(subject_id, root_id)
)

work_record_annotations(
  subject_id primary key references work_record_subjects,
  decision,             -- undecided | approved | rejected
  disposition,          -- applicable | obsolete | superseded
  favorite,
  todo,
  priority,             -- null | low | medium | high
  superseded_by_subject_id nullable references work_record_subjects,
  updated_at
)
```

The catalog upserts last-seen subject metadata and adds the observing approved-root route after accepted scans. A later scan never removes a historical route merely because a source is absent; only explicit root removal deletes that root's routes. Annotation rows are separate from replaceable index snapshots, so a rescan cannot erase them. Missing rows produce defaults in memory rather than eager database rows.

A temporarily absent record leaves its subject, root route, and annotations intact while its root remains approved. Removing an unrelated root cannot prune it. If the exact locator reappears, the annotations return. Backstage does not guess that a newly named or moved record is the same subject.

**Alternative considered:** Put annotation fields in index snapshot JSON. Rejected because accepted scans replace snapshots and would either erase user state or turn a cache into the authority for mutable private data.

### Validate annotation transitions in the pure core and commit atomically

Model annotation state as independent decision, disposition, and marker values:

- decision: `Undecided | Approved | Rejected`
- disposition: `Applicable | Obsolete | Superseded(SubjectId)`
- favorite: Boolean
- todo: Boolean
- priority: `None | Low | Medium | High`

The domain transition accepts current annotation state, the reachable subject graph, and one user command. It returns the next annotation and a persistence effect or a typed rejection. Superseded requires a distinct target. Graph traversal rejects direct and transitive cycles. Obsolete carries no target. Changing Superseded to Obsolete removes the relation. Decision and disposition remain orthogonal, so an Approved record may later become Obsolete while retaining the historical decision.

The Tauri command resolves current subjects, runs the pure transition, and writes the source annotation and relation in one transaction. Persistence failure leaves both stored and visible state unchanged. React replaces annotation state from the command response rather than assuming an optimistic write succeeded.

**Alternative considered:** Encode all states in independent booleans. Rejected because `approved && rejected`, `superseded` without a target, and cyclic relationships would become representable.

### Reconcile annotations with explicit root removal

Extend the existing root-removal transaction to delete the removed root's rows from `work_record_subject_roots`. Delete subject metadata, annotations, and generated views only for subjects that have no remaining approved-root route.

Accepted scans add or refresh routes but never remove them because ordinary source disappearance under an approved root is temporary unavailability, not an explicit forget command. Removing an unrelated root therefore cannot erase an unavailable subject associated with a retained root. Explicit removal of a subject's last route erases its private metadata to uphold the domain rule that removing approval removes Backstage's knowledge of that root.

When a deleted subject is the target of a retained supersession relationship, convert the retained source record from Superseded to Obsolete and clear target details in the same transaction. Overlapping roots preserve subjects still carrying another retained route.

**Alternative considered:** Keep orphan annotation history after root removal. Rejected because private metadata would preserve project names and paths after the user explicitly asked Backstage to forget that root.

### Overlay annotations after deterministic source facts

The catalog composes annotations onto Work Records after accepted format classification. The API keeps `derivedFacts` and `annotations` as separate fields. The UI displays separate labels and exposes annotation controls from the selected Work Record, not from adapter blocks.

Annotation filters operate across every format. Default ledger ordering remains observed source recency; priority is visible and filterable but does not silently replace recency. OpenSpec Active/Done/Archived and Wayfinder ticket facts remain derived source facts. Approved/Rejected/Obsolete/Superseded, favorite, todo, and priority remain private user state.

**Alternative considered:** Fold Approved, Rejected, or Obsolete into the existing Work Status enum. Rejected because Work Status currently derives from OpenSpec custody and progress, while annotations represent private user intent and apply to every format.

## Risks / Trade-offs

- [The neutral view schema is too generic for a future format] → Keep the initial block vocabulary small and versioned; add a reviewed block kind when a second real format proves a need rather than allowing opaque executable UI.
- [OpenSpec behavior regresses during extraction] → Move existing fixtures and assertions to adapter contract tests first, then compare neutral API output and frontend behavior before deleting legacy branches.
- [An adapter overlap hides a useful record] → Keep complete source inventory, show deterministic overlap warnings, and make precedence explicit in one registry definition.
- [Wayfinder conventions change] → Record adapter ID and version, parse unsupported values as warnings, and leave all source readable. A future convention becomes a new adapter version.
- [Grouping every descendant Markdown captures a later `spec.md`] → Treat non-ticket descendants as Source-only members; do not infer ticket or map semantics from them.
- [Exact locator identity loses annotations after rename or OpenSpec archival] → Preserve the unavailable old subject and never transfer private state heuristically. Adapter version upgrades preserve identity when format ID and record-key semantics remain stable; explicit relinking can be designed later if real usage warrants it.
- [Historical root routes retain subject metadata after files disappear] → Retain unavailable subjects for correctness while their root remains approved; explicit root removal prunes the route and then private knowledge when no route remains. Add another cleanup policy only with user-visible recovery semantics.
- [Supersession graph validation becomes expensive] → The expected graph is tens or hundreds of records; load only the relation adjacency needed for a bounded traversal and enforce a defensive node limit.
- [SQLite migration or annotation write fails] → Keep records readable, expose annotations as temporarily unavailable, and avoid partial or optimistic state changes.
- [Root removal races scans or annotation writes] → Reuse the root publication fence and perform retained-subject reconciliation transactionally with root removal.
- [New annotation badges crowd the ledger] → Show a compact primary marker set in rows and full controls in the reading desk; preserve accessible text and existing narrow-layout collapse rules.

## Migration Plan

1. Add pure neutral Work Record, format descriptor, claim, fact, capability, structured-block, and adapter-registry types with contract tests.
2. Wrap existing OpenSpec and planning-pattern behavior as adapters, including scan-time summarization from bounded snapshots, and compare their outputs against current fixtures before changing persistence or UI.
3. Add the plain-Markdown fallback adapter and migrate catalog composition to one neutral Work Record collection while preserving unique All Markdown counts and complete source counts.
4. Introduce backward-compatible neutral API types and render existing OpenSpec and plain Markdown behavior through the compiled capability renderer.
5. Add subject, subject-root route, and annotation tables. Backfill subjects and routes from the latest accepted indexes without creating non-default annotation rows.
6. Add `SubjectId` ownership to generated views and translate reachable legacy bundle cache rows without losing current or stale summary metadata.
7. Add atomic annotation commands, validation, overlay, filters, controls, and route-based root-removal reconciliation.
8. Add Wayfinder map, ticket, and frontier parsers; register `wayfinder-local-v1`; then add scan summaries, Overview, Questions, Source, and handoff presentation.
9. Remove legacy OpenSpec-specific top-level index, API, frontend, and generated-cache branches after parity tests pass.
10. Rescan approved roots to write neutral snapshots. If a legacy snapshot cannot translate safely, discard only unmappable app-owned cache data and rebuild indexes from repository sources.

Rollback may ignore or leave the additive subject and annotation tables in app-owned SQLite. Older binaries do not read them. Repository files remain unchanged throughout migration and rollback.

## Open Questions

None for the proposed slice. Annotations are confirmed local and private, frontmatter is excluded, Approved and Rejected are explicit, adapters are compiled in, and Wayfinder support is limited to the documented local-Markdown convention.
