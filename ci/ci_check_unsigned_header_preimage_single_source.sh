#!/usr/bin/env bash
#
# ci_check_unsigned_header_preimage_single_source.sh — PHASE4-N-S-A A4 gate.
#
# Enforces CN-KES-HEADER-01's "single source of truth" rule:
# the only function that constructs the canonical unsigned-
# header pre-image bytes is
# `ade_ledger::block_validity::unsigned_header_pre_image::unsigned_header_pre_image`.
#
# The branded `UnsignedHeaderPreImage(Vec<u8>)` type already
# has a private inner field, so the type system prevents
# arbitrary construction outside the module. This gate is
# defense-in-depth: it rejects any NEW external caller
# referencing the type outside the documented allow-list.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ALLOW_LIST=(
  "crates/ade_ledger/src/block_validity/unsigned_header_pre_image.rs"
  "crates/ade_ledger/src/block_validity/mod.rs"
  "crates/ade_runtime/src/producer/producer_shell.rs"
  "crates/ade_node/src/produce_mode.rs"
)

HITS=$(
  grep -rln "UnsignedHeaderPreImage" crates/ 2>/dev/null | sort -u || true
)

VIOLATIONS=""
for f in $HITS; do
  skip=0
  for allowed in "${ALLOW_LIST[@]}"; do
    if [[ "$f" == "$allowed" ]]; then
      skip=1
      break
    fi
  done
  if [[ $skip -eq 1 ]]; then
    continue
  fi
  case "$f" in
    */tests/*) continue ;;
    */testkit/*) continue ;;
  esac
  VIOLATIONS+="$f\n"
done

if [[ -n "$VIOLATIONS" ]]; then
  echo "[ci_check_unsigned_header_preimage_single_source] FAIL — new caller(s) outside allow-list:"
  echo -e "$VIOLATIONS" | sed 's/^/  /'
  echo ""
  echo "  Allow-list (canonical pre-image recipe + consumers):"
  for a in "${ALLOW_LIST[@]}"; do
    echo "    $a"
  done
  echo ""
  echo "  The branded type has a private inner field. New external"
  echo "  callers MUST use the canonical recipe"
  echo "    ade_ledger::block_validity::unsigned_header_pre_image::unsigned_header_pre_image"
  echo "  not roll their own ShelleyHeaderBody encoder."
  exit 1
fi

echo "[ci_check_unsigned_header_preimage_single_source] PASS"
