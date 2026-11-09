# PHASE4-N-F-G-K — Node serve lifetime decoupled from feed end

> **One-slice cluster.** A live C1 genesis-successor rehearsal run (2026-06-04, current binary) proved
> Ade's `--mode node` forges and self-accepts block 0 (`forge_attempted → forge_result:"succeeded"`)
> against a healthy Haskell follower session — but the follower never fetched it: the serve listener is
> torn down the moment the upstream feed ends, before the follower's ~160 s `:3002` retry lands. The
> serve task is already read-only over `ServedChainView`; the defect is **caller-side lifecycle
> coupling** in the `--mode node` On-arm. This cluster decouples serve-listener lifetime from feed-end.
>
> Grounding: the live-run state is recorded in `[[project_phase4_c1_genesis_rehearsal_live_state]]`; the
> coupling is at `crates/ade_node/src/node_lifecycle.rs` (serve task spawned in the `--listen` branch,
> stop flipped right after `run_relay_loop` returns).

## §0 Slices with sharply different IDD status

- **Mechanical, hermetically closeable (S1):** the serve-listener lifetime decoupling (`DC-NODE-09`) —
  closes on a focused async lifetime test + a structural gate. `run_node_serve_task` is reused verbatim;
  only its stop trigger moves from feed-end to the node lifecycle owner.
- **Operator-gated (the rehearsal completion):** the C1 rerun where the follower actually fetches +
  validates the served block stays `blocked_until_operator_c1_genesis_successor_rehearsal`. No RO-LIVE
  flip; acceptance only via the follower log through `correlate`.

## §1 Primary invariant (DC-NODE-09)

Once `--mode node` has spawned a `--listen` serve task over `ServedChainView`, feed-end alone MUST NOT
terminate that serve task. The serve listener remains alive **under the node lifecycle owner** until
**explicit node shutdown, a fatal serve error, or lifecycle-owner cancellation** — never merely because
the upstream feed (relay loop) ended. The serve task stays read-only over `ServedChainView` (fed only by
`forge → self_accept → SelfAcceptedHandoff → push_atomic`); decoupling grants **availability, not
authority**. Registry: `DC-NODE-09`, declared → enforced at close.

## §2 The defect (observed, not assumed)

Live C1 run (current binary): Ade self-accepts block 0 (`forge_attempted → forge_result:"succeeded"`),
the follower promotes Ade to a hot peer and runs ChainSync, but the follower never connects to Ade's
serve port `:3002` during Ade's listening window — Ade's `--mode node` halts when its feed ends
(~40–100 s, `forge_tick_skipped:"no_block_available"`), and the follower retries `:3002` only every
~160 s (`PromoteColdFailed` backoff). The windows never overlap. The block exists and is served-capable;
the listener just dies too early. This is lifecycle **availability**, not a forge / codec / PrevHash /
consensus defect.

## §3 The load-bearing FC/IS fact (why decoupling is safe)

`run_node_serve_task` (`node_lifecycle.rs:737`) takes only `(TcpListener, ServedChainView, network_magic,
shutdown watch)` — **no `ChainDb`, no WAL, no forge engine, no mutable ledger/chain_dep**. It is
structurally incapable of mutating authoritative state, advancing the durable tip, forging, or admitting
peer blocks. It serves request-driven ChainSync/BlockFetch from `ServedChainView` via the single shared
dispatch (`DC-NODE-07`), dropping peers on error, "never mutate authoritative state" (`:805`). Extending
its lifetime grants **zero** new authority — it keeps the "served window" open longer, nothing more.

## §4 Why the previous coupling existed (preserve its guarantee)

`run_relay_loop` may halt **clean** (feed-end / `HaltCleanly`) **without** the operator `shutdown` watch
flipping (the `:522-526` comment states exactly this). The On-arm stops the serve task right after the
loop returns to **guarantee the process exits** — otherwise an orphaned serve task with no stop trigger
would keep the process alive. That guarantee is correct and MUST be preserved: this cluster **moves** the
termination trigger from feed-end to the lifecycle owner (`shutdown` / fatal serve error / cancellation),
it does **not** remove it.

## §6 TCB color map

- **RED** — `ade_node::node_lifecycle` On-arm serve-task lifecycle wiring (the stop trigger). The only
  change. `run_node_serve_task`'s body is reused verbatim (it already terminates on its shutdown watch,
  a fatal accept error, and channel close).
- **No BLUE, no GREEN reducer change. No new canonical type.** Serving is RED I/O over
  already-self-accepted bytes → no replay weight.

## §7 Slices

| Slice | Scope | CE | TCB | Registry | Status |
|---|---|---|---|---|---|
| **S1** | Serve-task stop trigger moves from feed-end to the node lifecycle owner (`shutdown` / fatal serve error / cancellation); serve listener survives a clean feed-end halt; clean termination preserved | CE-G-K-1, CE-G-K-2 | RED | `DC-NODE-09` → enforced | planned |

## §8 Cluster Exit Criteria

- **CE-G-K-1 (mechanical):** after a self-accepted block is in `ServedChainView` and the feed loop ends
  (clean halt, `shutdown` not flipped), the serve listener remains alive and a late peer can BlockFetch
  that block. Named tests + gate resolved in the S1 slice doc.
- **CE-G-K-2 (mechanical):** explicit node shutdown (and a fatal serve error) terminates the
  longer-lived serve task cleanly — no hang, no leaked task. Named tests resolved in the S1 slice doc.
- **CE-G-K-3 (operator-gated):** a C1 rerun where Ade forges block 0, the serve listener stays alive past
  the follower's retry, the follower fetches + validates the null-prev block, and `correlate` produces
  the `PrivateRehearsalManifest`. `blocked_until_operator_c1_genesis_successor_rehearsal`; no RO-LIVE
  flip; acceptance only from the follower log through `correlate`.

## §9 Replay obligations

None new. No canonical type, no authoritative transition, no durable-state change. The served bytes are
the already-self-accepted block (`DC-FORGE-01` unchanged). Serving is RED I/O, outside the replay weight
class.

## §10 Invariants

- **Preserves:** `DC-NODE-06` (self-accept handoff), `DC-NODE-07` (single shared serve source),
  `DC-NODE-08` (cold-start reachability), `CN-NODE-04`, `DC-FORGE-01`, `CN-WIRE-09`, `RO-LIVE-01/06` (no
  flip). The process-termination guarantee of the old design is preserved (moved to the lifecycle owner,
  not removed).
- **Adds:** `DC-NODE-09` declared → enforced (serve-listener lifetime).

## §11 Forbidden during this cluster

No forge-semantics change; no PrevHash work; no durable block-1+ progression; no co-producer workaround;
no private-only flag; no RO-LIVE flip; no acceptance claim without the follower log through `correlate`;
**no serve of bytes outside `SelfAcceptedHandoff → ServedChainView`**; no `ChainDb` mutation / durable
tip advance / peer-block admission from the serve task; no BLUE change; **no unbounded /
never-terminating serve** — the serve task MUST still terminate on shutdown / fatal serve error /
lifecycle cancellation.

## §12 Open questions

- **OQ-K1 (resolved, scope):** after the feed loop returns, is the node serve-only until shutdown, or
  does it re-enter the feed loop? → **serve-only-until-shutdown.** WirePump feed reconnect/retry is a
  NON-GOAL (the block is already forged + served; the follower fetches on its own cadence).
- **OQ-K2 (resolved, S1):** hermetic test shape for a lifetime property → a focused async test over
  `run_node_serve_task` (loopback `TcpListener` + a client that BlockFetches a view-held block, then
  `shutdown` flips and the task ends) + a structural gate over the On-arm wiring.

## §13 Non-goals

WirePump feed reconnect/retry; cross-epoch; durable block-1+ progression and durable suppression;
C2/preprod execution; any change to `run_node_serve_task`'s serve dispatch or to the forge/self-accept
path; the Off-arm (forge off), which spawns no serve task.
