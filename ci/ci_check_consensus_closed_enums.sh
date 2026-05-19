#!/usr/bin/env bash
set -euo pipefail

# CI: enforce that consensus error enums and event taxonomies are closed.
# Disallows `#[non_exhaustive]` plus `Other` / `Unknown` open-tail variants
# anywhere in ade_core::consensus. Strengthens DC-CONS-04 / DC-CONS-10 /
# T-DET-01 by ensuring every reject reason is a structured value.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/crates/ade_core/src/consensus"

if [ ! -d "$TARGET" ]; then
    echo "FAIL: $TARGET does not exist."
    exit 1
fi

FAIL=0

# 1. No #[non_exhaustive] attribute. Must be at line start (allowing
#    leading whitespace) and not commented out.
if grep -RnE '^[[:space:]]*#\[non_exhaustive\]' "$TARGET" \
        > /tmp/ade_non_exhaustive_hits 2>/dev/null; then
    echo "FAIL: #[non_exhaustive] in ade_core::consensus (enums must stay closed):"
    cat /tmp/ade_non_exhaustive_hits
    FAIL=1
fi

# 2. No open-tail `Other` / `Unknown` variant in error enums or events.
#    Match lines like `Other,` `Other {` `Other(` `Unknown,` etc. that look
#    like enum variant declarations (indented, followed by punctuation).
if grep -RnE '^[[:space:]]+(Other|Unknown)([[:space:]]*[{(,])' \
        "$TARGET" > /tmp/ade_open_tail_hits 2>/dev/null; then
    echo "FAIL: open-tail Other/Unknown variant in ade_core::consensus:"
    cat /tmp/ade_open_tail_hits
    FAIL=1
fi

# 3. No String fields in error enums. The error files must stay flat-data.
#    Scope: only inspect errors.rs + encoding.rs DecodeError + events.rs.
for f in "$TARGET/errors.rs" "$TARGET/encoding.rs" "$TARGET/events.rs"; do
    if [ -f "$f" ]; then
        # Allow `&'static str` (used by DecodeError::Cbor) but reject
        # owned `String`. Filter out comment lines.
        if grep -nE '^[^/]*\bString\b' "$f" \
                | grep -vE '"[^"]*"' \
                | grep -vE '\&.*str' > /tmp/ade_string_hits 2>/dev/null; then
            if [ -s /tmp/ade_string_hits ]; then
                echo "FAIL: owned String in $f:"
                cat /tmp/ade_string_hits
                FAIL=1
            fi
        fi
    fi
done

# 4. No Box<dyn ...> usage. Exclude comment / doc lines.
if grep -RnE 'Box<dyn\b' "$TARGET" | grep -vE '^[^:]*:[[:space:]]*[0-9]+:[[:space:]]*(//|///)' \
        > /tmp/ade_box_dyn_hits 2>/dev/null; then
    if [ -s /tmp/ade_box_dyn_hits ]; then
        echo "FAIL: Box<dyn ...> in ade_core::consensus (must use flat error data):"
        cat /tmp/ade_box_dyn_hits
        FAIL=1
    fi
fi

if [ "$FAIL" -ne 0 ]; then
    exit 1
fi

echo "OK: ade_core::consensus error / event enums are closed"
