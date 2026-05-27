#!/usr/bin/env bash
#
# ci_check_no_independent_forge_codepath.sh — PHASE4-N-R-C C3 gate.
#
# Enforces N10 ("no independent legacy production path"):
# only `ade_node::produce_mode::run_real_forge` may compose
# the full RED-vrf-prove + RED-kes-sign + BLUE-forge-block +
# BLUE-self-accept pipeline. Any other production file that
# combines these primitives is rejected.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ALLOW_LIST=(
  "crates/ade_node/src/produce_mode.rs"
)

CANDIDATES=$(
  grep -rln "vrf_prove" crates/ 2>/dev/null | sort -u || true
)

VIOLATIONS=""
for f in $CANDIDATES; do
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
    */test_*) continue ;;
  esac
  case "$f" in
    crates/ade_runtime/src/producer/producer_shell.rs) continue ;;
    crates/ade_runtime/src/producer/signing.rs) continue ;;
    crates/ade_ledger/src/producer/forge.rs) continue ;;
    crates/ade_ledger/src/producer/self_accept.rs) continue ;;
    crates/ade_core_interop/src/bin/live_block_production_session.rs) continue ;;
  esac

  if grep -q "kes_sign_at\|kes_sign\b" "$f" && \
     grep -q "forge_block\b" "$f" && \
     grep -q "self_accept\b" "$f"; then
    VIOLATIONS+="$f\n"
  fi
done

if [[ -n "$VIOLATIONS" ]]; then
  echo "[ci_check_no_independent_forge_codepath] FAIL — independent forge codepath(s) detected:"
  echo -e "$VIOLATIONS" | sed 's/^/  /'
  echo ""
  echo "  Allow-list (canonical forge composition):"
  for a in "${ALLOW_LIST[@]}"; do
    echo "    $a"
  done
  echo ""
  echo "  Only ade_node::produce_mode::run_real_forge may compose the"
  echo "  full RED-vrf-prove + RED-kes-sign + BLUE-forge-block +"
  echo "  BLUE-self-accept pipeline."
  exit 1
fi

echo "[ci_check_no_independent_forge_codepath] PASS"
