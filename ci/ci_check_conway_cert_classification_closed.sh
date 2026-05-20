#!/usr/bin/env bash
set -uo pipefail

# DC-TXV-06: the Conway certificate-deposit classification is a closed, total,
# era-versioned function. This gate defends that closure against regression so
# the rule is CI-enforced, not only compiler-exhaustive-match + tests:
#
#   1. The classification value types stay CLOSED — no #[non_exhaustive] and no
#      open-tail `Other`/`Unknown` variant on ConwayCert / CertDisposition /
#      DepositEffect / CoinSource (crates/ade_types/src/conway/cert.rs).
#   2. The decoder REJECTS unknown cert tags and has NO catch-all accept arm —
#      `CodecError::UnknownCertTag` is present and no `_ =>` arm constructs a
#      `ConwayCert` (the reintroduced-Shelley-fallback anti-pattern)
#      (crates/ade_codec/src/conway/cert.rs).
#   3. `classify` stays exhaustive — no `_ =>` wildcard arm, so adding a new
#      ConwayCert variant breaks the build instead of silently classifying it
#      (crates/ade_ledger/src/cert_classify.rs).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

TYPES="$REPO_ROOT/crates/ade_types/src/conway/cert.rs"
DECODER="$REPO_ROOT/crates/ade_codec/src/conway/cert.rs"
CLASSIFY="$REPO_ROOT/crates/ade_ledger/src/cert_classify.rs"

FAIL=0

for f in "$TYPES" "$DECODER" "$CLASSIFY"; do
    if [ ! -f "$f" ]; then
        echo "FAIL: expected cert-surface file missing: $f"
        FAIL=1
    fi
done
[ "$FAIL" -eq 0 ] || exit 1

# Drop comment-only lines so a docstring describing a prohibition does not trip
# the gate. Input is plain file text (not grep -n output).
strip_comments() { grep -vE '^[[:space:]]*//' || true; }

# --- 1. Closed classification value types -----------------------------------

m=$(strip_comments < "$TYPES" | grep -nE '#\[non_exhaustive\]' || true)
if [ -n "$m" ]; then
    echo "FAIL: #[non_exhaustive] on a Conway cert classification type (must stay closed):"
    echo "$m"
    FAIL=1
fi

# Open-tail variant: a line that is a variant declaration named Other/Unknown.
m=$(strip_comments < "$TYPES" | grep -nE '^[[:space:]]+(Other|Unknown)[[:space:]]*[{(,]' || true)
if [ -n "$m" ]; then
    echo "FAIL: open-tail (Other/Unknown) variant in the Conway cert classification types:"
    echo "$m"
    FAIL=1
fi

# --- 2. Decoder rejects unknown tags, no catch-all accept -------------------

if ! grep -qE 'CodecError::UnknownCertTag' "$DECODER"; then
    echo "FAIL: $DECODER no longer rejects unknown cert tags (CodecError::UnknownCertTag absent)."
    FAIL=1
fi

# 2a. Single-line catch-all that yields a cert: `_ => [Ok(|Some(]ConwayCert::`
m=$(strip_comments < "$DECODER" \
    | grep -nE '_[[:space:]]*=>[[:space:]]*(Ok\(|Some\()?ConwayCert::' || true)
if [ -n "$m" ]; then
    echo "FAIL: catch-all decoder arm constructs a ConwayCert (Shelley-fallback anti-pattern):"
    echo "$m"
    FAIL=1
fi

# 2b. Block-form catch-all: any `_ => {` arm whose body mentions ConwayCert::.
#     Scan from each `_ =>` that opens a brace until brace depth returns to 0.
m=$(awk '
    /_[[:space:]]*=>/ && /{/ && !/^[[:space:]]*\/\// {
        inarm = 1; depth = 0; body = ""; startln = NR;
    }
    inarm {
        body = body "\n" $0;
        o = gsub(/{/, "{"); c = gsub(/}/, "}");
        depth += o - c;
        if (depth <= 0) {
            if (body ~ /ConwayCert::/) print startln ": catch-all arm body constructs ConwayCert";
            inarm = 0;
        }
    }
' "$DECODER" || true)
if [ -n "$m" ]; then
    echo "FAIL: block-form catch-all decoder arm constructs a ConwayCert (Shelley-fallback anti-pattern):"
    echo "$m"
    FAIL=1
fi

# --- 3. classify stays exhaustive (no wildcard arm) -------------------------

m=$(strip_comments < "$CLASSIFY" | grep -nE '_[[:space:]]*=>' || true)
if [ -n "$m" ]; then
    echo "FAIL: '_ =>' wildcard in cert_classify.rs — classify must stay compiler-exhaustive over ConwayCert:"
    echo "$m"
    FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: Conway cert classification is closed and total (DC-TXV-06)"
    exit 0
else
    exit 1
fi
