# Backstage v1 support

## Compiled-in planning formats

Backstage represents planning material as neutral Work Records. A deterministic, compiled-in registry evaluates formats in this order:

1. `openspec-v1`
2. `wayfinder-local-v1`
3. planning-path patterns
4. plain Markdown

A higher-precedence recognized format claims its safely indexed sources before fallback adapters, so each Markdown source appears in one ledger Work Record. Adapter code receives bounded source inventories or immutable captures; it does not receive filesystem, network, SQLite, Pi, or frontend callbacks. Version 1 does not load runtime plugins, executables, or repository-provided format definitions.

A Work Record's private subject identity derives from its project identity, stable format ID, and exact adapter record key. Adapter implementation-version changes preserve identity when those inputs and record-key semantics remain stable. A path, project, format, or record-key change creates a different subject; Backstage does not heuristically transfer annotations. Moving a current OpenSpec change into archival custody therefore creates a distinct subject.

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

The opt-in **All Markdown** scope indexes every safely readable `.md` file within the scan bounds. Ordinary Markdown remains an ordinary Work Record: it receives no OpenSpec lifecycle, Wayfinder frontier, or generated-summary workflow, but its exact source and read-only handoffs remain available. Search and counts cover the complete index while the ledger mounts records in bounded batches. Every ledger scope sorts observed source modification times newest first under **Today**, **Past 7 days**, **Older**, and **Date unavailable** headings.

## Local-Markdown Wayfinder

The compiled `wayfinder-local-v1` adapter recognizes only an exact, case-sensitive `.scratch/<effort>/map.md` path. It groups that map and safely indexed descendant Markdown below `.scratch/<effort>/` into one Work Record. A `map.md` elsewhere, a differently cased filename, or a Markdown link to GitHub, GitLab, Linear, Jira, or another remote tracker does not create a Wayfinder record and triggers no network fetch.

The structured reader provides **Overview**, **Questions**, and **Source**:

- Overview recognizes exact level-two `## Destination`, `## Notes`, `## Decisions so far`, `## Not yet specified`, and `## Out of scope` headings outside fenced Markdown. It preserves unambiguous, nonempty sections in source order.
- Only direct `issues/<NN>-<slug>.md` descendants are decision tickets. The number must contain at least two ASCII digits and normalize to a positive integer. The slug uses lowercase ASCII alphanumeric words separated by single hyphens. Noncanonical issue Markdown remains readable in Source but is not interpreted as a ticket.
- Before the first level-two heading, tickets recognize exact `Type:`, `Status:`, and `Blocked by:` metadata. Type supports `research`, `prototype`, `grilling`, and `task`. Status supports `claimed` and `resolved`; absent Status means open and unclaimed. Blocked by is a comma-separated list of two-or-more-digit positive ticket numbers. Outer value whitespace is ignored.
- Tickets recognize exact `## Question` and `## Answer` headings outside fences. Unsupported, empty, or duplicate fields become unavailable with a warning; Backstage does not select an arbitrary occurrence or infer an unsupported value.

The deterministic frontier contains open, unclaimed, otherwise valid tickets whose declared blockers each resolve to one resolved ticket in the same effort. Results sort by normalized ticket number, and the first is labeled the next candidate without being claimed. Missing, malformed, duplicate, ambiguous, or unresolved blockers exclude the affected ticket. Handoffs identify the exact map and observed frontier but never claim, resolve, or edit tickets.

Every source capture remains subject to approved-root containment, entry/depth/time limits, regular-file checks, maximum read size, UTF-8 handling, and source-stability checks. Excluded, escaping, oversized, unstable, or unreadable members are not interpreted. Safely readable partial records remain available with warnings. Rendered Markdown uses the existing sanitized, inert-link policy; exact bounded text remains in Source.

Local Wayfinder support does not include generated summaries, remote trackers, tracker authentication, issue mutation, synchronization, a canonical `frontier.md`, non-Markdown attachments, or alternate local grammars.

## Private Work Record annotations

Backstage stores private annotations in app-owned SQLite tables, separately from replaceable index snapshots and source-derived facts. Effective defaults require no row: **Undecided**, **Applicable**, favorite off, todo off, and no priority. Users may independently choose Approved or Rejected; Applicable, Obsolete, or Superseded; favorite; todo; and Low, Medium, or High priority. These annotations never alter OpenSpec lifecycle, task progress, Wayfinder status, or repository content.

Superseded requires one distinct replacement Work Record subject. Self-reference and direct or transitive cycles are rejected atomically. If a replacement temporarily disappears while its approved root remains, Backstage preserves the relationship and last-known local display details but disables navigation. Annotation-like repository frontmatter—including approval, favorite, todo, priority, obsolete, or supersession fields—is ignored as private annotation authority.

Accepted scans add or refresh historical subject-to-root routes. A temporary source absence does not delete annotations. Removing an approved root atomically removes its route, index, and unreachable generated views. A subject and its sparse annotation are deleted only when no retained approved-root route remains. If a forgotten target had an incoming supersession relationship from a retained subject, that retained subject becomes Obsolete without retaining the forgotten target's name or path. Private annotations are local to this Backstage installation; version 1 has no sync, sharing, import, or repository-frontmatter mode.

Annotation filters work across every compiled format. With no annotation filter, the ledger keeps its source-recency ordering and existing lifecycle, warning, stale, count, bounded-mounting, and selection-race behavior.

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

Settings can remove an approved root and reconcile its app-owned routes, subjects, annotations, index, and generated views. This action never removes or changes the folder. Generated views are owned by stable Work Record subjects. During the schema migration, reachable legacy bundle-owned cache rows are mapped to subject owners; rows that cannot be mapped safely are deleted and can be regenerated explicitly. Backstage writes no configuration, index, generated view, annotation, or preference data to an approved repository. Removing the app and its app-owned directories removes all Backstage state; repositories need no rollback.

## Detector gaps

Backstage v1 does not claim support for:

- runtime, dynamically downloaded, executable, or repository-provided format plugins;
- planning detectors beyond the compiled adapters and project-relative Markdown regular expressions;
- per-root or per-project planning-pattern overrides;
- Wayfinder conventions other than the exact local-Markdown grammar above;
- remote Wayfinder, GitHub, GitLab, Linear, Jira, or other tracker discovery, mutation, or sync;
- annotation frontmatter, annotation sync, shared annotations, or heuristic annotation transfer;
- evidence-folder semantics without a matching user pattern;
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
