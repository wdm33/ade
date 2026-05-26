# Cluster PHASE4-N-L-LIVE — Wire-only live smoke pass

> **Status:** Planning artifact (non-normative). Introduces
> `RO-LIVE-04` (wire-only smoke) and refines `RO-LIVE-03`'s
> `open_obligation` pointer. Carries `RO-LIVE-05`
> (admission/agreement) as `blocked_until_ledger_seed_cluster`.

## Primary invariant

> `ade_node --mode wire_only --peer ADDR --network NAME` opens TCP,
> completes the N2N handshake (`CN-SESS-02`), issues one chain-sync
> `FindIntersect(Origin)`, reads the peer's announced tip, and
> emits a closed-vocabulary JSONL log of the exchange before
> exiting cleanly. The wire-only event vocabulary is a closed
> sum (7 variants); the binary MUST NOT emit `agreement_verdict`,
> `admitted_block`, `ledger_applied`, or `projection_updated` —
> those belong to the admission cluster (`RO-LIVE-05`).

## Scope

- **GREEN (new):**
  - `ade_node::live_log::event` — closed `LiveLogEvent` enum +
    hand-rolled JSON serialization.
  - `ade_node::live_log::writer` — `LiveLogWriter` over a
    `std::io::Write` sink.
- **RED (new):**
  - `ade_node::wire_only` — wire-only mode entry point.
  - `ade_node::main` — extended with `--mode` parsing + routing.
- **BLUE:** unchanged.

Out-of-scope (declared in
`docs/planning/phase4-n-l-live-wire-smoke-invariants.md §10`):
admission / agreement_verdict / TLS / tx-submission /
peer-sharing / long-poll chain-sync / block production.

## Grounding (verified at HEAD `f988ed1`)

- **`ade_network::session::run_n2n_handshake_initiator`** —
  CN-SESS-02 sole handshake driver.
- **`ade_network::mux::transport::spawn_duplex`** — full-duplex
  bounded-queue TCP driver.
- **`ade_runtime::network::n2n_dialer::N2nDialer`** — outbound
  dialer + handshake + MuxPump spawn.
- **`ade_network::codec::chain_sync::{encode_chain_sync_message,
  decode_chain_sync_message, ChainSyncMessage}`** — N-A
  chain-sync codec; used to build the FindIntersect frame +
  parse the IntersectFound/NotFound reply.

## Slice index

| Slice | Scope | TCB |
|-------|-------|-----|
| S1 | GREEN `ade_node::live_log::{event, writer}` — closed `LiveLogEvent` enum + JSON serializer + writer. Forbidden-event-name CI gate. | GREEN + CI |
| S2 | RED `ade_node::wire_only` + main.rs `--mode` flag + per-peer dialer task + tip-read sequence. | RED |
| S3 | Hermetic loopback tests + JSONL validation + negative-event guards + operator-pass procedure doc. | RED + test |

Dependencies: S2 depends on S1; S3 depends on S2.

## Exit criteria (CI-verifiable)

- [ ] **CE-N-L-LIVE-1 (RO-LIVE-04 mechanical)** —
  `main_wire_only_exits_zero_after_tip_read` proves
  end-to-end against a loopback responder.
- [ ] **CE-N-L-LIVE-2 (¬P-1 enforcement)** —
  `ci/ci_check_wire_only_event_vocabulary_closed.sh` greps
  the `ade_node/src/` tree for the forbidden event-name string
  literals (`agreement_verdict`, `admitted_block`,
  `ledger_applied`, `projection_updated`). Test
  `main_wire_only_never_emits_agreement_verdict` asserts the
  same property dynamically.
- [ ] **CE-N-L-LIVE-3 (¬P-2 enforcement)** —
  `main_without_genesis_does_not_attempt_admission` asserts
  the wire-only mode does NOT call `bootstrap_initial_state`.
  Implemented via a grep on the wire-only-reachable source +
  a behavior test that drops a fresh empty store and proves
  no GenesisRequiredButAbsent ever surfaces.
- [ ] **CE-N-L-LIVE-4 (I-7 enforcement)** —
  `jsonl_events_are_valid_one_object_per_line` parses the
  emitted file line-by-line and asserts each line is a valid
  single JSON object with the `event` discriminator field.
- [ ] **CE-N-L-LIVE-5 (signal-shutdown)** —
  `main_signal_shutdown_flushes_jsonl` simulates a
  mid-tip-read shutdown and asserts the final line is a
  complete `node_shutdown` event.
- [ ] **CE-N-L-LIVE-6 (RO-LIVE-04 live)** — operator captures
  a real JSONL log against the AWS cardano-node + attaches it
  to `docs/clusters/PHASE4-N-L-LIVE/CE-N-L-LIVE-LIVE.log`.
  Flips RO-LIVE-04 to `enforced` at cluster close.

> No human review may substitute for CE-N-L-LIVE-1..5.
> CE-N-L-LIVE-6 IS an operator-action evidence step (the
> point of the cluster).

## TCB color map (FC/IS partition)

- **BLUE:** unchanged.
- **GREEN:**
  - `crates/ade_node/src/live_log/event.rs`
  - `crates/ade_node/src/live_log/writer.rs`
  - `crates/ade_node/src/live_log/mod.rs`
- **RED:**
  - `crates/ade_node/src/wire_only.rs`
  - `crates/ade_node/src/main.rs` (extended)
  - `crates/ade_node/src/cli.rs` (extended — `--mode`, `--log`,
    `--tip-read-timeout-secs`)

## Forbidden during this cluster

- No emitting / logging / serializing the strings
  `agreement_verdict`, `admitted_block`, `ledger_applied`,
  `projection_updated` from ANY ade_node source file
  (closed-enum + CI grep).
- No call to `bootstrap_initial_state` from the wire-only
  code path.
- No `mpsc::unbounded_channel` in the live-log writer or
  wire-only module.
- No retry / backoff loop in `wire_only` — one dial per peer,
  one tip-read, exit.
- No serde-derive dependency added to ade_node — hand-rolled
  JSON over the 7-variant enum keeps the surface tiny.

## Replay obligations introduced

- New canonical replay surface: the JSONL event stream is
  byte-identical for a deterministic clock + deterministic
  loopback responder trace.

## Open obligations carried after closure

- `RO-LIVE-03` —
  `open_obligation = "blocked_until_RO-LIVE-04_and_RO-LIVE-05_close"`.
  Closes when both halves close.
- `RO-LIVE-05` —
  `open_obligation = "blocked_until_ledger_seed_cluster"`.
  Future cluster (`PHASE4-N-M-LEDGER-SEED` or equivalent).
- `RO-LIVE-01`, `RO-LIVE-02`, `CN-CONS-06` — unchanged.

## Authority reminder

Correctness rules live in
`docs/ade-invariant-registry.toml`. If guidance here conflicts
with the registry:

> **Registry + CI enforcement win.**
