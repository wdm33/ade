# Cluster/Slice Plan — PHASE4-N-F-G-J (Node forge when feed is empty/at tip)

> Single follow-on sub-cluster (like G-E/G-H/G-I). Source: invariants sketch
> `docs/planning/phase4-n-f-g-j-invariants.md` (committed `9eb6f39b`); declared rules
> `CN-NODE-04` (operational) + `DC-NODE-08` (derived). A **shared `--mode node`** path
> change (node-lifecycle producer scheduling), **not** a C1-only path — the same change
> serves C2/preprod (unexercised there because preprod's feed is already Continuing). C1
> (the kept `~/.cardano-private-testnet-c1-conway` Conway net, whole-second `systemStart`)
> is the regression harness that reproduces the empty-feed halt.

## Cluster Index (Dependency Order)
1. **PHASE4-N-F-G-J** — Node forge when feed is empty/at tip — primary invariant: `DC-NODE-08`
   — `--mode node` may run the producer tick from the **recovered authoritative base** when
   the feed is `Empty|AtTip|NoBlockAvailable` (and only then), gated on a recovered base +
   `ForgeIntent::On` + a **due forge slot**, with **epoch/KES/leader eligibility enforced by
   the existing forge path**, and the forged block still flowing
   `self_accept → SelfAcceptedHandoff → ServedChainView`; a peer-lost/error feed stays
   fail-closed/halt.

## Planner-is-scheduler boundary (read first)
The pure planner (`run_loop_planner::plan_loop_step`) decides **WHEN to attempt a forge tick**
(scheduling), **not** whether Ade is the leader. Leadership/epoch/KES authority stays where it
is — the BLUE leader check + KES/opcert inside `forge_one_from_recovered` and `DC-EPOCH-03`.
S2 adds only a **scheduling** allowance (eligible empty/at-tip feed + recovered base + producer
intent + a due forge slot → emit `ForgeTick`); the forge path still enforces leader eligibility
and may return `ForgeNotLeader` (no block). The planner never becomes a leadership authority.

## Key load-bearing finding (S1/S2 spine)
`NodeBlockSource` today collapses every feed-end into a single `is_ended: bool` — **no
distinction** between "ended because the genesis/sole-producer peer had nothing" (eligible) and
"ended due to peer-loss/decode/protocol error" (ineligible). The **closed feed-state taxonomy**
is the spine of this cluster — a closed split:

- **eligible:** `NoBlockAvailable` / `AtTip` / `CleanEmpty`
- **ineligible:** `PeerLost` / `DecodeError` / `ProtocolError` / `SourceInvalid`

S1 introduces the taxonomy (emit-only, no behavior change); S2 consumes it (the one semantic
gate). The C1 case (WirePump channel disconnected because the genesis peer had nothing to serve)
**must** classify as an **eligible** `CleanEmpty`/`NoBlockAvailable` case, **never** `PeerLost`.

## PHASE4-N-F-G-J — Node forge when feed is empty/at tip
- **Primary invariant:** `DC-NODE-08` (declared → enforced at S2 close). Introduces `CN-NODE-04`
  (declared → enforced at S1 close). Preserves `DC-NODE-05/06/07`, `DC-EPOCH-03`, `CN-CINPUT-03`,
  `DC-CINPUT-02b`, `DC-FORGE-01`, `CN-FORGE-01..04`; cross-refs `RO-LIVE-01` (the live ACCEPT it
  unblocks, still operator-gated).
- **TCB partition:**
  - **BLUE** [reused, unchanged] — `forge_one_from_recovered`'s forge composition + leader check
    + `self_accept` (`ade_core`/`ade_ledger`; `CN-FORGE-*`, `DC-FORGE-01`). *Any BLUE change is a
    red flag → reject.*
  - **GREEN** — `ade_node::run_loop_planner` (`plan_loop_step` / `forge_slot_status` — the S2
    decision-table extension); the **closed feed-state taxonomy enum** (new); the **closed
    `CN-NODE-04` event vocabulary** (`ade_node::live_log::event`).
  - **RED** — `ade_node::node_sync` (`NodeBlockSource` feed-state classification — the `is_ended`
    → taxonomy mapping); `ade_node::node_lifecycle` + `run_node_sync` (relay loop — executes the
    planner, emits events); `ade_node::live_log::writer` (emission).
- **Cluster Exit Criteria:**
  - **CE-G-J-1** (mechanical, S1): `--mode node` emits the **closed, allow-listed** `CN-NODE-04`
    vocabulary — `feed_unavailable{reason}` (closed reason enum with the eligible split
    `NoBlockAvailable|AtTip|CleanEmpty` vs ineligible `PeerLost|DecodeError|ProtocolError|
    SourceInvalid`) + `forge_tick_considered` / `forge_tick_skipped{reason}` / `forge_attempted`
    / `forge_result{outcome}` — through `live_log`; an allow-list **negative test** drops
    non-vocabulary/stringly lines; a CI gate enforces **emit-only** (`run_loop_planner` /
    `plan_loop_step` never reads a `LiveLogEvent`); and a **no-behavior-change** proof —
    `plan_loop_step`'s decision is byte-identical pre/post S1 (the existing precedence-table /
    determinism tests stay green, unchanged). `CN-NODE-04` declared → enforced.
  - **CE-G-J-2** (mechanical, S2): `plan_loop_step`, given an **eligible** feed
    (`NoBlockAvailable|AtTip|CleanEmpty`) + a **recovered base** + **producer intent**
    (`ForgeIntent::On`) + a **due forge slot**, returns `ForgeTick` (was `Halt`) — with
    **epoch/KES/leader eligibility still enforced by the existing forge path**
    (`forge_one_from_recovered`'s BLUE leader check + KES/opcert + `DC-EPOCH-03`); an
    **ineligible** feed (`PeerLost|DecodeError|ProtocolError|SourceInvalid`) still returns
    `Halt`/fail-closed; the extended table stays **total + deterministic**; a hermetic
    **forge-from-empty-feed** test proves the tick fires and the block `self_accept`s →
    `SelfAcceptedHandoff` → `ServedChainView`; the BLUE forge bytes/base are unchanged
    (`DC-FORGE-01` reused; `ci_check_consensus_input_provenance.sh` + `DC-NODE-06/07` gates stay
    byte-/semantically unchanged). `DC-NODE-08` declared → enforced.
  - **CE-G-J-3** (operator-gated, S3): a C1 rerun harness + runbook (strict adaptation of the
    G-H/G-D operator pattern) proving, on the real C1 net (whole-second `systemStart`), that
    `--mode node` now **reaches a forge tick + self-accepts** from the empty feed (the prior halt
    is gone), the S1 events showing `forge_tick_skipped` → `forge_attempted` → `forge_result`;
    `blocked_until_operator_c1_forge_rerun`; **no synthetic evidence; no RO-LIVE flip** (peer
    ACCEPT stays operator-gated via `correlate`).
- **Slices:**
  - **S1 Closed feed/forge scheduling events + feed-state taxonomy** — invariant: a closed,
    allow-listed diagnostic event vocabulary (`CN-NODE-04`) + the closed feed-state taxonomy
    (`NoBlockAvailable|AtTip|CleanEmpty` vs `PeerLost|DecodeError|ProtocolError|SourceInvalid`),
    **emit-only**, **no behavior change**. Introduce the closed feed-state classification in
    `NodeBlockSource` (RED) feeding a closed taxonomy enum (GREEN); emit the `CN-NODE-04` events
    via `live_log`; add the allow-list negative test + the emit-only CI gate + the
    no-behavior-change proof. — addresses: CE-G-J-1 — **TCB: GREEN** (vocab + taxonomy) **+ RED**
    (classification + emission); **no BLUE change, no scheduling change**.
  - **S2 Empty/at-tip recovered-base forge scheduling** — invariant: the `DC-NODE-08` gate —
    extend `plan_loop_step` so an **eligible** empty/at-tip feed + recovered base + producer
    intent + a **due forge slot** yields `ForgeTick` (ineligible feeds stay fail-closed),
    leadership/epoch/KES still enforced by the existing forge path, the forged block flowing
    through the existing `self_accept → handoff → served` path. Reuse `forge_one_from_recovered`
    **verbatim**. — addresses: CE-G-J-2 — **TCB: GREEN** (planner table) **+ RED** (loop wiring);
    **no BLUE change**.
  - **S3 C1 forge rerun harness + runbook** — invariant: the empty-feed forge mechanism is
    exercised on the real C1 net, the prior halt gone, acceptance still proven only via
    `correlate`. Env-gated/operator harness + runbook (A7 topology note: Ade is the **sole**
    producer, the Haskell node a **follower-not-co-producer** — no co-producer cheating).
    `blocked_until_operator_c1_forge_rerun`; no RO-LIVE flip. — addresses: CE-G-J-3 — **TCB: RED**
    (harness + runbook); **no BLUE change**.
- **Replay obligations:** **none new.** No new authoritative state, no new canonical type on the
  forge path; the forged block + `self_accept/handoff/served` effects stay byte-identical
  (`DC-FORGE-01` / `DC-NODE-06/07`). `plan_loop_step` stays a total, deterministic table (existing
  `…precedence_table_is_total` / `…is_deterministic` tests extended, not weakened). The feed-state
  taxonomy classifies a RED shell input — not authoritative state. `CN-NODE-04` events are
  **operational tier**, outside the replay-equivalence weight class.
- **FC/IS partition:** BLUE = `ade_core`/`ade_ledger` forge composition (reused). GREEN =
  `ade_node::run_loop_planner` + the feed-state taxonomy enum + `ade_node::live_log::event`. RED =
  `ade_node::node_sync` (`NodeBlockSource`), `ade_node::node_lifecycle` (relay loop),
  `ade_node::live_log::writer`.

## Hard lines (inherited by every slice)
- No co-producer workaround; no private-only/C1-only flag or branch.
- No bypass of `import_live_consensus_inputs`; forge base is the recovered surface only
  (`CN-CINPUT-03`/`DC-CINPUT-02b` byte-/semantically unchanged); no forge from unanchored genesis;
  no stale-base forge.
- No durable tip advance from forge scheduling alone; no serve of non-self-accepted bytes
  (`DC-NODE-06/07` unweakened); no BLUE change to the forge composition; the planner stays a
  scheduler, never a leadership authority.
- The planner **never consumes** `CN-NODE-04` events (emit-only, one-directional).
- No peer-acceptance/BA-02 claim without a real peer log through `ba02_evidence::correlate`; **no
  `RO-LIVE-01/06` flip** at implementation close.
