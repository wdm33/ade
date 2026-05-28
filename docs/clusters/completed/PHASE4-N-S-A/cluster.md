# PHASE4-N-S-A — KES signs real unsigned-header pre-image (cluster doc)

> **Status:** Planning. 4-slice sub-cluster shipping the
> BLUE canonical pre-image recipe + branded
> `UnsignedHeaderPreImage` type + a real
> `kes_sign_header(&UnsignedHeaderPreImage)` API + the
> `forge_block` / `run_real_forge` integration that replaces
> N-R-A's placeholder.
>
> **Predecessor:** PHASE4-N-R close (HEAD `c02aefc`) +
> N-S planning (HEAD `72bcda3`).
> **Successor:** PHASE4-N-S-B (independent of A; can be
> developed in parallel since module trees are disjoint).
>
> **Closure type:** MECHANICAL (cargo test + CI gates).
> No operator action required.

## §1 Primary invariant

> The KES signature in a forged block's header is over the
> **canonical unsigned-header CBOR pre-image** — the CBOR
> encoding of `ShelleyHeaderBody` (the first element of the
> outer `[header_body, kes_signature]` header array). The
> producer-side recipe and the validator-side extractor
> (`header_input::decode_block.header_input.kes.header_body_bytes`)
> produce byte-identical output. The branded
> `UnsignedHeaderPreImage(Vec<u8>)` type makes
> arbitrary-byte signing mechanically unrepresentable —
> `kes_sign_header` accepts only this type, and the only
> constructor is the canonical recipe.

## §2 Doctrine: producer-validator pre-image equivalence

```
Producer side:                       Validator side:
                                     
ShelleyHeaderBody {                  decode_block(block_bytes):
  slot, block_no, prev_hash,           ↓
  vrf_data, opcert, kes_period,        DecodedBlock {
  hot_vkey, body_hash,                   header_input: HeaderInput {
  body_size, protocol_version            kes: Some(HeaderKes {
}                                          header_body_bytes  ← AUTHORITY
  ↓ encode (canonical CBOR)            }),
UnsignedHeaderPreImage(Vec<u8>)        ...
  ↓                                  } }
kes_sign_header(sk, period, &)
  ↓
KesSignature
```

A2's reference fixture asserts these two arrows produce
byte-identical bytes for every corpus block.

## §3 Slice index

| Slice | Purpose | Closes (invariant IDs) |
|---|---|---|
| **A1** | Planning + 3 candidate registry entries (`CN-KES-HEADER-01`, `DC-KES-HEADER-01`, `CN-PREIMAGE-FIXTURE-01`) declared. OQ1 + OQ2 + OQ-S-A pre-flight audits + fixture metadata. | — (declarative) |
| **A2** | New BLUE module `ade_ledger::block_validity::unsigned_header_pre_image` exporting `UnsignedHeaderPreImage(Vec<u8>)` branded type + the canonical recipe + the byte-match test against corpus `decode_block` extraction. Refactor `verify_header_kes` to consume the branded type (grep gate). | `CN-KES-HEADER-01`, `DC-KES-HEADER-01`, `CN-PREIMAGE-FIXTURE-01` (BLUE side); N1, N2, D1 |
| **A3** | New RED API `producer_shell.kes_sign_header(period, &UnsignedHeaderPreImage) -> KesSignature`. Bridge function in `ade_runtime::producer::scheduler` that runs the 9-step construction sequence (body → body_hash → body_size → header_body → pre-image → KES sign → assemble → block). Replace `run_real_forge` step 3's placeholder. | N1, N3, D2 |
| **A4** | Integration tests + sub-cluster close. `full_stake_answer_reaches_self_accept_and_rejects` inverts to `..._and_accepts` (real KES signature now verifies). Flip 3 rules to `enforced` + strengthen `CN-FORGE-01` + `DC-CONS-18`. | I3, R1; sub-cluster close |

## §4 Exit criteria (CI-verifiable)

- [ ] CE-A-1: Recipe + branded type land at
  `ade_ledger::block_validity::unsigned_header_pre_image`.
- [ ] CE-A-2: Byte-match test
  `unsigned_header_preimage_matches_decode_block_extraction`
  passes for every corpus block — Ade's recipe output is
  byte-identical to `decode_block.header_input.kes.header_body_bytes`.
- [ ] CE-A-3: `verify_header_kes` refactored to accept
  `&UnsignedHeaderPreImage`. Mechanical grep gate
  `ci/ci_check_unsigned_header_preimage_single_source.sh`
  passes — only `unsigned_header_pre_image` constructs the
  branded type.
- [ ] CE-A-4: `producer_shell.kes_sign_header(&UnsignedHeaderPreImage)`
  exists; accepts only the branded type at compile time.
- [ ] CE-A-5: `run_real_forge` step 3 no longer signs
  placeholder bytes (`expected_vrf_input`); signs the
  recipe's output instead.
- [ ] CE-A-6: A4 integration test
  `forge_signed_block_self_accepts_for_synthetic_full_stake_corpus`
  passes — full-stake `LeaderScheduleAnswer` now reaches
  `ForgeSucceeded` (step 6 self_accept = Accepted).
- [ ] CE-A-7: 3 N-S-A rules flipped to `enforced`;
  `CN-FORGE-01.strengthened_in += "PHASE4-N-S-A"`;
  `DC-CONS-18.strengthened_in += "PHASE4-N-S-A"`.
- [ ] CE-A-8: `cargo test --workspace --lib` clean.

## §5 Honest scope

A4's integration test still uses a synthetic
`LeaderScheduleAnswer` (in-memory; not a real chain). The
**bridge proof** (cardano-node accepts the forged block
over N2N) is N-S-C's deliverable. A4 proves the
*producer-side closure*: Ade can construct a block whose
KES signature verifies under its own validator
(`self_accept = Accepted`). The cross-impl claim against
cardano-node moves through N-S-B (transmit wiring) + N-S-C
(operator pass).

## §6 References

- Predecessor cluster: PHASE4-N-R close ([[project-phase4-n-r-closed]]).
- Cluster plan: [`../../planning/phase4-n-s-cluster-slice-plan.md`](../../planning/phase4-n-s-cluster-slice-plan.md).
- Reference fixture metadata: [`../../../crates/ade_ledger/tests/fixtures/unsigned_header_preimage/FIXTURE_METADATA.md`](../../../crates/ade_ledger/tests/fixtures/unsigned_header_preimage/FIXTURE_METADATA.md).
- Doctrine: [[feedback-fail-closed-validation]],
  [[feedback-proof-discipline]],
  [[feedback-shell-must-not-overstate-semantic-truth]].
