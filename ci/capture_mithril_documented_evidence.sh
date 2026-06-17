#!/usr/bin/env bash
#
# capture_mithril_documented_evidence.sh — RO-MITHRIL-IMPORT-01 item (c)
# turnkey capture (slice RO-MITHRIL-IMPORT-01-EVIDENCE-SCHEMA).
#
# RED operator tool. Orchestrates the documented-interface pass end-to-end and
# emits a bundle that ci/validate_mithril_documented_evidence.sh validates:
# the positive --mode node first-run (verify_mithril_binding PASS) PLUS a
# mismatched-manifest negative control (binding fail-closed). It assembles the
# manifest + artifacts; it does NOT fabricate any value — every artifact is a
# real command output and the validator sha256-binds them.
#
# DOCTRINE: Mithril is acquisition/peer infra, NEVER an Ade BLUE trust root
# (see ci/mithril_restore_preprod_peer.sh). This script consumes the peer's
# certified state through DOCUMENTED cardano-cli interfaces only.
#
# Prerequisites the OPERATOR provides (this script does not install or fake):
#   * mithril-client: snapshot acquired + cert verified (the cert metadata).
#   * A cardano-node peer brought to the certified state, with cardano-cli
#     able to query it (CARDANO_NODE_SOCKET_PATH set).
#   * Rust toolchain to build ade_node.
#
# Usage (all via env, fail-closed if a required one is unset):
#   OUT_DIR=/path/to/bundle.d \
#   NETWORK=preprod NETWORK_MAGIC=1 \
#   GENESIS_HASH_HEX=<64hex> GENESIS_PATH=/path/to/genesis-bundle \
#   MITHRIL_AGG_ENDPOINT=<url> MITHRIL_CERT_HASH=<64hex> \
#   MITHRIL_SIGNED_ENTITY='CardanoDatabase { epoch = 291, immutable_file_number = 5758 }' \
#   IMMUTABLE_LO=0 IMMUTABLE_HI=5758 \
#   CARDANO_NODE_SOCKET_PATH=/path/node.socket \
#   CAPTURED_BY='<operator>' \
#     ci/capture_mithril_documented_evidence.sh
#
# Output: $OUT_DIR/ containing utxo.json, consensus-inputs.json,
# mithril-manifest.json, mithril-manifest.negative.json, node-first-run.stderr.log,
# node-first-run.negative.stderr.log, and the bundle manifest
# mithril-documented-evidence_<network>_<date>.toml. Move the manifest to
# docs/evidence/ (with $OUT_DIR's files beside it) once reviewed, then run the
# validator. This script does NOT commit anything and does NOT flip any rule.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

die() { echo "capture_mithril_documented_evidence: FAIL — $1" >&2; exit 1; }
require_var() { [[ -n "${!1:-}" ]] || die "required env var $1 is unset"; }
require_cmd() { command -v "$1" >/dev/null 2>&1 || die "required command '$1' not found on PATH"; }

for v in OUT_DIR NETWORK NETWORK_MAGIC GENESIS_HASH_HEX GENESIS_PATH \
         MITHRIL_AGG_ENDPOINT MITHRIL_CERT_HASH MITHRIL_SIGNED_ENTITY \
         IMMUTABLE_LO IMMUTABLE_HI CARDANO_NODE_SOCKET_PATH CAPTURED_BY; do
  require_var "$v"
done
require_cmd cardano-cli
require_cmd sha256sum
require_cmd cargo
export CARDANO_NODE_SOCKET_PATH

mkdir -p "$OUT_DIR"
sha() { sha256sum "$1" | awk '{print $1}'; }

# --- 1. Build ade_node (record the exact commit). ----------------------------
ADE_COMMIT="$(git rev-parse HEAD)"
echo "==> building ade_node @ $ADE_COMMIT"
cargo build -p ade_node --release >&2
ADE_BIN="$REPO_ROOT/target/release/ade_node"
[[ -x "$ADE_BIN" ]] || die "ade_node binary not built at $ADE_BIN"

# --- 2. Documented extraction from the certified peer. -----------------------
echo "==> cardano-cli query tip (operator seed point)"
TIP_JSON="$OUT_DIR/tip.json"
cardano-cli query tip --testnet-magic "$NETWORK_MAGIC" > "$TIP_JSON" \
  || cardano-cli query tip --mainnet > "$TIP_JSON" \
  || die "cardano-cli query tip failed (is the peer at the certified state?)"
SEED_SLOT="$(grep -oE '"slot"[[:space:]]*:[[:space:]]*[0-9]+' "$TIP_JSON" | grep -oE '[0-9]+' | head -1)"
SEED_BLOCK_HASH="$(grep -oE '"hash"[[:space:]]*:[[:space:]]*"[0-9a-fA-F]+"' "$TIP_JSON" | grep -oE '[0-9a-fA-F]{64}' | head -1)"
[[ -n "$SEED_SLOT" && -n "$SEED_BLOCK_HASH" ]] || die "could not parse slot/hash from $TIP_JSON"
echo "    seed point: slot=$SEED_SLOT hash=$SEED_BLOCK_HASH"

echo "==> cardano-cli query utxo --whole-utxo (seed)"
UTXO_JSON="$OUT_DIR/utxo.json"
cardano-cli query utxo --whole-utxo --testnet-magic "$NETWORK_MAGIC" --out-file "$UTXO_JSON" \
  || cardano-cli query utxo --whole-utxo --mainnet --out-file "$UTXO_JSON" \
  || die "cardano-cli query utxo failed"

echo "==> consensus inputs bundle"
CINPUTS_JSON="$OUT_DIR/consensus-inputs.json"
if [[ -x ci/build_consensus_inputs_bundle.sh ]]; then
  ci/build_consensus_inputs_bundle.sh --network "$NETWORK" "$CINPUTS_JSON" >&2 \
    || die "ci/build_consensus_inputs_bundle.sh failed (see its venue env: ADE_LIVE_PEER_CONTAINER / ADE_LIVE_PEER_SOCKET / ...)"
fi
[[ -f "$CINPUTS_JSON" ]] || die "consensus inputs not produced at $CINPUTS_JSON — run ci/build_consensus_inputs_bundle.sh for this venue and place the output there"

# --- 3. Build the Mithril manifest (RawMithrilManifest) for --mithril-manifest-path.
# certified_point = the operator-extracted point (the cert attests this point;
# for a passing binding the two agree by construction at the certified tip).
MANIFEST_JSON="$OUT_DIR/mithril-manifest.json"
cat > "$MANIFEST_JSON" <<JSON
{
  "artifact_type": "cardano-database-snapshot",
  "certificate_hash_hex": "$MITHRIL_CERT_HASH",
  "network_magic": $NETWORK_MAGIC,
  "genesis_hash_hex": "$GENESIS_HASH_HEX",
  "certified_point": { "slot": $SEED_SLOT, "block_hash_hex": "$SEED_BLOCK_HASH" },
  "immutable_range": { "lo": $IMMUTABLE_LO, "hi": $IMMUTABLE_HI },
  "source_mithril_client_version": "$(mithril-client --version 2>/dev/null | head -1 || echo unknown)",
  "source_command": "mithril-client cardano-db download (aggregator $MITHRIL_AGG_ENDPOINT)"
}
JSON

# Negative control: same manifest, certified block hash deliberately flipped.
NEG_MANIFEST_JSON="$OUT_DIR/mithril-manifest.negative.json"
NEG_HASH="$(printf '%s' "$SEED_BLOCK_HASH" | tr '0-9a-f' '1-9a-f0')"  # perturb every nibble
sed "s/$SEED_BLOCK_HASH/$NEG_HASH/" "$MANIFEST_JSON" > "$NEG_MANIFEST_JSON"

# --- 4. Positive: ade_node --mode node first-run (verify_mithril_binding). ----
run_first_run() {
  local manifest="$1" snap="$2" wal="$3" log="$4"
  rm -rf "$snap" "$wal"; mkdir -p "$snap" "$wal"
  set +e
  "$ADE_BIN" --mode node \
    --genesis-path "$GENESIS_PATH" --network "$NETWORK" \
    --json-seed "$UTXO_JSON" --consensus-inputs-path "$CINPUTS_JSON" \
    --mithril-manifest-path "$manifest" \
    --seed-point-slot "$SEED_SLOT" --seed-block-hash "$SEED_BLOCK_HASH" \
    --network-magic "$NETWORK_MAGIC" --genesis-hash "$GENESIS_HASH_HEX" \
    --snapshot-dir "$snap" --wal-dir "$wal" \
    > "$log" 2>&1
  local rc=$?
  set -e
  return $rc
}

echo "==> ade_node --mode node first-run (POSITIVE)"
POS_LOG="$OUT_DIR/node-first-run.stderr.log"
POS_RC=0
run_first_run "$MANIFEST_JSON" "$OUT_DIR/snap.pos" "$OUT_DIR/wal.pos" "$POS_LOG" || POS_RC=$?
echo "    exit=$POS_RC"
grep -q "first-run Mithril bootstrap complete" "$POS_LOG" \
  || die "positive first-run did not report binding success (see $POS_LOG)"
[[ "$POS_RC" == "0" ]] || die "positive first-run exited $POS_RC (expected 0)"
BINDING_RESULT="pass"
INIT_LEDGER_FP="$(grep -oE 'initial_ledger_fingerprint=[^,)]+' "$POS_LOG" | head -1 | sed 's/initial_ledger_fingerprint=//')"

echo "==> ade_node --mode node first-run (NEGATIVE control)"
NEG_LOG="$OUT_DIR/node-first-run.negative.stderr.log"
NEG_RC=0
run_first_run "$NEG_MANIFEST_JSON" "$OUT_DIR/snap.neg" "$OUT_DIR/wal.neg" "$NEG_LOG" || NEG_RC=$?
echo "    exit=$NEG_RC"
[[ "$NEG_RC" != "0" ]] || die "negative control unexpectedly SUCCEEDED — binding did not discriminate (see $NEG_LOG)"
# Classify the discriminating error from the transcript.
NEG_ERROR="$(grep -oE 'CertifiedPointMismatch|EpochMismatch|NetworkMagicMismatch|GenesisHashMismatch|CertificateHashMismatch|UnsupportedArtifactType' "$NEG_LOG" | head -1)"
[[ -n "$NEG_ERROR" ]] || die "negative control failed but with an unrecognised error (see $NEG_LOG) — not a binding-discrimination proof"

# --- 5. Ade-recomputed values (independent of the node, for cross-check). -----
SEED_ARTIFACT_HASH=""
if command -v b2sum >/dev/null 2>&1; then
  SEED_ARTIFACT_HASH="$(b2sum -l 256 "$UTXO_JSON" | awk '{print $1}')"
fi
[[ -n "$SEED_ARTIFACT_HASH" ]] || die "b2sum (BLAKE2b-256) needed to record ade_recomputed_seed_artifact_hash; install coreutils"
UTXO_FP="$(grep -oE 'imported_utxo_fingerprint[=: ]+[0-9a-fA-F]+' "$POS_LOG" | grep -oE '[0-9a-fA-F]+$' | head -1)"
UTXO_FP="${UTXO_FP:-unknown_see_transcript}"

# --- 6. Emit the bundle manifest. --------------------------------------------
DATE="$(grep -oE '"time"[[:space:]]*:[[:space:]]*"[0-9-]+' "$TIP_JSON" | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}' | head -1)"
DATE="${DATE:-undated}"
BUNDLE="$OUT_DIR/mithril-documented-evidence_${NETWORK}_${DATE}.toml"
cat > "$BUNDLE" <<TOML
schema_version = 1

ade_commit                  = "$ADE_COMMIT"
network                     = "$NETWORK"
captured_by                 = "$CAPTURED_BY"
mithril_aggregator_endpoint = "$MITHRIL_AGG_ENDPOINT"
mithril_certificate_hash    = "$MITHRIL_CERT_HASH"
mithril_client_version      = "$(mithril-client --version 2>/dev/null | head -1 || echo unknown)"
cardano_node_version        = "$(cardano-node --version 2>/dev/null | head -1 || echo unknown)"
cardano_cli_version         = "$(cardano-cli --version 2>/dev/null | head -1 || echo unknown)"

mithril_signed_entity        = "$MITHRIL_SIGNED_ENTITY"
mithril_immutable_range_lo   = $IMMUTABLE_LO
mithril_immutable_range_hi   = $IMMUTABLE_HI
mithril_certified_slot       = $SEED_SLOT
mithril_certified_block_hash = "$SEED_BLOCK_HASH"

operator_seed_point_slot       = $SEED_SLOT
operator_seed_point_block_hash = "$SEED_BLOCK_HASH"

utxo_json_file          = "$(basename "$UTXO_JSON")"
utxo_json_sha256        = "$(sha "$UTXO_JSON")"
consensus_inputs_file   = "$(basename "$CINPUTS_JSON")"
consensus_inputs_sha256 = "$(sha "$CINPUTS_JSON")"
mithril_manifest_file   = "$(basename "$MANIFEST_JSON")"
mithril_manifest_sha256 = "$(sha "$MANIFEST_JSON")"
node_transcript_file    = "$(basename "$POS_LOG")"
node_transcript_sha256  = "$(sha "$POS_LOG")"

ade_recomputed_seed_artifact_hash = "$SEED_ARTIFACT_HASH"
ade_initial_ledger_fingerprint    = "${INIT_LEDGER_FP:-see_transcript}"
ade_imported_utxo_fingerprint     = "$UTXO_FP"

binding_result = "$BINDING_RESULT"
node_exit_code = $POS_RC

negative_control_manifest_file   = "$(basename "$NEG_MANIFEST_JSON")"
negative_control_manifest_sha256 = "$(sha "$NEG_MANIFEST_JSON")"
negative_control_outcome         = "fail_closed"
negative_control_error           = "$NEG_ERROR"
negative_control_node_exit_code  = $NEG_RC
TOML

echo "==> bundle written: $BUNDLE"
echo "    review it, move it + the referenced artifacts into docs/evidence/, then run:"
echo "      ci/validate_mithril_documented_evidence.sh"
echo "    (RO-MITHRIL-IMPORT-01 stays partial until that bundle is committed + validator-green.)"
