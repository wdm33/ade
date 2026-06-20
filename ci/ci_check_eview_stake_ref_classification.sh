#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONSENSUS-VIEW Slice 2 (DC-EVIEW-02): typed, era-gated stake-reference
# classification. The classifier extracts the per-output stake reference ONLY --
# one deterministic typed result (Base | Pointer | Null | Reject) from canonical
# address bytes + a TYPED era context BOUND to the block. No fixed byte offset is
# the contract across variants/eras (it routes through the typed decode chokepoint
# + per-form structural validation); malformed stays DISTINCT from Null; pointer
# stake is era-gated (retired at Conway). It resolves nothing, sums nothing, and
# no result changes stake totals (aggregation is Slice 3).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
M=crates/ade_ledger/src/stake_ref.rs

test -f "$M" || fail "the stake_ref classifier ($M) is missing"

# (1) the one typed 4-way result.
grep -qE 'pub enum StakeRefClass' "$M" || fail "StakeRefClass result enum missing"
for v in 'Base\(StakeCredential\)' 'Pointer\(PointerRef\)' 'Null' 'Reject\(StakeRefReject\)'; do
    grep -qE "$v" "$M" || fail "StakeRefClass is missing the $v variant"
done

# (2) era/PV authority is a TYPED bound context input -- the classifier takes a
#     CardanoEra parameter, NOT ambient config / wall-clock / a caller flag.
grep -qE 'pub fn classify_output_stake_ref\(addr_bytes: &\[u8\], era: CardanoEra\)' "$M" \
    || fail "classify_output_stake_ref does not take (addr_bytes, era: CardanoEra) -- era must be a typed bound input"

# (3) typed-decode-only: classification routes through the decode chokepoint, not
#     a blanket byte-offset attribution shortcut.
grep -qF 'decode_address(addr_bytes)' "$M" \
    || fail "classification does not route through the typed decode_address chokepoint"

# (4) era-gated pointer retirement keyed on the bound era (Conway+ -> Null).
grep -qE 'era >= CardanoEra::Conway' "$M" \
    || fail "pointer retirement is not gated on era >= Conway"

# (5) base length is structurally validated before the staking part is read (no
#     unchecked [29..57] as THE contract).
grep -qE 'b\.len\(\) != 57' "$M" || fail "base address length is not validated to 57 before extraction"

# (6) reward addresses are fail-closed (never ordinary output stake).
grep -qE 'RewardAddressNotValidAsOutput' "$M" \
    || fail "reward addresses are not fail-closed as invalid output stake"

# (7) the load-bearing proofs.
grep -qE 'fn pointer_is_decoded_pre_conway_and_retired_at_conway' "$M" \
    || fail "the era-gated pointer-retirement test is missing"
grep -qE 'fn malformed_but_prefix_valid_base_is_reject_not_null' "$M" \
    || fail "the malformed-distinct-from-Null test is missing"
grep -qE 'fn real_preview_addresses_classify_without_reject' "$M" \
    || fail "the real-preview-address interop test is missing"
grep -qE 'fn reward_address_is_rejected_not_summed' "$M" \
    || fail "the reward-fail-closed test is missing"

# (8) Slice-2 boundary: NO aggregation / resolution / EpochConsensusView here.
if grep -qiE 'pool_distribution|EpochConsensusView|resolve_pointer|sum.*stake|aggregate' "$M"; then
    fail "the classifier reaches into aggregation/resolution -- that is Slice 3"
fi
# the classifier does not touch the ledger track_utxo flag (it is a pure classifier).
if grep -qE 'track_utxo' "$M"; then
    fail "the classifier references track_utxo -- it must be a pure address classifier"
fi

if (( FAILED == 0 )); then
    echo "OK: stake-reference classification (DC-EVIEW-02; typed 4-way result, bound-era gate, Conway pointer retirement, malformed!=Null, reward fail-closed, real-interop; no aggregation)"
fi
exit $FAILED
