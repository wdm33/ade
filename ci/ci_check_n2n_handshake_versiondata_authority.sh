#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-L (CN-WIRE-10): one shared per-version N2N handshake versionData authority,
# used by BOTH the serve responder and the initiator; no placeholder / bare-int versionData.
#
#   (a) the single per-version authority `encode_n2n_version_params` exists.
#   (b) the serve responder (handshake_driver) builds AcceptVersion via that authority, and the prior
#       bare-int placeholder `VersionParams(vec![0x01])` (= CBOR TInt 1) is gone from the responder.
#   (c) the initiator (build_n2n_version_table) uses the SAME authority -- the two directions cannot
#       diverge.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VT="$REPO_ROOT/crates/ade_network/src/handshake/version_table.rs"
HD="$REPO_ROOT/crates/ade_network/src/session/handshake_driver.rs"
BS="$REPO_ROOT/crates/ade_node/src/admission/bootstrap.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

for f in "$VT" "$HD" "$BS"; do
    [[ -f "$f" ]] || print_fail "missing expected source $f"
done

# (a) the single per-version N2N versionData authority.
grep -Eq 'pub fn encode_n2n_version_params' "$VT" \
    || print_fail "(a) encode_n2n_version_params (the shared per-version N2N versionData authority) not found in version_table.rs"

# (b) the serve responder builds its AcceptVersion via the shared authority ...
grep -Eq 'encode_n2n_version_params\(' "$HD" \
    || print_fail "(b) handshake_driver responder does not build AcceptVersion via encode_n2n_version_params"
# ... and the prior bare-int placeholder is gone from the responder build (the loop-var `version` form).
if grep -Eq 'AcceptVersion\(version, VersionParams\(vec!\[0x01\]\)\)' "$HD"; then
    print_fail "(b) responder still emits the VersionParams(vec![0x01]) = CBOR TInt 1 placeholder"
fi

# (c) the initiator shares the SAME authority (no second hand-rolled per-version encoding).
grep -Eq 'encode_n2n_version_params' "$BS" \
    || print_fail "(c) build_n2n_version_table (initiator) does not use encode_n2n_version_params -- initiator/responder may diverge"

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_n2n_handshake_versiondata_authority: FAILED"
    exit 1
fi
echo "ci_check_n2n_handshake_versiondata_authority: OK (CN-WIRE-10 -- one shared per-version handshake versionData authority)"
