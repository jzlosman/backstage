# Backstage Public Release Implementation Plan

> **REQUIRED SUB-SKILL:** Use the executing-plans skill to implement this plan task-by-task.

**Goal:** Publish Backstage as a public MIT project and release a signed, notarized universal macOS `v0.1.0` preview.

**Architecture:** GitHub pull requests and pushes run an unprivileged verification workflow. Manual and tag-triggered release runs use a protected `release` environment, import Apple credentials into temporary files and a temporary keychain, build one universal Tauri app, verify it, then upload artifacts or publish the pre-release.

**Tech Stack:** GitHub Actions, Tauri v2, Rust, React, pnpm, Apple `codesign`, `notarytool`, `stapler`, `spctl`, and GitHub CLI.

---

### Task 1: Lock the public release contract

**Files:**
- Create: `scripts/check-release-config.sh`
- Modify: `package.json`
- Test: `scripts/check-release-config.sh`

**Steps:**
1. Write a shell check that requires the MIT license, public README safety and handoff language, CI workflow, release workflow, manual/tag triggers, `release` environment, universal target, Apple secret references, pre-release publication, and release notes.
2. Run `./scripts/check-release-config.sh` and verify that it fails because the public release files do not exist.
3. Add a root package script named `check:release-config`.
4. Keep the check failing until Tasks 2–4 satisfy it.

### Task 2: Add public repository content

**Files:**
- Create: `LICENSE`
- Create: `docs/images/backstage-overview.png`
- Create: `docs/releases/v0.1.0.md`
- Modify: `README.md`
- Modify: `.gitignore`
- Modify: `PRODUCT.md`
- Modify: `docs/v1-support.md`

**Steps:**
1. Add the standard MIT license with the current copyright holder and year.
2. Capture the current branded OpenSpec preview and save one curated screenshot.
3. Rewrite the README in controlled plain English around finding, understanding, and resuming scattered planning work.
4. Document installation, read-only behavior, current preview limits, development commands, and contribution links.
5. Add concise public-preview release notes.
6. Ignore raw Impeccable exploration files and signing outputs.
7. Correct stale product and support statements that contradict current branding or All Markdown browsing.
8. Run the release contract check and confirm that only workflow requirements still fail.

### Task 3: Add unprivileged CI

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: `package.json`
- Modify: `pnpm-lock.yaml`

**Steps:**
1. Add exact `@fission-ai/openspec` development tooling so CI can run strict validation.
2. Add a macOS CI job for pushes and pull requests with read-only repository permissions.
3. Run formatting, lint, tests, type checks, release-config checks, strict OpenSpec validation, and the production build.
4. Confirm that CI contains no Apple secret references and no write permission.
5. Run local equivalents and fix failures.

### Task 4: Add the protected universal macOS release workflow

**Files:**
- Create: `.github/workflows/release.yml`
- Test: `scripts/check-release-config.sh`

**Steps:**
1. Add `workflow_dispatch` and `v*` tag triggers.
2. Bind the job to the `release` GitHub environment and grant only the content permission needed to publish a tag release.
3. Install Node, pnpm, Rust, and both macOS Rust targets.
4. Run the full verification suite before credentials are imported.
5. Decode the certificate, create a temporary keychain, import the identity, and discover the Developer ID Application name.
6. Write the App Store Connect API key to a protected temporary file.
7. Build `universal-apple-darwin` app and DMG bundles with Tauri signing and notarization variables.
8. Verify signatures, Gatekeeper, stapling, and both executable architectures.
9. Create the app ZIP and SHA-256 checksum file.
10. Upload manual-run artifacts or publish the tagged pre-release only after all checks pass.
11. Delete the keychain and temporary credential files in an unconditional cleanup step.
12. Run the release contract check and YAML formatting check.

### Task 5: Verify and review before publication

**Files:**
- Review all changed and untracked public files.

**Steps:**
1. Run frontend tests, Rust workspace tests, Clippy, formatting, lint, type checks, production build, core-boundary checks, strict OpenSpec validation, release-config checks, and `git diff --check`.
2. Run a read-only code-review pass focused on workflow security, release correctness, README accuracy, and accidental public files.
3. Fix critical and important findings.
4. Repeat the full verification suite.

### Task 6: Create and configure the public repository

**Files:**
- External: `github.com/jzlosman/backstage`

**Steps:**
1. Create the repository as public without an auto-generated README, license, or `.gitignore`.
2. Add it as `origin`.
3. Create the initial public commit from the reviewed file set.
4. Push `main` and verify the public repository and CI run.
5. Create the `release` environment with owner approval, self-review allowed, and deployment policies limited to `main` and `v*` refs.
6. Add a repository ruleset that limits `v*` tag creation to repository administrators.
7. Export the local Developer ID Application identity to a temporary password-protected PKCS #12 file.
8. Set certificate and Apple API values as `release` environment secrets without printing them.
9. Set non-secret signing metadata as environment variables.
10. Delete temporary credential files.

### Task 7: Prove the release candidate and publish `v0.1.0`

**Files:**
- External: GitHub Actions and GitHub Releases

**Steps:**
1. Run the release workflow manually.
2. Inspect failures and correct the workflow without weakening checks.
3. Download the signed and notarized workflow artifact.
4. Verify its checksums, signatures, stapling, and universal architecture locally.
5. Install the DMG build and manually check launch, root approval, planning discovery, copy path, copy continuation prompt, and repository immutability.
6. Tag the verified commit `v0.1.0` and push the tag.
7. Wait for the release workflow to finish.
8. Download the public GitHub release assets and repeat checksum, install, launch, and handoff checks.
9. Confirm that `v0.1.0` is visible as a pre-release with DMG, ZIP, and checksum assets.
