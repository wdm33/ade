#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-M-A S5 — honest-scope CI gate (RO-GENESIS-REPLAY-01).
#
# Per memory [[feedback-shell-must-not-overstate-semantic-truth]]
# the cluster does NOT claim "Ade has independently replayed
# genesis → P". The claim is honestly carried as an open
# obligation; this gate prevents a future commit from flipping
# RO-GENESIS-REPLAY-01 to `enforced` without doing the actual
# multi-month work.
#
# Under IDSE the legacy entry RO-GENESIS-REPLAY-01 is excluded (migration
# batch B07, S7): Ade executes only Conway semantics and never replays
# historical eras (assumption A2). The honesty this gate protects is now
# carried by that exclusion and by A2's text.
#
# Mechanical guards:
#   1. The legacy map records RO-GENESIS-REPLAY-01 as `excluded`, with a
#      reason citing A2, and maps it to no IDSE invariant.
#   2. Assumption A2 in docs/ade-invariant-registry.toml is active and still
#      states that Ade never replays or executes historical eras.
#   3. No active IDSE invariant lists RO-GENESIS-REPLAY-01 in legacy_ids.
#
# If genesis replay is ever built, A2 must first be redefined by the human
# through a Define batch; this gate then fails until it is updated with it.
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

python3 -I - <<'PYEOF'
import sys, tomllib
reg = tomllib.load(open("docs/ade-invariant-registry.toml", "rb"))
rows = {r["id"]: r for r in tomllib.load(open("docs/legacy/idd-to-idse-map.toml", "rb"))["legacy"]}
fails = []
row = rows.get("RO-GENESIS-REPLAY-01")
if row is None:
    fails.append("RO-GENESIS-REPLAY-01 missing from the legacy map")
else:
    if row["disposition"] != "excluded" or row["new_ids"]:
        fails.append(f"RO-GENESIS-REPLAY-01 must stay excluded with no IDSE target (found {row['disposition']!r}, {row['new_ids']})")
    if "A2" not in row["reason"]:
        fails.append("RO-GENESIS-REPLAY-01's exclusion reason no longer cites A2")
a2 = next((a for a in reg["assumptions"] if a["id"] == "A2"), None)
if a2 is None or a2.get("status") != "active":
    fails.append("assumption A2 is missing or not active")
elif "never replays or executes historical eras" not in a2["text"]:
    fails.append("A2 no longer states that Ade never replays or executes historical eras")
for r in reg["rules"]:
    if "RO-GENESIS-REPLAY-01" in r.get("legacy_ids", []):
        fails.append(f"{r['id']} carries RO-GENESIS-REPLAY-01")
for f in fails:
    print(f"FAIL: {f}")
if fails:
    sys.exit(1)
print("OK: genesis replay stays unclaimed (RO-GENESIS-REPLAY-01 excluded under A2; A2 active)")
PYEOF
