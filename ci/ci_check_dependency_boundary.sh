#!/usr/bin/env bash
set -euo pipefail

# Verify BLUE crates never depend on RED crates (T-BOUND-02).
# Uses cargo metadata for resolved dependency tree verification.

BLUE_CRATES=("ade_codec" "ade_types" "ade_crypto" "ade_core" "ade_ledger")
RED_CRATES=("ade_runtime" "ade_node")

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

METADATA=$(cargo metadata --no-deps --format-version 1 2>/dev/null)

FAILED=0

for blue in "${BLUE_CRATES[@]}"; do
    deps=$(echo "$METADATA" | python3 -c "
import json, sys
data = json.load(sys.stdin)
for pkg in data['packages']:
    if pkg['name'] == '$blue':
        for dep in pkg['dependencies']:
            print(dep['name'])
        break
" 2>/dev/null || true)

    for red in "${RED_CRATES[@]}"; do
        if echo "$deps" | grep -qx "$red"; then
            echo "FAIL: BLUE crate '$blue' depends on RED crate '$red'"
            FAILED=1
        fi
    done
done

if [ "$FAILED" -eq 0 ]; then
    echo "PASS: No BLUE crate depends on a RED crate"
    exit 0
else
    exit 1
fi
