## 1. Index the Markdown manifest

- [x] 1.1 Add failing core/catalog tests for stable Markdown identity, case-insensitive `.md` discovery, planning-classification separation, bundle-member uniqueness, and project-relative ordering
- [x] 1.2 Add the standalone Markdown document model and collect its metadata during the existing bounded project walk without weakening exclusions or containment
- [x] 1.3 Add failing persistence coverage for document round-trips and legacy snapshots without the additive document collection, then implement compatible serialization

## 2. Read standalone Markdown safely

- [x] 2.1 Add failing catalog and vertical tests for indexed document lookup, exact contained UTF-8 reads, missing documents, source changes, and escaping paths
- [x] 2.2 Implement a document-specific detail model and `get_markdown_detail` Tauri command that never constructs bundle progress, generated views, or handoffs

## 3. Add scope-aware registry navigation

- [x] 3.1 Add frontend API types and preview fixtures for Markdown manifests and standalone detail reads
- [x] 3.2 Add failing frontend tests proving `Plan files` is the default and `All Markdown` reveals Markdown-only projects, counts unique files, avoids bundle-member duplicates, and searches both filenames and paths
- [x] 3.3 Implement deterministic scope-aware project, count, ledger, search, and planning-state-filter derivation

## 4. Integrate the generic reading desk

- [x] 4.1 Add failing frontend tests for selecting ordinary Markdown, using the generic reader, switching back to planning scope, and ignoring stale document or bundle responses
- [x] 4.2 Implement shared bundle/document selection sequencing, scope-safe selection fallback, visible read errors, and local-only generic Markdown presentation
- [x] 4.3 Add an accessible `Plan files / All Markdown` scope control, distinct document rows, responsive styling, and scope-specific empty-state copy
- [x] 4.4 Add scale coverage and bounded incremental ledger rendering while preserving full counts and search reachability

## 5. Verify the completed slice

- [x] 5.1 Run focused and full frontend/Rust tests, typecheck, lint, formatting, Clippy, production build, strict OpenSpec validation, and the pure-core boundary check
- [x] 5.2 Obtain bounded code and interface reviews, resolve material findings, and verify all requirements against the implementation
- [x] 5.3 Build and smoke-test the packaged macOS app, toggle a live registry to `All Markdown`, open an ordinary file, and confirm repository immutability and path containment
