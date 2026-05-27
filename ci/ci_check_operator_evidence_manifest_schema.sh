#!/usr/bin/env bash
#
# ci_check_operator_evidence_manifest_schema.sh — PHASE4-N-S-C gate.
#
# When a paired-evidence manifest file
# `docs/clusters/PHASE4-N-S-C/CE-N-S-LIVE_*.toml` is
# committed, verify it conforms to the closed schema:
# every required field is present and `peer_log_file_sha256`
# matches the actual SHA-256 of the committed peer.log
# file.
#
# When NO manifest is committed (typical state pre-operator-
# pass), the gate is vacuously satisfied — no manifest, no
# constraint.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

MANIFESTS=$(find docs/clusters/PHASE4-N-S-C -name "CE-N-S-LIVE_*.toml" 2>/dev/null || true)

if [[ -z "$MANIFESTS" ]]; then
  echo "[ci_check_operator_evidence_manifest_schema] PASS (no manifest committed; vacuous)"
  exit 0
fi

REQUIRED_FIELDS=(
  "schema_version"
  "ade_commit"
  "cardano_node_version"
  "cardano_cli_version"
  "network"
  "block_hash"
  "slot"
  "opcert_fingerprint"
  "genesis_fingerprint"
  "ade_evidence_file"
  "peer_log_file"
  "peer_log_capture_command"
  "peer_log_filter"
  "peer_log_file_sha256"
  "acceptance_keyword_match"
)

FAIL=0
for manifest in $MANIFESTS; do
  MISSING=""
  for field in "${REQUIRED_FIELDS[@]}"; do
    if ! grep -q "^${field}\s*=" "$manifest"; then
      MISSING+=" $field"
    fi
  done
  if [[ -n "$MISSING" ]]; then
    echo "[ci_check_operator_evidence_manifest_schema] FAIL — $manifest missing required field(s):$MISSING"
    FAIL=1
    continue
  fi

  # Verify peer_log_file_sha256 against actual file hash.
  peer_log_rel=$(grep "^peer_log_file\s*=" "$manifest" | head -1 | sed -E 's/^peer_log_file\s*=\s*"([^"]+)"/\1/')
  expected_sha=$(grep "^peer_log_file_sha256\s*=" "$manifest" | head -1 | sed -E 's/^peer_log_file_sha256\s*=\s*"([^"]+)"/\1/')
  manifest_dir=$(dirname "$manifest")
  actual_file="$manifest_dir/$peer_log_rel"
  if [[ ! -f "$actual_file" ]]; then
    echo "[ci_check_operator_evidence_manifest_schema] FAIL — $manifest references missing peer_log_file: $actual_file"
    FAIL=1
    continue
  fi
  actual_sha=$(sha256sum "$actual_file" | awk '{print $1}')
  if [[ "$expected_sha" != "$actual_sha" ]]; then
    echo "[ci_check_operator_evidence_manifest_schema] FAIL — $manifest peer_log_file_sha256 mismatch:"
    echo "  expected: $expected_sha"
    echo "  actual:   $actual_sha"
    FAIL=1
  fi
done

if [[ $FAIL -eq 1 ]]; then
  exit 1
fi

echo "[ci_check_operator_evidence_manifest_schema] PASS (all manifests valid)"
