#!/usr/bin/env bash
# ci_check_registry_unique_ids.sh -- invariant-registry ID uniqueness guard.
#
# The invariant registry is append-only: every rule has a stable, immutable `id`, and an id is
# NEVER reassigned or reused (even after deprecation). Two rules sharing one id is an append-only
# violation -- it silently overwrites one rule's traceability with another's. This gate parses
# the IDSE registry docs/ade-invariant-registry.toml and the frozen legacy registry
# docs/legacy/idd-invariant-registry.toml, collects every rule `id` in each, and FAILS listing any
# id that appears more than once in either.
set -euo pipefail

# Resolve the repo root from this script's location (ci/<script>) so the gate runs from anywhere.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
IDSE_REGISTRY="${ROOT}/docs/ade-invariant-registry.toml"
LEGACY_REGISTRY="${ROOT}/docs/legacy/idd-invariant-registry.toml"

fail() { echo "FAIL (ci_check_registry_unique_ids): $1" >&2; exit 1; }
for REGISTRY in "${IDSE_REGISTRY}" "${LEGACY_REGISTRY}"; do
[ -f "${REGISTRY}" ] || fail "registry ${REGISTRY} missing"

python3 - "${REGISTRY}" <<'PY'
import collections
import sys
import tomllib

path = sys.argv[1]
with open(path, "rb") as f:
    doc = tomllib.load(f)

rules = doc.get("rules", [])
ids = [r["id"] for r in rules]
dups = {k: v for k, v in collections.Counter(ids).items() if v > 1}

if dups:
    print("FAIL (ci_check_registry_unique_ids): duplicate invariant id(s):", file=sys.stderr)
    for rule_id in sorted(dups):
        print(f"  {rule_id}: appears {dups[rule_id]} times", file=sys.stderr)
    sys.exit(1)

print(f"OK: {path}: {len(ids)} invariant ids, all unique (ci_check_registry_unique_ids)")
PY
done
