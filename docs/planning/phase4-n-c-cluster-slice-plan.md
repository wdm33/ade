# Cluster/Slice Plan — PHASE4-N-C Block Production Closure

**Status**: cluster-plan phase complete; awaiting `/cluster-doc`
**HEAD pin**: `96d043c`
**Date**: 2026-05-25
**Source**: `docs/planning/phase4-n-c-invariants.md`

## Cluster Index (Dependency Order)

1. **PHASE4-N-C — Block Production Closure** — primary invariant: forged
   blocks are deterministic over a canonical `ProducerTick` (BLUE),
   pass Ade's N-B+B1 validators before broadcast (self-acceptance),
   and are accepted by cardano-node when delivered via N2N
   (cross-impl).

## PHASE4-N-C — Block Production Closure

- **Primary invariant**: a forged block is byte-deterministic over a
  canonical `ProducerTick` (BLUE), passes Ade's own N-B+B1 validators
  before broadcast (self-acceptance), and is accepted by cardano-node
  end-to-end. Private-key custody stays in RED; BLUE consumes signed
  artifacts only.

- **TCB partition**:
  - **BLUE**: `ade_core::consensus::{forge, opcert_validate,
    self_accept, producer_state}` (new); `ade_codec::shelley::block`
    body-encoder unification (existing module, hardened);
    `ade_crypto::{vrf,kes,opcert}::verify_*` (existing, unchanged).
  - **GREEN**: `ade_runtime::producer::tick_assembler` (composes
    canonical `ProducerTick` from RED outputs).
  - **RED**: `ade_runtime::producer::{signing, keys, scheduler,
    broadcast}` (new); `ade_core_interop::bin::live_block_production_session`
    (new evidence binary).

- **Cluster Exit Criteria**:
  - **CE-N-C-1** — RED signing transcript equivalence:
    `vrf_prove`, `kes_sign`, `kes_evolve` produce byte-identical
    outputs to cardano-node reference vectors and verify under
    existing `verify_*` paths. Forbidden states (period overflow,
    evolution backwards) reject with structured errors.
    *(Flips DC-CRYPTO-03/04/05 + OP-OPS-04 to `enforced`.)*
  - **CE-N-C-2** — OpCert validation + counter monotonicity (BLUE):
    `opcert_validate` rejects counter regression, period mismatch,
    and malformed CBOR with closed error shape; encoder produces
    cardano-cli-byte-identical output for the canonical fixture set;
    opcert decoder/encoder is the single producer-side authority.
    *(Flips DC-CONS-11/12 to `enforced`.)*
  - **CE-N-C-3** — Forge purity + replay byte-equality + leader-check
    gate + tx-admissibility (BLUE): given a canonical `ProducerTick`,
    `forge_block` is pure, replay-equivalent across two runs, refuses
    non-leader ticks, and refuses any tick whose tx-set is not a
    prefix of `mempool::admit`'s canonical accumulating order.
    *(Flips DC-CONS-13/14/15 + DC-LEDGER-12 to `enforced`.)*
  - **CE-N-C-4** — Body-hash parity via validator-shared encoder
    (BLUE): the producer's forged `body_hash` equals the validator's
    recomputed `body_hash` over the same canonical body bytes; no
    encoder bifurcation. CI gate forbids any per-producer body
    encoder.
    *(Flips DC-CONS-16 to `enforced`.)*
  - **CE-N-C-5** — Self-acceptance bridge (BLUE→RED gate):
    `self_accept` wraps the N-B header validator + B1 body validator;
    RED `broadcast` cannot consume forged bytes that fail self-accept;
    failure halts the producer deterministically.
    *(Flips CN-CONS-07 to `enforced`.)*
  - **CE-N-C-6** — Producer scheduler + tick assembler + broadcast
    handoff (RED+GREEN integration): deterministic slot loop drives
    RED signing → GREEN tick assembly → BLUE forge → BLUE self_accept
    → RED broadcast queue; full path completes within the slot
    deadline on the reference hardware fixture; non-leader slots are
    silent; self-accept failures halt cleanly.
    *(Flips OP-OPS-05 to `enforced`.)*
  - **CE-N-C-7** — Mechanical cross-impl adapter (CI gate): an offline
    adapter test in CI takes a captured `ProducerTick` corpus, runs
    Ade's forge, and verifies the bytes round-trip through
    `ade_codec`'s parser and through
    `ade_core::consensus::header_validate` exactly as an
    oracle-captured block would.
    *(Flips CN-CONS-06 mechanical half to `enforced`.)*
  - **CE-N-C-8** — Operator-action live evidence: sustained-window
    `live_block_production_session` against a private cardano-node
    produces `CE-N-C-LIVE_<date>.log` showing cardano-node accepting
    at least one Ade-forged block via N2N. Marked
    `blocked_until_operator_stake_available` if testnet SPO
    registration is not yet provisioned at cluster close.
    *(Flips CN-CONS-06 live half to `enforced` / conditionally enforced.)*

- **Slices**:

  - **N-C-S1** — RED signing primitives + cardano-cli key loading
    Invariant: VRF prove + KES sign + KES evolve produce byte-identical
    outputs to reference; private-key custody RED-confined;
    operator-supplied `*.skey` files load into RED in-memory secrets
    with zeroize-on-drop.
    Addresses: **CE-N-C-1**.
    TCB: **RED** (`ade_runtime::producer::{signing,keys}`), with a
    thin GREEN test harness in `ade_testkit::producer::reference_vectors`.

  - **N-C-S2** — BLUE opcert validate + counter monotonicity +
    closed-grammar encoder parity
    Invariant: `opcert_validate(opcert, cold_vk, expected_period,
    prev_counter)` rejects regression/repeat/mismatch; CBOR encoder
    bytes match cardano-cli's `issue-op-cert` output for the
    canonical fixture set; opcert decoder/encoder is the single
    producer-side authority.
    Addresses: **CE-N-C-2**.
    TCB: **BLUE** (`ade_core::consensus::opcert_validate`;
    `ade_codec::shelley::block` opcert encoder hardened).

  - **N-C-S3** — BLUE forge core: `ProducerTick`, leader-check gate,
    tx-admissibility prefix, purity
    Invariant: `forge_block(tick) -> Result<(ForgedBlock,
    ForgeEffects), ForgeError>` is pure; non-leader tick is
    `ForgeError::NotLeader`; tx-set that isn't a prefix of
    `mempool::admit`'s accumulating order is
    `ForgeError::TxSetNotAdmissiblePrefix`; replay over a captured
    tick stream is byte-identical.
    Addresses: **CE-N-C-3**.
    TCB: **BLUE** (`ade_core::consensus::{forge, producer_state}`;
    canonical types `ProducerTick`, `ForgedBlock`, `ForgeError`,
    `ForgeEffects`).

  - **N-C-S4** — BLUE body-hash parity via the validator-shared
    canonical body encoder
    Invariant: `forge_block` computes
    `header.body_hash = blake2b_256(encode_block_body(...))` where
    `encode_block_body` is the same function the validator path uses
    to recompute `body_hash` during N-B validation. CI gate enforces
    "no producer-private body encoder."
    Addresses: **CE-N-C-4**.
    TCB: **BLUE** (`ade_codec::shelley::block::encode_block_body`
    lifted into a single authority;
    `ci/ci_check_no_producer_body_encoder.sh`).

  - **N-C-S5** — BLUE self-acceptance gate before broadcast
    Invariant: `self_accept(forged_bytes, state, pparams)` runs the
    existing N-B header validator + B1 body validator paths against
    the freshly forged bytes; returns
    `AcceptVerdict::{Accept, Reject(reason)}` with the same reject
    reasons those validators already emit. RED `broadcast` is
    callable only with `AcceptVerdict::Accept` (type-level gate).
    Addresses: **CE-N-C-5**.
    TCB: **BLUE** (`ade_core::consensus::self_accept`).

  - **N-C-S6** — RED scheduler + GREEN tick-assembler + RED broadcast
    handoff
    Invariant: deterministic slot loop drives RED signing →
    GREEN `tick_assembler` → BLUE forge → BLUE self_accept → RED
    broadcast queue; full path completes within the slot deadline on
    the reference hardware fixture; non-leader slots are silent;
    self-accept failures halt cleanly. Tick assembler is observably
    deterministic (replayable from captured RED outputs).
    Addresses: **CE-N-C-6**.
    TCB: **RED** scheduler + broadcast, **GREEN** tick_assembler.

  - **N-C-S7** — Mechanical cross-impl adapter + operator-action
    live-evidence binary
    Invariant: a CI test exercises the full Ade producer path over a
    captured `ProducerTick` corpus and verifies forged bytes are
    accepted by Ade's own decoder + header validator with no
    producer-private fix-ups; a separate `live_block_production_session`
    binary in `ade_core_interop::bin` runs against a private
    cardano-node and captures `CE-N-C-LIVE_<date>.log`. Live half is
    conditional via `blocked_until_operator_stake_available`.
    Addresses: **CE-N-C-7**, **CE-N-C-8**.
    TCB: **RED** (`ade_core_interop::bin::live_block_production_session`);
    mechanical adapter is in `ade_testkit::producer::cross_impl_adapter`.

- **Replay obligations**:
  - New canonical replay corpus: ordered `Vec<ProducerTick>` paired
    with expected `Vec<ForgedBlockBytes>`. Lives at
    `crates/ade_testkit/src/producer/replay.rs` with fixtures under
    `crates/ade_testkit/fixtures/producer/`.
  - `T-DET-01` strengthened by PHASE4-N-C (new authoritative-state
    surface: forged block bytes from canonical inputs).
  - `T-ENC-01` strengthened by PHASE4-N-C (new hash-critical byte
    path: `body_hash` over validator-shared encoder).
  - Replay corpus excludes private-key material entirely (replay
    drives forge with captured signed artifacts).

- **Forbidden states across the cluster** (cross-slice invariants):
  - Non-leader tick reaches `forge_block` → `ForgeError::NotLeader`.
  - Counter regression in `opcert_validate` →
    `OpCertError::CounterRegression`.
  - Body-hash mismatch between producer-computed and
    validator-recomputed → `ForgeError::BodyHashBifurcation` (caught
    by S4's CI gate).
  - Self-accept failure reaches RED `broadcast` → compile-time
    impossible (type-level gate).
  - Replay corpus carrying private-key bytes → forbidden by
    `ci/ci_check_no_private_keys_in_corpus.sh`.

- **Live-evidence conditionality**: CE-N-C-8's
  `blocked_until_operator_stake_available` is the explicit conditional
  closure pattern (not deferral). If testnet SPO stake is not
  provisioned at cluster close, the mechanical half (CE-N-C-7) plus
  all enforced invariants close the cluster; CE-N-C-8 is recorded in
  the closure with the conditional status and a re-open obligation
  tied to operator-stake provisioning.

## CE coverage matrix

| CE | Slice | Registry IDs flipped to `enforced` on close |
|----|----|----|
| CE-N-C-1 | S1 | DC-CRYPTO-03, DC-CRYPTO-04, DC-CRYPTO-05, OP-OPS-04 |
| CE-N-C-2 | S2 | DC-CONS-11, DC-CONS-12 |
| CE-N-C-3 | S3 | DC-CONS-13, DC-CONS-14, DC-CONS-15, DC-LEDGER-12 |
| CE-N-C-4 | S4 | DC-CONS-16 |
| CE-N-C-5 | S5 | CN-CONS-07 |
| CE-N-C-6 | S6 | OP-OPS-05 |
| CE-N-C-7 | S7 | CN-CONS-06 (mechanical half) |
| CE-N-C-8 | S7 | CN-CONS-06 (live half — conditional) |

All 14 N-C registry entries reachable. No carry-forward.
