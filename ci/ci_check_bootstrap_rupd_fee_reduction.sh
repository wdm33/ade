#!/usr/bin/env bash
set -uo pipefail

# DC-EPOCH-23 (CE-3d bootstrap fee-buffer): the bootstrap reward update's feeSS (deltaF) is DECODED from
# the certified snapshot and the seed-boundary apply reduces the fee pot by it EXACTLY once (cardano's
# "the fee pot will be reduced by feeSS"), never leaking into a later native boundary. It is carried on a
# commitment-bound v3 codec, and schema v3 rejects a pre-fix store. Mechanical enforcement (IDD principle
# 10) of the properties the unit tests exercise but do not pin structurally.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LEDGERDB="$REPO_ROOT/crates/ade_ledger/src/ledgerdb_state.rs"
RUPD="$REPO_ROOT/crates/ade_ledger/src/bootstrap_reward_update.rs"
ACC="$REPO_ROOT/crates/ade_ledger/src/epoch_accumulator.rs"
FIRSTRUN="$REPO_ROOT/crates/ade_node/src/native_firstrun.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$LEDGERDB" "$RUPD" "$ACC" "$FIRSTRUN" "$REG"; do
  [[ -f "$f" ]] || fail "missing expected file $f"
done
[[ $FAILED -eq 0 ]] || exit 1

# (1) DECODE, DON'T SKIP: the RewardUpdate.deltaF field is READ (read_any_int), never skipped. The
#     pre-fix `skip_item(... RewardUpdate.deltaF)` is gone -- without it the feeSS is lost and the
#     seed+2 reward double-counts the seed epoch's fees.
if grep -Eq 'skip_item.*RewardUpdate\.deltaF' "$LEDGERDB"; then
  fail "(1) RewardUpdate.deltaF is still SKIPPED in the decoder -- it must be READ (read_any_int)"
fi
grep -Eq 'read_any_int.*RewardUpdate\.deltaF' "$LEDGERDB" \
  || fail "(1) the decoder does not READ RewardUpdate.deltaF via read_any_int"
grep -q 'rupd_delta_fees' "$LEDGERDB" \
  || fail "(1) decode_native_nonutxo_state does not carry rupd_delta_fees onto the decoded state"

# (2) CARRY + COMMITMENT-BIND: the bootstrap RUPD codec carries a delta_fees field and binds it into the
#     canonical commitment (encode_rupd_body), and native_firstrun threads the DECODED feeSS onto it.
grep -q 'pub delta_fees: Coin' "$RUPD" \
  || fail "(2) BootstrapRewardUpdate is missing the delta_fees field"
grep -q 'write_uint_canonical(&mut buf, delta_fees.0)' "$RUPD" \
  || fail "(2) delta_fees is not bound into the canonical commitment body (encode_rupd_body)"
grep -q 's1a.rupd_delta_fees' "$FIRSTRUN" \
  || fail "(2) native_firstrun does not thread s1a.rupd_delta_fees into the persisted BootstrapRewardUpdate"

# (3) SUBTRACT (the fix): the seed-boundary apply reduces epoch_fees by the DECODED rupd.delta_fees,
#     fail-closed on underflow. The reduction MUST use the decoded value, never a literal (see (5)).
grep -q 'checked_sub(rupd.delta_fees.0)' "$ACC" \
  || fail "(3) the seed-boundary apply does not reduce epoch_fees by the decoded rupd.delta_fees"
grep -q 'BootstrapRupdFeesUnderflow' "$ACC" \
  || fail "(3) the fee-pot reduction is not fail-closed on underflow (BootstrapRupdFeesUnderflow)"

# (4) SCHEMA v3: both the bootstrap RUPD codec and the accumulator are at v3, so a pre-fix v1/v2 store
#     fails closed (UnknownVersion) -- a fresh judge-snapshot re-bootstrap is the ONLY migration.
grep -q 'pub const BOOTSTRAP_RUPD_SCHEMA_VERSION: u32 = 3' "$RUPD" \
  || fail "(4) BOOTSTRAP_RUPD_SCHEMA_VERSION is not 3"
grep -q 'pub const EPOCH_ACCUMULATOR_SCHEMA_VERSION: u32 = 3' "$ACC" \
  || fail "(4) EPOCH_ACCUMULATOR_SCHEMA_VERSION is not 3"

# (5) NO CORRECTIVE CONSTANT: the reduction is the decoded feeSS, never the magnitude literal. The
#     confirmed value is 1157103223; test fixtures assert it only with digit separators (1_157_103_223),
#     so the underscore-free literal must appear NOWHERE in these sources (a raw corrective constant).
if grep -rq '1157103223' "$LEDGERDB" "$RUPD" "$ACC" "$FIRSTRUN"; then
  fail "(5) a raw feeSS magnitude literal (1157103223) appears in source -- the reduction must be the decoded deltaF, never a corrective constant"
fi

# (6) DC-EPOCH-23 is in the registry.
grep -q 'DC-EPOCH-23' "$REG" || fail "(6) DC-EPOCH-23 is not in the invariant registry"

[[ $FAILED -eq 0 ]] || exit 1
echo "DC-EPOCH-23 OK: deltaF decoded (not skipped), carried + commitment-bound, seed-boundary fee-pot reduction fail-closed on underflow, schema v3, no corrective constant"
