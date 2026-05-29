#!/usr/bin/env bash
#
# ci_check_sync_evidence_manifest_schema.sh — PHASE4-N-Y S5 gate
# (RO-SYNC-EVIDENCE-01, referencing the CN-OPERATOR-EVIDENCE-01 pattern).
#
# When a committed snapshot→tip sync-evidence manifest
# `docs/clusters/PHASE4-N-Y/CE-Y-SYNC-LIVE_*.toml` exists, verify it conforms
# to the closed schema: every required field is present and the referenced
# fixture file's sha256 matches `fixture_file_sha256`.
#
# When NO manifest is committed (the state until the operator-witnessed
# two-Haskell-node live pass runs, CE-Y-16), the gate is VACUOUSLY satisfied —
# no manifest, no constraint. This gate ships the schema; the live capture is
# operator action and is never executed or fabricated in CI.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

MANIFESTS=$(find docs/clusters/PHASE4-N-Y -name "CE-Y-SYNC-LIVE_*.toml" 2>/dev/null || true)

if [[ -z "$MANIFESTS" ]]; then
  echo "[ci_check_sync_evidence_manifest_schema] PASS (no manifest committed; vacuous)"
  exit 0
fi

REQUIRED_FIELDS=(
  "schema_version"
  "ade_commit"
  "cardano_node_version"
  "cardano_cli_version"
  "network"
  "start_point"
  "end_point"
  "fixture_file"
  "fixture_file_sha256"
  "diff_result"
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
    echo "[ci_check_sync_evidence_manifest_schema] FAIL — $manifest missing required field(s):$MISSING"
    FAIL=1
    continue
  fi

  # Verify fixture_file_sha256 against the actual referenced file.
  fixture_rel=$(grep "^fixture_file\s*=" "$manifest" | head -1 | sed -E 's/^fixture_file\s*=\s*"([^"]+)"/\1/')
  expected_sha=$(grep "^fixture_file_sha256\s*=" "$manifest" | head -1 | sed -E 's/^fixture_file_sha256\s*=\s*"([^"]+)"/\1/')
  manifest_dir=$(dirname "$manifest")
  actual_file="$manifest_dir/$fixture_rel"
  if [[ ! -f "$actual_file" ]]; then
    echo "[ci_check_sync_evidence_manifest_schema] FAIL — $manifest references missing fixture_file: $actual_file"
    FAIL=1
    continue
  fi
  actual_sha=$(sha256sum "$actual_file" | awk '{print $1}')
  if [[ "$expected_sha" != "$actual_sha" ]]; then
    echo "[ci_check_sync_evidence_manifest_schema] FAIL — $manifest fixture_file_sha256 mismatch:"
    echo "  expected: $expected_sha"
    echo "  actual:   $actual_sha"
    FAIL=1
  fi
done

if [[ $FAIL -eq 1 ]]; then
  exit 1
fi

echo "[ci_check_sync_evidence_manifest_schema] PASS (all manifests valid)"
