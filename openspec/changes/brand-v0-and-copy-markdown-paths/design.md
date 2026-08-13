## Context

Backstage already has a strong Accession Desk interface, but the titlebar and packaged app still use a generic archive glyph. Planning artifacts expose a safe `Copy path` handoff; ordinary Markdown documents expose the same provenance but no equivalent action. This v0 ship-readiness pass spans generated visual assets, frontend chrome, a Tauri clipboard command, and packaged macOS resources while preserving strict repository containment.

## Goals / Non-Goals

**Goals:**

- Give Backstage one distinctive, small-size mark that belongs to its archival control-tower visual language.
- Use the mark consistently in the titlebar and packaged macOS app.
- Let users copy the absolute path of the selected ordinary Markdown document.
- Resolve that path through the current index and approved-root reader immediately before copying it.
- Preserve concise accessible feedback and existing responsive behavior.

**Non-Goals:**

- Replace the Accession Desk design system or create a public marketing identity.
- Add a full family of lockups, typography, illustrations, or social assets.
- Add Markdown editing, continuation prompts, terminal handoffs, or Pi generation.
- Trust a path supplied directly by the frontend or mutate a scanned repository.

## Decisions

### Generate concepts, then select against explicit product criteria

Create three original icon concepts with the image-generation tool. Each concept will be a simple square app mark with no text, gradients, or illustrative detail. Selection criteria are: recognition at 24–32 px, fit with the graphite/paper/deep-teal Accession Desk palette, a clear silhouette, and distinction from generic archive/database icons. The selected concept becomes the only shipped mark; rejected concepts remain development artifacts outside the product bundle.

**Selection record:** Concept C won. Its three accession cards and central custody rail collapse into a recognizable abstract `B` at titlebar size while retaining Backstage's archival meaning. Concept A was distinctive but too detailed at 24 px; Concept B was clear but read as a generic file box. The selected C direction receives one simplification pass and is distilled to flat production geometry without adding new motifs.

Alternatives considered:

- **Keep the Phosphor archive icon:** consistent and crisp, but generic and not a product identity.
- **Commission a complete identity system:** higher ceiling, but disproportionate for an internal v0.
- **Use several generated marks in different contexts:** avoids adaptation work, but creates inconsistency and weak recognition.

### Use one selected source for both product chrome and bundle icons

Keep a high-resolution selected source under `src-tauri/icons/`, derive the platform icon set with the Tauri icon tool, and create a small optimized frontend asset from the same source. The titlebar retains the `BACKSTAGE` wordmark and descriptive text; only the generic glyph changes. This preserves hierarchy and avoids loading a large source image into the frontend.

Alternatives considered:

- **Embed the 1024 px source in the titlebar:** visually faithful but wasteful.
- **Manually redraw a separate SVG:** crisp, but risks drifting from the generated mark and pretending the raster concept is a finished vector system.

### Resolve Markdown paths at the trusted backend boundary

Add `copy_markdown_path(root_id, document_id)` beside the artifact path command. The backend loads the current index, creates a `ContainedReader` for the approved root, resolves the document through the existing Markdown-detail path, and calls the reader's contained file resolver before returning the canonical absolute path. Document identity remains project-relative-path based: a safe regular-file content update at that path is valid, while missing, non-regular, and escaping sources fail. The frontend writes only the returned value to the clipboard.

This duplicates a small amount of handoff plumbing but keeps untrusted frontend path strings out of the authority boundary and matches the existing artifact handoff.

Alternatives considered:

- **Copy `MarkdownDetail.absolutePath` directly in React:** simpler, but trusts stale frontend state and bypasses a fresh containment check.
- **Generalize artifacts and documents into one handoff command:** fewer commands, but adds an unnecessary tagged abstraction to two explicit paths.

### Reuse the existing action and notice grammar

Place a single `Copy path` button after Markdown provenance and before document content. Success uses the existing fixed handoff notice; errors use the existing detail error. The control remains keyboard accessible and respects coarse-pointer sizing and narrow layouts.

## Risks / Trade-offs

- **Generated raster details disappear at small sizes** → Select by downscaled inspection and reject concepts that depend on fine detail.
- **App and titlebar crops diverge** → Derive both from one selected square source and verify them together.
- **A document becomes non-regular or escapes containment after selection** → Resolve and canonicalize from the current index through `ContainedReader` at action time and surface the failure without copying. Safe content updates at the same path remain valid.
- **New action crowds narrow layouts** → Ship one action only and reuse responsive button rules.
- **Generated art may imply a broader brand system than exists** → Treat this as a v0 product mark, not a complete public identity.

## Migration Plan

1. Add the selected source and regenerate Tauri icon resources.
2. Update the titlebar and Markdown reader.
3. Add the backend command and frontend API contract.
4. Run focused tests, full verification, responsive screenshots, and a packaged macOS smoke test.
5. Roll back by restoring the prior icon resources and removing the Markdown handoff command; indexed data needs no migration.

## Open Questions

None. The user delegated concept selection to the implementation pass.
