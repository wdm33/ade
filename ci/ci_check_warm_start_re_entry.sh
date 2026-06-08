#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-AH S4b (DC-NODE-22): single-producer warm-start re-entry. In a declared rung-1
# single-producer venue, when warm-start recovery yields a durable local ChainDb::tip
# ABOVE the replay anchor (the derived replayed_anchor_block_no), forge mode re-enters
# SingleProducerExtendOwnDurableSpine{current_tip = ChainDb::tip} under the DC-NODE-20
# fence, WITHOUT a fresh followed-peer catch-up. ANY unmet condition FAILS CLOSED to
# InitialCatchupRequired. RED/GREEN forge-mode re-entry ONLY -- no ledger validation /
# chain selection / block validity / storage replay change.
#
# Asserts (production bodies; #[cfg(test)] + comments stripped):
#  (a) the GREEN decision warm_start_forge_mode (node_sync.rs): returns
#      SingleProducerExtendOwnDurableSpine ONLY behind VenueRole::SingleProducer + the
#      threshold (tip.block_no > anchor); the catch-all arm is InitialCatchupRequired;
#      no I/O / cert / fork-choice / admit token;
#  (b) the warm-start arm (node_lifecycle.rs) calls warm_start_forge_mode with the
#      recovered ChainDb::tip + state.replayed_anchor_block_no;
#  (c) the seam: BootstrapState carries replayed_anchor_block_no; warm_start_recovery
#      derives it from admit_count (the authoritative replay count, recovered_tip.block_no
#      - admit_count), NOT the snapshot-fragile raw wal.read_all() count.
#
# Fails closed if a future change drops the threshold/venue fence, mis-sources the anchor
# summary from the raw WAL count, or pulls cert / fork-choice into the re-entry.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SYNC="crates/ade_node/src/node_sync.rs"
LIFECYCLE="crates/ade_node/src/node_lifecycle.rs"
BOOTSTRAP="crates/ade_runtime/src/bootstrap.rs"

for f in "$SYNC" "$LIFECYCLE" "$BOOTSTRAP"; do
    if [[ ! -f "$f" ]]; then echo "FAIL: $f not found"; exit 1; fi
done

FAILED=0
fail() { echo "FAIL (warm-start-re-entry): $1"; FAILED=1; }

prod_body() { awk '/#\[cfg\(test\)\]/{exit} {print}' "$1" | sed -E 's://.*::'; }
SYNC_PROD="$(prod_body "$SYNC")"
LIFE_PROD="$(prod_body "$LIFECYCLE")"

WSFM="$(awk '/pub fn warm_start_forge_mode/{c=1} c{print} c&&/^}/{exit}' <<<"$SYNC_PROD")"

# --- (a) the GREEN decision ---------------------------------------------------
if [[ -z "$WSFM" ]]; then
    fail "pub fn warm_start_forge_mode not found in $SYNC"
else
    if ! grep -qE 'VenueRole::SingleProducer' <<<"$WSFM"; then
        fail "warm_start_forge_mode does not gate on VenueRole::SingleProducer"
    fi
    if ! grep -qE 'block_no > ' <<<"$WSFM"; then
        fail "warm_start_forge_mode does not apply the tip > anchor threshold (block_no >)"
    fi
    if ! grep -qE 'SingleProducerExtendOwnDurableSpine' <<<"$WSFM"; then
        fail "warm_start_forge_mode never returns the extend mode"
    fi
    if ! grep -qE 'InitialCatchupRequired' <<<"$WSFM"; then
        fail "warm_start_forge_mode does not fail closed to InitialCatchupRequired"
    fi
    for tok in 'read_adoption_cert' 'adoption_cert' 'select_best_chain' 'chain_selector' 'fork_choice' 'pump_block' 'File::'; do
        if grep -qE "$tok" <<<"$WSFM"; then
            fail "warm_start_forge_mode references a forbidden token ($tok) — it must be a pure GREEN decision (no I/O / cert / fork-choice / admit)"
        fi
    done
fi

# --- (b) the warm-start arm wiring -------------------------------------------
if ! grep -qE 'warm_start_forge_mode\(' <<<"$LIFE_PROD"; then
    fail "node_lifecycle.rs does not call warm_start_forge_mode"
fi
if ! grep -qE 'replayed_anchor_block_no' <<<"$LIFE_PROD"; then
    fail "the warm-start arm does not pass state.replayed_anchor_block_no into the decision"
fi
if ! grep -qE 'ChainDbServedSource::new' <<<"$LIFE_PROD"; then
    fail "the warm-start arm does not derive the recovered tip from ChainDbServedSource"
fi

# --- (c) the seam ------------------------------------------------------------
if ! grep -qE 'replayed_anchor_block_no:' "$BOOTSTRAP"; then
    fail "BootstrapState does not carry replayed_anchor_block_no ($BOOTSTRAP)"
fi
if ! grep -qE 'saturating_sub\(admit_count' <<<"$LIFE_PROD"; then
    fail "warm_start_recovery does not derive replayed_anchor_block_no from admit_count (recovered_tip.block_no - admit_count)"
fi
if grep -qE 'replayed_anchor.*read_all\(\)|read_all\(\).*replayed_anchor' <<<"$LIFE_PROD"; then
    fail "replayed_anchor_block_no must NOT be derived from the raw wal.read_all() count (snapshot-fragile)"
fi

if (( FAILED == 0 )); then
    echo "OK (warm-start-re-entry): warm_start_forge_mode is a pure GREEN decision returning SingleProducerExtendOwnDurableSpine only behind VenueRole::SingleProducer + the tip>anchor threshold (else InitialCatchupRequired, fail-closed; no I/O/cert/fork-choice/admit); the warm-start arm wires the recovered ChainDb::tip + state.replayed_anchor_block_no; BootstrapState carries the derived summary, computed from recovery's authoritative admit_count (not the raw WAL count) — DC-NODE-22; no BLUE/ledger/chain-selection change."
fi
exit $FAILED
