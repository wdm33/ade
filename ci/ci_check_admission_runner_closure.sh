#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-B S4 — admission runner sole-authority closure
# (CN-ADMIT-01).
#
# The admission runner is THE composition seam between N-M-A's
# storage stack (BootstrapAnchor + WAL + seed importer) and N-L's
# wire stack (N2nDialer + chain-sync). It MUST remain a single
# pub fn / single canonical input bundle so future slices cannot
# silently fork the admission path.
#
# Mechanical guards:
#   1. Exactly one `pub async fn run_admission` across the workspace.
#   2. Exactly one `pub struct AdmissionInputs` across the workspace.
#   3. The closed exit-code constants
#      (EXIT_LIVE_AGREEMENT_DIVERGED = 30,
#       EXIT_LIVE_INPUT_NOT_FOUND = 31,
#       EXIT_LIVE_WAL_APPEND_IO = 33)
#      are defined exactly once.
#   4. `AdmissionInputs` carries no `Option` fields (no soft
#      defaults — every field is required).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNNER="$REPO_ROOT/crates/ade_node/src/admission/runner.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$RUNNER" ]]; then
    print_fail "missing $RUNNER"
    exit "$FAILED"
fi

# Guard 1: sole pub async fn run_admission.
sites=$(grep -rn --include='*.rs' -E '^pub async fn run_admission\b' "$REPO_ROOT/crates" 2>/dev/null || true)
n=$(echo "$sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n" -ne 1 ]]; then
    print_fail "expected exactly 1 pub async fn run_admission, found $n:"
    echo "$sites"
fi

# Guard 2: sole pub struct AdmissionInputs.
ai_sites=$(grep -rn --include='*.rs' -E '^pub struct AdmissionInputs\b' "$REPO_ROOT/crates" 2>/dev/null || true)
n_ai=$(echo "$ai_sites" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n_ai" -ne 1 ]]; then
    print_fail "expected exactly 1 pub struct AdmissionInputs, found $n_ai:"
    echo "$ai_sites"
fi

# Guard 3: exit-code constants defined exactly once.
for k in EXIT_LIVE_AGREEMENT_DIVERGED EXIT_LIVE_INPUT_NOT_FOUND EXIT_LIVE_WAL_APPEND_IO; do
    defs=$(grep -rn --include='*.rs' -E "^pub const ${k}\s*:\s*i32" "$REPO_ROOT/crates" 2>/dev/null || true)
    nd=$(echo "$defs" | grep -c -v '^$' 2>/dev/null || echo 0)
    if [[ "$nd" -ne 1 ]]; then
        print_fail "expected exactly 1 definition of $k, found $nd:"
        echo "$defs"
    fi
done

# Guard 4: each fixed exit-code constant has the registered value.
expected_pairs=(
    "EXIT_LIVE_AGREEMENT_DIVERGED:30"
    "EXIT_LIVE_INPUT_NOT_FOUND:31"
    "EXIT_LIVE_WAL_APPEND_IO:33"
)
for pair in "${expected_pairs[@]}"; do
    k="${pair%%:*}"
    v="${pair##*:}"
    if ! grep -qE "pub const ${k}\s*:\s*i32\s*=\s*${v}\s*;" "$RUNNER"; then
        print_fail "constant $k must equal $v in $RUNNER"
    fi
done

# Guard 5: AdmissionInputs has no Option<_> fields (closed inputs,
# no soft defaults — caller must supply everything).
opt_fields=$(awk '
    /^pub struct AdmissionInputs/ { capture=1; next }
    capture && /^}/ { exit }
    capture && /Option</ { print NR ": " $0 }
' "$RUNNER" || true)
if [[ -n "$opt_fields" ]]; then
    print_fail "AdmissionInputs carries Option<_> fields (must be all-required):"
    echo "$opt_fields"
fi

# Guard 6: the runner module must NOT carry #[non_exhaustive] on
# AdmissionExitCode or AdmissionPeerEvent (closed sums).
if grep -nE '#\[non_exhaustive\]' "$RUNNER" 2>/dev/null | head -1 > /dev/null; then
    while IFS=':' read -r lineno _rest; do
        next=$((lineno + 1))
        next_line=$(awk "NR==$next" "$RUNNER")
        if echo "$next_line" | grep -qE 'pub enum (AdmissionExitCode|AdmissionPeerEvent)'; then
            print_fail "runner sum carries #[non_exhaustive]: $RUNNER:$lineno"
        fi
    done < <(grep -nE '#\[non_exhaustive\]' "$RUNNER" 2>/dev/null)
fi

if (( FAILED == 0 )); then
    echo "OK: sole pub async fn run_admission + sole AdmissionInputs + closed exit codes + closed sums"
fi
exit $FAILED
