# PHASE4-N-M-C — Live operator pass invariants (sketch)

> Status: sketch, post-scoping. Pre-cluster-doc.
> Author: ade methodology, 2026-05-26.
> Doctrine load: [[feedback-evidence-reducers-are-green-not-authority]],
> [[feedback-shell-must-not-overstate-semantic-truth]],
> [[feedback-tx-validity-priority]],
> [[feedback-fail-closed-validation]],
> [[feedback-real-interop-finds-codec-bugs]],
> [[reference-local-preprod-docker-cardano-node]].

---

## §0 — Honest scope + decision record

**Decision:**
PHASE4-N-M-C uses **path (b)**: operator-extracted, closed,
fingerprinted `LiveConsensusInputs` imported from `cardano-cli`
output. The BLUE path consumes only the typed, validated,
canonicalized view; the raw JSON is RED / operational evidence.

**Rejected:**
- **(a) Frozen epoch corpus**: operationally impractical for
  current-tip docker preprod. The local
  `cardano-node-preprod` will not replay back to a frozen
  epoch.
- **(c) Wire-only hash agreement**: useful plumbing but
  insufficient because it does NOT exercise BLUE block-validity
  authority. The bounty's hardest acceptance test is
  tx/block-validity agreement (memory
  [[feedback-tx-validity-priority]]) — tip-following alone is
  not the deliverable.

**C closes:** RO-LIVE-05 only, with the bounded statement
recorded in §4.

**C does NOT close:** full sync, full live consensus, full
chain selection over multi-day windows, block production, or
general mainnet readiness.

### Classification

| Item | Tier | Boundary |
|---|---|---|
| Consensus inputs required for validity | derived | Cardano-specific block-validity dependency |
| JSON importer shape | GREEN/RED boundary | not authority by itself |
| `LiveLedgerView` from imported inputs | GREEN | deterministic construction of BLUE inputs |
| `admit_via_block_validity` | BLUE | the authority actually exercised |
| `AgreementVerdict` | GREEN | evidence reducer only (unchanged from B) |
| Operator extraction procedure | operational / release | evidence-production method |

---

## §1 — Invariants

### I-C1 — Closed `LiveConsensusInputs` importer authority (CN-CONS-IN-01)

Exactly one `pub fn` in
`ade_runtime::consensus_inputs::import_live_consensus_inputs`
converts a `cardano-cli`-shaped JSON bundle into a closed,
typed, validated `LiveConsensusInputsCanonical`. The BLUE
admission path consumes only the canonicalized form. The raw
JSON file is operational evidence; it never enters BLUE.

Required fields on the canonical form:

```rust
LiveConsensusInputsCanonical {
    network_magic: u32,
    genesis_hash: Hash32,
    era: CardanoEra,
    epoch_no: EpochNo,
    epoch_start_slot: SlotNo,
    epoch_end_slot: SlotNo,
    active_slots_coeff: ActiveSlotsCoeff,
    epoch_nonce: Nonce,
    pool_distribution: BTreeMap<Hash28, PoolEntry>,
    pool_vrf_keyhashes: BTreeMap<Hash28, Hash32>,
    protocol_params_hash: Hash32,
    source_cardano_node_version: String,
    source_query_command: String,
    source_tip_hash: Hash32,
    source_tip_slot: SlotNo,
    // Computed:
    fingerprint: Hash32,
}
```

### I-C2 — Closed importer error sum (DC-CONS-IN-01)

`LiveConsensusInputsImportError` is a closed sum:
`Io | Json | BadField{field} | MissingField{field} |
 BadHashHex{field} | BadEpochWindow{epoch_start, epoch_end} |
 BadPoolDistribution{detail} | EraNotSupported{era}`.

No `Option` field gets a runtime default. Missing pool
distribution, nonce, ASC, VRF keyhash, era schedule, or any
hash field is fatal at import. No partial importer fallback.

### I-C3 — Canonical fingerprint of consensus inputs (DC-CONS-IN-02)

`LiveConsensusInputsCanonical::fingerprint` is
Blake2b-256 over a canonical CBOR encoding of every field
declared on the canonical form (in declared order). Same JSON
bytes → same canonical form → same fingerprint, byte-identical
across two import runs.

The fingerprint is the **load-bearing handle** for every
admission JSONL event referencing a block: `BlockAdmitted`,
`AgreementVerdict`, and the `BootstrapComplete` event all
carry `consensus_inputs_fingerprint`.

### I-C4 — `LiveLedgerView` determinism + epoch-window guard (DC-VIEW-01)

A `LiveLedgerView` is constructed deterministically from
`LiveConsensusInputsCanonical`. It implements the existing
`LedgerView` trait by returning data from the canonical form.

Two GREEN guards on every `LedgerView` query:
1. If the queried epoch is **not** `canonical.epoch_no`, return
   `None` (BLUE then fails closed via
   `BlockValidityError::MissingConsensusInput`).
2. If the admit call's block slot is outside
   `[epoch_start_slot, epoch_end_slot]`, the runner intercepts
   **before** calling `admit_via_block_validity` and emits
   `AgreementVerdict::Diverged { kind: cross_epoch_use_forbidden }`
   followed by `AdmissionHalted { reason: CrossEpochUse }`.

### I-C5 — Single wire pump entry per peer (CN-PUMP-01)

Exactly one `pub async fn` in
`ade_runtime::admission::wire_pump::run_admission_wire_pump`
drives the per-peer pump that produces `AdmissionPeerEvent`s.
The pump owns the `MuxTransportHandle` and runs the chain-sync
+ block-fetch state machine. No second pump path. The pump is
the **only** producer on the runner's `peer_events` channel.

### I-C6 — Wire pump emits events, never verdicts (DC-PUMP-01)

`run_admission_wire_pump` is allowed to emit
`AdmissionPeerEvent::{Block, TipUpdate, Disconnected}` only.
It MUST NOT synthesize `AgreementVerdict` values. The verdict
remains the GREEN reducer's sole output
([[feedback-evidence-reducers-are-green-not-authority]]).

### I-C7 — TipUpdate emitted on every chain-sync tip (DC-PUMP-02)

Every chain-sync reply (`IntersectFound`, `IntersectNotFound`,
`RollForward`, `RollBackward`) carries a `Tip`; the pump
forwards each as `AdmissionPeerEvent::TipUpdate` so the
runner's next `verdict::derive` call sees the freshest peer
comparison input.

### I-C8 — Every admission JSONL block-event references the fingerprint (DC-ADMIT-10)

`BlockAdmitted`, `AgreementVerdict`, and `BootstrapComplete`
events emit a `consensus_inputs_fingerprint` field. The
admission JSONL vocabulary stays closed (DC-ADMIT-04
preserved); the field is added to the existing variants, not a
new variant.

This is the load-bearing binding between the operator-supplied
oracle (`LiveConsensusInputs`) and every BLUE-authority claim
the transcript makes.

### I-C9 — Cross-epoch silent use forbidden (DC-ADMIT-11)

If a peer sends a block whose slot is outside
`[epoch_start_slot, epoch_end_slot]`, the runner MUST emit
`AdmissionHalted { reason: CrossEpochUse }` and exit non-zero
**without** calling `admit_via_block_validity`. There is no
silent "skip and continue" path.

### I-C10 — Undecodable peer bytes are Diverged (DC-ADMIT-12)

N-M-B's runner currently treats `ProcessedBlock::Undecodable`
as a clean drain (`AdmissionExitCode::Ok`). C strengthens this:
undecodable peer bytes are **adversarial** by default — the
peer is supposed to send canonical Conway block CBOR. C maps
`Undecodable` to `AgreementVerdict::Diverged` (when a peer tip
exists at the same slot) or `AdmissionHalted { reason:
PeerSentUndecodableBytes }` (when no peer tip exists at that
slot) — never a clean exit, never InputNotFound.

**Note:** this is a B-side strengthening that C ships. The
existing `B4 runner` Undecodable arm must be tightened in C3
or C5 before live pass.

### I-C11 — False-accept rejection across 4 mandatory mutation classes (DC-EVIDENCE-02)

Adversarial corpus must include at least:

| Mutation class | Expected failure surface |
|---|---|
| Body byte flip preserving envelope shape | body hash / tx validity / decode failure |
| Header body-hash mismatch | header-body binding |
| KES / signature corruption | consensus / header verification |
| VRF proof or output tamper | leadership / consensus context |

Each mutation must produce **either**:
- `BlockAdmitted` NOT emitted + `AgreementVerdict::Diverged`
  emitted + exit code 30, OR
- `AdmissionHalted { reason: PeerSentUndecodableBytes }` if
  the corruption breaks decode before admit-attempt.

In **no case** may a mutation produce `BlockAdmitted` (false
accept), `Agreed`, or `InputNotFound`. False-accept is
release-blocking.

### I-C12 — Operator-pass live evidence (DC-EVIDENCE-01)

The live operator pass produces a JSONL transcript carrying at
least:
- one `AdmissionStarted` referencing the
  `consensus_inputs_fingerprint`,
- one `BootstrapComplete` referencing it,
- ≥ N `BlockAdmitted` events (N configurable, default ≥ 1),
- ≥ 1 `AgreementVerdict { kind: "agreed" }`,
- ≥ 0 `AgreementVerdict { kind: "lagging" }` (allowed but
  not success),
- 0 `AgreementVerdict { kind: "diverged" }` (would mean
  divergence vs. live preprod — release-blocking),
- 0 `BlockAdmitted` for any block whose hash differs from a
  block the live peer announces at the same slot.

---

## §2 — Hard prohibitions

| ¬P | Statement |
|---|---|
| **¬P-C1** | **No ambient consensus context.** Every admitted block must reference a `consensus_inputs_fingerprint`. No `LedgerView` instance is constructed without going through `LiveConsensusInputsCanonical`. |
| **¬P-C2** | **No cross-epoch silent use.** Block slot outside `[epoch_start_slot, epoch_end_slot]` fails deterministically (DC-ADMIT-11). |
| **¬P-C3** | **No RED-derived verdicts.** Wire pump may emit `Block`, `TipUpdate`, `Disconnected`. It may not emit `Agreed`, `Diverged`, `Lagging`, or `InputNotFound`. |
| **¬P-C4** | **No partial importer fallback.** Missing pool distribution, nonce, ASC, VRF keyhash, era schedule, or any hash field is fatal at import. |
| **¬P-C5** | **No claim inflation.** C proves live admission for the covered epoch/input window — NOT full live sync, NOT chain selection, NOT block production, NOT mainnet readiness. |
| **¬P-C6** | **No wide-obligation closure.** C closes RO-LIVE-05 only. RO-LIVE-03 (wide), RO-LIVE-04 (wide), RO-GENESIS-REPLAY-01, RO-MITHRIL-IMPORT-01 stay open. |
| **¬P-C7** | **No InputNotFound for adversarial input.** Malformed / corrupted / undecodable peer bytes map to `Diverged` or `PeerSentUndecodableBytes`, never to `InputNotFound`. |
| **¬P-C8** | **No reference-script seed-import fallback.** DC-ADMIT-09 stays enforced (N-M-A fail-fast preserved). |
| **¬P-C9** | **No silent clean-exit on adversarial bytes.** N-M-B's Undecodable→Ok path must be tightened before live pass (DC-ADMIT-12). |
| **¬P-C10** | **No mid-epoch CLI swap.** A running `ade_node --mode admission` may not re-load `LiveConsensusInputs` at runtime; the canonical form is fixed at startup. |

---

## §3 — Slice plan

Following the user's scoping decision: C1 split into C1a + C1b.

| Slice | Scope | New rules |
|---|---|---|
| **C1a** — Importer schema + closed decode | GREEN `ade_runtime::consensus_inputs::{json, importer}`: cardano-cli JSON bytes → `LiveConsensusInputsRaw` → typed-error import. No ledger view construction yet. | CN-CONS-IN-01, DC-CONS-IN-01 |
| **C1b** — Canonicalization + fingerprint | GREEN `ade_runtime::consensus_inputs::canonical`: `LiveConsensusInputsRaw` → `LiveConsensusInputsCanonical` + Blake2b-256 fingerprint over canonical CBOR. | DC-CONS-IN-02 |
| **C2** — `LiveLedgerView` + epoch-window guard | GREEN `ade_runtime::consensus_inputs::view`: `dyn LedgerView` impl backed by canonical form. Epoch-window guard runs **before every admit** (not just at the runner-loop top). | DC-VIEW-01, DC-ADMIT-11, DC-ADMIT-10 (event vocabulary extension) |
| **C3** — RED wire pump | `ade_runtime::admission::wire_pump`: pulls blocks from `MuxTransportHandle` via chain-sync + block-fetch; emits closed `AdmissionPeerEvent` into the runner channel. Tightens N-M-B's Undecodable handling (DC-ADMIT-12). | CN-PUMP-01, DC-PUMP-01, DC-PUMP-02, DC-ADMIT-12 |
| **C4** — Adversarial false-accept corpus | Hermetic loopback fed 4 mandatory mutation classes (body flip / header-body mismatch / KES tamper / VRF tamper). Must pass before C5 live pass. | DC-EVIDENCE-02 |
| **C5** — Live operator pass | Run `ade_node --mode admission --json-seed ... --consensus-inputs-json ...` against local docker `cardano-node-preprod`. Capture JSONL transcript binding `(block_hash, slot, consensus_inputs_fingerprint)`. Verify `BlockAdmitted` + `Agreed` emitted. | DC-EVIDENCE-01 |
| **C6** — Cluster close | Flip 10 N-M-C rules to enforced + apply strengthenings + refresh grounding docs + register RO-LIVE-05 closure with bounded statement (§4) + commit + push. | (closure) |

### Slice ordering rationale

- **C1a before C1b**: importer bugs vs. canonicalization/fingerprint bugs are different failure classes; combining them makes later divergence harder to localize.
- **C2 after C1b**: `LiveLedgerView` depends on the canonical form, not the raw form.
- **C3 after C2**: wire pump uses the view's epoch-window guard; building the pump first would mean re-plumbing later.
- **C4 before C5**: false-accept evidence must exist before any positive live claim. Per [[feedback-fail-closed-validation]]: positive agreement is necessary but insufficient.
- **C5 last operational slice**: depends on every other slice + the live docker peer being up.

---

## §4 — RO-LIVE-05 closure statement (bounded)

> **RO-LIVE-05 closed** for live Conway block admission evidence
> against a local docker `cardano-node-preprod` peer using
> imported, fingerprinted `LiveConsensusInputs`, with adversarial
> false-accept corpus passing fail-closed checks. Evidence
> artifact: the JSONL transcript captured at C5.

**Unsafe closures explicitly NOT made:**
- "Ade fully syncs."
- "Ade implements live consensus."
- "Ade has full preprod compatibility."
- "Ade closes all live N2N evidence."
- "Ade can replace cardano-node in production."

---

## §5 — Doctrine references

- [[feedback-evidence-reducers-are-green-not-authority]] —
  `AgreementVerdict` stays GREEN; wire pump (RED) cannot
  synthesize verdicts (¬P-C3).
- [[feedback-shell-must-not-overstate-semantic-truth]] —
  C's success claim is bounded to the (epoch, fingerprint)
  window; no claim inflation (¬P-C5).
- [[feedback-tx-validity-priority]] — tx/block-validity
  agreement is the bounty deliverable; this is why C
  exercises `admit_via_block_validity` against real bytes.
- [[feedback-fail-closed-validation]] — C4 adversarial
  corpus must complete before C5 live pass.
- [[feedback-real-interop-finds-codec-bugs]] — C5's value is
  that it finds bugs synthetic round-trips miss.
- [[reference-local-preprod-docker-cardano-node]] — the
  canonical live-pass target for C.

---

## §6 — B-side strengthenings C ships

C is the cluster that "uses N-M-B for real" — it makes some
B-side decisions tighter than the hermetic test corpus could
demand:

| Strengthens | C-side fix |
|---|---|
| N-M-B's `ProcessedBlock::Undecodable → AdmissionExitCode::Ok` | Tighten to `Diverged` or `AdmissionHalted::PeerSentUndecodableBytes` (¬P-C9, DC-ADMIT-12). C3 implements; C4 tests. |
| N-M-B's empty `peer_events` channel in B5 dispatch | C3 connects the wire pump as the producer. C5 tests end-to-end. |
| N-M-B's `NoopLedgerView` in admission bootstrap | C2 replaces with `LiveLedgerView`. |
| N-M-B's minimal era schedule (single Conway entry, slot 0) | C2 replaces with the schedule consistent with `LiveConsensusInputs.epoch_start_slot / epoch_end_slot`. |
| N-M-B's `AdmissionLogEvent` block-event payloads | C2 extends `BlockAdmitted`, `AgreementVerdict`, `BootstrapComplete` with `consensus_inputs_fingerprint` (DC-ADMIT-10). Vocabulary stays closed. |

These are additive strengthenings, NOT rewrites. The closed
sums + sole authorities established in B carry forward
unchanged.

---

## §7 — New rules to register (declared)

| ID | Tier | One-line statement |
|---|---|---|
| **CN-CONS-IN-01** | release | Sole `import_live_consensus_inputs` authority converting cardano-cli JSON → `LiveConsensusInputsCanonical`. |
| **DC-CONS-IN-01** | derived | Closed importer error sum; no defaulting on any required field. |
| **DC-CONS-IN-02** | derived | Canonical fingerprint over Blake2b-256 of canonical CBOR encoding; deterministic; load-bearing handle. |
| **DC-VIEW-01** | derived | `LiveLedgerView` returns `None` outside `canonical.epoch_no`; epoch-window guard runs before every admit. |
| **CN-PUMP-01** | release | Sole `run_admission_wire_pump` per-peer entry; sole producer on the runner's peer_events channel. |
| **DC-PUMP-01** | derived | Wire pump emits `AdmissionPeerEvent::{Block, TipUpdate, Disconnected}` only; no verdicts. |
| **DC-PUMP-02** | derived | TipUpdate emitted on every chain-sync reply carrying a `Tip`. |
| **DC-ADMIT-10** | derived | Every admission JSONL block-event carries `consensus_inputs_fingerprint`; vocabulary stays closed. |
| **DC-ADMIT-11** | derived | Cross-epoch silent use forbidden; slot outside `[epoch_start_slot, epoch_end_slot]` is fatal. |
| **DC-ADMIT-12** | derived | Undecodable peer bytes → `Diverged` or `PeerSentUndecodableBytes`; never `InputNotFound`, never silent clean exit. |
| **DC-EVIDENCE-02** | derived | Adversarial corpus rejects across 4 mandatory mutation classes; false-accept release-blocking. |
| **DC-EVIDENCE-01** | derived | Operator-pass JSONL transcript binds `(block_hash, slot, consensus_inputs_fingerprint)`; emits ≥ 1 `BlockAdmitted` + ≥ 1 `Agreed`. |

**Strengthenings to existing rules (applied at C6):**
- CN-ADMIT-01 (wire pump connected; per-admit fingerprint binding)
- CN-CONS-08 (exercised against real-peer Conway bytes)
- DC-ADMIT-03 (Diverged also covers undecodable peer bytes)
- DC-ADMIT-04 (event vocabulary extended; closed-sum discipline preserved)
- DC-ADMIT-06 (verdict reducer purity exercised against real inputs)
- T-DET-01 (live admission preserves replay-equivalence with fingerprint as canonical input)

**RO-LIVE-05** → enforced at C6 with the bounded statement from §4.

---

## §8 — What's NOT in C (explicit non-goals)

- Block production (separate; future cluster).
- Multi-epoch admission (would need consensus-inputs refresh + epoch-boundary handling).
- Chain selection across forks (single-peer single-chain).
- Multi-day live runs (operator pass is bounded smoke).
- ChainDb integration of admitted blocks (deferred; C tests in-memory WAL identity; ChainDb persistence is a future strengthening).
- Mithril / cardano-node Mithril snapshot import (RO-MITHRIL-IMPORT-01 still open).
- Reference-script seed-import support (A1.1 still required for seeds containing refscript outputs; DC-ADMIT-09 preserved).
- Genesis → P self-replay (RO-GENESIS-REPLAY-01 still open with `blocked_until_genesis_replay_cluster`).
