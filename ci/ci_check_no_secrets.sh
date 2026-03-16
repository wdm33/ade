#!/usr/bin/env bash
set -euo pipefail

# Scan git-tracked files for accidental credential patterns.
# This is a public repository — no credentials, hostnames, IPs,
# keys, or connection details may be committed.
#
# Scope: Operational secret hygiene only.
# Does NOT handle provenance validation (see ci_check_ref_provenance.sh).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Get list of git-tracked files
export REPO_ROOT
export GIT_FILES
GIT_FILES=$(cd "$REPO_ROOT" && git ls-files)

python3 << 'PYEOF'
import os
import re
import sys

REPO_ROOT = os.environ.get("REPO_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GIT_FILES = os.environ.get("GIT_FILES", "")

failed = False

def fail(msg):
    global failed
    print(f"FAIL: {msg}")
    failed = True

# Patterns that indicate accidental credential commits
PATTERNS = [
    # Private key file extensions
    (r'\.pem\b', "PEM key file reference"),
    (r'\.key\b', "key file reference"),
    (r'id_rsa', "SSH private key reference"),
    # AWS-specific hostnames
    (r'ec2-\d+-\d+-\d+-\d+', "AWS EC2 hostname"),
    (r'[a-z0-9-]+\.amazonaws\.com', "AWS hostname"),
    # IP addresses (IPv4) — skip obvious non-sensitive patterns
    (r'\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b', "IPv4 address"),
    # SSH connection strings
    (r'ssh\s+\S+@\S+', "SSH connection string"),
    # AWS access keys
    (r'AKIA[0-9A-Z]{16}', "AWS access key ID"),
    # Generic secret patterns
    (r'BEGIN\s+(RSA\s+)?PRIVATE\s+KEY', "private key block"),
]

# IP addresses that are safe to ignore (localhost, documentation ranges, etc.)
SAFE_IP_PATTERNS = [
    r'127\.0\.0\.\d+',     # loopback
    r'0\.0\.0\.0',          # any
    r'255\.255\.255\.\d+',  # broadcast/netmask
    r'192\.168\.\d+\.\d+',  # private (used in examples)
    r'10\.\d+\.\d+\.\d+',   # private (used in examples)
    r'172\.(1[6-9]|2\d|3[01])\.\d+\.\d+',  # private
    r'169\.254\.\d+\.\d+',  # link-local
    r'0\.0\b',              # version number fragments
]

# Files to skip by name
SKIP_FILES = {
    "ci_check_no_secrets.sh",  # this script itself
    ".gitignore",              # contains protective patterns, not secrets
}

# Paths containing these substrings are skipped
SKIP_PATTERNS_IN_PATH = {
    ".env.example",  # example files are OK
}

# Extensions to skip (binary or non-source)
SKIP_EXTENSIONS = {".cbor", ".json", ".toml", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".woff", ".woff2", ".ttf", ".md"}

def should_skip(relpath):
    basename = os.path.basename(relpath)
    if basename in SKIP_FILES:
        return True
    for pattern in SKIP_PATTERNS_IN_PATH:
        if pattern in relpath:
            return True
    _, ext = os.path.splitext(relpath)
    if ext.lower() in SKIP_EXTENSIONS:
        return True
    return False

def is_safe_ip(match_text):
    for safe_pattern in SAFE_IP_PATTERNS:
        if re.fullmatch(safe_pattern, match_text):
            return True
    return False

tracked_files = [f for f in GIT_FILES.splitlines() if f.strip()]

scanned = 0
violations = 0

for relpath in tracked_files:
    if should_skip(relpath):
        continue

    filepath = os.path.join(REPO_ROOT, relpath)
    if not os.path.isfile(filepath):
        continue

    try:
        with open(filepath, "r", errors="ignore") as f:
            for lineno, line in enumerate(f, 1):
                for pattern, description in PATTERNS:
                    for match in re.finditer(pattern, line):
                        match_text = match.group(0)

                        # Skip safe IP addresses
                        if "IPv4" in description and is_safe_ip(match_text):
                            continue

                        # Skip version numbers that look like IPs
                        if "IPv4" in description:
                            parts = match_text.split(".")
                            if len(parts) != 4:
                                continue
                            # Check surrounding context: version strings are
                            # preceded/followed by word chars or inside quotes
                            # after version-like keys
                            start = match.start()
                            end = match.end()
                            before = line[start-1] if start > 0 else " "
                            after = line[end] if end < len(line) else " "
                            if before.isalpha() or after.isalpha():
                                continue
                            # "version": "0.26.0.3" — version in key name
                            if re.search(r'version', line, re.IGNORECASE):
                                continue

                        fail(f"{relpath}:{lineno}: {description} — '{match_text}'")
                        violations += 1
    except (UnicodeDecodeError, PermissionError):
        continue

    scanned += 1

print(f"Scanned {scanned} git-tracked files, found {violations} potential secret(s).")

if failed:
    print("RESULT: FAILED")
    sys.exit(1)
else:
    print("RESULT: PASS")
    sys.exit(0)
PYEOF
