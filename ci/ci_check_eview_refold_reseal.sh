#!/usr/bin/env bash
set -uo pipefail

# EVIEW-RECOVERY-LINEAGE R2 -- a refold must re-seal frozen leadership byte-identically.
#
# Gates DC-EPOCH-32/33. The regression these prevent is a measured live one: the co-advancer drives
# the reduced checkpoint to the durable TIP at the end of every pass, an accumulator reset does not
# rewind it, and the forward-only advance SILENTLY no-ops when asked to go back to a boundary point
# (`from = cursor + 1`, break on the first block past the target, `Ok(())`). So a refold read its
# boundary mark and `finalize()` commitment AT THE TIP and re-sealed frozen leadership that
# disagreed with the durable eview activation record. The divergence stayed latent -- a running node
# never compares -- and the NEXT restart halted on `EpochViewPostPromotionMismatch` (exit 43).
#
# This gate asserts STRUCTURE, not just that tests exist: reading the mark or the commitment before
# the checkpoint is positioned, dropping the rewind, dropping the verification, going back to the
# bare forward advance, or sealing anyway when the boundary point is unreachable each fail here even
# if everything still compiles.
#
# NB (here-string discipline): never `echo "$BIGVAR" | grep -q` under pipefail -- grep exits on first
# match, echo takes SIGPIPE, and the gate flakes on large files. Grep files directly, or `<<<`.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
FAILED=0
fail() { echo "FAIL: $1"; FAILED=1; }
ok()   { echo "  ok: $1"; }

LIFE=crates/ade_node/src/node_lifecycle.rs
CKPT=crates/ade_runtime/src/chaindb/reduced_utxo_checkpoint.rs

for f in "$LIFE" "$CKPT"; do
    [ -f "$f" ] || { fail "missing $f"; echo "RESULT: FAIL"; exit 1; }
done

echo "== DC-EPOCH-32: the boundary seal reads a checkpoint POSITIONED on the boundary point =="

CO="$(awk '/^fn advance_ledger_state_to_durable_tip/,/^}/' "$LIFE")"
[ -n "$CO" ] || fail "advance_ledger_state_to_durable_tip missing"

# The bare forward advance must NOT be what brings the checkpoint to s_prev. It is purely forward,
# so on a refold (cursor at the tip) it moves nothing and reports success.
grep -qE 'advance_reduced_checkpoint_forward_to\(Some\(cp\), *chaindb, *s_prev\)' <<< "$CO" \
    && fail "the boundary seal must NOT use the bare forward advance (it silently no-ops when the cursor is past s_prev)"
grep -qE 'position_reduced_checkpoint_at_boundary\(cp, *chaindb, *s_prev\)' <<< "$CO" \
    || fail "the boundary seal must position the checkpoint via position_reduced_checkpoint_at_boundary"
ok "boundary seal goes through the positioning helper"

# ORDERING is the invariant: the mark and the commitment are read OFF the checkpoint, so the
# positioning must happen strictly before both. A reordering silently reintroduces the whole defect.
line_of() { grep -nE "$1" <<< "$CO" | head -1 | cut -d: -f1; }
POS_LINE="$(line_of 'position_reduced_checkpoint_at_boundary\(cp,')"
MARK_LINE="$(line_of 'cp\.sum_base_credential_stake\(\)')"
FIN_LINE="$(line_of 'cp\.finalize\(\)')"
for v in POS_LINE MARK_LINE FIN_LINE; do
    [ -n "${!v}" ] || fail "could not locate $v in the co-advancer"
done
if [ -n "$POS_LINE" ] && [ -n "$MARK_LINE" ] && [ -n "$FIN_LINE" ]; then
    [ "$POS_LINE" -lt "$MARK_LINE" ] \
        || fail "the boundary MARK is read before the checkpoint is positioned (mark would be taken at the wrong chain point)"
    [ "$POS_LINE" -lt "$FIN_LINE" ] \
        || fail "the checkpoint COMMITMENT is finalized before the checkpoint is positioned"
    ok "positioning precedes both the mark capture and the commitment finalize"
fi

echo "== DC-EPOCH-32: positioning rewinds when ahead, and VERIFIES rather than assumes =="

POS="$(awk '/^fn position_reduced_checkpoint_at_boundary/,/^}/' "$LIFE")"
[ -n "$POS" ] || fail "position_reduced_checkpoint_at_boundary missing"

grep -qE 'reset_to_bootstrap\(\)' <<< "$POS" \
    || fail "positioning must REWIND when the cursor is past the boundary point (the reduced delta is not invertible)"
grep -qE 'verify_ready_at\(boundary_slot, *seed\)' <<< "$POS" \
    || fail "positioning must VERIFY it landed exactly on the boundary point, never assume it"
grep -qE 'CheckpointPositioning::Unreachable' <<< "$POS" \
    || fail "positioning must be able to report that the boundary point is unreachable"
ok "rewind + exact-position verification + unreachable outcome present"

# `verify_ready_at` is the gate the DERIVE path already fails closed on. If its Ahead arm were
# softened, the seal path's verification would silently accept a checkpoint past the target again.
READY="$(awk '/pub fn verify_ready_at/,/^    }/' "$CKPT")"
grep -qE 'CheckpointReadinessError::Ahead' <<< "$READY" \
    || fail "verify_ready_at must still fail closed when the checkpoint is AHEAD of the required slot"
grep -qE 'CheckpointReadinessError::Lagging' <<< "$READY" \
    || fail "verify_ready_at must still fail closed when the checkpoint is LAGGING"
ok "verify_ready_at retains exact-at-slot semantics in both directions"

echo "== DC-EPOCH-33: an unreachable boundary point STALLS, it never seals =="

# The PRODUCTION match only (`(cp,` -- the tests call it as `(&cp,`), from the match line to the
# arm block's closing brace. A plain awk range would run to EOF if the brace pattern drifted, and
# would then sweep in unrelated `_ =>` arms from the rest of the file.
MATCH="$(awk '/match position_reduced_checkpoint_at_boundary\(cp,/{f=1} f{print} f&&/^ {24}\}$/{exit}' "$LIFE")"
[ -n "$MATCH" ] || fail "could not locate the positioning match in the co-advancer"
[ "$(wc -l <<< "$MATCH")" -lt 40 ] \
    || fail "the positioning match block did not terminate where expected -- this gate's extraction has drifted"
grep -qE 'CheckpointPositioning::Unreachable \{' <<< "$MATCH" \
    || fail "the seal path must handle Unreachable explicitly"
grep -qE '^\s+_ =>' <<< "$MATCH" \
    && fail "no catch-all arm: a new positioning outcome must not silently fall through to sealing"
grep -qE 'break;' <<< "$MATCH" \
    || fail "an unreachable boundary point must STALL (break) -- sealing from an unpositioned checkpoint is the defect"
ok "unreachable stalls observe-only, no catch-all arm"

echo "== DC-EPOCH-32/33: proof tests present =="

check_test() { grep -qE "fn ${1}\b" "$2" || fail "missing proof test: ${1} (expected in $2)"; }
check_test positioning_rewinds_a_checkpoint_that_sits_past_the_boundary_point "$LIFE"
check_test a_bare_forward_advance_asked_to_go_backward_silently_moves_nothing "$LIFE"
check_test a_boundary_point_before_the_sealed_seed_is_unreachable             "$LIFE"
check_test a_refold_reseals_frozen_leadership_byte_identically                "$LIFE"
[ "$FAILED" -eq 0 ] && ok "all 4 named proof tests present"

if [ "$FAILED" -ne 0 ]; then
    echo "RESULT: FAIL (refold re-seal identity regressed)"
    exit 1
fi
echo "RESULT: PASS (DC-EPOCH-32/33 structurally enforced + proofs present)"
