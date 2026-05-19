#!/usr/bin/env bash
set -euo pipefail

# CI: enforce that no density-based ordering term creeps into the
# Praos fork-choice surface. Caught-up Praos chain selection uses
# (BlockNo, TiebreakerView) only — density is reserved for
# Genesis/catch-up logic and forbidden here. Strengthens DC-CONS-03.
#
# Allowed exception: a comment line whose first non-blank prefix is
# `// no-density:` may mention the word "density" (used to annotate
# the explicit forbid line and audit markers in the module docstring).
# Every other case-insensitive occurrence of `density` triggers a
# failure.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGETS=(
    "$REPO_ROOT/crates/ade_core/src/consensus/fork_choice.rs"
    "$REPO_ROOT/crates/ade_core/src/consensus/candidate.rs"
)

FAIL=0

for f in "${TARGETS[@]}"; do
    if [ ! -f "$f" ]; then
        echo "FAIL: $f does not exist."
        FAIL=1
        continue
    fi
    # Case-insensitive grep with line numbers, then filter to lines
    # whose first non-blank content is NOT `// no-density:`.
    if grep -niE 'density' "$f" \
            | grep -vE '^[0-9]+:[[:space:]]*//[[:space:]]*no-density:' \
            > /tmp/ade_density_hits 2>/dev/null; then
        if [ -s /tmp/ade_density_hits ]; then
            echo "FAIL: forbidden 'density' reference in $f:"
            cat /tmp/ade_density_hits
            FAIL=1
        fi
    fi
done

if [ "$FAIL" -ne 0 ]; then
    exit 1
fi

echo "OK: no density-based ordering in ade_core::consensus fork-choice / candidate"
