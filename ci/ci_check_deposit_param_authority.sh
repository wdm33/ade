#!/usr/bin/env bash
set -uo pipefail

# DC-TXV-07: canonical Conway deposit-parameter authority.
#
# Conway value-conservation accounting must source every deposit/refund amount
# (key_deposit, pool_deposit, drep_deposit, gov_action_deposit) from canonical
# ledger state — `ProtocolParameters.{key_deposit,pool_deposit}` plus
# `LedgerState.conway_deposit_params` — and NEVER from:
#   - the testkit `ConwayGovParams` object (RED snapshot-decode intermediate),
#   - `ProtocolParameters::default()` (fallback genesis defaults),
#   - literal deposit constants written next to a deposit field, or
#   - env / shell configuration.
#
# This greps the BLUE crate sources and fails on any such non-canonical
# deposit-param read. The legitimate write path (RED snapshot loader populating
# `conway_deposit_params` from parsed snapshot bytes) lives in ade_testkit,
# which is NOT a BLUE crate and is therefore out of scope here.

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core" "ade_ledger" "ade_plutus")

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FAILED=0

# Strip comment-only lines so prose mentioning a forbidden name (e.g. a
# docstring describing the prohibition) does not trip the gate.
strip_comments() {
    grep -v ':[0-9]*:[[:space:]]*//' || true
}

# Drop matches that fall inside a `#[cfg(test)]` module. Test fixtures may
# legitimately construct deposit params from literals as test inputs; the
# authority rule constrains the BLUE production read path. Input lines are
# `path:lineno:content`; we re-read each file to decide test-module membership.
drop_test_lines() {
    awk -F: '
    {
        file = $1; line = $2;
        if (file != cur) {
            cur = file; delete intest; n = 0;
            depth = 0; in_test = 0;
            while ((getline l < file) > 0) {
                n++;
                if (l ~ /#\[cfg\(test\)\]/) { pending = 1; }
                else if (pending && l ~ /mod[ \t]/) { in_test = 1; depth = 0; pending = 0; }
                if (in_test) {
                    o = gsub(/{/, "{", l); c = gsub(/}/, "}", l);
                    depth += o - c;
                    intest[n] = 1;
                    if (depth <= 0 && (o > 0 || c > 0)) { in_test = 0; }
                }
            }
            close(file);
        }
        if (!(line in intest)) print $0;
    }' || true
}

for crate in "${BLUE_CRATES[@]}"; do
    SRC_DIR="$REPO_ROOT/crates/$crate/src"
    [ -d "$SRC_DIR" ] || continue

    # 1. No BLUE path may name the testkit ConwayGovParams type at all — it is
    #    the RED snapshot-decode intermediate, never a BLUE read path.
    m=$(grep -rn "ConwayGovParams" "$SRC_DIR" --include='*.rs' 2>/dev/null | strip_comments || true)
    if [ -n "$m" ]; then
        echo "FAIL ($crate): BLUE path references testkit ConwayGovParams (deposit params must come from canonical state):"
        echo "$m"
        FAILED=1
    fi

    # 2. No deposit field may be sourced from ProtocolParameters::default().
    #    Match a default() construction on the same line as a deposit field.
    m=$(grep -rn "ProtocolParameters::default" "$SRC_DIR" --include='*.rs' 2>/dev/null \
        | strip_comments \
        | grep -E "drep_deposit|gov_action_deposit|key_deposit|pool_deposit" || true)
    if [ -n "$m" ]; then
        echo "FAIL ($crate): deposit param sourced from ProtocolParameters::default():"
        echo "$m"
        FAILED=1
    fi

    # 3. No literal deposit constant assigned to a Conway deposit field. The
    #    canonical Conway-only deposit params are read from
    #    `conway_deposit_params`, never written from an inline constant in BLUE.
    m=$(grep -rEn "(drep_deposit|gov_action_deposit)[[:space:]]*[:=][[:space:]]*(Coin\()?[0-9]" \
        "$SRC_DIR" --include='*.rs' 2>/dev/null | strip_comments \
        | grep -v "conway_deposit_params" | drop_test_lines || true)
    if [ -n "$m" ]; then
        echo "FAIL ($crate): Conway deposit field assigned a literal constant (must read canonical state):"
        echo "$m"
        FAILED=1
    fi

    # 4. No deposit param sourced from environment / shell config.
    m=$(grep -rEn "(std::)?env::(var|var_os)" "$SRC_DIR" --include='*.rs' 2>/dev/null \
        | strip_comments \
        | grep -E "deposit" || true)
    if [ -n "$m" ]; then
        echo "FAIL ($crate): deposit param sourced from environment/shell config:"
        echo "$m"
        FAILED=1
    fi
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: Conway deposit params sourced only from canonical ledger state (DC-TXV-07)"
    exit 0
else
    exit 1
fi
