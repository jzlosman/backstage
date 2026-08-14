# Product

## The problem

You can finish work in one agent tab and plan the next step in another. Those plans may land in different repos and folders. Some use OpenSpec. Others are loose Markdown files from Claude, Codex, Pi, a Grill Me session, or your own notes.

When you return later, you need to answer three questions:

- What planning work exists?
- Where did I leave off?
- What should I give the next agent?

Finding those answers by hand gets harder as the number of projects and agent sessions grows.

## What Backstage helps you do

Backstage is a read-only macOS app for local planning files.

You can use it to:

- Find planning work across approved local folders.
- Add and remove approved folders from an app-owned Settings view.
- Read current and archived OpenSpec changes as an overview, task list, or exact source.
- Distinguish Active, Done, and Archived changes while keeping task progress separate.
- Find recently modified work first, grouped by local date ranges.
- Check task progress from source checkboxes.
- Configure removable regular-expression patterns for planning Markdown paths.
- Browse all safely indexed Markdown when a file is not planning work.
- Copy an exact path or continuation prompt into a new agent session.
- Ask Pi for a summary when you choose to send the source.

Backstage groups recognized OpenSpec files. It does not claim to recognize Wayfinder folders, evidence folders, or every planning format. Files from those systems remain available as ordinary Markdown in **All Markdown** when they are within the scan bounds.

## Who it is for

Backstage is for developers who work across many Git projects and agent sessions. They need a quick way to recover context after tabs close or work sits for a while.

The first public release is for developers who use macOS and keep plans as local Markdown.

## What success looks like

A user can:

1. Approve a local folder.
2. Find the relevant plan without searching each repo.
3. See what is complete and what remains.
4. Read the exact source when needed.
5. Copy a path or continuation prompt.
6. Resume the work in another agent session.

The scanned repo stays unchanged throughout this flow.

## Safety boundaries

- Backstage does not edit, move, archive, delete, or mark repository files.
- All scanning, parsing, indexing, and caching stay local.
- Backstage sends source content to Pi only after an explicit request.
- Pi generation never runs as a background classification step.
- Generated summaries keep their source list and fingerprint. Backstage marks them stale when the source changes.
- OpenSpec task progress comes from deterministic parsing, not from a model.
- Parser failures leave the source readable and show a warning.
- Backstage scans only roots that the user approves.
- It rejects path traversal and links that escape an approved root.
- App settings, indexes, and generated summaries live in app-owned folders.

## Public preview scope

The public preview includes:

- Local discovery of Git working trees
- Current and archived OpenSpec change bundles
- Structured OpenSpec overview, task, and source views
- Explicit Active, Done, and Archived status with open/done task counts
- Configurable planning-path patterns with removable plan, TDD, and roadmap defaults
- App-owned approved-root management
- Opt-in browsing of safely indexed Markdown
- Newest-first date grouping, search, and filters
- Bounded row mounting for large indexes
- Optional, cached Pi summaries
- Freshness checks
- Copy-path and continuation-prompt handoffs
- Terminal handoff to the project folder

It does not include:

- Repository changes
- Automatic work continuation
- Non-Markdown source browsing
- Chat-history restoration
- Pi session management
- Guaranteed Superset deep links
- Complete support for every planning format

## Platform and stack

- macOS 13 or newer
- Universal Apple Silicon and Intel build
- Rust and Tauri desktop application
- React and TypeScript interface
- Local app-owned configuration, SQLite index, and generated-summary cache

## Product name and mark

The product name is **Backstage**.

The Backstage mark uses stacked accession cards to form a letter B. The frontend and Tauri icon sources contain the approved assets.

## Evidence and public claims

The project has no user testimonials, benchmarks, or usage data yet. Public material must not invent them.

The approved architecture is documented in [`artifact-control-tower-v1.md`](artifact-control-tower-v1.md). Supported syntax and known limits are documented in [`docs/v1-support.md`](docs/v1-support.md).

## Product principles

1. **Leave repository truth untouched.** Read project files without changing them.
2. **Show facts before guesses.** Keep parsed facts, warnings, and model output separate.
3. **Help the user resume.** Make the next useful handoff easy to copy.
4. **Keep work local by default.** Send content to Pi only after a clear request.
5. **Keep a busy workspace calm.** Show many projects without becoming another IDE.
