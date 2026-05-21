# Invariants — ENACTMENT-COMMITTEE-WRITEBACK

> IDD Part I sketch (non-normative). Frames the cluster that wires the
> committee-changing governance enactment logic that
> `ENACTMENT-COMMITTEE-FIDELITY` deliberately deferred ("Actually implementing
> `UpdateCommittee` enactment … a separate governance-enactment cluster").
> Strengthens **DC-EPOCH-01** (enactment atomicity) and **DC-LEDGER-10**
> (credential discriminant fidelity). Registry/specs win on any conflict.

## What must always be true

- **I-1 (committee write-back totality).** A ratified committee-changing
  governance action — `NoConfidence` or `UpdateCommittee` — deterministically
  rewrites the next-epoch `ConwayGovState.committee` (and, for `UpdateCommittee`,
  `committee_quorum`). No ratified committee change is observed-and-dropped: the
  current apply site (`rules.rs`) clones the committee unchanged, which silently
  discards a ratified `NoConfidence`/`UpdateCommittee`. After this cluster the
  committee at epoch *e+1* is a deterministic function of (committee at *e*) and
  the ratified committee actions enacted at the boundary.

- **I-2 (enactment priority preserved).** Committee write-back happens in the
  existing enactment priority order (`HardForkInitiation` < `UpdateCommittee /
  NoConfidence` < …), and within the class in `GovActionId` order. Wiring the
  effect must not change ratification or ordering.

- **I-3 (discriminant survives write-back).** The members removed/added by
  `UpdateCommittee` enter the committee map as discriminated `StakeCredential`
  (cold credential), never tag-erased `Hash28`. A key-hash member and a
  script-hash member of equal 28 bytes remain distinct committee entries through
  enactment. (This is the *active* use of the dormant
  `EnactmentEffects.committee_changes` type the FIDELITY cluster pre-discriminated.)

## What must never be possible

- **N-1.** A ratified `NoConfidence` that leaves the committee non-empty at the
  next epoch (it must dissolve to empty).
- **N-2.** An `UpdateCommittee` write-back that re-collapses two distinct
  discriminated committee members to one key (illegal-state: the map is
  `StakeCredential`-keyed; collapse is unrepresentable).
- **N-3.** A committee member added with no term-expiry epoch, or removed member
  lingering in the map.
- **N-4.** Best-effort / partial committee mutation: a malformed `UpdateCommittee`
  payload must be a deterministic structured reject at decode, never a
  silently-empty or partially-applied committee.

## What must remain identical across executions (replay)

- **R-1.** Replaying the same ratified proposal set produces a byte-identical
  post-enactment `ConwayGovState` committee + quorum fingerprint
  (`fingerprint.governance`). The structured `UpdateCommittee` re-encoding is a
  deliberate, oracle-confirmable fingerprint migration (**T-DET-01**) — the
  current `[4, prev, raw_bytes]` opaque-bytes encoding is replaced by the real
  Conway `[4, prev, removed_set, added_map, threshold]` shape.

## Canonical input → canonical output

- Input: the ratified `[GovActionState]` (already canonical) carrying structured
  `UpdateCommittee { prev_action, removed, added, threshold }`.
- Output: `EnactmentEffects { committee_dissolved, committee_changes,
  committee_threshold, … }` → applied to the next `ConwayGovState`.
- Pure transition, no I/O, no clock: BLUE (`ade_ledger::governance`,
  `ade_ledger::rules` boundary).

## Proof obligations (slice-entry, not footnotes)

- **PO-1 (wire grammar).** Conway `update_committee = (4, gov_action_id / null,
  set<committee_cold_credential>, { committee_cold_credential => epoch_no },
  unit_interval)`; `unit_interval = #6.30([uint, uint])`;
  `committee_cold_credential = credential = [0, keyhash] / [1, scripthash]`. The
  `set` may be a plain array or tag-258-wrapped array; the decoder accepts both.
  Pinned by the S1 decode tests against hand-built CBOR.
- **PO-2 (oracle availability).** No local snapshot pair exhibits a ratified
  committee transition (mainnet committee unchanged across the available
  boundary epochs; the epoch-576 VState oracle is environment-blocked —
  recoverable only by re-extracting from the ImmutableDB EBS snapshots). Real
  committee-change agreement vs cardano-node is therefore reclassified
  **environment-blocked** per tier doctrine (same posture as
  DC-LEDGER-08/09/10). Mechanical closure = synthetic positive + adversarial +
  replay, mirroring the `gov_state_corpus` (PHASE4-B5) pattern.
- **PO-3 (decode is newly written).** The snapshot loader's `parse_gov_action`
  currently discards committee data (`raw: Vec::new()`); structured decode is
  net-new in S1, not a refactor of existing parsing.

## Declared non-goals

- Decoding `proposal_procedures` from real **tx bodies** into `GovAction`
  (the wire codec keeps them as opaque `Option<Vec<u8>>`; a separate cluster).
- `NewConstitution` enactment write-back beyond its existing `new_constitution`
  effect (constitution is already captured; not committee state).
- Committee-member **tx-validity** gating (OQ-3, a declared separable follow-up).
