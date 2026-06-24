#!/usr/bin/env bash
set -uo pipefail

# ci_check_native_firstrun_no_cli_seed.sh -- MITHRIL-VERIFIED-ANCHOR-INTEGRATION S1d.
#
# The live `--mode node` FirstRun NATIVE route wires the verified Mithril
# manifest + the V2 LedgerDB `state` + the Stage-2 `tables` + the Cardano Shelley
# genesis through the UNCHANGED S1a/S1b/S1c native chain (the snapshot IS the
# source). The cardano-cli / JSON consensus-input seed is FORBIDDEN on this
# route: it must NEVER reach import_cardano_cli_json_utxo / import_live_consensus_inputs,
# and a forbidden flag (--json-seed-path / --consensus-inputs-path) supplied
# ALONGSIDE the native inputs is a structured TERMINAL error (no fallback, no
# silent ignore) (DC-MITHRIL-07).
#
# Mechanical guards (negative-controlled: each grep-for-absence fails on an
# injected token in the production body):
#   (A) the native module + the orchestration entry + the dispatch function exist;
#   (B) the native orchestration body (native_firstrun.rs, cfg(test) stripped)
#       references NONE of the CLI-seed extraction tokens;
#   (C) the dispatch function (first_run_native_mithril_bootstrap, cfg(test)
#       stripped) does NOT call the CLI-seed extraction importers, and DOES carry
#       the two forbidden-flag terminal arms;
#   (D) the native route is selected on state+tables presence and routes through
#       native_first_run_bootstrap -> bootstrap_from_native_mithril_snapshot (the
#       single closed composition), never a parallel storage-init path;
#   (E) the S1d tests are present.
#
# NOTE: the stripped bodies are written to temp files and grepped from the file
# (NOT `echo "$BODY" | grep -q`) -- under `set -o pipefail` a `grep -q` that
# matches closes the pipe early, the upstream `echo` gets SIGPIPE, and the
# pipeline exit status becomes the echo's 141 (not grep's 0), spuriously firing
# the `|| print_fail`. File grep avoids the pipe entirely.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO_ROOT/crates/ade_node/src/native_firstrun.rs"
LIFECYCLE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"
LIVE_TEST="$REPO_ROOT/crates/ade_node/tests/native_firstrun_live.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Strip the #[cfg(test)] module (from the test attribute to EOF) and line
# comments, so the guards only see production code. Writes to $2.
strip_for_grep() {
    awk '
        /^#\[cfg\(test\)\]/ { in_test=1 }
        in_test { next }
        { line=$0; sub(/\/\/.*$/, "", line); print line }
    ' "$1" > "$2"
}

# Extract a single fn body (from `fn <name>` to the next top-level `^}`) out of
# an already-stripped file. Writes to $3.
extract_fn() {
    awk -v fn="fn $2" '
        $0 ~ fn { capture=1 }
        capture { print }
        capture && /^}/ { exit }
    ' "$1" > "$3"
}

# (A) module + entry points exist.
[ -f "$NATIVE" ] || { print_fail "missing native module $NATIVE"; echo "FAIL: ci_check_native_firstrun_no_cli_seed"; exit 1; }
[ -f "$LIFECYCLE" ] || { print_fail "missing $LIFECYCLE"; echo "FAIL: ci_check_native_firstrun_no_cli_seed"; exit 1; }

NATIVE_BODY="$WORK/native.body"
LIFECYCLE_BODY="$WORK/lifecycle.body"
DISPATCH_FN="$WORK/dispatch.fn"
strip_for_grep "$NATIVE" "$NATIVE_BODY"
strip_for_grep "$LIFECYCLE" "$LIFECYCLE_BODY"

grep -Eq '\bpub fn native_first_run_bootstrap\b' "$NATIVE_BODY" \
    || print_fail "missing native_first_run_bootstrap orchestration entry"
grep -Eq '\bfn first_run_native_mithril_bootstrap\b' "$LIFECYCLE_BODY" \
    || print_fail "missing first_run_native_mithril_bootstrap dispatch function"

# (B) the native orchestration body references NONE of the CLI-seed extraction
# tokens. The native orchestration is the snapshot-only chain; it must not even
# name the operator-seed importers.
for forbidden in \
    'import_cardano_cli_json_utxo' \
    'import_live_consensus_inputs' \
    'json_seed_path' \
    'consensus_inputs_path' \
    '--json-seed'
do
    if grep -qF -- "$forbidden" "$NATIVE_BODY"; then
        print_fail "native orchestration references the CLI/JSON-seed token '$forbidden' (DC-MITHRIL-07: the snapshot is the sole source on the native route)"
    fi
done

# (C) the dispatch function does NOT call the CLI-seed extraction importers,
# and DOES carry the two forbidden-flag terminal arms (it references the flag
# NAMES only to REJECT them).
extract_fn "$LIFECYCLE_BODY" "first_run_native_mithril_bootstrap" "$DISPATCH_FN"
[ -s "$DISPATCH_FN" ] || print_fail "could not extract first_run_native_mithril_bootstrap body"
for forbidden in \
    'import_cardano_cli_json_utxo' \
    'import_live_consensus_inputs'
do
    if grep -qF -- "$forbidden" "$DISPATCH_FN"; then
        print_fail "the native dispatch calls the CLI-seed importer '$forbidden' (it must NEVER reach it)"
    fi
done
# The forbidden-flag terminal arms (reject --json-seed-path / --consensus-inputs-path).
grep -q 'NativeRouteForbiddenFlag("--json-seed-path")' "$DISPATCH_FN" \
    || print_fail "the native route must reject --json-seed-path (NativeRouteForbiddenFlag terminal)"
grep -q '"--consensus-inputs-path"' "$DISPATCH_FN" \
    || print_fail "the native route must reject --consensus-inputs-path"

# (D) the native route is SELECTED on state+tables presence and routes through
# the single closed native bootstrap composition.
grep -Eq 'mithril_state_path\.is_some\(\)[[:space:]]*&&[[:space:]]*cli\.mithril_tables_path\.is_some\(\)' "$LIFECYCLE_BODY" \
    || print_fail "the FirstRun arm must select the native route on (--mithril-state-path && --mithril-tables-path) presence"
grep -Eq 'native_firstrun::native_first_run_bootstrap\(' "$DISPATCH_FN" \
    || print_fail "the native dispatch must route through native_first_run_bootstrap"
grep -Eq '\bbootstrap_from_native_mithril_snapshot\(' "$NATIVE_BODY" \
    || print_fail "the native orchestration must route through the single closed bootstrap_from_native_mithril_snapshot"
# No parallel storage-init authority declared in the native module.
if grep -qE '\bpub fn bootstrap_initial_state\b' "$NATIVE_BODY"; then
    print_fail "the native module declares a second bootstrap_initial_state authority (no parallel storage-init path)"
fi

# (E) the S1d tests are present (dispatch-level hermetic + real-snapshot live).
for t in \
    'native_first_run_forbidden_json_seed_is_terminal' \
    'native_first_run_forbidden_consensus_inputs_is_terminal' \
    'native_first_run_missing_manifest_is_terminal' \
    'native_first_run_missing_shelley_genesis_is_terminal' \
    'native_first_run_malformed_manifest_is_terminal'
do
    grep -q "fn $t" "$LIFECYCLE" || print_fail "missing S1d dispatch test '$t'"
done
[ -f "$LIVE_TEST" ] || print_fail "missing S1d real-snapshot test $LIVE_TEST"
if [ -f "$LIVE_TEST" ]; then
    for t in \
        'native_first_run_real_snapshot_invokes_bootstrap_and_persists' \
        'native_first_run_real_snapshot_wrong_network_is_terminal'
    do
        grep -q "fn $t" "$LIVE_TEST" || print_fail "missing S1d real-snapshot test '$t'"
    done
fi

if (( FAILED == 0 )); then
    echo "OK: the live --mode node FirstRun NATIVE route wires the manifest + state + tables + Shelley genesis through the unchanged S1a/S1b/S1c chain, never reaches the cardano-cli / JSON seed, and fails closed (terminal) on a forbidden flag supplied alongside (DC-MITHRIL-07)"
fi
exit $FAILED
