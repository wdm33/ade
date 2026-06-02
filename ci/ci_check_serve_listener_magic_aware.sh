#!/usr/bin/env bash
#
# ci_check_serve_listener_magic_aware.sh — PHASE4-N-F-G-H S2b gate (DC-NODE-07).
#
# The live serve listeners (node-spine + producer) MUST advertise N2N versions
# using the CONFIGURED network magic — built via
# `ade_network::handshake::version_table::n2n_supported_for_magic(<magic>)` — NOT
# the static mainnet `N2N_SUPPORTED` table. Otherwise a real preprod (magic 1) /
# C1 (magic 42) follower's N2N handshake is refused on a magic mismatch. Version
# negotiation stays closed + enumerated; only `network_magic` is parameterized.
#
# Checks, for each live serve site (produce_mode.rs, node_lifecycle.rs):
#   (a) no `our_supported:` is set to the static `N2N_SUPPORTED`;
#   (b) the site builds its serve version table via `n2n_supported_for_magic`.
#
# Scope: only the live serve binaries. The dialer's use of `N2N_SUPPORTED` to
# ENUMERATE version numbers (it overrides the magic) and test listeners are not
# live serve sites and are out of scope.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

LIVE_SERVE_SITES=(
  "crates/ade_node/src/produce_mode.rs"
  "crates/ade_node/src/node_lifecycle.rs"
)

for f in "${LIVE_SERVE_SITES[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "[ci_check_serve_listener_magic_aware] FAIL — expected $f to exist; failing closed"
    exit 1
  fi
  # (a) No live serve listener may set `our_supported` to the static N2N_SUPPORTED.
  if grep -nE 'our_supported:[[:space:]]*([A-Za-z_]+::)*N2N_SUPPORTED' "$f"; then
    echo "[ci_check_serve_listener_magic_aware] FAIL — $f sets a serve listener's our_supported to the static mainnet N2N_SUPPORTED; build it via n2n_supported_for_magic(<configured magic>) instead"
    exit 1
  fi
  # (b) The live serve site must build the magic-aware table.
  if ! grep -qE 'n2n_supported_for_magic' "$f"; then
    echo "[ci_check_serve_listener_magic_aware] FAIL — $f does not build its serve version table via n2n_supported_for_magic"
    exit 1
  fi
done

echo "[ci_check_serve_listener_magic_aware] PASS — live serve listeners (node + produce) advertise the configured network magic via n2n_supported_for_magic; no static mainnet N2N_SUPPORTED at a live serve site"
