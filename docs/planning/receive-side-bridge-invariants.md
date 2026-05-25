# Receive-side header→body bridge — Invariants sketch

**Concept.** When peers send us blocks via the N2N client surface
(chain-sync follow-the-tip + block-fetch download), do we ingest them
through one closed validator authority — extending the ledger +
ChainDb in lockstep — or is the path scattered? At HEAD `2adfb45`, no
module in the workspace consumes `ForkChoiceSignal` or
`BatchDeliveryEvent` outside their N-A definition crates. The
receive-side bridge is greenfield.

**Mirror to N-G's send side.**

- N-G shipped: `AcceptedBlock` → `ServedChainSnapshot` → `ServerReply`
  → wire.
- This cluster ships: wire → peer events → **`block_validity` (B1)**
  → `AdmittedBlock` → `ChainDb::put_block` + ledger extension.

`AcceptedBlock` is the producer-side broadcast token; `AdmittedBlock`
is the receive-side admission token. The two are deliberately
**distinct types** (same shape, different gates) so accidental
cross-use is mechanically impossible. Per the strengthened ¬P-6 below,
no receive code path may touch producer state.

**Status.** Invariants sketch only. Cluster plan + slice docs follow
per the IDD workflow. Not yet implementation-bound. One blocker
flagged in §7 (ledger rollback authority) must be surveyed before
cluster-plan.

## Scope decisions (resolved before this sketch)

1. **Lazy authoritative validation.** `RollForward(header_bytes, tip)`
   is **announcement evidence only**. It MUST NOT mutate
   `LedgerState`, `PraosChainDepState`, or `ChainDb`. Full
   authoritative admission happens only when the body arrives via
   `BlockDelivered { block_bytes }` and `block_validity` (B1)
   returns `BlockValidityVerdict::Valid`. This is **permitted
   internal divergence** from cardano-node's eager-ish header
   pipeline — Cardano compatibility requires the same
   accepted/rejected block behavior, not the same staging
   architecture. Eager header validation as a fetch-DoS defense is
   **operational/resource hardening** (not a fifth tier), addable
   later without touching this cluster's authoritative invariants.
2. **`AdmittedBlock` is a new, distinct type from `AcceptedBlock`.**
   Same opaque-bytes shape; different private constructor — only
   reachable from a `block_validity::Valid` branch. The two gates
   (producer broadcast / receive admission) are symmetric but not
   identical; separate tokens prevent accidental cross-use and keep
   ¬P-6 mechanical.
3. **`block_validity` is the only admission authority.** No parallel
   "lite validator" or "header-only fast path" admits blocks to
   ChainDb. Receive-side analog of `CN-CONS-07`.
4. **RollBackward out of scope (Path A).** Survey at 2026-05-25 found
   that `ChainDb::rollback_to_slot` + `ade_core::consensus::rollback::
   apply_rollback` exist, but the caller-driven materialization (the
   rolled-back `LedgerState` + `PraosChainDepState`) does NOT — there
   is no `encode_ledger_state` / `decode_ledger_state` pair and no
   snapshot+replay-forward driver. The receive cluster ships
   **admit-only**: `RollBackward` returns a structured
   `ReceiveError::RollbackOutOfScope { target_point }`. The
   orchestrator halts the peer and logs the scope edge. A follow-on
   "ledger snapshot + replay-forward rollback" cluster (3–4 slices)
   closes the rollback half. This is the IDD complete-work-only
   discipline: two fully-closed clusters over one half-closed cluster.
   Bounty acceptance test #1 ("sync from Mithril/genesis to tip") is
   not blocked — well-behaved peers do not issue `RollBackward`
   against a follower starting from genesis.
5. **N2N-receive only.** N2C local-chain-sync receive is out of
   scope; different consumer model (operator client driving us
   instead of us driving an upstream peer).
6. **Single-source-of-truth follow.** Bridge assumes one trusted
   upstream peer at a time (orchestrator selects). Praos longest-
   chain fork choice across multiple competing peers is a future
   cluster; in scope here: "consume the followed peer's stream
   coherently."

## 1. What must always be true

- **I-1 — Single admission authority.** Every block that lands in
  ChainDb via the receive path passed `block_validity` with
  `BlockValidityVerdict::Valid`. No bypass, no header-only fast
  path, no "trusted prefix" mode. **Invalid verdicts leave receive
  state unchanged and halt the peer pipeline with a structured
  error; no silent skip, no partial application.** *(Receive-side
  analog of `CN-CONS-07`; strengthens `DC-CONS-13`; folds I-7
  fail-closed.)*
- **I-2 — Header-body sourcing coherence.** When a body arrives via
  `BlockDelivered { block_bytes }`, its decoded header bytes equal
  the `header_bytes` cached from the most recent `RollForward` at
  the same `(slot, hash)`. A peer cannot swap headers between
  announcement and body delivery. *(Receive-side analog of
  `DC-CONS-18`.)*
- **I-3 — ChainDb-ledger-chain_dep lockstep.** A successful
  admission updates `ChainDb` AND `LedgerState` AND
  `PraosChainDepState` as one structural transition. A successful
  rollback rolls back all three to the same slot. No path leaves
  them out of sync.
- **I-4 — RollBackward is a structured scope boundary (Path A).** A
  `RollBackward(point)` returns
  `Err(ReceiveError::RollbackOutOfScope { target_point })`
  deterministically. Receive state is unchanged. Never a panic, never
  a silent skip, never a partial rollback. The orchestrator halts
  the peer pipeline and logs the boundary. Full rollback authority
  (including the missing ledger snapshot + replay-forward driver)
  is the follow-on rollback cluster's deliverable.
- **I-5 — Deterministic event ordering.** Bridge consumes events
  in arrival order; given the same canonical input sequence + the
  same prior `(ledger, chain_dep, chaindb)` state, output state is
  byte-identical across replays.
- **I-6 — No header acceptance without body.** A header from
  `RollForward` does NOT mutate `PraosChainDepState`'s
  `op_cert_counters` or `last_slot` until the body arrives and the
  full block is valid. "Header is announcement; authority is the
  full block."
- **I-7 (folded into I-1).** Fail-closed on validity rejection —
  see I-1's strengthened wording. Captured by the same registry
  entry rather than a separate rule per user direction.

## 2. What must never be possible

- **¬P-1.** A block landing in ChainDb whose bytes did not pass
  `block_validity` Valid. *(Includes: peer-supplied bytes that
  bypass validity; bytes constructed from "trusted" prefixes;
  reconstructed bytes that differ from the wire bytes.)*
- **¬P-2.** Admission of a body whose decoded header differs from
  the `RollForward`-cached header for the same `(slot, hash)`.
- **¬P-3.** Mutation of `PraosChainDepState` from a `RollForward`
  alone, before the body arrives.
- **¬P-4.** ChainDb advancing without a corresponding ledger
  advance; ledger advancing without a corresponding ChainDb
  advance. Rollback applied to one but not the other.
- **¬P-5.** A `RollBackward(point)` returning `Ok` while leaving
  state at a higher slot than `point.slot`. No silent partial
  rollback. (Path A: `RollBackward` always returns
  `Err(RollbackOutOfScope)`; the Ok branch is unreachable until the
  rollback cluster lands.)
- **¬P-6.** The bridge mutating producer-side state
  (`BroadcastQueue`, `ServedChainSnapshot`, signing keys, opcert
  state). Receive side has no signing keys and no path to the
  producer pipeline. Mechanically: `AdmittedBlock` ≠
  `AcceptedBlock` at the type level (distinct private
  constructors), so producer entry points cannot accept receive-
  produced tokens and vice versa.
- **¬P-7.** Two simultaneous `BlockDelivered` events for distinct
  bodies at the same `(slot, hash)` both succeeding. Idempotent on
  byte-identity; structured conflict error on byte-divergence
  (same shape as `ServedChainAdmitError::KeyByteConflict` from
  N-G S2).
- **¬P-8.** A bridge transition that touches the file system, the
  wall clock, or the network. Those are RED orchestrator concerns;
  the bridge transition is pure BLUE.

## 3. What must remain identical across executions

The **receive transcript reduction.** Given canonical inputs
`(initial_ledger, initial_chain_dep, initial_chaindb,
event_sequence)` where `event_sequence` is the merged ordered list
of `ForkChoiceSignal` + `BatchDeliveryEvent` values, the bridge's
output state `(ledger', chain_dep', chaindb')` MUST be byte-identical
across two replays.

The bridge reducer is pure; the RED orchestrator drives I/O. Arrival
order is canonical input — the orchestrator stamps it before the
reducer sees the events, and it then becomes frozen replay input.

## 4. What must be replay-equivalent

For a synthetic fixture corpus of
`(initial_state, event_sequence) → expected_state` triples:

- Replaying the corpus twice MUST produce byte-identical
  `(ledger', chain_dep', chaindb')` fingerprints.
- Replaying with peer message bytes captured from a real
  cardano-node follow MUST produce an `expected_state` whose
  ChainDb tip matches cardano-node's tip at the same point.
- Corpus carries peer-supplied byte sequences (signed artifacts
  via headers + bodies); never private keys.

## 5. State transitions in scope

```text
fn receive_apply(
    state: ReceiveState,                   // bundles ledger + chain_dep + chaindb-ref + pending headers
    event: ReceiveEvent,                   // ForkChoiceSignal | BatchDeliveryEvent (lifted)
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<(ReceiveState, ReceiveEffect), ReceiveError>
```

Where:
- `ReceiveEvent` is a closed sum lifting the two N-A signal/event
  taxonomies into one canonical event stream.
- `ReceiveEffect` reports what happened (`Admitted{slot,hash}`,
  `Cached{slot,hash}`, `RolledBack{to_slot}`,
  `NoOp{reason_tag}`).
- `ReceiveError` is closed: `HeaderBodyMismatch`,
  `Validity(BlockValidityError)`,
  `RollbackOutOfScope { target_point }` (Path A scope boundary),
  `ChainDb(ChainDbError)`.
- `ChainDbWrite` may be a narrow trait if the bridge stays pure of
  `ChainDb`'s I/O (open question §7).

```text
fn receive_apply_sequence(
    state: ReceiveState,
    events: &[ReceiveEvent],
    era_schedule: &EraSchedule,
    ledger_view: &dyn LedgerView,
) -> Result<(ReceiveState, Vec<ReceiveEffect>), ReceiveError>
```

The fold of `receive_apply` — the replay surface.

## 6. TCB color hypothesis

- **BLUE (new):**
  - `ReceiveEvent` / `ReceiveEffect` / `ReceiveError` closed sums.
  - `PendingHeaderCache` — `BTreeMap<(SlotNo, Hash32), Vec<u8>>`,
    canonical iteration.
  - `AdmittedBlock` token — private constructor reachable only from
    a `block_validity::Valid` branch. Consumed by ChainDb-write
    wrapper.
  - `receive_apply` reducer — composes `block_validity` (B1) +
    `header_input::decode_block` for cross-check; calls a narrow
    `ChainDbWrite` trait.
- **GREEN (new):**
  - Adapter `events_to_state` translating
    `ForkChoiceSignal` + `BatchDeliveryEvent` per-protocol streams
    into the unified `ReceiveEvent` stream.
  - Replay test scaffolding.
- **RED (new):**
  - Per-peer receive orchestrator: drives the N2N client side
    (chain-sync client + block-fetch client), calls the reducer
    once per event, persists ChainDb writes through the real
    backing store.
  - Live-evidence binary: connects to a real cardano-node, follows
    the tip for N blocks, captures
    `CE-N-H-LIVE_<date>.log` of receive-side tip-following with
    zero validity rejections and zero ChainDb-tip drift.
- **Open color question:** `ChainDbWrite` trait — BLUE (closed
  shape) or GREEN? Trait shape stays BLUE; impl is RED (touches
  disk). Resolved at cluster-plan when the bridge's call boundary
  is wired.

## 7. Open questions (must resolve before cluster-plan)

1. **Ledger rollback authority — RESOLVED (Path A).** Survey at
   2026-05-25 confirmed `LedgerState` has no rollback path and no
   round-trippable encode/decode. The receive cluster ships
   admit-only; `RollBackward` is a structured scope-boundary error
   per scope decision #4. A follow-on rollback cluster (3–4 slices:
   `encode_ledger_state` + `decode_ledger_state` + snapshot cadence
   + replay-forward materializer) closes DC-CONS-20's rollback
   half. **DC-CONS-20 in this cluster flips only its admit-side
   half to `enforced`; the rollback-side half is recorded as a
   cross-cluster open obligation.**
2. **`PendingHeaderCache` size + eviction.** Narrow scope: cache
   the most recent K headers (K=1 for follow-the-tip; K=N for
   catch-up). Is K a structural BLUE parameter (gets a registry
   entry) or an operator-tunable surface (RED config)? Lean BLUE
   structural for replay-determinism; the orchestrator passes K
   as canonical input.
3. **`AdmittedBlock` private-constructor mechanics.** Per N-C's
   `AcceptedBlock` precedent, the type lives in `ade_ledger`
   alongside `block_validity` so the constructor is module-private
   to `block_validity::transition` (or a narrow helper). The
   ChainDb write wrapper takes `AdmittedBlock` by value and calls
   `ChainDb::put_block` — that wrapper is the only consumer.
4. **Multi-peer / fork choice carve-out.** Explicitly out of scope
   per scope decision #6. Will get its own cluster (Praos
   consensus-follower) when single-source follow is proven.
5. **Replay corpus shape.** Synthetic single-peer streams for the
   reducer; captured single-peer streams from a real cardano-node
   follow for the live-evidence half. No multi-peer corpus here.
6. **Live-evidence binary** — separate binary
   `live_block_fetch_follow_session` (or similar), or extend an
   existing `live_consensus_session`? Lean new binary for the same
   reason N-C / N-G shipped separate binaries: per-cluster evidence
   files.

## 8. Acceptance evidence shape

Mechanical CEs prove the BLUE reducer + GREEN adapter + replay
corpus (closes I-1 through I-6, all ¬P-* except ¬P-6 which is a
build-time/dep-boundary check).

Bounty acceptance test #1 ("sync from Mithril snapshot or genesis
to tip") sits adjacent: this cluster's live evidence (follow N
blocks, ChainDb tip equals peer tip at every step) is the
structural pre-condition for the full sync-to-tip claim. Sync-to-
tip itself spans this cluster + Mithril snapshot loading +
extended-window replay — multi-cluster effort.

Live-evidence half follows the `enforced + open_obligation`
pattern (N-C `CN-CONS-06`, N-G `RO-LIVE-01`) if a peer isn't
available at cluster close.

---

## Proposed registry entries (6, all `status="declared"`)

```toml
[[rules]]
id = "CN-CONS-08"
tier = "release"
statement = """
Receive-side single admission authority: every block that lands in
ChainDb via the receive path passed block_validity with
BlockValidityVerdict::Valid. No bypass, no header-only fast path, no
trusted-prefix mode. Invalid verdicts leave receive state unchanged
and halt the peer pipeline with a structured error; no silent skip,
no partial application. Receive-side analog of CN-CONS-07 (broadcast
gate).
"""
source = "docs/planning/receive-side-bridge-invariants.md §1 (I-1, folds I-7)"
cross_ref = ["CN-CONS-07", "DC-CONS-13", "DC-CONS-19", "DC-CONS-20"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
attack_rationale = "A peer that can sneak unvalidated bytes into our ChainDb pollutes the ledger downstream and forces divergence from the rest of the network."
evidence_notes = "Enforcement is type-level: AdmittedBlock token has a private constructor reachable only from a block_validity::Valid branch; the ChainDb-write wrapper takes AdmittedBlock by value. AdmittedBlock is deliberately a distinct type from AcceptedBlock to keep producer/receive gates non-interfering."

[[rules]]
id = "DC-CONS-19"
tier = "derived"
statement = """
Receive-side header-body sourcing coherence: when BlockDelivered
{block_bytes} arrives at the receive bridge, the decoded header bytes
of block_bytes equal the header_bytes cached from the most recent
RollForward at the same (slot, hash). A peer cannot switch headers
between announcement and body delivery.
"""
source = "docs/planning/receive-side-bridge-invariants.md §1 (I-2)"
cross_ref = ["DC-CONS-18", "CN-CONS-08"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "DC-CONS-20"
tier = "derived"
statement = """
ChainDb-ledger-chain_dep lockstep: a successful receive-side
admission updates ChainDb, LedgerState, and PraosChainDepState as one
structural transition. A successful RollBackward rolls back all three
to the same slot. No path leaves them out of sync; no partial
admission; no partial rollback.
"""
source = "docs/planning/receive-side-bridge-invariants.md §1 (I-3, I-4)"
cross_ref = ["CN-CONS-08", "DC-CONS-13"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "DC-PROTO-09"
tier = "derived"
statement = """
Receive-side transcript determinism: given canonical inputs
(initial_ledger, initial_chain_dep, initial_chaindb, event_sequence),
the bridge reducer's output state (ledger', chain_dep', chaindb') is
byte-identical across replays. The reducer is a pure, total transition.
"""
source = "docs/planning/receive-side-bridge-invariants.md §3, §4"
cross_ref = ["T-DET-01", "DC-PROTO-07"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "CN-PROTO-07"
tier = "derived"
statement = """
Receive-side agency closure: the receive bridge consumes only
peer-originated ForkChoiceSignal and BatchDeliveryEvent values valid
for the client-role N2N receive surface. Constructing or admitting
locally-originated / client-output events into the receive reducer is
unrepresentable in the public API.
"""
source = "docs/planning/receive-side-bridge-invariants.md §1 (closure)"
cross_ref = ["CN-PROTO-06", "DC-PROTO-06"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"

[[rules]]
id = "RO-LIVE-02"
tier = "release"
statement = """
A cardano-node peer's RollForward + BlockDelivered stream, consumed
by the receive bridge, produces a ChainDb tip equal to the peer's
announced tip at every step over a captured follow window. Live
evidence captured against a private cardano-node peer; underlying
invariants are CN-CONS-08, DC-CONS-19, DC-CONS-20, DC-PROTO-09, and
the existing block_validity (B1) authority.
"""
source = "docs/planning/receive-side-bridge-invariants.md §8"
cross_ref = ["RO-LIVE-01", "CN-CONS-08", "DC-CONS-19", "DC-CONS-20", "DC-PROTO-09"]
code_locus = ""
tests = []
ci_script = ""
status = "declared"
```

## Existing rules this cluster will eventually strengthen

(`strengthened_in` appends recorded at `/cluster-doc` once cluster
ID is assigned.)

- `T-DET-01` — new authoritative-deterministic surface (receive-
  transcript reduction).
- `T-ENC-01` — peer-supplied wire bytes flow into the authoritative
  ChainDb verbatim (no re-encoding on the receive path either).
- `DC-CONS-13` — forge purity strengthened by symmetric receive
  closure (admission = block_validity result; never a parallel
  path).
- `CN-CONS-07` — broadcast gate's mirror is the receive-side
  admission gate (`CN-CONS-08`); the same closed-authority
  doctrine is now enforced on both sides.
- `DC-PROTO-06` — version threaded through receive-side reducer.

## Related

- [[project-phase4-n-g-handoff]] — the send-side analog this cluster
  mirrors.
- [[project-bounty-requirements]] — block-validity agreement
  (#1 priority); this is the receive half.
- [[feedback-bounded-smoke-slices]] — live-evidence binary is
  bounded; the deliverable is the BLUE reducer + replay corpus.
- [[feedback-fail-closed-validation]] — folded into I-1.
- [[feedback-diverge-on-internal-surfaces]] — `PendingHeaderCache` +
  `ReceiveState` shape is permitted internal divergence.
