# Invariant Slice — PHASE4-N-F-G-J S3: Header-position validation + genesis-successor forge

> **Status:** Planning artifact (non-normative). Normative authority is the registry + CI.

## §2 Slice Header

- **Slice:** PHASE4-N-F-G-J S3 — Header-position validation (`block_number 0 ⟺ Genesis`) + forge/KES-pre-image emit `Genesis` for block 0; producer `prev_hash` migrates `Hash32 → PrevHash`.
- **Cluster:** PHASE4-N-F-G-J — Genesis-successor block correctness (`c167cd41`).
- **Status:** Merged (`0c1939a1`).
- **Cluster Exit Criteria addressed — CE-G-J-3 (verbatim):** "header-position validation rejects `Block` at `block_number 0` and `Genesis` at `block_number > 0`; the forge emits `PrevHash::Genesis` for the first block (hermetic); the all-zero parent is gone." *(CE-G-J-1/2 already met; CE-G-J-4 node-spine reachability and CE-G-J-5 C1 rehearsal out of scope.)*

## §3 Slice Dependencies

- **S2** (`CN-WIRE-09` codec portion, enforced — `3b24c572`) — **hard dependency**: `PrevHash = Genesis | Block(Hash32)` and the position-**blind** codec must exist before this slice can add the position-**aware** validator and migrate the producer representation.
- **S1** (`CN-NODE-04`, enforced) — independent; untouched.
- **S4** depends on this slice (node-spine reachability needs the genesis forge + position rule first).

## §4 Intent (invariant impact)

Make it **impossible for Ade to finalize a block whose `block_number`/`prev_hash` pair is position-illegal**: a genesis-successor (`block_number 0`) with anything but `PrevHash::Genesis`, or a non-genesis block (`block_number > 0`) with `PrevHash::Genesis`. The rule is enforced by the BLUE validator at the single `decode_block` chokepoint every validation path flows through; the producer is migrated so the genesis predecessor is structurally `Genesis` (no all-zero `Hash32` stand-in survives anywhere on the forge path). This fulfils the position-aware clause `CN-WIRE-09`'s statement explicitly defers to "the sibling forge/validation slice (CE-G-J-3, S3)."

## §5 Scope / What is built

1. **`ade_ledger::block_validity` (validator — the position rule, single authority).**
   - New pure, total `check_header_position(block_number: u64, prev_hash: &PrevHash) -> Result<(), HeaderPositionError>`: `0 ⟺ Genesis`, `>0 ⟺ Block`. No `block_number`/`prev_hash` coupling logic exists anywhere else.
   - `decode_block` (`header_input.rs:81`) calls it after decoding `hb` (it already holds both `hb.block_number` and the new `hb.prev_hash`), before building `HeaderInput` — fail-closed, ahead of the header authority.
   - New closed `BlockValidityError::HeaderPositionInvalid { block_number, prev_is_genesis }` (`verdict.rs:43`), mapped by `class()` to the **existing** `BlockRejectClass::HeaderInvalid` (no new oracle-comparable class; the precise reason rides the debug-only structured error).
2. **`ade_ledger::producer::forge` (`forge.rs:293`).** Emits the tick's `PrevHash` directly (`prev_hash: tick.prev_hash.clone()`) — the S2 `PrevHash::Block(...)` wrap is removed; the genesis decision is no longer the forge's to make (it carries what the migrated tick holds). The forged block-0 thus serializes `prev_hash` as CBOR `null`.
3. **`ade_ledger::block_validity::unsigned_header_pre_image` (`:89,:101`).** Signature `prev_hash: Hash32 → PrevHash`; body uses it directly (no re-wrap). The KES-pre-image and the forged header now draw the **same** `PrevHash` from one source, so the KES-signed bytes equal the forged null-prev header bytes for block 0 (resolves OQ-A's deep form; preserves `CN-PREIMAGE-FIXTURE-01`).
4. **`ade_ledger::producer::state::ProducerTick.prev_hash` (`state.rs:72`).** `Hash32 → PrevHash`.
5. **`ade_runtime::producer` (GREEN representation migration).** `TickInputs.prev_hash` (`tick_assembler.rs:43`) `Hash32 → PrevHash`; `assemble_tick` copy (`:99`) unchanged in shape. `ChainEvolution::prev_hash()` (`chain_evolution.rs:131`) returns `PrevHash`: **`tip None → PrevHash::Genesis`** (the all-zero source is deleted), `Some(t) → PrevHash::Block(Hash32(t.block_hash))`. `scheduler.rs:343` test fixture migrated.
6. **`ade_node` (RED call-site migration only).** `ForgeRequestContext.prev_hash` (`produce_mode.rs:623`) `Hash32 → PrevHash`; the two constructors — produce path (`:1101` `evo.prev_hash()`, already `PrevHash`) and node path (`node_sync.rs:573` `PrevHash::Block(selected_tip.hash.clone())`, the existing-parent case) — and the pre-image/TickInputs uses (`:870,:914`) carry it through.

**Out of scope (S4):** node-spine genesis reachability (both tips `None` → forge block 0). `node_sync.rs:555`'s cold-start `unwrap_or(1)` block-number convention (vs `chain_evolution`'s `0`) is a node-mode reachability concern — S3 leaves node_sync's `selected_tip`-present (`Block`) path only and does **not** wire its genesis branch. **Out of scope (always):** chain-linkage (does `prev_hash` equal the *actual* parent hash) — S3 enforces only the intra-header position rule, not chain continuity.

## §6 Execution Boundary (TCB color)

- **BLUE** — `ade_ledger::block_validity` (`check_header_position`, `decode_block` call, new `HeaderPositionInvalid` variant); `ade_ledger::producer::forge` (pass-through); `ade_ledger::block_validity::unsigned_header_pre_image` (pass-through); `ade_ledger::producer::state` (`ProducerTick.prev_hash` type). The position rule **and** the canonical encode/decode of `prev_hash` are the authoritative truth.
- **GREEN** — `ade_runtime::producer::{chain_evolution, tick_assembler, scheduler}`: deterministic glue that threads the value. **Color resolution (refines the cluster doc §5 table, which tags `chain_evolution` BLUE):** `chain_evolution` is GREEN deterministic glue — it derives the candidate predecessor from already-authoritative tip state, but it is **not** the final authority on header legality. The BLUE codec defines the byte representation and the BLUE `decode_block → check_header_position` rule is the final admission authority. A wrong GREEN proposal must be impossible to finalize.
- **RED** — `ade_node::{produce_mode, node_sync}`: shell call-site type plumbing only; no decision logic added.
- **Unchanged** — `ade_codec` (codec stays position-blind, S2); `ade_types` (`PrevHash` already defined, S2); `ade_network` Point/Tip codec (OQ-B; `null` stays header_body-only).

## §7 Invariants Preserved

- **`CN-WIRE-09` (codec portion)** — the byte codec stays **position-blind**; S3 adds the position-aware check in the validator, never in `ade_codec`.
- **`CN-WIRE-08`** — tag-24 envelope unchanged; `Block` case encodes byte-identical `hash32`.
- **`DC-FORGE-01`** — leader-check / forge determinism: two-run byte-identical (`advance_two_runs_byte_identical`, `run_real_forge_is_byte_identical_across_two_runs` still pass).
- **`CN-FORGE-01`** — forge → `self_accept` closed transition; the genesis block is admitted by the same gate (no new acceptance path).
- **`CN-FORGE-03`** — forge output decodes via `decode_block` (now also subject to the position rule); `forge_block_output_decodes_via_decode_block` still passes.
- **`CN-FORGE-04`** — Praos VRF construction untouched.
- **`CN-PREIMAGE-FIXTURE-01`** — pre-image bytes still byte-match `decode_block`'s extracted `header_body_bytes`, now including genesis headers.
- **`CN-CONS-04` / `DC-CONS-18`** — body-hash binding unchanged.
- **`DC-NODE-06` / `DC-NODE-07`** — self-accept handoff / single shared serve unchanged.
- **Real Conway corpus round-trips byte-identically** (the `Block` path) — replay equivalence for every existing block.
- **Point/Tip `hash32` grammar (OQ-B)** — `null` never leaks out of `ShelleyHeaderBody.prev_hash`.

## §8 Invariants Strengthened

**`CN-WIRE-09`** — the position-aware clause already in its `statement` ("block_number 0 requires Genesis; block_number > 0 requires Block … enforced by the sibling forge/validation slice (CE-G-J-3, S3)") becomes **mechanically enforced**:

- BLUE validator rejects `Block@0` and `Genesis@>0` at `decode_block`;
- forge + KES-pre-image emit `Genesis` for block 0;
- producer `prev_hash` migrates `Hash32 → PrevHash`, deleting the all-zero stand-in at its source.

**Registry action:** append the S3 tests (§11/§12) to `CN-WIRE-09.tests`; append the extended gate to `CN-WIRE-09.ci_script`; extend `code_locus` to the validator position rule + the migrated forge/pre-image/producer sites; add an `evidence_notes` line recording S3 closed the position clause. **No new rule** (the statement already owns the clause), and **no `strengthened_in` self-bump** (`introduced_in` is already this cluster). `DC-FORGE-01` is *reused/preserved*, not strengthened.

## §9 Open questions resolved in this slice

- **OQ-C → the position rule lives in `ade_ledger::block_validity`** (`check_header_position`, called by `decode_block`) — the single chokepoint shared by receive-side `block_validity`, `self_accept`, `ChainEvolution::advance`, and the forge self-test. **Not** a reshape of `ade_core::HeaderInput`/`ValidatedHeaderSummary` (which carry no `prev_hash` today and would force every constructor + the network path to change). `ade_codec` stays position-blind; `decode_block` already holds both `block_number` and `prev_hash`, making it the single BLUE chokepoint for the intra-header position rule — it rejects `Block@0` / `Genesis@>0` before the header authority runs.
- **OQ-A (deep form) → resolved by construction:** forge header and KES-pre-image carry one `PrevHash` from one source ⇒ KES-signed bytes == forged null-prev header bytes for block 0; the forged genesis block self-accepts.

## §11 Replay / Crash / Epoch Validation

- **Forged-block-0 self-consistency (new):** `forge_block_zero_self_consistent_through_decode_block` — a `block_number 0` tick forges a block whose `decode_block` yields `PrevHash::Genesis` and passes `check_header_position`.
- **KES-pre-image == forged header body bytes for block 0 (new, deepest):** `forged_block_zero_kes_preimage_equals_decoded_header_body_bytes` — for a `block_number 0` forge, the KES pre-image bytes are byte-identical to the decoded `ShelleyHeaderBody` bytes, including `PrevHash::Genesis` encoded as CBOR `null` (`0xf6`).
- **Genesis null round-trip (new):** `forge_block_number_zero_emits_genesis_prev_hash` (header `prev_hash == Genesis`, encoded `0xf6`) + `pre_image_block_zero_emits_genesis_prev` (pre-image null).
- **No regression (Block path byte-identity):** `forge_nonzero_block_emits_block_prev_byte_identical` + `pre_image_nonzero_block_prev_byte_identical` (encoded bytes equal pre-S3 output); the existing corpus suites (`self_accept_*`, `advance_*`, `block_validity_*`) stay green.
- **Determinism preserved:** existing `advance_two_runs_byte_identical` / `run_real_forge_is_byte_identical_across_two_runs` still pass across the migration.
- **Crash/epoch:** none — no WAL/checkpoint/epoch-boundary change.

## §12 Mechanical Acceptance Criteria

Complete only when all pass in CI:

- [ ] `check_header_position` unit tests (BLUE): `header_position_zero_requires_genesis_ok`, `header_position_zero_with_block_is_rejected`, `header_position_nonzero_requires_block_ok`, `header_position_nonzero_with_genesis_is_rejected`.
- [ ] End-to-end adversarial decode rejects: `decode_block_rejects_hash_prev_at_block_number_zero`, `decode_block_rejects_null_prev_at_nonzero_block_number` (both → `Invalid { class: HeaderInvalid }`).
- [ ] Non-regression: `corpus_blocks_pass_header_position_rule` (every real corpus block decodes `Ok`).
- [ ] Forge: `forge_block_number_zero_emits_genesis_prev_hash`, `forge_nonzero_block_emits_block_prev_byte_identical`, `forge_block_zero_self_consistent_through_decode_block`.
- [ ] KES pre-image: `pre_image_block_zero_emits_genesis_prev`, `pre_image_nonzero_block_prev_byte_identical`.
- [ ] `forged_block_zero_kes_preimage_equals_decoded_header_body_bytes`: for a `block_number 0` forge, the KES pre-image bytes are byte-identical to the decoded `ShelleyHeaderBody` bytes, including `PrevHash::Genesis` encoded as CBOR `null`.
- [ ] Producer migration (GREEN): `chain_evolution_prev_hash_genesis_at_cold_start`, `chain_evolution_prev_hash_block_with_tip`.
- [ ] CI gate `ci/ci_check_prevhash_single_wire_authority.sh` **extended**: (c) `check_header_position` defined in exactly one file and referenced by `decode_block`, no parallel position rule; (d) no `prev_hash: Hash32` field remains in `ProducerTick`/`TickInputs`/`ForgeRequestContext`, and `chain_evolution::prev_hash` contains `PrevHash::Genesis` with no `Hash32([0u8; 32])`.
- [ ] `cargo test -p ade_ledger -p ade_runtime -p ade_node` green (unmasked exit code); `cargo fmt --check` + `cargo clippy` clean on the three crates. *(Full unmasked `cargo test --workspace` is the cluster-close gate per `RO-CLOSE-01`; the `ade_testkit` corpus-suite timeout is pre-existing/environmental.)*

## §13 Failure Modes

- **`HeaderPositionInvalid`** — fail-fast: `decode_block` returns `Err` before the header authority runs ⇒ `block_validity` → `Invalid { class: HeaderInvalid }` with input states returned unchanged (no partial mutation, `DC-VAL-05`). On the forge side a mis-built tick surfaces as `SelfAcceptError::Rejected(HeaderPositionInvalid)`. Closed variant, no `String`, byte-stable across runs.

## §14 Hard Prohibitions

Inherits cluster §11 in full. Slice-specific:

- The position rule lives in **exactly one** BLUE function (`check_header_position`), called from `decode_block`; **no** second/parallel position check, and **never** in the `ade_codec` byte codec (it stays position-blind).
- **No all-zero `Hash32` stand-in** for the genesis predecessor survives on the forge path (`chain_evolution::prev_hash` returns `Genesis`, not `Hash32([0u8;32])`); no magic-value / state-dependent nullable stand-in for `prev_hash` anywhere in the authoritative producer path.
- `Block`-case encoding stays **byte-identical `hash32`** (representation migration, not a semantic change to existing blocks).
- **No new `BlockRejectClass`** (reuse `HeaderInvalid`); `null` is never emitted into Point/Tip.
- **No node-spine reachability** wiring (S4); **no** chain-linkage / parent-hash-match check; **no** change to `node_sync.rs` cold-start block-number convention; **no** RO-LIVE flip.

## §15 Explicit Non-Goals

Node-spine first-block reachability (S4); the `chain_evolution`-vs-`node_sync` cold-start block-number reconciliation (S4); chain-linkage validation; a dedicated genesis-mismatch reject class; any C1/preprod execution; any `ProducerTick` field beyond `prev_hash`.
