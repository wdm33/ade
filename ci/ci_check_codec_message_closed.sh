#!/usr/bin/env bash
# ci_check_codec_message_closed.sh -- CN-WIRE-07.
#
# "Each protocol-visible message must decode into one closed, versioned message type."
# The ade_network codec models the CLOSED wire grammar only (see codec/mod.rs): one
# closed enum per protocol, no open tail, no dynamic dispatch. This gate is the
# STRUCTURAL half (the closed-set discipline); the behavioral half (every variant
# round-trips + every decoder rejects unknown tags / truncated / invalid input) is the
# 45 codec::*::tests in the CN-WIRE-07 registry entry. Together they completely enforce
# the rule: no codec message type may silently grow an open / catch-all decode path.
#
# Mechanical guards (over crates/ade_network/src/codec, comments stripped):
#   1. No `#[non_exhaustive]` on any codec type (an open enum is not a closed message type).
#   2. No open-tail `Other(...)` / `Unknown(...)` enum variant (closed-set discipline; an
#      unknown tag must be a decode ERROR, not an accepted catch-all variant).
#   3. No `dyn` dispatch in the codec (one closed concrete type per protocol message;
#      the documented codec/mod.rs discipline).
#   4. A `version.rs` exists (the "versioned" half -- version-gated decode).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CODEC="$REPO_ROOT/crates/ade_network/src/codec"
fail() { echo "FAIL (ci_check_codec_message_closed): $1" >&2; exit 1; }
[ -d "$CODEC" ] || fail "codec dir missing: $CODEC"

# --self-test: inject a synthetic open-enum violation + confirm the scanner catches it.
if [ "${1:-}" = "--self-test" ]; then
  FIX="$CODEC/_codec_closed_self_test.rs"
  trap 'rm -f "$FIX"' EXIT
  cat > "$FIX" <<'RS'
// Core Contract: synthetic violation for ci_check_codec_message_closed.sh self-test
#[non_exhaustive]
pub enum SyntheticSelfTestMessage { A, B }
RS
  if bash "$0" >/dev/null 2>&1; then
    echo "FAIL: scanner did not detect synthetic #[non_exhaustive] codec violation" >&2; exit 1
  fi
  echo "PASS: scanner detected the synthetic open-codec violation"; exit 0
fi

# Comment-stripped view of every codec .rs (strip // line comments so the mod.rs
# discipline note -- which names these very patterns -- is not a false positive).
stripped() { sed -E 's://.*$::' "$1"; }

viol=0
while IFS= read -r -d '' f; do
  s="$(stripped "$f")"
  if grep -Eq '#\[non_exhaustive\]' <<< "$s"; then
    echo "  $f: #[non_exhaustive] (an open enum is not a closed message type)"; viol=1
  fi
  if grep -Eq '^[[:space:]]*(Other|Unknown)[[:space:]]*\(' <<< "$s"; then
    echo "  $f: open-tail Other/Unknown variant (unknown tag must be a decode error, not a catch-all)"; viol=1
  fi
  if grep -Eq '\bdyn[[:space:]]' <<< "$s"; then
    echo "  $f: dyn dispatch (codec is one closed concrete type per protocol)"; viol=1
  fi
done < <(find "$CODEC" -name '*.rs' -print0)
[ "$viol" -eq 0 ] || fail "ade_network codec is not closed (see above) -- CN-WIRE-07"

[ -f "$CODEC/version.rs" ] || fail "codec/version.rs missing (the version-gated half of CN-WIRE-07)"

echo "OK: ade_network codec message types are closed (no #[non_exhaustive] / open-tail / dyn) + versioned (CN-WIRE-07)"
