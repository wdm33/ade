#!/usr/bin/env bash
set -euo pipefail

# C2 public-live pre-launch STAKE-EQUALITY GATE (operator-side, RED).
# Venue-parametric: --network preview|preprod (default preprod).
#
# Bounty-critical: before Ade forges with its pool identity against the venue
# node, this gate proves Ade's leader-election view (the extracted
# consensus-input bundle) AGREES with the reference node's leader-election `set`
# snapshot. Per cardano-ledger, THIS epoch's slot-leader election uses the `set`
# snapshot (`go` is the rewards snapshot, one epoch behind; `mark` is not yet
# active). If the bundle does not match `set`, Ade would win slots the node
# rejects (false readiness). See docs/active/c2-public-live-acceptance-runbook.md §1.
#
# Usage:
#   ci/check_ade1_leader_stake_active.sh [--network preview|preprod] <consensus-inputs-bundle.json>
#   Preview requires the pool id via env (no preprod default leaks into preview):
#     ADE1_POOL_HEX=<28-byte hex> ADE1_POOL_BECH=<pool1...> ... --network preview <bundle>
#
# Exit 0  iff  pool stakeSet > 0  AND  |bundle_sigma(pool) - setFraction(pool)| / setFraction < EPSILON.
# Exit 3  = pool stakeSet == 0  (stake not active for THIS epoch's leader election).
# Exit 4  = pool leader-fraction mismatch (extractor stake source != leader-election `set`).
# Exit 2  = usage / query error.
#
# It ALSO prints a whole-distribution consistency sample (bundle_sigma vs
# setFraction across established pools) so the extractor's stake source can be
# validated even when the pool itself is not yet active.

NETWORK="preprod"
BUNDLE=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --network) NETWORK="${2:-}"; shift 2 ;;
        -h|--help) echo "Usage: $0 [--network preview|preprod] <consensus-inputs-bundle.json>"; exit 0 ;;
        --*) echo "FATAL: unknown flag '$1'" >&2; exit 2 ;;
        *) BUNDLE="$1"; shift ;;
    esac
done
[[ -n "$BUNDLE" && -f "$BUNDLE" ]] || { echo "Usage: $0 [--network preview|preprod] <consensus-inputs-bundle.json>" >&2; exit 2; }

# Venue defaults (env overrides win). The Preview pool id is deliberately NOT
# defaulted — it is a different ledger identity and must be supplied.
case "$NETWORK" in
    preprod) DEF_CONTAINER="cardano-node-preprod"; DEF_MAGIC="1"
             DEF_HEX="4590e0ee152ca3325b1cb00118ff02f1394b136ed8b2a23cfec8b070"
             DEF_BECH="pool1gkgwpms49j3nykcukqq33lcz7yu5kymwmze2y087ezc8qqpt397" ;;
    preview) DEF_CONTAINER="cardano-node-preview"; DEF_MAGIC="2"
             DEF_HEX=""; DEF_BECH="" ;;
    *) echo "FATAL: --network must be 'preview' or 'preprod' (got '$NETWORK')" >&2; exit 2 ;;
esac
CONTAINER="${ADE_LIVE_PEER_CONTAINER:-$DEF_CONTAINER}"
MAGIC="${ADE_LIVE_NETWORK_MAGIC:-$DEF_MAGIC}"
SOCKET="${ADE_LIVE_PEER_SOCKET:-/ipc/node.socket}"
ADE_HEX="${ADE1_POOL_HEX:-$DEF_HEX}"
ADE_BECH="${ADE1_POOL_BECH:-$DEF_BECH}"
EPSILON="${ADE_STAKE_EPSILON:-0.02}"   # 2% tolerance on the leader-fraction match
if [[ -z "$ADE_HEX" || -z "$ADE_BECH" ]]; then
    echo "FATAL: pool id required for --network $NETWORK; set ADE1_POOL_HEX + ADE1_POOL_BECH" >&2; exit 2
fi

run_cli() { docker exec "$CONTAINER" sh -c "export CARDANO_NODE_SOCKET_PATH=$SOCKET; cardano-cli $*"; }

echo "=== C2 stake-equality gate (bundle: $BUNDLE) ==="

# 1. ADE1 stake-snapshot: set + total set.
ADE_SNAP=$(run_cli "query stake-snapshot --stake-pool-id $ADE_BECH --testnet-magic $MAGIC")

# 2. All-pools set (best-effort: validates the extractor's stake source across the
#    whole distribution). Falls back to ADE1-only if --all-stake-pools is unsupported.
ALL_SNAP=$(run_cli "query stake-snapshot --all-stake-pools --testnet-magic $MAGIC" 2>/dev/null || echo "")

python3 - "$BUNDLE" "$ADE_HEX" "$EPSILON" "$ADE_SNAP" "$ALL_SNAP" <<'PYEOF'
import json, sys

bundle_path, ade_hex, epsilon, ade_snap_s, all_snap_s = sys.argv[1:6]
epsilon = float(epsilon)

bundle = json.load(open(bundle_path))
pd = bundle["pool_distribution"]
bundle_total = sum(p["active_stake"] for p in pd.values())
def bundle_sigma(h):
    p = pd.get(h)
    return (p["active_stake"] / bundle_total) if (p and bundle_total) else None

ade_snap = json.loads(ade_snap_s)
def snap_pools(d):
    # cardano-cli shapes: {"pools":{hex:{stakeSet..}}, "total":{stakeSet..}} OR
    # a flat {hex:{stakeSet..}} with a "total" sibling.
    pools = d.get("pools", {k: v for k, v in d.items() if k != "total"})
    total = d.get("total", {})
    return pools, total

ade_pools, ade_total = snap_pools(ade_snap)
ade_set = None
for k, v in ade_pools.items():
    if k.lower() == ade_hex.lower():
        ade_set = int(v.get("stakeSet", 0))
total_set = int(ade_total.get("stakeSet", 0)) if ade_total else 0
mark = go = None
for k, v in ade_pools.items():
    if k.lower() == ade_hex.lower():
        mark, go = int(v.get("stakeMark", 0)), int(v.get("stakeGo", 0))

print(f"ADE1 stake-snapshot: mark={mark} set={ade_set} go={go}  (total set={total_set})")
print("  (leader election uses `set`; `go` is the rewards snapshot, one epoch behind)")
ade_set_frac = (ade_set / total_set) if (ade_set and total_set) else 0.0
ade_bundle_sigma = bundle_sigma(ade_hex)
print(f"ADE1 setFraction = {ade_set_frac:.3e}   bundle_sigma = "
      + (f"{ade_bundle_sigma:.3e}" if ade_bundle_sigma is not None else "(ABSENT from bundle)"))

# Whole-distribution consistency sample (extractor stake-source validation).
if all_snap_s.strip():
    try:
        ap, at = snap_pools(json.loads(all_snap_s))
        ts = int(at.get("stakeSet", 0)) or sum(int(v.get("stakeSet", 0)) for v in ap.values())
        rows = []
        for h, v in ap.items():
            s = int(v.get("stakeSet", 0))
            if s <= 0: continue
            bf = bundle_sigma(h)
            if bf is None: continue
            sf = s / ts
            rel = abs(bf - sf) / sf if sf else float("inf")
            rows.append(rel)
        if rows:
            rows.sort()
            agree = sum(1 for r in rows if r < epsilon)
            med = rows[len(rows)//2]
            print(f"\nExtractor stake-source check over {len(rows)} pools with set>0: "
                  f"{agree}/{len(rows)} within {epsilon:.0%} (median rel-err {med:.2%})")
            if agree < 0.9 * len(rows):
                print("  WARNING: bundle sigma disagrees with leader-election `set` for many pools "
                      "-> build_consensus_inputs_bundle.sh is NOT sourcing the `set` snapshot.")
            else:
                print("  OK: bundle sigma tracks leader-election `set` for established pools "
                      "(ADE1's own mismatch, if any, is the registration transient).")
    except Exception as e:
        print(f"\n(all-pools consistency sample skipped: {e})")
else:
    print("\n(all-pools snapshot unavailable; ADE1-only gate)")

# The gate verdict.
if not ade_set or ade_set <= 0:
    print("\nGATE: ADE1 stakeSet == 0 -> NOT active for this epoch's leader election. "
          "Do NOT launch (expected before ~epoch 296, when the faucet delegation reaches `set`).")
    sys.exit(3)
if ade_bundle_sigma is None:
    print("\nGATE: ADE1 absent from the bundle -> extractor did not include the pool. ABORT.")
    sys.exit(4)
rel = abs(ade_bundle_sigma - ade_set_frac) / ade_set_frac if ade_set_frac else float("inf")
if rel >= epsilon:
    print(f"\nGATE: ADE1 bundle_sigma vs setFraction rel-err {rel:.2%} >= {epsilon:.0%} "
          "-> extractor stake source != leader-election `set`. ABORT + fix the extractor.")
    sys.exit(4)
print(f"\nGATE PASS: ADE1 set>0 and bundle_sigma matches setFraction (rel-err {rel:.2%}). "
      "Ade's leader view agrees with the node; a forged ADE1 block CAN be accepted.")
PYEOF
