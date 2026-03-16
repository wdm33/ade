#!/usr/bin/env bash
set -euo pipefail

# Validate DC-REF-01 provenance integrity for all reference artifacts.
# Scans corpus/reference/*/manifest.toml, validates:
#   - All required fields present and non-empty
#   - SHA-256 checksums match file content
#   - No untracked files missing from manifest
#
# Scope: DC-REF-01 provenance integrity only.
# Does NOT handle secret scanning (see ci_check_no_secrets.sh).
#
# Uses python3 with tomllib (Python 3.11+).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REFERENCE_DIR="$REPO_ROOT/corpus/reference"

if [ ! -d "$REFERENCE_DIR" ]; then
    echo "No corpus/reference directory found — nothing to validate."
    echo "RESULT: PASS"
    exit 0
fi

python3 << 'PYEOF'
import tomllib
import hashlib
import os
import sys
import glob

REPO_ROOT = os.environ.get("REPO_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
REFERENCE_DIR = os.path.join(REPO_ROOT, "corpus", "reference")

failed = False

def fail(msg):
    global failed
    print(f"FAIL: {msg}")
    failed = True

REQUIRED_STRING_FIELDS = [
    "file", "source_block", "era", "type",
    "extraction_tool", "extraction_tool_version", "extraction_tool_git_rev",
    "cardano_node_version", "protocol_version",
    "extraction_method", "extraction_date", "source_type",
    "reproducibility", "sha256",
]

REQUIRED_FIELDS = REQUIRED_STRING_FIELDS + ["network_magic"]

manifests_found = 0

for manifest_path in sorted(glob.glob(os.path.join(REFERENCE_DIR, "*", "manifest.toml"))):
    manifests_found += 1
    manifest_dir = os.path.dirname(manifest_path)
    corpus_name = os.path.basename(manifest_dir)

    print(f"Checking: {corpus_name}/manifest.toml")

    # Parse TOML
    try:
        with open(manifest_path, "rb") as f:
            data = tomllib.load(f)
    except Exception as e:
        fail(f"Invalid TOML in {manifest_path}: {e}")
        continue

    entries = data.get("entries", [])
    tracked_files = set()

    for i, entry in enumerate(entries):
        entry_file = entry.get("file", f"<entry {i}>")

        # Check all required fields present and non-empty
        for field in REQUIRED_FIELDS:
            if field not in entry:
                fail(f"{corpus_name}: entry '{entry_file}' missing field '{field}'")
            elif field in REQUIRED_STRING_FIELDS and not str(entry[field]).strip():
                fail(f"{corpus_name}: entry '{entry_file}' has empty field '{field}'")

        # Verify SHA-256 checksum
        if "file" in entry and "sha256" in entry:
            artifact_path = os.path.join(manifest_dir, entry["file"])
            if os.path.exists(artifact_path):
                with open(artifact_path, "rb") as af:
                    actual_sha256 = hashlib.sha256(af.read()).hexdigest()
                if actual_sha256 != entry["sha256"]:
                    fail(f"{corpus_name}: checksum mismatch for '{entry_file}': "
                         f"expected {entry['sha256']}, got {actual_sha256}")
            else:
                fail(f"{corpus_name}: artifact file not found: {artifact_path}")

            tracked_files.add(entry["file"])

    # Check for untracked reference data files not in manifest
    # Skip documentation (.md) and manifest files (.toml)
    SKIP_EXTENSIONS = {".md", ".toml"}
    for root, dirs, files in os.walk(manifest_dir):
        for fname in files:
            _, ext = os.path.splitext(fname)
            if ext.lower() in SKIP_EXTENSIONS:
                continue
            rel = os.path.relpath(os.path.join(root, fname), manifest_dir)
            if rel not in tracked_files:
                fail(f"{corpus_name}: untracked file not in manifest: '{rel}'")

if manifests_found == 0:
    print("No reference manifests found — nothing to validate.")

print(f"\nChecked {manifests_found} manifest(s).")

if failed:
    print("RESULT: FAILED")
    sys.exit(1)
else:
    print("RESULT: PASS")
    sys.exit(0)
PYEOF
