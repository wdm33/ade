#!/usr/bin/env bash
set -uo pipefail

# DC-EPOCH-19 (LIVE-LEDGER-EPOCH-TRANSITION S1): the EpochAccumulator is the non-UTxO companion
# authority -- precisely `LedgerState - UTxO + {prev-epoch buffers, pending reward update}`, NOT a
# second full ledger. Mechanical enforcement (IDD principle 10) of the field-ownership contract so the
# accumulator can never quietly drift into holding the UTxO set (which would defeat MEM-OPT and the
# whole point: a forever-running follower that keeps the UTxO on disk, not in live RAM):
#   (A) the EpochAccumulator struct has NO UTxO field (the FORBIDDEN set is structurally unrepresentable);
#   (B) it DOES carry the two-buffer reward inputs + the bootstrap-transient seed (the OWNED set);
#   (C) as_ledger_view materializes an EMPTY UTxO -- the accumulator never feeds a real UTxO to the
#       reused boundary/cert primitives;
#   (D) the canonical codec is version-gated + Conway-only + has the byte-canonical re-encode backstop;
#   (E) DC-EPOCH-19 is in the registry.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ACC="$REPO_ROOT/crates/ade_ledger/src/epoch_accumulator.rs"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$ACC" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done
[[ $FAILED -eq 0 ]] || exit 1

# Isolate the `pub struct EpochAccumulator { ... }` body (header line to the first closing brace), then
# keep ONLY field-declaration lines (`pub <name>: <Type>,`) -- doc comments legitimately mention "UTxO"
# (e.g. "the large UTxO set is NOT here"), so checking comment text would false-FAIL.
STRUCT_BODY="$(awk '/^pub struct EpochAccumulator \{/{f=1} f{print} f&&/^\}/{exit}' "$ACC")"
[[ -n "$STRUCT_BODY" ]] || print_fail "could not locate the EpochAccumulator struct definition"
FIELD_LINES="$(grep -E '^[[:space:]]*pub[[:space:]]+[A-Za-z_]+[[:space:]]*:' <<<"$STRUCT_BODY")"
[[ -n "$FIELD_LINES" ]] || print_fail "could not isolate EpochAccumulator field declarations"

# (A) FORBIDDEN: no UTxO field. No field name OR type may reference a UTxO or a full per-credential
#     stake map -- the UTxO is the reduced checkpoint's, read via ctx, never stored here. (Field lines
#     only, case-insensitive, so the structural guarantee can't be evaded by casing or a renamed type.)
if grep -Eiq 'utxo' <<<"$FIELD_LINES"; then
    print_fail "(A) an EpochAccumulator field references UTxO -- the FORBIDDEN set must be unrepresentable"
fi
if grep -Eiq 'stake_by_cred|credential_stake|stakebycred' <<<"$FIELD_LINES"; then
    print_fail "(A) an EpochAccumulator field is a full per-credential stake map -- defer it to ctx"
fi

# (B) OWNED: the two-buffer reward model + the bootstrap-transient seed are present (the cardano
#     nesBprev/nesBcur split + the DC-EPOCH-18 seed the accumulator must carry to be self-sufficient).
grep -Eq 'prev_block_production: *BTreeMap<PoolId, *u64>' <<<"$STRUCT_BODY" \
    || print_fail "(B) prev_block_production (nesBprev) is missing -- the reward consumes nesBprev, not nesBcur"
grep -Eq 'prev_epoch_fees: *Coin' <<<"$STRUCT_BODY" \
    || print_fail "(B) prev_epoch_fees is missing -- it pairs with prev_block_production for the boundary reward"
grep -Eq 'pending_reward_update: *Option<BootstrapRewardUpdate>' <<<"$STRUCT_BODY" \
    || print_fail "(B) pending_reward_update (the bootstrap-transient seed) is missing"

# (C) as_ledger_view must build an EMPTY UTxO -- the accumulator never smuggles a real UTxO into the
#     reused single-authority primitives.
grep -Eq 'utxo_state: *UTxOState::new\(\)' "$ACC" \
    || print_fail "(C) as_ledger_view does not materialize an EMPTY UTxO (UTxOState::new())"

# (D) the canonical persistence codec is version-gated, Conway-only, and has the re-encode backstop.
grep -q 'pub const EPOCH_ACCUMULATOR_SCHEMA_VERSION' "$ACC" \
    || print_fail "(D) the codec is not version-pinned"
grep -q 'EraNotSupported' "$ACC" \
    || print_fail "(D) the codec does not gate the era (Conway-only)"
grep -q 'if encode_epoch_accumulator(&acc) != bytes' "$ACC" \
    || print_fail "(D) the byte-canonical re-encode backstop is missing from decode"

# (E) DC-EPOCH-19 in the registry.
grep -q 'DC-EPOCH-19' "$REG" \
    || print_fail "(E) DC-EPOCH-19 is not declared in the invariant registry"

if [[ $FAILED -ne 0 ]]; then
    echo "DC-EPOCH-19 epoch-accumulator structural check FAILED" >&2
    exit 1
fi
echo "OK: epoch-accumulator field-ownership contract holds (no UTxO; two-buffer + seed owned; codec gated)"
