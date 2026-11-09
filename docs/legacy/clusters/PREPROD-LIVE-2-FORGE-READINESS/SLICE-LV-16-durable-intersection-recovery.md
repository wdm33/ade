# SLICE LV-16 — a recovered node finds its common ancestor with the peer

**Entry state:** `0ddb36d1` (LV-15b-0, store semantics 13). LV-15b-0 stays open: its CE-10 (restart
identity), CE-11 (cold replay) and CE-16 (sustained operation) wait on this slice. This slice changes no
ledger rule, no governance rule and no persisted interpretation. It is a storage-recovery and
network-intersection authority slice.

---

## 0. The incident — settled facts, do not re-derive

| fact | evidence |
|---|---|
| The v13 primary (PID 1326135) was killed by the host, not by Ade | syslog: `earlyoom ... sending SIGTERM to process 1326135 ... VmRSS 8203 MiB` at 18:22:27Z, SIGKILL 1.2 s later; host swap free 0 |
| Durable tip at the kill: slot 138,046,917, block 5,369,626, hash `f8c86fec…` | restart trace `durable_tip=138046917/5369626/f8c86fec` |
| The peer had just replaced that block; its chain contains `d9fe3452…` at 138,046,915 | pre-kill trace `rollback_admit action=reset_to_settled ... rollback_target=138046915/5369625/d9fe3452`; the v12 diagnostic run saw the same fork and logged `rollback-followed: to_slot=138046915 to_hash=d9fe3452` |
| The kill landed after the accumulator pre-clear and before the ChainDb rollback committed | the S5 (2b) order: `accumulator_admit_and_clear_for_rollback` clears the anchor, then `apply_chain_event` trims the chain; no `rollback-followed` line before the kill |
| The stored state is consistent, with a stale tip | chain tip = `f8c86fec`; WAL holds its admit; reduced checkpoint at 138,046,903 (below both blocks); accumulator anchor absent, so recovery refolds from canonical (`reset_and_refold reason=anchor_absent`) |
| The restart cannot resume | `admission_wire_pump: peer=127.0.0.1:3001 exit=Error(UnsupportedRollbackPoint)`, then `not reconnecting` |
| Master copy | `~/.cardano-live1/lv15b0-v13-killed-store`, read-only, `SHA256SUMS` (chain.db, epoch-accumulator.redb, reduced-checkpoint.redb) and `SHA256SUMS.wal` (37 WAL segments, node.log). Every experiment runs on a separately checksummed copy. |

The S5 pre-clear did its job: a crash inside the rollback window left an uncertified accumulator, not a
certified one over an abandoned prefix. The defect is only in how a restarted node asks the peer for a
common point.

**Why it fails.** `wire_pump_start_point` (`node_lifecycle.rs:1352`) offers exactly one point, the
recovered tip, and `run_admission_wire_pump` sends `FindIntersect[that point]`. The peer no longer has it
and answers `IntersectNotFound`. The pump then sends `RequestNext` anyway (`wire_pump.rs:685`); a
follower that never moved sits at Origin, so the peer answers `RollBackward(Origin)`, which Ade correctly
refuses (AI-S4a). The in-process reconnect (`spawn_live_wire_pump_source`) resumes the same way, from the
last delivered block only, and fails the same way if that block was orphaned.

---

## 1. The contract

> **Given a valid durable store whose tip the peer no longer has, which points may Ade offer, how does it
> read the reply, and what must hold for the state that results?**

---

## 2. Protocol semantics — reference-derived

From `ouroboros-consensus` (local reference copy):

- **Server.** `handleFindIntersect` calls `followerForward points` (`MiniProtocol/ChainSync/Server.hs`).
  `followerForward` "must be given a list of points in order of preference; the iterator will move forward
  to the first point on the list that is on the current chain". When it succeeds, "the first call to
  `followerInstruction` after `followerForward` will be a `RollBack` to the point returned"
  (`Storage/ChainDB/API.hs:824-834`; `Impl/Follower.hs` `forward` is `findM checkIfPointOnChain`). When it
  fails, the follower does not move. A new follower is `FollowerInit`, equivalent to a rollback to genesis,
  which is the `RollBackward(Origin)` Ade received.
- **Client.** `findIntersection` offers `AF.selectPoints offsets ourFrag` with
  `mkOffsets k maxOffset = [0] ++ takeWhile (< l) [fib n | n <- [2..]] ++ [l]`, `l = min k maxOffset`
  (`MiniProtocol/ChainSync/Client.hs:1041-1071, 2039-2043`). The comment states why: an intersection is
  only useful within the last k blocks, because the client "could never switch to this candidate chain
  anyway" beyond that. On `IntersectNotFound` the client terminates that peer
  (`Client.hs:1076`, `terminate $ mkResult ...`). There is one `FindIntersect` per initial intersection.

So the peer's reply is a captured external input: it names the newest offered point on its chain, or
says none is. It never names a point Ade did not offer, and it is never an instruction to rewrite Ade's
chain.

---

## 3. Invariants

**T-REC-07 (true) — recovery reaches a common ancestor without changing replay.** After any process
termination, a node recovering from a valid durable state can find a common chain ancestor with a
compatible peer whenever one exists within its supported intersection history, without silently
discarding authoritative state and without changing the deterministic result of replay. When none
exists, it stops following that peer with a structured outcome and leaves its durable state untouched.

**DC-NODE-48 (derived) — the intersection candidate and reply rule.**

- **(a) Candidates come only from the durable chain.** Starting at the tip, the ancestry is read by
  parent hash through stored blocks. Each block is decoded, and its recomputed header hash must equal the
  hash it was looked up by (the store's hash index is not trusted on its own). Heights decrease by
  exactly one and slots strictly decrease. Any break is a typed `IntersectionAncestryError`, and that
  peer is not dialled.
- **(b) Cardano's offsets, newest first.** The offered depths are `mkOffsets(k, retained)` exactly:
  `0`, the Fibonacci numbers `1, 2, 3, 5, 8, …` below `l`, then `l = min(k, retained)`, where `retained`
  is the depth of the oldest stored ancestor. The deepest candidate, at depth `l`, is always offered. For
  k = 2160 with full history this is 18 points.
- **(c) The bootstrap anchor is offered only as the tip itself.** A peer `RollBackward` to the recovered
  anchor is an idempotent no-op (DC-NODE-33), which is only correct when no stored block sits above it.
  So the anchor is offered when it is the tip (a fresh bootstrap), and never below stored blocks.
- **(d) One `FindIntersect` per connection.** No refinement round. §3.1 shows that one round decides
  whether a usable common point exists.
- **(e) A found point must be one Ade offered.** `IntersectFound` naming any other point is
  `IntersectionPointNotOffered`: the session fails closed and does not reconnect. It emits nothing
  beyond the `TipUpdate` that DC-PUMP-02 requires for every reply carrying a tip.
- **(f) No common point is a structured outcome, not a rewind.** `IntersectNotFound` ends the session
  with `NoCommonIntersection`, carrying the bound that limited the offer (`SecurityParam`,
  `RetainedHistory` or `NoStoredAncestry`) and the deepest point offered. No `RequestNext` is sent, there is
  no reconnect (the same offer would get the same answer), and nothing durable changes. It is never
  turned into a rollback to genesis.
- **(g) The intersection is not rollback authority.** The `RollBackward(point)` that follows a found
  intersection is resolved exactly as today: DC-NODE-29 binds it to the stored block, the S5 pre-clear
  admits it against k and lineage, `commit_rollback` trims the chain and writes `WalEntry::RollBack`, and
  RF-1 re-certifies. None of that changes.
- **(h) Reconnect uses the same rule.** An in-process reconnect computes the candidates from the durable
  chain at dial time, and puts the last block delivered to the consumer first when it is newer than the
  durable tip. That head point is a resume hint, not lineage evidence: if the peer picks it, the
  `RollBackward` to it reaches the consumer after the buffered events that make it durable (INV-WL-7) and
  is still resolved by DC-NODE-29.

### 3.1 Why one round with these points is enough

Let `D` be the deepest candidate, at depth `l`.

- **Coverage.** Suppose the peer's chain contains some block `A` of Ade's chain with `depth(A) ≤ l`. A
  chain containing a block contains all of its ancestors. `D` is `A` or an ancestor of `A`, so the peer
  has `D`, and `D` was offered. So the peer answers `IntersectFound`. Equivalently, `IntersectNotFound`
  means the peer shares no block within depth `l` of Ade's tip.
- **What that means.** With `l = k`, the fork is deeper than k. Cardano's own client cannot switch to
  that chain either, and Ade's rollback admission refuses it (`ExceededRollback`). With `l < k`, the fork
  is at or below the parent of the oldest stored block, which is the Mithril-certified bootstrap anchor.
  Following that peer needs a fresh bootstrap. That resynchronisation path is not part of this slice (§8).
- **Precision.** The peer returns the newest offered point on its chain, `P`. The true fork point `F`
  satisfies `depth(F) ≤ depth(P)`, and the gap between `P` and the next shallower offered depth bounds the
  extra rollback and re-fetch. `depth(P) ≤ l ≤ k`, so the rollback to `P` is always admissible. For the
  killed store, depth 1 is offered, so the intersection is exactly `d9fe3452` and nothing extra is
  re-fetched.

### 3.2 Answers to the design questions

| question | answer |
|---|---|
| ordering | newest to oldest; the server takes the first one on its chain |
| how ancestors are selected and validated | (a) and (b): parent-hash walk over stored blocks, recomputed hashes, contiguous heights, decreasing slots, Cardano's offsets |
| which points survive restart | only stored blocks (the durable ChainDb) and, when it is the tip, the persisted bootstrap anchor; nothing in memory |
| the peer shares no offered point | (f): `NoCommonIntersection`, typed, terminal for that peer, no durable change |
| an additional intersection round | not permitted, and not needed (§3.1) |
| rollback depth beyond retained history | `l < k` and no common point: `NoCommonIntersection { bound: RetainedHistory }`; re-bootstrap is the operator path (§8) |

---

## 4. Placement

| boundary | responsibility | where |
|---|---|---|
| BLUE | choose candidates from a validated ancestry; validate the ancestry; classify the reply | `ade_ledger::rollback::intersection` (new, beside `admit_rollback`) |
| BLUE | rollback admission and commit after the intersection | unchanged: `admit_rollback`, `commit_rollback`, `materialize_rolled_back_state` |
| GREEN | turn the candidates into the `FindIntersect` point list, newest first | `ade_runtime::admission::wire_pump` (request construction) |
| RED | read the ancestry from the ChainDb; dial, send, receive; persist WAL and checkpoints | `ade_node::node_lifecycle` (ancestry read, supervisor), `wire_pump` (session), stores unchanged |

The reply classification is BLUE and the pump calls it. The pump does not decide on its own what a
reply means.

Two other callers send a single point, and keep doing so: the admission-mode judge path (the operator's
source tip) and the fork-switch body prefetch (the fork anchor). Each is offered as a single point with no
stored ancestry (`NoStoredAncestry`), so its replies are read by the same classifier. Before this slice
their `IntersectNotFound` also ended in the refused `RollBackward(Origin)`; now it ends one message
earlier, with `NoCommonIntersection`.

---

## 5. What a pending rollback leaves behind, and why recovery stays consistent

After the kill the durable state is: chain tip `917` (orphaned), the WAL through `917`, the reduced
checkpoint at `903`, and an accumulator with no anchor. Each part is recovered by an existing authority:

1. The ledger is rebuilt from the checkpoint plus WAL forward replay to `917` (T-REC-05).
2. The accumulator, having no anchor, refolds from canonical through `917` (S5).
3. **This slice:** the offer includes `915` at depth 1; the peer answers `IntersectFound(915)`, then
   `RollBackward(915)`.
4. DC-NODE-29 resolves `915` to the stored block; the pre-clear admits a depth-1 rollback;
   `commit_rollback` trims `917` and records `WalEntry::RollBack`; RF-1 re-certifies the settled point.
5. Forward blocks from the peer extend from `915`.

The obligation is that the state after step 5 equals the state of an execution that never adopted `917`,
compared at the same canonical chain points. The tip, checkpoint, accumulator and WAL must stay in their
normal relationship throughout: the chain tip is the WAL's last effective point, the checkpoint is at or
below the tip on the same lineage, and the accumulator is either uncertified or certified at a canonical
point.

---

## 6. Mechanical acceptance criteria

- **CE-LV16-1 Offsets.** `intersection_offsets(k, retained)` equals Cardano's `mkOffsets` for k = 2160
  with full history (`0,1,2,3,5,8,13,21,34,55,89,144,233,377,610,987,1597,2160`) and for short histories
  (`retained` = 0, 1, 2, 4, 100), and the deepest offset equals `min(k, retained)`.
- **CE-LV16-2 Selection.** Over a valid ancestry the candidates are newest first, at exactly those depths,
  and the deepest is present. The killed-store shape (tip `917`/5,369,626, parent `915`/5,369,625) offers
  `917` then `915` first.
- **CE-LV16-3 Ancestry faults.** An empty ancestry, a parent-hash break, a height gap, a non-decreasing
  slot, and an ancestry longer than k + 1 are each a distinct typed error.
- **CE-LV16-4 Reply.** `Found` naming an offered point gives that candidate and its depth; `Found` naming
  any other point is `IntersectionPointNotOffered`; `NotFound` is `NoCommonIntersection` with the bound
  and the deepest point offered.
- **CE-LV16-5 Wire.**
  - The pump sends one `FindIntersect` carrying every candidate, newest first.
  - On `IntersectFound(915)` it sends `RequestNext`; the peer's `RollBackward(915)` becomes a
    `RollBackward` event.
  - On `IntersectNotFound` it returns `NoCommonIntersection` and sends no `RequestNext`.
  - On an unoffered `IntersectFound` it returns `IntersectionPointNotOffered` and emits nothing beyond
    DC-PUMP-02's `TipUpdate`.
  - Neither outcome reconnects (`should_reconnect_after`).
- **CE-LV16-6 Ancestry read.**
  - Over a stored chain, the reader returns the validated ancestry to depth k, or to the oldest stored
    block.
  - A block whose recomputed hash differs from the hash it was found by is refused.
  - A tip that is not stored (the recovered anchor) is offered alone, as `NoStoredAncestry`.
- **CE-LV16-7 Killed-store shape, hermetic.**
  - The setup is a durable chain ending `…, 915, 917` and a peer whose chain contains `915` but not
    `917`.
  - The offer includes `915`; the intersection is `915`.
  - The followed `RollBackward(915)` leaves tip `915` and a `WalEntry::RollBack`.
  - The recovered ledger state equals the reference execution's state at the same point, where the
    reference holds `…, 915` and never saw `917`.
- **CE-LV16-8 Killed store, live** (release; on disposable checksummed copies, never the master):
  - **Recovery:** the fixed binary logs the intersection at `138046915/d9fe3452` and
    `rollback-followed: to_slot=138046915`, then follows forward.
  - **Matches the run before the kill, at the same chain points:**
    - The accumulator fingerprint is equal wherever both the recovered run and the uninterrupted run
      logged one. These points are the refold's per-epoch pass ends, which are the last block before
      each boundary, and any per-block cursor both runs logged.
    - Two whole stores are compared byte for byte with `lv16_recovered_store_equivalence` wherever they
      stand at the same cursor.
    - The frozen 323 record is fixed for the whole epoch, so its canonical encoding is byte-compared
      with the uninterrupted 323 capture whatever the cursor.
  - **Two recoveries agree:** a second copy recovered independently passes the same comparisons, at the
    chain points both runs share after the rollback.
  - **Master untouched:** the master's checksums are unchanged afterwards.

  Already observed on the master, before any recovery: the S5 pre-clear had rewound the accumulator to
  its settled point, cursor 137,894,414 (the first block of epoch 323). That state is byte-identical, all
  10,149,025 bytes, to the uninterrupted run's capture at the same cursor. The rewind inside the crash
  window restored exact state. Its anchor is absent, so recovery still refolds from the bootstrap (RF-1);
  that costs time, not correctness, and is not changed here.
- **CE-LV16-9 Gate.**
  - The new gate `ci/ci_check_durable_intersection.sh` passes.
  - These existing gates stay green: store semantics lock and gate, rollback target canonical binding,
    recovered anchor intersectable, and frozen ratification authority.
- **CE-LV16-10 Mutations.** Each must be killed by its intended test and named by the gate:
  - (a) offer only the tip;
  - (b) drop the deepest (depth `l`) candidate;
  - (c) order oldest first;
  - (d) accept an `IntersectFound` naming an unoffered point;
  - (e) on `IntersectNotFound`, send `RequestNext` (the pre-slice behaviour);
  - (f) reconnect after `NoCommonIntersection`;
  - (g) trust the hash index without recomputing the block hash;
  - (h) offer the bootstrap anchor below stored blocks.

---

## 7. Store semantics — neutral

No persisted field, encoding or interpretation changes. Candidates are computed from the existing
ChainDb and are never stored. The WAL, checkpoint and accumulator formats are untouched, and the store
semantics version stays 13.

---

## 8. Not in this slice

- **A resynchronisation path** for a peer that shares no point within the supported history: a fresh
  bootstrap is the operator path.
- **Multi-peer intersection policy.** Each peer is intersected independently, as today.
- **The depth-0 rollback on every reconnect.** After any intersection the peer sends `RollBackward` to
  the intersection point, which runs the accumulator pre-clear even at depth 0. That is existing
  behaviour and costs refold time, not correctness. It is recorded, not changed.
- **Host memory.** The one-live-node operating limit is a mitigation, not a code change.
- **The clean-stop investigation (LV-13/LV-15a)**, and the v12 run ignoring SIGINT and SIGTERM. Those are
  separate findings.
- **LV-15b-0 CE-10, CE-11 and CE-16.** They resume after this slice, starting from a preserved-store copy.

---

## 9. Order of work

1. Preserve the master copy and its checksums. *(done)*
2. Commit this contract.
3. Implement and test the BLUE selection, validation and reply classification.
4. Wire the request and reply through the pump, the restart dial and the in-process reconnect, with no
   ledger or governance change.
5. Run the existing replay, single-authority, rollback-binding and store-version gates, plus the new gate
   and the mutations.
6. Run CE-LV16-8 on disposable copies.
7. Push only when the slice's own invariants are mechanically enforced and the owner approves.
