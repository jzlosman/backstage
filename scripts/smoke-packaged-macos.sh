#!/usr/bin/env bash
set -euo pipefail

app="${1:-target/release/bundle/macos/Backstage.app}"
binary="$app/Contents/MacOS/backstage-app"
log="${TMPDIR:-/tmp}/backstage-packaged-smoke.log"

[[ "$(uname -s)" == "Darwin" ]] || { echo "macOS packaged smoke only" >&2; exit 1; }
[[ -x "$binary" ]] || { echo "Packaged binary missing: $binary" >&2; exit 1; }
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app/Contents/Info.plist" >/dev/null

"$binary" >"$log" 2>&1 &
pid=$!
cleanup() { kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; }
trap cleanup EXIT
sleep 3
kill -0 "$pid"
if rg -i 'panic|fatal|uncaught|failed to initialize' "$log"; then
  echo "Packaged app logged a fatal startup error" >&2
  exit 1
fi

# The disposable fixture flow exercises the same Rust command-domain adapters used by
# the packaged binary: approval/discovery, detail/progress, bounded generation state,
# source invalidation, copies/launcher request, and repository-manifest preservation.
cargo test -p backstage-app --test vertical_smoke -- --exact disposable_real_repository_completes_the_vertical_read_only_flow

echo "Packaged macOS app launched and the real Git/OpenSpec vertical smoke flow passed."
