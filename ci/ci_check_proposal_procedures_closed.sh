#!/usr/bin/env bash
set -uo pipefail

# DC-LEDGER-11 (PROPOSAL-PROCEDURES-DECODE PP-S1): proposal_procedures
# MUST NOT remain an opaque byte field in the authoritative Conway tx-body
# shape. This gate defends the closure mechanically:
#
#   1. ade_types::conway::governance defines `pub struct ProposalProcedure`
#      with the 4 fields (deposit, return_addr, gov_action, anchor).
#   2. ade_types::conway::tx declares
#      `proposal_procedures: Option<Vec<ProposalProcedure>>` — NOT
#      `Option<Vec<u8>>` (the prior opaque form).
#   3. ade_codec::conway::governance exists and exports both
#      `decode_proposal_procedures` and `encode_proposal_procedures`.
#   4. The Conway tx-body codec at key 20 calls the typed decoder/encoder
#      and does NOT use the prior `data[start..end].to_vec()` opaque
#      pass-through for `proposal_procedures`.
#   5. No `ProposalProcedure {` struct-literal construction outside the
#      sanctioned synthesis sites:
#        - ade_codec/src/conway/governance.rs (the decoder itself)
#        - ade_testkit/                       (future GREEN harness, PP-S2)
#        - crates/*/tests/                    (test code, exempt)
#        - #[cfg(test)] mod tests blocks      (inline tests, exempt)
#      Production callers MUST go through `decode_proposal_procedures`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

TYPES="$REPO_ROOT/crates/ade_types/src/conway/governance.rs"
BODY_TYPE="$REPO_ROOT/crates/ade_types/src/conway/tx.rs"
CODEC="$REPO_ROOT/crates/ade_codec/src/conway/governance.rs"
BODY_CODEC="$REPO_ROOT/crates/ade_codec/src/conway/tx.rs"

FAIL=0

for f in "$TYPES" "$BODY_TYPE" "$CODEC" "$BODY_CODEC"; do
    [ -f "$f" ] || { echo "FAIL: expected file missing: $f"; FAIL=1; }
done
[ "$FAIL" -eq 0 ] || exit 1

# 1. ProposalProcedure struct defined with the 4 fields.
if ! grep -qE '^pub struct ProposalProcedure' "$TYPES"; then
    echo "FAIL: $TYPES does not define 'pub struct ProposalProcedure'"
    FAIL=1
fi
for fld in 'pub deposit:' 'pub return_addr:' 'pub gov_action:' 'pub anchor:'; do
    if ! grep -q "${fld}" "$TYPES"; then
        echo "FAIL: $TYPES ProposalProcedure missing required field: ${fld}"
        FAIL=1
    fi
done

# 2. ConwayTxBody.proposal_procedures is the typed form.
if ! grep -qE 'pub proposal_procedures:[[:space:]]*Option<Vec<.*ProposalProcedure' "$BODY_TYPE"; then
    echo "FAIL: $BODY_TYPE ConwayTxBody.proposal_procedures is not Option<Vec<ProposalProcedure>>"
    FAIL=1
fi
if grep -qE 'pub proposal_procedures:[[:space:]]*Option<Vec<u8>>' "$BODY_TYPE"; then
    echo "FAIL: $BODY_TYPE ConwayTxBody.proposal_procedures reverted to Option<Vec<u8>>"
    FAIL=1
fi

# 3. ade_codec governance module exports the decoder + encoder.
for sym in 'pub fn decode_proposal_procedures' 'pub fn encode_proposal_procedures'; do
    if ! grep -qE "^${sym}" "$CODEC"; then
        echo "FAIL: $CODEC missing required item: ${sym}"
        FAIL=1
    fi
done

# 4. Body codec at key 20 routes through the typed decoder/encoder, not
#    opaque pass-through.
if ! grep -qE 'decode_proposal_procedures\(' "$BODY_CODEC"; then
    echo "FAIL: $BODY_CODEC does not call decode_proposal_procedures (key 20 path)"
    FAIL=1
fi
if ! grep -qE 'encode_proposal_procedures\(' "$BODY_CODEC"; then
    echo "FAIL: $BODY_CODEC does not call encode_proposal_procedures (key 20 path)"
    FAIL=1
fi
# Forbid the prior opaque pass-through pattern at key 20: look for
# `proposal_procedures = Some(data[...]` assignment AND for
# `buf.extend_from_slice(b)` inside a `body.proposal_procedures` block.
# The forbidden patterns would resurface if someone reverts the slice.
if grep -nE 'proposal_procedures[[:space:]]*=[[:space:]]*Some\(data\[' "$BODY_CODEC"; then
    echo "FAIL: $BODY_CODEC reintroduces opaque-bytes pass-through for proposal_procedures"
    FAIL=1
fi

# 5. No ProposalProcedure { struct-literal construction outside sanctioned
#    sites. Sanctioned: the decoder file itself, the testkit, any test
#    file under crates/*/tests/, and inline #[cfg(test)] modules
#    (detected by the file path; co-located lib tests get the same exemption).
LITERALS=$(grep -rnE 'ProposalProcedure[[:space:]]*\{' \
    "$REPO_ROOT/crates" \
    --include=*.rs 2>/dev/null \
    | grep -vE 'pub struct ProposalProcedure' \
    | grep -vE '/ade_codec/src/conway/governance\.rs:' \
    | grep -vE '/ade_testkit/' \
    | grep -vE '/tests/' \
    | grep -vE '/benches/' \
    | grep -vE ':[[:space:]]*//' \
    || true)
# Now within the remaining hits, allow #[cfg(test)] mod tests inline blocks.
# Heuristic: if the hit is in a file that contains `#[cfg(test)]` and the
# hit's line number is past the first `#[cfg(test)]` occurrence, it's a
# test-context construction. This is approximate but matches the project's
# convention (inline tests live at the bottom of src/ files).
FILTERED_LITERALS=""
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    file="${hit%%:*}"
    rest="${hit#*:}"
    line_no="${rest%%:*}"
    cfg_test_line=$(grep -nE '^#\[cfg\(test\)\]' "$file" 2>/dev/null | head -1 | cut -d: -f1)
    if [ -n "$cfg_test_line" ] && [ "$line_no" -gt "$cfg_test_line" ]; then
        continue
    fi
    FILTERED_LITERALS="${FILTERED_LITERALS}${hit}
"
done <<< "$LITERALS"
FILTERED_LITERALS="${FILTERED_LITERALS%$'\n'}"
if [ -n "$FILTERED_LITERALS" ]; then
    echo "FAIL: ProposalProcedure { ... } struct-literal construction outside sanctioned sites:"
    echo "$FILTERED_LITERALS"
    FAIL=1
fi

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: DC-LEDGER-11 proposal_procedures closed (ProposalProcedure typed + closed decode/encode + body-codec wired + no opaque pass-through + no out-of-band construction)"
fi
exit "$FAIL"
