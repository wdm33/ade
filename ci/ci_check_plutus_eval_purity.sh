#!/usr/bin/env bash
# ci_check_plutus_eval_purity.sh -- CN-PLUTUS-04 + CN-PLUTUS-01.
#
# CN-PLUTUS-04: "No host-environment property may influence script results."
# CN-PLUTUS-01: "Same script + redeemers/datum/context + cost model must produce
#                identical result and budget accounting" (determinism).
#
# Ade's Plutus verdict is a pure function of its CANONICAL inputs: the tx, the
# resolved UTxOs, the cost model, the per-script budget, and the slot config
# (all passed as arguments, never read from the host). The actual UPLC run is a
# pinned, deterministic aiken commit. So the only way a host-environment property
# could leak into a script result through Ade is if the BLUE `ade_plutus` crate
# itself read wall-clock / randomness / env / filesystem / a nondeterministically
# ordered collection. This gate forbids exactly that, which mechanically backs
# both the no-host-environment rule and the determinism rule (no nondeterminism
# source => identical result + budget across runs). The behavioral half of
# CN-PLUTUS-01 is `plutus_eval_is_deterministic` (two evals byte-identical).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$REPO_ROOT/crates/ade_plutus/src"
TEST="$REPO_ROOT/crates/ade_plutus/tests/end_to_end_plutus_eval.rs"
fail() { echo "FAIL (ci_check_plutus_eval_purity): $1" >&2; exit 1; }

# Host-environment + nondeterminism sources forbidden in the BLUE evaluator crate.
FORBIDDEN='SystemTime|Instant::|std::time|UNIX_EPOCH|::now\(\)|thread_rng|OsRng|getrandom|\brand::|std::env|env::var|env::vars|env::args|process::id|gethostname|std::fs|fs::read|fs::write|std::net|TcpStream|thread::spawn|\bHashMap\b|\bHashSet\b'

# Pure iff no forbidden token survives line-comment stripping.
file_is_pure() { ! grep -Eq "$FORBIDDEN" <<< "$(sed -E 's://.*$::' "$1")"; }

if [ "${1:-}" = "--self-test" ]; then
  tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
  printf 'fn leak() -> u64 {\n  std::time::SystemTime::now().elapsed().unwrap().as_secs()\n}\n' > "$tmp"
  if file_is_pure "$tmp"; then
    echo "FAIL: scanner missed a host-environment (wall-clock) read" >&2; exit 1
  fi
  # A pure file must pass.
  printf 'fn pure(a: u64, b: u64) -> u64 { a.saturating_add(b) }\n' > "$tmp"
  if ! file_is_pure "$tmp"; then
    echo "FAIL: scanner false-flagged a pure file" >&2; exit 1
  fi
  echo "PASS: scanner detects a host-environment read and clears a pure file"; exit 0
fi

[ -d "$SRC" ] || fail "missing $SRC"
[ -f "$TEST" ] || fail "missing $TEST"

viol=0
while IFS= read -r f; do
  if ! file_is_pure "$f"; then
    echo "  $f: host-environment / nondeterminism source in the BLUE evaluator" >&2
    grep -EnH "$FORBIDDEN" "$f" | sed -E 's:^([^:]*:[0-9]*:).*//.*$:\1 (comment):' || true
    viol=1
  fi
done < <(find "$SRC" -name '*.rs' | sort)
[ "$viol" -eq 0 ] || fail "ade_plutus evaluator is not host-pure -- a host-environment property could influence script results"

# Determinism (CN-PLUTUS-01) behavioral check present and actually runs.
grep -Eq 'fn plutus_eval_is_deterministic' "$TEST" \
  || fail "missing the plutus_eval_is_deterministic test (CN-PLUTUS-01 determinism)"
ctx="$(grep -B3 'fn plutus_eval_is_deterministic' "$TEST" | sed -E 's://.*$::' || true)"
if grep -Eq '#\[ignore' <<< "$ctx"; then
  fail "the determinism test is #[ignore]'d -- it must run in CI"
fi

echo "OK: ade_plutus evaluator is host-pure (no wall-clock / rand / env / fs / nondeterministic \
collections) and the determinism test runs -- CN-PLUTUS-04 + CN-PLUTUS-01 mechanically enforced"
