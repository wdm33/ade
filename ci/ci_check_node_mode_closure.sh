#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-B S5 — node mode dispatch closure (CN-NODE-01).
# Repaired PHASE4-N-F-C L1: the mode set grew past {WireOnly, Admission}
# to {WireOnly, Admission, KeyGenKes (N-O), Produce (N-Q), Node (N-F-C)};
# this gate had gone stale-RED on `main` because it still pinned the old
# two-variant set. It now pins the full closed set and requires every
# variant to have an explicit main.rs dispatch arm with no wildcard.
#
# `Mode` is a closed sum in `crates/ade_node/src/cli.rs`. This gate
# asserts:
#   1. The sum's variant set is EXACTLY
#      { WireOnly, Admission, KeyGenKes, Produce, Node } (doc-comments
#      between variants are ignored).
#   2. The sum does NOT carry `#[non_exhaustive]`.
#   3. main.rs's `match cli.mode { ... }` covers every variant by name
#      with no wildcard arm (which would silently swallow a new variant).
#   4. There is exactly ONE call to `dispatch_admission(` (the binary entry).
#   5. There is exactly ONE call to `run_wire_only(` in main.rs.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLI_FILE="$REPO_ROOT/crates/ade_node/src/cli.rs"
MAIN_FILE="$REPO_ROOT/crates/ade_node/src/main.rs"

EXPECTED_VARIANTS="Admission KeyGenKes Node Produce WireOnly"  # sorted

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$CLI_FILE" ]] || [[ ! -f "$MAIN_FILE" ]]; then
    print_fail "missing cli.rs or main.rs"
    exit "$FAILED"
fi

# Guard 1: Mode enum variant set is EXACTLY the expected closed set.
# Capture the enum body, drop comment lines (`//` / `///`) and blanks,
# then take the leading identifier of each remaining line as a variant.
mode_block=$(awk '
    /^pub enum Mode \{$/ { capture=1; next }
    capture && /^\}/ { exit }
    capture { print }
' "$CLI_FILE")
if [[ -z "$mode_block" ]]; then
    print_fail "Mode enum block not found in $CLI_FILE"
else
    got_variants=$(echo "$mode_block" \
        | sed 's://.*$::' \
        | grep -vE '^\s*$' \
        | grep -oE '^\s*[A-Z][A-Za-z0-9]*' \
        | tr -d ' ' \
        | sort \
        | tr '\n' ' ' \
        | sed 's/ *$//')
    expected_sorted=$(echo "$EXPECTED_VARIANTS" | tr ' ' '\n' | sort | tr '\n' ' ' | sed 's/ *$//')
    if [[ "$got_variants" != "$expected_sorted" ]]; then
        print_fail "Mode variant set mismatch."
        echo "  expected: $expected_sorted"
        echo "  got:      $got_variants"
    fi
fi

# Guard 2: Mode must NOT be #[non_exhaustive].
while IFS=':' read -r lineno _rest; do
    [[ -z "$lineno" ]] && continue
    next=$((lineno + 1))
    next_line=$(awk "NR==$next" "$CLI_FILE")
    if echo "$next_line" | grep -qE 'pub enum Mode\b'; then
        print_fail "Mode carries #[non_exhaustive]: $CLI_FILE:$lineno"
    fi
done < <(grep -nE '#\[non_exhaustive\]' "$CLI_FILE" 2>/dev/null)

# Guard 3: main.rs `match cli.mode` covers every variant, no wildcard.
main_match=$(awk '
    /match cli\.mode \{/ { capture=1; next }
    capture && /^\s*\}\s*$/ { exit }
    capture { print }
' "$MAIN_FILE")
if [[ -z "$main_match" ]]; then
    print_fail "match cli.mode block missing from $MAIN_FILE"
else
    for v in WireOnly Admission KeyGenKes Produce Node; do
        if ! echo "$main_match" | grep -qE "^\s*Mode::${v}\s*=>"; then
            print_fail "main.rs mode dispatch missing arm for Mode::$v"
        fi
    done
    if echo "$main_match" | grep -qE '^\s*_\s*=>'; then
        print_fail "main.rs mode dispatch uses wildcard arm (would silently swallow a new variant)"
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

# Guard 5: exactly one binary call to run_wire_only( in main.rs.
wo_calls=$(grep -nE 'run_wire_only\s*\(' "$MAIN_FILE" 2>/dev/null || true)
n_wo=$(echo "$wo_calls" | grep -c -v '^$' 2>/dev/null || echo 0)
if [[ "$n_wo" -ne 1 ]]; then
    print_fail "expected exactly 1 run_wire_only() call in main.rs, found $n_wo:"
    echo "$wo_calls"
fi

if (( FAILED == 0 )); then
    echo "OK: Mode is closed {WireOnly, Admission, KeyGenKes, Produce, Node}; main.rs dispatches all arms without wildcard"
fi
exit $FAILED
