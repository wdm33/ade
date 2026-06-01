#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-A S1 — genesis-consistency pinning fixture + harness presence.
#
# Backs CE-G-A-1 (genesis-consistency). The S1 pinning harness must be hermetic:
# it consumes ONLY the committed S1b private-net Ade-as-leader reference fixture
# (no Docker / cardano-cli / live node). This gate proves the fixture + harness
# are committed and well-formed, and that no secret key material leaked into the
# committed fixture (S1b containment).
#
# Guards:
#   1. The three fixture files are committed (consensus-inputs.json,
#      shelley-genesis.json, PROVENANCE.md).
#   2. The bundle is well-formed and Ade-as-leader: epoch_nonce_hex (eta0),
#      active_slots_coeff{numer,denom}, a NON-EMPTY pool_distribution, and a
#      pool_vrf_keyhashes entry for each pool. eta0 == genesis_hash_hex
#      (genesis-derived initial nonce, cross-validated at extraction).
#   3. NO secret key material is committed in the fixture dir (no *.skey /
#      *.vkey files; no cardano-cli "...SigningKey..." envelope).
#   4. The GREEN harness module exists, is wired into ade_testkit, embeds the
#      fixture via include_str!, and defines the four named pinning tests.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FIXDIR="$REPO_ROOT/crates/ade_testkit/fixtures/nfg_a_privnet_reference"
BUNDLE="$FIXDIR/consensus-inputs.json"
GENESIS="$FIXDIR/shelley-genesis.json"
PROVENANCE="$FIXDIR/PROVENANCE.md"
HARNESS="$REPO_ROOT/crates/ade_testkit/src/consensus/genesis_pinning.rs"
CONSENSUS_MOD="$REPO_ROOT/crates/ade_testkit/src/consensus/mod.rs"

FAIL=0
print_fail() {
    echo "FAIL: $1"
    FAIL=1
}

# --- Guard 1: fixture files present -----------------------------------------
for f in "$BUNDLE" "$GENESIS" "$PROVENANCE" "$HARNESS" "$CONSENSUS_MOD"; do
    [ -f "$f" ] || print_fail "Guard 1 (expected file missing: $f)"
done
[ "$FAIL" -eq 0 ] || exit 1

# --- Guard 2: bundle well-formed + Ade-as-leader ----------------------------
G2=$(python3 - "$BUNDLE" <<'PYEOF'
import json, sys
d = json.load(open(sys.argv[1]))
def fail(m):
    print("BAD: " + m); sys.exit(1)
for k in ("epoch_nonce_hex", "genesis_hash_hex", "active_slots_coeff",
          "pool_distribution", "pool_vrf_keyhashes", "epoch_no"):
    if k not in d:
        fail("missing field " + k)
eta0 = d["epoch_nonce_hex"]
if not (isinstance(eta0, str) and len(eta0) == 64):
    fail("epoch_nonce_hex must be 32-byte hex")
if d["genesis_hash_hex"] != eta0:
    fail("eta0 must equal genesis_hash_hex (genesis-derived initial nonce)")
asc = d["active_slots_coeff"]
if not (isinstance(asc, dict) and asc.get("denom", 0) > 0 and 0 < asc.get("numer", 0) <= asc["denom"]):
    fail("active_slots_coeff must be a valid numer/denom fraction")
dist = d["pool_distribution"]
vrfs = d["pool_vrf_keyhashes"]
if not dist:
    fail("pool_distribution must be non-empty (Ade pool must hold stake)")
for pool, entry in dist.items():
    if int(entry.get("active_stake", 0)) <= 0:
        fail("pool %s has non-positive active_stake" % pool)
    if pool not in vrfs or len(vrfs[pool]) != 64:
        fail("pool %s missing a 32-byte vrf keyhash" % pool)
print("OK")
PYEOF
)
[ $? -eq 0 ] || print_fail "Guard 2 (bundle shape): $G2"

# --- Guard 3: no secret key material committed ------------------------------
if find "$FIXDIR" -type f \( -name '*.skey' -o -name '*.vkey' -o -name '*.counter' \) | grep -q .; then
    print_fail "Guard 3 (a key file is committed in the fixture dir)"
fi
# No cardano-cli signing-key envelope in the committed JSON files.
if grep -rlE '"type"[[:space:]]*:[[:space:]]*"[^"]*SigningKey' "$FIXDIR"/*.json 2>/dev/null | grep -q .; then
    print_fail "Guard 3 (a SigningKey envelope is committed in a fixture JSON)"
fi

# --- Guard 4: harness wired + embeds fixture + names the four tests ----------
if ! grep -qE 'pub mod genesis_pinning;' "$CONSENSUS_MOD"; then
    print_fail "Guard 4 (genesis_pinning not wired into consensus/mod.rs)"
fi
if ! grep -qE 'include_str!\(\s*"\.\./\.\./fixtures/nfg_a_privnet_reference/consensus-inputs\.json"' "$HARNESS"; then
    print_fail "Guard 4 (harness does not embed the committed fixture via include_str!)"
fi
REQUIRED_TESTS=(
    "pinning_recovered_eta0_matches_genesis_fixture"
    "pinning_recovered_stake_asc_vrf_matches_genesis_fixture"
    "pinning_preseed_warmstart_roundtrip_faithful"
    "pinning_praos_vrf_input_and_threshold_match_fixture"
)
for t in "${REQUIRED_TESTS[@]}"; do
    if ! grep -qE "fn $t\b" "$HARNESS"; then
        print_fail "Guard 4 (pinning test $t missing from genesis_pinning.rs)"
    fi
done

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: genesis-consistency fixture + harness present, Ade-as-leader, secrets-free (4/4 guards)"
    exit 0
else
    exit 1
fi
