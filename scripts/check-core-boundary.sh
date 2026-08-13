#!/usr/bin/env bash
set -euo pipefail

forbidden='tauri|rusqlite|walkdir|chrono|directories|clipboard|dialog|tokio|std::fs|std::process|std::time::SystemTime'

deps="$(cargo tree -p backstage-core --edges normal --prefix none)"
if printf '%s\n' "$deps" | grep -E "^($forbidden)( |$)"; then
  echo "backstage-core contains an adapter dependency" >&2
  exit 1
fi

if grep -R -n -E 'std::(fs|process)|SystemTime|tauri::|rusqlite::' crates/backstage-core/src; then
  echo "backstage-core contains an I/O dependency" >&2
  exit 1
fi

echo "pure-core boundary verified"
