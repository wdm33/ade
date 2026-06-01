#!/usr/bin/env bash
set -euo pipefail

# CE-G-A-4 (PHASE4-N-F-G-A S4): the --mode node forge path fails closed at the
# single recovered seed-epoch boundary BEFORE leadership / KES signing, and drives
# NO nonce promotion (DC-EPOCH-03). Asserts:
#  (a) forge_one_from_recovered calls the explicit forge_epoch_admission guard
#      BEFORE query_leader_schedule — off-epoch fails closed before leadership;
#  (b) the guard derives the candidate epoch via the BLUE EraSchedule::locate
#      (no fabricated epoch math);
#  (c) the node forge path drives no NonceInput::EpochBoundary / CandidateFreeze
#      nonce promotion.
# Fails closed if a future change reorders the guard after leadership, fabricates
# the epoch, or introduces a nonce roll on the forge path.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SYNC="crates/ade_node/src/node_sync.rs"
LIFECYCLE="crates/ade_node/src/node_lifecycle.rs"

for f in "$SYNC" "$LIFECYCLE"; do
    if [[ ! -f "$f" ]]; then
        echo "FAIL: $f not found"
        exit 1
    fi
done

# Production body of node_sync.rs (drop the #[cfg(test)] module; strip line/doc
# comments so commentary naming a token does not trip the greps).
PROD="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$SYNC" | sed -E 's://.*::')"
if [[ -z "$PROD" ]]; then
    echo "FAIL: could not isolate production body of $SYNC"
    exit 1
fi

# Isolate forge_one_from_recovered's body (signature → its column-0 closing brace).
FORGE_FN="$(awk '/pub fn forge_one_from_recovered/{c=1} c{print} c&&/^}/{exit}' <<<"$PROD")"
if [[ -z "$FORGE_FN" ]]; then
    echo "FAIL: could not isolate forge_one_from_recovered in $SYNC"
    exit 1
fi

# (a) the explicit guard is called, and BEFORE query_leader_schedule.
ADM_LINE="$(grep -nE 'forge_epoch_admission\(' <<<"$FORGE_FN" | head -1 | cut -d: -f1)"
QLS_LINE="$(grep -nE 'query_leader_schedule\(' <<<"$FORGE_FN" | head -1 | cut -d: -f1)"
if [[ -z "$ADM_LINE" ]]; then
    echo "FAIL: forge_one_from_recovered does not call the explicit forge_epoch_admission guard"
    exit 1
fi
if [[ -z "$QLS_LINE" ]]; then
    echo "FAIL: forge_one_from_recovered no longer calls query_leader_schedule (unexpected)"
    exit 1
fi
if (( ADM_LINE >= QLS_LINE )); then
    echo "FAIL: forge_epoch_admission (line $ADM_LINE) must precede query_leader_schedule (line $QLS_LINE) — off-epoch must fail closed BEFORE leadership"
    exit 1
fi

# (b) the guard derives the candidate epoch via the BLUE EraSchedule::locate.
ADM_FN="$(awk '/pub fn forge_epoch_admission/{c=1} c{print} c&&/^}/{exit}' <<<"$PROD")"
if ! grep -qE 'era_schedule\.locate\(' <<<"$ADM_FN"; then
    echo "FAIL: forge_epoch_admission does not derive the epoch via era_schedule.locate (no fabricated epoch math)"
    exit 1
fi

# (c) no nonce promotion on the node forge path.
for f in "$SYNC" "$LIFECYCLE"; do
    BODY="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$f" | sed -E 's://.*::')"
    if grep -qE 'NonceInput::(EpochBoundary|CandidateFreeze)' <<<"$BODY"; then
        echo "FAIL: $f drives a nonce promotion (NonceInput::EpochBoundary/CandidateFreeze) on the forge path — forbidden (S4 fails closed instead)"
        exit 1
    fi
done

echo "OK: node forge fails closed at the seed-epoch boundary before leadership; guard uses EraSchedule::locate; no nonce promotion (CE-G-A-4 / DC-EPOCH-03)"
exit 0
