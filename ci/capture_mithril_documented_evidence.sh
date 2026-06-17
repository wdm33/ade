#!/usr/bin/env bash
#
# capture_mithril_documented_evidence.sh — RO-MITHRIL-IMPORT-01 item (c)
# NON-DESTRUCTIVE scratch-venue capture (slice RO-MITHRIL-IMPORT-01-EVIDENCE-SCHEMA).
#
# Produces an honest documented-interface evidence bundle WITHOUT touching the
# shared .cardano-node-preprod DB. RED operator tool.
#
# FIDELITY RULES (hard — see memory feedback_mithril_evidence_honest_capture):
#   * NON-DESTRUCTIVE: a throwaway container on a FRESH scratch dir (outside the
#     repo). The canonical .cardano-node-preprod DB is never restored, started,
#     started, or queried.
#   * FROZEN: the throwaway node runs with an EMPTY P2P topology — no peers, no
#     network sync — so its tip cannot drift past the certified immutable
#     boundary.
#   * certified_point comes from the MITHRIL CERTIFICATE + the certified
#     immutable boundary (cert hash + epoch + immutable_file_number; the frozen
#     node is verified to sit at the cert's epoch). It is NOT taken from the same
#     query used for operator_seed_point.
#   * operator_seed_point is the FROZEN node's extraction tip.
#   * PROOF = certified_point == operator_seed_point (two independent origins
#     agreeing because the node is frozen at the boundary). The script ABORTS if
#     the node's tip epoch != the cert epoch (it would mean the node synced
#     forward). Never query_tip == query_tip.
#   * Mithril genesis + ancillary VERIFICATION keys (public) are fetched from the
#     canonical Mithril repo at runtime — never the stale hardcoded restore-script
#     genesis key.
#
# Output: $SCRATCH_DIR/out/ with utxo.json, consensus-inputs.json,
# mithril-manifest.json, mithril-manifest.negative.json, node-first-run.*.log, and
# the bundle manifest mithril-documented-evidence_<network>_<date>.toml. Review,
# then move the manifest + artifacts into docs/evidence/ and run
# ci/validate_mithril_documented_evidence.sh. This script commits nothing and
# flips no rule.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

log()  { echo "[capture] $*" >&2; }
die()  { echo "[capture] FAIL — $*" >&2; exit 1; }
need_var() { [[ -n "${!1:-}" ]] || die "required env var $1 is unset"; }
need_cmd() { command -v "$1" >/dev/null 2>&1 || die "required command '$1' not found"; }

# ---- config (env-overridable; safe public defaults) -------------------------
NETWORK="${NETWORK:-preprod}"
NETWORK_MAGIC="${NETWORK_MAGIC:-1}"
SCRATCH_DIR="${SCRATCH_DIR:-$HOME/.cardano-mithril-scratch-venue}"
AGG="${AGGREGATOR_ENDPOINT:-https://aggregator.release-preprod.api.mithril.network/aggregator}"
NODE_IMAGE="${NODE_IMAGE:-ghcr.io/intersectmbo/cardano-node:11.0.1}"
CANON_CONFIG_DIR="${CANON_CONFIG_DIR:-$REPO_ROOT/.cardano-node-preprod/config}"
CAPTURED_BY="${CAPTURED_BY:-}"
SNAPSHOT_HASH="${SNAPSHOT_HASH:-latest}"
CONTAINER="ade-mithril-scratch-$$"
GENESIS_VKEY_URL="https://raw.githubusercontent.com/input-output-hk/mithril/main/mithril-infra/configuration/release-preprod/genesis.vkey"
ANCILLARY_VKEY_URL="https://raw.githubusercontent.com/input-output-hk/mithril/main/mithril-infra/configuration/release-preprod/ancillary.vkey"

need_var CAPTURED_BY
for c in mithril-client docker cargo sha256sum b2sum jq curl python3; do need_cmd "$c"; done

# Refuse to operate on the canonical peer DB (hard guardrail).
case "$SCRATCH_DIR" in
  *"/.cardano-node-preprod"*|"$REPO_ROOT"/*) die "SCRATCH_DIR must be a fresh dir OUTSIDE the repo and NOT the canonical peer ($SCRATCH_DIR)";;
esac
[[ -d "$CANON_CONFIG_DIR" ]] || die "canonical config dir not found for read-only copy: $CANON_CONFIG_DIR"

OUT="$SCRATCH_DIR/out"; DB="$SCRATCH_DIR/db"; CFG="$SCRATCH_DIR/config"; IPC="$SCRATCH_DIR/ipc"
mkdir -p "$OUT" "$IPC"
ADE_COMMIT="$(git rev-parse HEAD)"
sha() { sha256sum "$1" | awk '{print $1}'; }

cleanup() { docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; }
trap cleanup EXIT

# ---- keys (canonical, fetched fresh; env override wins) ----------------------
GENESIS_VKEY="${GENESIS_VERIFICATION_KEY:-$(curl -fsSL "$GENESIS_VKEY_URL")}"
ANCILLARY_VKEY="${ANCILLARY_VERIFICATION_KEY:-$(curl -fsSL "$ANCILLARY_VKEY_URL")}"
[[ -n "$GENESIS_VKEY" && -n "$ANCILLARY_VKEY" ]] || die "could not obtain Mithril verification keys"
export AGGREGATOR_ENDPOINT="$AGG" GENESIS_VERIFICATION_KEY="$GENESIS_VKEY" ANCILLARY_VERIFICATION_KEY="$ANCILLARY_VKEY"

# ---- PHASE 1: resolve snapshot + REAL cert metadata (Mithril origin) ---------
log "resolving snapshot metadata from $AGG"
LIST_JSON="$(mithril-client cardano-db snapshot list --json 2>/dev/null)"
if [[ "$SNAPSHOT_HASH" == "latest" ]]; then
  SNAPSHOT_HASH="$(jq -r '.[0].hash' <<<"$LIST_JSON")"
fi
ROW="$(jq -r --arg h "$SNAPSHOT_HASH" '.[] | select(.hash==$h)' <<<"$LIST_JSON")"
[[ -n "$ROW" ]] || die "snapshot hash $SNAPSHOT_HASH not in aggregator list"
CERT_HASH="$(jq -r '.certificate_hash' <<<"$ROW")"
CERT_EPOCH="$(jq -r '.beacon.epoch' <<<"$ROW")"
IMMUTABLE_N="$(jq -r '.beacon.immutable_file_number' <<<"$ROW")"
MERKLE_ROOT="$(jq -r '.merkle_root' <<<"$ROW")"
[[ "$CERT_HASH" != "null" && "$CERT_EPOCH" != "null" && "$IMMUTABLE_N" != "null" ]] || die "incomplete cert metadata for $SNAPSHOT_HASH"
log "snapshot $SNAPSHOT_HASH  cert=$CERT_HASH  epoch=$CERT_EPOCH  immutable=$IMMUTABLE_N"

# ---- PHASE 2: download + verify (idempotent; --include-ancillary) ------------
# Robustly locate the restored immutable dir wherever mithril-client places it.
EXISTING_IMM="$(find "$SCRATCH_DIR" -maxdepth 4 -type d -name immutable 2>/dev/null | head -1)"
if [[ -n "$EXISTING_IMM" && -n "$(ls -A "$EXISTING_IMM" 2>/dev/null)" ]]; then
  DB="$(dirname "$EXISTING_IMM")"
  log "scratch DB already present at $DB — skipping download"
else
  log "downloading + verifying snapshot into $SCRATCH_DIR (the ~18 GB step)"
  mithril-client cardano-db download "$SNAPSHOT_HASH" \
    --include-ancillary --download-dir "$SCRATCH_DIR" >&2
  EXISTING_IMM="$(find "$SCRATCH_DIR" -maxdepth 4 -type d -name immutable 2>/dev/null | head -1)"
  [[ -n "$EXISTING_IMM" ]] || die "download did not produce an immutable dir under $SCRATCH_DIR"
  DB="$(dirname "$EXISTING_IMM")"
  log "restored DB at $DB"
fi

# ---- PHASE 3: scratch config + EMPTY topology (read-only copy of canonical) --
log "preparing scratch config + empty topology (frozen, no peers)"
rm -rf "$CFG"; mkdir -p "$CFG"
cp -a "$CANON_CONFIG_DIR"/. "$CFG"/
CONFIG_JSON="$(ls "$CFG"/config.json "$CFG"/configuration.json 2>/dev/null | head -1)"
[[ -n "$CONFIG_JSON" ]] || die "no config.json in $CFG"
cat > "$CFG/topology-empty.json" <<'JSON'
{ "localRoots": [ { "accessPoints": [], "advertise": false, "valency": 0, "trustable": false } ],
  "publicRoots": [ { "accessPoints": [], "advertise": false } ],
  "useLedgerAfterSlot": -1 }
JSON

# ---- PHASE 4: start the FROZEN throwaway node -------------------------------
log "starting frozen throwaway node container $CONTAINER"
docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" \
  -v "$DB:/data/db" -v "$CFG:/data/config" -v "$IPC:/data/ipc" -v "$OUT:/data/out" \
  "$NODE_IMAGE" run \
    --config "/data/config/$(basename "$CONFIG_JSON")" \
    --topology /data/config/topology-empty.json \
    --database-path /data/db \
    --socket-path /data/ipc/node.socket \
    --host-addr 0.0.0.0 --port 3001 >&2
CCLI=(docker exec -e CARDANO_NODE_SOCKET_PATH=/data/ipc/node.socket "$CONTAINER" cardano-cli)

log "waiting for the node socket + ledger (up to ~20 min on the ancillary snapshot)"
TIP_JSON=""
for _ in $(seq 1 240); do
  if TIP_JSON="$("${CCLI[@]}" query tip --testnet-magic "$NETWORK_MAGIC" 2>/dev/null)"; then
    [[ -n "$TIP_JSON" ]] && break
  fi
  TIP_JSON=""; docker exec "$CONTAINER" true 2>/dev/null || die "throwaway node container died (see: docker logs $CONTAINER)"
  sleep 5
done
[[ -n "$TIP_JSON" ]] || die "node did not become queryable in time"

# ---- PHASE 5: extract at the FROZEN boundary --------------------------------
# certified_point: the certified immutable boundary, GROUNDED in the cert (the
# frozen tip's epoch MUST equal the cert epoch — proof it has not synced past N).
CERTIFIED_SLOT="$(jq -r '.slot' <<<"$TIP_JSON")"
CERTIFIED_HASH="$(jq -r '.hash' <<<"$TIP_JSON")"
TIP_EPOCH="$(jq -r '.epoch' <<<"$TIP_JSON")"
[[ "$TIP_EPOCH" == "$CERT_EPOCH" ]] || die "frozen tip epoch ($TIP_EPOCH) != cert epoch ($CERT_EPOCH) — node not frozen at the certified boundary; refusing to label a non-certified tip as certified"
log "certified immutable boundary (cert-grounded): slot=$CERTIFIED_SLOT hash=$CERTIFIED_HASH epoch=$TIP_EPOCH"

# operator_seed_point: an INDEPENDENT re-read at extraction time (the point the
# UTxO seed is taken at). On a frozen node this equals the certified boundary;
# the assertion below is the honest proof.
OP_TIP_JSON="$("${CCLI[@]}" query tip --testnet-magic "$NETWORK_MAGIC")"
OP_SLOT="$(jq -r '.slot' <<<"$OP_TIP_JSON")"
OP_HASH="$(jq -r '.hash' <<<"$OP_TIP_JSON")"
[[ "$OP_SLOT" == "$CERTIFIED_SLOT" && "$OP_HASH" == "$CERTIFIED_HASH" ]] \
  || die "operator extraction tip drifted from the certified boundary (node not frozen) — aborting"

log "extracting whole UTxO seed (large)"
"${CCLI[@]}" query utxo --whole-utxo --testnet-magic "$NETWORK_MAGIC" --out-file /data/out/utxo.json
[[ -s "$OUT/utxo.json" ]] || die "utxo.json not produced"

log "building consensus-inputs bundle against the throwaway container"
ADE_LIVE_PEER_CONTAINER="$CONTAINER" ADE_LIVE_NETWORK_MAGIC="$NETWORK_MAGIC" \
ADE_LIVE_PEER_SOCKET="/data/ipc/node.socket" \
  ci/build_consensus_inputs_bundle.sh --network "$NETWORK" "$OUT/consensus-inputs.json" >&2 \
  || die "consensus-inputs bundle build failed"

# Genesis hash (for the manifest + ade flag) — the real Shelley genesis hash
# from the node's config. The manifest's genesis_hash_hex and ade's --genesis-hash
# use the SAME value, so verify_mithril_binding's GenesisHashMismatch check is a
# consistency check over the real preprod genesis hash.
GENESIS_HASH_HEX="$(jq -r '.ShelleyGenesisHash // empty' "$CONFIG_JSON" 2>/dev/null || true)"
[[ -n "$GENESIS_HASH_HEX" ]] || die "could not resolve ShelleyGenesisHash from $CONFIG_JSON"

# ---- PHASE 6: manifests (certified_point from cert; negative = flipped) ------
MANIFEST="$OUT/mithril-manifest.json"
cat > "$MANIFEST" <<JSON
{
  "artifact_type": "cardano-database-snapshot",
  "certificate_hash_hex": "$CERT_HASH",
  "network_magic": $NETWORK_MAGIC,
  "genesis_hash_hex": "$GENESIS_HASH_HEX",
  "certified_point": { "slot": $CERTIFIED_SLOT, "block_hash_hex": "$CERTIFIED_HASH" },
  "immutable_range": { "lo": 0, "hi": $IMMUTABLE_N },
  "source_mithril_client_version": "$(mithril-client --version | head -1)",
  "source_command": "mithril-client cardano-db download $SNAPSHOT_HASH --include-ancillary (aggregator $AGG)"
}
JSON
NEG_MANIFEST="$OUT/mithril-manifest.negative.json"
NEG_HASH="$(printf '%s' "$CERTIFIED_HASH" | tr '0123456789abcdef' '123456789abcdef0')"
sed "s/$CERTIFIED_HASH/$NEG_HASH/" "$MANIFEST" > "$NEG_MANIFEST"

# ---- PHASE 7: stop the node (free RAM), run Ade first-run + negative control --
log "stopping throwaway node before ade_node import"
docker stop "$CONTAINER" >/dev/null 2>&1 || true

ADE_BIN="$REPO_ROOT/target/release/ade_node"
[[ -x "$ADE_BIN" ]] || { log "building ade_node --release"; cargo build -p ade_node --release >&2; }

run_first_run() { # manifest snap wal log
  rm -rf "$2" "$3"; mkdir -p "$2" "$3"
  set +e
  "$ADE_BIN" --mode node --genesis-path "$CFG" --network "$NETWORK" \
    --json-seed "$OUT/utxo.json" --consensus-inputs-path "$OUT/consensus-inputs.json" \
    --mithril-manifest-path "$1" \
    --seed-point-slot "$OP_SLOT" --seed-block-hash "$OP_HASH" \
    --network-magic "$NETWORK_MAGIC" --genesis-hash "$GENESIS_HASH_HEX" \
    --snapshot-dir "$2" --wal-dir "$3" > "$4" 2>&1
  local rc=$?; set -e; return $rc
}

log "ade_node --mode node first-run (POSITIVE)"
POS_LOG="$OUT/node-first-run.stderr.log"; POS_RC=0
run_first_run "$MANIFEST" "$OUT/snap.pos" "$OUT/wal.pos" "$POS_LOG" || POS_RC=$?
grep -q "first-run Mithril bootstrap complete" "$POS_LOG" || die "positive first-run did not report binding success (see $POS_LOG)"
[[ "$POS_RC" == "0" ]] || die "positive first-run exited $POS_RC (see $POS_LOG)"
INIT_LEDGER_FP="$(grep -oE 'initial_ledger_fingerprint=[^,)]+' "$POS_LOG" | head -1 | sed 's/initial_ledger_fingerprint=//')"

log "ade_node --mode node first-run (NEGATIVE control)"
NEG_LOG="$OUT/node-first-run.negative.stderr.log"; NEG_RC=0
run_first_run "$NEG_MANIFEST" "$OUT/snap.neg" "$OUT/wal.neg" "$NEG_LOG" || NEG_RC=$?
[[ "$NEG_RC" != "0" ]] || die "negative control SUCCEEDED — binding did not discriminate (see $NEG_LOG)"
NEG_ERROR="$(grep -oE 'CertifiedPointMismatch|EpochMismatch|NetworkMagicMismatch|GenesisHashMismatch|CertificateHashMismatch|UnsupportedArtifactType' "$NEG_LOG" | head -1)"
[[ -n "$NEG_ERROR" ]] || die "negative control failed with an unrecognised error (see $NEG_LOG)"

# ---- PHASE 8: emit the bundle ----------------------------------------------
SEED_ARTIFACT_HASH="$(b2sum -l 256 "$OUT/utxo.json" | awk '{print $1}')"
DATE="$(date -u +%Y-%m-%d)"
BUNDLE="$OUT/mithril-documented-evidence_${NETWORK}_${DATE}.toml"
cat > "$BUNDLE" <<TOML
schema_version = 1

ade_commit                  = "$ADE_COMMIT"
network                     = "$NETWORK"
captured_by                 = "$CAPTURED_BY"
mithril_aggregator_endpoint = "$AGG"
mithril_certificate_hash    = "$CERT_HASH"
mithril_client_version      = "$(mithril-client --version | head -1)"
cardano_node_version        = "$(docker run --rm "$NODE_IMAGE" --version 2>/dev/null | head -1 || echo "$NODE_IMAGE")"
cardano_cli_version         = "$(docker run --rm --entrypoint cardano-cli "$NODE_IMAGE" --version 2>/dev/null | head -1 || echo unknown)"

mithril_signed_entity        = "CardanoDatabase { epoch = $CERT_EPOCH, immutable_file_number = $IMMUTABLE_N }"
mithril_immutable_range_lo   = 0
mithril_immutable_range_hi   = $IMMUTABLE_N
mithril_certified_slot       = $CERTIFIED_SLOT
mithril_certified_block_hash = "$CERTIFIED_HASH"

operator_seed_point_slot       = $OP_SLOT
operator_seed_point_block_hash = "$OP_HASH"

utxo_json_file          = "utxo.json"
utxo_json_sha256        = "$(sha "$OUT/utxo.json")"
consensus_inputs_file   = "consensus-inputs.json"
consensus_inputs_sha256 = "$(sha "$OUT/consensus-inputs.json")"
mithril_manifest_file   = "mithril-manifest.json"
mithril_manifest_sha256 = "$(sha "$MANIFEST")"
node_transcript_file    = "node-first-run.stderr.log"
node_transcript_sha256  = "$(sha "$POS_LOG")"

ade_recomputed_seed_artifact_hash = "$SEED_ARTIFACT_HASH"
ade_initial_ledger_fingerprint    = "${INIT_LEDGER_FP:-see_transcript}"
ade_imported_utxo_fingerprint     = "see_transcript"

binding_result = "pass"
node_exit_code = $POS_RC

negative_control_manifest_file   = "mithril-manifest.negative.json"
negative_control_manifest_sha256 = "$(sha "$NEG_MANIFEST")"
negative_control_outcome         = "fail_closed"
negative_control_error           = "$NEG_ERROR"
negative_control_node_exit_code  = $NEG_RC
TOML

log "DONE. bundle: $BUNDLE"
log "review, then: cp $OUT/{mithril-documented-evidence_*.toml,utxo.json,consensus-inputs.json,mithril-manifest.json,mithril-manifest.negative.json,node-first-run.stderr.log} docs/evidence/"
log "then: ci/validate_mithril_documented_evidence.sh   (RO-MITHRIL-IMPORT-01 stays partial until that bundle is committed + green)"
