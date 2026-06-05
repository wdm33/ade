#!/usr/bin/env bash
set -uo pipefail

# DC-MEM-03 (PHASE4-N-E S1): tx ingress reduces to a closed IngressEvent
# before BLUE mempool admission. This gate defends that:
#
#   1. crates/ade_ledger/src/mempool/ingress.rs exists and defines
#      IngressSource (closed), IngressEvent, and mempool_ingress.
#   2. IngressSource is a closed 2-variant enum (N2N, N2C); no
#      #[non_exhaustive].
#   3. MempoolState.accumulating field is constructed/replaced only from
#      inside admit.rs. The only sanctioned production write site is the
#      `accumulating: applied` field set in admit().
#   4. admit() is called only from sanctioned production sites:
#        - crates/ade_ledger/src/mempool/admit.rs (definition + #[cfg(test)] only)
#        - crates/ade_ledger/src/mempool/ingress.rs (the bridge)
#        - crates/ade_ledger/src/producer/forge.rs (reject-only admit-prefix
#          re-validation: each call either continues or returns Err — it can
#          narrow/reject but never false-accept, so the admission chokepoint
#          invariant is preserved)
#      Any other src/ caller is a closure violation. (tests/ dirs and
#      benches/ dirs are exempt: tests legitimately drive admit directly,
#      and the gate explicitly forbids production callers, not test ones.)
#      Note: InMemorySnapshotCache::admit (ade_runtime rollback caches) is a
#      distinct 3-arg snapshot-cache method, unrelated to mempool admit; its
#      call sites are excluded by path below.
#   5. mempool_ingress's body does not branch on event.source — the
#      verdict is a function of (state, tx_bytes) alone (N-E-8).
#
# Activate (per clone): bash ci/ci_check_mempool_ingress_closure.sh

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

INGRESS="$REPO_ROOT/crates/ade_ledger/src/mempool/ingress.rs"
ADMIT="$REPO_ROOT/crates/ade_ledger/src/mempool/admit.rs"
MOD="$REPO_ROOT/crates/ade_ledger/src/mempool/mod.rs"

FAIL=0

# 1. ingress.rs exists and defines the three names; re-exports in mod.rs.
[ -f "$INGRESS" ] || { echo "FAIL: missing $INGRESS"; FAIL=1; }
[ -f "$ADMIT" ]   || { echo "FAIL: missing $ADMIT";   FAIL=1; }
[ -f "$MOD" ]     || { echo "FAIL: missing $MOD";     FAIL=1; }
[ "$FAIL" -eq 0 ] || exit 1

for sym in 'pub enum IngressSource' 'pub struct IngressEvent' 'pub fn mempool_ingress'; do
    if ! grep -qE "^${sym}" "$INGRESS"; then
        echo "FAIL: $INGRESS missing required item: ${sym}"
        FAIL=1
    fi
done

if ! grep -qE 'pub use ingress::\{.*mempool_ingress.*IngressEvent.*IngressSource' "$MOD" \
   && ! grep -qE 'pub use ingress::\{.*IngressEvent.*IngressSource.*mempool_ingress' "$MOD" \
   && ! grep -qE 'pub use ingress::\{.*mempool_ingress.*IngressSource.*IngressEvent' "$MOD" \
   && ! grep -qE 'pub use ingress::\{.*IngressSource.*IngressEvent.*mempool_ingress' "$MOD" \
   && ! grep -qE 'pub use ingress::\{.*IngressEvent.*mempool_ingress.*IngressSource' "$MOD" \
   && ! grep -qE 'pub use ingress::\{.*IngressSource.*mempool_ingress.*IngressEvent' "$MOD"; then
    echo "FAIL: $MOD does not re-export {mempool_ingress, IngressEvent, IngressSource} from ingress"
    FAIL=1
fi

# 2. IngressSource is a closed 2-variant enum; no #[non_exhaustive].
if grep -qE '#\[non_exhaustive\]' "$INGRESS"; then
    echo "FAIL: $INGRESS uses #[non_exhaustive] — IngressSource must be closed"
    FAIL=1
fi
# Variant names present.
for v in 'N2N' 'N2C'; do
    if ! grep -qE "^[[:space:]]*${v}," "$INGRESS"; then
        echo "FAIL: $INGRESS missing IngressSource variant ${v}"
        FAIL=1
    fi
done
# No extra `pub enum` lines inside ingress.rs (a single closed sum-type).
ENUM_COUNT=$(grep -cE '^pub enum ' "$INGRESS" || true)
if [ "$ENUM_COUNT" -ne 1 ]; then
    echo "FAIL: $INGRESS defines $ENUM_COUNT pub enums; expected exactly 1 (IngressSource)"
    FAIL=1
fi

# 3. MempoolState.accumulating: the only field-write of `accumulating:` must be
#    inside admit.rs (initializer and replace site). Scan all production src/
#    files for `accumulating:` field initialization; anything outside admit.rs
#    is a closure violation.
WRITES=$(grep -rnE 'accumulating:[[:space:]]*[A-Za-z]' \
    "$REPO_ROOT/crates/ade_ledger/src" \
    --include=*.rs 2>/dev/null \
    | grep -v ':[[:space:]]*//' \
    | grep -vE '/mempool/admit\.rs:' || true)
if [ -n "$WRITES" ]; then
    echo "FAIL: MempoolState.accumulating written outside mempool/admit.rs:"
    echo "$WRITES"
    FAIL=1
fi

# 4. admit() callers outside sanctioned production sites. Sanctioned:
#      - mempool/admit.rs (definition + co-located #[cfg(test)] tests)
#      - mempool/ingress.rs (the BLUE bridge)
#    Test files under crates/*/tests/ are EXEMPT (they exercise both
#    admit and mempool_ingress directly; that's intended).
CALLS=$(grep -rnE '(^|[^A-Za-z_])admit\(' \
    "$REPO_ROOT/crates" \
    --include=*.rs 2>/dev/null \
    | grep -vE '/tests/' \
    | grep -vE '/benches/' \
    | grep -vE '/mempool/admit\.rs:' \
    | grep -vE '/mempool/ingress\.rs:' \
    | grep -vE '/producer/forge\.rs:' \
    | grep -vE '/rollback/(in_memory|persistent)_cache\.rs:' \
    | grep -vE ':[[:space:]]*//' \
    | grep -vE '^[^:]*\.md:' \
    | grep -vE 'fn admit\(' \
    || true)
if [ -n "$CALLS" ]; then
    echo "FAIL: admit() called from outside sanctioned production sites (mempool/admit.rs definition + mempool/ingress.rs bridge):"
    echo "$CALLS"
    FAIL=1
fi

# 5. mempool_ingress must not branch on event.source. The body of
#    mempool_ingress is a thin pass-through; any `match` over IngressSource
#    or any read of `event.source()`/`event.source` inside the function body
#    would let the source variant change the verdict (forbidden by N-E-8).
#    Heuristic: between `pub fn mempool_ingress(` and the next top-level `}`,
#    no occurrence of `source` may appear.
BODY=$(awk '/^pub fn mempool_ingress\(/,/^}$/' "$INGRESS" || true)
if [ -n "$BODY" ] && echo "$BODY" | grep -qE '\bsource\b'; then
    echo "FAIL: mempool_ingress body references 'source' — must not branch on IngressSource (N-E-8)"
    echo "$BODY" | grep -nE '\bsource\b'
    FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: DC-MEM-03 mempool ingress closure (IngressEvent gate + admit chokepoint + source-invariant bridge)"
fi
exit "$FAIL"
