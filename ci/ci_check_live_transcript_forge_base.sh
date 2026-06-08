#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-AH S4a (CN-NODE-04 / DC-NODE-20 evidence): the --mode node sched transcript
# directly witnesses the forge-base decision. The closed NodeSchedEvent vocabulary
# carries a ForgeBaseSelected event (forge_base_source=local_chaindb_tip, the entered
# forge mode, cert_path_present) + an enriched ForgeResult (self_admit_via_pump_block);
# the relay loop emits ForgeBaseSelected with ForgeBaseSource::LocalChaindbTip +
# cert_path_present: false; the transcript is wired to the --log JSONL file. RED
# evidence ONLY -- it observes + serializes the decision already made (DC-NODE-20),
# changing no authority.
#
# Asserts (source-level; #[cfg(test)] + line/doc comments stripped):
#  (a) vocabulary (sched_event.rs): ForgeBaseSelected with forge_base_source /
#      forge_base_hash / forge_base_block_no / cert_path_present; ForgeResult with
#      self_admit_via_pump_block / entered_forge_mode; ForgeBaseSource::LocalChaindbTip
#      -> "local_chaindb_tip";
#  (b) encoder (sched_writer.rs): emits the forge_base_source / cert_path_present /
#      self_admit_via_pump_block JSON keys;
#  (c) emit site (node_lifecycle.rs): the loop emits ForgeBaseSelected sourced as
#      ForgeBaseSource::LocalChaindbTip with cert_path_present: false, and the sched
#      writer is wired to the --log file (cli.log_path);
#  (d) NO adoption-cert token (read_adoption_cert / adoption_cert_path /
#      VenueAdoptionCertificate) appears in the sched vocabulary / encoder.
#
# Fails closed if a future change drops the forge-base witness, mis-sources the base,
# stops wiring the transcript to the --log file, or smuggles a cert token into the
# sched path.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SCHED_EVENT="crates/ade_node/src/live_log/sched_event.rs"
SCHED_WRITER="crates/ade_node/src/live_log/sched_writer.rs"
LIFECYCLE="crates/ade_node/src/node_lifecycle.rs"

for f in "$SCHED_EVENT" "$SCHED_WRITER" "$LIFECYCLE"; do
    if [[ ! -f "$f" ]]; then echo "FAIL: $f not found"; exit 1; fi
done

FAILED=0
fail() { echo "FAIL (live-transcript-forge-base): $1"; FAILED=1; }

prod_body() { awk '/#\[cfg\(test\)\]/{exit} {print}' "$1" | sed -E 's://.*::'; }
EVENT_PROD="$(prod_body "$SCHED_EVENT")"
WRITER_PROD="$(prod_body "$SCHED_WRITER")"
LIFE_PROD="$(prod_body "$LIFECYCLE")"

# --- (a) the closed vocabulary witnesses the forge base ----------------------
for tok in ForgeBaseSelected forge_base_source forge_base_hash forge_base_block_no cert_path_present self_admit_via_pump_block entered_forge_mode; do
    if ! grep -qE "$tok" <<<"$EVENT_PROD"; then
        fail "the closed NodeSchedEvent vocabulary is missing '$tok' (sched_event.rs)"
    fi
done
if ! grep -qE 'LocalChaindbTip' <<<"$EVENT_PROD" || ! grep -qE '"local_chaindb_tip"' <<<"$EVENT_PROD"; then
    fail "ForgeBaseSource::LocalChaindbTip / \"local_chaindb_tip\" not defined (sched_event.rs)"
fi

# --- (b) the encoder emits the JSON keys ------------------------------------
for key in forge_base_source cert_path_present self_admit_via_pump_block; do
    if ! grep -qE "\"$key\"" <<<"$WRITER_PROD"; then
        fail "the encoder does not emit the JSON key '$key' (sched_writer.rs)"
    fi
done

# --- (c) the emit site sources the base from the local tip + wires --log ----
if ! grep -qE 'NodeSchedEvent::ForgeBaseSelected' <<<"$LIFE_PROD"; then
    fail "node_lifecycle.rs does not emit ForgeBaseSelected"
fi
if ! grep -qE 'ForgeBaseSource::LocalChaindbTip' <<<"$LIFE_PROD"; then
    fail "the emit does not source the forge base as ForgeBaseSource::LocalChaindbTip"
fi
if ! grep -qE 'cert_path_present: false' <<<"$LIFE_PROD"; then
    fail "the emit does not record cert_path_present: false (DC-NODE-21)"
fi
if ! grep -qE 'File::create\(&cli\.log_path\)' <<<"$LIFE_PROD"; then
    fail "the sched transcript is not wired to the --log JSONL file (cli.log_path) — node-run.jsonl must be the canonical artifact"
fi

# --- (d) no adoption-cert token in the sched path ---------------------------
for tok in read_adoption_cert adoption_cert_path VenueAdoptionCertificate; do
    if grep -qE "$tok" <<<"$EVENT_PROD$WRITER_PROD"; then
        fail "an adoption-cert token ($tok) appears in the sched vocabulary/encoder — the transcript must be cert-free"
    fi
done

if (( FAILED == 0 )); then
    echo "OK (live-transcript-forge-base): the --mode node sched transcript witnesses the DC-NODE-20 forge base — ForgeBaseSelected{forge_base_source=local_chaindb_tip, cert_path_present=false} + ForgeResult{self_admit_via_pump_block} in the closed vocabulary + encoder, emitted by the relay loop, wired to the --log JSONL file; no adoption-cert token in the sched path (RED evidence, CN-NODE-04)."
fi
exit $FAILED
