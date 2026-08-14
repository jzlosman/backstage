# Backstage v1 support

## Supported OpenSpec material

Backstage v1 recognizes supported files in both current and archived OpenSpec changes:

- `openspec/changes/<change>/proposal.md`
- `openspec/changes/<change>/design.md`
- `openspec/changes/<change>/tasks.md`
- `openspec/changes/<change>/specs/<capability>/spec.md`
- The same member paths below `openspec/changes/archive/YYYY-MM-DD-<change>/`

It groups these files into one OpenSpec change bundle with the same Overview, Tasks, and Source reader in either location. Current changes with open or unavailable progress are **Active**. A current change with no open tasks is **Done**. A change in the archive is **Archived** regardless of its task counts; archival location and task progress remain separate facts.

Backstage parses task markers in `tasks.md` when a Markdown list item uses `- [ ]`, `- [x]`, `- [X]`, `* [ ]`, `* [x]`, or `* [X]`. Markers inside fenced code blocks do not count. Unsupported or malformed markers produce warnings; the source remains readable.

The default **Plan files** scope also includes Markdown paths matched by app-owned Rust-compatible regular expressions. Settings seeds removable patterns for `PLAN.md`/`plan.md`, `TDD.md`/`tdd.md`, and `ROADMAP.md`/`roadmap.md` at any project-relative depth. Users may remove every default, add global project-relative path patterns, or restore missing defaults. Pattern changes remain local and trigger bounded rescans of approved roots.

The opt-in **All Markdown** scope indexes every safely readable `.md` file within the scan bounds. Ordinary Markdown remains an ordinary document: it receives no OpenSpec lifecycle, task progress, continuation prompt, or Pi workflow. Search and counts cover the complete index while the ledger mounts records in bounded batches. Every ledger scope sorts observed source modification times newest first under **Today**, **Past 7 days**, **Older**, and **Date unavailable** headings.

## Pi capability requirements

Generated Summary views require the audited Pi CLI version `0.82.1` and the configured `openai-codex/gpt-5.6-sol` model. Backstage invokes Pi only after **Generate Summary** or **Regenerate Summary**.

The capability probe requires:

- noninteractive JSON mode;
- `--no-tools`;
- extensions, skills, templates, themes, context files, and project trust disabled;
- an app-owned working directory and Pi configuration directory;
- a macOS `sandbox-exec` profile that allows filesystem reads but denies writes outside the app-owned Pi directory;
- a strict successful assistant result and `agent_settled` event;
- no tool, retry, or compaction events;
- bounded input and output;
- no repository working directory.

Generation stays disabled when the installed version, executable path, authentication, model, nonce, event stream, timeout, output limits, or macOS sandbox fail the probe. Backstage resolves `BACKSTAGE_PI_EXECUTABLE`, the current `PATH`, or an installed NVM Pi executable. The macOS process sandbox gives the Pi subprocess read access to the bounded environment but write access only inside Backstage's app-owned Pi directory. Platforms without an equivalent configured sandbox keep generation disabled.

## App-owned data

On macOS, the `directories` crate derives Backstage's configuration, cache, and data directories for organization `Earendil`, application `Backstage`, and bundle ID `works.earendil.backstage`. Approved roots, planning patterns, indexes, generated views, and preferences live in the app data path as `backstage.sqlite3`. Pi isolation files live under the app cache path in `pi/`.

Settings can remove an approved root and its unreachable app-owned index and generated views. This action never removes or changes the folder. Backstage writes no configuration, index, generated view, or preference data to an approved repository. Removing the app and its app-owned directories removes all Backstage state; repositories need no rollback.

## Detector gaps

Backstage v1 does not claim support for:

- planning detectors beyond project-relative Markdown regular expressions;
- per-root or per-project planning-pattern overrides;
- Wayfinder or evidence-folder semantics without a matching user pattern;
- every historical or future OpenSpec schema or nonstandard archive layout;
- lifecycle labels inferred from content, such as abandoned;
- nested worktree grouping by shared Git directory;
- browsing non-Markdown source files;
- Superset deep links;
- Pi session inventory or restoration.

Nested Git working trees appear as separate projects. Git metadata failure adds a warning but does not hide readable project material.

## macOS development and test build

Prerequisites: Rust 1.85 or newer, Node 22 or newer, pnpm 10, Xcode command-line tools, and the Tauri v2 macOS prerequisites.

```bash
pnpm install
pnpm format
pnpm lint
pnpm test
pnpm typecheck
pnpm build:release
openspec validate --all --strict
pnpm exec tauri build --bundles app
```

An unsigned local test application is written to:

```text
target/release/bundle/macos/Backstage.app
```

Launch and verify the local packaged bundle with:

```bash
./scripts/smoke-packaged-macos.sh
```

The harness launches the actual `.app`, checks its startup log, then runs the disposable real Git/OpenSpec vertical flow through the same command-domain adapters: discovery, parsed progress and Markdown, controlled Summary state, source invalidation, handoffs, and before/after repository manifests. Real Pi invocation remains separately covered by sandbox/capability integration tests so smoke verification does not spend provider credits or depend on network availability.
