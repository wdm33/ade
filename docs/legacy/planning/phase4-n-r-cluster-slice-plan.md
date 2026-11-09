# PHASE4-N-R — Cluster & slice plan

> **Predecessor invariants:** [`phase4-n-r-invariants.md`](./phase4-n-r-invariants.md).
> **Three sub-clusters:** N-R-A, N-R-B, N-R-C — independently
> mergeable, replay-verifiable, and reviewable.
>
> **Audit of existing surfaces** (Ade already ships most BLUE primitives;
> N-R is genuinely composition):
>
> | Surface | Crate / file | What it gives N-R |
> |---|---|---|
> | `forge_block(&ProducerTick)` | `ade_ledger::producer::forge` | BLUE block construction; leader-check + opcert validate + mempool admit + body-hash recipe + header assembly. N-R-A composes this. |
> | `self_accept(bytes, ledger, chain_dep, schedule, view)` | `ade_ledger::producer::self_accept` | BLUE verdict. N-R-A composes this. |
> | `ServedChainSnapshot` + `served_chain_admit` | `ade_ledger::producer::served_chain` | Pure value-typed snapshot + admit primitive. N-R-B wraps with a RED shared handle. |
> | `drain_and_admit(snap, queue)` | `ade_runtime::producer::broadcast_to_served` | GREEN drain — `BroadcastQueue → ServedChainSnapshot`. N-R-B drives this from the `BroadcastBlock` effect. |
> | `dispatch_chain_sync_frame` / `dispatch_block_fetch_frame` | `ade_runtime::network::n2n_server` | Per-peer reducers (closed `ServerReply` surface). N-R-B routes listener frames into these. |
> | `producer_shell.{vrf_prove,kes_sign_at}` | `ade_runtime::producer::producer_shell` | RED key custody + signing. N-R-A calls these from the real forge handler. |
> | `live_block_production_session.rs` | `ade_core_interop/src/bin/` | Legacy independent path. N-R-C reduces it to a shim. |
>
> **What N-R adds (composition):**
>
> 1. A BLUE `verify_and_evaluate_leader` extraction from
>    `forge_block`'s leader-check section so leader-eligibility
>    is callable on its own (without paying the full
>    block-construction cost just to gate forging).
> 2. The real `RequestForge → forge_block → self_accept →
>    ForgeResult` composition inside
>    `produce_mode::apply_effects_with_forge_handler`.
> 3. A RED `push_atomic` API around the existing
>    `served_chain_admit` (so listener tasks can read a
>    snapshot view without torn updates).
> 4. The per-peer dispatch wiring in
>    `produce_mode::handle_listener_event` (replaces
>    `_ => {}`).
> 5. Real cardano-cli opcert envelope parser (gated on
>    captured fixtures).
> 6. Real Conway genesis closed-contract parser (gated on
>    captured fixtures).
> 7. Legacy binary shim.
> 8. The paired bounty evidence capture.

## §1 Sub-cluster scope summary

### PHASE4-N-R-A — Real forge composition

**Closes:** I1, I2, I3 (strengthens DC-CONS-18), I9, N1, N3, N8
(strengthens CN-PROD-02), N12 (strengthens T-KEY-01), D1, R3.
**Pre-flight proof obligations resolved as A1's deliverable:**
OQ4 (opcert envelope shape + fixtures), OQ7 (Conway genesis
closed contract + fixtures), OQ8 (cardano-node block-fetch
failure semantics for partial-overlap), OQ9
(`dispatch_chain_sync_frame` / `dispatch_block_fetch_frame`
signatures vs the snapshot read API).

### PHASE4-N-R-B — Served snapshot + per-peer dispatch

**Closes:** I4 (push_atomic ordering), I5 (unbroken dispatch
path), N4 (partial-overlap failure), N5 (no silent dispatch),
N11 (no torn snapshot), D6 (push_atomic determinism), R2
(served bytes byte-identity under real load).
**Strengthens:** CN-PROD-01 (per-peer dispatch closure clears
its open_obligation), DC-CONS-17.

### PHASE4-N-R-C — Bounty artifact run path

**Closes:** I6, I7, I8, N6, N7, N10, R4, D4, D5.
**Strengthens (status flip):** `CN-CONS-06` → fully `enforced`
(live half closed); `RO-LIVE-01` → fully `enforced`;
`CN-PROD-02.open_obligation` cleared (opcert parser + genesis
parser); `DC-PROD-02.open_obligation` cleared (operator-pass
replay anchor); `N10`-driven retirement of the legacy binary.

## §2 Slice index

| Sub-cluster | Slice | Purpose | Closes (invariant IDs) |
|---|---|---|---|
| N-R-A | **A1** | Planning + 9 candidate registry entries proposed (`declared`) + the 4 pre-flight proof obligations captured (OQ4, OQ7, OQ8, OQ9). **Hard rule:** A1 is planning + fixtures only; if it bloats, split off `N-R-PREFLIGHT`. | — |
| N-R-A | **A2** | BLUE `verify_and_evaluate_leader` extracted to new module `ade_ledger::consensus::leader_check` (per DQ-A1). Closed contract: `(slot, eta0, stake_distribution, leader_threshold, vrf_vk, vrf_proof/output) -> LeaderCheckVerdict`. Refactor `forge_block` to consume a proven leader artifact, not raw signing authority. Unit tests on the verify + threshold math. | I2, D1, R3, N12 (BLUE side) |
| N-R-A | **A3** | Real forge handler in `produce_mode::apply_effects_with_forge_handler`: build `ProducerTick` from `RequestForge` inputs + `producer_shell.{vrf_prove,kes_sign_at}` outputs → `forge_block` → `self_accept` → `ForgeResult`; replace S5 stub. **Entry gate (DQ-A2):** `forge_block_accepts_empty_mempool` test must pass before any composition wiring; if it fails, halt A3 and revise the `ProducerTick` contract — DO NOT patch around it in `produce_mode`. | I1, I9, N1, N3, N8, I3 (DC-CONS-18 strengthening), N12 (RED side) |
| N-R-A | **A4** | A3 integration tests against a synthetic stake-distribution + Eta0 corpus: non-leader → `ForgeNotLeader`; leader → `ForgeSucceeded` that survives `self_accept`; opcert period out → `ForgeFailed { KesPeriodOutOfRange }`; self-accept failure → `ForgeFailed { SelfAcceptRejected }`. Sub-cluster close. | — (closes loose ends from A2/A3) |
| N-R-B | **B1** | Planning. Record DQ-B1 + DQ-B2 resolutions verbatim in the cluster doc (watch channel + documented trade-offs + `ServedTip { slot, hash }` return type) and propose `CN-SNAPSHOT-01`, `CN-SNAPSHOT-02`, `DC-SNAPSHOT-01` as `declared`. | — |
| N-R-B | **B2** | RED shared-handle `ServedChainHandle` wrapping `tokio::sync::watch::channel<ServedChainSnapshot>` (per DQ-B1). `push_atomic(artifact: ForgedBlockArtifact) -> Result<ServedTip, PushError>` (per DQ-B2; covers the full insertion in one watch send). Wire `BroadcastBlock` effect handler to call it; emit `ProducerLogEvent::BlockServed` from the returned `ServedTip`, NOT from a later snapshot read. Fail-closed shutdown on `PushError`. | I4, N11, D6 |
| N-R-B | **B3** | Per-peer dispatch wiring in `produce_mode::handle_listener_event`: replace `_ => {}` with explicit routing into `dispatch_chain_sync_frame` / `dispatch_block_fetch_frame`; structured failure on unhandled variants | I5, N5 |
| N-R-B | **B4** | Integration tests + sub-cluster close: synthetic dialer fetches a pushed block → byte-identical bytes; partial-overlap `RequestRange` over unknown slots → protocol-defined reply (resolved per OQ8 in A1); concurrent read during `push_atomic` → either pre-push or post-push view, never torn. Flip CN-PROD-01.open_obligation cleared, DC-CONS-17 strengthened_in += "PHASE4-N-R" | N4, R2, sub-cluster close |
| N-R-C | **C1** | Real cardano-cli opcert envelope parser driven by A1's OQ4 fixtures (shape locked by fixtures, NOT by design preference, per DQ-C1): closed envelope-type check + canonical CBOR decode + golden-fixture suite (accepted, malformed type, malformed cborHex, wrong arity); wires into `produce_mode`'s opcert-loading path | I6, N6, D4 |
| N-R-C | **C2** | Conway genesis closed-contract parser per DQ-C2: required fields fail-closed on missing/malformed/wrong-type; extra unknown keys accepted-and-ignored iff inert; **no implicit defaults**, **no stringly fallback**. Golden-fixture suite (accepted, missing-required, malformed-numeric, extra-inert-key, stringly-int-attempt). Wires into `produce_mode`'s genesis-loading path. | I7, N7, D5 |
| N-R-C | **C3** | `live_block_production_session.rs` rewritten as thin shim per DQ-C3: keeps the binary name, prints `"DEPRECATED: live_block_production_session is a shim; invoking produce_mode::run_produce_mode."` as line 1, then **immediately** delegates — no alternate flags, no alternate defaults, no duplicate parser logic. CI gate verifies no independent forge codepath remains outside `produce_mode`. | N10 |
| N-R-C | **C4** | Operator-pass evidence capture per DQ-C4: run `ade_node --mode produce` against docker `cardano-node-preprod`; emit `BlockForged H` + capture peer `BlockAccepted H`; write paired files `producer-evidence-YYYYMMDD-<short_commit>.jsonl` + `peer-acceptance-YYYYMMDD-<short_commit>.log` + `evidence-pair-YYYYMMDD-<short_commit>.toml` (closed manifest schema per §5 DQ-C4). Flip `CN-CONS-06` + `RO-LIVE-01` to fully `enforced`; cluster close + registry strengthenings + grounding-doc refresh. | I8, R4, sub-cluster + cluster close |

## §3 Dependency graph

```
A1  (planning + fixtures + OQ resolutions)
 │
 ├── A2  (verify_and_evaluate_leader extract)
 │    │
 │    └── A3  (real forge handler in produce_mode)
 │         │
 │         └── A4  (forge integration tests + N-R-A close)
 │              │
 │              ├── B1  (shared-handle design)
 │              │    │
 │              │    └── B2  (push_atomic + BroadcastBlock effect)
 │              │         │
 │              │         └── B3  (per-peer dispatch wiring)
 │              │              │
 │              │              └── B4  (N-R-B integration tests + close)
 │              │                   │
 │              │                   ├── C1  (opcert parser)
 │              │                   │
 │              │                   ├── C2  (Conway genesis parser)
 │              │                   │
 │              │                   ├── C3  (legacy binary shim)
 │              │                   │
 │              │                   └── C4  (operator-pass evidence + cluster close)
 │              │
 │              └──── (B and C can also start independently from A4
 │                     since A4 closes the forge composition; B1 needs
 │                     forge handler emitting BroadcastBlock for the
 │                     effect handler to wire, so B1 ⊃ A4.)
 │
 └── (Proof obligation fixtures are PREREQUISITES for C1 + C2
      and for B4's OQ8 reply choice; A1 captures them up-front
      so the later slices are not blocked.)
```

**Hard ordering constraints:**

- A1 ≺ A2, A3, A4 (proof fixtures + OQ resolutions are
  pre-flight).
- A2 ≺ A3 (the real forge handler calls
  `verify_and_evaluate_leader`).
- A3 ≺ A4 (A4 tests A3's composition).
- A4 ≺ B1 (B1's design assumes A4's forge emits real
  `ForgeSucceeded` artifacts whose `BroadcastBlock` effects
  need handling).
- B2 ≺ B3 (B3's dispatch reads `ServedChainView` returned by
  B2's API).
- B3 ≺ B4 (B4 tests B3's dispatch under real load).
- B4 ≺ C1, C2, C3, C4 (the C slices add operator-facing
  surfaces on top of the closed N-R-A + N-R-B composition).
- C1 + C2 ≺ C4 (operator pass needs real parsers).
- C3 ≺ C4 (the operator-pass runbook should reference the
  current binary surface, not the deprecated path).

**Soft ordering (can parallelize):**

- C1 and C2 are independent — both depend on B4 but not on
  each other.
- A4's integration tests can land in parallel with B1's
  design slice if A4 is purely test-only.

## §4 Out of scope (carried as future clusters / smoke)

| Item | Tracked under | Cluster |
|---|---|---|
| Multi-peer concurrent forge load | new CN-* rule TBD | future |
| Multi-listener / multi-port | — | future |
| TLS over N2N (¬P-8 continues) | DC-SESS-05 carry | future |
| Mlocked secret memory | new OP-* rule TBD | future operational |
| **Mempool / TxSubmission2 integration** | new CN-* family TBD | TxSubmission cluster |
| Hot-key KES rotation across periods | OP-OPS-04 follow-on | rotation cluster |
| Multi-relay topology | — | future |
| **Private-testnet two-Haskell-node bounty leg** | `blocked_until_two_node_private_testnet_pass` carried on bounty rule | private-testnet cluster |

Empty-block forging is the explicit N-R scope. The runbook
(N-R-C C4) MUST state this so empty-block evidence is not
misread as closing the broader TxSubmission obligation.

## §5 Design decisions (locked)

The 8 DQs are real design choices and the user has resolved
them. Each sub-cluster's `cluster.md` opens with the
resolution recorded in this section; slice docs treat these as
non-negotiable starting points.

### N-R-A

- **DQ-A1 (locked).** Extract `verify_and_evaluate_leader` to
  its own module at **`ade_ledger::consensus::leader_check`**.
  This means a small refactor of `forge_block`'s call site
  inside `ade_ledger::producer::forge`. Rationale: leader
  eligibility and block construction are two distinct
  authorities; burying eligibility inside the forge pipeline
  blurs the RED/BLUE boundary. Clean contract:
  ```
  RED:   vrf_sign(slot, vrf_sk) -> vrf_proof / vrf_output
  BLUE:  verify_and_evaluate_leader(
             slot, eta0,
             stake_distribution, leader_threshold,
             vrf_vk,
             vrf_proof / vrf_output,
         ) -> LeaderCheckVerdict
  ```
  `forge_block` then consumes a *proven leader artifact*, not
  raw signing authority. **Classification:** derived,
  Cardano/Praos-specific refinement of true determinism and
  the key-custody boundary law.

- **DQ-A2 (locked).** Approve empty `MempoolState` + empty
  `mempool_tx_bytes` vector for empty-block forging, **gated
  on an explicit unit proof**. Add this as an A3 entry
  gate: a `forge_block_accepts_empty_mempool` test that
  builds a `ProducerTick` with `mempool: MempoolState::empty()`
  + `mempool_tx_bytes: vec![]` + valid other fields, then
  asserts:
  - no `MempoolWidthMismatch`;
  - no `MempoolAcceptedMismatch`;
  - forged body `tx_count = 0`;
  - body_hash matches header.body_hash;
  - `self_accept` returns `Accepted`.

  **Discipline:** if this test fails, halt A3 and revise the
  `ProducerTick` contract — DO NOT patch around it inside
  `produce_mode`. A failure here means the producer tick
  contract is under-specified, not that produce mode needs a
  workaround.

### N-R-B

- **DQ-B1 (locked).** Use
  **`tokio::sync::watch::channel<ServedChainSnapshot>`**.
  Rationale: N-R-B serves a *snapshot value*, not an append
  log. Watch preserves the pure-value model — peer tasks
  observe a coherent latest snapshot; skipped intermediate
  snapshots are acceptable because BlockFetch asks against
  the current view at dispatch time, not against every
  historical notification.

  **Documented trade-offs (B1's cluster doc must restate
  verbatim):**

  > Watch channel **guarantees**:
  > - readers see a whole `ServedChainSnapshot` value
  > - no torn push
  > - delayed readers may skip intermediate snapshots
  >
  > Watch channel **does NOT guarantee**:
  > - every peer observes every intermediate producer update
  > - notification history

  This is permitted because block-fetch semantics depend on
  whether the requested block is present in the snapshot read
  at dispatch time. If a peer asks for a block not present,
  the server follows the closed failure semantics resolved by
  the OQ8 pre-flight.

  **Forward-compat note:** if later multi-block serving needs
  historical retention beyond the snapshot's own contents,
  that history belongs *inside* `ServedChainSnapshot`, NOT
  inside the transport handle.

- **DQ-B2 (locked).** `push_atomic` returns a closed
  `ServedTip`:
  ```rust
  pub struct ServedTip {
      pub slot: SlotNo,
      pub hash: BlockHash,
  }

  push_atomic(artifact: ForgedBlockArtifact)
      -> Result<ServedTip, PushError>
  ```
  Rationale: prevents a second observation point. The
  `ProducerLogEvent::BlockServed` event is constructed from
  the returned `ServedTip`, NOT from a later snapshot read
  — log emission is deterministic from the transition result.

### N-R-C

- **DQ-C1 (locked).** Opcert envelope CBOR shape is **locked
  by fixture capture, not design preference.** A1's OQ4
  obligation freezes golden fixtures from a real
  `cardano-cli node issue-op-cert` invocation against
  cardano-node 10.6.2 and 11.0.1 before C1 implementation
  begins. Whether the envelope is a tagged wrapper or a plain
  array is whatever the captured bytes say it is.

- **DQ-C2 (locked).** Closed **required-field exact parsing
  with extra-key tolerance**:

  | Field | Behavior |
  |---|---|
  | `network_magic` | Required; fail-closed on missing / malformed / wrong type |
  | `slot_zero_time_unix_ms` | Required; fail-closed |
  | `slot_length_ms` | Required; fail-closed |
  | `slots_per_kes_period` | Required; fail-closed |
  | `kes_anchor_slot` | Required; fail-closed |
  | `kes_max_period` | Required; fail-closed |
  | Extra keys not in the required set | Accepted-and-ignored for forward compat, *iff* they do not collide with required field names or alter interpretation |

  **Hard rules:**
  - No implicit defaults. No "if missing, assume preprod."
  - No stringly fallback (e.g., `"1"` accepted for an
    integer field → rejected).
  - No semantic weakening through extra-key tolerance —
    extras must be inert.

  **Classification:** required-field exactness is *derived*
  (Cardano genesis closure rule); no-implicit-defaults is a
  *true* invariant (no permissive authority input);
  extra-key tolerance is *permitted internal/operational
  compatibility behavior*, not semantic weakening.

- **DQ-C3 (locked).** **Keep `live_block_production_session`
  as the binary name with a deprecation banner.** The
  invariant is "no independent legacy production path," NOT
  "no old binary name." First line printed:

  ```
  DEPRECATED: live_block_production_session is a shim;
  invoking produce_mode::run_produce_mode.
  ```

  Then **immediately delegate** to
  `produce_mode::run_produce_mode`. No alternate flags. No
  alternate defaults. No duplicate parser logic. The CI gate
  in C3 verifies no second forge codepath exists.

- **DQ-C4 (locked).** Evidence filenames carry **date +
  short commit hash**, not date alone. Date-only matches
  precedent but is weak for audit; commit hash prevents
  ambiguity across multiple captures on the same day or
  after local amendments.

  ```
  docs/clusters/PHASE4-N-R/
    producer-evidence-YYYYMMDD-<short_commit>.jsonl
    peer-acceptance-YYYYMMDD-<short_commit>.log
    evidence-pair-YYYYMMDD-<short_commit>.toml
  ```

  The pair manifest TOML schema:

  ```toml
  ade_commit            = "<full sha>"
  cardano_node_version  = "<e.g., 11.0.1>"
  cardano_cli_version   = "<e.g., 10.6.2>"
  network               = "preprod"
  block_hash            = "<hex>"
  slot                  = <integer>
  opcert_fingerprint    = "<hex>"
  genesis_fingerprint   = "<hex>"
  ade_evidence_file     = "producer-evidence-YYYYMMDD-<short_commit>.jsonl"
  peer_log_file         = "peer-acceptance-YYYYMMDD-<short_commit>.log"
  ```

  Classification: release/evidence discipline — NOT a BLUE
  invariant. The manifest schema is the closed surface;
  individual field semantics are captured at C4 close.

## §5.5 Pre-flight proof obligations (not design choices)

The 4 pre-flight items are **evidence-gathering
prerequisites**, not open questions. They have one right
answer captured before the relevant slice's implementation
begins. They are bundled into A1 (planning + proof-obligation
capture) so the later slices are not blocked — **but A1 must
not bloat into a giant implementation slice.** If A1 grows
beyond capture + write-up, split it into N-R-PREFLIGHT (a
discrete fixture-capture slice) before any of A2/A3/A4 begins.

| ID | What | Captured under | Consumed by |
|---|---|---|---|
| **OQ4** | Opcert envelope bytes from `cardano-cli node issue-op-cert` on cardano-node 10.6.2 + 11.0.1 + CBOR shape + 4 golden fixtures (accepted, malformed cborHex, malformed type, wrong arity) | `crates/ade_runtime/tests/fixtures/opcert/` | C1 (opcert parser) |
| **OQ7** | Conway genesis JSON samples from `cardano-node-preprod` + the closed-contract behavior table (numeric forms, missing fields, extra fields, key ordering, duplicate keys, null vs missing) + golden fixtures | `crates/ade_runtime/tests/fixtures/conway_genesis/` | C2 (genesis parser) |
| **OQ8** | Exact cardano-node block-fetch protocol reply for a `RequestRange` covering unknown / partially-unavailable slots (confirmed against `ouroboros-network` Haskell reference) | A1 slice doc + B4 test corpus | B4 (partial-overlap test) |
| **OQ9** | `dispatch_chain_sync_frame` / `dispatch_block_fetch_frame` signatures + frame/event types — does the dispatch API take an owned `ServedChainSnapshot` or a `&ServedChainView`? | A1 slice doc | B2 + B3 (dispatch wiring) |

## §6 Candidate registry entries (proposed; not yet appended)

Per the user decision (option 2), registry entries are
deferred to each sub-cluster's first slice (the planning
slice). At that point the IDs + statements + families +
status (`declared`) get proposed and confirmed before
appending to `docs/ade-invariant-registry.toml`.

Candidate IDs by sub-cluster:

- **N-R-A** introduces: `CN-FORGE-01`, `CN-FORGE-02`, `DC-FORGE-01`.
- **N-R-B** introduces: `CN-SNAPSHOT-01`, `CN-SNAPSHOT-02`,
  `DC-SNAPSHOT-01`.
- **N-R-C** introduces: `CN-OPCERT-01`, `CN-GENESIS-01`,
  `DC-OPCERT-01`, `DC-GENESIS-01`.

Carry-forward strengthenings (no new IDs; recorded in each
target rule's `strengthened_in` field at sub-cluster close):

- `T-KEY-01` ← N-R-A (N12 BLUE-side; key custody boundary
  exercised under real forge load).
- `DC-CONS-18` ← N-R-A (I3; body-hash binding under real
  load).
- `CN-PROD-01` ← N-R-B (N5; per-peer dispatch closure;
  open_obligation cleared).
- `CN-PROD-02` ← N-R-A + N-R-C (N3, N8 under real load;
  opcert + genesis parsers; open_obligation cleared).
- `DC-CONS-17` ← N-R-B (R2; served bytes byte-identity
  under real load).
- `DC-PROD-02` ← N-R-A + N-R-C (R1 under real forge; R4
  operator-pass replay anchor).
- `CN-CONS-06` ← N-R-C (status flip to fully `enforced`;
  live half closed by paired evidence).
- `RO-LIVE-01` ← N-R-C (status flip to fully `enforced`;
  paired evidence captured).

## §7 Cluster-level exit criteria

PHASE4-N-R as a whole closes when ALL of:

1. All three sub-clusters' `cluster.md`s exist with their own
   exit criteria, and every sub-cluster MAC is green in CI.
2. `cargo test --workspace --lib` clean.
3. `ci/ci_check_producer_coordinator_no_secrets.sh` still
   passes (carry-forward).
4. New CI gates from B (no-torn-snapshot smoke) and C (no
   independent forge codepath outside `produce_mode`) pass.
5. The registry has the 10 new rules declared + flipped to
   `enforced` per their sub-cluster's exit criteria; the
   carry-forward strengthenings are recorded; `CN-CONS-06`
   and `RO-LIVE-01` are fully `enforced`.
6. `docs/clusters/PHASE4-N-R/CE-N-R-C-LIVE_<date>.{jsonl,log}`
   evidence pair exists with at least one matched
   `BlockForged H` ↔ `BlockAccepted H` row.
7. `live_block_production_session.rs` is a documented shim;
   no independent forge codepath remains.
8. Grounding docs (CODEMAP / SEAMS / HEAD_DELTAS /
   TRACEABILITY) refreshed at cluster close.

## §8 References

- Invariants: [`phase4-n-r-invariants.md`](./phase4-n-r-invariants.md).
- Predecessor cluster: [`../clusters/PHASE4-N-Q/cluster.md`](../clusters/PHASE4-N-Q/cluster.md).
- Predecessor operator runbook: [`../active/cn-cons-06-operator-runbook.md`](../active/cn-cons-06-operator-runbook.md).
- Bounty: [[project-bounty-requirements]].
- Doctrine carry-forwards: [[feedback-hard-closure-gates]],
  [[feedback-proof-discipline]] (OQ4 + OQ7 + OQ8 + OQ9 are
  proof obligations resolved in A1, not assumptions);
  [[feedback-shell-must-not-overstate-semantic-truth]] (paired
  evidence — Ade emit + peer-witnessed line);
  [[feedback-fail-closed-validation]] (N4, N6, N7, N11);
  [[feedback-bounded-smoke-slices]] (private-testnet smoke is
  additional evidence, never a substitute).
