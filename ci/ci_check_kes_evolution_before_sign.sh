#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-AC S1 — KES signing evolves key to current period (DC-CRYPTO-10).
#
# The forge's real KES sign MUST evolve the operator KES key to the requested
# period BEFORE signing, via kes_sign_header_advancing -> kes_advance_to ->
# kes_update (the deterministic Sum6KES update), passing the period VERBATIM; and
# kes_update MUST keep its fail-closed backwards + exhausted guards. Signing stays
# RED (the standing ci_check_no_signing_in_blue.sh is the BLUE fence). Fences:
#   (a) the forge real KES sign uses kes_sign_header_advancing, NOT the raw
#       kes_sign_header / kes_sign_at;
#   (b) kes_sign_header_advancing evolves (kes_advance_to(period)) before signing,
#       period passed verbatim (no period +/- N);
#   (c) kes_update retains the EvolutionBackwards + EvolutionExhausted guards.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PRODUCE="$REPO_ROOT/crates/ade_node/src/produce_mode.rs"
SHELL_RS="$REPO_ROOT/crates/ade_runtime/src/producer/producer_shell.rs"
SIGNING="$REPO_ROOT/crates/ade_runtime/src/producer/signing.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$PRODUCE" "$SHELL_RS" "$SIGNING"; do
    [[ -f "$f" ]] || { echo "FAIL: missing $f"; exit 1; }
done

# Production view: strip // comments + stop at the first #[cfg(test)] module.
strip_prod() {
    awk '/^[[:space:]]*#\[cfg\(test\)\]/ { exit } { l=$0; sub(/\/\/.*$/, "", l); print l }' "$1"
}
PROD_PRODUCE="$(strip_prod "$PRODUCE")"
PROD_SHELL="$(strip_prod "$SHELL_RS")"
PROD_SIGNING="$(strip_prod "$SIGNING")"

# Guard (a) — the forge real KES sign uses the EVOLVING variant.
if ! echo "$PROD_PRODUCE" | grep -qE 'kes_sign_header_advancing[[:space:]]*\('; then
    print_fail "produce_mode forge does not use kes_sign_header_advancing (the evolve-then-sign variant)"
fi
if echo "$PROD_PRODUCE" | grep -qE '\.kes_sign_header[[:space:]]*\('; then
    print_fail "produce_mode forge still calls the non-evolving .kes_sign_header( — must use kes_sign_header_advancing:"
    echo "$PROD_PRODUCE" | grep -nE '\.kes_sign_header[[:space:]]*\('
fi
if echo "$PROD_PRODUCE" | grep -qE '\.kes_sign_at[[:space:]]*\('; then
    print_fail "produce_mode forge calls the raw .kes_sign_at( — must use kes_sign_header_advancing"
fi

# Guard (b) — the advancing method evolves BEFORE signing, period verbatim.
if ! echo "$PROD_SHELL" | grep -qE 'fn kes_sign_header_advancing'; then
    print_fail "ProducerShell::kes_sign_header_advancing is missing"
fi
if ! echo "$PROD_SHELL" | grep -qE 'kes_advance_to[[:space:]]*\([[:space:]]*period[[:space:]]*\)'; then
    print_fail "kes_sign_header_advancing does not call kes_advance_to(period) (verbatim) before signing"
fi
if echo "$PROD_SHELL" | grep -qE 'kes_advance_to[[:space:]]*\([[:space:]]*period[[:space:]]*[-+]|kes_sign_(header|at)[[:space:]]*\([[:space:]]*period[[:space:]]*[-+]'; then
    print_fail "manual period mutation (period +/- N) on the KES sign path — the requested period must be passed verbatim:"
    echo "$PROD_SHELL" | grep -nE 'kes_(advance_to|sign_header|sign_at)[[:space:]]*\([[:space:]]*period[[:space:]]*[-+]'
fi

# Guard (c) — kes_update retains the fail-closed guards.
if ! echo "$PROD_SIGNING" | grep -qE 'EvolutionBackwards'; then
    print_fail "kes_update lost the EvolutionBackwards (period < current) fail-closed guard"
fi
if ! echo "$PROD_SIGNING" | grep -qE 'EvolutionExhausted'; then
    print_fail "kes_update lost the EvolutionExhausted (beyond key lifetime / unreachable) fail-closed guard"
fi

if (( FAILED == 0 )); then
    echo "OK: forge KES sign evolves to the requested period via kes_sign_header_advancing (verbatim); kes_update fail-closed backwards/exhausted intact (DC-CRYPTO-10)"
fi
exit $FAILED
