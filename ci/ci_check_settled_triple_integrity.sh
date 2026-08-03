#!/usr/bin/env bash
set -uo pipefail

# LIVE-REFOLD-THRASH RF-1 / CE-RF-6 -- settled-triple integrity enforcement.
#
# Gates DC-EPOCH-34. Today the recovery path always re-derives the accumulator from the
# Mithril-certified baseline, so a silently corrupted settled blob is self-healed by recomputation.
# RF-1 removes that accidental self-heal on the bounded path: recovery will RESTORE from the settled
# triple instead. `decode_epoch_accumulator` fails closed on malformed bytes, but a flipped bit
# inside an otherwise valid numeric field decodes cleanly and would be trusted and folded forward
# from. This gate keeps the triple PROVEN rather than trusted by convention.
#
# It asserts STRUCTURE, not just that tests exist: dropping the verification, dropping the write,
# grandfathering an unfingerprinted triple, forgetting to clear the fingerprint with its triple, or
# removing the length-prefixing each fail here even if everything still compiles.
#
# NB (here-string discipline): never `echo "$BIGVAR" | grep -q` under pipefail -- grep exits on first
# match, echo takes SIGPIPE, and the gate flakes on large files. Grep files directly, or `<<<`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
ok()   { echo "  ok: $1"; }

STORE=crates/ade_runtime/src/chaindb/epoch_accumulator_store.rs
[ -f "$STORE" ] || { fail "missing $STORE"; echo "RESULT: FAIL"; exit 1; }

echo "== DC-EPOCH-34: the fingerprint binds the whole triple, unambiguously =="

FP="$(awk '/^fn settled_fingerprint/,/^}/' "$STORE")"
[ -n "$FP" ] || fail "settled_fingerprint missing -- the triple would be trusted unverified"

grep -qE 'SETTLED_FP_DOMAIN' <<< "$FP" \
    || fail "the preimage must be DOMAIN-SEPARATED (a bare concatenation can collide with other digests)"
# Length-prefixing: without it, ("ab","c") and ("a","bc") share a preimage and the binding is a lie.
grep -qE 'len\(\) as u64\)\.to_be_bytes\(\)' <<< "$FP" \
    || fail "each member must be LENGTH-PREFIXED, or two different triples can share a preimage"
for part in seed_slot leadership_schema point blob leadership; do
    grep -qE "\b${part}\b" <<< "$FP" || fail "the fingerprint must bind ${part}"
done
ok "domain-separated, length-prefixed, binds seed/schema/point/blob/leadership"

# A binding change without a version bump would silently re-interpret old fingerprints.
grep -qE 'SETTLED_FP_DOMAIN: &\[u8\] = b"ade-settled-triple-v[0-9]+"' "$STORE" \
    || fail "SETTLED_FP_DOMAIN must carry an explicit version so a binding change fails closed"
ok "domain tag is versioned"

echo "== DC-EPOCH-34: written on promote, VERIFIED on restore =="

ROLL="$(awk '/pub fn roll_settled_rewind_point/,/^    }/' "$STORE")"
grep -qE 'settled_fingerprint\(' <<< "$ROLL" \
    || fail "promotion must COMPUTE the fingerprint"
grep -qE 'insert\(SETTLED_FP_KEY' <<< "$ROLL" \
    || fail "promotion must WRITE the fingerprint in the same commit as the triple"
ok "fingerprint written when the triple is promoted"

SETTLED="$(awk '/pub fn reset_to_settled/,/^    }/' "$STORE")"
grep -qE 'get\(SETTLED_FP_KEY' <<< "$SETTLED" \
    || fail "restore must READ the stored fingerprint"
grep -qE 'settled_fingerprint\(' <<< "$SETTLED" \
    || fail "restore must RECOMPUTE the fingerprint -- reading it alone verifies nothing"
# The comparison and both fail-closed exits must survive.
grep -qE 'return Ok\(false\)' <<< "$SETTLED" \
    || fail "restore must fail CLOSED to Ok(false) so the caller falls back to reset_to_bootstrap"
ok "restore reads + recomputes + falls back on failure"

# An unfingerprinted (pre-slice) triple must be refused, never grandfathered. The binding MUST be
# let-else, which forces the absent case to diverge. A permissive `if let Some(stored_fp)` would
# verify when present and silently FALL THROUGH TO RESTORING when absent -- the exact regression a
# name-only grep misses, so assert the shape, not the identifier.
grep -qE 'let Some\(stored_fp\)[^;]*=[^;]*$' <<< "$SETTLED" \
    || fail "an ABSENT fingerprint must be refused (no grandfathering of unverifiable triples)"
grep -qE '^[[:space:]]*(\}[[:space:]]*)?else[[:space:]]*\{' <<< "$SETTLED" \
    || fail "the absent-fingerprint binding must be let-ELSE so the absent case cannot fall through"
grep -qE 'if let Some\(stored_fp\)' <<< "$SETTLED" \
    && fail "permissive 'if let Some(stored_fp)' verifies only when present and restores when absent -- must be let-else"
ok "absent fingerprint is refused (let-else, cannot fall through)"

echo "== DC-EPOCH-34: the fingerprint dies with the triple it binds =="

BOOT="$(awk '/pub fn reset_to_bootstrap/,/^    }/' "$STORE")"
grep -qE 'remove\(SETTLED_FP_KEY' <<< "$BOOT" \
    || fail "reset_to_bootstrap must clear the fingerprint with the triple (a stale fingerprint could validate a later torn promotion)"
ok "bootstrap reset clears the fingerprint"

echo "== CE-RF-6: proof tests present =="

check_test() { grep -qE "fn ${1}\b" "$STORE" || fail "missing proof test: ${1}"; }
check_test a_corrupted_settled_triple_is_refused_and_falls_back
check_test a_settled_triple_with_no_fingerprint_is_refused
check_test the_settled_fingerprint_binds_every_input
[ "$FAILED" -eq 0 ] && ok "all 3 named proof tests present"

if [ "$FAILED" -ne 0 ]; then
    echo "RESULT: FAIL (settled-triple integrity regressed)"
    exit 1
fi
echo "RESULT: PASS (DC-EPOCH-34 structurally enforced + proofs present)"
