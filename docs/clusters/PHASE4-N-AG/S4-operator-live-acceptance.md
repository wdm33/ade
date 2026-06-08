# Invariant Slice — Operator-gated live acceptance, sustained past k (DC-NODE-19 S4 / CE-AF-6b)

## 2. Slice Header
**Slice Name:** Operator-gated live acceptance — sustained single-producer forging past a follow-link EOF (DC-NODE-19, PHASE4-N-AG S4 = CE-AF-6b)
**Cluster:** PHASE4-N-AG — single-producer loop continuation after follow-link EOF; **rung-1 only, single-producer only**
**Status:** Proposed — **operator-gated** (the live run is executed by the operator, not in CI)
**Authority source:** `docs/clusters/PHASE4-N-AG/cluster.md` (§4, §5 CE-AG-5); registry `DC-NODE-19` (declared → **enforced at `/cluster-close`** on this transcript + the S1–S3 hermetic core)

**Cluster Exit Criteria Addressed:**
- [ ] **CE-AG-5 (operator-gated; hard close gate = CE-AF-6b):** sustained **> k** Ade blocks settle into the relay's ImmutableDB across **≥ 1** follow-link EOF, warm-start replay byte-identical; **committed live transcript** (rung1-auto, C2-LOCAL).

Exit criteria not listed (CE-AG-1=S1; CE-AG-2/3=S2; CE-AG-4=S3; CE-AG-6=close) are out of scope.

**Slice Dependencies:** S1 (`b9ef6e69`) + S2 (`46098c8c`) + S3 (`a65e2039`) — the merged hermetic core (planner, loop continuation, replay-equivalence). **No code is added by S4**; the `--mode node` binary already continues past feed-EOF via S2.

## 3. Implementation Instruction (Operator)
**This slice is a live RUN, not a code change.** The hermetic core is merged + green (S1–S3). S4 = execute the C2-LOCAL single-producer harness with the *current* binary, observe the sustained-past-k + EOF-crossing + warm-start behavior, and **commit the transcript** `docs/evidence/phase4-n-ag-loop-continuation.{md,jsonl}`. Use the **verbatim `--mode node` path** (no special build, no flag the bounty path lacks). Do **not** write a synthetic transcript. Do **not** flip `DC-NODE-19` until the transcript genuinely shows the §12 acceptance (the flip happens at `/cluster-close`). If the live run surfaces a defect (as c2t7 did — 2 loop bugs), **fix it as a scoped follow-up and re-run** — do not soften the claim.

## 4. Intent
Prove **live** what S1–S3 proved on the workbench: a certified single-producer Ade, behind a real non-producing `cardano-node` relay, **keeps forging its own durable spine across an actual follow-link EOF** — sustaining production **past k** with the blocks settling into the relay's ImmutableDB, and recovering byte-identically on restart. This is the live evidence that, with the hermetic core, makes **DC-NODE-19** enforceable. It is the c2t7 stop-cause turned into a sustained run.

> **Scope guard — this is NOT an indefinite-operation claim.** S4 proves the bounded DC-NODE-19 failure class: a **structural follow-link EOF does not terminate certified single-producer forging before settlement beyond k**. Indefinite/long-running operation is *not* one invariant — it is composed later from additional bounded rungs (longer windows / multiple EOFs+restarts, epoch transition, KES/opcert boundary, forecast horizon, multi-producer fork-choice, preprod) plus long-duration evidence. See the forward-ladder note after §15.

## 5. Scope
- **Artifacts:** the operator harness `~/.cardano-rung1-host/rung1-auto.sh` (C2-LOCAL cardano-testnet, magic 42, k=5, frozen, node2 = non-producing relay); the committed evidence `docs/evidence/phase4-n-ag-loop-continuation.{md,jsonl}`.
- **Binary:** `ade_node --mode node --single-producer-venue --adoption-cert-path <file>` (the S2/DC-NODE-18 flags; the harness writes the adoption certificate on observing relay adoption).
- **State machines / persistence / network:** none changed — the live run exercises the merged S1–S3 code as-is.
- **Out of scope:** any code change; multi-producer (rung 2); preprod (rung 3); follow-link keep-alive (OQ-KA).

## 6. Execution Boundary (TCB color)
- **BLUE / GREEN:** none changed.
- **RED:** the operator harness + the committed evidence (file I/O outside the node; non-authoritative). The node binary under test is the merged S1–S3 (RED loop + GREEN planner; BLUE untouched).

## 7. Invariants Preserved (registry IDs)
All — the run exercises the merged code without change: `DC-NODE-19` (the continuation under test), `DC-NODE-05` / `DC-NODE-12` (pump_block sole durable tip), `DC-NODE-18` (the extend authority + cert), `DC-NODE-15` (initial catch-up), `DC-CONS-03` (fork-choice authority, untouched), `T-REC-03` / `T-REC-05` (replay/recovery — exercised live by the warm-start leg).

## 8. Invariants Strengthened or Introduced
`DC-NODE-19` — this transcript is the **live half** of its evidence. Combined with the S1–S3 hermetic gates, it makes `DC-NODE-19` enforceable at `/cluster-close` (the flip, the `strengthened_in += PHASE4-N-AG` appends on DC-NODE-05 / CN-NODE-02 / T-REC-03 / T-REC-05 / DC-NODE-18, and the four-grounding-doc refresh all happen at close — **not** in S4). Exactly one invariant family (DC-NODE-19 loop continuation).

## 9. The runbook (operator)
1. **Venue:** start the frozen C2-LOCAL `cardano-testnet` (3 pools, magic 42, k=5, short epoch), with node2 relaunched as a **non-producing** Haskell relay (the c2t7 venue). Guard **k ≥ 2** (the recover-anchor k=0 snapshot-conflict edge is out of scope).
2. **Run:** `~/.cardano-rung1-host/rung1-auto.sh` driving `ade_node --mode node --single-producer-venue --adoption-cert-path <file>`. The harness writes the RED adoption certificate (`<block_no> <slot> <hash_hex64>`) on observing the relay's first adoption (relay `query tip`), exactly as in c2t7.
3. **Observe the continuation:** Ade catches up (DC-NODE-15) → forges N → relay adopts → cert written → promote (DC-NODE-18) → forges N+1 → **the follow link EOFs** (relay ~5 s idle timeout; no keep-alive) → **Ade does NOT stop** (S2 / DC-NODE-19): the loop wakes on the slot-cadence timer and **keeps forging** N+2, N+3, … on its own durable spine → the relay keeps adopting (over the serve link) → blocks pass **k** and become **immutable** in the relay's ImmutableDB.
4. **Warm-start leg:** kill `ade_node` mid-run; restart from the same on-disk state; confirm it recovers the same durable chain and **resumes** forging the spine (no ChainBreak) — the live analogue of S3's kill/warm-start.
5. **Capture the transcript** (next section) and commit it.

## 10. Changes Introduced
- **Code:** none.
- **Evidence:** `docs/evidence/phase4-n-ag-loop-continuation.md` (narrative, modeled on `phase4-n-af-extend-own-spine.md`) + `.jsonl` (the closed `NodeSchedEvent` / adoption event log). The transcript must record: the catch-up + first adoption + cert; **≥ 1 explicit follow-link EOF** event; the **continued forges past that EOF**; the relay's `AddedToCurrentChain` for each; the **ImmutableDB settlement of > k blocks**; the warm-start recovery to the same tip; and the counts (`forged > k`, relay-adopted, `eof_crossed ≥ 1`, `sparse = 0`).
- **Optional strengthening (informational, not a CE):** if the run naturally continues longer, also record total duration, total forged blocks, number of EOFs crossed, and whether an epoch boundary was approached or crossed. These are informational unless separately claimed — they do not change the S4 bar and must not be read as an indefinite-operation or epoch-transition claim.

## 11. Replay, Crash, and Epoch Validation
- **Replay/Crash (live):** the warm-start leg (§9.4) — kill mid-run, restart, recover the same durable chain + resume. The hermetic byte-identity is already S3 (`continue_past_eof_kill_warm_start_recovers_byte_identical`); S4 confirms it on the live store.
- **Epoch:** within-epoch sustained production (off-epoch fails closed, DC-EPOCH-03 — out of scope as a *claim*; if the short-epoch venue crosses an epoch, record it honestly per the optional line but do not claim epoch-transition support).

## 12. Mechanical Acceptance Criteria (operator-gated — NOT CI)
- [ ] **CE-AF-6b — committed transcript** `docs/evidence/phase4-n-ag-loop-continuation.{md,jsonl}` showing: **> k** (i.e. ≥ 6, k=5) Ade-forged blocks settled as **immutable** in the relay's ImmutableDB; **≥ 1** follow-link EOF crossed with Ade **continuing to forge** past it (the S2 behavior, not a halt); the relay `AddedToCurrentChain` for the post-EOF blocks; and a **warm-start** recovery to the same durable tip mid-run.
- [ ] The run used the verbatim `ade_node --mode node` path (path fidelity); the transcript is a real operator capture (relay `query tip` / node logs), not synthetic.
- [ ] The hermetic core is green (already merged): `cargo test -p ade_node` + `ci_check_single_producer_loop_continuation.sh` + `ci_check_node_run_loop_containment.sh` + `ci_check_loop_planner_closed.sh` + `ci_check_single_producer_extend_own_spine.sh`.
- [ ] **DC-NODE-19 is flipped declared → enforced ONLY at `/cluster-close`, gated on this committed transcript.** S4 itself does not edit the registry.

> **No-overclaim guard:** this transcript backs the bounded DC-NODE-19 failure class only (structural follow-link EOF does not terminate certified single-producer forging before settlement beyond k). It is **not** evidence of indefinite operation, epoch-transition correctness, multi-producer fork-choice, or preprod readiness — each of those is a separate, separately-claimed rung.

## 13. Failure Modes
- The live run reveals a non-determinism or a loop defect (the live gate's value — c2t7 surfaced 2) → **fix as a scoped follow-up + re-run**; never a false CE claim.
- The follow link does **not** EOF in the window (e.g., the relay keeps the link alive) → the EOF-crossing sub-claim is unproven; record honestly and re-run a longer window. (Sustained-past-k without an EOF still proves continuation but not the EOF-survival specifically — note which sub-claims the transcript actually backs.)
- The relay stops adopting (e.g., forecast-horizon / venue limit, Finding A) → not a DC-NODE-19 failure; record the venue constraint.

## 14. Hard Prohibitions
**Inherited (cluster §8):** no code change that weakens the S1–S3 boundaries; DC-CONS-03 untouched; no flip of `DC-NODE-19` without the committed transcript.
**Slice-specific:**
- **No synthetic / hand-edited transcript** — the evidence is a real operator capture.
- **No false or softened CE claim** — the transcript states exactly which sub-claims it backs (sustained-past-k, EOF-crossing, warm-start); if any is unproven, say so.
- **No indefinite-operation / epoch / multi-producer / preprod claim from this run** — S4 backs the bounded EOF-survival-past-k class only.
- **No non-`--mode node` path** (path fidelity — the bounty path, not a special build/flag).
- **No registry edit in S4** (the flip + strengthenings are the `/cluster-close` step).
- No keep-alive / epoch-transition / multi-producer / preprod work.

## 15. Explicit Non-Goals
The hermetic core (S1–S3) · flipping `DC-NODE-19` / the registry strengthenings (close) · follow-link keep-alive (OQ-KA) · multi-producer fork-choice (rung 2) · preprod (rung 3) · the recover-anchor k=0 edge.

### Path to indefinite operation (forward context — NOT proven by S4)
Indefinite/long-running production is composed from bounded rungs, each separately enforced + evidenced, not claimed in one run:
1. **DC-NODE-19 (this cluster):** survive a follow-link EOF and continue past k.
2. **Longer-window liveness:** continue across longer windows, multiple EOFs/restarts, no WAL drift.
3. **Epoch transition:** keep producing across an epoch boundary; recompute leadership/stake/nonce correctly.
4. **KES/opcert boundary:** handle KES-period constraints; fail closed / rotate operationally.
5. **Forecast horizon:** never blindly forge past protocol forecast bounds.
6. **Multi-producer rung:** observe competing chains, run fork-choice, rollback/recover correctly.
7. **Preprod:** prove the same behavior under real public-chain conditions.

## 16. Completion Checklist
- [ ] The live run executed on the C2-LOCAL rung1-auto harness with the verbatim `--mode node` binary.
- [ ] The transcript `docs/evidence/phase4-n-ag-loop-continuation.{md,jsonl}` is committed and shows sustained > k across ≥ 1 follow-link EOF + a warm-start recovery.
- [ ] The hermetic gates + `cargo test -p ade_node` are green.
- [ ] `/cluster-close` flips `DC-NODE-19` → enforced on this transcript + the hermetic core (separate step).
```
