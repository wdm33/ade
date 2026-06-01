#!/usr/bin/env bash
#
# ci_check_ba02_evidence_manifest_schema.sh — PHASE4-N-F-G-C S2 gate
# (RO-LIVE-06 BA-02 evidence; mirrors ci_check_operator_evidence_manifest_schema.sh).
#
# When a committed BA-02 evidence manifest
# `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml` is present, verify it
# conforms to the closed schema (every required field present) AND that
# `peer_log_file_sha256` matches the actual SHA-256 of the committed
# operator-captured peer-log file it binds.
#
# When NO manifest is committed (the typical state — the live operator pass is
# `blocked_until_operator_stake_available`), the gate is vacuously satisfied:
# no manifest, no claim.
#
# What this gate enforces (the "no synthetic manifest" line): a committed
# manifest MUST bind a REAL committed peer-log fixture by matching sha256 — a
# hand-authored manifest with no real fixture, or a tampered fixture, FAILS.
# The complementary provenance fence is in code: `ba02_pass::write_ba02_manifest`
# accepts ONLY a `Ba02Manifest`, which ONLY `ba02_evidence::correlate`'s
# exact-match arm constructs (the sole `Ba02Manifest` constructor) — so a
# written manifest is always correlate-produced from a real peer log. Ade
# self-accept / `ForgeSucceeded` / served-block / wire success are NOT evidence.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

MANIFESTS=$(find docs/clusters/PHASE4-N-F-G-C -name "CE-G-C-LIVE_*.toml" 2>/dev/null || true)

if [[ -z "$MANIFESTS" ]]; then
  echo "[ci_check_ba02_evidence_manifest_schema] PASS (no manifest committed; vacuous — live ACCEPT is operator-gated)"
  exit 0
fi

REQUIRED_FIELDS=(
  "schema_version"
  "block_hash"
  "slot"
  "peer_log_file"
  "peer_log_file_sha256"
  "peer_log_capture_command"
  "peer_log_filter"
  "accept_event_kind"
)

FAIL=0
for manifest in $MANIFESTS; do
  MISSING=""
  for field in "${REQUIRED_FIELDS[@]}"; do
    if ! grep -qE "^${field}\s*=" "$manifest"; then
      MISSING+=" $field"
    fi
  done
  if [[ -n "$MISSING" ]]; then
    echo "[ci_check_ba02_evidence_manifest_schema] FAIL — $manifest missing required field(s):$MISSING"
    FAIL=1
    continue
  fi

  # schema_version must equal the canonical BA02_MANIFEST_SCHEMA_VERSION (1).
  schema_ver=$(grep -E "^schema_version\s*=" "$manifest" | head -1 | sed -E 's/^schema_version\s*=\s*([0-9]+).*/\1/')
  if [[ "$schema_ver" != "1" ]]; then
    echo "[ci_check_ba02_evidence_manifest_schema] FAIL — $manifest schema_version is '$schema_ver', expected 1 (BA02_MANIFEST_SCHEMA_VERSION)"
    FAIL=1
    continue
  fi

  # Verify peer_log_file_sha256 against the actual file hash (the no-synthetic
  # binding: the manifest must bind a REAL committed peer-log fixture).
  peer_log_rel=$(grep -E "^peer_log_file\s*=" "$manifest" | head -1 | sed -E 's/^peer_log_file\s*=\s*"([^"]+)"/\1/')
  expected_sha=$(grep -E "^peer_log_file_sha256\s*=" "$manifest" | head -1 | sed -E 's/^peer_log_file_sha256\s*=\s*"([^"]+)"/\1/')
  manifest_dir=$(dirname "$manifest")
  actual_file="$manifest_dir/$peer_log_rel"
  if [[ ! -f "$actual_file" ]]; then
    echo "[ci_check_ba02_evidence_manifest_schema] FAIL — $manifest references missing peer_log_file: $actual_file"
    FAIL=1
    continue
  fi
  actual_sha=$(sha256sum "$actual_file" | awk '{print $1}')
  if [[ "$expected_sha" != "$actual_sha" ]]; then
    echo "[ci_check_ba02_evidence_manifest_schema] FAIL — $manifest peer_log_file_sha256 mismatch:"
    echo "  expected: $expected_sha"
    echo "  actual:   $actual_sha"
    FAIL=1
  fi
done

if [[ $FAIL -eq 1 ]]; then
  exit 1
fi

echo "[ci_check_ba02_evidence_manifest_schema] PASS (all committed BA-02 manifests schema-valid + sha256-bound)"
