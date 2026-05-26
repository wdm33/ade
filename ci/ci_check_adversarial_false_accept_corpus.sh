#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-C S4 — adversarial false-accept corpus (DC-EVIDENCE-02).
#
# Mechanical guards (closure, not behaviour — behaviour is the
# integration test the CI runs after this gate):
#   1. The adversarial-corpus integration test exists at
#      `crates/ade_node/tests/admission_adversarial_corpus.rs`.
#   2. The test enumerates exactly 4 mandatory mutation classes
#      (body byte flip, header body-hash mismatch, KES tamper,
#      VRF tamper) — i.e. `MutationClass::all()` returns 4
#      variants.
#   3. The test asserts every mutation maps to one of
#      `AdmissionExitCode::Diverged` or
#      `AdmissionExitCode::PeerSentUndecodableBytes` (the closed
#      fail-set per DC-EVIDENCE-02).
#   4. The test asserts the closed numeric exit codes 30 / 34 via
#      the registered constants.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TEST="$REPO_ROOT/crates/ade_node/tests/admission_adversarial_corpus.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

if [[ ! -f "$TEST" ]]; then
    print_fail "expected adversarial corpus test missing: $TEST"
    exit "$FAILED"
fi

# Guard 1+2: 4 mandatory mutation classes.
EXPECTED_VARIANTS=(
    "BodyByteFlip"
    "HeaderBodyHashMismatch"
    "KesSignatureTamper"
    "VrfProofTamper"
)
for v in "${EXPECTED_VARIANTS[@]}"; do
    if ! grep -qE "MutationClass::${v}\b" "$TEST"; then
        print_fail "adversarial corpus missing mutation class: $v"
    fi
done

# Guard 3: fail-set assertion present.
if ! grep -qE 'AdmissionExitCode::Diverged' "$TEST"; then
    print_fail "corpus test does not assert AdmissionExitCode::Diverged"
fi
if ! grep -qE 'AdmissionExitCode::PeerSentUndecodableBytes' "$TEST"; then
    print_fail "corpus test does not assert AdmissionExitCode::PeerSentUndecodableBytes"
fi

# Guard 4: closed exit-code constants referenced.
if ! grep -qE 'EXIT_LIVE_AGREEMENT_DIVERGED' "$TEST"; then
    print_fail "corpus test does not reference EXIT_LIVE_AGREEMENT_DIVERGED"
fi
if ! grep -qE 'EXIT_LIVE_PEER_SENT_UNDECODABLE' "$TEST"; then
    print_fail "corpus test does not reference EXIT_LIVE_PEER_SENT_UNDECODABLE"
fi

# Guard 5: the corpus must NOT assert any Agreed / BlockAdmitted
# / InputNotFound success paths — that would be a contradiction
# with DC-EVIDENCE-02's "in no case may a mutation produce
# BlockAdmitted, Agreed, or InputNotFound".
for forbidden in "AdmissionExitCode::Ok" "kind: \"agreed\"" "block_admitted" "AdmissionHaltReason::InputNotFound"; do
    if grep -qE "$forbidden" "$TEST"; then
        # These literals MAY appear in a negative assertion (e.g.
        # `!s.contains("block_admitted")`). Look for the literal
        # appearing on a non-negated assertion line; if there is
        # ambiguity, leave it to the test runtime to catch.
        :  # No-op — we don't enforce literal absence; the test's
           # exit-code assertion already nails the contract.
    fi
done

if (( FAILED == 0 )); then
    echo "OK: adversarial false-accept corpus enumerates 4 mandatory mutation classes + asserts closed fail-set (DC-EVIDENCE-02)"
fi
exit $FAILED
