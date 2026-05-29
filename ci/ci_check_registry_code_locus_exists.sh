#!/usr/bin/env bash
set -euo pipefail

# Traceability drift guard: every CODE path cited in a registry rule's
# `code_locus` must exist on disk. Catches stale pointers after a file
# move/rename (e.g. PHASE4-N-Y S3 renamed recovery.rs -> recovery/mod.rs,
# leaving DC-STORE-05 / T-REC-01 / T-REC-02 pointing at the old path —
# a SOFT drift the schema validator did not catch).
#
# Scope (deliberately narrow, to stay green + false-positive-free):
#   - Only crates/**.rs and ci/**.sh tokens (the load-bearing code + gate
#     pointers; existence is unambiguous).
#   - Tokens containing a glob '*' are skipped (e.g. docs globs, dir-wildcards).
#   - docs/ paths are NOT hard-checked here (they use globs + are archived on
#     cluster close); a doc-path coherence pass is a separate concern.
#
# Repo-root-relative. python3 + tomllib (3.11+).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

python3 - <<'PYEOF'
import tomllib, re, os, sys

REG = "docs/ade-invariant-registry.toml"
try:
    rules = tomllib.load(open(REG, "rb"))["rules"]
except Exception as e:
    print(f"FAIL: cannot parse {REG}: {e}")
    sys.exit(1)

# crates/**.rs or ci/**.sh, allowing word chars, dots, slashes, dashes.
TOKEN = re.compile(r"(?:crates|ci)/[\w./\-]+\.(?:rs|sh)")
missing = []
checked = 0
for r in rules:
    loc = r.get("code_locus", "")
    if not loc:
        continue
    for tok in TOKEN.findall(loc):
        if "*" in tok:
            continue
        checked += 1
        if not os.path.exists(tok):
            missing.append((r["id"], tok))

if missing:
    print("FAIL: registry code_locus cites code path(s) that do not exist "
          "(moved/renamed? — update the code_locus or restore the path):")
    for rid, tok in missing:
        print(f"    {rid}: {tok}")
    sys.exit(1)

print(f"OK: all {checked} crates/**.rs + ci/**.sh code_locus paths exist "
      f"({len(rules)} rules scanned)")
PYEOF
