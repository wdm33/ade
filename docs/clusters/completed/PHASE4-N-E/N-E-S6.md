# Invariant Slice — PHASE4-N-E S6

## Slice Header
**Slice Name:** Live N2N tx-submission2 session binary + closure-gate test
**Cluster:** PHASE4-N-E
**Status:** Proposed
**CEs addressed:** CE-N-E-6 (live-wire half of the binary mechanical evidence; live-log half via operator `--connect` pass)
**Dependencies:** S1 (BLUE chokepoint), S2 (replay harness), S3 (canonicalizer), S4 (`ade_core_interop::tx_submission` GREEN bridge); PHASE4-N-A closed (handshake + tx-submission2 codec + state machine + mux frame)

---

## Intent

Ship a sustained-window RED probe binary modeled on
`live_consensus_session.rs` that:

1. Connects to a real cardano-node N2N peer as outbound client.
2. Negotiates the N2N handshake.
3. Opens the tx-submission2 mini-protocol (id 4) and drives the BLUE
   `tx_submission2_transition` state machine against the peer's
   server-agency messages for a sustained window.
4. Funnels any opportunistically-received `InventoryEvent::TxsDelivered`
   bytes through the S4 `event_to_ingress` / `PeerAccumulator` bridge.
5. Writes `docs/clusters/PHASE4-N-E/CE-N-E-6_<date>.log` per the
   schema documented in `CE-N-E-6_PROCEDURE.md`.

The default hermetic `main` prints a readiness banner and exits so
the `#[ignore]` build-and-start closure-gate test stays offline; the
operator passes `--connect` to perform the live capture.

---

## Honest scope (what this binary evidences vs what it does NOT)

**Evidences:**
- N2N handshake against a real cardano-node 11.x peer accepted.
- tx-submission2 mini-protocol opened (protocol id 4).
- BLUE `tx_submission2_transition` drives the protocol grammar for
  every received message without `IllegalTransition` /
  `MalformedMessage` errors.
- Every codec round-trip (encode/decode of every
  `TxSubmission2Message` variant we exchange) succeeds against live
  wire bytes — proving the codec is wire-compatible with
  cardano-node's tx-submission2 implementation.
- The cluster's `ade_core_interop::tx_submission` bridge is wired
  end-to-end (any opportunistic tx delivery flows through it).

**Does NOT directly evidence (joins CE-NODE-N2C-LTX in the future
node-binary cluster's deferral):**
- Bulk receipt of real tx_bytes from the peer. In the outbound
  client direction the peer does not push txs at us; bulk tx
  ingestion requires Ade to host an inbound listener (real node
  scaffolding). If the peer happens to send `ReplyTxs`
  opportunistically (e.g., in response to a future responder-mode
  patch), the bridge captures them and the log records
  `[bridge] tx_bytes=<N>`.

---

## The change

### 1. New binary `crates/ade_core_interop/src/bin/live_tx_submission_session.rs`

~390 lines, modeled directly on `live_consensus_session.rs`:

- `main()` — hermetic default (prints readiness banner); `--connect`
  dispatches to `run_live`.
- `run_live(args)` — connect + handshake + send `Init` + drive the
  state machine in a sustained loop until the deadline or
  `max_frames`. Replies empty for non-blocking `RequestTxIds` and
  `RequestTxs`; holds open on blocking `RequestTxIds` (loop continues
  reading; mempool-gossip cadence).
- `connect_and_handshake`, `write_frame`, `read_frame`,
  `read_for_protocol` — same shape as the chain-sync probe; lifted
  with minimal change. Frames for protocols other than
  tx-submission2 are buffered (and ignored) so multiplexed traffic
  doesn't block reads.
- `utc_stamp` / `utc_stamp_full` / `epoch_secs_to_ymd` — same
  no-dependency UTC helpers; date-only stamp for the filename, full
  ISO-8601 timestamp for the session window in the log.

### 2. New `[[bin]]` entry in `crates/ade_core_interop/Cargo.toml`

Mirrors the existing `live_consensus_session` entry.

### 3. New closure-gate test
   `crates/ade_core_interop/tests/live_tx_submission_session.rs`

Same shape as `tests/live_consensus_session.rs`:

- `#[test] #[ignore = "needs network egress …"]` —
  `cardano_node_tx_submission2_sustained_window`.
- Hermetic body: locates the binary in `target/{debug,release}/`,
  spawns it WITHOUT `--connect`, asserts exit code 0 and the
  readiness banner is in stdout.

### 4. Update `docs/clusters/PHASE4-N-E/CE-N-E-6_PROCEDURE.md`

Replace the placeholder "build the session loop mirroring
`live_consensus_session`" step with the concrete command:

```
cargo run -p ade_core_interop --bin live_tx_submission_session -- \
    --connect --network preprod --max-seconds 600 --max-frames 1000
```

and clarify the honest-scope framing (the binary validates
handshake + mux + codec + state machine on live wire; bulk tx
delivery is the CE-NODE-N2C-LTX deferral).

### 5. Update `docs/clusters/PHASE4-N-E/cluster.md`

- Update CE-N-E-6 wording to reflect the realistic outbound-client
  scope.
- Mark CE-N-E-7 as **DEFERRED** to the future node-binary cluster
  as cross-cluster obligation `CE-NODE-N2C-LTX`.
- Add a "Cluster status" section using the user's
  authoritative wording: "Tier-1 authority closed; live N2N
  evidence binary complete; live N2C evidence deferred to
  node-binary cluster as CE-NODE-N2C-LTX."
- Add a "Deferred / cross-cluster obligation" section documenting
  the requirement on the future cluster.
- TCB Color Map: list the new RED binary + the `[ignore]` gate.

### 6. Update `docs/planning/phase4-n-e-tier1-cluster-slice-plan.md`

Add the N-E-S6 row to the slice table; add a "Deferred from N-E"
note recording the CE-N-E-7 split into `CE-NODE-N2C-LTX`.

### 7. Update `docs/clusters/PHASE4-N-E/CE-N-E-7_PROCEDURE.md`

Add a banner at the top recording the deferral; keep the procedure
content intact as the canonical spec for the future closure.

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build -p ade_core_interop --bin live_tx_submission_session` green.
- **AC-2** — `./target/debug/live_tx_submission_session` (no args) prints the readiness banner and exits 0.
- **AC-3** — `cargo test -p ade_core_interop --test live_tx_submission_session -- --ignored` passes (the `#[ignore]` gate; runs the binary, no network).
- **AC-4** — `cargo test -p ade_core_interop` (default) unchanged (all S4 + S5 tests green; the new `#[ignore]` test does not run by default).
- **AC-5** — `cargo test -p ade_ledger` and `cargo test -p ade_testkit` unchanged.
- **AC-6** — All N-E CI gates still PASS (`ci_check_mempool_ingress_closure.sh`,
  `ci_check_mempool_ingress_replay.sh`,
  `ci_check_constitution_coverage.sh`).

---

## Hard Prohibitions

- **No N2C UDS server scaffolding.** That work is owned by the
  future node-binary cluster as `CE-NODE-N2C-LTX`.
- **No registry edits.** S6 ships a binary + a procedure; the
  invariants it evidences are already in DC-MEM-01 / DC-MEM-03 /
  DC-MEM-04.
- **No new closed enums or BLUE chokepoints.** The binary is a
  consumer of existing surfaces.
- **No claim of CE-N-E-7 closure in the cluster docs.** The cluster
  status MUST read "live N2C evidence deferred to node-binary
  cluster as CE-NODE-N2C-LTX."
- **No async, RNG, wall-clock in any BLUE crate touched by this
  slice.** The binary is RED; tokio + SystemTime are sanctioned only
  in `ade_core_interop` (already RED).

---

## Explicit Non-Goals

- Responder-direction tx-submission2 (we'd ask the peer; not how
  the relay protocol normally works on outbound connections).
- Mempool eviction / shedding (Tier-5).
- N2C UDS server (`CE-NODE-N2C-LTX`).
- Outbound tx propagation (separate cluster).
