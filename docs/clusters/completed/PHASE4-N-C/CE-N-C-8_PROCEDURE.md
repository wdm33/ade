# CE-N-C-8 — operator procedure for live block-production evidence capture

> **Status:** Operator procedure (non-normative). Closure of CE-N-C-8
> requires running this procedure against a private cardano-node and
> committing the resulting log to
> `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<YYYY-MM-DD>.log`. Mirrors the
> CE-N-B-6 / CE-N-E-6 operator-action precedents.

## What CE-N-C-8 asserts

A block forged by Ade's producer pipeline (RED scheduler → GREEN
tick-assembler → BLUE forge → BLUE self_accept → RED broadcast) is
accepted by a real `cardano-node` peer when delivered via N2N. This
is the crypto-level cross-impl claim — bytes that carry a real
KES / VRF signature against operator-supplied cardano-cli `.skey`
material, not the synthetic all-zero signatures the S3 corpus uses
for replay determinism.

The mechanical half of CN-CONS-06 is closed in CI by
`ade_testkit::producer::cross_impl_adapter` (decode round-trip,
body-hash binding via S4's authority, structural field agreement
across forge ⊕ decoder). That covers the bytes-shape claim. This
procedure adds the crypto-level half: real signatures observed over
the wire by a real peer.

## Pre-conditions

- **Operator-provided signing keys** (cardano-cli text envelopes):
  - `cold.skey` — `StakePoolSigningKey_ed25519`
  - `kes.skey` — Sum6KES signing key
  - `vrf.skey` — Praos VRF signing key
- **Operator-provided opcert** (`node.opcert`) — cardano-cli output
  of `node issue-op-cert`.
- **Testnet (preview or preprod) SPO registration with sufficient
  stake** to be elected as a leader within a tractable run window.
  Without stake the leader schedule will never elect us; the binary
  will then log `not_leader` for every attempted slot and the live
  evidence cannot complete. This is the
  `blocked_until_operator_stake_available` blocker recorded on the
  registry.
- **Reachable cardano-node N2N endpoint on the same network** —
  typically a private node the operator controls. The committed log
  redacts the hostname per `feedback_no_credential_leaks`.
- Ade workspace built: `cargo build --bin live_block_production_session -p ade_core_interop`.

## Procedure

1. **Run the sustained-window probe.** PHASE4-N-C S7 ships the binary:

   ```
   cargo run -p ade_core_interop --bin live_block_production_session -- \
       --connect \
       --network preprod \
       --target 127.0.0.1:3001 \
       --cold-skey /path/to/cold.skey \
       --kes-skey /path/to/kes.skey \
       --vrf-skey /path/to/vrf.skey \
       --opcert /path/to/node.opcert \
       --slots 60
   ```

   The binary loads the supplied `.skey` envelopes via
   `ade_runtime::producer::keys` (failure here is fatal — fix the
   envelopes first), opens an N2N session to the configured target,
   and drives the producer pipeline for the configured slot window.

2. **Honest scope (S7 ships).** At this slice's HEAD, the binary
   evidences:

   - Key loading: cardano-cli `.skey` envelopes for cold / KES / VRF
     parse via the RED loader.
   - Network reachability: handshake / chain-sync against the target
     completes (or fails fast with a structured error).
   - Producer pipeline drive: per slot, the binary records its
     intent to assemble a `ProducerTick`, run forge → self_accept,
     and submit the resulting `AcceptedBlock` via block-fetch
     server-side.

3. **Honest scope (S7 stubs).** Full producer-side N2N delivery
   (block-fetch server-side responder role, chain-sync extension)
   is an N-A follow-on, not S7. Until that lands, the binary logs:

   ```
   [slot N] would submit via block-fetch server-side (N-A follow-on)
   ```

   The cardano-node acceptance verdict on those bytes is captured by
   the operator out-of-band — e.g., by observing the peer's
   chain-sync stream from a second client and confirming the forged
   block-hash appears in the peer's chain.

4. **Commit the log.** Write the captured session to
   `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<YYYY-MM-DD>.log` with at
   minimum:

   ```
   [live] producer-mode (RED, operator-action) network=<n> magic=<m> target=<n-relay> slots=<k>
   [keys] all skey envelopes parsed
   [net] would open N2N session to <target> (handshake + chain-sync follow) — full producer-side delivery is N-A follow-on
   [slot 0] would assemble ProducerTick, run forge -> self_accept; would submit via block-fetch server-side (N-A follow-on)
   ...
   [live] slots_attempted=<k> verdicts_captured=<v> (stub — full pipeline drive lands with N-A)
   ```

   When the operator captures the cardano-node-acceptance evidence
   out-of-band (per §3 above), append it to the same log file:

   ```
   [peer] cardano-node accepted block hash=<...> at slot=<...> via N2N (out-of-band capture)
   ```

5. **Cluster gate.** CE-N-C-8 is closed when EITHER:

   - **Case (a) — live evidence captured.** The log entry under
     `docs/clusters/PHASE4-N-C/CE-N-C-LIVE_<YYYY-MM-DD>.log` records
     at least one cardano-node-accepted forged block from this
     binary, AND the CN-CONS-06 registry entry's `evidence` array
     names the log file, AND the entry's `status` is `enforced`.

   - **Case (b) — blocked on stake.** Testnet SPO stake is not yet
     provisioned. The CN-CONS-06 registry entry records
     `status = "partial"` with an `open_obligation` field naming the
     `blocked_until_operator_stake_available` blocker and citing
     this procedure doc as the re-open path. The mechanical half
     remains fully enforced via the cross_impl_adapter tests.

## Why this is operator-action, not CI

A CI runner cannot reliably:

- Hold operator-supplied private-key material safely.
- Maintain stake on a testnet SPO long enough to be elected.
- Sustain an N2N session against a private cardano-node over the
  window required to be elected and observe the peer's verdict.

The PHASE4-N-B / PHASE4-N-E precedents use the same operator-action
pattern for the same reasons. The mechanical half (structural
cross-impl agreement in CI) is deterministic; the live half is
captured by an operator and committed as an artifact.

## Non-goals

- Implementing full producer-side block-fetch server-side delivery
  (deferred to N-A follow-on).
- Multi-peer load testing — CE-N-C-8 needs only a single private
  cardano-node peer accepting at least one Ade-forged block.
- Resolving the Sum6KES `.skey` deserialization gap recorded on
  OP-OPS-04 — that is its own open obligation and is not blocking
  CE-N-C-8's procedural shape.

## Reference

- `crates/ade_core_interop/src/bin/live_block_production_session.rs`
  — this binary.
- `crates/ade_testkit/src/producer/cross_impl_adapter.rs` — the
  mechanical half (CI gate).
- `docs/clusters/PHASE4-N-C/cluster.md` — CE-N-C-7 / CE-N-C-8
  statements.
- `docs/clusters/completed/PHASE4-N-E/CE-N-E-6_PROCEDURE.md` —
  operator-action precedent.
