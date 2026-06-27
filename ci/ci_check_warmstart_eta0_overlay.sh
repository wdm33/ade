#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-N (T-REC-04 / DC-CINPUT-03): the WarmStart-recovered forge eta0 comes from the persisted
# seed-epoch consensus sidecar, never the snapshot placeholder / genesis.
#   (A) SeedEpochConsensusInputs carries an explicit epoch_nonce field.
#   (B) the sidecar codec is versioned (SEED_CINPUT_SCHEMA_VERSION = 5) and fail-closed (UnknownVersion) —
#       an old sidecar (no eta0 / no seed bootstrap point) must NOT default-to-zero.
#   (C) the admission merge persists epoch_nonce from the imported LiveConsensusInputsCanonical.
#   (D) bootstrap_initial_state overlays the recovered sidecar epoch_nonce onto chain_dep (+ evolving_nonce).
#   (E) the regression test exists.
#   (F) NO VRF variant migration (still vrf-draft03; no vrf-draft13/batch-compat) — the fix is eta0 sourcing.
#   (G) T-REC-04 + DC-CINPUT-03 enforced in the registry.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCI="$REPO_ROOT/crates/ade_ledger/src/seed_consensus_inputs.rs"
MERGE="$REPO_ROOT/crates/ade_runtime/src/seed_consensus_merge.rs"
BOOT="$REPO_ROOT/crates/ade_runtime/src/bootstrap.rs"
CRYPTO_TOML="$REPO_ROOT/crates/ade_crypto/Cargo.toml"
REG="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$SCI" "$MERGE" "$BOOT" "$CRYPTO_TOML" "$REG"; do
    [[ -f "$f" ]] || print_fail "missing expected file $f"
done

# (A) explicit epoch_nonce field on the persisted sidecar.
grep -Eq 'pub epoch_nonce: Nonce' "$SCI" \
    || print_fail "(A) SeedEpochConsensusInputs has no explicit epoch_nonce field"

# (B) versioned + fail-closed.
grep -Eq 'SEED_CINPUT_SCHEMA_VERSION: u32 = 5' "$SCI" \
    || print_fail "(B) SEED_CINPUT_SCHEMA_VERSION not bumped to 5 (versioned schema change)"
grep -Eq 'UnknownVersion' "$SCI" \
    || print_fail "(B) decode no longer fail-closes on an unknown/old schema version"
grep -Eq 'fn seed_cinput_decode_rejects_unknown_version' "$SCI" \
    || print_fail "(B) the fail-closed version test is missing"

# (C) merge persists the imported eta0.
grep -Eq 'epoch_nonce: canonical\.epoch_nonce' "$MERGE" \
    || print_fail "(C) merge_seed_epoch_consensus_inputs does not persist canonical.epoch_nonce"

# (D) WarmStart overlays the recovered sidecar eta0 onto chain_dep (sourced from the sidecar, not a literal).
grep -Eq 'chain_dep\.epoch_nonce = sidecar\.epoch_nonce' "$BOOT" \
    || print_fail "(D) bootstrap_initial_state does not overlay the recovered sidecar epoch_nonce onto chain_dep"
grep -Eq 'chain_dep\.evolving_nonce = sidecar\.epoch_nonce' "$BOOT" \
    || print_fail "(D) the seed-epoch evolving_nonce overlay is missing"

# (E) the regression test exists.
grep -Eq 'fn warm_start_overlays_recovered_eta0_onto_chain_dep_g_n' "$BOOT" \
    || print_fail "(E) the G-N overlay regression test is missing"

# (F) NO VRF variant migration: still draft-03, and ade_crypto must NOT enable a draft-13/batch-compat VRF.
grep -Eq 'vrf-draft03' "$CRYPTO_TOML" \
    || print_fail "(F) ade_crypto no longer enables vrf-draft03 (variant drift)"
if grep -Eq 'vrf-draft13|batch.?compat' "$CRYPTO_TOML"; then
    print_fail "(F) ade_crypto enables a draft-13/batch-compat VRF — G-N forbids a VRF variant migration"
fi

# (G) both rules present and enforced in the registry.
for rule in "T-REC-04" "DC-CINPUT-03"; do
    awk -v r="$rule" '$0 ~ ("id = \"" r "\""){f=1} f&&/status = "enforced"/{print "ok"; exit}' "$REG" | grep -q ok \
        || print_fail "(G) $rule not present-and-enforced in the registry"
done

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_warmstart_eta0_overlay: FAILED"
    exit 1
fi
echo "ci_check_warmstart_eta0_overlay: OK (T-REC-04 / DC-CINPUT-03 — WarmStart forge eta0 from the seed-epoch sidecar)"
