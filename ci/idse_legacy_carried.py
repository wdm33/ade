#!/usr/bin/env python3
"""Legacy-ID carry-over check for the IDSE registry.

Usage: python3 -I ci/idse_legacy_carried.py [--ci-script ci/<script>.sh] LEGACY_ID [LEGACY_ID ...]

Ade's CI gates were written against the legacy IDD registry, now frozen at
docs/legacy/idd-invariant-registry.toml. Each legacy ID is followed forward through
docs/legacy/idd-to-idse-map.toml. The check passes for a legacy ID only when:
  - the map gives it a disposition of migrated or merged;
  - every IDSE entry it maps to exists in docs/ade-invariant-registry.toml with status "active";
  - for a migrated ID, at least one of those entries lists it in legacy_ids;
  - with --ci-script, at least one of those entries lists that script in ci_scripts, so the
    registry still records the gate as enforcing the carried invariant.
Exit status 0 when every named ID passes, 1 otherwise.
"""
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REG = ROOT / "docs/ade-invariant-registry.toml"
MAP = ROOT / "docs/legacy/idd-to-idse-map.toml"


def main(argv):
    script = None
    if len(argv) >= 2 and argv[0] == "--ci-script":
        script, argv = argv[1], argv[2:]
    if not argv:
        print("usage: idse_legacy_carried.py [--ci-script PATH] LEGACY_ID ...", file=sys.stderr)
        return 2
    rules = {r["id"]: r for r in tomllib.loads(REG.read_text())["rules"]}
    rows = {r["id"]: r for r in tomllib.loads(MAP.read_text())["legacy"]}
    ok = True
    for lid in argv:
        row = rows.get(lid)
        if row is None:
            print(f"FAIL: {lid} is not in the legacy map")
            ok = False
            continue
        if row["disposition"] not in ("migrated", "merged") or not row["new_ids"]:
            print(f"FAIL: {lid} has disposition {row['disposition']!r}; nothing carries it")
            ok = False
            continue
        targets = [rules.get(n) for n in row["new_ids"]]
        bad = [n for n, t in zip(row["new_ids"], targets) if t is None or t.get("status") != "active"]
        if bad:
            print(f"FAIL: {lid} maps to {bad}, missing or not active in the IDSE registry")
            ok = False
            continue
        if row["disposition"] == "migrated" and not any(lid in t.get("legacy_ids", []) for t in targets):
            print(f"FAIL: {lid} is migrated, but none of {row['new_ids']} lists it in legacy_ids")
            ok = False
            continue
        if script and not any(script in t.get("ci_scripts", []) for t in targets):
            print(f"FAIL: none of {row['new_ids']} (carrying {lid}) lists {script} in ci_scripts")
            ok = False
            continue
        print(f"ok: {lid} -> {', '.join(row['new_ids'])} ({row['disposition']}, active)")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
