#!/usr/bin/env bash
# ci_check_post_switch_convergence_window.sh -- PHASE4-N-AO CE-AO-6 (DC-EVIDENCE-04).
#
# RELEASE/EVIDENCE-tier transcript checker (NOT a BLUE consensus rule). Validates a
# committed two-producer SELECT transcript against the BOUNDED, BRANCH-BOUND
# post-switch convergence window: the hard fork-switch proof stays exactly as-is,
# then a bounded window must show Ade STAYED on the adopted branch and converged --
# without relying on a lucky exact-tip moment and without freezing the venue.
#
# Hard proof (must precede the window): fork_choice_selected{win} -> branch_fetch_*
# -> branch_prevalidated -> fork_switch_applied{ForkChoiceWin} at X -> block_admitted X.
# Window after X (bounds FIXED here, not tuned post-hoc):
#   max_slots = 200, max_admitted_blocks = 20
#   - no diverged verdict
#   - every fork-choice win has a terminal (applied | failed | superseded)
#   - every admitted block is X or a descendant (slot >= X, strictly forward -- no
#     rollback below X)
#   - agreement_verdict{agreed, our_hash==peer_hash} at X or a descendant Y in-window
set -euo pipefail
C="${1:?usage: $0 <transcript-conv.jsonl>}"
[ -f "$C" ] || { echo "FAIL: transcript $C not found" >&2; exit 1; }

python3 - "$C" 200 20 <<'PY'
import sys, json
path, MAX_SLOTS, MAX_BLOCKS = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
ev = [json.loads(l) for l in open(path) if l.strip()]

def fail(m):
    print("FAIL (post_switch_convergence_window): " + m, file=sys.stderr); sys.exit(1)

# --- the HARD fork-switch proof (unchanged) ---
applied = [e for e in ev if e["event"] == "fork_switch_applied"]
if not applied:
    fail("no fork_switch_applied -- no winning branch was durably adopted")
X = applied[0]
if X.get("rollback_reason") != "fork_choice_win":
    fail("fork_switch_applied rollback_reason is not fork_choice_win")
xslot, xhash, xfsid = X["new_tip_slot"], X["new_tip_hash_hex"], X["fork_switch_id"]
chain = [e["event"] for e in ev if e.get("fork_switch_id") == xfsid]
for need in ("fork_choice_selected", "branch_fetch_started", "branch_fetch_completed",
             "branch_prevalidated", "fork_switch_applied"):
    if need not in chain:
        fail(f"the adopted fork_switch_id {xfsid} is missing its prior {need}")
admits = [e for e in ev if e["event"] == "block_admitted"]
if not any(a["block_hash_hex"] == xhash for a in admits):
    fail("the adopted switch tip X was never block_admitted")
# both peers delivered input.
peers = {e["peer"] for e in ev if e["event"] == "block_received" and "peer" in e}
if len(peers) < 2:
    fail(f"fewer than two peers delivered block_received ({sorted(peers)})")

# --- the BOUNDED post-switch convergence window ---
i_x = ev.index(X)
window = ev[i_x:]
# bound it: stop after MAX_SLOTS past X or MAX_BLOCKS admitted blocks.
win, n_admit = [], 0
for e in window:
    s = e.get("slot", e.get("new_tip_slot", xslot))
    if s is not None and s > xslot + MAX_SLOTS:
        break
    if e["event"] == "block_admitted":
        n_admit += 1
        if n_admit > MAX_BLOCKS:
            break
    win.append(e)

# (a) no diverged in the window.
if any(e["event"] == "agreement_verdict" and e.get("kind") == "diverged" for e in win):
    fail("a diverged verdict occurred inside the post-switch window")
# (b) every fork-choice win (anywhere) has a terminal -- no dangling win.
wins = {e["fork_switch_id"] for e in ev if e["event"] == "fork_choice_selected" and e.get("result") == "win"}
terms = {e["fork_switch_id"] for e in ev if e["event"] in
         ("fork_switch_applied", "fork_switch_failed", "fork_switch_superseded")}
dangling = sorted(wins - terms)
if dangling:
    fail(f"win(s) without a terminal (applied|failed|superseded): {dangling}")
# (c) every admitted block in the window is X or a descendant: slot >= X, strictly
#     forward (no rollback below X within the window).
prev = xslot - 1
for a in [e for e in win if e["event"] == "block_admitted"]:
    if a["slot"] < xslot:
        fail(f"admitted block at slot {a['slot']} < switch tip {xslot} (rolled back below X)")
    if a["slot"] <= prev:
        fail(f"non-forward admitted slot {a['slot']} after {prev} (rollback in window)")
    prev = a["slot"]
# (d) agreed at X or a descendant Y in-window, our_hash == peer_hash.
agreed = [e for e in win if e["event"] == "agreement_verdict" and e.get("kind") == "agreed"
          and e.get("our_hash_hex") == e.get("peer_hash_hex")
          and xslot <= e.get("slot", -1) <= xslot + MAX_SLOTS]
if not agreed:
    fail(f"no agreement_verdict{{agreed, our==peer}} at X or a descendant within {MAX_SLOTS} slots of {xslot}")
y = agreed[0]

print("OK: post-switch convergence window PASSES (DC-EVIDENCE-04 / CE-AO-6)")
print(f"  hard proof: fork_switch_applied X @ slot {xslot} ({xhash[:16]}..) fsid {xfsid}; switch tip block_admitted")
print(f"  both peers delivered: {sorted(peers)}")
print(f"  window (<= {MAX_SLOTS} slots / {MAX_BLOCKS} blocks): 0 diverged, all wins terminal, "
      f"{len([e for e in win if e['event']=='block_admitted'])} admitted all forward >= X")
print(f"  converged: agreed @ slot {y['slot']} ({'X' if y['slot']==xslot else 'descendant'}), "
      f"our_hash==peer_hash=={y['our_hash_hex'][:16]}..")
PY
