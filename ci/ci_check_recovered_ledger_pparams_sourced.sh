#!/usr/bin/env bash
set -euo pipefail

# CE-G-A-2a (PHASE4-N-F-G-A S2a): the recovered ledger's protocol_params are
# sourced from the operator consensus-inputs bundle's oracle preimage at the
# forge-capable seed import — never ProtocolParameters::default() / genesis-
# initial. This gate asserts the wiring is present and fails closed if a future
# change reverts the forge recovered-ledger to a defaulted protocol_params.
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SEED="crates/ade_node/src/admission/seed_to_snapshot.rs"
BOOT="crates/ade_node/src/admission/bootstrap.rs"
CANON="crates/ade_runtime/src/consensus_inputs/canonical.rs"

for f in "$SEED" "$BOOT" "$CANON"; do
    if [[ ! -f "$f" ]]; then
        echo "FAIL: $f not found"
        exit 1
    fi
done

# --- build_seed_ledger installs the caller-supplied current pparams -----------
if ! grep -qE 'pub fn build_seed_ledger\(utxo: UTxOState, protocol_params: ProtocolParameters\)' "$SEED"; then
    echo "FAIL: build_seed_ledger does not take a protocol_params argument ($SEED)"
    exit 1
fi
if ! grep -qE '^\s*ledger\.protocol_params = protocol_params;' "$SEED"; then
    echo "FAIL: build_seed_ledger does not install the supplied protocol_params ($SEED)"
    exit 1
fi

# Negative: build_seed_ledger's production body must NOT default protocol_params.
# (Drop the #[cfg(test)] module first, then strip line/doc comments.)
SEED_PROD="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$SEED" | sed -E 's://.*::')"
if grep -qE 'protocol_params.*ProtocolParameters::default\(\)' <<<"$SEED_PROD"; then
    echo "FAIL: build_seed_ledger production body defaults protocol_params ($SEED)"
    exit 1
fi

# --- the forge-capable bootstrap binds + installs the current pparams ----------
if ! grep -qE 'require_forge_current_pparams\(\)' "$BOOT"; then
    echo "FAIL: forge bootstrap does not call require_forge_current_pparams ($BOOT)"
    exit 1
fi
if ! grep -qE '^\s*ledger\.protocol_params = current_pparams' "$BOOT"; then
    echo "FAIL: forge bootstrap runner ledger does not install current_pparams ($BOOT)"
    exit 1
fi
if ! grep -qE 'current_pparams\.clone\(\)' "$BOOT"; then
    echo "FAIL: forge bootstrap does not thread current_pparams into seed_to_snapshot ($BOOT)"
    exit 1
fi

# --- the bind is fail-closed (absent preimage / hash mismatch) ----------------
if ! grep -qE 'pub fn require_forge_current_pparams' "$CANON"; then
    echo "FAIL: require_forge_current_pparams not defined ($CANON)"
    exit 1
fi
for variant in PreimageAbsent BindMismatch; do
    if ! grep -qE "ForgeCurrentPParamsError::$variant" "$CANON"; then
        echo "FAIL: require_forge_current_pparams missing fail-closed variant $variant ($CANON)"
        exit 1
    fi
done

echo "OK: recovered-ledger protocol_params are sourced from the oracle bundle preimage, fail-closed (CE-G-A-2a)"
exit 0
