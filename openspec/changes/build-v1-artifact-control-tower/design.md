## Context

Backstage starts in a documentation-only repository with an approved architecture blueprint and product record. The first release must prove one complete loop: approve a local root, discover projects and OpenSpec changes, inspect deterministic progress and Markdown, request and cache a Pi summary, detect when it becomes stale, and copy or open a handoff.

The application is macOS-first but must keep its domain and adapter contracts portable. It serves developers who may have 10–20 projects and 50–200 bundles in one window. Scanned content is untrusted and immutable from Backstage's perspective. The application may write only its own configuration, index, and generated-view cache.

The approved UX direction is a persistent three-pane “Accession Desk”: project rail, bundle ledger, and reading/detail pane. PRODUCT.md and `artifact-control-tower-v1.md` remain product and architecture inputs; the OpenSpec capability specs define acceptance behavior for this change.

## Goals / Non-Goals

**Goals:**

- Deliver a vertical desktop slice with real local discovery, parsing, rendering, generated-view freshness, and handoff behavior.
- Keep domain decisions deterministic, typed, and testable without Tauri or filesystem execution.
- Make repository read-only behavior and approved-root containment enforceable at every file boundary.
- Keep observed facts, deterministic assessments, heuristics, and generated output distinct from storage through presentation.
- Preserve prior usable index and generated-view state through refresh and generation failures.
- Establish extension seams for more artifact detectors, generated-view modes, and external launchers.

**Non-Goals:**

- A generic IDE, file explorer, or editor.
- Repository writes or artifact lifecycle management.
- Background AI or agent orchestration.
- Perfect detection of every planning-file convention.
- Cross-platform packaging parity in v1.
- Superset-specific integration without a verified external contract.

## Decisions

### 1. Organize the Rust side as a pure domain core plus adapter ports

Use a Cargo workspace with a pure core crate and a Tauri application crate. The core owns approved-path values, artifact/project/bundle models, recognition results, progress, generated-view states, commands, transitions, domain errors, and requested effects. It has no dependency on Tauri, platform launch APIs, subprocesses, Git executables, wall-clock access, or app storage.

The Tauri crate owns adapter implementations and effect orchestration. React consumes serialized read models and sends intent-shaped commands instead of reimplementing domain rules.

**Why:** This keeps freshness, state preservation, classification, and error behavior directly testable and prevents UI or framework handlers from becoming the hidden domain model.

**Alternatives considered:**

- One Tauri crate with command handlers containing all logic: less initial structure, but hard to test and likely to mix I/O, decisions, and serialization.
- Frontend-owned state rules: fast for mockups, but duplicates Rust truth and weakens the read-only security boundary.

### 2. Use typed immutable snapshots at filesystem boundaries

Adapters resolve and normalize a requested path, verify containment beneath an approved canonical root, inspect metadata, read bounded bytes, and produce an immutable `SourceSnapshot`. A snapshot includes normalized relative path, content bytes/text, modification metadata, and a digest. Bundle fingerprints hash an ordered manifest of relative paths, contents, and membership.

Containment is checked after resolving symlinks and immediately before every read, not only when a root is approved. Scan adapters never open files for write and tests compare repository manifests before and after operations.

**Why:** A previously safe lexical path can escape through symlinks or change between scan and read. The snapshot gives parsing and Pi generation a stable input and makes source-change races observable.

**Alternatives considered:**

- Modification-time freshness only: cheaper, but misses membership semantics and can report false freshness.
- Hash paths without contents: cannot establish what Pi or a parser actually saw.

### 3. Implement discovery as detector output followed by pure classification

The scan adapter discovers bounded file metadata and Git working-tree boundaries. A detector registry evaluates deterministic rules for OpenSpec directories and a small configured set of plan-like candidate names. Detectors return evidence, not final UI prose. The core groups evidence into projects, bundles, recognized artifacts, or possible artifacts.

OpenSpec parsing begins with the current task-list syntax and preserves unsupported or malformed files as readable bundle members with warnings. Other formats remain candidates until explicit detectors are added.

**Why:** This avoids background AI, makes false positives explainable, and permits detectors to evolve independently.

**Alternatives considered:**

- Search every Markdown file for keywords: high false-positive rate and weak provenance.
- Pi classification during scan: violates the explicit-invocation privacy boundary and makes inventory nondeterministic.

### 4. Use SQLite for app-owned durable state, with an in-memory repository abstraction

Store approved roots, last usable project/artifact index, warnings, generated-view provenance and output, and prompt-version metadata in an app-owned SQLite database. Keep schema migration code inside the application storage adapter. The domain depends on repository interfaces and can use in-memory implementations in tests.

Do not store full artifact source content in the durable index unless required for a generated-view snapshot; prefer metadata, parsed facts, and digests. Generated-view source manifests record paths and digests, while generated text is stored locally.

**Why:** SQLite supports atomic replacement of an index snapshot, searchable state, and future scale without scattering JSON files. App-owned storage preserves repository immutability.

**Alternatives considered:**

- JSON files: simple initially, but awkward for atomic multi-entity updates and cache queries.
- No persistence: loses cross-session value and cannot present prior state during refresh.

### 5. Treat scan and generation as cancellable jobs with stale-result guards

Tauri commands start jobs and return identifiers; typed events or queryable state report progress and completion. Each job captures the relevant input generation/fingerprint. Completion is accepted only against the state generation that requested it. A superseded scan or generation may finish operationally, but cannot replace newer state.

The first slice supports one active scan per approved root and one generation per selected mode/scope key. Explicit user requests may cancel or supersede earlier jobs. No automatic Pi retry occurs.

**Why:** Files and selections can change during asynchronous work. Guarding completion by generation prevents old results from appearing current.

**Alternatives considered:**

- Blocking Tauri commands: simpler but degrades the desktop experience and cancellation.
- Last process to finish wins: creates incorrect freshness and index races.

### 6. Invoke Pi through a constrained adapter and prompt envelope

Before invocation, the application builds a bounded snapshot outside the repository working directory. The Pi adapter receives a supported mode, prompt version, untrusted-source envelope, and snapshot. It launches a configured noninteractive Pi command from an app-owned temporary directory with repository-modifying tools unavailable. If the installed Pi CLI cannot provide this boundary, generated views remain disabled with an actionable configuration error.

The adapter captures model and timing metadata where available and treats output as opaque generated text. Generated text never feeds artifact classification or task progress.

**Why:** Repository Markdown may contain prompt injection, and invoking a fully empowered coding agent inside a project would violate the product's central safety promise.

**Alternatives considered:**

- Run Pi at the project root: convenient, but unsafe and contrary to the approved boundary.
- Call a model API directly: possible later, but does not reuse the user's authorized Pi model access.

### 7. Expose a small intent-based Tauri API

Initial commands cover root approval/removal, refresh, index query, artifact detail query, generation request/cancel, generated-view query, copy-path/copy-prompt, and external-open requests. Payloads use stable IDs and serialized read models rather than arbitrary filesystem paths supplied by the frontend. Backend lookup revalidates paths before effects.

Frontend state uses a query/cache layer for server state and local component state only for presentation. Domain states arrive as discriminated tagged objects with provenance categories.

**Why:** Stable IDs reduce path injection surface and prevent React from becoming authoritative for filesystem state.

**Alternatives considered:**

- Generic “read path” Tauri command: flexible but defeats containment and capability boundaries.
- Mirror all domain transitions in React: creates divergent state machines.

### 8. Build the shell as one accessible desktop workspace

The React shell keeps the project rail mounted, uses a virtualized or windowed ledger when needed, and renders the reading pane independently. Resizable panes persist app-owned preferences. The command palette dispatches only currently valid intents. Focus movement among panes, selection identity, keyboard shortcuts, mouse parity, reduced motion, and textual status labels are part of the shell contract.

First-run, scanning, warning, unavailable, empty, generation, stale, and failure states use the same topology whenever prior data exists. Markdown is sanitized; raw HTML and active external resources are disabled by default.

The Accession Desk direction provides layout and state grammar. Durable visual tokens will be recorded in DESIGN.md after a real implementation is built and reviewed, not before.

**Why:** Stable topology protects orientation for users moving rapidly among many projects, while state honesty preserves trust.

**Alternatives considered:**

- Route-per-project navigation: replaces too much context and weakens the permanent rail model.
- Dashboard cards: inefficient for 50–200 bundles and obscures file lineage.

### 9. Verify behavior at three layers

- Pure-core tests cover approved values, grouping, progress parsing, state transitions, fingerprint freshness, stale generation completion, and previous-state preservation.
- Adapter integration tests use temporary Git repositories and OpenSpec fixtures to cover canonical containment, symlink escapes, repository immutability, Git failures, storage migrations, Pi adapter contracts, and sanitization.
- Frontend tests cover keyboard movement, focus restoration, semantic states, pane behavior, and handoff intent. A packaged smoke test exercises one real root through summary staleness.

**Why:** The highest-risk failures occur at boundaries, while most state complexity can be proven cheaply in a pure core.

## Risks / Trade-offs

- **[Pi CLI cannot guarantee read-only isolated invocation]** → Build and test a capability probe first; disable generated views unless the contract passes. Never weaken the repository boundary as fallback.
- **[Canonicalization has platform and time-of-check/time-of-use edge cases]** → Revalidate directly before each read, disallow escaped symlinks, use read-only handles, and retain race warnings rather than claiming atomic scans.
- **[OpenSpec formats vary across versions and repositories]** → Start with explicit supported syntax fixtures, attach parser version/provenance, and degrade to readable Markdown with warnings.
- **[Large roots make initial scans slow]** → Apply exclusions and bounds, stream partial progress, inspect Git roots before deep content, and retain the prior index during refresh.
- **[SQLite adds schema and migration work]** → Keep the first schema narrow, test migrations, and make the database disposable/rebuildable from repositories plus generated-view cache policy.
- **[Generated text may contain incorrect or unsafe claims]** → Label provenance, sanitize rendering, keep deterministic facts authoritative, and never feed output back into status.
- **[A visually distinctive archival grammar becomes decorative bureaucracy]** → Keep the document and task dominant, use metadata once in a provenance spine, and validate the implemented shell through Impeccable finish review.
- **[macOS-first launcher behavior leaks into the core]** → Define platform launcher ports and keep macOS implementations in adapters.

## Migration Plan

1. Establish the Cargo/Tauri/React workspace, shared formatting/linting, test commands, and app-owned data directory without scanning repositories.
2. Deliver root approval, contained project/OpenSpec discovery, parsing, persistence, and a minimal real-data three-pane shell.
3. Add artifact Markdown detail and provenance, then handoff actions.
4. Prove the safe Pi capability boundary; add one Summary mode, local cache, and fingerprint freshness only if the probe succeeds.
5. Exercise representative team repositories read-only, record unsupported detector/parser cases, and package a macOS test build.
6. Run Impeccable implementation and finish review against the confirmed Accession Desk direction, then document the built visual system.

Rollback is removal of the application and its app-owned data directory. No repository migration or rollback is required because repositories remain untouched. Database migrations must be backward-safe during development or permit explicit deletion/rebuild before v1 release.

## Open Questions

- What exact installed Pi CLI invocation and tool-policy flags satisfy noninteractive, isolated, read-only generation?
- Which OpenSpec versions and task syntaxes exist in representative team repositories?
- Should nested worktrees appear as separate projects in v1, or should common Git directories group them?
- Which deterministic non-OpenSpec candidate filenames belong in the first slice?
- Which React build tooling and query/virtualization libraries best fit the current Tauri versions at implementation time?
- What supported contract, if any, can open Superset at a project path in a later change?
