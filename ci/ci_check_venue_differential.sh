#!/usr/bin/env bash
set -uo pipefail

# PREPROD-ENTRY-AUTHORITY P6-S5 (DC-EPOCH-37) -- semantics must be proven PER VENUE.
#
# A green mainnet-shaped corpus is not evidence that preview/preprod semantics are correct. P3 is the
# proof: it computed the epoch from hardcoded MAINNET constants, the whole mainnet corpus stayed
# byte-identical, and the defect was fatal on preprod (phantom boundary -> exit 43) and silently
# corrosive on preview (the ledger epoch never advanced for a store's entire life). Mainnet is the one
# venue where the wrong formula is right, so no mainnet test could ever have caught it.
#
#   (A) every venue in the CLOSED node-side registry has differential coverage -- adding a venue
#       without adding it to the differential fails here, rather than silently going untested.
#   (B) the differential's geometry MATCHES the node-side authorities verbatim (no drifted copy).
#   (C) the differential still asserts the properties that matter, including the P3 regression pin.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STATE="$REPO_ROOT/crates/ade_ledger/src/state.rs"
FIRSTRUN="$REPO_ROOT/crates/ade_node/src/native_firstrun.rs"
PROFILE="$REPO_ROOT/crates/ade_node/src/bootstrap_export.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$STATE" "$FIRSTRUN" "$PROFILE"; do
    [[ -e "$f" ]] || print_fail "missing expected path $f"
done

# --- the node-side authority: shelley_boundary_for_magic -> (start_epoch, start_slot) ---
AUTHORITY=$(awk '/fn shelley_boundary_for_magic/{f=1} f{print} f&&/^\}/{exit}' "$FIRSTRUN" \
    | grep -oE 'Some\(\(EpochNo\([0-9_]+\), SlotNo\([0-9_]+\)\)\)' \
    | grep -oE '[0-9_]+' | tr -d '_' | paste - - | sort)

# --- the differential's table: (name, start_epoch, start_slot, epoch_length) ---
DIFFERENTIAL=$(awk '/const VENUES: \[\(&str, u64, u64, u32\); [0-9]+\]/{f=1} f{print} f&&/\];/{exit}' "$STATE" \
    | grep -oE '\("[a-z]+", [0-9_]+, [0-9_]+, [0-9_]+\)' \
    | sed -E 's/\("[a-z]+", ([0-9_]+), ([0-9_]+), [0-9_]+\)/\1\t\2/' | tr -d '_' | sort)

if [[ -z "$AUTHORITY" ]]; then
    print_fail "(A) could not read shelley_boundary_for_magic -- the authority table moved"
fi
if [[ -z "$DIFFERENTIAL" ]]; then
    print_fail "(A) could not read the VENUES differential table in state.rs"
fi

if [[ "$AUTHORITY" != "$DIFFERENTIAL" ]]; then
    cat <<EOF
FAIL: (A/B) the venue differential does not cover the node-side venue registry exactly.

  authority (native_firstrun::shelley_boundary_for_magic, as epoch<TAB>slot):
$(sed 's/^/    /' <<<"$AUTHORITY")

  differential (ade_ledger state.rs VENUES):
$(sed 's/^/    /' <<<"$DIFFERENTIAL")

Every venue the node accepts must be exercised by the differential. A venue added to the registry
without differential coverage is a venue whose semantics nothing proves -- which is exactly how P3
shipped.
EOF
    FAILED=1
fi

# (B) epoch lengths agree with the network profile registry.
for pair in "preview:86_400" "preprod:432_000"; do
    venue="${pair%%:*}"; want="${pair##*:}"
    grep -q "\"$venue\"" "$PROFILE" \
        || print_fail "(B) $venue is missing from resolve_network_profile"
    grep -qE "\(\"$venue\", [0-9_]+, [0-9_]+, $want\)" "$STATE" \
        || print_fail "(B) the differential's epoch_length for $venue does not match the profile registry ($want)"
done

# (C) the properties are still asserted.
for t in venue_differential_epoch_derivation_is_correct_for_every_venue \
         venue_differential_boundary_fires_exactly_at_each_venue_boundary \
         venue_differential_epoch_agreement_discriminates_for_every_venue \
         venue_differential_mainnet_formula_is_wrong_off_mainnet; do
    grep -qE "^\s*fn $t\(\)" "$STATE" || print_fail "(C) the differential lost '$t'"
done

# The P3 regression pin must keep its MEASURED anchors as ASSERTIONS, not as prose.
#
# The first version of this check grepped the whole file for the bare numbers -- which also matched the
# module docs that RECOUNT the story ("preprod slot 130,046,891 -> 498 instead of 304"). Deleting the
# assertions therefore still passed. Its own negative test caught that. Scope to the pin's body and
# require the value to appear on an assert line, so documentation can never stand in for evidence.
PIN_BODY=$(awk '/fn venue_differential_mainnet_formula_is_wrong_off_mainnet\(\)/{f=1} f{print} f&&/^    \}$/{exit}' "$STATE")
if [[ -z "$PIN_BODY" ]]; then
    print_fail "(C) the P3 regression pin body was not found"
else
    for v in 304 498 1378 473; do
        grep -E 'assert' <<<"$PIN_BODY" | grep -qE "\b$v\b" \
            || print_fail "(C) the P3/P4 measured anchor $v is no longer ASSERTED in the regression pin (prose does not count)"
    done
fi

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_venue_differential: FAILED"
    exit 1
fi
echo "ci_check_venue_differential: OK ($(wc -l <<<"$AUTHORITY") venues covered; geometry matches the node registry)"
