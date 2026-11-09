# Invariant Slice — PHASE4-N-F-G-K S1: Node serve lifetime decoupled from feed end

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header

- **Slice:** PHASE4-N-F-G-K S1 — the `--mode node` `--listen` serve task's termination trigger moves from
  feed-end (`run_relay_loop` return) to the node lifecycle owner (`shutdown` / fatal serve error /
  lifecycle cancellation). The serve listener survives a clean feed-end halt so a late peer can still
  BlockFetch an already-self-accepted block.
- **Cluster:** PHASE4-N-F-G-K — Node serve lifetime decoupled from feed end.
- **Status:** Merged (`b8829a6a`).
- **Cluster Exit Criteria addressed — CE-G-K-1 + CE-G-K-2.** (CE-G-K-3 = the operator-gated C1 rerun,
  out of slice scope.)

## §3 Slice Dependencies

- **DC-NODE-07** (single shared serve source) — the serve task already serves request-driven from
  `ServedChainView`; this slice only changes WHEN it stops.
- **DC-NODE-06** (`SelfAcceptedHandoff`) — `ServedChainView` is fed only by `into_accepted() →
  push_atomic`; the served block's provenance is unchanged.
- **DC-NODE-08** (cold-start reachability) — the self-accepted block 0 the listener must outlive comes
  from the cold-start forge.
- **CN-NODE-04** — the feed-end (`forge_tick_skipped` / `HaltCleanly`) that currently triggers the
  premature serve stop.

## §4 Intent (invariant impact)

Make the `--mode node` serve listener outlive a clean feed-end halt. Today the On-arm flips the serve
task's dedicated stop channel immediately after `run_relay_loop` returns (`node_lifecycle.rs:587-588`),
so a feed that ends (the relay loop halts clean, `shutdown` still false) tears down serving — and a peer
that retries later gets connection-refused. This is the exact, observed C1 blocker: Ade self-accepts
block 0 but the Haskell follower's ~160 s `:3002` retry lands after Ade has already stopped listening.
This slice gates the serve task on the **operator `shutdown` watch** (plus its own fatal-accept-error /
channel-close exits) instead, so the listener stays available until explicit shutdown / fatal serve error
/ lifecycle cancellation. `DC-NODE-09` declared → enforced. The process-termination guarantee the old
coupling provided is **preserved** — moved to the lifecycle owner, not removed.

## §5 Scope / What is built

1. **`ade_node::node_lifecycle` On-arm — serve-task stop trigger.**
   - The serve task is spawned with the **operator `shutdown` watch** (a clone), not a dedicated
     feed-end stop channel.
   - The `node_serve_stop.send(true)` flip after `run_relay_loop` returns (`:587-588`) is **removed**.
   - After `run_relay_loop` returns due to feed-end (`shutdown` false), the lifecycle owner **awaits the
     serve task** (`node_serve_handle`), which now ends only when `shutdown` flips (or a fatal serve
     error). On operator shutdown both the relay loop and the serve task observe `shutdown` and
     terminate; the node exits cleanly.
   - **`run_node_serve_task` body is unchanged** — it already breaks on its shutdown watch (`:752-754`),
     a fatal accept error (`:763`), and channel close (`:777`).
2. **Registry `DC-NODE-09`** — declared → enforced; the serve-listener-lifetime invariant.
3. **CI gate** `ci/ci_check_node_serve_lifetime.sh` — the On-arm wires the serve task to the operator
   `shutdown` watch (not a feed-end-flipped stop); no `node_serve_stop.send(true)` after
   `run_relay_loop`; the serve task signature takes no `ChainDb`/forge handle (read-only over
   `ServedChainView`).

**Out of scope:** `run_node_serve_task`'s serve logic (reused verbatim); the Off-arm (spawns no serve
task); WirePump feed reconnect.

## §6 Execution Boundary (TCB color)

- **RED** — the On-arm serve-task lifecycle wiring (the stop trigger). The only change.
- **No GREEN reducer change, no BLUE change.** `run_node_serve_task` (serve dispatch over
  `ServedChainView`) and the forge/self-accept path are untouched. No new canonical type → no replay
  weight.

## §7 Invariants Preserved

- **DC-NODE-07** — single shared serve source; the served bytes still come only from `ServedChainView`
  via the shared dispatch.
- **DC-NODE-06** — `ServedChainView` fed only by `SelfAcceptedHandoff::into_accepted() → push_atomic`; no
  new serve source.
- **DC-NODE-08 / DC-FORGE-01 / CN-WIRE-09 / CN-FORGE-01..04** — forge / self-accept / cold-start /
  PrevHash unchanged.
- **Process-termination guarantee** — the node still always terminates: on `shutdown` (both loop and
  serve task observe it), a fatal serve error, or lifecycle cancellation. The old feed-end stop is
  replaced by the lifecycle-owner trigger, not deleted.
- **RO-LIVE-01/06** — no flip.

## §8 Invariants Strengthened

**`DC-NODE-09`** declared → **enforced** (serve-listener lifetime). Once `--mode node` has a `--listen`
serve task over `ServedChainView`, feed-end alone does not terminate it; it ends only on explicit
shutdown / fatal serve error / lifecycle cancellation. The serve task remains read-only over
`ServedChainView`. **Registry:** new rule; tests + gate appended at close.

## §9 Open questions resolved in this slice

- **Serve-only-until-shutdown vs feed re-entry → resolved:** after a clean feed-end halt the node is
  serve-only until shutdown; feed reconnect is a non-goal.
- **Lifetime test shape → resolved:** a focused async test over `run_node_serve_task` (loopback
  `TcpListener` + a client that BlockFetches a view-held block, then `shutdown` flips and the task ends)
  + a structural gate over the On-arm wiring.

## §11 Replay / Crash / Epoch Validation

None new. No WAL/checkpoint/canonical-type change; no authoritative transition. The served block is
already self-accepted; serving is RED I/O. Determinism unaffected.

## §12 Mechanical Acceptance Criteria

Complete only when all pass in CI.

- [ ] `serve_task_survives_feed_end_and_serves_blockfetch` (CE-G-K-1) — a `run_node_serve_task` over a
  `ServedChainView` holding a self-accepted block: a loopback peer connecting AFTER a simulated feed-end
  can BlockFetch that block; the task is still alive (it has not been stopped).
- [ ] `serve_task_terminates_on_shutdown` (CE-G-K-2) — flipping the `shutdown` watch ends the serve task
  promptly (the `await` completes; no hang).
- [ ] `serve_task_terminates_on_fatal_accept_error` (CE-G-K-2) — a fatal accept fault ends the task
  (fail-closed), no leaked task.
- [ ] `node_onarm_serve_not_stopped_by_feed_end` — the On-arm does not flip a serve stop on
  `run_relay_loop` return; the serve task is wired to the operator `shutdown` watch.
- [ ] `bash ci/ci_check_node_serve_lifetime.sh` green — On-arm wires serve to the `shutdown` watch; no
  post-`run_relay_loop` `node_serve_stop.send(true)`; serve task takes no `ChainDb`/forge handle.
- [ ] `cargo test -p ade_node` green (unmasked exit code). *(Full `cargo test --workspace` unmasked is
  the cluster-close gate.)*

## §13 Failure Modes

- **Operator shutdown** — both the relay loop and the serve task observe the `shutdown` watch; the node
  exits cleanly (no hang, no leak).
- **Fatal serve accept error** — the serve task breaks (`:763`); the node proceeds to clean exit.
- **No self-accepted block in `ServedChainView`** — the listener stays up but serves nothing (a peer's
  BlockFetch finds no block); no fabricated serve. Unchanged.
- **CI/hermetic run with no operator** — the harness owns the shutdown/timeout; without it the
  serve-only wait blocks (by design — that is the availability the live peer needs). Tests flip
  `shutdown` explicitly.

## §14 Hard Prohibitions

Inherits cluster §11 in full. Slice-specific:
- **No unbounded / never-terminating serve** — the serve task MUST still terminate on shutdown / fatal
  serve error / lifecycle cancellation; the termination guarantee is preserved.
- **No serve of bytes outside `SelfAcceptedHandoff → ServedChainView`** — the serve source is unchanged;
  the slice only extends lifetime.
- **No `ChainDb` mutation / durable tip advance / peer-block admission** from the serve task — it remains
  read-only over `ServedChainView` (structurally: no such handle).
- **No BLUE change; no forge-semantics change; no PrevHash work; no durable block-1+; no co-producer
  workaround; no private-only flag; no RO-LIVE flip.**

## §15 Explicit Non-Goals

WirePump feed reconnect/retry; cross-epoch; durable block-1+ progression and durable suppression;
C2/preprod execution; any change to `run_node_serve_task`'s serve dispatch or to the forge/self-accept
path; the Off-arm.
