#!/usr/bin/env bash
# ci_check_plutus_conformance.sh -- A4 Plutus result/budget conformance manifest.
#
# Verifies the registry-bound conformance evidence for CN-PLUTUS-01's budget-correctness
# (B) half (result + ex_units parity vs the IOG reference). It binds four things and fails
# on any drift:
#   1. CORPUS IMMUTABLE  -- recompute the content hash over the evaluation/ tree; must
#      equal the manifest's corpus.content_sha256.
#   2. EVALUATOR PINNED  -- the manifest's aiken commit must be the one Cargo.lock resolves.
#   3. MANIFEST INTACT   -- recompute the binding hash over the load-bearing fields; must
#      equal the recorded manifest.sha256 (tamper-evidence).
#   4. SUITE NOT WEAKENED -- the behavioral proof
#      (ade_testkit::plutus_conformance::plutus_conformance_evaluation_suite) must still
#      carry the EXACT-count assertions (total / skipped / 0 result_mismatch / 0
#      budget_mismatch / 0 parse_failed), so "CI fails on any diverge" cannot be relaxed
#      back to a floor.
#
# The suite itself (run under cargo test) is the behavioral 0-diverge check; this gate
# locks the evidence it rests on.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$REPO_ROOT/docs/evidence/plutus-conformance-manifest.toml"
SUITE="$REPO_ROOT/crates/ade_testkit/tests/plutus_conformance.rs"
LOCK="$REPO_ROOT/Cargo.lock"
fail() { echo "FAIL (ci_check_plutus_conformance): $1" >&2; exit 1; }

# Extract a "key = value" from a TOML [section] (strips quotes, inline comments, ws).
mval() {  # $1 = file, $2 = section, $3 = key
  awk -v sec="[$2]" -v key="$3" '
    $0==sec {ins=1; next}
    /^\[/    {ins=0}
    ins && $1==key { sub(/^[^=]*=[ \t]*/,""); sub(/[ \t]*#.*/,""); gsub(/"/,""); gsub(/[ \t]+$/,""); print; exit }
  ' "$1"
}

# Recompute the binding hash from a manifest's own fields; succeed iff it matches the
# manifest's recorded manifest.sha256.
verify_binding_hash() {  # $1 = manifest file
  local m="$1" aiken corpus content total skipped runnable exact alpha rm bm pf rec canon got
  aiken=$(mval "$m" evaluator commit);       corpus=$(mval "$m" corpus commit)
  content=$(mval "$m" corpus content_sha256)
  total=$(mval "$m" outcome total);          skipped=$(mval "$m" outcome skipped)
  runnable=$(mval "$m" outcome runnable)
  exact=$(mval "$m" outcome result_parity_exact); alpha=$(mval "$m" outcome result_parity_alpha)
  rm=$(mval "$m" outcome result_mismatch);   bm=$(mval "$m" outcome budget_mismatch)
  pf=$(mval "$m" outcome parse_failed);      rec=$(mval "$m" manifest sha256)
  canon="aiken=${aiken}|corpus=${corpus}|corpus_content=${content}|total=${total}|skipped=${skipped}|runnable=${runnable}|exact=${exact}|alpha=${alpha}|result_mismatch=${rm}|budget_mismatch=${bm}|parse_failed=${pf}"
  got=$(printf '%s' "$canon" | sha256sum | awk '{print $1}')
  [ "$rec" = "$got" ]
}

if [ "${1:-}" = "--self-test" ]; then
  [ -f "$MANIFEST" ] || fail "missing $MANIFEST"
  tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
  # Tamper one bound field; the recorded hash must no longer verify.
  sed -E 's/(^total[[:space:]]*=[[:space:]]*)728/\1999/' "$MANIFEST" > "$tmp"
  if verify_binding_hash "$tmp"; then
    echo "FAIL: a tampered outcome field was not detected by the binding hash" >&2; exit 1
  fi
  # The untouched manifest must verify.
  if ! verify_binding_hash "$MANIFEST"; then
    echo "FAIL: the real manifest's binding hash does not verify" >&2; exit 1
  fi
  echo "PASS: binding hash verifies the manifest and detects a tampered field"; exit 0
fi

for f in "$MANIFEST" "$SUITE" "$LOCK"; do [ -f "$f" ] || fail "missing $f"; done

# 1. Corpus immutable.
CORPUS_PATH="$(mval "$MANIFEST" corpus path)"
[ -n "$CORPUS_PATH" ] && [ -d "$REPO_ROOT/$CORPUS_PATH" ] || fail "corpus path '$CORPUS_PATH' not found"
EXPECT_CONTENT="$(mval "$MANIFEST" corpus content_sha256)"
ACTUAL_CONTENT="$( cd "$REPO_ROOT/$CORPUS_PATH" && find . -type f | LC_ALL=C sort | xargs sha256sum | sha256sum | awk '{print $1}' )"
[ "$ACTUAL_CONTENT" = "$EXPECT_CONTENT" ] \
  || fail "corpus content drift: manifest=$EXPECT_CONTENT disk=$ACTUAL_CONTENT (corpus changed -- re-bless the manifest)"

# 2. Evaluator pinned: the manifest's aiken commit is what Cargo.lock resolves.
AIKEN_COMMIT="$(mval "$MANIFEST" evaluator commit)"
grep -q "aiken-lang/aiken?tag=v1.1.21#${AIKEN_COMMIT}" "$LOCK" \
  || fail "manifest aiken commit ${AIKEN_COMMIT} is not the one pinned in Cargo.lock"

# 3. Manifest binding hash intact (tamper-evidence).
verify_binding_hash "$MANIFEST" \
  || fail "manifest binding hash does not match its fields -- a bound value was edited without re-blessing"

# 4. The behavioral suite still carries the EXACT 0-diverge assertions (not weakened to a floor).
S="$(sed -E 's://.*$::' "$SUITE")"
for a in 'stats\.total, 728' 'stats\.skipped, 214' 'stats\.budget_mismatch, 0' 'stats\.result_mismatch, 0' 'stats\.parse_failed, 0'; do
  grep -Eq "$a" <<< "$S" || fail "conformance suite missing exact assertion ($a) -- the 0-diverge binding was relaxed"
done

echo "OK: Plutus conformance manifest bound -- corpus immutable (sha256 $EXPECT_CONTENT) + aiken pinned \
(${AIKEN_COMMIT:0:12}) + binding hash intact + suite asserts 514/514 result + ex_units parity, 0 diverge"
