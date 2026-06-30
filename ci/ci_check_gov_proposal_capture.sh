#!/usr/bin/env bash
set -uo pipefail

# CONWAY-PROPOSAL-DEPOSIT-EXPIRY S3 (DC-GOV-01, input half): the within-epoch
# governance capture + the field-19 vote tripwire + the live-proposal expiry
# TIMING AUTHORITY. This gate defends the closure mechanically:
#
#   1. The within-epoch governance pass `apply_block_governance` exists and is
#      WIRED into `apply_within_epoch` (not dead code).
#   2. The three fail-closed terminals exist on `LedgerTransitionError`:
#      VoteOnTrackedProposal, MalformedGovernanceField, GovActionLifetimeUnproven.
#   3. The field-19 vote-tripwire extractor `extract_voted_action_ids` exists.
#   4. `govActionLifetime` is IMPORTED from the certified curPParams (read at the
#      named index, NOT skipped), carried on `ImportedGovState`, and SEEDED into
#      the accumulator from the import — never a hardcoded 0 placeholder.
#   5. The bootstrap commitment BINDS `gov_action_lifetime` and is at v7 (the
#      import bump); the stale v6 commitment string is gone.
#   6. The capture path REFUSES a 0 (unimported) lifetime rather than fabricating
#      `expires_after = proposed_in`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

ACC="$REPO_ROOT/crates/ade_ledger/src/epoch_accumulator.rs"
LDB="$REPO_ROOT/crates/ade_ledger/src/ledgerdb_state.rs"
ASM="$REPO_ROOT/crates/ade_runtime/src/mithril_native_assembly.rs"

FAIL=0
for f in "$ACC" "$LDB" "$ASM"; do
    [ -f "$f" ] || { echo "FAIL: expected file missing: $f"; FAIL=1; }
done
[ "$FAIL" -eq 0 ] || exit 1

req() {  # file  extended-regex  failure-message
    grep -qE "$2" "$1" || { echo "FAIL: $3"; FAIL=1; }
}
forbid() {  # file  extended-regex  failure-message
    if grep -qE "$2" "$1"; then echo "FAIL: $3"; FAIL=1; fi
}

# 1. the within-epoch governance pass exists AND is wired into apply_within_epoch.
req "$ACC" 'fn apply_block_governance' \
    "apply_block_governance (the within-epoch governance pass) is missing"
req "$ACC" 'Some\(apply_block_governance\(' \
    "apply_block_governance is not wired into apply_within_epoch (dead code)"

# 2. the three fail-closed terminals.
req "$ACC" 'VoteOnTrackedProposal \{ tx_index: u64 \}' \
    "VoteOnTrackedProposal terminal (vote tripwire) is missing"
req "$ACC" 'MalformedGovernanceField \{ tx_index: u64 \}' \
    "MalformedGovernanceField terminal (no silent skip of field 19/20) is missing"
req "$ACC" 'GovActionLifetimeUnproven \{ tx_index: u64 \}' \
    "GovActionLifetimeUnproven terminal (no fabricated expiry) is missing"

# 3. the field-19 vote-tripwire extractor.
req "$ACC" 'fn extract_voted_action_ids' \
    "extract_voted_action_ids (the field-19 vote tripwire) is missing"

# 4. govActionLifetime imported (idx 26, not skipped) + carried + seeded from the import.
req "$LDB" 'CONWAY_PP_GOV_ACTION_LIFETIME_INDEX' \
    "govActionLifetime curPParams index const is missing (lifetime not captured from the import)"
req "$LDB" 'pub gov_action_lifetime: u64' \
    "ImportedGovState.gov_action_lifetime field is missing"
req "$ASM" 's1a\.imported_gov\.gov_action_lifetime' \
    "the accumulator assembly does not seed gov_action_lifetime from the import"
forbid "$ASM" 'gov_action_lifetime: 0' \
    "the accumulator assembly still seeds a hardcoded 0 gov_action_lifetime placeholder"

# 5. the bootstrap commitment binds the lifetime and is at v7.
req "$LDB" 'g\.gov_action_lifetime\.to_be_bytes' \
    "the bootstrap commitment does not bind gov_action_lifetime (tamper-evidence)"
req "$LDB" 'ade-native-nonutxo-state-commitment-v7' \
    "the bootstrap commitment was not bumped to v7 for the gov_action_lifetime binding"
forbid "$LDB" 'ade-native-nonutxo-state-commitment-v6' \
    "a stale v6 commitment string is still present"

# 6. the unproven-lifetime guard on the capture path.
req "$ACC" 'gov_action_lifetime == 0' \
    "the unproven (0) lifetime guard is missing from the proposal-capture path"

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: DC-GOV-01 S3 governance capture + vote tripwire + imported expiry-lifetime authority closed"
    exit 0
fi
exit 1
