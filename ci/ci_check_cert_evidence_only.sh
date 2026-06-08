#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-AH (DC-NODE-21): the adoption certificate is rung-1 evidence-only, never
# forge authority. NO ade_node production code reads / parses / names the certificate
# in the forge-authority surface. The operator harness owns cert/evidence parsing
# OUTSIDE the node; the cert file + parser/writer live in harness/evidence tooling.
#
# Asserts: NONE of the certificate tokens
#   read_adoption_cert | adoption_cert_path | parse_hex32 | VenueAdoptionCertificate |
#   AdoptionCertificate
# appears in the PRODUCTION body (#[cfg(test)] stripped, line/doc comments stripped)
# of the ade_node forge-authority surface (node_sync.rs, node_lifecycle.rs, cli.rs).
# Test / harness / evidence paths are out of scope -- the cert may be written there as
# operator evidence.
#
# Fails closed if a future change re-introduces the certificate into ade_node forge
# authority (the exact authority-creep DC-NODE-21 prevents).
#
# Repo-root-relative. Mirrors the other ci_check_*.sh gates.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

FAILED=0
fail() { echo "FAIL (cert-evidence-only): $1"; FAILED=1; }

CERT_TOKENS='read_adoption_cert|adoption_cert_path|parse_hex32|VenueAdoptionCertificate|AdoptionCertificate'

for f in crates/ade_node/src/node_sync.rs crates/ade_node/src/node_lifecycle.rs crates/ade_node/src/cli.rs; do
    if [[ ! -f "$f" ]]; then
        echo "FAIL: $f not found"; exit 1
    fi
    # Production body: drop the #[cfg(test)] module; strip line/doc comments so a
    # comment naming a token (e.g. a "token REMOVED" notice) does not trip the grep.
    PROD="$(awk '/#\[cfg\(test\)\]/{exit} {print}' "$f" | sed -E 's://.*::')"
    HITS="$(grep -nE "$CERT_TOKENS" <<<"$PROD" || true)"
    if [[ -n "$HITS" ]]; then
        fail "$f production body references a certificate token (DC-NODE-21: the cert is evidence-only, never forge authority):"
        echo "$HITS" | head | sed 's/^/    /'
    fi
done

if (( FAILED == 0 )); then
    echo "OK (cert-evidence-only): no certificate token (read_adoption_cert / adoption_cert_path / parse_hex32 / VenueAdoptionCertificate / AdoptionCertificate) appears in the ade_node forge-authority production surface (node_sync / node_lifecycle / cli) — the operator harness owns cert/evidence parsing outside the node (DC-NODE-21)."
fi
exit $FAILED
