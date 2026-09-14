#!/usr/bin/env bash
# Run the Bombadil property tests against the built web bundle, one level at
# a time. Usage:
#   tests/bombadil/run.sh [time-limit] [level ...]
# Requires: `bombadil` on PATH (or BOMBADIL=/path), a Chrome/Chromium
# (CHROME=/path, e.g. a wrapper adding SwiftShader flags for GPU-less CI),
# and a built `dist/` (trunk build --cargo-profile wasm-release).
set -euo pipefail
cd "$(dirname "$0")/../.."
LIMIT="${1:-2m}"; shift || true
LEVELS=("$@"); [ ${#LEVELS[@]} -gt 0 ] || LEVELS=(levels/template.ldtk levels/first-steps.ldtk levels/example_world.ldtk)
BOMBADIL="${BOMBADIL:-bombadil}"
OUT="${BOMBADIL_OUT:-target/bombadil}"
PORT="${PORT:-8791}"

python3 -m http.server "$PORT" --directory dist >/dev/null 2>&1 &
SERVER=$!
trap 'kill $SERVER 2>/dev/null || true' EXIT
sleep 1

status=0
for level in "${LEVELS[@]}"; do
  name="$(basename "$level" .ldtk)"
  echo "=== $level (limit $LIMIT) -> $OUT/$name"
  rm -rf "$OUT/$name"
  "$BOMBADIL" browser test --headless --no-sandbox --time-limit="$LIMIT" \
    --output-path "$OUT/$name" \
    "http://127.0.0.1:$PORT/?level=$level" tests/bombadil/flezzle.spec.ts \
    | grep -vE '^\s*[0-9:.]+ (holdKey|focusCanvas|PressKey)' || status=$?
done
exit $status
