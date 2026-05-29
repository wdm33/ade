#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-Y S1 — Mithril import enters the single closed bootstrap
# authority only (CN-MITHRIL-01 / CE-Y-3), and never re-verifies the
# STM multisig in BLUE (DC-MITHRIL-01).
#
# Mechanical guards:
#   1. Positive: the workspace references `bootstrap_initial_state(`
#      (the single closed bootstrap authority is the storage-init
#      chokepoint — Mithril-sourced state has no separate path).
#   2. Negative: no `trait *Anchor` (no GenesisAnchor / MithrilAnchor
#      plugin seam — `BootstrapAnchor` stays a struct).
#   3. Negative: the mithril-import shell declares no second
#      storage-init authority (no `pub fn bootstrap_initial_state`
#      anywhere but the one authority module).
#   4. Negative: no `mithril` / STM-verify crate import under any BLUE
#      crate path (the mithril-client verifies the STM signature; BLUE
#      never re-verifies it or treats it as a trust root).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES="$REPO_ROOT/crates"
BOOTSTRAP_AUTHORITY="$CRATES/ade_runtime/src/bootstrap.rs"

# BLUE crate source roots (authoritative core).
BLUE_PATHS=(
    "$CRATES/ade_codec"
    "$CRATES/ade_types"
    "$CRATES/ade_crypto"
    "$CRATES/ade_core"
    "$CRATES/ade_ledger"
    "$CRATES/ade_plutus"
)

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1"
}

# Rule 1: positive — bootstrap_initial_state( referenced in workspace.
if ! grep -rqE '\bbootstrap_initial_state\(' "$CRATES" --include='*.rs'; then
    print_fail "no reference to bootstrap_initial_state( — Mithril-sourced state must enter the single closed authority"
fi

# Rule 2: negative — no `trait *Anchor` (no plugin seam).
ANCHOR_TRAITS=$(grep -rnE '\btrait +[A-Za-z0-9_]*Anchor\b' "$CRATES" --include='*.rs' || true)
if [[ -n "$ANCHOR_TRAITS" ]]; then
    print_fail "found an *Anchor trait (no GenesisAnchor/MithrilAnchor plugin seam allowed):"
    echo "$ANCHOR_TRAITS"
fi

# Rule 3: negative — single bootstrap_initial_state authority.
EXTRA_AUTHORITY=()
for f in $(find "$CRATES" -type f -name '*.rs'); do
    if [[ "$f" == "$BOOTSTRAP_AUTHORITY" ]]; then continue; fi
    body=$(strip_for_grep "$f")
    if echo "$body" | grep -qE '\bpub fn bootstrap_initial_state\b'; then
        EXTRA_AUTHORITY+=("$f")
    fi
done
if (( ${#EXTRA_AUTHORITY[@]} > 0 )); then
    print_fail "second pub fn bootstrap_initial_state (no second storage-init path) in:"
    for f in "${EXTRA_AUTHORITY[@]}"; do echo "  $f"; done
fi

# Rule 4: negative — no mithril / STM-verify crate import under BLUE.
for blue in "${BLUE_PATHS[@]}"; do
    [[ -d "$blue" ]] || continue
    HITS=$(grep -rniE '\b(use|extern +crate) +.*\b(mithril|mithril_client|mithril_stm|stm)\b' \
        "$blue" --include='*.rs' || true)
    if [[ -n "$HITS" ]]; then
        print_fail "mithril/STM import under BLUE crate path $blue (STM verification is RED-only):"
        echo "$HITS"
    fi
done

if (( FAILED == 0 )); then
    echo "OK: Mithril import enters the single closed bootstrap authority; no plugin seam; no STM verify in BLUE (CN-MITHRIL-01 / DC-MITHRIL-01)"
fi
exit $FAILED
