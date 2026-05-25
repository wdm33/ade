# proposal_procedures tx-body decode — invariant sketch

> **Status:** Planning artifact (non-normative). Produced via `/invariants`
> on 2026-05-25 against HEAD `3af9e2b`. Closes the last open
> governance-domain decode seam (declared non-goal in
> ENACTMENT-COMMITTEE-WRITEBACK; carried forward in N-E). Authority
> on close will live in `docs/ade-invariant-registry.toml` as the
> new `DC-LEDGER-11`. If this doc conflicts with the registry/specs,
> those win.

## Scope

Convert `ConwayTxBody.proposal_procedures` from `Option<Vec<u8>>`
(opaque pass-through at codec key 20) to a typed closed sub-grammar
`Option<Vec<ProposalProcedure>>` decoded via a new closed
`decode_proposal_procedures` function. Parallel to the established
`decode_conway_certs` pattern: unknown tags reject, structural
failures reject, no decode-and-re-encode at the body boundary.

**Out of scope (per user-locked OQ resolutions, 2026-05-25):**

| OQ | Resolution |
|---|---|
| OQ-1 | Defer `voting_procedures` (key 19). Combining both turns a sealed decode slice into a broad "governance tx-body cleanup" refactor. |
| OQ-2 | Keep `GovAction::ParameterChange.update` opaque. Full pparams update sub-grammar is a separate large semantic surface; pulling it in blurs this slice's proof obligation. |
| OQ-3 | Keep `GovAction::NewConstitution.raw` opaque. Same reasoning as OQ-2; later strengthening slice. |
| OQ-4 | Keep `proposal_procedure.return_addr` as raw reward-account `Vec<u8>`. Typed `RewardAccount` is a separate fidelity decision. |
| OQ-5 | Prefer existing corpus; synthesize canonical fixtures if no real Conway tx in current corpus carries proposals. Real-chain corpus is a strengthening slice. |
| OQ-6 | New registry rule (`DC-LEDGER-11`), not a strengthening of an existing one. Closed governance sub-grammar boundary is large enough to warrant a dedicated entry. |
| OQ-7 | New CI gate `ci/ci_check_proposal_procedures_closed.sh`. Extending a generic closed-enum script would hide the actual risk: opaque governance bytes re-entering authoritative tx-body paths. |
| OQ-8 | Reset irrelevant tests to `None`; synthesize only where load-bearing. Drop meaningless `Some(vec![0x80])` fixtures. |

## 1. What must always be true

- **PP-1 (closed sub-grammar).** `proposal_procedures` decodes through
  exactly one closed-grammar entry point
  `decode_proposal_procedures` (parallel to `decode_conway_certs`).
  Successful decode produces a typed `Vec<ProposalProcedure>`; any
  bytes the spec does not name fail at decode.
- **PP-2 (PreservedCbor).** The original `proposal_procedures` bytes
  survive end-to-end alongside the typed decode. Encoding the typed
  form reproduces the original bytes byte-identically for every
  well-formed Conway tx body.
- **PP-3 (closed `GovAction` reuse).** Each procedure's `gov_action`
  field decodes to the *existing* closed `GovAction` enum (already
  structured). `UpdateCommittee.{removed, added}` continues to use the
  discriminated `StakeCredential` form — DC-LEDGER-10 stays enforced.
- **PP-4 (closed `Anchor`).** Each procedure carries an `Anchor`
  decoded to the *existing* `Anchor` struct (`url: text`,
  `hash: Hash32`).
- **PP-5 (deposit + return_addr present).** Every procedure has a
  canonical `deposit: Coin` and `return_addr: Vec<u8>` (reward-account
  bytes, per OQ-4). Missing either is a deterministic decode reject.
- **PP-6 (non-empty set when present).** Per CIP-1694, the
  `proposal_procedures` field is a non-empty set; `Some(vec![])` is a
  decode-time invariant violation.
- **PP-7 (era gating preserved).** `proposal_procedures` decodes only
  on Conway+ era. The existing era-gate
  (`ProposalProceduresInPreConway`) stays as-is.
- **PP-8 (consume-completely).** `decode_proposal_procedures`
  consumes the entire input byte slice; trailing garbage is a decode
  reject (same `require_consumed` discipline as `decode_conway_certs`).
- **PP-9 (CI-enforced closure).** `ci_check_proposal_procedures_closed.sh`
  defends:
  (a) `ConwayTxBody.proposal_procedures` is
      `Option<Vec<ProposalProcedure>>`, not `Option<Vec<u8>>`;
  (b) `decode_proposal_procedures` is the only sanctioned production
      decoder;
  (c) no path constructs `ProposalProcedure` from raw bytes outside
      the decoder.

## 2. What must never be possible

- **PP-N1.** A `ProposalProcedure` with an unknown `gov_action` tag
  accepted (unknown tags reject — same shape as the cert decoder's
  `UnknownCertTag`).
- **PP-N2.** Tag-erased `UpdateCommittee` cold credentials (DC-LEDGER-10
  guard already exists; the new decoder MUST route through the
  existing structured construction, not bypass it).
- **PP-N3.** A `proposal_procedures` byte run that decodes to
  `Vec<ProposalProcedure>` but does NOT re-encode to the same input
  bytes (PreservedCbor round-trip on the whole field).
- **PP-N4.** A `proposal_procedures` field that decodes successfully
  in a pre-Conway era.
- **PP-N5.** An empty `proposal_procedures` set treated as `Some([])`
  rather than rejected (PP-6).
- **PP-N6.** Trailing CBOR after the procedures consumed silently
  (PP-8).
- **PP-N7.** A future authority path branching on `ProposalProcedure`
  shape outside `decode_proposal_procedures` / direct pattern-match
  on the closed type — i.e., no string-keyed map of procedures, no
  `dyn` trait object behind procedures.
- **PP-N8.** `Anchor.url`/`Anchor.hash` constructed outside the
  anchor sub-decoder (closure rule for the nested anchor surface).

## 3. What must remain identical across executions

- The function `(tx_body_bytes, era) → Result<ConwayTxBody, CodecError>`
  remains total and deterministic; the only change is that
  `body.proposal_procedures` becomes a typed structure, not opaque bytes.
- Closed discriminants frozen per cluster cut:
  - `ProposalProcedure` (new closed struct)
  - `Anchor` (existing, no change)
  - `GovAction` (existing, no change; UpdateCommittee stays
    DC-LEDGER-10-compliant)
- Re-encoding `Vec<ProposalProcedure>` produces byte-identical
  Cardano-canonical CBOR for every well-formed Conway tx body.

## 4. What must be replay-equivalent

- **PreservedCbor end-to-end.** For every well-formed Conway tx in the
  existing corpus, `decode_conway_tx_body(bytes) → body →
  encode_conway_tx_body(body) → bytes'` produces `bytes == bytes'`
  byte-identically; the typed `proposal_procedures` field round-trips
  as part of that test.
- **Cert-state replay holds.** Existing `decode_conway_certs`
  corpus-replay tests stay green (no change to cert decoder).
- **Real-chain proposals replay** *(if corpus available)*. New corpus
  harness loads real Conway-era txs that carry `proposal_procedures`
  and asserts: (a) decode succeeds with the expected procedure count;
  (b) re-encode is byte-identical. Per OQ-5, synthesize canonical
  fixtures if no real-chain corpus exists.
- **GovState accumulation unaffected.** `ConwayGovState.proposals`
  population logic is downstream of the body decoder; today it reads
  proposals via the GREEN snapshot loader. The new tx-body decode
  adds a new BLUE decode path for txs in flight; the snapshot path
  is unchanged. Existing gov_state corpus tests stay green.

## 5. State transitions in scope

| Layer | Transition | Status |
|---|---|---|
| BLUE (exists) | `decode_conway_tx_body(bytes) → ConwayTxBody` | already closed; modified to populate the new typed field |
| **BLUE (new)** | **`decode_proposal_procedures(bytes) → Result<Vec<ProposalProcedure>, CodecError>`** | the new closed entry point |
| BLUE (new) | `decode_proposal_procedure(bytes, offset) → Result<ProposalProcedure, CodecError>` | per-procedure helper |
| BLUE (new) | `decode_gov_action(bytes, offset) → Result<GovAction, CodecError>` | nested action decoder (CIP-1694 7-variant grammar; uses existing GovAction enum; UpdateCommittee/NoConfidence/etc. all close) |
| BLUE (exists, reused) | `decode_anchor(...)` | already used elsewhere; reuse |
| BLUE (new) | `encode_proposal_procedures(&[ProposalProcedure]) → Vec<u8>` | mirror encoder for PreservedCbor round-trip |
| GREEN (new) | corpus harness `proposal_procedures_replay` over real-or-synthetic Conway tx bytes | round-trip + decode-shape assertions |

## 6. TCB color hypothesis

- **BLUE:**
  - `ade_codec::conway::governance::{decode,encode}_proposal_procedures`
    (new module — same shape as `ade_codec::conway::cert`).
  - `ade_types::conway::governance::ProposalProcedure` (new closed
    struct; existing `Anchor` + `GovAction` reused).
- **GREEN:**
  - `ade_testkit::governance::proposal_procedures_replay` harness.
  - Corpus fixture (real Conway tx bytes carrying proposal procedures,
    OR synthetic canonical fixtures per OQ-5).
- **RED:** none.

## 7. Cluster-plan tier framing (per user 2026-05-25)

| Item | Classification |
|---|---|
| Closed proposal procedure decode | derived |
| Preserved original CBOR bytes | true / derived bridge under byte-authority model |
| Conway+ era gating | derived |
| CI gate + corpus harness | release enforcement |
| No RED behavior | true boundary enforcement |

## 8. Registry move on close

**Append `DC-LEDGER-11` (verified next-free; existing IDs run -01 through -10).**

```toml
[[rules]]
id = "DC-LEDGER-11"
tier = "derived"
statement = "proposal_procedures MUST NOT remain an opaque byte field in the authoritative Conway tx-body shape. ConwayTxBody.proposal_procedures is Option<Vec<ProposalProcedure>>, decoded through a single closed entry point that rejects unknown gov_action tags, structural failures, empty sets, and trailing garbage deterministically; the typed form re-encodes byte-identically (PreservedCbor)."
source = "CIP-1694; Project constitution §3 (closed semantic surfaces, T-CORE-01); DC-LEDGER-10 (downstream credential discriminant must not be re-collapsed)"
cross_ref = ["DC-LEDGER-10", "T-CORE-01"]
code_locus = "crates/ade_codec/src/conway/governance.rs (decode_proposal_procedures, decode_proposal_procedure, decode_gov_action, encode_proposal_procedures); crates/ade_types/src/conway/governance.rs (ProposalProcedure)"
tests = []   # TBD per slice — round-trip, unknown-tag-rejects, non-empty-set, era-gate, real-or-synth corpus
ci_script = "ci/ci_check_proposal_procedures_closed.sh"
status = "enforced"          # flips on first-slice landing
introduced_in = "PROPOSAL-PROCEDURES-DECODE"
strengthened_in = []          # voting_procedures / ParameterChange.update / NewConstitution.raw closures are future strengthenings
```

**Narrowing applied per user pushback:** the statement is
`proposal_procedures`-specific, NOT the broader "no opaque-bytes
governance fields anywhere" claim. Voting procedures,
`ParameterChange.update`, and `NewConstitution.raw` remain opaque
(deliberately, per OQ-1/2/3); they are future
`strengthened_in` targets, not blockers to this rule's enforcement
state.

## 9. Open items for `/cluster-plan`

- **Cluster ID.** `PROPOSAL-PROCEDURES-DECODE` (named) — short
  and identifies the seam directly. Alternative: `PHASE4-G-A`
  (governance track A) — only fits if a "G-track" is meaningful;
  the bounty's classification doesn't naturally form a G-track.
  Default: `PROPOSAL-PROCEDURES-DECODE`.
- **Slice count.** Likely 2 slices — S1 the BLUE decoder + types +
  CI gate + registry append; S2 the GREEN corpus harness + replay
  evidence. Could be 1 if scope allows; cluster-plan decides.
- **CI gate scope.** Single script `ci_check_proposal_procedures_closed.sh`
  per OQ-7. Concrete guards: ConwayTxBody field type, decoder
  exists, no opaque `Vec<u8>` reconstruction outside decoder, no
  ProposalProcedure tuple-struct construction outside decoder.

## Related

- `docs/clusters/completed/ENACTMENT-COMMITTEE-WRITEBACK/cluster.md` —
  original declared-non-goal source.
- `docs/clusters/completed/PHASE4-N-E/cluster.md` — non-goal carried
  forward; this slice closes it.
- `docs/ade-SEAMS.md` §1 candidate row for `proposal_procedures` —
  flips from candidate to wired+closed on this cluster's close.
- `ade_codec::conway::cert::decode_conway_certs` — the closed-grammar
  pattern this decoder mirrors.
