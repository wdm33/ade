#!/usr/bin/env bash
set -uo pipefail

# MEM-OPT-UTXO-DISK S2b-2c.0: the admission seam SPEC must exist -- with the
# atomicity/recovery rule + the hard prohibitions -- BEFORE any live-path code. The
# highest-risk step composes RED storage I/O / BLUE validation / RED durable commit,
# which must not blur. This gate is doc-structural (spec-only slice) and guards
# against the live rewire leaking in early.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
D=docs/clusters/MEM-OPT-UTXO-DISK/S2b-2c0-admission-seam.md

test -f "$D" || { echo "FAIL: the admission seam spec is missing"; exit 1; }

# (1) the GREEN/BLUE/RED seam shape.
grep -qF 'collect_required_txins_block' "$D" || fail "seam: GREEN collect step missing"
grep -qF 'resolve_required' "$D" || fail "seam: RED resolve step missing"
grep -qF 'WorkingSet::seed_required_from_anchor' "$D" || fail "seam: GREEN WorkingSet seed step missing"
grep -qiE 'validate.*against the .?WorkingSet|resolved view only' "$D" || fail "seam: BLUE validate-over-resolved-view step missing"

# (2) the load-bearing rule: validation never calls redb; commit never decides validation.
grep -qiE 'validation never calls redb' "$D" || fail "the 'validation never calls redb' rule is missing"
grep -qiE 'redb commit never decides validation' "$D" || fail "the 'commit never decides validation' rule is missing"

# (3) the atomicity rule: two substrates, one atomic redb txn, bytes-before-admit ordering.
grep -qiE 'two durable substrates|file log and a redb' "$D" || fail "the two-substrate atomicity analysis is missing"
grep -qiE 'one redb write-txn|single redb write' "$D" || fail "the atomic single-redb-txn anchor commit is missing"
grep -qiE 'bytes-before-admit|BEFORE the WAL append' "$D" || fail "the bytes-before-admit ordering invariant is missing"

# (4) the recovery rule (NOT hand-waved).
grep -qiE 'roll forward|roll-forward' "$D" || fail "the roll-forward recovery is missing"
grep -qiE 'replay_from_anchor' "$D" || fail "recovery does not reuse the existing replay_from_anchor"
grep -qiE 'AHEAD of WAL.*IMPOSSIBLE|never leads the WAL' "$D" || fail "the anchor-never-ahead invariant is missing"
grep -qiE 'fail closed' "$D" || fail "the fail-closed corruption rule is missing"
grep -qiE 'split-brain' "$D" || fail "the no-split-brain argument is missing"

# (5) the required tests (the 2c.1 merge gate) are enumerated.
grep -qiE 'torn commit' "$D" || fail "required test: torn-commit half-admitted missing"
grep -qiE 'replay after restart matches' "$D" || fail "required test: replay-after-restart missing"
grep -qiE 'block-hash agreement unchanged' "$D" || fail "required test: block-hash agreement missing"

# (6) the hard prohibitions.
grep -qiE 'No lazy disk lookup inside' "$D" || fail "prohibition: no lazy disk lookup in UtxoStore missing"
grep -qiE 'No cache in this slice' "$D" || fail "prohibition: no cache in this slice missing"
grep -qiE 'visibility before UTxO anchor durability|before UTxO anchor durability' "$D" || fail "prohibition: no visibility before anchor durability missing"
grep -qiE 'No .?OP-MEM-02.? claim' "$D" || fail "prohibition: no OP-MEM-02 claim pre-re-measure missing"

# (7) 2c.0 is SPEC ONLY -- the resolved path must NOT yet be wired into the admission
#     runner (that is 2c.1). Guard against shipping live code under the spec slice.
if grep -rqE 'resolve_required|seed_required_from_anchor' crates/ade_node/src/admission/; then
    fail "the admission runner already wires the resolved path -- that is 2c.1, not the 2c.0 spec"
fi

if (( FAILED == 0 )); then
    echo "OK: admission seam spec (S2b-2c.0; GREEN/BLUE/RED seam; one-redb-txn anchor commit + WAL-authority roll-forward recovery; ahead=impossible/fail-closed; prohibitions stated; no live rewire yet)"
fi
exit $FAILED
