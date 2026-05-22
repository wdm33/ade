# Cluster ENACTMENT-COMMITTEE-WRITEBACK — wire committee enactment logic

> **Status:** Planning artifact (non-normative). **Strengthens DC-EPOCH-01**
> (enactment atomicity now actually rewrites committee state) and **DC-LEDGER-10**
> (discriminated committee credential survives the enactment write-back). This is
> the "separate governance-enactment cluster" that `ENACTMENT-COMMITTEE-FIDELITY`
> explicitly deferred. Two slices. Registry/specs win on any conflict.

## Primary invariant (strengthens DC-EPOCH-01)
> A ratified committee-changing governance action (`NoConfidence` or
> `UpdateCommittee`) deterministically rewrites the next-epoch
> `ConwayGovState.committee` and `committee_quorum`. No ratified committee change
> is observed-and-dropped. The members removed/added enter the committee as
> discriminated `StakeCredential` (cold credential), never tag-erased `Hash28`,
> so the write-back cannot re-collapse the map this credential family
> discriminated (DC-LEDGER-10).

## The latent gap (what is broken at HEAD)
1. **Enactment is a no-op for the committee.** `enact_proposals` (`governance.rs`)
   sets `committee_dissolved` for `NoConfidence` but its `UpdateCommittee` arm is
   `let _ = raw;` — it produces no `committee_changes`.
2. **The apply site drops both.** `rules.rs` (~line 1222) builds the next
   `ConwayGovState` with `committee: gov.committee.clone()` and
   `committee_quorum: gov.committee_quorum` — it reads neither
   `effects.committee_dissolved` nor `effects.committee_changes`. So even a
   ratified `NoConfidence` leaves the committee intact. **N-1 is violable today.**
3. **The data is never captured.** The snapshot loader's `parse_gov_action`
   returns `UpdateCommittee { prev_action: None, raw: Vec::new() }` — the
   removed/added members and threshold are discarded at decode (PO-3).
4. **The fingerprint is not wire-faithful.** `write_gov_action` encodes
   `UpdateCommittee` as `[4, prev, raw_bytes]` (a 3-element array with an opaque
   byte string) instead of the real Conway `[4, prev, removed_set, added_map,
   threshold]` 5-element shape.

## Grounding (HEAD 3706534)
- `GovAction::UpdateCommittee { prev_action: Option<GovActionId>, raw: Vec<u8> }`
  — `ade_types/src/conway/governance.rs:34`.
- `enact_proposals` `UpdateCommittee` arm `let _ = raw` — `governance.rs:399-403`.
- `EnactmentEffects.committee_changes: Option<(Vec<StakeCredential>,
  Vec<(StakeCredential,u64)>)>` (discriminated, dormant) — `governance.rs:348`.
- Apply site `committee: gov.committee.clone()` — `rules.rs:1222-1223`.
- `write_gov_action` `UpdateCommittee` → `[4, prev, raw_bytes]` —
  `fingerprint.rs:664-669`.
- `parse_gov_action` tag 4 → `raw: Vec::new()` —
  `snapshot_loader.rs:2505-2507`.
- Only constructors of `UpdateCommittee` from CBOR: the loader (raw empty). The
  wire codec keeps `proposal_procedures` opaque (`conway/tx.rs`) — non-goal.
- No test/corpus pins an `UpdateCommittee` fingerprint → the migration is safe.

## Per-surface change
| surface | before | after |
|---|---|---|
| `GovAction::UpdateCommittee` | `{ prev_action, raw: Vec<u8> }` | `{ prev_action, removed: BTreeSet<StakeCredential>, added: BTreeMap<StakeCredential,u64>, threshold: (u64,u64) }` |
| `write_gov_action` (UpdateCommittee) | `[4, prev, bytes(raw)]` | `[4, prev, set(removed), map(added), unit_interval(threshold)]` |
| `parse_gov_action` (tag 4) | discards → `raw: Vec::new()` | decodes set / map / unit_interval into structured fields |
| `EnactmentEffects` | `committee_changes` dormant; no threshold | `committee_changes` populated; `+ committee_threshold: Option<(u64,u64)>` |
| `enact_proposals` (UpdateCommittee) | `let _ = raw` | populate `committee_changes` + `committee_threshold` |
| `rules.rs` apply site | clones committee + quorum | applies `committee_dissolved` then `committee_changes` + `committee_threshold` |

## Exit Criteria (CI-Verifiable)
- **CE-1 (structured type + wire-faithful fingerprint).** `GovAction::UpdateCommittee`
  carries `{removed, added, threshold}`; `write_gov_action` emits the 5-element
  Conway array; round-trips via the loader decode. Compile + new decode tests.
- **CE-2 (decode fidelity).** `update_committee_decodes_removed_added_threshold`
  — a hand-built Conway `update_committee` CBOR (both plain-array and tag-258 set
  forms) decodes to the exact removed set, added map (with epochs), and
  threshold; `update_committee_decode_rejects_malformed` — a truncated / wrong-tag
  payload is a deterministic structured reject (N-4), never silent-empty.
- **CE-3 (discriminant through decode + write-back).**
  `update_committee_keyhash_scripthash_members_distinct` — key-hash and
  script-hash members of equal bytes decode to distinct map entries and remain
  distinct after enactment write-back (I-3 / DC-LEDGER-10).
- **CE-4 (NoConfidence dissolve).** `enact_noconfidence_dissolves_committee` —
  the apply site empties the committee for a ratified `NoConfidence` (N-1).
- **CE-5 (UpdateCommittee write-back).**
  `enact_update_committee_applies_changes` — removed members leave the map, added
  members enter with their expiry epoch, and `committee_quorum` becomes the new
  threshold, under a controlled base `ConwayGovState` (I-1).
- **CE-6 (replay).** `committee_enactment_replays_byte_identical` — the
  post-enactment gov-state fingerprint is byte-identical across two runs
  (R-1 / T-DET-01).
- **CE-7 (CI + strengthen).** The Conway/credential CI gate covers the new
  surface; DC-EPOCH-01 `strengthened_in += ENACTMENT-COMMITTEE-WRITEBACK` and
  DC-LEDGER-10 likewise; coverage PASS; full `cargo test --workspace` green.

## Slices
- **S1 — structured `UpdateCommittee` (type + decode + fingerprint).** Replace
  `raw` with `{removed, added, threshold}`; rewrite `write_gov_action` to the
  real Conway encoding; rewrite `parse_gov_action` tag-4 to decode set/map/
  unit_interval (accepting plain + tag-258 sets). Decode + distinctness +
  fingerprint-migration tests. *(CE-1..CE-3, CE-6 fingerprint half.)* TCB: BLUE
  (`ade_types`, `ade_ledger::fingerprint`); GREEN (`ade_testkit` loader).
- **S2 — committee enactment write-back logic.** `enact_proposals` populates
  `committee_changes` + `committee_threshold`; the `rules.rs` apply site applies
  `committee_dissolved` then `committee_changes`/`committee_threshold` into the
  next `ConwayGovState`. Positive + adversarial + replay corpus (mirrors
  `gov_state_corpus`). *(CE-4..CE-7.)* TCB: BLUE (`ade_ledger::governance`,
  `ade_ledger::rules`).

## TCB Color Map
- **BLUE** — `crates/ade_types/src/conway/governance.rs`,
  `crates/ade_ledger/src/governance.rs`, `crates/ade_ledger/src/fingerprint.rs`,
  `crates/ade_ledger/src/rules.rs` (apply site).
- **GREEN** — `crates/ade_testkit/src/harness/snapshot_loader.rs` (decode),
  corpus tests.
- **RED** — none.

## Forbidden
- Reverting `committee_changes` to bare `Hash28`, or keying any committee map on
  `Hash28`.
- Reintroducing the opaque `[4, prev, raw_bytes]` fingerprint encoding.
- Best-effort / partial committee mutation on a malformed payload (must reject).
- Changing ratification logic or enactment priority order.

## Declared non-goals
- Decoding `proposal_procedures` from tx bodies into `GovAction` (separate
  cluster; codec keeps them opaque).
- `NewConstitution` write-back beyond the existing `new_constitution` effect.
- Committee-member tx-validity gating (OQ-3).
- Real-chain committee-transition oracle agreement — environment-blocked (PO-2),
  reclassified per tier doctrine.
