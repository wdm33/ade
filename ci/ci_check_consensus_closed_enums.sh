#!/usr/bin/env bash
set -euo pipefail

# CI: enforce that consensus error enums and event taxonomies are closed.
# Disallows `#[non_exhaustive]` plus `Other` / `Unknown` open-tail variants
# anywhere in ade_core::consensus, ade_ledger::block_validity, AND
# ade_ledger::tx_validity. Strengthens DC-CONS-04 / DC-CONS-10 / T-DET-01
# (consensus) and DC-VAL-02/04/05/06 (block-validity) and DC-TXV-01/02/04/05 /
# DC-VAL-06 (tx-validity: SignerSource / RequiredSignerError /
# WitnessClosureError / TxValidityVerdict / TxRejectClass / TxValidityError
# must stay closed structured values) by ensuring every reject reason, signer
# source, and tx verdict class is a structured value.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGETS=(
    "$REPO_ROOT/crates/ade_core/src/consensus"
    "$REPO_ROOT/crates/ade_ledger/src/block_validity"
    "$REPO_ROOT/crates/ade_ledger/src/tx_validity"
)

# Files whose enums must stay flat-data (no owned String). `&'static str`
# (DecodeError::Cbor / SurfaceDecodeError::Cbor) is permitted.
STRING_SCOPE=(
    "$REPO_ROOT/crates/ade_core/src/consensus/errors.rs"
    "$REPO_ROOT/crates/ade_core/src/consensus/encoding.rs"
    "$REPO_ROOT/crates/ade_core/src/consensus/events.rs"
    "$REPO_ROOT/crates/ade_ledger/src/block_validity/verdict.rs"
    "$REPO_ROOT/crates/ade_ledger/src/block_validity/encoding.rs"
    "$REPO_ROOT/crates/ade_ledger/src/tx_validity/required_signers.rs"
    "$REPO_ROOT/crates/ade_ledger/src/tx_validity/witness.rs"
    "$REPO_ROOT/crates/ade_ledger/src/tx_validity/verdict.rs"
    "$REPO_ROOT/crates/ade_ledger/src/tx_validity/phase1.rs"
    "$REPO_ROOT/crates/ade_ledger/src/tx_validity/transition.rs"
)

FAIL=0

for TARGET in "${TARGETS[@]}"; do
    if [ ! -d "$TARGET" ]; then
        echo "FAIL: $TARGET does not exist."
        FAIL=1
        continue
    fi

    # 1. No #[non_exhaustive] attribute. Must be at line start (allowing
    #    leading whitespace) and not commented out.
    if grep -RnE '^[[:space:]]*#\[non_exhaustive\]' "$TARGET" \
            > /tmp/ade_non_exhaustive_hits 2>/dev/null; then
        echo "FAIL: #[non_exhaustive] in $TARGET (enums must stay closed):"
        cat /tmp/ade_non_exhaustive_hits
        FAIL=1
    fi

    # 2. No open-tail `Other` / `Unknown` variant in error enums or events.
    #    Match lines like `Other,` `Other {` `Other(` `Unknown,` etc. that look
    #    like enum variant declarations (indented, followed by punctuation).
    if grep -RnE '^[[:space:]]+(Other|Unknown)([[:space:]]*[{(,])' \
            "$TARGET" > /tmp/ade_open_tail_hits 2>/dev/null; then
        echo "FAIL: open-tail Other/Unknown variant in $TARGET:"
        cat /tmp/ade_open_tail_hits
        FAIL=1
    fi

    # 4. No Box<dyn ...> usage. Exclude comment / doc lines.
    if grep -RnE 'Box<dyn\b' "$TARGET" \
            | grep -vE '^[^:]*:[[:space:]]*[0-9]+:[[:space:]]*(//|///)' \
            > /tmp/ade_box_dyn_hits 2>/dev/null; then
        if [ -s /tmp/ade_box_dyn_hits ]; then
            echo "FAIL: Box<dyn ...> in $TARGET (must use flat error data):"
            cat /tmp/ade_box_dyn_hits
            FAIL=1
        fi
    fi
done

# 3. No owned String fields in error enums. The error files must stay flat-data.
for f in "${STRING_SCOPE[@]}"; do
    if [ -f "$f" ]; then
        # Allow `&'static str` (used by *DecodeError::Cbor) but reject
        # owned `String`. Filter out comment lines and string literals.
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

if [ "$FAIL" -ne 0 ]; then
    exit 1
fi

echo "OK: ade_core::consensus + ade_ledger::block_validity + ade_ledger::tx_validity error / event / signer enums are closed"
