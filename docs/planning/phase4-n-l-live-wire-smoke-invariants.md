# PHASE4-N-L-LIVE — Wire-only live smoke pass — invariants sketch

## Framing

PHASE4-N-L shipped the mechanical wire layer (session reducer +
mux pump + dialer + handshake driver). The binary `ade_node`,
however, is still the PHASE4-N-K honest-scope stub: it parses CLI
and prints `ready` and exits SUCCESS. No actual dial happens.

This cluster wires the stub into a real wire-only mode:
`ade_node --mode wire_only --peer ADDR --network NAME` opens TCP,
runs the N2N handshake, issues one `chain-sync FindIntersect(Origin)`,
reads the peer's announced tip, emits a closed-vocabulary JSONL
log of the exchange, and exits cleanly. The pass is then runnable
against a real cardano-node (the operator's AWS-hosted relay).

**Hard scope split (per
[[feedback-shell-must-not-overstate-semantic-truth]]):**

- This cluster closes **RO-LIVE-04 (wire-only smoke)** —
  handshake + peer tip read + JSONL emit + clean exit. The
  mechanical half (build + run + signal-shutdown) is CI-evidenced;
  the live half (against a real cardano-node) is operator-action
  evidence captured as a JSONL log file in
  `docs/clusters/PHASE4-N-L-LIVE/`.
- This cluster **DOES NOT** close RO-LIVE-05 (admission /
  agreement_verdict). That requires a real initial ledger state
  (genesis-bundle → initial UTXO seeding, or imported
  cardano-node ledger snapshot) — `PHASE4-N-M-LEDGER-SEED` (or
  equivalent) is the follow-on cluster's deliverable.
- This cluster **DOES NOT** close RO-LIVE-03. RO-LIVE-03 was
  found at planning time to bundle two distinct layer claims;
  per append-only discipline its statement is untouched but its
  `open_obligation` is refined to
  `"blocked_until_RO-LIVE-04_and_RO-LIVE-05_close"`. RO-LIVE-03
  flips to enforced when both halves close.

Predecessor anchors (HEAD `f988ed1`): PHASE4-N-L (session
reducer + mux pump + dialer + handshake driver).

## 1. What must always be true

- **I-1 Closed JSONL event vocabulary.** The log a wire-only run
  emits is a closed sum of seven variants: `node_started`,
  `peer_dial_started`, `handshake_ok`, `peer_tip_read`,
  `peer_dial_failed`, `wire_smoke_complete`, `node_shutdown`.
  Adding a variant requires an explicit code change + a
  corresponding allow-list update in the CI gate.
- **I-2 Wire-only mode never claims semantic truth.** The
  binary in `wire_only` mode MUST NOT emit
  `agreement_verdict`, `admitted_block`, `ledger_applied`,
  or `projection_updated`. Type-level: the closed event enum
  has no such variants. CI grep: the string literals do not
  appear in any wire-only-mode reachable source.
- **I-3 Fail-closed on missing prerequisites.** If `--mode` is
  not `wire_only` and no ledger-seed path is available, the
  binary exits with `EXIT_GENERIC_STARTUP` and emits a single
  `node_started { mode: "wire_only_required" }` event followed
  by `node_shutdown { reason: "ledger_seed_unavailable" }`. No
  attempt to dial, no half-done admission, no silent fallback.
- **I-4 Tip read is a single round-trip.** The wire-only pass
  sends exactly one `FindIntersect(Origin)`, waits for one
  `IntersectFound` or `IntersectNotFound` reply (peer tip is
  embedded in the reply), emits `peer_tip_read`, sends `Done`,
  closes the socket. No streaming, no batch fetch, no
  long-poll. Time-bounded by a configurable
  `--tip-read-timeout-secs` (default 30 s) — exceeded → emit
  `peer_dial_failed { reason: "tip_read_timeout" }` and exit
  non-zero.
- **I-5 Per-peer isolation at the binary level.** With multiple
  `--peer` flags, each peer's dial + tip-read runs as an
  independent task. One peer's failure does not abort the
  others; each peer emits its own JSONL events tagged by
  `peer` address.
- **I-6 Deterministic exit code mapping.** `EXIT_SUCCESS = 0`
  iff every dialed peer completed its handshake + tip-read.
  `EXIT_GENERIC_STARTUP = 1` for CLI / config errors.
  `EXIT_AUTHORITY_FATAL_IO = 10` reserved (not reachable in
  wire-only). `EXIT_LIVE_PASS_PEER_FAILURE = 20` (new) if at
  least one peer dialed but did not complete tip-read.
- **I-7 JSONL is one valid object per line.** Each line is a
  single JSON object with `"event"` discriminator + per-variant
  fields. No multi-line objects, no trailing commas, no
  comments. The file is parseable by `jq -c '.' < log.jsonl`.

## 2. What must never be possible

- **¬P-1** Emitting `agreement_verdict`, `admitted_block`,
  `ledger_applied`, or `projection_updated` from any binary
  mode this cluster ships. The closed enum forbids it; CI grep
  on the ade_node source forbids it as a string literal.
- **¬P-2** Calling `bootstrap_initial_state` in wire-only mode.
  Wire-only must not even attempt the bootstrap — there is no
  ledger seed, so the call would error and the binary would
  exit with the wrong code.
- **¬P-3** Emitting partial JSONL lines on signal interruption.
  Each event line is flushed atomically; SIGINT/SIGTERM during
  a tip-read produces a complete `node_shutdown` event before
  exit.
- **¬P-4** Silently retrying a failed dial. A peer dial that
  fails emits exactly one `peer_dial_failed` event and that
  peer's task exits. No backoff loop in this cluster.
- **¬P-5** Cross-peer event interleaving without a `peer`
  field. Every per-peer event MUST carry its peer address so
  the operator can de-interleave the log.
- **¬P-6** Calling `n2n_dialer::dial` from a non-wire-only
  mode without first verifying that the ledger seed exists.
  Mode-gated.

## 3. What must remain identical across executions

- The JSONL event sequence for a given `(peer trace, clock
  trace)` is byte-identical across runs. The wire-only mode
  uses `DeterministicClock` for any timing operations under
  test (the `SystemClock` is used in the production binary,
  but timestamps in the event records are emitted as the
  `Clock::now_millis()` output — so under a deterministic
  clock the trace is byte-identical).
- Exit code is determined by the closed `(success | peer_dial_failed
  | startup_error)` outcome.

## 4. What must be replay-equivalent

- The JSONL writer over a recorded `LiveLogEvent` vector
  produces byte-identical output across two runs.

## 5. State transitions in scope

```text
ade_node main(argv)
  -> parse Cli
  -> if cli.mode == WireOnly: enter wire_only_run(cli) [this cluster]
  -> else: enter admission_run(cli) [PHASE4-N-M-LEDGER-SEED]
  -> on signal: emit node_shutdown, flush, exit

wire_only_run(cli)
  -> open jsonl_writer (path = cli.log_path or "./wire_smoke.jsonl")
  -> emit node_started { mode: WireOnly, peer_count: N }
  -> spawn per-peer task for each cli.peer_addrs:
       peer_session(addr) -> emits peer_dial_started, then
         handshake_ok | peer_dial_failed, then
         peer_tip_read | peer_dial_failed
  -> wait for all per-peer tasks to complete
  -> emit wire_smoke_complete { admission_enabled: false }
  -> emit node_shutdown { reason: TipReadComplete }
  -> exit EXIT_SUCCESS (or EXIT_LIVE_PASS_PEER_FAILURE if any peer failed)
```

## 6. TCB color hypothesis

- **GREEN (new):**
  - `ade_node::live_log::event` — closed `LiveLogEvent` sum +
    `WireOnlyEventKind` discriminant + JSON serialization.
  - `ade_node::live_log::writer` — `LiveLogWriter` over a
    `std::io::Write` sink (file or stdout). Pure (modulo the
    write trait).
- **RED (new):**
  - `ade_node::wire_only` — tokio-driven wire-only mode entry
    point.
  - `ade_node::main` — extended with `--mode` parsing + routing.
- **BLUE:** unchanged.

## 7. Decisions on framing questions

| # | Question | Decision |
|---|----------|----------|
| 1 | JSONL serializer | Hand-rolled JSON (the project avoids serde-derive on RED surfaces; the event vocabulary is tiny — 7 variants). No serde_json dep added to ade_node. |
| 2 | Per-peer task model | One `tokio::spawn` per peer; each owns its own dialer + JSONL forward channel. Per-peer isolation mirrors PHASE4-N-K DC-NODE-01. |
| 3 | Tip-read mechanism | One chain-sync `FindIntersect(Origin)` → read one reply → send `Done`. The reply carries the tip in the `tip` field. |
| 4 | Default log path | `./wire_smoke.jsonl` in the binary's CWD. Operator override via `--log PATH`. |
| 5 | Default tip-read timeout | 30 s. Operator override via `--tip-read-timeout-secs N`. |
| 6 | Default `--mode` | `wire_only`. The admission mode requires explicit `--mode admission` AND a ledger seed prerequisite; until the seed cluster lands, admission mode exits 1 with a clear error. |
| 7 | Network name handling | `--network mainnet/preprod/preview` maps to the protocol magic via the existing `MAINNET_NETWORK_MAGIC` constant set. Mainnet magic by default. |
| 8 | Closing RO-LIVE-04 | The operator pass must capture a real JSONL log against a real cardano-node and attach it to `docs/clusters/PHASE4-N-L-LIVE/`. Log capture is part of the cluster-close commit. |

## 8. Registry deltas (planned at /cluster-plan)

- `RO-LIVE-03` — `open_obligation` rewritten from
  `"blocked_until_operator_peer_available"` to
  `"blocked_until_RO-LIVE-04_and_RO-LIVE-05_close"`. Statement
  untouched (append-only).
- `RO-LIVE-04` — new, `declared`, scope = wire-only smoke.
  Flipped to `enforced` at cluster close with the captured
  JSONL log as evidence.
- `RO-LIVE-05` — new, `declared`,
  `open_obligation = "blocked_until_ledger_seed_cluster"`.
  Carried forward.

## 9. Slice shape (proposed; refine at /cluster-plan)

| Slice | Scope | TCB |
|-------|-------|-----|
| S1 | GREEN `ade_node::live_log::{event, writer}` — closed `LiveLogEvent` enum + hand-rolled JSON serializer + `LiveLogWriter`. CI gate: forbidden event-name string literals across `ade_node/src/`. | GREEN + CI |
| S2 | RED `ade_node::wire_only` module + main.rs `--mode` flag + per-peer dialer task + tip-read sequence + signal-handler shutdown. | RED |
| S3 | Tests: hermetic loopback responder + JSONL validation + negative-event guards. + operator-pass procedure doc. | RED + test |

## 10. Honest-scope carry-forward

- **Admission / agreement_verdict / projection_updated** — out
  of scope (RO-LIVE-05 + PHASE4-N-M-LEDGER-SEED).
- **TLS / authenticated transport** — out of scope
  (PHASE4-N-L ¬P-8).
- **Tx-submission / mempool / peer-sharing** — out of scope.
- **Long-poll chain-sync** — out of scope; wire-only is
  one-round-trip-and-exit.
- **Block production** — out of scope; CN-CONS-06 still
  operator-action.

## 11. Why this is the right next cluster

PHASE4-N-L made the wire layer mechanically correct; this
cluster proves it works against a real cardano-node and pins
the JSONL vocabulary as a closed contract. It doesn't pretend
to evidence ledger admission — that's the next cluster's job,
and the registry now encodes the split mechanically so neither
clusterclose can overstate the other's evidence.
