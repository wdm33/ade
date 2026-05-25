# Invariant Slice — PROPOSAL-PROCEDURES-DECODE PP-S1

## Slice Header
**Slice Name:** BLUE decoder + closed `ProposalProcedure` type + CI gate + DC-LEDGER-11
**Cluster:** PROPOSAL-PROCEDURES-DECODE
**Status:** Proposed
**CEs addressed:** CE-PP-1, CE-PP-2, CE-PP-3, CE-PP-4, CE-PP-5
**Dependencies:** PHASE4-B1..B5 (Conway tx-body decoder); ENACTMENT-COMMITTEE-WRITEBACK (UpdateCommittee structured form via DC-LEDGER-10).

---

## Intent

Convert `ConwayTxBody.proposal_procedures` from opaque `Option<Vec<u8>>`
to a typed closed `Option<Vec<ProposalProcedure>>`, decoded through a
single sanctioned entry point. Mirrors the established
`decode_conway_certs` pattern: unknown tags reject, structural
failures reject, trailing garbage rejects, era-gate preserved,
DC-LEDGER-10 (UpdateCommittee discriminant) consumed unchanged.

---

## The change (atomic; compile green as one unit)

### 1. New closed type `ProposalProcedure`

`crates/ade_types/src/conway/governance.rs` — add:

```rust
/// CIP-1694 proposal_procedure = [deposit, return_addr, gov_action, anchor].
/// `return_addr` carries reward-account bytes verbatim (OQ-4 — typed
/// RewardAccount is a separate fidelity decision). `anchor` reuses
/// the existing opaque struct (OQ-3-adjacent — nested anchor opacity
/// is not in this cluster's scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalProcedure {
    pub deposit: Coin,
    pub return_addr: Vec<u8>,
    pub gov_action: GovAction,
    pub anchor: Anchor,
}
```

### 2. `ConwayTxBody` type-shape change

`crates/ade_types/src/conway/tx.rs:46`:

```rust
// before:
pub proposal_procedures: Option<Vec<u8>>,
// after:
pub proposal_procedures: Option<Vec<ProposalProcedure>>,
```

### 3. New BLUE decoder module

`crates/ade_codec/src/conway/governance.rs` (new), wired into
`crates/ade_codec/src/conway/mod.rs`:

```rust
// Public entry: decode_proposal_procedures(bytes) -> Vec<ProposalProcedure>.
// Closed grammar, fully consumed; non-empty set; trailing rejects.
pub fn decode_proposal_procedures(data: &[u8]) -> Result<Vec<ProposalProcedure>, CodecError>;
pub fn encode_proposal_procedures(procs: &[ProposalProcedure]) -> Vec<u8>;

// Helpers:
fn decode_proposal_procedure(data: &[u8], offset: &mut usize) -> Result<ProposalProcedure, CodecError>;
fn decode_gov_action(data: &[u8], offset: &mut usize) -> Result<GovAction, CodecError>;
fn decode_anchor(data: &[u8], offset: &mut usize) -> Result<Anchor, CodecError>;  // opaque - skip_item, capture bytes
fn decode_gov_action_id_opt(data: &[u8], offset: &mut usize) -> Result<Option<GovActionId>, CodecError>;
fn decode_stake_credential(data: &[u8], offset: &mut usize) -> Result<StakeCredential, CodecError>;  // local copy of the cert-decoder shape
fn decode_unit_interval(data: &[u8], offset: &mut usize) -> Result<(u64, u64), CodecError>;
```

The decoder rejects on:
  - `arr_len < 1` outer array (PP-N6)
  - `set_len == 0` for the procedures (PP-N5)
  - non-array `proposal_procedure` shape (PP-N6)
  - unknown `gov_action` tag (PP-N1)
  - missing or wrong-typed deposit / return_addr / anchor / gov_action (PP-5)
  - structurally invalid Anchor (PP-N8 — opaque but the OUTER frame
    must still be a CBOR item)
  - trailing bytes after the procedures (PP-8 / PP-N6)

### 4. Modify body decoder + encoder at key 20

`crates/ade_codec/src/conway/tx.rs`:

```rust
// decoder, key 20:
let (start, end) = cbor::skip_item(data, offset)?;
let typed = decode_proposal_procedures(&data[start..end])?;
proposal_procedures = Some(typed);

// encoder, key 20:
if let Some(ref procs) = body.proposal_procedures {
    write_uint(buf, 20);
    buf.extend_from_slice(&encode_proposal_procedures(procs));
}
```

Era-gate stays at the existing site (no change to the
`ProposalProceduresInPreConway` error path).

### 5. Cascading test-file updates (OQ-8)

Per the `grep` evidence in the invariants sketch:
  - ~10 files set `proposal_procedures: None` — **no change needed**;
    `None` stays type-compatible with `Option<Vec<ProposalProcedure>>`.
  - 2 files set `Some(vec![0x80])` in `crates/ade_ledger/src/conway.rs`
    (lines :344, :519) — these are negative-path tests that don't
    care about proposal shape; convert to `None` (the field is no
    longer load-bearing for those tests' adversarial intent).

### 6. New CI gate
   `ci/ci_check_proposal_procedures_closed.sh`

Mechanical guards:
  1. `crates/ade_types/src/conway/governance.rs` defines
     `pub struct ProposalProcedure` (with the 4 fields).
  2. `crates/ade_types/src/conway/tx.rs` declares
     `proposal_procedures: Option<Vec<ProposalProcedure>>` —
     **not** `Option<Vec<u8>>`.
  3. `crates/ade_codec/src/conway/governance.rs` exists and exports
     `decode_proposal_procedures` + `encode_proposal_procedures`.
  4. No `ProposalProcedure {` struct-literal construction outside
     the new `ade_codec/src/conway/governance.rs` decoder file and
     the testkit (the testkit governance harness is the sanctioned
     synthesis site for fixtures). Production callers MUST go through
     `decode_proposal_procedures`.
  5. No bare `Vec<u8>` reconstruction of the field anywhere
     (`proposal_procedures: Some(vec!` / `proposal_procedures = Some(vec!`
     greps to nothing in production source, including the
     `Some(vec![0x80])` form that was scrubbed in step 5 above).

### 7. Registry append

```toml
[[rules]]
id = "DC-LEDGER-11"
tier = "derived"
statement = "proposal_procedures MUST NOT remain an opaque byte field in the authoritative Conway tx-body shape. ConwayTxBody.proposal_procedures is Option<Vec<ProposalProcedure>>, decoded through a single closed entry point that rejects unknown gov_action tags, structural failures, empty sets, and trailing garbage deterministically; the typed form re-encodes byte-identically (PreservedCbor) for every well-formed Conway tx body."
source = "CIP-1694; Project constitution §3 (closed semantic surfaces, T-CORE-01); DC-LEDGER-10 (downstream credential discriminant must not be re-collapsed)"
cross_ref = ["DC-LEDGER-10"]
code_locus = "crates/ade_codec/src/conway/governance.rs (decode_proposal_procedures, decode_proposal_procedure, decode_gov_action, encode_proposal_procedures); crates/ade_types/src/conway/governance.rs (ProposalProcedure); crates/ade_codec/src/conway/tx.rs (typed key 20 path)"
tests = [...]  # listed below
ci_script = "ci/ci_check_proposal_procedures_closed.sh"
status = "enforced"
introduced_in = "PROPOSAL-PROCEDURES-DECODE"
strengthened_in = []
```

`DC-LEDGER-10.cross_ref += "DC-LEDGER-11"` (bidirectional pairing).

### 8. Unit tests (BLUE: `crates/ade_codec/src/conway/governance.rs`
   `#[cfg(test)] mod tests`)

Round-trip:
  - `roundtrip_info_action_proposal` — single procedure with InfoAction.
  - `roundtrip_hard_fork_initiation` — HardForkInitiation with prev_action and protocol_version.
  - `roundtrip_no_confidence` — NoConfidence with prev_action.
  - `roundtrip_treasury_withdrawals` — TreasuryWithdrawals with 1 withdrawal + null policy_hash.
  - `roundtrip_parameter_change` — ParameterChange with prev_action=null + opaque pparams update bytes + scripthash.
  - `roundtrip_new_constitution` — NewConstitution with prev_action + constitution opaque bytes.
  - `roundtrip_update_committee` — UpdateCommittee with discriminated StakeCredential cold creds in BOTH removed (1 KeyHash + 1 ScriptHash) and added (1 KeyHash) — DC-LEDGER-10 preserved.
  - `roundtrip_multi_procedure` — vec of 3 different procedures.

Rejection grammar:
  - `rejects_unknown_gov_action_tag` — tag 99 in the inner action.
  - `rejects_empty_proposal_procedures_set` — outer set with 0 elements.
  - `rejects_trailing_garbage` — extra bytes after the set.
  - `rejects_truncated_proposal_procedure` — missing anchor field.
  - `rejects_structurally_invalid_anchor` — anchor not a CBOR item.
  - `rejects_invalid_stake_credential_in_update_committee` — DC-LEDGER-10 cross-check.

DC-LEDGER-10 preservation:
  - `update_committee_keeps_stake_credential_discriminant` — decode a
    procedure with `removed = {KeyHash(h), ScriptHash(h)}` (same 28
    bytes); the two creds remain distinct after round-trip.

### 9. Integration tests
   (`crates/ade_codec/tests/conway_tx_body_proposal_procedures.rs`)

  - `body_with_proposal_procedures_round_trips_through_typed_field` —
    construct a full ConwayTxBody with `Some(vec![pp])`, encode,
    decode, assert the typed field round-trips.
  - `pre_conway_era_still_rejects_proposal_procedures` —
    pre-Conway era body decoder still emits
    `ProposalProceduresInPreConway` (CE-PP-4 ground truth).

---

## Mechanical Acceptance Criteria

- **AC-1** — `cargo build --workspace` green.
- **AC-2** — `cargo test -p ade_codec` green (all new unit + integration tests pass).
- **AC-3** — `cargo test -p ade_ledger`, `cargo test -p ade_types`,
  `cargo test -p ade_testkit` green (no regressions from the
  type-shape change or the OQ-8 fixture scrubs).
- **AC-4** — `bash ci/ci_check_proposal_procedures_closed.sh` PASS (all 5 guards).
- **AC-5** — `bash ci/ci_check_credential_discriminant_closed.sh`
  still PASS (DC-LEDGER-10 unaffected).
- **AC-6** — `bash ci/ci_check_constitution_coverage.sh` PASS
  (176 entries — was 175).
- **AC-7** — registry rule count goes from 175 → 176.

---

## Hard Prohibitions

- No `#[non_exhaustive]` on `ProposalProcedure` (closed struct).
- No `Vec<u8>` form of `ConwayTxBody.proposal_procedures` on the
  BLUE authority path (CI-enforced).
- No `ProposalProcedure {` struct-literal construction outside the
  decoder + the testkit fixture builders (CI-enforced).
- No decoding of `voting_procedures`, `ParameterChange.update`, or
  `NewConstitution.raw` in this slice (OQ-1/2/3 scope locks).
- No typed `RewardAccount` for `return_addr` (OQ-4 scope lock —
  keep `Vec<u8>`).
- No `HashMap`/`HashSet`/wall-clock/RNG/float in the decoder.
- No new dependency edge from `ade_codec` to `ade_ledger` /
  `ade_testkit` / `ade_runtime`.

---

## Explicit Non-Goals

- Corpus replay harness (that's PP-S2).
- `voting_procedures` decode (OQ-1; separate cluster).
- `ParameterChange.update` decode (OQ-2; separate cluster).
- `NewConstitution.raw` decode (OQ-3; separate cluster).
- Typed `RewardAccount` (OQ-4; separate fidelity decision).
- GREEN snapshot-loader changes (different path).
