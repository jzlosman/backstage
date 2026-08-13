---
spec: architecture-spec v3
status: approved
immutability: immutable
approval: user-confirmed
title: Read-Only Artifact Control Tower
id: DPC-001
version: 1
created: 2026-08-13
updated: 2026-08-13
language: F#-first, Rust/Tauri with a web frontend
---

# Domain Pipeline Blueprint

## 1. Scope and impact map

| ID | Item | Status | Evidence or rationale |
|---|---|---|---|
| **ACT-001** | A developer working across roughly ten repositories and up to twenty concurrent Pi sessions | confirmed | The user described losing track of plans and OpenSpec changes after sessions become stale or close. |
| **SCOPE-001** | The boundary is a native desktop artifact control tower over user-approved parent directories. | confirmed | It addresses cross-project discovery without becoming a source-code IDE. |
| **SCOPE-002** | The selected v1 slice is read-only discovery, structured OpenSpec progress, Markdown viewing, cached on-demand Pi explanations, freshness detection, and handoff actions. | confirmed | The user explicitly accepted this slice. |
| **SCOPE-003** | Repository mutation, artifact lifecycle mutation, autonomous work continuation, full source browsing, and Pi session management are non-goals. | confirmed | The user selected a read-only first version. |
| **SCOPE-004** | Guaranteed Superset deep linking is outside v1 until its supported URI or CLI contract is confirmed. | deferred | Deep linking is desirable, but feasibility is not established. |
| **SCOPE-005** | Touched surfaces are approved-root configuration, filesystem and Git inspection, artifact parsers, an app-owned index/cache, Markdown rendering, Pi invocation, clipboard, and external launching. | recommended | These are the smallest surfaces that satisfy the selected slice. |
| **REQ-001** | Discover project roots and planning artifacts beneath approved roots. | confirmed | This is the primary inventory requirement. |
| **REQ-002** | Recognize OpenSpec bundles structurally and identify other deterministic or possible planning artifacts. | confirmed | The user named OpenSpec, plans, TDDs, Wayfinder, evidence, artifacts, and related Markdown. |
| **REQ-003** | Show observed OpenSpec completion and remaining tasks without AI interpretation. | confirmed | Progress hidden inside Markdown is a central pain point. |
| **REQ-004** | Render selected artifacts as Markdown without showing the whole source tree. | confirmed | The requested experience is artifact-focused rather than code-focused. |
| **REQ-005** | Provide copy-path, continuation-prompt, terminal, and configurable external-open handoffs. | confirmed | The user needs to return artifacts to working agents. |
| **REQ-006** | Generate summaries, ELI5 explanations, folder overviews, and remaining-work explanations with Pi only on explicit request. | confirmed | The user chose on-demand Pi invocation. |
| **REQ-007** | Cache generated views and mark them stale when their source inputs change. | confirmed | The user explicitly requested a stale indicator and regeneration action. |
| **FLOW-001** | Approved-root scan: discover projects, find candidates, classify, parse, and index them. | recommended | This is the deterministic inventory flow. |
| **FLOW-002** | Artifact inspection: select a project or bundle, show structured facts and warnings, then render Markdown. | confirmed | This is the primary user interaction. |
| **FLOW-003** | Explanation: snapshot selected sources, invoke Pi outside the repository, cache the result, and compare future source fingerprints. | recommended | It preserves repository read-only behavior and reliable freshness. |
| **FLOW-004** | Handoff: derive a path, continuation prompt, or external-open request from the selected project and bundle. | confirmed | This restores continuity with another agent or terminal. |

Confirmed facts are the user-approved product boundary and requirements. Recommendations describe the smallest architecture that satisfies them; assumptions and unresolved forks remain visible in section 12.

## Decision ledger

| ID | Decision / question | Status | Evidence or rationale |
|---|---|---|---|
| **DEC-001** | Build a cross-repository planning-artifact control tower. | confirmed | Primary problem statement |
| **DEC-002** | Separate artifact inventory from agent and session orchestration. | recommended | This limits coupling and first-version risk. |
| **DEC-003** | Scan only user-approved roots. | confirmed | User-selected security and scope boundary |
| **DEC-004** | Prefer Git working trees as project boundaries. | recommended | Git is the strongest broadly available project signal. |
| **DEC-005** | Keep all repository interactions read-only. | confirmed | Explicit user decision |
| **DEC-006** | Permit writes only to app-owned configuration and cache storage. | recommended | Indexing and generated views require local persistence. |
| **DEC-007** | Build a native desktop application. | confirmed | Explicit user decision |
| **DEC-008** | Invoke Pi only on explicit user request. | confirmed | Explicit user decision |
| **DEC-009** | Cache generated views and detect source changes. | confirmed | Explicit user requirement |
| **DEC-010** | Decide freshness with source fingerprints and explain staleness with dates and changed paths. | recommended | Fingerprints avoid timestamp-only false freshness. |
| **DEC-011** | Adopt the selected v1 scope in `SCOPE-002`. | confirmed | Explicit user acceptance |
| **DEC-012** | Keep observed facts, deterministic assessments, and AI output visibly distinct. | recommended | This prevents guesses from appearing as repository truth. |
| **DEC-013** | Defer guaranteed Superset integration. | deferred | No supported integration contract has been confirmed. |
| **DEC-014** | Approve this version 1 blueprint as an immutable snapshot. | confirmed | User-confirmed approval on 2026-08-13 |

## 2. Boundaries and I/O

`@edge-input`

| ID | Boundary input or output | Classification | Constraint |
|---|---|---|---|
| **IO-001** | Approved-root configuration | Input | Only explicit roots enter discovery. |
| **IO-002** | Directory traversal and artifact file reads | True I/O | Reads must remain beneath an approved root and must never mutate repository content. |
| **IO-003** | Git working-tree and metadata inspection | True I/O | Git failure reduces context but does not hide readable artifacts. |
| **IO-004** | Clock | True I/O | Used for display age, generation timestamps, and human-readable stale explanations; fingerprints decide freshness. |
| **IO-005** | App-owned index, configuration, and generated-view cache | True I/O | Writes occur outside scanned repositories. |
| **IO-006** | Pi subprocess invocation and response | True I/O | Pi receives a bounded source snapshot outside the repository and no repository-write authority. |
| **IO-007** | Clipboard, terminal launch, and configured external URI launch | True I/O | Launch requests require explicit user action. |
| **IO-008** | Manual refresh or filesystem change notification | Input | Either may request deterministic re-indexing; filesystem watching is not required by the first migration slice. |
| **IO-009** | Pure/effect seam | Boundary | The core decides classification, state, freshness, errors, and requested effects; adapters perform filesystem, Git, clock, storage, Pi, clipboard, and launch operations. |

Business data consists of approved roots, discovered project and artifact identities, parsed task facts, selected scopes, source manifests, and generated-view metadata. Scan limits, timeouts, cancellation, model selection, and launch configuration are controls rather than artifact data.

## 3. Domain wrappers and invariants

`@domain-wrapper`

| ID | Wrapper | Invariant and rejected cases |
|---|---|---|
| **WRAP-001** | `ApprovedRoot` | An absolute, normalized directory explicitly approved by the user; rejects relative paths, files, and unapproved directories. |
| **WRAP-002** | `ArtifactPath` | A normalized file path contained by an `ApprovedRoot`; rejects traversal and symlink resolution outside approved roots. |
| **WRAP-003** | `SourceFingerprint` | A deterministic digest of included paths, content, and bundle membership; rejects incomplete manifests rather than claiming freshness. |

`@rules`

- **RULE-001** Repository files are never created, changed, moved, archived, or deleted.
- **RULE-002** Only `ArtifactPath` values may be read or included in Pi snapshots.
- **RULE-003** Observed facts, deterministic assessments, and generated text have distinct labels and provenance.
- **RULE-004** OpenSpec progress comes from parsed task markers; AI output cannot alter completion.
- **RULE-005** A generated view is current only when its recorded `SourceFingerprint` equals the current fingerprint for the same scope.
- **RULE-006** A failed regeneration preserves any prior generated view and reports the failure separately.
- **RULE-007** Parser failure preserves access to the source Markdown and yields a warning rather than hiding the artifact.
- **RULE-008** “Possibly stale” is a heuristic and never means “abandoned.”
- **RULE-009** Pi runs only after an explicit user command and receives the bounded snapshot selected for that command.

Display labels, task counts, timestamps, model names, and prompt versions remain primitives because this slice assigns them no independent validation rule that warrants wrapper ceremony.

## 4. Control inputs

| ID | Control | Behavior |
|---|---|---|
| **CTRL-001** | Scan policy | Exclusions, maximum depth, maximum file size, and candidate filename patterns bound discovery work. |
| **CTRL-002** | Symlink policy | The default prevents traversal outside approved roots; escaped targets require separate approval. |
| **CTRL-003** | Cancellation and timeout | Scans, Git inspection, and Pi invocation can stop without discarding the last usable index or generated view. |
| **CTRL-004** | Explanation scope limit | Caps files and bytes included in a Pi snapshot and returns an explicit rejection when the selected scope cannot fit. |
| **CTRL-005** | Pi configuration | Mode, model, timeout, and prompt version are recorded with generated output; v1 performs no automatic retries. |
| **CTRL-006** | Clock value | Produces ages and timestamps but cannot override fingerprint-based freshness. |
| **CTRL-007** | Refresh mode | Manual refresh is sufficient for the first migration slice; change notifications may request the same transition later. |
| **CTRL-008** | External-launch configuration | Maps supported handoff types to terminal commands or URI schemes without granting artifact mutation rights. |

Randomness, authorization context, business idempotency keys, and correlation IDs do not affect v1 decisions. A generated-view cache key may deduplicate identical source fingerprint, mode, and prompt-version requests, but this is an adapter optimization rather than business state.

## 5. State tree

`@domain-state`

- **STATE-001** `ProjectIndex` is `NotScanned`, `Scanning(previous?)`, `Ready`, `ReadyWithWarnings`, or `Unavailable(previous?)`. Refresh preserves the prior usable snapshot until replacement succeeds.
- **STATE-002** `ArtifactRecognition` is `Recognized(Parsed)`, `Recognized(ParsedWithWarnings)`, or `PossibleArtifact(reason)`. Parser failure cannot produce a hidden state.
- **STATE-003** `GeneratedView` is `NeverGenerated`, `Generating(previous?)`, `Current(result)`, `Stale(result, changedInputs)`, or `Failed(previous?, failure)`. A previous result remains readable during generation and after failure.
- **STATE-004** `OpenSpecProgress` is an observed structure containing total, completed, and remaining task facts plus parse warnings. It is `Unavailable` when no supported task structure can be parsed; the system does not invent a percentage.

Legal transitions include:

- `NotScanned → Scanning → Ready | ReadyWithWarnings | Unavailable`.
- `Ready | ReadyWithWarnings | Unavailable(previous) → Scanning(previous)` on refresh.
- `NeverGenerated | Current | Stale | Failed → Generating(previous?)` on explicit generation.
- `Generating → Current` only when the generated fingerprint still matches current sources.
- `Generating → Stale` when sources changed during generation.
- `Generating → Failed(previous?, failure)` on adapter failure.
- `Current → Stale` when a later scan produces a different source fingerprint.

## 6. Errors as data

`@domain-error`

| ID | Expected rejection | Meaning and recovery |
|---|---|---|
| **ERR-001** | `OutsideApprovedRoot` | A requested path is not beneath an approved root. Reject before reading; the user may approve another root explicitly. |
| **ERR-002** | `ArtifactUnavailable` | A selected artifact disappeared or became unreadable before snapshotting. Preserve the index entry as unavailable and allow refresh. |
| **ERR-003** | `ExplanationScopeTooLarge` | The selected scope exceeds `CTRL-004`. Ask the user to narrow the scope or change the explicit limit. |
| **ERR-004** | `OperationalWarning` | Permission failures, Git unavailability, parser failures, files changing during reads, cache failures, Pi timeouts, malformed Pi responses, and external-launch failures remain adapter warnings/failures. They are not promoted into domain-error variants unless later product behavior requires distinct decisions. |

`ERR-004` may move `STATE-001`, `STATE-002`, or `STATE-003` into a warning/failure state while preserving prior usable data. It must not imply artifact incompleteness, abandonment, or task failure.

## 7. Effects

`@domain-effect`

| ID | Requested effect | Causal constraint |
|---|---|---|
| **EFFECT-001** | `ScanApprovedRoot` | Operates only on `WRAP-001` and emits discovered paths or operational warnings. |
| **EFFECT-002** | `ReadArtifactSnapshot` | Resolves `WRAP-002`, reads bounded contents, and produces `WRAP-003`; source changes during the read are reported. |
| **EFFECT-003** | `InspectGitContext` | Enriches a project with working-tree, branch, and recency facts; failure is non-fatal. |
| **EFFECT-004** | `StoreIndex` | Writes only to app-owned storage and never to a discovered project. |
| **EFFECT-005** | `ObserveFilesystem` | May emit refresh commands; manual refresh can interpret this effect in the first migration slice. |
| **EFFECT-006** | `GenerateWithPi` | Runs only after `EFFECT-002`, against the captured snapshot outside the repository, with `CTRL-004` and `CTRL-005`. |
| **EFFECT-007** | `StoreGeneratedView` | Stores the result with source fingerprint, included paths, generation time, mode, model, and prompt version. |
| **EFFECT-008** | `CopyHandoff` | Copies a path or derived continuation prompt after explicit user action. |
| **EFFECT-009** | `OpenExternalTarget` | Opens a terminal or configured URI after explicit user action; unsupported Superset integration remains deferred. |

If sources change after `EFFECT-006` starts, `EFFECT-007` may still cache the result, but the transition must place it in `STATE-003` `Stale`. Failure of any generated-view or launch effect cannot alter repository content or deterministic OpenSpec progress.

## 8. Applicability and omissions

All seven modeling categories apply and are represented: `@domain-wrapper`, `@rules`, `@edge-input`, `@domain-state`, `@domain-error`, `@domain-effect`, and `@pure-core-transition`.

The approved slice omits the following intentionally:

- Repository and artifact mutation, including edit, move, archive, delete, and “mark abandoned,” per `SCOPE-003`.
- Background AI discovery, classification, summarization, or autonomous continuation, per `DEC-008` and `RULE-009`.
- A full source-code tree or code editor, per `SCOPE-001` and `REQ-004`.
- Pi session inventory, restoration, and orchestration, per `DEC-002`.
- Guaranteed Superset deep links, per `SCOPE-004` and `DEC-013`.
- AI-derived task completion or lifecycle status, per `RULE-003`, `RULE-004`, and `RULE-008`.
- Automatic Pi retries. A user may explicitly regenerate after an operational failure.

## 9. Pure transition

`@pure-core-transition`

- **TRANS-001** The deterministic core accepts current application state, a command or observed adapter event, and controls. It returns next state, typed domain errors, requested effects, and operational warnings without performing I/O.

F#-first shape:

```fsharp
type Transition = {
    State: AppState
    Effects: DomainEffect list
    Warnings: OperationalWarning list
}

val transition:
    Controls -> AppState -> Command -> Result<Transition, DomainError>
```

Representative commands are `RefreshRoot`, `ScanCompleted`, `SourcesChanged`, `RequestGeneratedView`, `GenerationCompleted`, `GenerationFailed`, `CopySelectedHandoff`, and `OpenSelectedTarget`.

- **TARGET-001** In Rust, model commands, states, domain errors, effects, and parse outcomes as enums and records/structs. Implement the transition as a pure function returning `Result<Transition, DomainError>`. Tauri command handlers and platform adapters interpret effects; the web frontend renders state and sends commands. The architecture does not require a particular frontend framework.

Key deterministic decisions are artifact classification from detector output, OpenSpec progress from parsed tasks, generated-view freshness from fingerprints, preservation of previous usable state, and whether a request produces an effect or `ERR-001` through `ERR-003`.

## 10. Expectation and verification matrix

| ID | Preconditions and input | Expected state, error, and effects | Evidence strategy | Traceability |
|---|---|---|---|---|
| **EXP-001** | An approved root contains a Git project with a valid OpenSpec change and mixed task markers; refresh is requested. | `EFFECT-001` through `EFFECT-004`; `STATE-001` becomes `Ready`; `STATE-002` is recognized and parsed; `STATE-004` reports exact completed and remaining counts. | Representative filesystem fixture plus deterministic parse and transition evidence | `ACT-001`, `REQ-001`–`REQ-003`, `FLOW-001`, `RULE-004` |
| **EXP-002** | A recognized OpenSpec or plan file contains malformed structure. | Artifact remains viewable under `STATE-002` `ParsedWithWarnings`; Markdown rendering remains available; no false progress is invented. | Malformed fixture and rendered-state inspection | `REQ-002`, `REQ-004`, `RULE-007`, `ERR-004` |
| **EXP-003** | A selected bundle fits the context limit and has no cached summary; the user requests Summary. | `STATE-003` moves through `Generating` to `Current`; `EFFECT-002`, `EFFECT-006`, and `EFFECT-007` occur in order with provenance stored. | Captured effect sequence and a controlled Pi-adapter response | `REQ-006`, `FLOW-003`, `RULE-009` |
| **EXP-004** | A current cached summary exists, then an included file changes and refresh completes. | `WRAP-003` changes; `STATE-003` becomes `Stale` and identifies the changed path/date; the prior summary remains readable with Regenerate available. | Before/after source manifests and transition output | `REQ-007`, `RULE-005`, `DEC-010` |
| **EXP-005** | A stale summary exists and explicit regeneration times out. | `STATE-003` becomes `Failed` with the previous stale result preserved; deterministic artifact facts remain unchanged. | Controlled timeout from the Pi adapter | `REQ-006`, `RULE-006`, `ERR-004` |
| **EXP-006** | Sources change after Pi generation starts but before its result returns. | The result may be cached with its original fingerprint but enters `STATE-003` `Stale`; it cannot appear current. | Interleaved source-change and generation-completion events | `REQ-007`, `RULE-005`, `EFFECT-007` |
| **EXP-007** | A project disappears during refresh and later reappears. | `STATE-001` becomes `Unavailable(previous)` without losing prior context, then returns through `Scanning` to `Ready` after a later refresh. | Controlled disappearance/recovery of a fixture root | `REQ-001`, `STATE-001`, `ERR-002` |
| **EXP-008** | A symlink or requested path resolves outside all approved roots. | Return `ERR-001`; perform no artifact read and no Pi invocation. | Path-containment evidence including symlink escape cases | `DEC-003`, `WRAP-001`, `WRAP-002`, `RULE-002` |
| **EXP-009** | The selected folder overview exceeds the configured source limit. | Return `ERR-003`; request no Pi effect; preserve any cached result. | Boundary-size inputs around `CTRL-004` | `REQ-006`, `CTRL-004`, `ERR-003` |
| **EXP-010** | A user requests a continuation prompt for a selected artifact. | `EFFECT-008` contains project path, selected bundle, deterministic status, and explicit continuation instructions; generated claims are labeled or omitted. | Inspect derived handoff against selected state | `REQ-005`, `FLOW-004`, `RULE-003` |

These scenarios cover the normal path, rejection, parser partial failure, recovery, stale-cache recovery, and source-change concurrency without assigning product meaning to operational faults.

## 11. Migration slice

- **MIG-001** Deliver one read-only vertical slice: configure one approved parent root; discover Git working trees; recognize OpenSpec change bundles; parse task progress; browse bundles in the three-pane UI; render Markdown; copy paths and continuation prompts; and generate one on-demand cached Summary with fingerprint-based staleness.

Value: this slice proves that orphaned OpenSpec work can be found, understood, and handed back to an agent without repository changes.

Risk: project-boundary mistakes, parser drift across OpenSpec variants, path escape, large snapshots, and unsafe Pi invocation. Verification uses `EXP-001` through `EXP-010` before broadening artifact detectors.

Non-goals for this migration slice are Wayfinder/evidence/TDD detector breadth, automatic filesystem watching, ELI5 and other prompt modes, external deep links, session management, and artifact mutation. These can be added behind the same transitions and effects.

Rollback and compatibility guardrail: the slice writes only app-owned configuration and cache data. Removing that data and the application restores the prior environment; repositories remain byte-for-byte untouched. Unsupported or malformed artifacts fall back to candidate classification and Markdown viewing rather than destructive migration.

## 12. Assumptions, deferred decisions, and confidence

### Assumptions

| ID | Assumption | Evidence and consequence |
|---|---|---|
| **ASSUME-001** | Most relevant projects are Git working trees beneath a small set of approved parent roots. | This matches the user’s repository-based workflow. If false, explicit project registration must become primary. |
| **ASSUME-002** | Pi supports a noninteractive invocation that can consume a bounded snapshot without repository-write tools or repository working-directory access. | This must be confirmed before enabling `EFFECT-006`; otherwise v1 must omit Pi or use another authorized read-only model adapter. |
| **ASSUME-003** | App-owned local configuration and cache storage are acceptable. | The user allowed caching; without it, cross-session indexing and generated-view freshness cannot persist. |

### Deferred decisions and unresolved forks

| ID | Unresolved fork | Status | Owner / question |
|---|---|---|---|
| **FORK-001** | Nested repositories and worktrees | deferred | Product: show each working tree separately in the migration slice; should later versions group worktrees by common Git directory? |
| **FORK-002** | Exact deterministic detector registry for plans, TDDs, Wayfinder, evidence, artifacts, and candidate Markdown | deferred | Product: validate names and structures against real repositories after the OpenSpec slice. |
| **FORK-003** | Superset URI or CLI integration | deferred | Integration owner: what supported contract opens a new tab at a project path, and can it carry continuation context? |
| **FORK-004** | Web frontend framework inside Tauri | deferred | Implementation: choose from project constraints; it does not change the domain model. |
| **FORK-005** | Supported OpenSpec schema and version variants | deferred | Product/implementation: inventory real repositories before declaring parser coverage. |

Recommendation: implement `MIG-001` before broadening detectors or adding orchestration. The strongest risks are safe Pi invocation and path containment, not the UI framework.

Confidence: 90%.

The recommendation would change if Pi cannot be invoked without repository-write authority, if Git working trees poorly represent the user’s project boundaries, or if real OpenSpec variants cannot yield deterministic task progress. Deferred decisions do not block the read-only OpenSpec vertical slice unless repository inspection contradicts these assumptions.
