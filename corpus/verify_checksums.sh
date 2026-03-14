#!/usr/bin/env bash
set -euo pipefail

# Verify all corpus fixtures against their manifest SHA-256 checksums.
# Uses python3 with tomllib for reliable TOML parsing.
# Exits 0 on all pass, 1 on any mismatch or missing file.

CORPUS_DIR="$(cd "$(dirname "$0")" && pwd)"
GOLDEN_DIR="$CORPUS_DIR/golden"

ERAS=(byron shelley allegra mary alonzo babbage conway)

TOTAL=0
PASSED=0
FAILED=0

for era in "${ERAS[@]}"; do
    MANIFEST="$GOLDEN_DIR/$era/manifest.toml"

    if [ ! -f "$MANIFEST" ]; then
        echo "WARN: No manifest for $era"
        continue
    fi

    # Parse manifest and verify each fixture
    python3 << PYEOF
import tomllib
import hashlib
import sys
import os

manifest_path = "$MANIFEST"
era_dir = "$GOLDEN_DIR/$era"

with open(manifest_path, "rb") as f:
    try:
        data = tomllib.load(f)
    except Exception as e:
        print(f"FAIL: {manifest_path}: invalid TOML: {e}")
        sys.exit(1)

fixtures = data.get("fixtures", [])
if not fixtures:
    sys.exit(0)

required_fields = ["file", "era", "type", "sha256",
                    "source", "fetch_tool", "fetch_date", "reproducibility"]

failed = False
for fix in fixtures:
    # Check all required fields present and non-empty
    for field in required_fields:
        if field not in fix or (isinstance(fix[field], str) and not fix[field].strip()):
            print(f"FAIL: {manifest_path}: fixture missing or empty field '{field}'")
            failed = True

    filepath = os.path.join(era_dir, fix.get("file", ""))
    expected_sha = fix.get("sha256", "")

    if not os.path.exists(filepath):
        print(f"FAIL: Missing file: {filepath}")
        failed = True
        continue

    # Compute actual SHA-256
    h = hashlib.sha256()
    with open(filepath, "rb") as bf:
        for chunk in iter(lambda: bf.read(8192), b""):
            h.update(chunk)
    actual_sha = h.hexdigest()

    if actual_sha != expected_sha:
        print(f"FAIL: {filepath}: sha256 mismatch")
        print(f"  expected: {expected_sha}")
        print(f"  actual:   {actual_sha}")
        failed = True
    else:
        print(f"  OK: {filepath}")

sys.exit(1 if failed else 0)
PYEOF

    result=$?
    if [ $result -ne 0 ]; then
        FAILED=$((FAILED + 1))
    else
        PASSED=$((PASSED + 1))
    fi
done

echo ""
echo "Verification: $PASSED era(s) passed, $FAILED era(s) failed"

if [ "$FAILED" -gt 0 ]; then
    echo "RESULT: FAILED"
    exit 1
else
    echo "RESULT: PASS"
    exit 0
fi
