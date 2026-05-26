#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-B S3 — admission must not silently skip reference scripts
# (DC-ADMIT-09).
#
# A1 shipped `JsonSeedError::UnsupportedTxOutFeature { feature:
# "referenceScript" }` as a fail-fast guard so admission cannot
# bootstrap from a UTxO seed that includes a reference-script
# output (those outputs require Conway script-decode authority that
# A1.1 will add). Sub-cluster B must NOT add any permissive
# matching that bypasses this guard:
#
#   - no `JsonSeedError::UnsupportedTxOutFeature ... => continue|skip|()|Ok(_)`
#   - no `if entry.reference_script.is_some() { continue }`
#   - no `// TODO: refscript` / `// FIXME: refscript` / `// HACK: refscript`
#   - no second `pub fn import_*_utxo*` shadowing the seed importer.
#
# Mechanical guards:
#   1. Inside the admission code paths (`crates/ade_node/src/admission/`,
#      `crates/ade_runtime/src/seed_import/`), grep for the forbidden
#      shapes and fail if any match.
#   2. Confirm `crates/ade_runtime/src/seed_import/importer.rs` still
#      contains the literal A1 fail-fast guard, so no future commit
#      can remove it without tripping this gate.
#   3. Confirm exactly one `pub fn import_cardano_cli_json_utxo`
#      survives.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SEED_IMPORTER="$REPO_ROOT/crates/ade_runtime/src/seed_import/importer.rs"
ADMISSION_DIR="$REPO_ROOT/crates/ade_node/src/admission"
SEED_DIR="$REPO_ROOT/crates/ade_runtime/src/seed_import"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

# Guard 1a: forbidden permissive shapes in admission + seed code.
forbidden_arms=$(
    {
        grep -rnE 'UnsupportedTxOutFeature[^=]*=>[^,]*(continue|skip|_ /\* skip \*/|\(\)|Ok\(_)' \
            "$ADMISSION_DIR" "$SEED_DIR" 2>/dev/null || true
        grep -rnE 'reference_script\.is_some\(\)[^=]*=>[^,]*(continue|skip)' \
            "$ADMISSION_DIR" "$SEED_DIR" 2>/dev/null || true
        grep -rnE 'if[^{]+reference_script[^{]+\{[^}]*continue' \
            "$ADMISSION_DIR" "$SEED_DIR" 2>/dev/null || true
    }
)

if [[ -n "$forbidden_arms" ]]; then
    print_fail "admission / seed_import code path treats reference-script as skippable:"
    echo "$forbidden_arms"
fi

# Guard 1b: forbidden refscript TODO/FIXME/HACK markers.
refscript_todos=$(
    grep -rnE '//\s*(TODO|FIXME|HACK)[: ].*ref.?script' \
        "$ADMISSION_DIR" "$SEED_DIR" 2>/dev/null || true
)
if [[ -n "$refscript_todos" ]]; then
    print_fail "refscript TODO/FIXME/HACK in admission or seed-import:"
    echo "$refscript_todos"
fi

# Guard 2: the A1 fail-fast guard is still present verbatim.
if [[ ! -f "$SEED_IMPORTER" ]]; then
    print_fail "missing $SEED_IMPORTER"
else
    if ! grep -qE 'UnsupportedTxOutFeature[[:space:]]*\{[[:space:]]*$' "$SEED_IMPORTER" && \
       ! grep -qE 'UnsupportedTxOutFeature[[:space:]]*\{[[:space:]]*feature:[[:space:]]*"referenceScript"' "$SEED_IMPORTER"; then
        print_fail "A1 fail-fast guard for referenceScript missing from $SEED_IMPORTER"
    fi
    if ! grep -qE 'entry\.reference_script\.is_some\(\)' "$SEED_IMPORTER"; then
        print_fail "A1 reference_script.is_some() check missing from $SEED_IMPORTER"
    fi
fi

# Guard 3: exactly one pub fn import_cardano_cli_json_utxo across the workspace.
sites=$(grep -rn --include='*.rs' -E '^pub fn import_cardano_cli_json_utxo\b' "$REPO_ROOT/crates" 2>/dev/null || true)
n_sites=$(echo "$sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n_sites" -ne 1 ]]; then
    print_fail "expected exactly 1 pub fn import_cardano_cli_json_utxo, found $n_sites:"
    echo "$sites"
fi

# Guard 4: exactly one pub fn seed_to_snapshot across the workspace.
ss_sites=$(grep -rn --include='*.rs' -E '^pub fn seed_to_snapshot\b' "$REPO_ROOT/crates" 2>/dev/null || true)
n_ss=$(echo "$ss_sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n_ss" -ne 1 ]]; then
    print_fail "expected exactly 1 pub fn seed_to_snapshot, found $n_ss:"
    echo "$ss_sites"
fi

if (( FAILED == 0 )); then
    echo "OK: admission + seed_import never skip reference scripts; A1 fail-fast intact; sole seed_to_snapshot authority"
fi
exit $FAILED
