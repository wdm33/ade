#!/usr/bin/env bash
set -euo pipefail

# Registry coherence guard over the IDSE registry docs/ade-invariant-registry.toml.
# Four load-bearing checks:
#   1. Rule IDs are unique (the registry is append-only; IDs are never reused).
#   2. Every `depends_on` resolves to a real rule ID. depends_on is a DIRECTED
#      relation; reciprocity is NOT required and is intentionally NOT checked.
#   3. Every path in an enforcement locus (`enforcement_compile_time`,
#      `enforcement_runtime`, form "<path>[::<item>] — <how>") exists on disk,
#      unless the locus is listed in `enforcement_superseded`.
#   4. Every path in `ci_scripts` exists on disk.
#
# Before the IDSE migration this guard read the legacy IDD registry's
# `cross_ref` and `code_locus` fields. That registry is frozen at
# docs/legacy/idd-invariant-registry.toml; its code loci were carried into the
# enforcement fields checked here (docs/planning/idse-migration-plan.md, M2).
#
# Repo-root-relative. python3 + tomllib (3.11+).

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

python3 - <<'PYEOF'
import tomllib, os, sys

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

ids = set()
for r in rules:
    rid = r.get("id", "")
    if rid in ids:
        fail(f"duplicate rule id: {rid}")
    ids.add(rid)

for r in rules:
    for ref in r.get("depends_on", []):
        if ref not in ids:
            fail(f"{r.get('id','?')}: depends_on '{ref}' does not resolve to any rule id")

loci = scripts = 0
for r in rules:
    superseded = {s.get("locus") for s in r.get("enforcement_superseded", []) if isinstance(s, dict)}
    for key in ("enforcement_compile_time", "enforcement_runtime"):
        for entry in r.get(key, []):
            if entry in superseded:
                continue
            path = entry.partition(" — ")[0].partition("::")[0].strip()
            loci += 1
            if not path or not os.path.exists(path):
                fail(f"{r['id']}: {key} path does not exist (moved/renamed? supersede the locus or restore the path): {path!r}")
    for script in r.get("ci_scripts", []):
        scripts += 1
        if not os.path.isfile(script):
            fail(f"{r['id']}: ci_scripts path does not exist: {script}")

if failed:
    sys.exit(1)

print(f"OK: registry coherence — {len(rules)} rules, {len(ids)} unique ids, "
      f"all depends_on resolve, all {loci} enforcement-locus paths and {scripts} ci_scripts paths exist")
PYEOF
