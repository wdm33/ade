#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-H S1 — AdmittedBlock single-construction-site closure.
#
# Mechanical guards for CE-N-H-1 + CN-PROTO-07:
#
#   1. The only `pub fn` whose return type is `AdmittedBlock` lives
#      at crates/ade_ledger/src/receive/admitted.rs::admit_via_block_validity
#      (returns Result<AdmittedOutcome, _> which carries AdmittedBlock).
#      Any other `pub fn .* -> .* AdmittedBlock` outside that file is a
#      regression.
#   2. `pub struct AdmittedBlock` lives only in admitted.rs.
#   3. No `pub` re-export of the tuple-struct constructor (the inner
#      field is private; we additionally forbid the constructor being
#      surfaced via `pub use ...AdmittedBlock as ...`).
#   4. The receive crate's events.rs does not constructor-export
#      locally-originated chain-sync/block-fetch messages (CN-PROTO-07).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CANONICAL_SITE="crates/ade_ledger/src/receive/admitted.rs"
EVENTS_SITE="crates/ade_ledger/src/receive/events.rs"

FAILED=0

print_fail() {
    echo "FAIL: $1"
    FAILED=1
}

if [[ ! -f "$REPO_ROOT/$CANONICAL_SITE" ]]; then
    print_fail "canonical site missing: $CANONICAL_SITE"
    exit "$FAILED"
fi

# 1. Find any pub fn whose return path mentions AdmittedBlock outside
#    the canonical site. The canonical returns `Result<AdmittedOutcome, _>`
#    which references AdmittedBlock indirectly through the outcome
#    struct's field; we therefore grep on the literal type name in
#    the signature.
mapfile -t HITS < <(
    grep -rEn 'pub fn [a-zA-Z0-9_]+\b[^{]*AdmittedBlock\b' \
        "$REPO_ROOT/crates" --include='*.rs' \
        | grep -v "/$CANONICAL_SITE:" \
        || true
)
if (( ${#HITS[@]} > 0 )); then
    print_fail "found pub fn with AdmittedBlock in signature outside $CANONICAL_SITE:"
    for h in "${HITS[@]}"; do
        echo "  $h"
    done
fi

# 2. pub struct AdmittedBlock — only in canonical site.
mapfile -t STRUCT_HITS < <(
    grep -rEn 'pub struct AdmittedBlock\b' \
        "$REPO_ROOT/crates" --include='*.rs' \
        | grep -v "/$CANONICAL_SITE:" \
        || true
)
if (( ${#STRUCT_HITS[@]} > 0 )); then
    print_fail "pub struct AdmittedBlock outside $CANONICAL_SITE:"
    for h in "${STRUCT_HITS[@]}"; do
        echo "  $h"
    done
fi

# 3. No `pub use ... AdmittedBlock as ...` rename re-exports outside
#    receive/mod.rs (which re-exports the type itself, fine — but not
#    the inner constructor).
mapfile -t RENAME_HITS < <(
    grep -rEn 'pub use .*AdmittedBlock as ' \
        "$REPO_ROOT/crates" --include='*.rs' || true
)
if (( ${#RENAME_HITS[@]} > 0 )); then
    print_fail "pub use AdmittedBlock as <alias> is forbidden:"
    for h in "${RENAME_HITS[@]}"; do
        echo "  $h"
    done
fi

# 4. Receive events.rs: no constructor for locally-originated
#    chain-sync / block-fetch messages (RequestNext, RequestRange,
#    ClientDone, FindIntersect).
if [[ -f "$REPO_ROOT/$EVENTS_SITE" ]]; then
    body=$(awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$REPO_ROOT/$EVENTS_SITE")
    if echo "$body" | grep -qE '\b(RequestNext|RequestRange|ClientDone|FindIntersect)\b'; then
        print_fail "$EVENTS_SITE references locally-originated chain-sync/block-fetch variants — CN-PROTO-07 violation"
    fi
fi

# Positive presence checks.
if ! grep -qE 'pub fn admit_via_block_validity' "$REPO_ROOT/$CANONICAL_SITE"; then
    print_fail "admit_via_block_validity missing from canonical site"
fi
if ! grep -qE 'pub struct AdmittedBlock' "$REPO_ROOT/$CANONICAL_SITE"; then
    print_fail "AdmittedBlock struct missing from canonical site"
fi

if (( FAILED == 0 )); then
    echo "OK: AdmittedBlock is single-site; receive events.rs is CN-PROTO-07-closed"
fi
exit $FAILED
