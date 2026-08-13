## Context

Backstage already has an intentional Accession Desk interface and a tested three-pane shell. The audit found failures at the seams: a persisted collapsed pane can hide the only selection path, the command palette declares modal behavior without containing focus, separators lack keyboard interaction, compact project rows lose identity, and visual primitives have drifted into many one-off colors and SVGs. The implementation must remain frontend-only and preserve local, read-only repository handling.

## Goals / Non-Goals

**Goals:**

- Make the existing master-detail workspace recoverable across relaunch and narrow widths.
- Complete keyboard and assistive-technology behavior for the palette, pane separators, and refresh shortcut.
- Keep projects identifiable and expose deterministic indexed-file counts.
- Improve narrow layout, control targets, icon consistency, contrast, and reduced-motion behavior without redesigning the product.
- Remove measurable render work from pane resizing and unrelated state changes.
- Cover each audited regression with focused frontend tests and bounded browser verification.

**Non-Goals:**

- Change artifact discovery, Rust commands, index storage, or repository permissions.
- Add a theme switcher, new visual direction, mobile-native information architecture, or new generated content.
- Persist selected artifact content or add routing.
- Virtualize current v1 data volumes unless measurement shows a separate bottleneck.

## Decisions

### Derive effective pane visibility from saved preference and current selection

Store the user’s explicit ledger preference, but force the ledger visible when no artifact is selected. At narrow widths, selecting a bundle collapses the ledger; the named titlebar control restores it. This preserves the existing master-detail behavior without persisting stale selection state.

Alternative considered: persist the selected artifact. Rejected because source/index freshness and missing artifacts would require a broader restoration contract.

### Keep the custom palette and implement the ARIA dialog pattern

Add first/last focus wrapping, Escape close, initial focus, trigger restoration, and background isolation while open. This is narrower than replacing the overlay with a new component system and preserves current interaction and tests.

Alternative considered: native `<dialog>`. Deferred because changing overlay semantics and Tauri WebView behavior adds unrelated migration risk.

### Make separators real keyboard controls

Give each separator a focus position and range metadata. Arrow keys adjust by a small step, Shift-arrow by a larger step, and Home/End reach bounds. Pointer movement is scheduled through one `requestAnimationFrame`, with cleanup on pointer release and unmount.

### Derive project visibility and counts from the loaded index

Build a memoized map from project ID to the number of unique bundle members. Show only projects with at least one indexed member and use that same filtered set for registry and workspace counts; discovered Git projects without planning work do not belong in work navigation. No backend or schema change is needed. Compact rows show a stable short project monogram plus count; their accessible names retain the full project, branch state, and count.

### Use Phosphor React with explicit imports

Add `@phosphor-icons/react` and import only the required icons. Use one weight and the existing size scale. Accessible names remain on controls; `title` supplies a visible native tooltip for unfamiliar icon-only controls. Tree-shaken imports keep the bundle impact bounded.

Alternative considered: copy Phosphor SVG paths into local components. Rejected because that recreates one-off icon ownership and makes upgrades inconsistent.

### Consolidate semantic tokens without inventing dark mode

Promote existing values into semantic tokens for surfaces, text, borders, focus, selection, warning, error, success, and disabled states. Replace repeated hard-coded values in the audited path. The project rail remains the intentional dark surface; this change does not introduce theme switching.

### Memoize expensive source rendering at the artifact boundary

Memoize sanitized Markdown from `detail.markdown` and keep cheap shell state outside that computation. Measure production bundle output before and after icon adoption; do not add broader memoization without evidence.

### Replace blanket reduced-motion timing with explicit states

Under `prefers-reduced-motion`, stop scan sweeps and pulses and show their static terminal/active visual state. Retain immediate color and focus changes. No new decorative motion is added.

## Risks / Trade-offs

- **[Risk] Compact monograms may collide across similarly named projects** → Pair them with a visible count, full native tooltip, and complete accessible name; keep the full names above the compact breakpoint.
- **[Risk] Focus containment can break if the palette command set changes** → Query enabled focusable controls at keydown time and test forward and reverse wrapping.
- **[Risk] Phosphor increases JavaScript size** → Use direct named imports, compare gzip output, and reject accidental broad imports.
- **[Risk] Semantic-token cleanup can create broad visual churn** → Limit replacements to the existing palette and audited path; verify desktop and narrow screenshots in one bounded pass.
- **[Risk] Forced ledger visibility can appear to ignore a saved preference** → Apply the preference only when a selected artifact makes collapse meaningful.
- **[Risk] Frame scheduling can drop the final pointer position** → Flush the latest coordinate on pointerup before canceling scheduled work.

## Migration Plan

1. Add failing frontend tests for effective pane visibility, palette focus wrap, keyboard resizing, refresh shortcut, project counts/copy, and Markdown memoization.
2. Add the icon package and implement behavior changes behind existing components.
3. Consolidate tokens and responsive rules.
4. Run unit, accessibility-oriented interaction, type, lint, build, detector, and bounded browser checks.
5. Rebuild and relaunch the packaged app. Rollback is a frontend code/dependency revert; no persisted data migration is required.

## Open Questions

None. The audit, supplied screenshot, existing PRODUCT.md, and DESIGN.md define the required scope.
