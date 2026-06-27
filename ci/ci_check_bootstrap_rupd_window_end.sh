#!/usr/bin/env bash
# DC-EPOCH-18: the bootstrap reward update (the seed+2 authority's window-end reward distribution) is
# applied as a PURE BLUE transition at the WINDOW-END, fails closed at the SINGLE derive_candidate site,
# and is NEVER mutated into the durable seed cert-state. Mechanical enforcement (IDD principle 10) of the
# Option-B temporal model that replaced the rejected apply-at-bootstrap approach (whose mutated pseudo-seed
# was correct only by accident -- a seed-window-tail reward-withdrawer would have received 0).
set -euo pipefail
cd "$(dirname "$0")/.."

fail() { echo "DC-EPOCH-18 VIOLATION: $1" >&2; exit 1; }

DRIVER=crates/ade_runtime/src/chaindb/reduced_window_driver.rs
CAND=crates/ade_node/src/epoch_candidate.rs
FIRSTRUN=crates/ade_node/src/native_firstrun.rs
DELEG=crates/ade_ledger/src/delegation.rs
CODEC=crates/ade_ledger/src/bootstrap_reward_update.rs

# (1) FC/IS: the reward apply is a PURE BLUE fn the RED driver CALLS -- the driver must NOT mutate core
#     state (delegation.rewards) inline.
grep -q "pub fn apply_bootstrap_reward_deltas" "$DELEG" \
  || fail "the pure BLUE apply_bootstrap_reward_deltas is missing from delegation.rs"
grep -q "apply_bootstrap_reward_deltas" "$DRIVER" \
  || fail "the window driver does not call the BLUE apply_bootstrap_reward_deltas"
# Only the PRODUCTION code (before #[cfg(test)]) -- tests legitimately construct delegation.rewards.
if awk '/#\[cfg\(test\)\]/{exit} 1' "$DRIVER" | grep -qE "delegation\.rewards\.(entry|insert)"; then
  fail "the RED window driver mutates delegation.rewards inline (FC/IS: the shell must not modify core state)"
fi

# (2) The seed+2 fail-closed is MECHANICAL at the single derive_candidate site (no caller can drift).
grep -q "BootstrapRewardUpdateAbsent" "$CAND" \
  || fail "derive_candidate is missing the absent-rupd fail-closed (BootstrapRewardUpdateAbsent)"
grep -q "BootstrapRewardUpdateEpochMismatch" "$CAND" \
  || fail "derive_candidate is missing the wrong-epoch fail-closed (BootstrapRewardUpdateEpochMismatch)"

# (3) NO apply-at-bootstrap: native_firstrun MUST NOT mutate the durable seed cert-state rewards.
if awk '/#\[cfg\(test\)\]/{exit} 1' "$FIRSTRUN" | grep -qE "cert_state\.delegation\.rewards\.(entry|insert)"; then
  fail "native_firstrun mutates the seed cert-state rewards (the apply-at-bootstrap pseudo-state must stay reverted)"
fi

# (4) The closed, version-gated, commitment-bound codec is the sole encode/decode surface.
test -f "$CODEC" || fail "the bootstrap_reward_update codec is missing"
grep -q "pub const BOOTSTRAP_RUPD_SCHEMA_VERSION" "$CODEC" \
  || fail "the bootstrap reward-update codec is not version-gated"

echo "DC-EPOCH-18 OK: window-end BLUE apply (no inline core mutation), single-site fail-closed, no apply-at-bootstrap, closed version-gated codec"
