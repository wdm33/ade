#!/usr/bin/env bash
set -euo pipefail

# Registry coherence guard. Three load-bearing, Ade-correct invariants over
# docs/ade-invariant-registry.toml:
#   1. Rule IDs are unique (the registry is append-only; IDs are never reused).
#   2. Every `cross_ref` resolves to a real rule ID. cross_ref is a DIRECTED
#      relation (depends-on / refines / see-also); reciprocity is NOT required
#      and is intentionally NOT checked (forcing it would invert dependencies
#      and bloat foundational rules like T-DET-01).
#   3. Every CODE path cited in a rule's `code_locus` exists on disk — catches
#      stale pointers after a move/rename (e.g. PHASE4-N-Y renamed recovery.rs
#      -> recovery/mod.rs, leaving DC-STORE-05 / T-REC-01 / T-REC-02 stale).
#
# Checks 1+2 absorb the only load-bearing checks of the retired
# ci_check_constitution_coverage.sh (a ziranity-v3 import that validated a
# foreign schema — CL-* clusters, tier-by-prefix, bidirectional cross_refs,
# $HOME planning-doc coverage — and never matched Ade's registry; see
# docs/planning/registry-cross-ref-bidirectional-repair.md).
#
# Scope notes (deliberately narrow, to stay green + false-positive-free):
#   - code_locus: only crates/**.rs and ci/**.sh tokens (existence unambiguous).
#   - Tokens containing a glob '*' are skipped (docs globs, dir-wildcards).
#   - docs/ paths are NOT hard-checked here (globs + archived on cluster close).
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

failed = False
def fail(msg):
    global failed
    print(f"FAIL: {msg}")
    failed = True

# 1. Unique rule IDs (append-only registry; IDs are never reused).
ids = set()
for r in rules:
    rid = r.get("id", "")
    if rid in ids:
        fail(f"duplicate rule id: {rid}")
    ids.add(rid)

# 2. Every cross_ref resolves to a real rule ID. Directed edge; reciprocity is
#    intentionally NOT required (see header).
for r in rules:
    for ref in r.get("cross_ref", []):
        if ref not in ids:
            fail(f"{r.get('id','?')}: cross_ref '{ref}' does not resolve to any rule id")

# 3. code_locus crates/**.rs + ci/**.sh paths exist on disk.
TOKEN = re.compile(r"(?:crates|ci)/[\w./\-]+\.(?:rs|sh)")
checked = 0
for r in rules:
    for tok in TOKEN.findall(r.get("code_locus", "") or ""):
        if "*" in tok:
            continue
        checked += 1
        if not os.path.exists(tok):
            fail(f"{r['id']}: code_locus path does not exist "
                 f"(moved/renamed? — update code_locus or restore the path): {tok}")

if failed:
    sys.exit(1)

print(f"OK: registry coherence — {len(rules)} rules, {len(ids)} unique ids, "
      f"all cross_refs resolve, all {checked} crates/**.rs + ci/**.sh "
      f"code_locus paths exist")
PYEOF
