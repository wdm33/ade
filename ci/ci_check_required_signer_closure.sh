#!/usr/bin/env bash
# ci_check_required_signer_closure.sh -- CN-LEDGER-09 / DC-LEDGER-05 (CLOSURE half).
#
# "Witnesses must bind exactly ... for the era." A required signer that is silently dropped
# from the enumeration is a FALSE-ACCEPT (a tx missing that witness would pass). This gate
# locks the STRUCTURAL closure: every source of a required signer is a variant of the CLOSED
# `SignerSource` enum (no open tail, no #[non_exhaustive]), the witness field + closure-error
# sums are closed, an unknown era fail-closes (`UnsupportedEra`), and the SignerSource
# enumeration is COMPLETE (all six sources present -- none may be silently removed).
# Behavioral half = the 20 tx_witness_closure.rs adversarial tests (each source's
# missing-witness reject + wrong-body/wrong-size/extra-witness/script-credential cases).
#
# SCOPE NOTE: this gate is the CONWAY closed-enumeration + binding closure only. The per-era
# binding completeness (Byron TxWitness / Shelley bootstrap / Alonzo redeemers vs Conway) is
# the remaining scope -- CN-LEDGER-09 / DC-LEDGER-05 stay `partial`; this does not flip them.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RS="$REPO_ROOT/crates/ade_ledger/src/tx_validity/required_signers.rs"
W="$REPO_ROOT/crates/ade_ledger/src/tx_validity/witness.rs"
fail() { echo "FAIL (ci_check_required_signer_closure): $1" >&2; exit 1; }
for f in "$RS" "$W"; do [ -f "$f" ] || fail "missing $f"; done

check_closed() {  # $1 = file; returns 1 if an open enum / open-tail variant is present
  local s; s="$(sed -E 's://.*$::' "$1")"
  if grep -Eq '#\[non_exhaustive\]' <<< "$s"; then echo "  $1: #[non_exhaustive] (open enum)"; return 1; fi
  if grep -Eq '^[[:space:]]*(Other|Unknown)[[:space:]]*[({]' <<< "$s"; then echo "  $1: open-tail Other/Unknown variant"; return 1; fi
  return 0
}

if [ "${1:-}" = "--self-test" ]; then
  tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
  printf 'pub enum SignerSource {\n  InputPaymentKey,\n  Unknown(u8),\n}\n' > "$tmp"
  if check_closed "$tmp"; then echo "FAIL: scanner missed an open signer-source surface" >&2; exit 1; fi
  echo "PASS: scanner detects an open signer-source surface"; exit 0
fi

check_closed "$RS" || fail "required-signer surface ($RS) is not closed"
check_closed "$W"  || fail "witness surface ($W) is not closed"
# Completeness: every required-signer SOURCE is enumerated (none silently dropped).
for v in InputPaymentKey ExplicitRequiredSigner WithdrawalKey CertificateKey GovernanceVoter CollateralPaymentKey; do
  grep -Eq "\b$v\b" "$RS" || fail "SignerSource missing the $v source -- a required witness could be skipped (false-accept)"
done
# Era-versioned: an unknown era fail-closes rather than under-enumerating.
grep -Eq 'UnsupportedEra' "$RS" || fail "required-signer enumeration must fail-close on an unsupported era (UnsupportedEra)"
echo "OK: SignerSource closed + complete (6 sources) + era-fail-closed; witness surfaces closed (CN-LEDGER-09 / DC-LEDGER-05 closure half; per-era completeness remains open)"
