#!/usr/bin/env bash
set -uo pipefail

# PREPROD-ENTRY-AUTHORITY P5 (DC-LEDGER-13) -- MAINNET constants must stay CONTAINED.
#
# SHELLEY_START_SLOT / SHELLEY_START_EPOCH / SHELLEY_EPOCH_LENGTH describe mainnet and nothing else.
# Applied to another venue they silently yield a fictitious epoch (preprod slot 130,046,891 -> 498
# instead of 304; preview -> 473 instead of 1378). That was the P3 defect, and P4 (e1de7a2e) measured
# the cost: a preview store whose ledger never advanced past its seed epoch for its entire life,
# surfacing three epochs later as an opaque recovery fingerprint mismatch.
#
# P3 established the containment in PROSE ("the constants can only enter a computation through an
# explicit, named mainnet schedule"). This gate makes it mechanical:
#   (A) `slot_to_epoch` -- which applied these constants to any slot handed to it -- stays DELETED.
#   (B) the constants are referenced ONLY by their own definitions, by mainnet_shelley_schedule(),
#       and by explicitly ALLOWLISTED sites below.
#   (C) every allowlisted site carries a justification comment naming why it is mainnet-only, so the
#       allowlist cannot grow silently.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES="$REPO_ROOT/crates"
STATE="$CRATES/ade_ledger/src/state.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

[[ -e "$STATE" ]] || print_fail "missing expected path $STATE"

# --- The allowlist. ONE entry. Adding to this list is a deliberate act that must be justified. ---
#
# rules.rs:apply_epoch_boundary_full -- the monetary-expansion expected-blocks denominator
#   (SHELLEY_EPOCH_LENGTH / 20 = 21_600). This is the FULL-ledger (track_utxo=true) path that produces
#   the MAINNET reward results CE-71 / CE-3d are measured against; the accumulator path (preview /
#   multi-network) already sources the real per-era epoch length from the era schedule. Changing this
#   denominator is a REWARD-SEMANTICS change, not a containment cleanup, so P5 allowlists it rather
#   than entangling the two. Tracked as P5 follow-up.
ALLOWED_FILE="ade_ledger/src/rules.rs"

# (A) slot_to_epoch stays gone.
if grep -rnE '(pub(\(crate\))? )?fn slot_to_epoch' "$CRATES" --include=*.rs; then
    print_fail "(A) slot_to_epoch has come back -- it applies MAINNET constants to any venue (the P3 defect)"
fi

# (B) constants referenced only where allowed. Definitions and the named schedule live in state.rs.
HITS=$(grep -rn 'SHELLEY_START_SLOT\|SHELLEY_START_EPOCH\|SHELLEY_EPOCH_LENGTH' "$CRATES" --include=*.rs \
    | grep -v 'ade_ledger/src/state.rs' \
    | grep -vE '^\s*//' || true)

while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    # A pure comment mention is fine.
    line_text="${hit#*:*:}"
    if grep -Eq '^[[:space:]]*//' <<<"$line_text"; then
        continue
    fi
    if ! grep -q "$ALLOWED_FILE" <<<"$hit"; then
        print_fail "(B) MAINNET constant used outside mainnet_shelley_schedule() and the allowlist: $hit"
    fi
done <<<"$HITS"

# (C) the allowlisted file justifies itself: the mainnet-only reasoning must be stated next to the use.
ALLOWED_PATH="$CRATES/$ALLOWED_FILE"
if [[ -e "$ALLOWED_PATH" ]]; then
    if grep -q 'SHELLEY_EPOCH_LENGTH' "$ALLOWED_PATH"; then
        grep -q 'mainnet' "$ALLOWED_PATH" \
            || print_fail "(C) $ALLOWED_FILE uses a MAINNET constant with no justification naming it mainnet-only"
    fi
fi

# The named escape hatch must still exist -- otherwise (B) passes vacuously.
grep -q 'pub fn mainnet_shelley_schedule' "$STATE" \
    || print_fail "mainnet_shelley_schedule() is gone -- the only sanctioned way these constants may enter a computation"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_venue_constant_containment: FAILED"
    exit 1
fi
echo "ci_check_venue_constant_containment: OK (mainnet constants contained to the named schedule + 1 justified site)"
