#!/usr/bin/env bash
# CE-C2 (PHASE4-N-AE.C) — recover→follow WAL prior-fp lineage continuity.
#
# The live `--mode node` ForwardSyncState prior-fp seed MUST be the fingerprint
# of the recovered ledger tip the follow extends (`fingerprint(&state.ledger)`),
# never `Hash32([0u8;32])` / zero / default(). Otherwise the FIRST followed
# AdmitBlock's WAL `prior_fp` is 0 instead of chaining from the recovered
# ledger-tip post_fp (DC-WAL-02 first-entry clause), so a recover→followed store
# fails closed on the next warm-start (ChainBreak@1 — the CE-A5 exit-42 failure)
# and recovery is NOT replay-equivalent (T-REC-05).
#
# The fix seeds the chain correctly; it does NOT loosen WAL verification — this
# gate also fences that `verify_chain` / `replay_from_anchor` still raise
# ChainBreak on a prior_fp mismatch.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LF="crates/ade_node/src/node_lifecycle.rs"
RP="crates/ade_ledger/src/wal/replay.rs"
ST="crates/ade_ledger/src/wal/store_trait.rs"

fail() { echo "FAIL ci_check_recover_follow_wal_lineage: $1" >&2; exit 1; }

[ -f "$LF" ] || fail "missing $LF"

# (1) The two known live ForwardSyncState::new sites (forge-off + forge-on relay
#     setup in run_node_lifecycle_inner).
n_new="$(grep -c 'ForwardSyncState::new(' "$LF" || true)"
[ "$n_new" -eq 2 ] || fail "expected 2 ForwardSyncState::new sites in node_lifecycle.rs, found $n_new"

# (2) Both prior-fp seeds derive from the recovered ledger fingerprint
#     (the off site via a `let anchor_fp = fingerprint(&state.ledger).combined;`
#     local, the on site inline) => the expression appears for each site.
n_fp="$(grep -c 'fingerprint(&state.ledger).combined' "$LF" || true)"
[ "$n_fp" -ge 2 ] || fail "expected >=2 'fingerprint(&state.ledger).combined' prior-fp seeds, found $n_fp"

# (3) NO live ForwardSyncState::new is seeded with a zero/default fingerprint —
#     inspect the 4 lines following each call for the bug pattern.
if awk '/ForwardSyncState::new\(/{c=4; next} c>0{print; c--}' "$LF" \
     | grep -qE 'Hash32\(\[0u8; *32\]\)|Hash32::default\(\)|Hash32\(\[0; *32\]\)'; then
  fail "a ForwardSyncState::new prior-fp seed is a zero/default fingerprint (the recover→follow WAL bug)"
fi

# (4) WAL chain verification is NOT loosened: replay_from_anchor + verify_chain
#     must still raise ChainBreak on a prior_fp mismatch (no new accept-break /
#     skip path). The fix is a correct seed, never a relaxed verifier.
grep -q 'WalError::ChainBreak' "$RP" || fail "replay.rs no longer raises ChainBreak (WAL verification weakened)"
grep -q 'WalError::ChainBreak' "$ST" || fail "store_trait.rs verify_chain no longer raises ChainBreak (WAL verification weakened)"

echo "OK ci_check_recover_follow_wal_lineage: both live ForwardSyncState prior-fp seeds = fingerprint(&state.ledger); no zero/default seed; WAL ChainBreak verification intact."
