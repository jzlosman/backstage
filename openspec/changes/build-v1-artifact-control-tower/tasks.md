## 1. Workspace and safety baseline

- [x] 1.1 Scaffold the Rust Cargo workspace, Tauri v2 application, and React/TypeScript frontend with repeatable format, lint, test, and build commands.
- [x] 1.2 Create the pure Rust core crate and adapter-facing Tauri crate, with compile-time boundaries that keep Tauri, filesystem, Git, clock, storage, Pi, clipboard, and launcher dependencies out of the core.
- [x] 1.3 Define app-owned configuration/cache paths and verify startup creates no files inside a scanned fixture repository.
- [x] 1.4 Add representative temporary Git/OpenSpec fixtures plus a repository-manifest assertion used before and after every read-only integration flow.

## 2. Approve one root and discover real projects

- [x] 2.1 Write failing core tests and implement `ApprovedRoot` and `ArtifactPath` normalization, absolute-directory validation, stable IDs, and serialized domain errors.
- [x] 2.2 Write failing adapter tests and implement canonical containment checks that reject relative paths, traversal, and symlink escapes immediately before each read.
- [x] 2.3 Implement app-owned root persistence and Tauri intents for listing, approving, and removing roots without accepting arbitrary frontend read paths.
- [x] 2.4 Implement bounded, cancellable traversal and Git working-tree discovery with exclusions, depth/file-size limits, partial warnings, and no write handles.
- [x] 2.5 Build the first-run approved-root surface and persistent project rail against real Tauri data, including no-root, scanning, ready-with-warnings, and unavailable states.
- [x] 2.6 Verify symlink escapes are unread, cancellation preserves safety, Git failure does not hide readable projects, and repository manifests remain unchanged.

## 3. Recognize and browse OpenSpec work

- [x] 3.1 Write failing core tests and implement detector evidence, recognized/possible artifact states, project grouping, and OpenSpec bundle grouping.
- [x] 3.2 Write failing parser tests and implement supported OpenSpec task-marker parsing with exact total/completed/remaining facts, source locations, parser provenance, and progress-unavailable fallback.
- [x] 3.3 Implement immutable artifact snapshots and ordered bundle source fingerprints over normalized paths, contents, and membership, including source-change race detection.
- [x] 3.4 Add SQLite schema/migrations and atomic persistence for approved roots, the last usable index, parsed facts, fingerprints, and warnings; cover cache-write failure with an in-memory usable result.
- [x] 3.5 Expose typed index/detail Tauri queries and scan job events guarded by request generation so superseded results cannot replace newer state.
- [x] 3.6 Build the bundle ledger and All Work view for unfinished, warning-bearing, possibly stale, and recently changed artifacts, with project and deterministic-state filters.
- [x] 3.7 Verify malformed OpenSpec content remains browsable with warnings, ordinary Markdown is excluded, possible artifacts show their evidence, refresh failure preserves the prior index, and project recovery replaces unavailable state.

## 4. Read artifact content and perform handoffs

- [x] 4.1 Implement sanitized Markdown rendering that supports headings, lists, task markers, tables, links, and code blocks while blocking raw active content and external execution.
- [x] 4.2 Build the reading/detail pane with a single provenance spine for project, bundle, artifact path, source dates, Git context, parser status, deterministic progress, warnings, and source fingerprint.
- [x] 4.3 Implement backend-derived copy-path and continuation-prompt intents using stable selected IDs, deterministic status, explicit continuation instructions, and labeled or omitted generated claims.
- [x] 4.4 Implement platform launcher ports and a macOS terminal adapter, plus configured-external-target failure behavior that offers copy alternatives.
- [x] 4.5 Verify Markdown injection is inert, path and prompt copies use normalized approved paths, launcher requests execute no repository command, and all handoffs preserve repository manifests.

## 5. Generate and invalidate one Pi summary

- [x] 5.1 Investigate the installed Pi CLI and add an executable capability probe proving noninteractive invocation from an app-owned location with repository-writing tools unavailable; keep generation disabled when the probe fails.
- [x] 5.2 Write failing core transition tests for `NeverGenerated`, `Generating(previous?)`, `Current`, `Stale`, and `Failed(previous?)`, including source changes during generation and prior-result preservation.
- [x] 5.3 Implement bounded source-snapshot creation with approved-path revalidation, file/byte limits, untrusted-source prompt envelope, mode, prompt version, and no repository working directory.
- [x] 5.4 Implement the cancellable Pi adapter and generation job events with request/fingerprint guards, timeout handling, malformed-response handling, and no automatic retry.
- [x] 5.5 Extend SQLite persistence for generated text, included paths, source fingerprint, generated time, model metadata, mode, and prompt version; reuse only equivalent current cache entries.
- [x] 5.6 Build the Summary UI with explicit user invocation, current/stale/generating/failed labels, changed-input explanation, regenerate action, and previous-result preservation.
- [x] 5.7 Verify browsing never invokes Pi, over-limit and escaped scopes invoke no process, contradictory summaries cannot alter task facts, timestamp-only changes preserve freshness, and content/membership changes mark summaries stale.

## 6. Complete the desktop operating shell

- [x] 6.1 Implement resizable persistent panes following the confirmed Accession Desk direction, keeping the project rail accessible and preserving selection when the bundle ledger collapses.
- [x] 6.2 Implement global search, command palette, pane-to-pane keyboard navigation, focus restoration, visible focus, and equivalent mouse/trackpad actions.
- [x] 6.3 Add honest empty/loading/warning/failure states with skeleton/progressive scan feedback, reduced-motion behavior, and text/shape cues that do not rely on color.
- [x] 6.4 Add frontend tests for first-run root approval, project filtering, bundle selection, keyboard-only artifact inspection, stale-summary regeneration, failed regeneration with prior content, and command-palette focus restoration.
- [x] 6.5 Confirm the shell remains usable at 10–20 projects and 50–200 bundles and add ledger windowing only if measured rendering requires it.

## 7. Integrated verification and design finish

- [x] 7.1 Run Rust unit and integration tests, frontend tests, type checking, linting, release builds, and strict OpenSpec validation; resolve every failure before packaging.
- [x] 7.2 Run a packaged macOS smoke flow against a disposable real Git/OpenSpec fixture: approve root, discover bundle, inspect progress/Markdown, generate Summary, change a source, observe stale state, and copy/open handoffs.
- [x] 7.3 Compare repository manifests before and after the complete smoke flow and fail release verification on any project-file or directory mutation.
- [x] 7.4 Apply Impeccable to the implemented shell, produce the required desktop/narrow inspection captures, run its detector and finish reviewer, resolve material findings within the bounded review cycle, and record the built system in DESIGN.md.
- [x] 7.5 Record supported OpenSpec syntax, Pi capability requirements, app-owned data locations, known detector gaps, and macOS test-build instructions without claiming deferred Superset or session-management support.
