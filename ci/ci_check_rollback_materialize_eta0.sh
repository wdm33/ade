#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AN (T-REC-06): rollback materialization preserves the recovered eta0.
# materialize_rolled_back_state overlays the recovered seed-epoch eta0 onto the
# nearest-snapshot chain_dep BEFORE the replay-forward fold, so rollback replay
# validates each block's header VRF against eta0 — the SAME nonce live admit used —
# NOT the persisted snapshot's Nonce::ZERO placeholder (replay-equivalence). This
# guards:
#   (A) the SINGLE eta0-overlay authority exists on PraosChainDepState.
#   (B) materialize takes the recovered_eta0 param AND applies the overlay.
#   (C) NO VRF bypass: the replay still runs block_validity; no skip/unchecked.
#   (D) eta0 is sourced from the recovered sidecar (apply_chain_event via
#       ForwardSyncState.recovered_eta0; bootstrap via the seed-epoch sidecar) —
#       never peer/CLI/wall-clock.
#   (E) the replay-equivalence + no-bypass regression tests exist.
#   (F) T-REC-06 is enforced in the registry.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PRAOS="$REPO_ROOT/crates/ade_core/src/consensus/praos_state.rs"
MAT="$REPO_ROOT/crates/ade_ledger/src/rollback/materialize.rs"
NODE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
BOOT="$REPO_ROOT/crates/ade_runtime/src/bootstrap.rs"
FWD="$REPO_ROOT/crates/ade_runtime/src/forward_sync/reducer.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$PRAOS" "$MAT" "$NODE" "$BOOT" "$FWD" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done

# (A) the SINGLE eta0-overlay authority on PraosChainDepState.
grep -Eq 'pub fn overlay_recovered_eta0' "$PRAOS" \
    || print_fail "(A) PraosChainDepState::overlay_recovered_eta0 (the shared overlay authority) is missing"

# (B) materialize takes the recovered_eta0 param AND applies the overlay before replay.
grep -Eq 'recovered_eta0: Option<&Nonce>' "$MAT" \
    || print_fail "(B) materialize_rolled_back_state does not take the recovered_eta0 param"
grep -Eq 'overlay_recovered_eta0\(eta0\)' "$MAT" \
    || print_fail "(B) materialize_rolled_back_state does not apply the eta0 overlay"

# (C) NO VRF bypass: the replay still validates via block_validity; no skip/unchecked.
grep -Eq 'block_validity\(' "$MAT" \
    || print_fail "(C) materialize replay no longer runs block_validity (VRF must stay verified)"
# Real bypass CODE only — exclude the no-bypass regression test name + comments
# (both legitimately contain the substring "bypass_vrf"/"VRF").
if grep -Ei 'skip_vrf|unchecked_vrf|bypass_vrf|no_vrf' "$MAT" \
    | grep -vqE 'fn rollback_materialize_does_not_bypass_vrf|^\s*//|^\s*///'; then
    print_fail "(C) materialize must NOT bypass/skip VRF — the overlay supplies the correct nonce, not a skip"
fi

# (D) eta0 sourced from the recovered sidecar — never peer/CLI/wall-clock.
grep -Eq 'fwd\.recovered_eta0\.as_ref\(\)' "$NODE" \
    || print_fail "(D) apply_chain_event does not pass fwd.recovered_eta0 into materialize"
grep -Eq 'recovered_eta0 = state\s*$|recovered_eta0 = state\.|\.seed_epoch_consensus_inputs' "$NODE" \
    || print_fail "(D) ForwardSyncState.recovered_eta0 is not set from the recovered seed-epoch sidecar"
grep -Eq 'seed_epoch_consensus_inputs\.as_ref\(\)\.map\(\|s\| &s\.epoch_nonce\)' "$BOOT" \
    || print_fail "(D) bootstrap does not source the materialize eta0 from the seed-epoch sidecar"
grep -Eq 'pub recovered_eta0: Option<Nonce>' "$FWD" \
    || print_fail "(D) ForwardSyncState lacks the recovered_eta0 carrier field"

# (E) the regression tests exist.
grep -Eq 'fn rollback_materialize_overlays_recovered_eta0_replay_equivalent' "$MAT" \
    || print_fail "(E) the replay-equivalence regression test is missing"
grep -Eq 'fn rollback_materialize_does_not_bypass_vrf_on_wrong_eta0' "$MAT" \
    || print_fail "(E) the no-VRF-bypass regression test is missing"

# (F) T-REC-06 enforced in the registry.
awk '/^id = "T-REC-06"/{f=1} f&&/^status = /{print; exit}' "$REG" | grep -q 'enforced' \
    || print_fail "(F) T-REC-06 is not enforced in the registry"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_rollback_materialize_eta0: FAILED"
    exit 1
fi
echo "ci_check_rollback_materialize_eta0: OK (T-REC-06 — rollback materialize overlays the recovered eta0; replay-equivalent with live admit; no VRF bypass)"
