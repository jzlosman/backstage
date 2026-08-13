## Why

Backstage is ready for internal v0 distribution, but it still uses a generic archive icon and ordinary Markdown documents lack the path handoff available to planning artifacts. A recognizable product mark and a consistent read-only copy-path action will make the build feel intentional and easier to use without expanding its repository authority.

## What Changes

- Generate several original Backstage logo concepts and select one against the existing Accession Desk visual direction and small-size legibility.
- Install the selected mark in the titlebar and the macOS application bundle.
- Add a `Copy path` action to the ordinary Markdown reading desk.
- Resolve Markdown paths from the current safe index and approved-root containment boundary before writing them to the system clipboard.
- Show concise success and failure feedback through the existing handoff notice patterns.
- Preserve the current planning-artifact handoffs and repository read-only guarantees.

### Non-goals

- Rename Backstage or replace the Accession Desk visual system.
- Build a full public brand system, marketing identity, or typography program.
- Add Markdown continuation prompts, terminal actions, editing, or write access.
- Add new remote services, analytics, or generation workflows.

## Capabilities

### New Capabilities

- `v0-product-identity`: An original Backstage mark appears consistently in the product chrome and packaged macOS application.
- `markdown-path-handoff`: An ordinary indexed Markdown document exposes a safe read-only path-copy handoff.

### Modified Capabilities

None.

## Impact

- Frontend titlebar, Markdown reader, API contract, styles, tests, and static assets.
- Tauri commands and path-derivation tests.
- macOS icon bundle generated from the selected source asset.
- No new runtime dependency or repository mutation authority.
