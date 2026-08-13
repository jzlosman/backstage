# Backstage Design System

<!-- impeccable:design-schema 1 -->

## Direction

Backstage uses the **Accession Desk** visual world: archival finding aids, accession ledgers, conservation-box labels, registry stamps, and a broad reading desk. The interface should feel calm and exact under dense daily use, never like an IDE, project-management dashboard, or decorative bureaucracy.

## Product Mode

**Operate.** Source lineage, deterministic state, keyboard flow, and familiar desktop affordances outrank expression. The artifact and its next valid task remain dominant.

## Composition

- A permanent dark project registry anchors the left edge.
- A cool-board bundle ledger holds dense, deterministic records and filters.
- A broad white reading desk owns the selected artifact, provenance spine, generated view, and handoffs.
- At narrower widths the bundle ledger collapses first; project registry and selected artifact remain accessible.
- Panes are resizable and app-owned preferences persist their widths.

## Color

Restrained strategy: neutral surfaces with deep teal for selection/actions and amber reserved for stale or warning-bearing states.

| Token | Value | Role |
|---|---|---|
| Graphite | `#171A1B` | Primary text and deepest structure |
| Paper | `#F7F8F6` | Reading surface |
| Archival board | `#E5E8E6` | Ledger and secondary surfaces |
| Rule | `#B7BFBD` | Finding-aid separators |
| Deep teal | `#28566B` | Current selection, primary actions, registry stamps |
| Teal dark | `#173D50` | Hover/pressed action |
| Amber | `#D58A2A` | Warning and stale state |
| Amber paper | `#F6E7CA` | Stale/generated warning field |

Never use color alone. Every state also carries explicit text, shape, or a named action.

## Typography

Use the macOS system sans stack for dense product operation. Use `ui-monospace` only for paths, fingerprints, code, and source locations. Headings use tight but restrained tracking, never below `-0.04em`. Body Markdown targets `65–75ch`.

## Component Grammar

- **Registry stamps:** double teal rule, uppercase measured lettering, slight physical rotation. Used for provenance, not decoration.
- **Project rows:** flat ledger entries; teal full-row selection.
- **Bundle records:** ruled rows with small deterministic kind labels and explicit progress text.
- **Provenance spine:** one definition-list grid, not repeated metadata cards.
- **Generated Summary:** clearly labeled Pi-generated; amber paper when stale/failed; prior output remains visible.
- **Buttons:** square archival controls with one-pixel rules. Primary actions use teal fill.
- **Warnings:** amber paper and plain recovery copy.
- **Focus:** visible amber outline; keyboard and pointer actions remain equivalent.

## Motion

Motion communicates state only. Scanning uses a bounded pass over skeleton rows and the progress rule. Normal transitions take 150–250 ms. `prefers-reduced-motion` reduces all nonessential motion to near-zero duration.

## Responsive Rules

- Desktop: project registry + resizer + bundle ledger + resizer + reading desk.
- Below 960 px: bundle ledger collapses; project registry remains visible.
- Below 680 px: project registry becomes an icon-width strip and nonessential titlebar metadata hides.
- Selected IDs are preserved when a pane collapses.

## Accessibility

Use semantic `aside`, `nav`, `article`, headings, definition lists, and named regions. Pane shortcuts are `Alt+1`, `Alt+2`, `Alt+3`; global search is `Cmd/Ctrl+F`; command palette is `Cmd/Ctrl+K`. Palette close restores focus. Markdown links remain inert rather than launching external content. Raw HTML, scripts, remote media, event handlers, and unsafe URLs are blocked.

## Quality Bar

The artifact document and task facts must dominate. Metadata appears once. No nested cards, dashboard metrics, glow, gradients, emoji icons, decorative grids, or fake terminal chrome. Every loading, empty, warning, unavailable, stale, generating, and failed state names what happened and what the user can do next.
