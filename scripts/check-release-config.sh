#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'release config check failed: %s\n' "$1" >&2
  exit 1
}

require_file() {
  [[ -f "$1" ]] || fail "missing $1"
}

require_match() {
  local file="$1"
  local pattern="$2"
  local description="$3"
  grep -Eiq -- "$pattern" "$file" || fail "$file does not declare $description"
}

require_file LICENSE
require_file README.md
require_file docs/images/backstage-overview.png
require_file docs/releases/v0.1.0.md
require_file .github/workflows/ci.yml
require_file .github/workflows/release.yml

require_match LICENSE 'MIT License' 'the MIT license'
require_match README.md 'read-only' 'the read-only safety model'
require_match README.md 'copy (its |an )?(exact )?path|copy path' 'path handoff'
require_match README.md 'continuation prompt' 'continuation-prompt handoff'
require_match README.md 'macOS 13' 'the minimum macOS version'
require_match README.md 'docs/images/backstage-overview\.png' 'the product screenshot'

require_match .github/workflows/ci.yml 'pull_request:' 'pull-request verification'
require_match .github/workflows/ci.yml 'contents: read' 'read-only repository permissions'
require_match .github/workflows/ci.yml 'components:.*rustfmt,clippy' 'Rust formatting and lint components'
if grep -Eq 'APPLE_(CERTIFICATE|API_KEY|API_ISSUER)' .github/workflows/ci.yml; then
  fail '.github/workflows/ci.yml must not reference Apple release secrets'
fi

require_match .github/workflows/release.yml 'workflow_dispatch:' 'a manual release-candidate trigger'
require_match .github/workflows/release.yml 'tags:' 'a tag trigger'
require_match .github/workflows/release.yml "['\"]?v\\*['\"]?" 'v* release tags'
require_match .github/workflows/release.yml 'environment: release' 'the protected release environment'
require_match .github/workflows/release.yml 'universal-apple-darwin' 'a universal macOS build'
require_match .github/workflows/release.yml 'components:.*rustfmt,clippy' 'Rust formatting and lint components'
require_match .github/workflows/release.yml 'APPLE_CERTIFICATE' 'the signing certificate secret'
require_match .github/workflows/release.yml 'APPLE_API_KEY_PATH' 'the notarization key path'
require_match .github/workflows/release.yml 'xcrun notarytool submit' 'explicit DMG notarization'
require_match .github/workflows/release.yml 'xcrun stapler staple' 'explicit DMG stapling'
require_match .github/workflows/release.yml "github.event_name == ['\"]push['\"]" 'tag-push-only publication'
require_match .github/workflows/release.yml 'docs/releases/\$\{GITHUB_REF_NAME\}\.md' 'version-scoped release notes'
require_match .github/workflows/release.yml '--prerelease' 'pre-release publication'
require_match .github/workflows/release.yml 'if: always\(\)' 'unconditional credential cleanup'

if git ls-files --cached --others --exclude-standard | grep -Eq '(^|/)\.impeccable/'; then
  fail 'raw Impeccable files must not be published'
fi

root_version="$(jq -r '.version' package.json)"
frontend_version="$(jq -r '.version' frontend/package.json)"
tauri_version="$(jq -r '.version' src-tauri/tauri.conf.json)"
[[ "$root_version" == "$frontend_version" && "$frontend_version" == "$tauri_version" ]] ||
  fail "package versions differ: root=$root_version frontend=$frontend_version tauri=$tauri_version"

printf 'public release contract verified for v%s\n' "$root_version"
