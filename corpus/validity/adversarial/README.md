# Adversarial / negative agreement corpus (B1-S7, CE-B1-4)

This directory holds **no committed CBOR blobs**. The adversarial blocks are
generated deterministically at test time from the committed real Conway-576
positive corpus (`corpus/validity/conway_epoch576/`) by the named mutators in
`ade_testkit::validity::adversarial`. The mutator code plus this README are the
provenance.

Each mutation takes a real corpus block's `[era, inner_block]` envelope, applies
a single targeted corruption that violates one spec rule, and is judged by the
BLUE `block_validity` authority using the SAME consensus recipe the positive
corpus passes (eta0(576), epoch-576 ledger with `track_utxo = false`, the
corpus pool-distribution view, the mainnet Conway era schedule).

## Load-bearing invariant (CE-B1-4 / DC-LEDGER-02)

**No mutation ever yields `Valid`.** A `Valid` here would be a real fail-open
bug. Verified for all 6 mutations × all 14 corpus blocks by
`block_validity_adversarial_corpus::no_mutation_is_ever_valid`.

## Mutation set

| # | Mutation | Spec rule violated | `block_validity` reject (observed) |
|---|---|---|---|
| M1 | Truncate the header VRF proof (80 → 79 bytes) | fixed-size VRF proof | `MalformedField` (VrfProof, via `expect_size`) — all 14 blocks |
| M2 | Tamper the header VRF vkey (`blake2b256(vkey) != registered keyhash`) | VRF key binding | `HeaderInvalid` (`VrfKeyhashMismatch`) — all 14 blocks |
| M3 | Flip a byte in the KES signature | KES authenticity | `HeaderInvalid` (`KesInvalid`) — all 14 blocks |
| M4 | Set the header slot far beyond the EraSchedule horizon | forecast horizon | `HeaderInvalid` (`OutsideForecastRange`) — all 14 blocks |
| M5 | Flip a body byte (header `body_hash` unchanged) | header↔body binding | `BodyHashMismatch` on the base block (block 0); on the other blocks the flip lands in a `tx_bodies` segment whose corruption is caught structurally as `BodyInvalid` — still fail-closed (the §13 "different but still fail-closed class" case). Never `Valid`. |
| M6 | Forge a witness (fabricated `[vkey, sig]`), header `body_hash` patched to the mutated body | Ed25519 witness verification | `HeaderInvalid` (`KesInvalid`) — see finding below. Never `Valid`. — all 14 blocks |

The class-mapping assertion (`each_mutation_maps_to_expected_class`) is anchored
on the base block (block 0): M1 `MalformedField`, M2/M3/M4/M6 `HeaderInvalid`,
M5 `BodyHashMismatch`.

## Finding — M6 forged spend is fenced by the header, not the body authority

M6's slice premise was: patch the header `body_hash` to the forged body so the
block passes the body-hash gate (step 4) and reaches body validation (step 5),
where a forged Ed25519 witness is rejected fail-closed → `BodyInvalid`.

In this node that premise does not hold, because:

1. The **KES signature signs the header body**, which **includes the
   `body_hash` field** (`ade_core::consensus::kes_check::verify_header_kes`
   over `header_body_bytes`).
2. The whole header pipeline — including the KES check (step 7b) — runs **before**
   the body-hash gate and body validation in `block_validity`
   (`ade_ledger::block_validity::transition`).

So patching `body_hash` invalidates the KES signature first, and the forged
spend is rejected fail-closed at the **header** (`HeaderInvalid` / `KesInvalid`).
This is **correct, secure behavior**: the header commits to the body, so a
forged spend cannot pass `block_validity` by any byte mutation of a real block.
A forged spend that does *not* patch `body_hash` is exactly M5
(`BodyHashMismatch`). Either way: never `Valid`.

### Separate, deeper note (out of B1-S7 scope) — Conway body-level witness verification

Reaching the body authority's own forged-witness check requires *bypassing* the
header (which a real chain never does). When the inner block is fed directly to
`ade_ledger::rules::apply_block_with_verdicts` for a Conway block, the fabricated
vkey witness is **not** rejected — `verify_shelley_witnesses`
(`ade_ledger::shelley`, fail-closed via `Ed25519VerificationKey::from_bytes` +
`verify_ed25519`) is wired only on the **Shelley** UTxO-application path, and the
Conway path runs structural validation (and, under `track_utxo`, phase-1 /
Plutus dispatch) without per-tx Ed25519 vkey-witness verification. This gap is
**not reachable through `block_validity`** (the header fences it), so it is not a
CE-B1-4 false-accept, but it is a genuine pre-release finding about the Conway
body authority and is recorded here for the follow-up track (it is out of scope
for B1-S7, which adds no new validation logic beyond the agreement corpus).

## Determinism

The mutators are pure byte transforms; the per-(mutation, block) verdict surface
is byte-identical across runs, asserted by `adversarial_replays_identically`
(`T-DET-01`).
