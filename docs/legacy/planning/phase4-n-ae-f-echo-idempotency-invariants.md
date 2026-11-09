# PHASE4-N-AE.F (scoping) — post-adoption echo idempotency

> **Status: invariants sketch (the `/invariants` phase). Not yet a cluster/slice doc, not implemented.**
> Prerequisite for long-running C2-LOCAL or preprod-style relay operation. Surfaced by the
> CE-A5 manifest run (venue c2ae18, 2026-06-07).

## 1. The phenomenon (what we observed)

In continuous `--mode node` operation Ade plays two roles against the same peer:
- **server** — Ade `--listen 3012`; the relay pulls from Ade and adopted Ade's forged block 17.
- **follower** — Ade `--peer <relay>`; Ade pulls from the relay.

After the relay adopted Ade's block 17 (slot 421, hash `db3b5675`), the relay's tip *became*
that block. On Ade's next follow pull, the relay served block 17 **back** to Ade — a block
Ade had already forged and durably applied (Ade's own tip). Ade's receive path rejected it:

```
Receive(Validity(Header(SlotBeforeLastApplied { last: SlotNo(421), attempted: SlotNo(421) })))
→ fail-closed, exit 43
```

This happened **after** `AddedToCurrentChain` — it is a post-adoption follow-loop artifact,
not a failure of adoption. But it makes a continuous run terminate, so it blocks
long-running relay operation.

## 2. Root cause

`SlotBeforeLastApplied` is a **safety** fail-close: the receive path applies blocks strictly
**after** the last-applied slot, so a block at-or-before the last-applied slot halts
deterministically (it would be a non-extending block / an implicit rollback). This is correct
as the default. The defect is that it does **not distinguish two cases** at the same slot:

1. **Idempotent re-announce** — the peer re-sent a block Ade has **already durably applied**,
   byte-identical (same slot, **same hash**). Ade already has exactly this block.
2. **Genuine conflict** — a **different** block (different hash) at/before the last-applied
   slot (a real fork / rollback attempt).

Today both collapse to the same fail-close. Case (1) is benign and should be an idempotent
no-op; case (2) must stay fail-closed (or route to the existing rollback authority).

## 3. Invariants to add

- **AE-F-INV-1 (receive idempotency).** A peer-delivered block whose `(slot, hash)` is
  **already present in the durable ChainDb** (Ade applied exactly this block) is an
  **idempotent no-op**: skip it, do not re-apply, do not advance state, do not fail-close.
  The post-state (ledger, chain_dep, ChainDb tip) is **identical** to before the re-announce.
- **AE-F-INV-2 (fail-closed boundary preserved).** A block at-or-before the last-applied slot
  whose `hash` is **not** the durably-stored block at that slot remains fail-closed exactly as
  today (`SlotBeforeLastApplied` / the existing receive error), or routes to the existing
  rollback path — **unchanged**. The idempotent skip is gated on **hash equality against the
  durable store**, never on slot alone.
- **AE-F-INV-3 (no skip-past, no fork-choice).** The skip applies **only** to a block Ade
  already has byte-identically. It must never skip *past* a gap, never accept a *better* chain,
  never select among competing tips. Multi-producer fork-choice (DC-CONS-03) is untouched and
  remains a separate future cluster.

## 4. The discriminator (load-bearing)

The decision key is the **hash against the durable store**, not the slot:

```
on receive of block B (slot s, hash h):
  if ChainDb.get_block_by_hash(h) == Some(stored) && stored.slot == s:
        → AlreadyHave  (idempotent no-op; AE-F-INV-1)
  else if s <= last_applied_slot:
        → SlotBeforeLastApplied / rollback path  (AE-F-INV-2, unchanged)
  else:
        → normal extend-apply
```

The `AlreadyHave` check must consult the **durable** ChainDb (`get_block_by_hash`), not just
the in-memory tip, so it recognizes any already-applied block, not only the immediate parent.

## 5. TCB placement

- The **decision** (AlreadyHave vs SlotBeforeLastApplied vs extend) belongs in the **BLUE**
  receive reducer (`ade_ledger::receive`) as a new closed verdict variant
  (e.g. `ReceiveOutcome::AlreadyHave`) — a pure function of `(durable store membership, block,
  last_applied)`. Illegal-state-unrepresentable: the variant is explicit, not a boolean.
- The **orchestration** (consult ChainDb, route the variant, continue the loop on AlreadyHave
  instead of exiting) belongs in the **RED** shell (`ade_node::node_sync` /
  `run_node_sync`/`pump_block` caller) — it maps `AlreadyHave` to "continue, no state change",
  and only `SlotBeforeLastApplied` (different hash) to the fail-close.

## 6. Slice-entry proof obligations

1. **Does `get_block_by_hash` already index forged blocks?** AE.B confirmed forged + followed
   blocks are durable `get_block_by_hash`-indexed StoredBlocks (the live store dump showed
   block_no 0–20 with SLOT_BY_HASH populated by `put_block`). Re-confirm the forged self-accept
   path populates SLOT_BY_HASH for Ade's own block (it must, for AlreadyHave to fire).
2. **Where exactly is `SlotBeforeLastApplied` raised** — header validity vs the reducer admit?
   The fix must intercept **before** the fail-close, with the durable-membership check, without
   moving the safety boundary for the different-hash case.
3. **Is the re-announced block delivered as a single block or a batch?** The skip must handle a
   batch that contains already-have blocks followed by genuinely-new ones (skip the prefix,
   apply the suffix) — without skipping a gap.

## 7. Acceptance surface (mechanical)

- **Hermetic (BLUE/GREEN):** recover → follow → forge a successor → feed Ade **its own forged
  block back** → the reducer returns `AlreadyHave`; ledger/chain_dep/ChainDb tip unchanged; no
  error. A **negative** test: a **different** block (same slot, different hash) → still
  `SlotBeforeLastApplied` (AE-F-INV-2 holds).
- **Replay:** the AlreadyHave decision is deterministic over `(durable store, block,
  last_applied)`; same inputs → same outcome (extends T-REC-05 / DC-PROTO-09).
- **Live (the real fix target):** a sustained `--mode node` run where the relay re-announces
  Ade's adopted block — Ade no-ops and keeps following/forging past the echo (no exit 43). This
  is the long-running-relay precondition.

## 8. Non-goals / hard boundaries

- **No fork-choice.** AlreadyHave is exact-match idempotency, not chain selection.
- **No skip-past / no fallback.** Only a byte-identical already-stored block is skipped; gaps
  and different blocks keep their current semantics.
- **No weakening of `SlotBeforeLastApplied` for conflicting blocks** (AE-F-INV-2 is the fence).

## 9. Candidate registry rule + slice

- One new rule (proposed **DC-NODE-16** or **DC-PROTO-11**): "Receive idempotency — a
  peer-re-announced block already durably applied byte-identically is a no-op; a different
  block at/before the last-applied slot stays fail-closed." Strengthens DC-PROTO-09 (receive
  transcript determinism) and cross-refs DC-CONS-20 (admit authority), DC-CONS-03 (fork-choice,
  explicitly untouched).
- Slice **PHASE4-N-AE.F** (same cluster, follow-up). Estimated one BLUE reducer variant + one
  RED routing change + the hermetic positive/negative pair. Small.

## 10. Why this is the right next move (not long-running relay yet)

The CE-A5 manifest proves the **adoption** surface. Long-running C2-LOCAL / preprod-style
operation requires the follow loop to **survive its own served tip coming back** — which is
exactly AE-F-INV-1. Scope + close AE.F before any sustained run, so the run isn't a timing
gamble over a known fail-close.
