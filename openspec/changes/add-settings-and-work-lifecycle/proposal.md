## Why

Backstage cannot remove approved roots from the interface, fixes planning-file conventions in code, and treats archived OpenSpec changes as ordinary Markdown. Its work ledger also buries modification dates, making recent work harder to recover than it should be.

## What Changes

- Add a dedicated in-app Settings view, opened from the titlebar and command palette, with a ruled register of approved roots and explicit Add and Remove actions.
- Remove the growing Approved Roots list from the project registry while preserving the registry as the permanent work-navigation anchor.
- Add a global ordered list of planning-path regular expressions. Seed the current plan, TDD, and roadmap conventions as ordinary removable defaults; support adding, removing, and restoring defaults.
- Validate and bound planning patterns before saving them, match them against normalized project-relative Markdown paths, and rescan approved roots after a successful configuration change.
- Recognize archived changes at `openspec/changes/archive/YYYY-MM-DD-<change>/` as OpenSpec bundles with the same Overview, Tasks, and Source experience as current changes.
- Keep archival custody separate from deterministic task progress: a current change with no open tasks is **Done**, while an archived change is **Archived** even if its task file is incomplete or unavailable.
- Make lifecycle and progress prominent in the work ledger and reading desk with explicit Active, Done, or Archived text plus separate open and done task counts.
- Sort work by observed source modification time, newest first, and divide the ledger into Today, Past 7 days, Older, and Date unavailable groups. Promote the date from secondary metadata to a primary scan cue.

Non-goals:
- Editing, completing, closing, moving, or archiving repository files from Backstage.
- Running OpenSpec CLI commands to infer lifecycle during background scans.
- Per-root planning-pattern overrides in this iteration.
- Matching non-Markdown files or weakening scan containment and resource bounds.
- Treating task completion and archival custody as interchangeable states.

## Capabilities

### New Capabilities
- `workspace-settings`: Navigate and operate a dedicated settings surface for approved-root management.
- `planning-patterns`: Configure bounded global planning-path patterns with removable defaults and deterministic rescans.
- `openspec-lifecycle`: Recognize current and archived OpenSpec changes uniformly while preserving lifecycle and task progress as separate facts.
- `activity-ledger`: Sort and group work by source recency while making lifecycle, dates, and open/done counts easy to scan.

### Modified Capabilities

None.

## Impact

- Rust configuration, storage migrations, root removal coordination, regex validation, artifact detection, OpenSpec path classification, lifecycle models, index persistence, and contained rescans.
- Tauri commands and frontend API contracts for settings, pattern mutations, root removal, lifecycle, and source-time grouping inputs.
- React titlebar navigation, Settings view, project registry cleanup, bundle ledger rows/groups, work filters, and OpenSpec reading-desk metadata.
- Core, persistence, catalog, safety, frontend interaction, keyboard, responsive, and accessibility tests.
- No new network service or repository write path is introduced. A bounded Rust regex dependency may be added if the existing dependency graph does not already provide one.
