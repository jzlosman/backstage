# Backstage v1 support

## Supported OpenSpec material

Backstage v1 recognizes files below `openspec/changes/<change>/` when they match this set:

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/<capability>/spec.md`

It groups these files into one OpenSpec change bundle. It parses task markers in `tasks.md` when a Markdown list item uses `- [ ]`, `- [x]`, `- [X]`, `* [ ]`, `* [x]`, or `* [X]`. Markers inside fenced code blocks do not count. Unsupported or malformed markers produce warnings; the source remains readable.

Candidate filenames `PLAN.md`, `plan.md`, `TDD.md`, `tdd.md`, `ROADMAP.md`, and `roadmap.md` appear as possible artifacts with their deterministic filename evidence in the default **Plan files** scope.

The opt-in **All Markdown** scope indexes every safely readable `.md` file within the scan bounds. Ordinary Markdown remains an ordinary document: it receives no planning state, OpenSpec view, continuation prompt, or Pi workflow. Search and counts cover the complete index while the ledger mounts records in bounded batches.

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

On macOS, the `directories` crate derives Backstage's configuration, cache, and data directories for organization `Earendil`, application `Backstage`, and bundle ID `works.earendil.backstage`. The SQLite index lives at the app data path as `backstage.sqlite3`. Pi isolation files live under the app cache path in `pi/`.

Backstage writes no configuration, index, generated view, or preference data to an approved repository. Removing the app and its app-owned directories removes Backstage state; repositories need no rollback.

## Detector gaps

Backstage v1 does not claim support for:

- planning classification for arbitrary filenames;
- Wayfinder or evidence-folder structure;
- broad TDD variants beyond deterministic candidate filenames;
- every historical or future OpenSpec schema;
- lifecycle labels such as abandoned;
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
