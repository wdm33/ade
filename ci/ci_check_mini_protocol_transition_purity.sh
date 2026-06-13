#!/usr/bin/env bash
# ci_check_mini_protocol_transition_purity.sh -- DC-PROTO-01 + DC-PROTO-06.
#
# DC-PROTO-01 "Protocol state machines have deterministic transitions" + DC-PROTO-06
# "BLUE mini-protocol transitions are pure functions of (canonical prior state, canonical
# input message, selected protocol version)." Structural half: every mini-protocol's FSM
# modules (state.rs / agency.rs / transition.rs, + session/state.rs) carry NO nondeterminism
# source in PRODUCTION code -- no HashMap/HashSet (unordered iteration), no SystemTime/
# Instant (wall-clock), no rand (true randomness), no f32/f64 (float). The trailing
# #[cfg(test)] module is excluded (tests may use anything). No-async is already enforced
# crate-wide by DC-CORE-01 (ci_check_no_async_in_blue.sh). Behavioral half = the ~90
# DC-PROTO-01/06 transition + replay-determinism tests. Together = complete enforcement.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NET="$REPO_ROOT/crates/ade_network/src"
fail() { echo "FAIL (ci_check_mini_protocol_transition_purity): $1" >&2; exit 1; }
[ -d "$NET" ] || fail "ade_network/src missing"

FORBID='\b(HashMap|HashSet)\b|\bSystemTime\b|\bInstant\b|\brand\b|\bf32\b|\bf64\b'

# Production view: drop the trailing #[cfg(test)] module, then strip // line comments.
prod() { sed -E '/#\[cfg\(test\)\]/,$d' "$1" | sed -E 's://.*$::'; }

scan() {  # $1 = root; returns 1 if any FSM transition file has a nondeterminism source
  local viol=0 f
  while IFS= read -r -d '' f; do
    if prod "$f" | grep -Eq "$FORBID"; then
      echo "  $f: nondeterminism source in a protocol transition (HashMap/HashSet/SystemTime/Instant/rand/float)"; viol=1
    fi
  done < <(find "$1" -path '*/bin/*' -prune -o \( -name state.rs -o -name agency.rs -o -name transition.rs \) -print0)
  return $viol
}

if [ "${1:-}" = "--self-test" ]; then
  d="$(mktemp -d)"; trap 'rm -rf "$d"' EXIT; mkdir -p "$d/p"
  printf 'use std::collections::HashMap;\npub fn transition() { let _m: HashMap<u8,u8> = HashMap::new(); }\n' > "$d/p/transition.rs"
  if scan "$d" >/dev/null 2>&1; then echo "FAIL: scanner missed a synthetic HashMap transition" >&2; exit 1; fi
  echo "PASS: scanner detects a nondeterminism source in a transition fixture"; exit 0
fi

scan "$NET" || fail "ade_network protocol FSM is not pure/deterministic (see above) -- DC-PROTO-01/06"
echo "OK: all ade_network mini-protocol transitions are nondeterminism-free in production (DC-PROTO-01/06; no-async via DC-CORE-01)"
