#!/usr/bin/env bash
set -euo pipefail

# Grep BLUE crate src/ dirs for forbidden nondeterministic patterns (T-CORE-02).
# Excludes comment lines and deny attribute lines.

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core")

FORBIDDEN_PATTERNS=(
    "HashMap"
    "HashSet"
    "SystemTime"
    "Instant"
    "std::fs"
    "std::net"
    "tokio"
    "async fn"
    "f32"
    "f64"
    "anyhow"
    "rand::thread_rng"
    "thread::spawn"
)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    if [ ! -d "$SRC_DIR" ]; then
        continue
    fi

    for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
        # Search .rs files, exclude comment-only lines and deny attribute lines
        # After grep -rn, lines look like "path:lineno:content" so we match // after the line number
        matches=$(grep -rn "$pattern" "$SRC_DIR" --include='*.rs' 2>/dev/null | \
            grep -v ':[0-9]*:\s*//' | \
            grep -v '#!\[deny' | \
            grep -v '#\[deny' || true)

        if [ -n "$matches" ]; then
            echo "FAIL: Forbidden pattern '$pattern' found in $crate:"
            echo "$matches"
            FAILED=1
        fi
    done
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No forbidden patterns in BLUE crates"
    exit 0
else
    exit 1
fi
