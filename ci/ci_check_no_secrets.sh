#!/usr/bin/env bash
set -euo pipefail

# Scan git-tracked files for accidental credential patterns.
# This is a public repository — no credentials, hostnames, IPs,
# keys, or connection details may be committed.
#
# Scope: Operational secret hygiene only.
# Does NOT handle provenance validation (see ci_check_ref_provenance.sh).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Write the git-tracked file list to a temp FILE and pass its PATH to python3 —
# NOT the list itself via an env var. The list is large enough that exporting it
# into the python3 process environment exceeds ARG_MAX (execve packs args + env
# together), failing as "Argument list too long" (E2BIG) — the secret scan then
# silently never runs (exit 126), providing zero protection.
export REPO_ROOT
GIT_FILES_LIST="$(mktemp)"
trap 'rm -f "$GIT_FILES_LIST"' EXIT
(cd "$REPO_ROOT" && git ls-files) > "$GIT_FILES_LIST"
export GIT_FILES_LIST

python3 << 'PYEOF'
import os
import re
import sys

REPO_ROOT = os.environ.get("REPO_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GIT_FILES_LIST = os.environ.get("GIT_FILES_LIST", "")

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
    # Synthetic / documentation placeholder IPs used in tests + docstrings, never
    # real infrastructure: all-four-octets-identical (1.1.1.1, 2.2.2.2, 3.3.3.3, …)
    # and the canonical sequential example 1.2.3.4.
    octets = match_text.split(".")
    if len(octets) == 4:
        if len(set(octets)) == 1:
            return True
        if octets == ["1", "2", "3", "4"]:
            return True
    return False

with open(GIT_FILES_LIST, "r", errors="ignore") as _flist:
    tracked_files = [line.rstrip("\n") for line in _flist if line.strip()]

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
                            # Version-number-shaped token (NOT an IP): a 4-part
                            # dotted number in a version context — the word
                            # "version", or a tool/lib that carries a 4-part
                            # version (OpenSSL, cardano-cli, cardano-node, ghc),
                            # or a "generated using" header. e.g. "OpenSSL 3.0.14.4",
                            # "cardano-cli 11.0.0.0", "version": "0.26.0.3".
                            if re.search(
                                r'version|openssl|cardano-(cli|node)|\bghc\b|generated using',
                                line,
                                re.IGNORECASE,
                            ):
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
