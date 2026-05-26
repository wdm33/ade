#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-M-C S5 — operator-side LiveConsensusInputs bundle
# generator.
#
# Reads cardano-node-preprod (docker container `cardano-node-preprod`
# at 127.0.0.1:3001) via `docker exec cardano-cli ...` queries,
# assembles the JSON envelope `LiveConsensusInputsRaw`
# (`ade_runtime::consensus_inputs::json::RawConsensusInputs`)
# expects, and writes it to the requested output path.
#
# Usage:
#   ci/build_consensus_inputs_bundle.sh <output.json>
#
# The bundle is RED / operational evidence — the BLUE admission
# path consumes only the canonicalized form
# (`LiveConsensusInputsCanonical`) produced by
# `ade_runtime::consensus_inputs::import_live_consensus_inputs`.
#
# Doctrine:
#   - The Shelley genesis hash is read from `.cardano-node-preprod
#     /config/config.json` (per the local preprod runbook).
#   - The active-slots-coefficient is `0.05` per
#     `.cardano-node-preprod/config/shelley-genesis.json`. The
#     bundle records the closed `numer=1, denom=20` form.
#   - Preprod network magic is `1`.
#   - The epoch length on preprod is `432000` slots; the bundle
#     records `[epoch_start_slot, epoch_end_slot]` as
#     `[tip_slot - slot_in_epoch, tip_slot - slot_in_epoch + 432000 - 1]`.

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <output.json>" >&2
    exit 2
fi
OUT="$1"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

CONTAINER="${ADE_LIVE_PEER_CONTAINER:-cardano-node-preprod}"
MAGIC="${ADE_LIVE_NETWORK_MAGIC:-1}"
SOCKET_PATH_INSIDE="${ADE_LIVE_PEER_SOCKET:-/opt/cardano/ipc/node.socket}"

run_cli() {
    docker exec "$CONTAINER" sh -c "cardano-cli $* --testnet-magic $MAGIC --socket-path $SOCKET_PATH_INSIDE"
}

# 1. Tip — hash + slot + slot-in-epoch.
TIP_JSON=$(run_cli query tip)
TIP_HASH=$(echo "$TIP_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["hash"])')
TIP_SLOT=$(echo "$TIP_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["slot"])')
EPOCH_NO=$(echo "$TIP_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["epoch"])')
SLOT_IN_EPOCH=$(echo "$TIP_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["slotInEpoch"])')
NODE_VERSION=$(docker exec "$CONTAINER" cardano-node --version 2>&1 | head -1)

EPOCH_START=$((TIP_SLOT - SLOT_IN_EPOCH))
EPOCH_END=$((EPOCH_START + 432000 - 1))

# 2. Protocol state for epoch nonce.
PROTOCOL_STATE=$(run_cli query protocol-state)
EPOCH_NONCE=$(echo "$PROTOCOL_STATE" | python3 -c 'import json,sys; print(json.load(sys.stdin)["epochNonce"])')

# 3. Stake distribution (sigma fractions).
STAKE_DISTR_JSON=$(run_cli query stake-distribution)

# 4. Pool state for VRF keyhashes.
POOL_STATE_JSON=$(run_cli query pool-state --all-stake-pools)

# 5. Protocol parameters — used for the bundle's
# `protocol_params_hash_hex` (blake2b-256 of canonical JSON).
PROTO_PARAMS_JSON=$(run_cli query protocol-parameters)

# 6. Genesis hash from local config.
GENESIS_HASH=$(python3 -c "
import json,sys
with open('${REPO_ROOT}/.cardano-node-preprod/config/config.json') as f:
    cfg = json.load(f)
print(cfg.get('ShelleyGenesisHash'))
")
if [[ -z "$GENESIS_HASH" || "$GENESIS_HASH" == "None" ]]; then
    echo "FATAL: ShelleyGenesisHash missing from preprod config.json" >&2
    exit 1
fi

# 7. Assemble the JSON envelope. Large blobs are written to
# temp files because argv has a length limit.
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
echo "$STAKE_DISTR_JSON" > "$TMP_DIR/stake_distr.json"
echo "$POOL_STATE_JSON" > "$TMP_DIR/pool_state.json"
echo "$PROTO_PARAMS_JSON" > "$TMP_DIR/proto_params.json"

python3 - "$OUT" \
    "$MAGIC" \
    "$GENESIS_HASH" \
    "$EPOCH_NO" \
    "$EPOCH_START" \
    "$EPOCH_END" \
    "$EPOCH_NONCE" \
    "$TIP_HASH" \
    "$TIP_SLOT" \
    "$NODE_VERSION" \
    "$TMP_DIR/stake_distr.json" \
    "$TMP_DIR/pool_state.json" \
    "$TMP_DIR/proto_params.json" <<'PYEOF'
import hashlib
import json
import sys

(
    out_path,
    magic,
    genesis_hash,
    epoch_no,
    epoch_start,
    epoch_end,
    epoch_nonce,
    tip_hash,
    tip_slot,
    node_version,
    stake_distr_path,
    pool_state_path,
    proto_params_path,
) = sys.argv[1:]

with open(stake_distr_path) as f:
    stake_distr = json.load(f)
with open(pool_state_path) as f:
    pool_state = json.load(f)
with open(proto_params_path) as f:
    proto_params = json.load(f)

# stake-distribution returns bech32 pool IDs ("pool1...") with
# sigma {numerator, denominator}. The bundle expects hex pool IDs
# (28-byte) with active_stake as an integer.
def bech32_decode_pool_id(p_str):
    # cardano pool IDs are Bech32 with HRP "pool".
    import importlib
    try:
        bech32 = importlib.import_module("bech32")
    except ModuleNotFoundError:
        raise SystemExit(
            "FATAL: python3 'bech32' package not available — apt install python3-bech32 or pip install bech32"
        )
    hrp, data = bech32.bech32_decode(p_str)
    if hrp != "pool" or data is None:
        raise ValueError(f"bad pool bech32: {p_str}")
    decoded = bech32.convertbits(data, 5, 8, False)
    return bytes(decoded).hex()

total_stake = sum(int(v["numerator"]) * (1_000_000_000_000_000_000 // max(1, int(v["denominator"]))) for v in stake_distr.values())

# Build pool_distribution: hex_id -> {active_stake}
# pool_state already keys by hex pool ID; cross-reference sigma
# via bech32-decoded keys from stake_distr.
sd_by_hex = {}
for p_bech, sigma in stake_distr.items():
    try:
        hex_id = bech32_decode_pool_id(p_bech)
    except Exception as e:
        # Skip pools we can't decode (operator-side anomaly).
        continue
    sd_by_hex[hex_id] = sigma

pool_distribution = {}
pool_vrf_keyhashes = {}
for hex_id, ps in pool_state.items():
    vrf = ps.get("poolParams", {}).get("spsVrf")
    if not vrf:
        continue
    sigma = sd_by_hex.get(hex_id)
    if sigma is None:
        # Pool in pool-state but not in stake-distribution; skip.
        continue
    # Convert sigma {num,denom} -> active_stake (lovelace) by
    # scaling to the network's mainnet-equivalent active-stake
    # base of 30B ADA = 3e16 lovelace. The exact base is
    # implementation-detail: the bundle's pool active_stake is
    # only used for is_leader fraction comparisons; the ratio
    # matters, not the absolute base.
    num = int(sigma["numerator"])
    denom = int(sigma["denominator"])
    if denom == 0:
        continue
    base = 30_000_000_000_000_000  # 3e16
    active = (num * base) // denom
    pool_distribution[hex_id] = {"active_stake": active}
    pool_vrf_keyhashes[hex_id] = vrf

# protocol_params_hash: blake2b-256 of a canonical JSON dump.
canonical_pp = json.dumps(proto_params, sort_keys=True, separators=(",", ":")).encode()
proto_params_hash = hashlib.blake2b(canonical_pp, digest_size=32).hexdigest()

bundle = {
    "network_magic": int(magic),
    "genesis_hash_hex": genesis_hash.lower(),
    "era": "conway",
    "epoch_no": int(epoch_no),
    "epoch_start_slot": int(epoch_start),
    "epoch_end_slot": int(epoch_end),
    "active_slots_coeff": {"numer": 1, "denom": 20},
    "epoch_nonce_hex": epoch_nonce.lower(),
    "pool_distribution": pool_distribution,
    "pool_vrf_keyhashes": pool_vrf_keyhashes,
    "protocol_params_hash_hex": proto_params_hash,
    "source_cardano_node_version": node_version,
    "source_query_command": (
        "docker exec cardano-node-preprod cardano-cli query "
        "{tip,protocol-state,stake-distribution,pool-state,protocol-parameters}"
    ),
    "source_tip_hash_hex": tip_hash.lower(),
    "source_tip_slot": int(tip_slot),
}

with open(out_path, "w") as f:
    json.dump(bundle, f, indent=2, sort_keys=False)
print(f"wrote {out_path}: {len(pool_distribution)} pools, epoch {epoch_no}, tip slot {tip_slot}")
PYEOF
