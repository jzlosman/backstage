## Context

Backstage currently discovers Git projects, walks each project within explicit budgets, and promotes only recognized OpenSpec material and configured planning filenames into `ArtifactBundle`s. The frontend derives its project rail and ledger entirely from those bundles, so ordinary Markdown is neither listed nor addressable through the Tauri API. The existing generic Markdown reader can render a selected non-OpenSpec artifact, but the index has no standalone-document boundary.

Repository content remains untrusted and read-only. Discovery must stay beneath approved roots, obey exclusions and scan budgets, and avoid sending content to Pi without an explicit generation request. Persisted index snapshots are JSON and older snapshots must remain loadable.

## Goals / Non-Goals

**Goals:**

- Keep the existing planning-focused registry as the launch default.
- Add an explicit `Plan files / All Markdown` scope control.
- Make every in-budget, safely contained `.md` file addressable through a generic read path.
- Preserve recognized OpenSpec bundles and their structured viewer in both scopes.
- Avoid duplicate ledger entries for Markdown already available as a member of a visible planning bundle.
- Make project visibility, counts, search, and empty states accurately reflect the selected scope.
- Preserve deterministic classification, local-only indexing, and the repository read-only boundary.

**Non-Goals:**

- Reading non-Markdown formats.
- Treating ordinary Markdown as planning evidence.
- Editing or annotating repositories.
- Automatically generating Pi summaries for ordinary Markdown.
- Persisting the selected scope between launches.
- Removing existing scan depth, entry, timeout, exclusion, encoding, or file-size limits.

## Decisions

### Maintain a parallel Markdown document manifest

Each `IndexedProject` will gain a `markdownDocuments` collection containing stable document identity, project identity, project-relative path, and observed modified time. `#[serde(default)]` will preserve compatibility with persisted snapshots created before this field existed.

The catalog walk will collect every regular file with a case-insensitive `.md` extension while deriving planning evidence from the same bounded traversal. Classification will continue to discard ordinary Markdown as planning evidence; the manifest is an observed-file index, not a new artifact heuristic.

**Alternative considered:** Represent every Markdown file as a one-member `PossibleArtifact` bundle. Rejected because it would contaminate planning counts, progress filters, bundle fingerprints, generated views, handoffs, and recognition language.

### Compose All Markdown from bundles plus unrepresented documents

`Plan files` will retain the current bundle-only ledger. `All Markdown` will show the same planning bundles plus standalone rows for documents whose IDs are not members of a planning bundle. OpenSpec proposal, design, task, and spec files therefore remain accessible through one structured bundle and are not duplicated as generic rows.

Project file counts will use unique member/document IDs. Projects with no planning bundles stay hidden in the default scope but appear in `All Markdown` when they contain indexed Markdown.

**Alternative considered:** Replace planning bundles with one row per file in the broader scope. Rejected because it removes the meaningful OpenSpec unit and specialized viewer exactly when users broaden their view.

### Add a document-specific contained read boundary

A `get_markdown_detail(rootId, documentId)` command will resolve a document only through the current persisted index and read a fresh immutable snapshot with `ContainedReader`. The response will contain document provenance and exact bounded UTF-8 Markdown, without bundle progress, fingerprint, generated summary, or continuation-prompt semantics.

If the source changes between observation and read, the fresh contained snapshot is authoritative for the displayed detail and its provenance. Missing, oversized, non-UTF-8, unstable, or escaped files fail visibly without mutating the repository or replacing another selected item.

**Alternative considered:** Route standalone documents through `get_artifact_detail`. Rejected because that endpoint intentionally reconstructs live bundle state and OpenSpec structure.

### Model registry scope independently from work-state filters

The frontend will hold an ephemeral registry scope with `planning` as the initial value. The existing work-state filters continue to apply to bundles. In `All Markdown`, ordinary documents participate in `All`, search, and project filtering but do not pretend to be unfinished, stale, warning-bearing, or recently changed planning work unless the filter has a document-defined meaning. The UI will explain an empty filtered result rather than silently reclassifying documents.

The ledger will filter and count the complete in-memory result set but mount only a bounded initial batch, with an explicit control to reveal additional batches. Search is applied before that presentation bound, so every indexed path remains directly reachable without mounting thousands of rows at once.

### Preserve selection and asynchronous response safety

Switching scope will retain the selection only if its row remains visible. Otherwise the app will select the first visible row or show the scope-specific empty state. Bundle and document reads will share request sequencing so a delayed response cannot replace a newer selection or leak a generated result across rows.

## Risks / Trade-offs

- **More indexed paths increase payload and scan work** → Reuse the existing traversal, store metadata rather than content, preserve all budgets, and verify representative scale.
- **A broad repository may contain many Markdown files** → Keep the broader mode opt-in, apply deterministic ordering and existing search/project narrowing, and incrementally mount bounded result batches.
- **Mixed bundle/document rows could confuse planning status** → Use distinct labels and metadata; never assign planning recognition or progress to ordinary documents.
- **Old cache payloads lack document manifests** → Default the additive field to an empty collection and populate it on the next scan.
- **A document can disappear or change after indexing** → Resolve through the index, perform a fresh contained snapshot read, and surface recoverable read errors.
- **OpenSpec files could appear twice** → Exclude every bundle member ID from standalone document rows and test uniqueness.
