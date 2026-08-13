## Why

Backstage’s visual system is coherent, but its narrow layout, keyboard paths, pane persistence, and control affordances can hide work or block non-pointer users. The first real multi-project run also showed that each project needs an immediate planning-file count and clearer artifact-candidate language.

## What Changes

- Keep the bundle ledger recoverable when no artifact is selected, including after relaunch and at narrow widths.
- Contain focus in the command palette and make pane separators keyboard-operable.
- Preserve project identity, primary actions, readable content, and adequate targets from 320 px through desktop widths.
- Add project planning-file counts derived from the current local index and omit projects with no indexed planning work from the registry.
- Rename vague candidate labels and evidence using factual, user-facing language.
- Make the documented refresh shortcut work.
- Memoize Markdown rendering and bound pointer-resize updates to animation frames.
- Replace one-off interface SVGs with one consistent Phosphor icon family.
- Consolidate semantic color tokens, strengthen non-text contrast, and provide intentional reduced-motion states.
- Add regression coverage for the audited behavior.

Non-goals:

- No repository writes or changes to Backstage’s read-only boundary.
- No new artifact detectors, generated-view modes, themes, routes, or backend APIs.
- No redesign of the Accession Desk visual direction.
- No session management, mobile-native navigation, or autonomous Pi behavior.

## Capabilities

### New Capabilities
- `audited-interface-hardening`: Accessible, responsive, recoverable operation of the existing Backstage workspace across supported viewport and input modes.

### Modified Capabilities

None. The repository has no archived baseline specs; this change records the hardened interface contract as a new capability.

## Impact

The change affects the React workspace shell, pane-layout preference handling, Markdown rendering, interface copy, CSS tokens and responsive rules, frontend tests, and the frontend icon dependency. Rust commands, persisted index schemas, repository scanners, and repository safety boundaries remain unchanged.
