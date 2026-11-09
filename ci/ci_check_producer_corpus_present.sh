#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-C S7 — producer replay corpus presence + cross-impl adapter
# wiring + live-evidence binary presence + procedure doc + registry
# status. Closes CN-CONS-06 mechanical half.
#
# Mechanical guards (closure proof for CE-N-C-7 / CN-CONS-06 mechanical):
#
#   1. `producer_replay_fixtures()` is wired and exposes the three S3
#      fixtures: empty_mempool_leader, non_leader, two_tx_mempool_leader.
#   2. At least one fixture in fixtures.rs holds a non-empty
#      `expected_forged` (byte literal of a captured forged block).
#      Guarantees the positive byte-equality case is covered.
#   3. The operator-production entrypoint `ade_node --mode produce`
#      (cli.rs) parses its required inputs: the `produce` mode token +
#      the ProduceMissingFlag-validated flags (--listen / --cold-skey /
#      --kes-skey / --vrf-skey / --opcert / --genesis-file / --json-seed /
#      --consensus-inputs-path). Repointed from the superseded
#      live_block_production_session binary.
#   4. The operator runbook (docs/active/cn-cons-06-operator-runbook.md)
#      documents `ade_node --mode produce` + CN-CONS-06 / RO-LIVE-01.
#   5. The legacy entry CN-CONS-06 is carried to an active IDSE invariant
#      (PRC-023 LiveAcceptanceReleaseGate) that lists this gate and the three
#      named cross_impl_adapter tests, and whose claim is not `supported`
#      without passing evidence (the live half needs operator stake).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

FIXTURES_RS="$REPO_ROOT/crates/ade_testkit/src/producer/fixtures.rs"
REPLAY_RS="$REPO_ROOT/crates/ade_testkit/src/producer/replay.rs"
ADAPTER_RS="$REPO_ROOT/crates/ade_testkit/src/producer/cross_impl_adapter.rs"
CLI_RS="$REPO_ROOT/crates/ade_node/src/cli.rs"
RUNBOOK_MD="$REPO_ROOT/docs/active/cn-cons-06-operator-runbook.md"
REGISTRY="$REPO_ROOT/docs/ade-invariant-registry.toml"

FAIL=0

print_fail() {
    echo "FAIL: $1"
    FAIL=1
}

for f in "$FIXTURES_RS" "$REPLAY_RS" "$ADAPTER_RS" "$CLI_RS" "$RUNBOOK_MD" "$REGISTRY"; do
    [ -f "$f" ] || { print_fail "expected file missing: $f"; }
done
[ "$FAIL" -eq 0 ] || exit 1

# ---------------------------------------------------------------------------
# Guard 1 — replay corpus exposes the three S3 fixtures.
# ---------------------------------------------------------------------------
REQUIRED_FIXTURES=(
    "fixture_empty_mempool_leader"
    "fixture_non_leader"
    "fixture_two_tx_mempool_leader"
)
for fx in "${REQUIRED_FIXTURES[@]}"; do
    if ! grep -qE "pub fn $fx\b" "$FIXTURES_RS"; then
        print_fail "Guard 1 (fixture $fx is not exposed in fixtures.rs)"
    fi
done
# Replay harness must expose `producer_replay_fixtures()`.
if ! grep -qE 'pub fn producer_replay_fixtures\b' "$REPLAY_RS"; then
    print_fail "Guard 1 (producer_replay_fixtures() is not exposed in replay.rs)"
fi

# ---------------------------------------------------------------------------
# Guard 2 — at least one captured non-empty expected_forged byte literal.
# Looks for the named const `EXPECTED_FORGED_EMPTY_MEMPOOL_LEADER` carrying
# a non-trivial byte slice (more than the empty-literal `&[]` case).
# ---------------------------------------------------------------------------
if ! grep -qE 'pub const EXPECTED_FORGED_EMPTY_MEMPOOL_LEADER\s*:\s*&\[u8\]' "$FIXTURES_RS"; then
    print_fail "Guard 2 (EXPECTED_FORGED_EMPTY_MEMPOOL_LEADER byte literal not declared in fixtures.rs)"
fi
# A non-empty captured corpus means the const is bound to a `&[ ... 0x.. ]`
# literal containing at least a few dozen byte items. Use a coarse byte
# count (lines following the const decl up to the closing `];`) — must
# exceed 8 lines to be a real capture.
CAPTURED_LINES=$(awk '
    /pub const EXPECTED_FORGED_EMPTY_MEMPOOL_LEADER[[:space:]]*:[[:space:]]*&\[u8\][[:space:]]*=[[:space:]]*&\[/ { capture=1; next }
    capture && /^\];/ { exit }
    capture { print }
' "$FIXTURES_RS" | wc -l)
if [ "$CAPTURED_LINES" -lt 8 ]; then
    print_fail "Guard 2 (EXPECTED_FORGED_EMPTY_MEMPOOL_LEADER is suspiciously short ($CAPTURED_LINES lines) — positive fixture not properly captured)"
fi

# The cross_impl_adapter must be wired into the producer module.
if ! grep -qE 'pub mod cross_impl_adapter\b' "$REPO_ROOT/crates/ade_testkit/src/producer/mod.rs"; then
    print_fail "Guard 2 (cross_impl_adapter module not wired into ade_testkit::producer)"
fi
# And it must reference all three named tests.
REQUIRED_TESTS=(
    "cross_impl_adapter_forged_block_decodes_through_ade_codec"
    "cross_impl_adapter_forged_block_structurally_agrees_with_decoder"
    "cross_impl_adapter_corpus_round_trips_byte_identical"
)
for t in "${REQUIRED_TESTS[@]}"; do
    if ! grep -qE "fn $t\b" "$ADAPTER_RS"; then
        print_fail "Guard 2 (cross_impl_adapter test $t is missing from cross_impl_adapter.rs)"
    fi
done

# ---------------------------------------------------------------------------
# Guard 3 — the operator-production entrypoint (`ade_node --mode produce`)
# parses its required CLI inputs. Repointed at the live surface in cli.rs
# (was: the superseded live_block_production_session binary): the `produce`
# mode token + the ProduceMissingFlag-validated required flags.
# ---------------------------------------------------------------------------
if ! grep -qE '"produce"[[:space:]]*=>' "$CLI_RS"; then
    print_fail "Guard 3 (cli.rs does not recognize the \"produce\" mode token)"
fi
REQUIRED_PRODUCE_FLAGS=(
    "--listen"
    "--cold-skey"
    "--kes-skey"
    "--vrf-skey"
    "--opcert"
    "--genesis-file"
    "--json-seed"
    "--consensus-inputs-path"
)
for flag in "${REQUIRED_PRODUCE_FLAGS[@]}"; do
    if ! grep -qF "ProduceMissingFlag(\"$flag\")" "$CLI_RS"; then
        print_fail "Guard 3 (ade_node --mode produce does not require $flag via ProduceMissingFlag in cli.rs)"
    fi
done

# ---------------------------------------------------------------------------
# Guard 4 — the current operator runbook (file presence checked above)
# documents the live entrypoint. Repointed at
# docs/active/cn-cons-06-operator-runbook.md (was: the historical
# CE-N-C-8_PROCEDURE.md for the superseded binary).
# ---------------------------------------------------------------------------
if ! grep -qF "ade_node --mode produce" "$RUNBOOK_MD"; then
    print_fail "Guard 4 (operator runbook does not reference 'ade_node --mode produce')"
fi
if ! grep -qE 'CN-CONS-06|RO-LIVE-01' "$RUNBOOK_MD"; then
    print_fail "Guard 4 (operator runbook does not reference CN-CONS-06 / RO-LIVE-01)"
fi

# ---------------------------------------------------------------------------
# Guard 5 — CN-CONS-06 registry entry shape.
#   Either: status="enforced" AND tests=[...all three adapter tests...]
#   Or:     status="partial"  AND open_obligation mentions blocked_until_operator_stake_available
# ---------------------------------------------------------------------------
G5=$(python3 -I - "$REGISTRY" "$REPO_ROOT/docs/legacy/idd-to-idse-map.toml" <<'PYEOF'
import sys
import tomllib

rules = {r["id"]: r for r in tomllib.load(open(sys.argv[1], "rb"))["rules"]}
rows = {r["id"]: r for r in tomllib.load(open(sys.argv[2], "rb"))["legacy"]}

row = rows.get("CN-CONS-06")
if row is None or row["disposition"] != "migrated" or not row["new_ids"]:
    print(f"MISSING: CN-CONS-06 is not migrated in the legacy map ({row})")
    sys.exit(1)
targets = [rules.get(n) for n in row["new_ids"]]
if any(t is None or t.get("status") != "active" for t in targets):
    print(f"BAD: CN-CONS-06 maps to {row['new_ids']}, missing or not active")
    sys.exit(1)

required_tests = {
    "cross_impl_adapter_forged_block_decodes_through_ade_codec",
    "cross_impl_adapter_forged_block_structurally_agrees_with_decoder",
    "cross_impl_adapter_corpus_round_trips_byte_identical",
}
script = "ci/ci_check_producer_corpus_present.sh"
for t in targets:
    tests = set(t.get("tests", []))
    gated = script in t.get("ci_scripts", [])
    claim = t.get("claim_status", "")
    passing = any(e.get("result") == "pass" for e in t.get("evidence", []))
    if required_tests.issubset(tests) and gated and (claim != "supported" or passing):
        print("OK")
        sys.exit(0)
print(f"BAD: no target of CN-CONS-06 ({row['new_ids']}) lists the three tests and this gate with an honest claim")
sys.exit(1)
PYEOF
)
G5_RC=$?
if [ $G5_RC -ne 0 ]; then
    print_fail "Guard 5 (CN-CONS-06 carried to the IDSE registry):"
    echo "  $G5"
fi

if [ "$FAIL" -eq 0 ]; then
    echo "PASS: producer corpus + cross-impl adapter + produce-mode CLI + operator runbook + registry green (5/5)"
    exit 0
else
    exit 1
fi
