#!/usr/bin/env bash
#
# ci_check_rehearsal_manifest_schema.sh — PHASE4-N-F-G-D S2 gate
# (CN-REHEARSAL-FIDELITY-01 clause 2 — non-promotable rehearsal evidence).
#
# When a committed private-testnet rehearsal manifest
# `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml` is present, verify the
# closed rehearsal schema + the non-promotability markers + the sha256 binding
# to the committed peer log. When NONE is committed (the typical state — the C1
# dry-run is operator-gated), the gate is vacuously satisfied.
#
# Non-promotability (three independent barriers):
#   - distinct rehearsal home (docs/evidence/, never the bounty home
#     docs/clusters/PHASE4-N-F-G-C/);
#   - explicit is_rehearsal = true + not_bounty_evidence = true markers;
#   - the bounty BA-02 gate (ci_check_ba02_evidence_manifest_schema.sh) globs
#     only docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml, so it cannot see a
#     rehearsal manifest; this gate additionally forbids rehearsal markers from
#     appearing in any .toml under the bounty home (a rehearsal manifest must
#     never leak there).
#
# A rehearsal manifest is correlate-produced by construction:
# rehearsal_pass::write_private_rehearsal_manifest accepts ONLY a
# PrivateRehearsalManifest, which only
# PrivateRehearsalManifest::from_correlate_outcome's Ba02Manifest arm builds
# (NoEvidence writes nothing). Ade self-accept / served bytes / wire success are
# NOT acceptance.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

REHEARSAL_HOME="docs/evidence"
REHEARSAL_GLOB="phase4-n-f-g-d-private-rehearsal-*.toml"
BOUNTY_HOME="docs/clusters/PHASE4-N-F-G-C"

FAIL=0

# --- non-promotability cross-check: no rehearsal marker in any .toml under the
#     bounty home (a rehearsal manifest must never live in the bounty home) -----
if [[ -d "$BOUNTY_HOME" ]]; then
  LEAK="$(grep -rlE '^(is_rehearsal|not_bounty_evidence)[[:space:]]*=' "$BOUNTY_HOME" --include='*.toml' 2>/dev/null || true)"
  if [[ -n "$LEAK" ]]; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — rehearsal marker found in a .toml under the bounty home $BOUNTY_HOME (a rehearsal manifest must never live in the bounty home):"
    echo "$LEAK"
    FAIL=1
  fi
fi

MANIFESTS="$(find "$REHEARSAL_HOME" -name "$REHEARSAL_GLOB" 2>/dev/null || true)"

if [[ -z "$MANIFESTS" ]]; then
  if [[ $FAIL -eq 0 ]]; then
    echo "[ci_check_rehearsal_manifest_schema] PASS (no rehearsal manifest committed; vacuous — the C1 dry-run is operator-gated)"
    exit 0
  fi
  exit 1
fi

REQUIRED_FIELDS=(
  "schema_version"
  "venue"
  "is_rehearsal"
  "not_bounty_evidence"
  "peer_log_file"
  "peer_log_file_sha256"
  "forged_block_hash_hex"
  "slot"
  "network_magic"
  "peer_accept_source"
  "peer"
  "matched_block_hash_hex"
)

for manifest in $MANIFESTS; do
  MISSING=""
  for field in "${REQUIRED_FIELDS[@]}"; do
    if ! grep -qE "^${field}[[:space:]]*=" "$manifest"; then
      MISSING+=" $field"
    fi
  done
  if [[ -n "$MISSING" ]]; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — $manifest missing required field(s):$MISSING"
    FAIL=1
    continue
  fi

  # schema_version must equal REHEARSAL_MANIFEST_SCHEMA_VERSION (1).
  schema_ver="$(grep -E '^schema_version[[:space:]]*=' "$manifest" | head -1 | sed -E 's/^schema_version[[:space:]]*=[[:space:]]*([0-9]+).*/\1/')"
  if [[ "$schema_ver" != "1" ]]; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — $manifest schema_version is '$schema_ver', expected 1 (REHEARSAL_MANIFEST_SCHEMA_VERSION)"
    FAIL=1
    continue
  fi

  # Non-promotability markers MUST be present and true.
  if ! grep -qE '^is_rehearsal[[:space:]]*=[[:space:]]*true' "$manifest"; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — $manifest must set is_rehearsal = true"
    FAIL=1
    continue
  fi
  if ! grep -qE '^not_bounty_evidence[[:space:]]*=[[:space:]]*true' "$manifest"; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — $manifest must set not_bounty_evidence = true"
    FAIL=1
    continue
  fi
  # venue must be a private-testnet venue.
  if ! grep -qE '^venue[[:space:]]*=[[:space:]]*"private-testnet' "$manifest"; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — $manifest venue must be a private-testnet venue"
    FAIL=1
    continue
  fi

  # peer_log_file_sha256 must match the committed peer-log file (no synthetic).
  peer_log_rel="$(grep -E '^peer_log_file[[:space:]]*=' "$manifest" | head -1 | sed -E 's/^peer_log_file[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/')"
  expected_sha="$(grep -E '^peer_log_file_sha256[[:space:]]*=' "$manifest" | head -1 | sed -E 's/^peer_log_file_sha256[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/')"
  manifest_dir="$(dirname "$manifest")"
  actual_file="$manifest_dir/$peer_log_rel"
  if [[ ! -f "$actual_file" ]]; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — $manifest references missing peer_log_file: $actual_file"
    FAIL=1
    continue
  fi
  actual_sha="$(sha256sum "$actual_file" | awk '{print $1}')"
  if [[ "$expected_sha" != "$actual_sha" ]]; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — $manifest peer_log_file_sha256 mismatch:"
    echo "  expected: $expected_sha"
    echo "  actual:   $actual_sha"
    FAIL=1
  fi
done

if [[ $FAIL -eq 1 ]]; then
  exit 1
fi

echo "[ci_check_rehearsal_manifest_schema] PASS (all committed rehearsal manifests schema-valid + sha256-bound + non-promotable)"
