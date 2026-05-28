# PHASE4-N-V — producer/validator codec symmetry (cluster doc)

> **Status:** Planning. 3-slice single cluster. Fixes the pre-existing forge
> bug root-caused in PHASE4-N-T: `forge_block` emits a **bare** Conway block
> (`0x85…`, `array(5)`) with no era envelope, so `decode_block` →
> `decode_block_envelope` (which requires `[era, block]`, `array(2)`) rejects
> **every** forged block at offset 0 — before any header/KES/leader/self_accept
> logic. The producer has never emitted a block its own validator can decode.
> **Scope-locked:** storage form `[era, block]` + self-decode round-trip only.
> The serve-side tag-24 wire-wrap for a live cardano-node peer is a **named
> follow-on**, not N-V.
>
> **Predecessor HEAD:** `be748ff` (PHASE4-N-T close).
> **Inputs:** [`docs/planning/phase4-n-v-invariants.md`](../../planning/phase4-n-v-invariants.md)
> + [`docs/planning/phase4-n-v-cluster-slice-plan.md`](../../planning/phase4-n-v-cluster-slice-plan.md).

---

## §1 Primary invariant

> Every block `forge_block` emits decodes via the **same** `decode_block`
> authority that validates received blocks: `forge_block` output is the
> era-tagged `[era, block]` form (era = Conway discriminant `7`), and
> `decode_block(forge_block(tick).bytes)` yields a `DecodedBlock` whose
> header-body fields and four preserved body-bucket bytes equal what was
> forged. A bare-block (no-envelope) forge output, or any forge↔decode
> asymmetry, is CI-gated impossible. *(new rule `CN-FORGE-03`; strengthens
> `CN-FORGE-01`, `DC-CONS-18`.)*

## §1.5 Doctrine: one envelope authority, both directions

`ade_codec::cbor::envelope::decode_block_envelope` is the sole block-envelope
**decoder** (`[era, block]`, `array(2)`, era uint then the inner block;
envelope.rs:29,45–48). N-V adds its **encoder twin**
`ade_codec::encode_block_envelope(era, block_bytes)` —
`array(2) ‖ uint(era) ‖ block_bytes` — so the producer and validator share a
single canonical envelope grammar. `forge_block` wraps its existing
`ShelleyBlock::ade_encode` output (the bare inner block, unchanged) via this
encoder. The body-bucket bytes the body-hash binds (`DC-CONS-16` / `DC-CONS-18`)
are untouched by the wrap — the envelope adds only the leading `82 07`.

## §2 Scope

### In scope
- **New BLUE `ade_codec::encode_block_envelope`** (symmetric to
  `decode_block_envelope`), in `crates/ade_codec/src/cbor/envelope.rs`.
- **`forge_block` (BLUE) emits enveloped bytes** via the new encoder;
  `ForgedBlock.bytes` becomes `[7, block]`. `ForgedBlock.block` (the
  `ShelleyBlock` value) is unchanged.
- **The missing forge↔decode round-trip test + CI gate** — the permanent gate
  whose absence hid this bug since N-C.
- **First in-process `ForgeSucceeded`** — a consistent eligible-leader fixture
  driving `forge_block → self_accept → Accepted`.
- **1 new registry rule** `CN-FORGE-03` (declared here, enforced at S3 close);
  strengthenings of `CN-FORGE-01` + `DC-CONS-18` recorded at S3 close.

### Out of scope (named follow-ons)
- **Serve-side tag-24 wire-wrap** — wrapping `[era, block]` into
  `[serialisationInfo, tag24(...)]` so a live cardano-node peer accepts the
  served block over block-fetch (the operator-pass leg of `CN-CONS-06` /
  `RO-LIVE-01`). The receive path already unwraps tag-24 (N-M-FRAG); the
  symmetric serve wrap is a separate cluster.
- **Forged-block durability** — N-U (WAL/ChainDB/snapshot/warm-start), carried
  from N-T.
- Multi-era envelopes beyond Conway (era 7) — N-V pins Conway only.

### Honest-scope reminder
N-V makes a forged block decodable + self-acceptable by Ade's own validator in
one process. It does **not** make the block servable to a live cardano-node
peer (that is the deferred wire-wrap), and it does not persist the block (N-U).

## §3 Slice index

| Slice | Purpose | Strengthens | Introduces |
|---|---|---|---|
| **S1** | New BLUE `ade_codec::encode_block_envelope(era, block_bytes) -> Vec<u8>` = `array(2) ‖ uint(era) ‖ block_bytes`, round-trip-inverse to `decode_block_envelope`. Unit tests: `decode(encode(era,b)) == (era,b)`; the Conway head `82 07 …` pinned against a real `ConwayValidityCorpus` block's first two bytes (resolves OQ1). Standalone; no consumer yet. | — | `CN-FORGE-03` (declared) |
| **S2** | `forge_block` wraps its `ShelleyBlock::ade_encode` output via `encode_block_envelope(CardanoEra::Conway, …)`. New forge↔decode round-trip test (`forge_block(tick) → decode_block` → Ok + structural equality of header fields + 4 body buckets) + CI gate `ci_check_forge_decode_round_trip.sh`. Confirm OQ5 (no consumer relied on the bare-block output; update any test asserting `ForgedBlock.bytes` byte-shape — `ForgedBlock.block` is unchanged). After S2, `self_accept` is reachable past decode. | `DC-CONS-18` | `ci_check_forge_decode_round_trip.sh` |
| **S3** | First in-process `ForgeSucceeded`: a consistent eligible-leader fixture (operator pool registered with `vrf_keyhash = blake2b256(vrf_vk)`, positive stake, ASC 1/1, matching eta0 + era schedule) drives `forge_block → self_accept → Accepted`; integration test `forge_to_self_accept_succeeds`. Resolves OQ4 — root-cause + fix any VRF-cert keyhash binding / nonce mismatch within the slice; **no faked success**. Flip `CN-FORGE-03` enforced; record strengthenings; cluster close. | `CN-FORGE-01` | — |

## §4 Exit criteria (CI-verifiable)

- [ ] **CE-V-1.** `docs/planning/phase4-n-v-{invariants,cluster-slice-plan}.md` + `docs/clusters/PHASE4-N-V/{cluster,S1,S2,S3}.md` exist.
- [ ] **CE-V-2.** `ade_codec::encode_block_envelope` exists; test `encode_decode_block_envelope_round_trips` passes (`decode_block_envelope(encode_block_envelope(era, b)) == (era, b)` for representative `b`).
- [ ] **CE-V-3.** Test `conway_envelope_head_matches_corpus` passes: `encode_block_envelope(Conway, inner)` begins `82 07`, byte-equal to the first two bytes of a real `ConwayValidityCorpus` block (OQ1 pinned).
- [ ] **CE-V-4.** Test `forge_block_output_decodes_via_decode_block` passes: `decode_block(forge_block(base_tick()).bytes)` is `Ok`, and the decoded header fields (slot, block_number, body_hash) + the 4 body-bucket spans equal the forged inputs. (This test fails on `main` today — it is the regression-lock.)
- [ ] **CE-V-5.** `ci/ci_check_forge_decode_round_trip.sh` passes and FAILs if `forge_block` is changed to emit a non-enveloped block (grep gate: `forge_block` routes through `encode_block_envelope`; no bare `ShelleyBlock::ade_encode` result returned as `ForgedBlock.bytes`).
- [ ] **CE-V-6.** Integration test `forge_to_self_accept_succeeds` passes: a consistent eligible tick yields `forge_block → self_accept(...) == Ok(AcceptedBlock)` (first in-process `ForgeSucceeded`).
- [ ] **CE-V-7.** `CN-FORGE-03` flips to `enforced` with populated `tests` + `ci_script`; `CN-FORGE-01` + `DC-CONS-18` get `strengthened_in += "PHASE4-N-V"`.
- [ ] **CE-V-8.** `cargo test --workspace` clean (no regressions); `forge_handler_variants.rs` still green (its `full_stake…rejects` case now reaches `self_accept` rather than the placeholder-decode `Other`, but still returns `ForgeFailed` against its empty pool_distr — confirm it stays green or update its tolerance honestly).

> No human review may substitute for these checks.

## §5 TCB color map (FC/IS partition)

- **BLUE (new):** `ade_codec::encode_block_envelope` (`crates/ade_codec/src/cbor/envelope.rs`).
- **BLUE (modified):** `ade_ledger::producer::forge::forge_block` (`crates/ade_ledger/src/producer/forge.rs`).
- **BLUE (unchanged, consumed):** `ade_codec::cbor::envelope::decode_block_envelope`, `ade_ledger::block_validity::header_input::decode_block`, `ade_codec::conway::decode_conway_block`, `ade_ledger::producer::self_accept::self_accept`, `ade_ledger::block_body_hash`, `ade_types::shelley::block::ShelleyBlock::ade_encode` (still emits the bare inner block).
- **GREEN / RED:** none new. Tests live in `ade_codec` (S1) + `ade_ledger` (S2/S3); one CI gate. No clock/IO/HashMap/float introduced.

Color resolved for every module — no open color question.

## §6 Hard prohibitions (slices inherit)

- **No second block serializer.** Block-envelope encoding lives only in
  `ade_codec::encode_block_envelope`; `forge_block` must call it, not hand-roll
  an envelope.
- **No bare-block forge output.** `ForgedBlock.bytes` must be the enveloped
  `[era, block]` form (CI-gated).
- **No body-bucket re-encoding.** The envelope wraps the existing
  `ShelleyBlock::ade_encode` output; the preserved body buckets and the
  body-hash recipe (`DC-CONS-16` / `DC-CONS-18`) are untouched.
- **No faked `ForgeSucceeded`.** S3's success must come from a real
  `self_accept == Ok`; if a deeper validation mismatch blocks it, root-cause +
  fix in-slice or halt and re-scope — do not loosen the assertion.
- **No serve-side wire-wrap / tag-24 in N-V** (deferred follow-on).
- **No WAL/persistence** (N-U).
- **No new BLUE authority beyond the envelope encoder** — `forge_block`'s
  existing pipeline (leader-check, opcert, body buckets, KES) is unchanged
  except the final envelope wrap; any header-validation binding fix in S3 is a
  correction to an existing authority, not a new one (record precisely in the
  S3 slice doc).

## §7 Replay obligations

- `DC-CONS-18` **strengthened**: the forge transcript now includes the
  envelope; forge↔decode round-trip is a replay-equivalence property
  (R2: `decode_block(forge_block(tick))` structurally stable; R1:
  `forge_block(tick).bytes` byte-identical — carried).
- **No new on-disk replay corpus** — S1 reuses `ConwayValidityCorpus` for the
  era-tag pin.
- **No new canonical types** — `encode_block_envelope` returns `Vec<u8>`;
  `ForgedBlock` shape unchanged.

## §8 Open issues / proof obligations

- **OQ1 (era tag byte) — settled in S1.** Conway discriminant = 7
  (`CardanoEra::Conway = 7`); pinned in CE-V-3 against a real corpus block.
- **OQ4 (failures behind decode) — S3 proof obligation.** After the envelope
  fix, whether a consistent eligible tick reaches `ForgeSucceeded` depends on
  header validation (VRF-cert keyhash binding, nonce, body_size — note
  `body_size` is informational per `header_input.rs`, not checked). S3
  root-causes + fixes any remaining binding mismatch; if it needs BLUE work
  beyond an in-slice correction, halt and re-scope (do not balloon or fake).
- **OQ5 (other consumers) — confirmed in S2.** No code path may rely on the
  bare-block `forge_block` output. Known consumers: `produce_mode`
  (run_real_forge decode + served-chain `push_atomic` decode) and
  `forge_handler_variants` — all decode, so all are *fixed* by the envelope,
  not broken. Any test asserting `ForgedBlock.bytes` byte-shape is updated.

## §9 References
- Root cause: PHASE4-N-T `CLOSURE.md` §"Honest residual"; reproduced during N-T S5 Tier-A.
- Existing surfaces: `decode_block_envelope` (envelope.rs:29,45–48), `decode_block` (header_input.rs:81), `forge_block` (forge.rs:142), `self_accept` (self_accept.rs:66), `ConwayValidityCorpus` (`ade_testkit::validity`).
- Doctrine: [[feedback-real-interop-finds-codec-bugs]] (synthetic round-trips miss wire-format bugs — this is the in-house analogue: forge↔decode was never round-tripped), [[feedback-proof-discipline]] (OQ1/OQ4/OQ5 are obligations), [[feedback-codec-closed-grammar]] (one closed envelope grammar, both directions).

## §10 Authority reminder
Planning aid only. Normative documents + CI enforcement win.
