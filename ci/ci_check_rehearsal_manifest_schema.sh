#!/usr/bin/env bash
#
# ci_check_rehearsal_manifest_schema.sh — PHASE4-N-F-G-D S2 gate
# (CN-REHEARSAL-FIDELITY-01 clause 2 — non-promotable rehearsal evidence).
#
# When a committed private-testnet rehearsal manifest is present — the G-D bounty
# dry-run (`docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`) or the G-J
# genesis-successor rehearsal (`docs/evidence/phase4-n-f-g-j-genesis-rehearsal-*.toml`)
# — verify the closed rehearsal schema + the non-promotability markers + the
# sha256 binding to the committed peer log. When NONE is committed (the typical
# state — the C1 rehearsals are operator-gated), the gate is vacuously satisfied.
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
# Rehearsal-manifest homes this gate covers: the G-D bounty dry-run and the G-J
# genesis-successor rehearsal. Each is a non-promotable private-testnet manifest
# held to the identical closed schema + markers + sha256 binding.
REHEARSAL_GLOBS=("phase4-n-f-g-d-private-rehearsal-*.toml" "phase4-n-f-g-j-genesis-rehearsal-*.toml")
# All real bounty-evidence homes a CE-G-C-LIVE manifest could live in: the active
# operator-pass home AND the archived home (G-C is archived to completed/). A
# rehearsal marker must never appear in ANY of them. (PHASE4-N-F-G-D S4 — the
# pre-S4 gate scanned only the active home behind an `[[ -d ]]` guard, so after
# G-C's archival the whole check silently skipped.)
BOUNTY_HOMES=("docs/clusters/PHASE4-N-F-G-C" "docs/clusters/completed/PHASE4-N-F-G-C")

FAIL=0

# --- non-promotability cross-check: no rehearsal marker in any .toml under any
#     bounty home. Build the list of EXISTING homes first, then scan those.
#     "Home absent" => empty contribution (deliberate) — never "skip the check".
#     A scan ERROR on an EXISTING home (grep rc >= 2) => fail closed, NOT swallowed
#     (so `2>/dev/null || true` is deliberately NOT used over the existing homes). -
EXISTING_BOUNTY_HOMES=()
for h in "${BOUNTY_HOMES[@]}"; do
  [[ -d "$h" ]] && EXISTING_BOUNTY_HOMES+=("$h")
done
if (( ${#EXISTING_BOUNTY_HOMES[@]} > 0 )); then
  set +e
  LEAK="$(grep -rlE '^(is_rehearsal|not_bounty_evidence)[[:space:]]*=' "${EXISTING_BOUNTY_HOMES[@]}" --include='*.toml')"
  grep_rc=$?
  set -e
  if (( grep_rc >= 2 )); then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — leak scan errored on an existing bounty home (grep rc=$grep_rc); failing closed (an existing home that cannot be scanned is never silently passed)"
    FAIL=1
  elif [[ -n "$LEAK" ]]; then
    echo "[ci_check_rehearsal_manifest_schema] FAIL — rehearsal marker found in a .toml under a bounty home (a rehearsal manifest must never live in a bounty home):"
    echo "$LEAK"
    FAIL=1
  fi
fi

MANIFESTS="$(find "$REHEARSAL_HOME" \( -name "${REHEARSAL_GLOBS[0]}" -o -name "${REHEARSAL_GLOBS[1]}" \) 2>/dev/null || true)"

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
