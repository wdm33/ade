# PHASE4-N-AI — CE-AI-6 convergence-pass operator runbook

> Operator-gated, derived-tier (cluster hard line 8). This proves the **exercised
> single-best-peer competing-producer convergence venue** — Ade follows one peer
> through a ChainSync rollback and replay-equivalently re-converges on the peer's
> branch. It does **NOT** prove full multi-peer Cardano ChainSel; multi-candidate
> live selection remains out of scope.

## What CE-AI-6 demonstrates

Ade (`--mode node`, **Participant** venue) + ≥1 Haskell `cardano-node` producer on a
competing-producer venue converge on the **same tip**, including **through a peer
reorg** (a `RollBackward`), with **no `Diverged`** verdict. The hermetic
arrival-order-independence of the selection authority is already proven by CE-AI-5
(`select_best_chain_arrival_order_independent_*`); this pass is the live counterpart.

## Prerequisites

- A competing-producer venue — see `docs/active/c2-preprod-tip-guide.md` (C2 = recover
  the real preprod tip via Mithril, then a second producer creates a short competing
  branch so the peer issues a `RollBackward`). **Never** a from-genesis Conway local net.
- Ade built at the cluster HEAD; operator KES/op-cert/VRF keys for the Participant.
- The local docker preprod peer (`reference-local-preprod-docker-cardano-node`) or a
  private testnet with a competing producer.

## Steps

1. Recover Ade's durable tip (Mithril → recover), per the C2 guide.
2. Start Ade **keyed-but-unstaked (σ=0)** so it is a pure follower (Ade never wins a slot ⇒
   never forges ⇒ no self-forged competing block ⇒ no `diverged`):
   `ade_node --mode node --participant-venue <keys/paths>
   --convergence-evidence-path docs/evidence/phase4-n-ai-convergence-pass.jsonl`.
   The convergence-evidence sink (PHASE4-N-AJ AJ-S1/S2, DC-NODE-30) writes the closed
   `block_received` / `block_admitted` / `agreement_verdict` transcript **directly** to that
   path. It is a **dedicated** file — **not** the sched/`forge_*` `--log` (that log is a
   separate, non-gate-valid artifact). Absent `--convergence-evidence-path`, no transcript is
   written and node behavior is unchanged.
3. Drive a peer reorg: have the competing Haskell producer extend a branch that wins,
   so the followed peer issues a `RollBackward` — Ade follows it (AI-S4b-ii:
   `run_participant_sync` → `apply_chain_event` → `WalEntry::RollBack`).
4. Let Ade re-converge on the peer's branch (sustained `agreement_verdict` agreed at the
   peer's tip; **0 diverged**).
5. Stop Ade. Write the manifest `docs/evidence/phase4-n-ai-convergence-pass.md` binding
   the `.jsonl` sha256:
   `echo "convergence-pass manifest; jsonl sha256: $(sha256sum docs/evidence/phase4-n-ai-convergence-pass.jsonl | cut -d' ' -f1)" >> docs/evidence/phase4-n-ai-convergence-pass.md`
   (plus run metadata: venue, peer, slots of the reorg).

## Validate + commit

- `bash ci/ci_check_convergence_evidence_schema.sh` — must pass (closed vocabulary,
  sha256-bound, 0 diverged, **≥1 slot regression** = the reorg was followed, no boring
  same-tip-only run).
- Commit the `.jsonl` + `.md`. The gate is **vacuous-until-committed**, so until then CI
  stays green without the transcript.

## Honesty (binding)

The committed transcript proves convergence for the **exercised** venue only. At
`/cluster-close`, scope or strengthen `CN-CONS-03` per its exact registry wording — do
**not** over-flip it to a full multi-peer ChainSel claim.
