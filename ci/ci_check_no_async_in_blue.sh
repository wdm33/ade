#!/usr/bin/env bash
set -euo pipefail

# Enforce DC-CORE-01: BLUE authoritative code is sync-only.
#
# Forbidden in any .rs file under a BLUE path declared in
# .idd-config.json `core_paths`:
#   - `async fn`
#   - `.await`
#   - `tokio` (any reference)
#   - `async_std` (any reference)
#   - `futures::` (any reference)
#   - free `spawn` (filtered against `std::thread::spawn` allowlist)
#   - `tokio::time::{sleep,timeout,interval}` and analogues
#
# Comment-line filter removes `^\s*//` to suppress false positives in
# doc/contract headers. Inline string literals are NOT filtered; if a
# legitimate string contains these tokens, a follow-up syn-based
# scanner will replace this grep version.
#
# Reads BLUE paths from .idd-config.json `core_paths`. Each entry is
# resolved relative to repo root; paths ending in `/` are scanned as
# directories, file paths are scanned directly.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONFIG="$REPO_ROOT/.idd-config.json"

if [ ! -f "$CONFIG" ]; then
    echo "FAIL: .idd-config.json not found at $CONFIG"
    exit 1
fi

# Self-test mode: plant a synthetic violation under a BLUE path,
# re-invoke the scanner, confirm exit code 1, then clean up. Used by
# `bash ci/ci_check_no_async_in_blue.sh --self-test` to verify the
# enforcement mechanism is live.
if [ "${1:-}" = "--self-test" ]; then
    FIXTURE_DIR="$REPO_ROOT/crates/ade_network/src/codec/_async_in_blue_self_test"
    trap 'rm -rf "$FIXTURE_DIR"' EXIT
    mkdir -p "$FIXTURE_DIR"
    cat > "$FIXTURE_DIR/violation.rs" <<'RS'
// Core Contract: synthetic violation for ci_check_no_async_in_blue.sh self-test
async fn synthetic_self_test_violation() -> u32 {
    let _ = tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    0
}
RS
    if bash "$0" >/dev/null 2>&1; then
        echo "FAIL: scanner did not detect synthetic async violation"
        exit 1
    fi
    echo "PASS: scanner detected synthetic async violation"
    exit 0
fi

mapfile -t BLUE_PATHS < <(python3 -c "
import json, sys
with open('$CONFIG') as fh:
    cfg = json.load(fh)
for p in cfg.get('core_paths', []):
    print(p)
")

FORBIDDEN_REGEX='\basync[[:space:]]+fn\b|\.await\b|\btokio\b|\basync_std\b|\bfutures::'

FAILED=0

scan_path() {
    local target="$1"
    if [ ! -e "$target" ]; then
        echo "WARN: BLUE path missing on disk: $target" >&2
        return
    fi

    # Build the file list (file or directory).
    local files=()
    if [ -f "$target" ]; then
        files=("$target")
    else
        while IFS= read -r -d '' f; do
            files+=("$f")
        done < <(find "$target" -name '*.rs' -print0)
    fi

    for f in "${files[@]}"; do
        # Strip comment-only lines, then grep for forbidden tokens.
        local hits
        hits=$(grep -nE "$FORBIDDEN_REGEX" "$f" 2>/dev/null | \
            grep -vE '^[0-9]+:[[:space:]]*//' || true)
        if [ -n "$hits" ]; then
            echo "FAIL: async/await/tokio reference in BLUE file $f:"
            echo "$hits"
            FAILED=1
        fi

        # `spawn` is allowed only as std::thread::spawn or rayon-style;
        # forbid tokio::spawn / async_std::task::spawn / smol::spawn.
        # Look for `tokio::spawn(`, `async_std::task::spawn(`, and bare
        # `spawn(` in files that already import tokio/async_std/futures.
        hits=$(grep -nE 'tokio::spawn|async_std::task::spawn|smol::spawn' "$f" 2>/dev/null | \
            grep -vE '^[0-9]+:[[:space:]]*//' || true)
        if [ -n "$hits" ]; then
            echo "FAIL: async-runtime spawn in BLUE file $f:"
            echo "$hits"
            FAILED=1
        fi

        # Async timers/sleep — explicit module-prefixed forms.
        hits=$(grep -nE 'tokio::time::|async_std::task::sleep|futures_timer::' "$f" 2>/dev/null | \
            grep -vE '^[0-9]+:[[:space:]]*//' || true)
        if [ -n "$hits" ]; then
            echo "FAIL: async timer in BLUE file $f:"
            echo "$hits"
            FAILED=1
        fi
    done
}

for raw in "${BLUE_PATHS[@]}"; do
    # core_paths entries are repo-relative; some end with `/`.
    full="$REPO_ROOT/$raw"
    # Trim a single trailing slash so `find` and `-f` both work.
    full="${full%/}"
    scan_path "$full"
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No async/await/tokio in BLUE paths (DC-CORE-01)"
    exit 0
else
    exit 1
fi
