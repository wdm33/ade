#!/usr/bin/env bash
set -euo pipefail

# Validate constitution_registry.toml: schema, coverage, tiers, status,
# uniqueness, cross-ref bidirectionality, and enforcement regression guard.
# Uses python3 with tomllib (Python 3.11+).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REGISTRY="$REPO_ROOT/constitution_registry.toml"

# Source documents for mechanical ID extraction
PLAN_DOC="$HOME/Documents/ade-planning/ade_replay_first_constitutional_node_plan_v1.md"
CLASSIFICATION_TABLE="$HOME/Documents/ade-planning/classification_table.md"

if [ ! -f "$REGISTRY" ]; then
    echo "FAIL: constitution_registry.toml not found"
    exit 1
fi

python3 << 'PYEOF'
import tomllib
import re
import sys
import os

REGISTRY_PATH = os.environ.get("REGISTRY", "constitution_registry.toml")
PLAN_DOC = os.environ.get("PLAN_DOC", "")
CLASSIFICATION_TABLE = os.environ.get("CLASSIFICATION_TABLE", "")

failed = False

def fail(msg):
    global failed
    print(f"FAIL: {msg}")
    failed = True

# --- 1. Parse TOML ---
try:
    with open(REGISTRY_PATH, "rb") as f:
        data = tomllib.load(f)
except Exception as e:
    print(f"FAIL: Invalid TOML: {e}")
    sys.exit(1)

if "rules" not in data:
    print("FAIL: No [[rules]] entries found")
    sys.exit(1)

rules = data["rules"]

# --- 2. Check for duplicate IDs ---
ids = [r["id"] for r in rules]
seen = set()
for rid in ids:
    if rid in seen:
        fail(f"Duplicate ID: {rid}")
    seen.add(rid)

# --- 3. Build ID-to-rule map ---
id_map = {}
for r in rules:
    id_map[r["id"]] = r

# --- 4. Tier-to-ID prefix validation ---
# T-* must be "true", DC-* must be "derived", RO-* must be "release", OP-* must be "operational"
# CN-* validated separately against classification table
for r in rules:
    rid = r["id"]
    tier = r.get("tier", "")

    if rid.startswith("T-") and tier != "true":
        fail(f"{rid}: T-* entry must have tier 'true', got '{tier}'")
    elif rid.startswith("DC-") and tier != "derived":
        fail(f"{rid}: DC-* entry must have tier 'derived', got '{tier}'")
    elif rid.startswith("RO-") and tier != "release":
        fail(f"{rid}: RO-* entry must have tier 'release', got '{tier}'")
    elif rid.startswith("OP-") and tier != "operational":
        fail(f"{rid}: OP-* entry must have tier 'operational', got '{tier}'")

# --- 5. Validate status ---
VALID_STATUSES = {"declared", "partial", "enforced"}
for r in rules:
    status = r.get("status", "")
    if status not in VALID_STATUSES:
        fail(f"{r['id']}: invalid status '{status}', must be one of: {', '.join(sorted(VALID_STATUSES))}")

# --- 6. Tier-appropriate field validation ---
TRUE_DERIVED_REQUIRED = ["id", "tier", "statement", "source", "cross_ref", "code_locus", "tests", "ci_script", "status"]
RELEASE_OP_REQUIRED = ["id", "tier", "statement", "source", "cross_ref", "status"]
RELEASE_OP_FORBIDDEN = ["code_locus", "tests", "ci_script"]
CN_TRUE_DERIVED_EXTRA = ["attack_rationale", "evidence_notes"]

for r in rules:
    rid = r["id"]
    tier = r.get("tier", "")

    if tier in ("true", "derived"):
        for field in TRUE_DERIVED_REQUIRED:
            if field not in r:
                fail(f"{rid}: missing required field '{field}' for {tier} entry")

        # CN-* true/derived need extra fields
        if rid.startswith("CN-"):
            for field in CN_TRUE_DERIVED_EXTRA:
                if field not in r:
                    fail(f"{rid}: CN-* {tier} entry missing '{field}'")

    elif tier in ("release", "operational"):
        for field in RELEASE_OP_REQUIRED:
            if field not in r:
                fail(f"{rid}: missing required field '{field}' for {tier} entry")
        for field in RELEASE_OP_FORBIDDEN:
            if field in r:
                fail(f"{rid}: {tier} entry must NOT have '{field}'")
    else:
        fail(f"{rid}: invalid tier '{tier}'")

# --- 7. Enforcement regression guard ---
for r in rules:
    status = r.get("status")
    if status == "enforced":
        for field in ["code_locus", "ci_script"]:
            if not r.get(field):
                fail(f"{r['id']}: 'enforced' entry must have non-empty '{field}'")
        if not r.get("tests"):
            fail(f"{r['id']}: 'enforced' entry must have non-empty 'tests'")
    elif status == "partial":
        has_evidence = bool(r.get("code_locus")) or bool(r.get("tests")) or bool(r.get("ci_script"))
        if not has_evidence:
            fail(f"{r['id']}: 'partial' entry must have at least one non-empty code_locus, tests, or ci_script")

# --- 8. Cross-ref bidirectionality ---
for r in rules:
    rid = r["id"]
    for ref in r.get("cross_ref", []):
        if ref not in id_map:
            fail(f"{rid}: cross_ref '{ref}' not found in registry")
        elif rid not in id_map[ref].get("cross_ref", []):
            fail(f"{rid}: cross_ref to '{ref}' is not bidirectional ('{ref}' does not reference '{rid}')")

# --- 9. Extract expected IDs from source documents ---

def extract_ids_from_first_column(filepath, id_prefix_pattern):
    """Extract IDs matching pattern from the FIRST data column of markdown tables."""
    extracted = set()
    if not os.path.exists(filepath):
        return extracted
    with open(filepath, "r") as f:
        for line in f:
            line = line.strip()
            if line.startswith("|"):
                cells = [c.strip() for c in line.split("|")]
                # First data column is cells[1] (cells[0] is empty string before first |)
                if len(cells) > 1:
                    first_col = cells[1]
                    m = re.fullmatch(id_prefix_pattern, first_col)
                    if m:
                        extracted.add(first_col)
    return extracted

def extract_cn_tiers_from_classification_table(filepath):
    """Extract CN-* IDs and their tiers from the classification table body tables."""
    cn_tiers = {}
    if not os.path.exists(filepath):
        return cn_tiers
    with open(filepath, "r") as f:
        for line in f:
            line_stripped = line.strip()
            if not line_stripped.startswith("|"):
                continue
            cells = [c.strip() for c in line_stripped.split("|")]
            # Find cells matching CN-* pattern
            cn_id = None
            tier_val = None
            for i, cell in enumerate(cells):
                if re.match(r"CN-[A-Z]+-\d+", cell):
                    cn_id = cell
                # Tier is typically in the 4th column (index 3 after split)
                # Look for bold tier markers like **true**, **derived**, etc.
                tier_match = re.match(r"\*\*(true|derived|release|operational)\*\*", cell)
                if tier_match:
                    tier_val = tier_match.group(1)
            if cn_id and tier_val:
                cn_tiers[cn_id] = tier_val
    return cn_tiers

# Extract expected IDs from plan document
if os.path.exists(PLAN_DOC):
    expected_t = extract_ids_from_first_column(PLAN_DOC, r"T-[A-Z]+-\d+")
    expected_dc = extract_ids_from_first_column(PLAN_DOC, r"DC-[A-Z]+-\d+")
    expected_ro = extract_ids_from_first_column(PLAN_DOC, r"RO-[A-Z]+-\d+")
    expected_op = extract_ids_from_first_column(PLAN_DOC, r"OP-[A-Z]+-\d+")

    # Check T-* coverage
    registry_t = {r["id"] for r in rules if r["id"].startswith("T-")}
    for tid in expected_t:
        if tid not in registry_t:
            fail(f"Missing T-* entry: {tid} (from plan §2)")
    for tid in registry_t:
        if tid not in expected_t:
            fail(f"Extra T-* entry: {tid} (not in plan §2)")

    # Check DC-* coverage
    registry_dc = {r["id"] for r in rules if r["id"].startswith("DC-")}
    for did in expected_dc:
        if did not in registry_dc:
            fail(f"Missing DC-* entry: {did} (from plan §3)")
    for did in registry_dc:
        if did not in expected_dc:
            fail(f"Extra DC-* entry: {did} (not in plan §3)")

    # Check RO-* coverage
    registry_ro = {r["id"] for r in rules if r["id"].startswith("RO-")}
    for rid in expected_ro:
        if rid not in registry_ro:
            fail(f"Missing RO-* entry: {rid} (from plan §4a)")
    for rid in registry_ro:
        if rid not in expected_ro:
            fail(f"Extra RO-* entry: {rid} (not in plan §4a)")

    # Check OP-* coverage
    registry_op = {r["id"] for r in rules if r["id"].startswith("OP-")}
    for oid in expected_op:
        if oid not in registry_op:
            fail(f"Missing OP-* entry: {oid} (from plan §4b)")
    for oid in registry_op:
        if oid not in expected_op:
            fail(f"Extra OP-* entry: {oid} (not in plan §4b)")
else:
    print(f"WARN: Plan document not found at {PLAN_DOC}, skipping T/DC/RO/OP coverage check")

# Extract expected CN-* IDs and validate tiers from classification table
if os.path.exists(CLASSIFICATION_TABLE):
    cn_tiers = extract_cn_tiers_from_classification_table(CLASSIFICATION_TABLE)

    registry_cn = {r["id"]: r for r in rules if r["id"].startswith("CN-")}

    for cn_id, expected_tier in cn_tiers.items():
        if cn_id not in registry_cn:
            fail(f"Missing CN-* entry: {cn_id} (from classification_table.md)")
        else:
            actual_tier = registry_cn[cn_id].get("tier", "")
            if actual_tier != expected_tier:
                fail(f"{cn_id}: tier must be '{expected_tier}' (per classification_table.md), got '{actual_tier}'")

    for cn_id in registry_cn:
        if cn_id not in cn_tiers:
            fail(f"Extra CN-* entry: {cn_id} (not in classification_table.md)")
else:
    print(f"WARN: Classification table not found at {CLASSIFICATION_TABLE}, skipping CN-* coverage check")

# --- Summary ---
total = len(rules)
by_prefix = {}
for r in rules:
    prefix = r["id"].split("-")[0]
    by_prefix[prefix] = by_prefix.get(prefix, 0) + 1

print(f"Registry: {total} entries ({', '.join(f'{k}:{v}' for k,v in sorted(by_prefix.items()))})")

if failed:
    print("RESULT: FAILED")
    sys.exit(1)
else:
    print("RESULT: PASS")
    sys.exit(0)
PYEOF
