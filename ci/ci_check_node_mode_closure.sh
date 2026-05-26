#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-B S5 — node mode dispatch closure (CN-NODE-01
# strengthening for admission dispatch).
#
# `Mode` is a closed sum in `crates/ade_node/src/cli.rs`. This gate
# asserts:
#   1. The sum has the registered variant set: { WireOnly, Admission }.
#   2. The sum does NOT carry `#[non_exhaustive]`.
#   3. main.rs's `match cli.mode { ... }` is exhaustive at the
#      source level and covers both variants by name (no wildcard
#      arm allowed — that would silently swallow a new variant).
#   4. There is exactly ONE call to `dispatch_admission(` across
#      the workspace (the binary entry).
#   5. There is exactly ONE call to `run_wire_only(` across the
#      workspace (the binary entry; tests pass through it).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLI_FILE="$REPO_ROOT/crates/ade_node/src/cli.rs"
MAIN_FILE="$REPO_ROOT/crates/ade_node/src/main.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$CLI_FILE" ]] || [[ ! -f "$MAIN_FILE" ]]; then
    print_fail "missing cli.rs or main.rs"
    exit "$FAILED"
fi

# Guard 1: Mode enum variant set.
mode_block=$(awk '
    /^pub enum Mode \{$/ { capture=1; next }
    capture && /^\}/ { exit }
    capture { print }
' "$CLI_FILE")
if [[ -z "$mode_block" ]]; then
    print_fail "Mode enum block not found in $CLI_FILE"
else
    for v in WireOnly Admission; do
        if ! echo "$mode_block" | grep -qE "^\s*${v}\s*,?\s*$"; then
            print_fail "Mode variant '$v' missing from cli.rs"
        fi
    done
    extras=$(echo "$mode_block" | grep -vE '^\s*(WireOnly|Admission)\s*,?\s*$' | grep -vE '^\s*$' || true)
    if [[ -n "$extras" ]]; then
        print_fail "Mode has unrecognized variants:"
        echo "$extras"
    fi
fi

# Guard 2: Mode must NOT be #[non_exhaustive].
if grep -nE '#\[non_exhaustive\]' "$CLI_FILE" 2>/dev/null | head -1 > /dev/null; then
    while IFS=':' read -r lineno _rest; do
        next=$((lineno + 1))
        next_line=$(awk "NR==$next" "$CLI_FILE")
        if echo "$next_line" | grep -qE 'pub enum Mode\b'; then
            print_fail "Mode carries #[non_exhaustive]: $CLI_FILE:$lineno"
        fi
    done < <(grep -nE '#\[non_exhaustive\]' "$CLI_FILE" 2>/dev/null)
fi

# Guard 3: main.rs match must cover both Mode variants explicitly
# and not use a wildcard fallthrough.
main_match=$(awk '
    /match cli\.mode \{/ { capture=1; next }
    capture && /^\s*\}/ { exit }
    capture { print }
' "$MAIN_FILE")
if [[ -z "$main_match" ]]; then
    print_fail "match cli.mode block missing from $MAIN_FILE"
else
    for v in 'Mode::WireOnly' 'Mode::Admission'; do
        if ! echo "$main_match" | grep -qE "^\s*${v}\s*=>"; then
            print_fail "main.rs mode dispatch missing arm for $v"
        fi
    done
    if echo "$main_match" | grep -qE '^\s*_\s*=>'; then
        print_fail "main.rs mode dispatch uses wildcard arm (would silently swallow new variant)"
    fi
fi

# Guard 4: exactly one call to dispatch_admission.
ds_calls=$(grep -rn --include='*.rs' -E 'dispatch_admission\s*\(' "$REPO_ROOT/crates" 2>/dev/null \
    | grep -v -E '(fn |pub fn |pub async fn )dispatch_admission' || true)
n_ds=$(echo "$ds_calls" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n_ds" -ne 1 ]]; then
    print_fail "expected exactly 1 dispatch_admission() call, found $n_ds:"
    echo "$ds_calls"
fi

# Guard 5: exactly one binary call to run_wire_only(&cli, ...) in
# main.rs (tests call it through the lib re-export and are
# explicitly skipped via path filter).
wo_calls=$(grep -nE 'run_wire_only\s*\(' "$MAIN_FILE" 2>/dev/null || true)
n_wo=$(echo "$wo_calls" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n_wo" -ne 1 ]]; then
    print_fail "expected exactly 1 run_wire_only() call in main.rs, found $n_wo:"
    echo "$wo_calls"
fi

if (( FAILED == 0 )); then
    echo "OK: Mode is closed {WireOnly, Admission}; main.rs dispatches both arms without wildcard; sole dispatch_admission call"
fi
exit $FAILED
