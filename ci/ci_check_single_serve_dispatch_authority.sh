#!/usr/bin/env bash
#
# ci_check_single_serve_dispatch_authority.sh — PHASE4-N-F-G-H S1 gate (DC-NODE-07).
#
# Single serve-dispatch authority. The node/producer serve-dispatch core
# `dispatch_server_frame_event_to_outbound` (maps an inbound N2N server-frame
# event over the ServedChainView through the BLUE chain-sync/block-fetch serve
# reducers and relays the typed reply as an OutboundCommand) has EXACTLY ONE
# definition, and it lives in shared `ade_runtime` — never duplicated in
# `ade_node`. This makes DC-NODE-07's "no second serve authority" clause
# mechanical: when `--mode node` is wired (S2) it MUST reuse this one core, not
# grow its own.
#
# Checks:
#   (a) exactly one `fn dispatch_server_frame_event_to_outbound` definition in
#       the workspace, and it is under crates/ade_runtime/;
#   (b) crates/ade_node/src/node_lifecycle.rs defines NO serve-dispatch of its
#       own (no parallel `fn ...dispatch_server_frame_event_to_outbound`);
#   (c) crates/ade_node/src/produce_mode.rs references the shared core (it was
#       re-pointed, not left holding a private copy).
#
# Note: at S1 the node spine does NOT yet CALL the core (that wiring is S2), so
# this gate only forbids a DUPLICATE definition — it is valid at both S1 and S2.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

CORE_FN='dispatch_server_frame_event_to_outbound'
NODE_LIFECYCLE='crates/ade_node/src/node_lifecycle.rs'
PRODUCE_MODE='crates/ade_node/src/produce_mode.rs'

# (a) Exactly one DEFINITION of the serve-dispatch core, under ade_runtime.
DEF_HITS="$(grep -rnE "fn ${CORE_FN}\b" crates --include='*.rs' || true)"
DEF_COUNT="$(printf '%s' "$DEF_HITS" | grep -c . || true)"
if [[ "$DEF_COUNT" -ne 1 ]]; then
  echo "[ci_check_single_serve_dispatch_authority] FAIL — expected exactly 1 definition of fn ${CORE_FN}, found ${DEF_COUNT}:"
  printf '%s\n' "$DEF_HITS"
  exit 1
fi
if ! printf '%s\n' "$DEF_HITS" | grep -qE '^crates/ade_runtime/'; then
  echo "[ci_check_single_serve_dispatch_authority] FAIL — the single ${CORE_FN} definition must live under crates/ade_runtime/ (shared home); found:"
  printf '%s\n' "$DEF_HITS"
  exit 1
fi

# (b) node_lifecycle.rs defines NO parallel serve-dispatch of its own.
if [[ ! -f "$NODE_LIFECYCLE" ]]; then
  echo "[ci_check_single_serve_dispatch_authority] FAIL — expected $NODE_LIFECYCLE to exist; failing closed"
  exit 1
fi
if grep -qE "fn ${CORE_FN}\b" "$NODE_LIFECYCLE"; then
  echo "[ci_check_single_serve_dispatch_authority] FAIL — $NODE_LIFECYCLE must not define its own serve-dispatch (${CORE_FN}); it must reuse the shared ade_runtime core"
  exit 1
fi

# (c) produce_mode.rs references the shared core (re-pointed, no private copy).
if [[ ! -f "$PRODUCE_MODE" ]]; then
  echo "[ci_check_single_serve_dispatch_authority] FAIL — expected $PRODUCE_MODE to exist; failing closed"
  exit 1
fi
if ! grep -qE "${CORE_FN}" "$PRODUCE_MODE"; then
  echo "[ci_check_single_serve_dispatch_authority] FAIL — $PRODUCE_MODE must reference the shared ${CORE_FN}"
  exit 1
fi

echo "[ci_check_single_serve_dispatch_authority] PASS — single serve-dispatch authority under crates/ade_runtime/; node_lifecycle defines none; produce_mode reuses it"
