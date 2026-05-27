# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **89 CI checks** at HEAD (`d6f3399`).
> Reads CODEMAP for the module list and TCB colors; reads the invariant
> registry (`docs/ade-invariant-registry.toml` — **264 entries**) for
> rule IDs; reads the Phase 4 cluster plan
> (`docs/active/phase_4_cluster_plan.md`), the closed N-D / N-A / N-B /
> N-E / N-C / N-G / N-H / N-I / N-J / N-K / N-L / N-L-LIVE / N-M-A /
> N-M-A1.1 / N-M-B / N-M-C / N-M-FRAG / N-M-SCHED / N-M-FOLLOW / N-O /
> N-P / B1 / B2 / B3 / B4 / B5 cluster docs, the OQ5 / COMMITTEE /
> DREP / ENACTMENT-COMMITTEE-FIDELITY / ENACTMENT-COMMITTEE-WRITEBACK /
> PROPOSAL-PROCEDURES-DECODE cluster docs, and the **just-closed
> PHASE4-N-P cluster doc + CLOSURE record**
> (`docs/clusters/completed/PHASE4-N-P/{cluster,CLOSURE}.md`).
>
> **This is the PHASE4-N-P FULL CLOSE refresh (HEAD `d6f3399`).** The
> previous SEAMS regeneration pinned the PHASE4-N-L state at HEAD
> `d62c2bc`. Between that anchor and this one, **ten clusters closed
> in sequence** (N-L-LIVE → N-M-A → N-M-A1.1 → N-M-B → N-M-C →
> N-M-FRAG → N-M-SCHED → N-M-FOLLOW → N-O → N-P). They are surfaced
> here as a roll-up; per-cluster delta narratives live in
> HEAD_DELTAS. This refresh focuses on **PHASE4-N-P** (the most
> recent close) and threads the new BLUE `ade_crypto::kes_sum/`
> module + `KesAlgorithm` trait + `KesParseError` closed grammar +
> `KesSecret::from_blue_signing_key` constructor seam through the
> existing sections.
>
> **THE PHASE4-N-P KEY FULL-CLOSE DELTAS.** PHASE4-N-P closes the
> cardano-cli expanded KES seam. Ade now owns the Sum6KES algorithm
> in BLUE; `cardano-crypto` is demoted to a `#[cfg(test)]` oracle
> for KES. Five new seams ship:
>
> 1. **BLUE `ade_crypto::kes_sum::KesAlgorithm` trait (CN-CRYPTO-N9).**
>    The closed BLUE surface for the Sum_n KES family —
>    `gen_key_kes_from_seed_bytes`, `derive_verification_key`,
>    `update_kes`, `sign_kes`, `verify_kes`,
>    `raw_serialize_signing_key_kes`, `raw_deserialize_signing_key_kes`,
>    `raw_serialize_signature_kes`, `raw_deserialize_signature_kes`,
>    `current_period_of_signing_key`. Implementors today: `Sum0Kes` (=
>    `SingleKes<Ed25519>`) → `Sum1Kes` → … → `Sum6Kes`. Pure; no I/O,
>    no clock, no `HashMap`/`HashSet`, no float, no `String`-bearing
>    error variant. New Sum_n attaches as an internal type-alias step
>    in `ade_crypto::kes_sum::mod.rs`; the trait surface does not
>    change.
> 2. **Closed parse-grammar `KesParseError` (DC-CRYPTO-09).**
>    `ade_crypto::kes_sum::errors::KesParseError` is a closed enum:
>    `WrongPayloadSize { actual }`, `LeafSignKeyAllZero`,
>    `InconsistentSubtreeVkLeft { level }`,
>    `InconsistentSubtreeVkRight { level }`,
>    `LevelOutOfRange { level }`,
>    `InvalidEd25519SignatureLength { actual }`. Carries only
>    non-secret metadata (`u32` / `usize`); no key bytes; no hex; no
>    `String`. Adding a variant requires a new `[[rules]]` entry under
>    family `DC-CRYPTO` and a strengthening of `DC-CRYPTO-09`.
> 3. **608-byte expanded `Sum6KES` ingress (DC-CRYPTO-07).**
>    `load_kes_signing_key_skey` now accepts the cardano-cli envelope
>    type `KesSigningKey_ed25519_kes_2^6` (608-byte expanded payload)
>    by routing through the BLUE deserializer. Wrong-size payloads
>    still fail-close via `KeyLoadError::UnsupportedExpandedKesKeyFormat`
>    (the narrower N-O surface preserved); structural defects fail-close
>    via a new `KeyLoadError::KesParse(KesParseError)` variant. No
>    fallback parser — the BLUE deserializer IS the structural
>    validator. The Ade-native `ade.kes.seed.v1` envelope remains the
>    recommended path.
> 4. **Internal RED-side constructor seam
>    `KesSecret::from_blue_signing_key`.**
>    `pub(super) fn from_blue_signing_key(inner: <Sum6Kes as
>    KesAlgorithm>::SigningKey, current_period: KesPeriod) -> Self`
>    in `ade_runtime::producer::signing` is the RED-side construction
>    site used by the cardano-cli loader after the BLUE deserializer
>    returns. Visibility is `pub(super)` — RED-internal only;
>    `producer::keys` is the sole call site. The Ade-native flow
>    continues to construct via `from_seed_at_period`; this new
>    constructor is the cardano-cli-flow-only path.
> 5. **N9 module rule: no `cardano-crypto` in production code under
>    `crates/ade_crypto/src/**` (KES paths).**
>    `cardano-crypto` is a `#[cfg(test)]` oracle only for KES.
>    Mechanically enforced by **the new
>    `ci/ci_check_kes_sum_compatibility.sh`** (4 guards: corpus +
>    throwaway-comment present; no committed `.skey` envelopes under
>    `crates/ade_crypto/`; `cardano_crypto::kes` imports only inside
>    `#[cfg(test)]` files; `expand_seed` prefix bytes match Haskell
>    `cardano-base` (0x01 / 0x02), not `cardano-crypto` Rust 1.0.8
>    (0x00 / 0x01)). VRF + DSIGN paths remain on upstream
>    `cardano-crypto`; N9 is KES-scoped per the cluster doc.
>
> **A critical Haskell-vs-Rust divergence was surfaced during S4.**
> `cardano-crypto` Rust 1.0.8 uses `expand_seed` prefix bytes
> `0x00 / 0x01`; Haskell `cardano-base` (which drives cardano-cli and
> the on-chain wire format) uses `0x01 / 0x02`. Ade tracks Haskell.
> The S4 cardano-cli ground-truth corpus is the live oracle; two
> divergence-documenting tests assert the asymmetry. Implication: any
> Ade-native `kes.ade.skey` generated before S5 close (pre-`d6f3399`)
> derives a different VK from the same seed under post-S5 signing.
> No real deployments existed at S5 close, so this is documented but
> non-load-bearing.
>
> **No new BLUE chokepoint for header / forge / consensus.** The
> existing chokepoints (`opcert_validate`, `verify_kes`,
> `validate_and_apply_header`, `forge_block`, `block_validity`) are
> reused unchanged. The N-P closure is internal to the cryptographic
> primitive — verification semantics, byte-shape contract, and
> per-period evolution discipline are unchanged.
>
> Counts at this refresh: **+1 CI script** (88 → 89:
> `ci_check_kes_sum_compatibility.sh`) + **1 CI script narrowed**
> (`ci_check_kes_envelope_closed.sh` Guard 2 now requires
> `raw_deserialize_signing_key_kes` in the loader-body — N-O's
> unconditional-fail-close assertion is replaced by the
> accept-via-BLUE-deserializer assertion); **+2 registry rules
> introduced + 2 closed obligations** (`DC-CRYPTO-08` + `DC-CRYPTO-09`
> declared and flipped to `enforced`; `OP-OPS-04.open_obligation` +
> `DC-CRYPTO-07.open_obligation` cleared); **+3 carried rules
> strengthened** (`DC-CRYPTO-03` / `DC-CRYPTO-04` / `DC-CRYPTO-05`
> each gain `strengthened_in += PHASE4-N-P`); **+1 new BLUE module
> tree** (`crates/ade_crypto/src/kes_sum/` — `mod`, `single`, `sum`,
> `hash`, `period`, `errors`, plus `#[cfg(test)]` `tests` and
> `cardano_cli_corpus`); **+1 new RED-internal constructor**
> (`KesSecret::from_blue_signing_key`); **0 new operator-action
> probe binaries**; **0 new external ingress surfaces**.
>
> **Intermediate-cluster roll-up (N-L-LIVE → N-O), surfaced here as
> deltas-since-`d62c2bc` but not folded into the per-cluster prose
> below — see HEAD_DELTAS and per-cluster docs:**
>
> - **N-L-LIVE** (`48cb2bb`): one-slice operator-action close;
>   `RO-LIVE-04` `enforced`. Wire-only live pass captured against
>   local preprod docker peer; sustained handshake + chain-sync
>   tip-following.
> - **N-M-A** (`c7e2a23`): oracle seed importer + `BootstrapAnchor`
>   + Ade-native WAL + replay-equivalence; `CN-SEED-01`,
>   `DC-SEED-01`, `CN-ANCHOR-01`, `DC-ANCHOR-01`,
>   `CN-WAL-01..DC-WAL-03` (8 rules) `enforced`.
> - **N-M-A1.1** (`03d1d24`): reference-script (4 variants) + Byron
>   Base58 seed-import; full preprod UTxO (1.9M entries) imports
>   cleanly. Closes the structural-blocker on RO-LIVE-05.
> - **N-M-B** (`2b9d4f1`): admission orchestrator +
>   `AgreementVerdict` + per-admit WAL + replay-equivalence; 11
>   N-M-B rules `enforced` + 5 strengthenings.
> - **N-M-C** (`8843e20`): live operator pass scaffolding; 11
>   N-M-C rules `enforced` + `DC-EVIDENCE-01`
>   `enforced_scaffolding`.
> - **N-M-FRAG** (`4d3dc98`): session-reducer per-mini-protocol
>   payload reassembly; live admission reaches `block_received`.
>   `CN-SESS-04` + `DC-SESS-06` `enforced`; live wire layer now
>   handles fragmented frames correctly.
> - **N-M-SCHED** (`d8feabb`): `era_schedule.epoch_no` wiring;
>   **`DC-EVIDENCE-01` + `RO-LIVE-05` `enforced`** with committed
>   `BlockAdmitted`+`Agreed` live transcript (peer-hash byte-match
>   at slot 124140368, peer-acceleration via local preprod docker).
> - **N-M-FOLLOW** (`7bed3d5`): wire pump follows the peer's chain
>   (RollForward → block-fetch + IntersectFound emits RequestNext
>   only); op-cert counter rule fixed (equal counter = no-op
>   within KES period). **34 consecutive `BlockAdmitted` with zero
>   divergence** across slots 124137045..124137868.
> - **N-O** (`6eb4fbd`): Ade-native KES key-gen flow shipped
>   (`ade.kes.seed.v1` envelope; `ade_node --mode key_gen_kes`);
>   cardano-cli expanded envelope fail-closed with
>   `KeyLoadError::UnsupportedExpandedKesKeyFormat`. `OP-OPS-04`
>   `enforced`; `DC-CRYPTO-07` `enforced` (fail-closed-always
>   posture, with `open_obligation` until PHASE4-N-P).
> - **N-P** (`d6f3399`) — this refresh's anchor; details below.

Ade is a Cardano block-producing node. Its closure surface is dominated
by two facts:

1. The Cardano protocol fixes wire bytes and hashes for hash-critical
   paths (Tier 1 — must-conform). New work that touches those bytes
   has essentially no degrees of freedom.
2. Everything operator-facing — storage layout, query API, telemetry,
   packaging — is Tier 5: deliberate divergence "in our own image"
   (per `docs/active/CE-79_tier5_addendum.md`).

This document names where the system opens and where it stays closed.

**PHASE4-N-P is fully closed at this HEAD.** The Ade workspace now
owns the Sum6KES algorithm end-to-end: `ade_crypto::kes_sum/` is the
BLUE implementation; `KesSecret.inner` is the Ade-owned signing key;
`ade_runtime`'s `cardano-crypto` declaration no longer carries the
`kes-sum` feature; the cardano-cli 608-byte `KesSigningKey_ed25519_kes_2^6`
envelope is loadable via the BLUE deserializer; and a 3-fixture
cardano-cli ground-truth corpus + a 64-period round-trip test +
4-guard CI script mechanically defend the Haskell-equivalence claim.

**PHASE4-N-O remains fully closed** (carried — Ade-native KES key-gen
flow). **PHASE4-N-M-FOLLOW remains fully closed** (carried —
sustained 34-block live admission). **PHASE4-N-M-SCHED remains
fully closed** (carried — `RO-LIVE-05` `enforced`). **PHASE4-N-M-A**
through **N-M-FRAG remain fully closed** (carried). **PHASE4-N-L
remains fully closed** (carried; the session reducer + handshake
driver + mux pump + n2n dialer + keep-alive session shipped by N-L
underlie every post-N-L live-evidence cluster). **PHASE4-N-L-LIVE**
through **PHASE4-N-K, N-J, N-I, N-H, N-G, N-C, N-E, B1..B5,
PROPOSAL-PROCEDURES-DECODE, OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-***
— all remain closed (carried).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative
> pipelines. At HEAD there remain **eight** fully-wired *external*
> ingress surfaces (block bytes, Plutus script bytes, snapshot bytes,
> Ouroboros mux frames, genesis JSON bundles, chain-selector stream
> inputs, the N-E wire-level mempool ingress, and the N-H receive-side
> N2N peer ingress).
>
> **PHASE4-N-P does not add a new external surface.** It does, however,
> add **a new internal RED→BLUE reduction**: cardano-cli's 608-byte
> `KesSigningKey_ed25519_kes_2^6` envelope payload now reduces to a
> typed `<Sum6Kes as KesAlgorithm>::SigningKey` via the BLUE
> deserializer before any RED custody wrapping happens. The reduction
> is detailed in §2 below and surfaces here as **an additional
> reduction step inside the existing key-loader path** rather than a
> new ingress surface.

### Surface: cardano-cli `.skey` envelope ingress (extended in PHASE4-N-P — DC-CRYPTO-07 + DC-CRYPTO-09)

```
Surface: cardano-cli text-envelope `.skey` files (kes / vrf / cold)
Reduces to: (kes flavor) <Sum6Kes as KesAlgorithm>::SigningKey
            (BLUE-owned; via raw_deserialize_signing_key_kes)
         then: KesSecret (RED custody wrapper via from_blue_signing_key)
         (vrf flavor + cold flavor) unchanged from N-C / N-O carriage
Pipeline (fixed step ordering — no reorder, no shortcut):
  1. RED ade_runtime::producer::keys::read_envelope_payload — text
     envelope parse; type-field equality check
     ("KesSigningKey_ed25519_kes_2^6" / "VrfSigningKey_PraosVrf" /
     "StakePoolSigningKey_ed25519").
  2. (kes only — NEW step in N-P) RED size gate:
     payload.len() == Sum6Kes::SIGNING_KEY_SIZE (608) — else
     KeyLoadError::UnsupportedExpandedKesKeyFormat.
  3. (kes only — NEW step in N-P) BLUE structural validate +
     deserialize: Sum6Kes::raw_deserialize_signing_key_kes(&payload).
     Returns the typed SigningKey OR a closed KesParseError variant.
     KesParseError is converted to KeyLoadError::KesParse(_).
  4. (kes only — NEW step in N-P) BLUE period inference:
     current_period = Sum6Kes::current_period_of_signing_key(&inner).
     Total function over a structurally-valid SigningKey — never panics.
  5. (kes only — NEW step in N-P) RED custody wrap:
     KesSecret::from_blue_signing_key(inner, KesPeriod(current_period)).
     pub(super) — RED-internal; producer::keys is the sole call site.
Cross-surface state sharing: none. Each envelope load is a single-shot
  operation; no shared caches.
```

**Rule (NEW in N-P).** The cardano-cli KES `.skey` flavor has a
SINGLE reduction chokepoint to canonical BLUE bytes —
`Sum6Kes::raw_deserialize_signing_key_kes`. The RED loader does not
parse the 608-byte tree-shape; it hands the bytes to BLUE and consumes
the typed result. The constructor seam
`KesSecret::from_blue_signing_key` is the ONLY RED-side construction
path for a `KesSecret` whose `inner` came from a cardano-cli envelope
(the Ade-native `ade.kes.seed.v1` path uses `from_seed_at_period`
unchanged).

— **not** by adding a parallel `raw_deserialize_signing_key_kes`
outside `ade_crypto::kes_sum::sum`, **not** by adding a fallback
parser for malformed 608-byte payloads, **not** by widening
`KesSecret::from_blue_signing_key` past `pub(super)`, **not** by
exposing `<Sum6Kes as KesAlgorithm>::SigningKey` bytes through a
public byte accessor on `KesSecret`.

### Surfaces carried unchanged from prior revisions

- **Live N2N TCP peer ingress** (N-L; extended through N-M-*): carried.
- **Process-boundary node entry** (N-K; carried).
- **Receive-side N2N peer ingress** (N-H + N-I + N-J + N-K + N-L +
  N-M-*): carried.
- **Producer-side chain-sync server-role ingress** (N-G): carried.
- **Producer-side block-fetch server-role ingress** (N-G): carried.
- **Forge-block transition** (N-C): carried.
- **Self-accept broadcast gate** (N-C): carried.
- **Scheduler input ingress** (N-C; N-K Clock-driven): carried.
- **Mempool ingress** (Tier-1 wire-level — N-E): carried.
- **Conway tx-body `proposal_procedures` sub-grammar** (PP): carried.
- **Single-tx validity** (B2): carried.
- **Mempool admission** (Tier-1 gate — B2): carried.
- **Full block validity** (B1): carried.
- **Persistent ledger snapshot encoding** (N-J): carried.
- **Block bytes, Plutus script bytes, Snapshot bytes,
  Consensus-input extraction, Ouroboros mux frames, Genesis JSON
  bundles, Chain-selector stream inputs**: all carried.
- **Receive-side rollback authority** (N-I + N-J + N-K + N-L +
  N-M-*): carried.

### Operator-action evidence (carried — no PHASE4-N-P addition)

PHASE4-N-P added **no new operator-action probe binary** and **no
new live-evidence obligation**. The cluster's MAC 5 (cardano-cli
corpus + 3 ground-truth fixtures) is mechanically enforced and
already committed at `crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs`
(`#[cfg(test)]` — hex-literal `&[u8; 608]` constants only; no `.skey`
files committed; OQ4 hex-literal-only corpus rule per
`feedback_no_credential_leaks`).

### Candidates — surfaces not yet wired

All N-L / N-M-* / N-O candidates carry forward. **PHASE4-N-P
introduces no new candidate surface.** The cluster doc explicitly
declares two out-of-scope follow-ons:

| Cluster | Surface | Expected reduction target | Confidence |
|---------|---------|---------------------------|------------|
| **PHASE4-N-P (FULLY CLOSED at this HEAD — mechanical close)** | **cardano-cli expanded KES `.skey` envelope (608-byte) → BLUE SigningKey → RED KesSecret** | **DONE:** `Sum6Kes::raw_deserialize_signing_key_kes` (the BLUE structural validator); `KesSecret::from_blue_signing_key` (the RED custody constructor); `KeyLoadError::KesParse(KesParseError)` (the closed parse-error route). | **wired & closed in PHASE4-N-P** |
| **CANDIDATE — mlocked secret memory (`sodium_mlock`)** *(declared out-of-scope in N-P; future cluster `TBD-OPS`)* | Hardened sub-seed storage in mlocked pages | A new RED-side allocator wrapper; no BLUE surface change. | **candidate (future cluster)** |
| **CANDIDATE — `CompactSum6Kes` (192-byte signature form)** *(declared out-of-scope in N-P)* | A second BLUE KES algorithm variant for internal node-to-node bandwidth optimization | `kes_sum::CompactSum6Kes` alongside `Sum6Kes`; **not on the bounty path** — mainnet on-chain headers use the non-compact 448-byte form. | **candidate (future cluster; not bounty-priority)** |
| **CANDIDATE — VRF / cold-key BLUE migration** *(declared out-of-scope in N-P)* | Pure-Rust Ade-owned VRF (`VrfDraft03`) + cold Ed25519 algorithm | New BLUE modules paralleling `kes_sum/`; existing custody discipline carries. N9 currently scoped KES-only — VRF + DSIGN remain on `cardano-crypto`. | **candidate (future cluster; separate from KES N9)** |
| **CANDIDATE — cardano-cli `.skey` WRITE path** *(declared explicitly out-of-scope in N-P)* | Emit cardano-cli compatible envelopes from Ade | New BLUE serializer + RED writer. **Not needed** — operators generate via cardano-cli or Ade-native, not via Ade emitting cardano-cli envelopes. | **non-goal** |
| **CANDIDATE — ChainDb persistence of partially-evolved KES keys across process restarts** *(declared out-of-scope in N-P)* | Either: persist `(seed, period_idx)` and re-derive at startup (Ade-native flow); or persist the expanded tree directly (cardano-cli flow). Both paths are loadable post-N-P; explicit rotation tooling is operator-side. | Future operator-facing tool; no BLUE invariants change. | **candidate (operator tooling)** |

All N-L / N-L-LIVE / N-M-* / N-O candidate carry forward. See
HEAD_DELTAS for the per-cluster surface tables.

---

## 2. Data-Only vs. Authoritative Layers

Ade has **twenty-two** authoritative domains as of HEAD `d6f3399`.
**PHASE4-N-P promotes one existing domain (KES verification +
signing) from "BLUE verifies, RED owns the algorithm via
`cardano-crypto`" to "BLUE owns the algorithm end-to-end, RED owns
custody only".** This is the only domain change since the N-L
refresh (the N-M-* clusters added domains for seed-import,
bootstrap-anchor, WAL, admission orchestrator, evidence reducer; the
N-O cluster added the Ade-native KES key-gen flow domain; those are
documented in their cluster docs and in HEAD_DELTAS, and are
inherited here unchanged).

### KES signing-key algorithm authority (NEW BLUE owner in PHASE4-N-P)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **BLUE algorithm chokepoint** | `ade_crypto::kes_sum::KesAlgorithm` trait + `Sum0Kes..Sum6Kes` impls | BLUE | The **SOLE BLUE-owned Sum_n KES algorithm in the workspace**. Pure: no I/O, no clock, no `HashMap`/`HashSet`, no float, no `String`-bearing variant. Hand-rolled `Drop` on `SumSigningKey` zeroizes sub-seed buffers via `ZeroizingSeed`. Pinned `VerificationKey = [u8; 32]` for the entire chain. Compile-time size assertions in `mod.rs` (Sum6 = 608 bytes signing key / 448 bytes signature / 32 bytes VK / 64 total periods). |
| **BLUE serde chokepoint** | `ade_crypto::kes_sum::sum::SumKes::{raw_serialize_signing_key_kes, raw_deserialize_signing_key_kes, raw_serialize_signature_kes, raw_deserialize_signature_kes}` | BLUE | Canonical Haskell-equivalent byte serialization for the expanded tree (608 bytes for Sum6) and the structured signature (448 bytes for Sum6). The deserializer IS the structural validator; closed `KesParseError` surface. |
| **BLUE period chokepoint** | `ade_crypto::kes_sum::period::period_from_zeroed_sum6_tree_shape` | BLUE | Period inference from tree shape per the S1 proof obligation at `docs/clusters/completed/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md`. Total over a structurally-valid 608-byte payload; never heuristic. |
| **BLUE hash chokepoint** | `ade_crypto::kes_sum::hash::{expand_seed, hash_pair}` | BLUE | Blake2b-256-based seed expansion (left prefix `0x01`, right prefix `0x02` — Haskell convention; CI Guard 4) and vk-pair hashing. **Defended against `cardano-crypto` Rust 1.0.8 drift** (which uses `0x00 / 0x01`). |
| **BLUE verify chokepoint** | `ade_crypto::kes::verify_kes_signature` *(migrated to BLUE call in N-P S3)* | BLUE | Recursive VK-hash mismatch check + leaf Ed25519 verify. Calls `Sum6Kes::verify_kes` (BLUE, the new Ade-owned impl). |
| **BLUE closed parse-error surface** | `ade_crypto::kes_sum::errors::KesParseError` | BLUE | 6 variants: `WrongPayloadSize`, `LeafSignKeyAllZero`, `InconsistentSubtreeVkLeft`, `InconsistentSubtreeVkRight`, `LevelOutOfRange`, `InvalidEd25519SignatureLength`. Each carries only non-secret `u32` / `usize` metadata. No `String`; no `#[non_exhaustive]`. `Debug` derived (no manual redaction needed). |
| **BLUE closed runtime-error surface** | `ade_crypto::kes_sum::KesError` | BLUE | 5 variants: `InvalidSeedLength`, `PeriodOutOfRange`, `VerificationFailed`, `KeyExpired`, `Ed25519(&'static str)`. No key bytes, no seeds, no hex — `Ed25519` arm carries only a `&'static str` literal. |
| **RED custody chokepoint** *(N-C carried; N-P widened to host the new BLUE-typed `inner`)* | `ade_runtime::producer::signing::KesSecret` | RED | RED custody wrapper: redacted `Debug`, no public byte accessors, `kes-secret` whitelist in `ci_check_private_key_custody.sh`. `inner: <Sum6Kes as KesAlgorithm>::SigningKey` (BLUE-typed). |
| **RED constructor (Ade-native flow)** *(N-O carried)* | `ade_runtime::producer::signing::KesSecret::from_seed_at_period` | RED | Seed-bytes → `KesSecret` advanced to `period_idx`. Used by `load_ade_kes_signing_key` (the `ade.kes.seed.v1` envelope path). |
| **RED constructor (cardano-cli flow)** *(NEW in N-P S5)* | `ade_runtime::producer::signing::KesSecret::from_blue_signing_key` | RED | `pub(super) fn from_blue_signing_key(inner: <Sum6Kes as KesAlgorithm>::SigningKey, current_period: KesPeriod) -> Self`. **The ONLY RED-side construction path for a `KesSecret` whose `inner` originated from a cardano-cli envelope.** Single call site: `ade_runtime::producer::keys::load_kes_signing_key_skey`. |
| **RED loader** *(N-O extended in N-P)* | `ade_runtime::producer::keys::load_kes_signing_key_skey` | RED | The cardano-cli envelope loader. N-O posture was unconditional fail-close; N-P posture is "608-byte structurally-valid → `Ok(KesSecret)` via the BLUE deserializer; anything else → closed `KeyLoadError`". Routes wrong-size payloads to `KeyLoadError::UnsupportedExpandedKesKeyFormat` (preserves the N-O surface) and structural defects to `KeyLoadError::KesParse(KesParseError)` (new N-P variant). |
| **GREEN test corpus** *(NEW in N-P S4)* | `ade_crypto::kes_sum::cardano_cli_corpus` (`#[cfg(test)]`) | GREEN (by content, inside BLUE module) | 3 throwaway-key fixtures committed as hex-literal `&[u8; 608]` constants. Each preceded by the mandatory `// TEST ONLY: throwaway deterministic fixture generated for Sum6KES` comment. The cardano-cli ground-truth oracle: `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth` + `cardano_cli_corpus_sign_then_upstream_verifies` + `cardano_cli_corpus_negative_flip_one_byte_in_vk_left_fail_closed`. **OQ4 rule: no `.skey` envelope files committed under the repo; hex-literal corpus only.** |
| **CI gates (1 new + 1 narrowed)** | `ci/ci_check_kes_sum_compatibility.sh` (NEW — 4 guards) + `ci/ci_check_kes_envelope_closed.sh` (narrowed Guard 2) | CI | (1) `kes_sum_compatibility` — corpus + throwaway-comment present; no committed `.skey` envelopes under `crates/ade_crypto/`; `cardano_crypto::kes` imports only inside `#[cfg(test)]` (N9 — KES-scoped); `expand_seed` prefix bytes match Haskell `cardano-base` (0x01 / 0x02). (2) `kes_envelope_closed` Guard 2 narrowed — now requires the loader body to call `raw_deserialize_signing_key_kes` for the accept path (replacing N-O's unconditional-fail-close assertion). |

**Rule.** This domain has:
- **One BLUE algorithm trait** (`KesAlgorithm` — closed surface;
  pure; no I/O).
- **One BLUE Sum_n chain** (`Sum0Kes..Sum6Kes` — `Sum6Kes = SumKes<Sum5Kes>`).
- **One BLUE serde chokepoint pair** (`raw_serialize_*` /
  `raw_deserialize_*`).
- **One BLUE period inference function**
  (`period_from_zeroed_sum6_tree_shape`).
- **One BLUE closed parse-error surface** (`KesParseError`).
- **One BLUE closed runtime-error surface** (`KesError`).
- **One BLUE verify call site** (`verify_kes_signature`).
- **One RED custody wrapper** (`KesSecret`) with two construction
  paths (`from_seed_at_period` for Ade-native; `from_blue_signing_key`
  for cardano-cli).
- **One new CI gate + one narrowed** defending the above.

**THE KEY SEAMS:**

1. **`KesAlgorithm` is the SOLE BLUE-owned Sum_n KES trait** in the
   workspace. CI-defended via `ci_check_kes_sum_compatibility.sh`
   Guard 3 (no `cardano_crypto::kes` outside `#[cfg(test)]`).
2. **`KesParseError` is closed and `Debug`-safe**: every variant
   carries only `u32` / `usize` metadata; no key bytes; no hex.
   New variant = new `[[rules]]` entry under `DC-CRYPTO` family +
   strengthening of `DC-CRYPTO-09`.
3. **`KesSecret::from_blue_signing_key` is `pub(super)`** —
   RED-internal; `producer::keys` is the sole call site. Widening
   to `pub` is a discipline violation.
4. **The `expand_seed` prefix-byte convention is Haskell, not
   `cardano-crypto` Rust** (CI-defended by Guard 4 of
   `ci_check_kes_sum_compatibility.sh`).
5. **No `.skey` envelopes committed under `crates/ade_crypto/`**
   (OQ4 rule; CI-defended by Guard 2 of
   `ci_check_kes_sum_compatibility.sh`). Hex-literal `&[u8; 608]`
   corpus only, with the mandatory throwaway-fixture comment.
6. **Per-period evolution discipline survives the migration**
   (`DC-CRYPTO-04` strengthened): the BLUE `update_kes` chain is
   one-way; the consumed sub-seed is best-effort zeroized at
   `Drop`.

**New work** that adds a KES-algorithm-side feature attaches by:
- Adding a new method to the BLUE `KesAlgorithm` trait (requires
  re-implementation across the full `Sum0..Sum6` chain; new
  `[[rules]]` entry; new CI test).
- Adding a new `KesParseError` variant + matching match arm in
  `raw_deserialize_signing_key_kes` (closed-sum extension;
  `DC-CRYPTO-09` strengthening).
- Adding a parallel KES algorithm (e.g., `CompactSum6Kes`,
  out-of-N-P-scope) under a new `ade_crypto::kes_sum::*` submodule;
  the trait surface does not change.

— **not** by reintroducing `cardano_crypto::kes::*` outside
`#[cfg(test)]` in `crates/ade_crypto/src/**`, **not** by adding a
fallback parser for malformed 608-byte payloads, **not** by widening
`KesSecret::from_blue_signing_key` past `pub(super)`, **not** by
landing a compatibility shim that constructs an upstream
`SumSigningKey` via `unsafe`, `transmute`, vendored `pub(crate)`
access, or fork-only constructors (N9 — load-bearing hard
prohibition).

**Declared non-goals carried from the cluster doc:**
- Mlocked secret memory (`sodium_mlock`) — future cluster (TBD-OPS).
- `CompactSum6Kes` (192-byte signature form) — future cluster; not
  bounty-priority.
- VRF (`VrfDraft03`) + cold Ed25519 BLUE migration — separate
  future clusters; N9 currently KES-scoped.
- ChainDb persistence of partially-evolved KES keys across process
  restarts — operator-side tooling; either path (Ade-native
  `(seed, period_idx)` or cardano-cli expanded-tree) loadable
  post-N-P.
- Generating cardano-cli compatible `KesSigningKey_ed25519_kes_2^6`
  envelopes from Ade (reverse direction; write path) — not needed.

### Wire-layer session authority (carried from PHASE4-N-L; extended N-M-FRAG with per-protocol payload reassembly)

Carried structurally. **N-M-FRAG note (carried — `4d3dc98`)**: the
session reducer now performs per-mini-protocol payload reassembly;
live admission reaches `block_received` against the fully-synced
docker peer. `CN-SESS-04` + `DC-SESS-06` `enforced`.
**N-M-FOLLOW note (carried — `7bed3d5`)**: the wire pump now drives
RollForward → header-point extraction → block-fetch → BatchDone-gated
RequestNext, plus the op-cert "equal counter = no-op" fix. **34
consecutive `BlockAdmitted` across slots 124137045..124137868 with
0 diverged**.

### Receive-side admission authority (carried from PHASE4-N-H + N-K + N-L + N-M-B + N-M-C)

Carried. **N-M-B / N-M-C note (carried)**: admission now driven by a
dedicated GREEN admission orchestrator + `AgreementVerdict` evidence
reducer + per-admit WAL + replay-equivalence harness.
`DC-EVIDENCE-01` + `RO-LIVE-05` `enforced` at HEAD `d8feabb`
(N-M-SCHED) with committed live transcript at
`docs/evidence/phase4-n-m-c-operator-pass-transcript.jsonl`.

### Node orchestration authority (carried from PHASE4-N-K + N-L)

Carried.

### Persistent ledger snapshot encoding authority (carried from PHASE4-N-J + N-K)

Carried unchanged.

### Receive-side rollback authority (carried from N-I + N-J + N-K + N-L + N-M-*)

Carried.

### Producer-side server response authority (carried from N-G)

Carried.

### Block production authority (carried from N-C + N-K)

Carried. **N-P note**: the producer-side signing path now invokes the
Ade-owned BLUE `KesAlgorithm` via `KesSecret.inner` instead of the
upstream `cardano-crypto` Rust impl. `R2 (cross-impl replay
equivalence)` is the load-bearing rule for the migration: a WAL
replay containing a block whose KES sig was produced by
`ade_crypto::kes_sum::Sum6Kes::sign_kes` hash-equals the same replay
if the sig were produced by `cardano_crypto::kes::Sum6Kes::sign_kes`
(for the same seed under matching `expand_seed` prefixes — which
the cardano-cli corpus mechanically validates).

### Mempool ingress (carried from N-E)

Carried.

### Conway tx-body `proposal_procedures` sub-grammar authority (carried from PROPOSAL-PROCEDURES-DECODE)

Carried.

### Conway value-conservation accounting / Conway certificate-state accumulation / Credential discriminant fidelity / Conway governance-cert accumulation / Single-tx validity / Mempool admission / Full block validity / Ledger application / Stake-snapshot projection for consensus / Plutus phase-2 evaluation / Governance ratification & enactment / Mini-protocol wire conformance / Praos consensus runtime

All carried unchanged. **PHASE4-N-P-specific strengthening:**
`DC-CRYPTO-03` (signing transcript equivalence) /
`DC-CRYPTO-04` (per-period evolution discipline) /
`DC-CRYPTO-05` (KES boundary discipline) each gain
`strengthened_in += PHASE4-N-P`. The strengthening reflects that
the rules' enforcement now backs onto Ade-owned BLUE code rather
than `cardano-crypto` upstream.

### Where the boundary is enforced

- `ci_check_dependency_boundary.sh` — no BLUE crate may depend on
  RED. **N-P added no new BLUE-crate edge** (`ade_crypto`'s outbound
  deps remain `{ade_types}` — see CODEMAP §"Cross-crate dependency
  audit"; the `cardano-crypto` dev-dep is `#[cfg(test)]`-only after
  N-P).
- `ci_check_no_async_in_blue.sh` — async forbidden in BLUE. The new
  `kes_sum/` files contain no `async fn`, no `tokio::*`, no
  `.await`.
- **`ci_check_kes_sum_compatibility.sh`** *(NEW in N-P)* — 4 guards
  defending the BLUE-owned KES algorithm: (1) corpus +
  throwaway-comment present; (2) no `.skey` envelope files under
  `crates/ade_crypto/`; (3) `cardano_crypto::kes` only inside
  `#[cfg(test)]` files; (4) `expand_seed` prefix bytes match
  Haskell `cardano-base`.
- **`ci_check_kes_envelope_closed.sh`** *(narrowed Guard 2 in N-P)*
  — now requires the loader body to call
  `raw_deserialize_signing_key_kes` for the accept path. The N-O
  unconditional-fail-close assertion is replaced by the
  accept-via-BLUE-deserializer assertion.
- `ci_check_private_key_custody.sh` — RED-confined custody;
  `kes-secret` whitelist; redacted `Debug`. **N-P:**
  `kes_sum/` paths added to the whitelist (`mod.rs` declares
  hand-rolled `Drop` for `Sum0SigningKey` and the recursive
  `SumSigningKey<D>` via `ZeroizingSeed`).
- *N-L carried CI gates:* `ci_check_mux_frame_closure.sh`,
  `ci_check_handshake_closure.sh`,
  `ci_check_session_core_closure.sh`,
  `ci_check_mini_protocol_id_registry_closed.sh`,
  `ci_check_session_no_unbounded.sh`, `ci_check_clock_seam.sh`
  (extended).
- *N-K carried CI gates:* `ci_check_bootstrap_closure.sh`,
  `ci_check_orchestrator_core_purity.sh`,
  `ci_check_persistent_writer_no_parallel_cadence.sh`,
  `ci_check_peer_session_isolation.sh`,
  `ci_check_node_binary_uses_single_bootstrap.sh`.
- *N-J carried CI gates:* `ci_check_snapshot_encoder_closure.sh`.
- *N-I carried CI gates:* `ci_check_rollback_materialize_closure.sh`,
  `ci_check_snapshot_cadence_purity.sh`.
- *N-H carried CI gates:* `ci_check_admitted_block_closure.sh`,
  `ci_check_receive_reducer_closure.sh`,
  `ci_check_receive_replay_purity.sh`,
  `ci_check_receive_orchestrator_no_producer_dep.sh`,
  `ci_check_receive_paths_corpus_present.sh`.
- *N-G carried CI gates:* `ci_check_no_parallel_header_splitter.sh`,
  `ci_check_served_chain_closure.sh`,
  `ci_check_chain_sync_server_closure.sh`,
  `ci_check_block_fetch_server_closure.sh`,
  `ci_check_broadcast_to_served_purity.sh`,
  `ci_check_n2n_server_no_signing_dep.sh`,
  `ci_check_server_paths_corpus_present.sh`.
- *N-C carried CI gates:* `ci_check_private_key_custody.sh`
  (narrowed by N-P whitelist), `ci_check_opcert_closed.sh`,
  `ci_check_forge_purity.sh`, `ci_check_no_producer_body_encoder.sh`,
  `ci_check_self_accept_gate.sh`, `ci_check_scheduler_closure.sh`,
  `ci_check_producer_corpus_present.sh`.
- *N-M-* carried CI gates:* the N-M sub-cluster gates per their
  respective cluster docs (seed-import, bootstrap-anchor, WAL,
  evidence reducer, frame-reassembly, schedule wiring, follow-mode
  closure).
- `ci_check_constitution_coverage.sh` — carried.
- `ci_check_proposal_procedures_closed.sh` — carried.
- `ci_check_mempool_ingress_closure.sh` /
  `ci_check_mempool_ingress_replay.sh` — carried.
- `ci_check_credential_discriminant_closed.sh` — carried.
- `ci_check_gov_cert_accumulation_closed.sh` — carried.
- `ci_check_deposit_param_authority.sh` — carried.
- `ci_check_conway_cert_classification_closed.sh` — carried.
- `ci_check_no_chaindb_in_consensus_blue.sh` /
  `ci_check_no_float_in_consensus.sh` /
  `ci_check_no_density_in_fork_choice.sh` /
  `ci_check_consensus_closed_enums.sh` — carried.
- `ci_check_pallas_quarantine.sh`, `ci_check_no_signing_in_blue.sh`,
  `ci_check_ingress_chokepoints.sh`, `ci_check_ce_n_a_5_proof.sh` —
  carried.

---

## 3. Closed vs. Extensible Registries

Ade's authority surface is **almost entirely closed.** **PHASE4-N-P
adds four closed surfaces** under the new BLUE `kes_sum/` module —
the `KesAlgorithm` trait, the `KesParseError` closed sum, the
`KesError` closed sum, and the `KesSecret::from_blue_signing_key`
RED-internal constructor — plus **one new CI gate** and one **narrowed
gate**. Registry total moves to **264 entries** (223 → 264 across
N-L-LIVE / N-M-* / N-O / N-P; the N-P-specific increment is
**+2 introduced + 2 closed obligations + 3 strengthenings**).

### Closed (frozen — version-gated changes only)

The full carried table from the N-L refresh is preserved. **The new
PHASE4-N-P entries** are listed inline below; they slot into the
existing table.

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| *(carried — full table preserved from the N-L SEAMS refresh; see HEAD_DELTAS for the N-L-LIVE / N-M-* / N-O entries)* | | | |
| **`KesAlgorithm` trait** *(NEW in N-P — DC-CRYPTO-08)* | `ade_crypto::kes_sum::KesAlgorithm` | closed trait with 11 required methods + 4 associated consts + 1 derived const | The **SOLE BLUE-owned Sum_n KES algorithm trait**. Implementors: `Sum0Kes..Sum6Kes` (closed chain). New method = strengthening; new variant of the Sum_n chain = internal additive only. CI-defended via `ci_check_kes_sum_compatibility.sh`. |
| **`KesParseError` closed sum** *(NEW in N-P — DC-CRYPTO-09)* | `ade_crypto::kes_sum::errors::KesParseError` | 6 variants | Closed parse-grammar error surface. Each variant carries only non-secret `u32` / `usize` metadata. No `String`; no `#[non_exhaustive]`. New variant = `DC-CRYPTO-09` strengthening + new test. |
| **`KesError` closed sum** *(NEW in N-P)* | `ade_crypto::kes_sum::KesError` | 5 variants | Closed runtime-error surface. `Ed25519` arm carries only a `&'static str` (no key bytes). |
| **`KeyLoadError::KesParse(KesParseError)` variant** *(NEW in N-P)* | `ade_runtime::producer::keys::KeyLoadError` | 1 new variant on the closed N-O surface | Closed-sum extension on the N-O `KeyLoadError` enum. Routes structural defects from the BLUE deserializer to the RED caller. CI-defended via `ci_check_kes_envelope_closed.sh` (existing — variant-presence check). |
| **`KesSecret::from_blue_signing_key` constructor** *(NEW in N-P — RED-internal constructor seam)* | `ade_runtime::producer::signing::KesSecret` | 1 `pub(super) fn` | RED-internal — `producer::keys` is the sole call site. `pub(super)` is load-bearing: widening to `pub` permits bypassing the BLUE deserializer's structural validation. No CI gate today (visibility is the gate); a follow-on CI script could grep for the `pub(super)` qualifier specifically. |
| **Haskell `expand_seed` prefix-byte convention** *(NEW in N-P — DC-CRYPTO-08)* | `ade_crypto::kes_sum::hash` | 2 literal byte constants (`0x01`, `0x02`) | CI-defended via `ci_check_kes_sum_compatibility.sh` Guard 4. Drift to `cardano-crypto` Rust 1.0.8 prefixes (`0x00`, `0x01`) is a discipline violation. |
| **Sum_n compile-time size assertions** *(NEW in N-P)* | `ade_crypto::kes_sum::mod` (`const _: () = { ... }`) | Sum0..Sum6 × {signing-key, signature, VK, seed} = ~16 assertions | Compile-fail if any Sum_n size drifts from the recurrence. Sum6 = 608 / 448 / 32 / 32. |
| **CI check set (updated)** | `ci/ci_check_*.sh` | **89 scripts at HEAD `d6f3399` (88 → 89 in PHASE4-N-P; the N-L→N-P interval added ~23 across N-L-LIVE / N-M-* / N-O / N-P combined)** | Existing checks may be tightened, never relaxed. **N-P-specific**: `ci_check_kes_sum_compatibility.sh` added; `ci_check_kes_envelope_closed.sh` Guard 2 narrowed. |
| **Invariant registry families (updated)** | `docs/ade-invariant-registry.toml` | Families T / CN / DC / OP / RO; **N-P introduced 2 new (`DC-CRYPTO-08` + `DC-CRYPTO-09`, both `enforced`); cleared 2 `open_obligation`s (`OP-OPS-04` + `DC-CRYPTO-07`); strengthened 3 carried (`DC-CRYPTO-03` / `DC-CRYPTO-04` / `DC-CRYPTO-05` each `strengthened_in += PHASE4-N-P`).** Total: **264 entries** (223 → 264 across N-L-LIVE / N-M-* / N-O / N-P combined). | Append-only IDs. |

### Extensible (open within constraints)

Carried from the N-L refresh. **PHASE4-N-P added no new extensible
registry** — the cluster's only "growth point" is the cardano-cli
test corpus, which is extensible-with-constraints:

| Registry | Location | Extension Rule |
|----------|----------|---------------|
| *(carried — full table preserved from the N-L SEAMS refresh)* | | |
| **cardano-cli `Sum6KES` corpus** *(NEW in N-P — tooling-only, `#[cfg(test)]`)* | `crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs` | Hex-literal `&[u8; 608]` SKEY constants + matching `&[u8; 32]` VKEY constants. **Each fixture MUST be preceded by `// TEST ONLY: throwaway deterministic fixture generated for Sum6KES`** (OQ4 — `feedback_no_credential_leaks`). Adding a fixture = adding `SKEY{N} / VKEY{N}` pair + extending the ground-truth + sign-verify tests. No `.skey` files committed (CI-defended). |

### Candidates — extensible surfaces not yet wired

Carried from the N-L refresh + N-L-LIVE / N-M-* / N-O candidates
(see HEAD_DELTAS). **PHASE4-N-P added candidates** for the
out-of-scope follow-ons listed in §1 (mlocked memory,
`CompactSum6Kes`, VRF + cold BLUE migration). Each requires its
own cluster scope.

### Closed-grammar audit (PHASE4-N-P full close)

This sweep was performed after PHASE4-N-P full close.

1. **`KesAlgorithm` trait** — **closed by intent and CI-defended.**
   `ade_crypto::kes_sum::KesAlgorithm` is the SOLE BLUE-owned Sum_n
   KES algorithm trait in the workspace (DC-CRYPTO-08;
   `ci_check_kes_sum_compatibility.sh` Guard 3).
2. **`KesParseError` closed sum** — **closed by intent.** 6
   variants; each carries only `u32` / `usize`; `Debug` derived; no
   `String`; no `#[non_exhaustive]` (DC-CRYPTO-09).
3. **`KesError` closed sum** — **closed by intent.** 5 variants;
   `Ed25519` arm carries only `&'static str`.
4. **`KeyLoadError::KesParse` extension** — **closed-sum extension
   on the N-O `KeyLoadError`.** Routes structural defects from the
   BLUE deserializer to the RED caller.
5. **`KesSecret::from_blue_signing_key`** — **`pub(super)` is the
   gate.** RED-internal; sole call site `producer::keys`.
6. **`expand_seed` prefix-byte convention** — **closed by intent
   and CI-defended.** Haskell `cardano-base` (0x01 / 0x02);
   `ci_check_kes_sum_compatibility.sh` Guard 4.
7. **cardano-cli ground-truth corpus** — **closed by intent and
   CI-defended.** Hex-literal-only; OQ4 throwaway-comment per
   fixture; no `.skey` files committed
   (`ci_check_kes_sum_compatibility.sh` Guards 1 + 2).
8. **`ade_crypto::kes::verify_kes_signature` BLUE call site** —
   **migrated to BLUE in N-P S3.** Inner call switched from
   `cardano_crypto::kes::Sum6Kes::verify_kes` to
   `ade_crypto::kes_sum::Sum6Kes::verify_kes`.
9. **Sum_n compile-time size assertions** — **closed by `const _:
   () = {assert!(...)}`** in `kes_sum::mod`.

**Gap note — mlocked secret memory.** Declared out-of-scope in N-P;
future cluster (TBD-OPS).

**Gap note — `CompactSum6Kes`.** Declared out-of-scope in N-P; not
bounty-priority.

**Gap note — VRF + cold Ed25519 BLUE migration.** N9 is currently
KES-scoped; VRF (`VrfDraft03`) + cold (Ed25519) algorithm continue
to use `cardano-crypto` upstream. Separate future clusters.

### Closed-grammar audit (carried — PHASE4-N-L / N-K / N-J / N-I / N-H / N-G / N-C / PROPOSAL-PROCEDURES-DECODE / N-E / B3 / B4 / B5)

All carried unchanged from prior revision. The N-L-LIVE / N-M-* / N-O
sweeps are recorded in HEAD_DELTAS.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

Carried from the N-L refresh + intermediate clusters. **PHASE4-N-P
adds the following frozen contracts:**

- **Single Sum6KES algorithm authority** *(NEW in N-P — DC-CRYPTO-08)*:
  `ade_crypto::kes_sum::Sum6Kes` is the SOLE BLUE-owned Sum6KES
  implementation in the workspace. Byte-identical to Haskell
  `cardano-base`'s `Sum6KES Ed25519DSIGN`. CI-defended via
  `ci_check_kes_sum_compatibility.sh`.
- **Closed Sum6KES expanded-skey serde grammar** *(NEW in N-P —
  DC-CRYPTO-09)*: `raw_serialize_signing_key_kes` /
  `raw_deserialize_signing_key_kes` are byte-identical to Haskell's
  `rawSerialiseSignKeyKES` / `rawDeserialiseSignKeyKES`. On-disk
  format is exactly 608 bytes; any other size = closed
  `KesParseError::WrongPayloadSize`. `current_period` uniquely
  inferable from tree shape per the S1 proof obligation.
- **Closed Sum6KES signature serde grammar** *(NEW in N-P —
  DC-CRYPTO-09)*: `raw_serialize_signature_kes` /
  `raw_deserialize_signature_kes` produce/consume exactly 448 bytes.
  Closed `KesParseError::InvalidEd25519SignatureLength` on the leaf
  size mismatch.
- **`KesParseError` closed grammar** *(NEW in N-P — DC-CRYPTO-09)*:
  6 variants; each carries only `u32` / `usize`; no `String`; no
  `#[non_exhaustive]`. Adding a variant requires `DC-CRYPTO-09`
  strengthening.
- **Haskell `expand_seed` prefix-byte convention** *(NEW in N-P —
  DC-CRYPTO-08)*: left prefix `0x01`, right prefix `0x02`. Drift to
  `cardano-crypto` Rust 1.0.8 prefixes (`0x00` / `0x01`) is a
  CI-block-level discipline violation.
- **N9 KES-scoped BLUE algorithm ownership** *(NEW in N-P)*: no
  `cardano-crypto::kes::*` import in production code under
  `crates/ade_crypto/src/**` (outside `#[cfg(test)]`). The
  `cardano-crypto` crate stays as a `dev-dependency` of
  `ade_crypto` for KES; as a regular dep of `ade_runtime` for VRF +
  DSIGN. CI-defended via `ci_check_kes_sum_compatibility.sh`
  Guard 3.
- **`KesSecret.inner` BLUE-typed migration** *(NEW in N-P — DC-CRYPTO-08)*:
  `KesSecret.inner: <Sum6Kes as KesAlgorithm>::SigningKey`
  (BLUE-owned). Carries forward the redacted-`Debug` + no-public-byte-
  accessors + RED-confined custody discipline established in N-C.
- **`KesSecret::from_blue_signing_key` constructor visibility**
  *(NEW in N-P)*: `pub(super)` — RED-internal. Widening to `pub`
  permits bypassing the BLUE deserializer's structural validation.
- **`load_kes_signing_key_skey` accept-path single chokepoint**
  *(NEW in N-P — DC-CRYPTO-07 narrowed)*: 608-byte payload size →
  `Sum6Kes::raw_deserialize_signing_key_kes` → `KesSecret::from_blue_signing_key`.
  No fallback parser. The BLUE deserializer IS the structural
  validator. CI-defended via `ci_check_kes_envelope_closed.sh`
  Guard 2 (narrowed).

Plus all prior frozen contracts carried unchanged (see the N-L
refresh + HEAD_DELTAS for N-L-LIVE / N-M-* / N-O additions).

### Version-gated (can evolve across major versions)

Carried from the N-L refresh + intermediate clusters. **PHASE4-N-P
adds the following version-gated extension points:**

- **New `KesAlgorithm` impl** *(NEW in N-P — extension point)*:
  e.g., `CompactSum6Kes`; future cluster. The trait surface does
  not change.
- **New `KesParseError` variant** *(NEW in N-P — extension point)*:
  closed-sum extension; `DC-CRYPTO-09` strengthening required.
- **New `KesError` variant** *(NEW in N-P — extension point)*:
  closed-sum extension; new `[[rules]]` entry required.
- **New `KeyLoadError` variant** *(carried from N-O; N-P added
  `KesParse`)*: closed-sum extension.
- **VRF + cold-key BLUE migration** *(NEW future cluster flagged
  by N-P close)*: a future N9 expansion past KES would close the
  upstream `cardano-crypto` dependency entirely; each algorithm
  family is a separate cluster.

---

## 5. Module Addition Rules

Ade's workspace is small and color-disciplined. **PHASE4-N-P added
one new BLUE module tree** (`crates/ade_crypto/src/kes_sum/` —
`mod`, `single`, `sum`, `hash`, `period`, `errors`, plus
`#[cfg(test)]` `tests` and `cardano_cli_corpus`). **One new CI gate
+ one narrowed**. **Two new registry rules + two closed
obligations + three strengthenings**. No new RED file (only the
new RED-internal constructor `KesSecret::from_blue_signing_key` in
existing `signing.rs`). **No new external ingress surface**, **no
new operator-action probe binary**.

**N-P also tightened the `ade_crypto` crate-internal structure**:

1. `ade_crypto` outbound deps remain `{ade_types}` — unchanged.
   `cardano-crypto` is now dev-dep (KES paths) + regular dep (VRF +
   DSIGN paths, unchanged).
2. `ade_runtime`'s `cardano-crypto` declaration drops the
   `kes-sum` feature (`features = ["vrf-draft03", "dsign"]`).
   This is the load-bearing build-time signal that KES production
   code no longer touches `cardano-crypto`.

**The module-addition rule N-P sets for future crypto-side work
(load-bearing N9 boundary):**

1. **A new BLUE crypto algorithm submodule attaches under
   `ade_crypto::*`** as a closed trait + closed impl chain + closed
   error surface. Must be pure: no `cardano-crypto` import (KES);
   no `unsafe` outside an explicit allowlist; no `HashMap`,
   `SystemTime`, `rand`, float. Hand-rolled `Drop` on any
   secret-bearing type.
2. **A new closed parse-error surface attaches in
   `<algorithm>::errors`** as a closed enum with `u32` / `usize`
   metadata only — no `String`, no key bytes, no `#[non_exhaustive]`.
3. **A new RED-internal constructor on a custody wrapper attaches
   as `pub(super)`** (or tighter). Widening visibility is a
   discipline violation — the BLUE deserializer's structural
   validation must be the sole entry path for cardano-cli-flavor
   payloads.
4. **A new CI gate attaches for each closed surface**: model on
   `ci_check_kes_sum_compatibility.sh` (4 guards: corpus +
   throwaway-comment present; no committed key files; upstream
   import only inside `#[cfg(test)]`; prefix-byte / magic-byte
   convention match).
5. **N9 module rule: "no upstream-crate import in production code
   for the algorithm family currently owned by BLUE"** is per-family.
   The rule is currently KES-scoped (`cardano_crypto::kes` only
   inside `#[cfg(test)]` under `crates/ade_crypto/src/**`). VRF +
   DSIGN remain on `cardano-crypto`. Future clusters that migrate
   VRF or cold Ed25519 must extend the rule per family and add a
   matching CI guard.
6. **A new BLUE algorithm cluster MUST ship a ground-truth corpus**
   under `#[cfg(test)]` with hex-literal throwaway fixtures (OQ4 —
   `feedback_no_credential_leaks`). The fixtures are the live
   oracle for the Haskell-equivalence claim.

### Cross-cluster obligation pattern (carried — no new obligation from N-P)

**N-P adds no new cross-cluster obligation.** The cluster's two
"deferred" items (mlocked secret memory; `CompactSum6Kes`) are
**future-cluster scope**, not cross-cluster obligations on N-P's
close.

### Operator-action evidence pattern (carried — no N-P addition)

**N-P adds no new operator-action probe binary.** The cardano-cli
ground-truth corpus is `#[cfg(test)]` — committed as hex literals,
mechanically validated by `cargo test`. The family remains at five
probe binaries from prior clusters.

### Cluster scope-edge pattern (carried — N-P applies it cleanly)

**N-P applies the scope-edge pattern to the cryptographic-primitive
boundary**: the cluster ships the Sum6KES algorithm + serde + period
inference + corpus + KES boundary; mlocked memory, `CompactSum6Kes`,
and VRF + cold migration are deliberate out-of-scope follow-ons.
The scope edge is documented in the cluster doc's §2 "Out of scope"
section and the closure record.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` | First line of every `.rs` is the contract banner. `lib.rs` carries `#![deny(unsafe_code, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_arithmetic)]`. No `#[cfg(feature = ...)]`. No async. **N-P:** new BLUE tree `ade_crypto::kes_sum/` — pure; no `cardano-crypto` import outside `#[cfg(test)]`; hand-rolled `Drop` on every secret-bearing struct; CI-defended by `ci_check_kes_sum_compatibility.sh`. | Other BLUE crates / submodules only. **`ade_crypto` outbound:** `{ade_types}`. | Any RED submodule or crate; GREEN in non-dev deps; `cardano-crypto::kes::*` outside `#[cfg(test)]` (NEW N9 in N-P, KES-scoped); `pallas_*` (except `ade_plutus`); async runtime; `HashMap`/`HashSet`/`IndexMap`; clock/rand/float/env/I/O. |
| **GREEN** | `ade_*` | Banner + deny attrs are project convention. **N-P:** `ade_crypto::kes_sum::cardano_cli_corpus` (`#[cfg(test)]`) and `ade_crypto::kes_sum::tests` (`#[cfg(test)]`) sit inside the new BLUE tree but carry GREEN-by-content classification (hex-literal corpus + 35 unit tests). | BLUE crates + standard library + ecosystem crates. | RED submodules in non-test paths. Results must never feed back into a BLUE authoritative decision. |
| **RED** | `ade_*` | No special header. Free to use clocks, I/O, async, `HashMap`, signing keys. **N-P:** `ade_runtime::producer::signing` extended with `pub(super) KesSecret::from_blue_signing_key`; `ade_runtime::producer::keys::load_kes_signing_key_skey` routes 608-byte payloads through the new BLUE deserializer. | Any BLUE / GREEN crate or submodule (one-way). **`ade_runtime` outbound:** `cardano-crypto` features narrowed to `["vrf-draft03", "dsign"]` (no more `kes-sum`). | Cannot be depended on by BLUE. |

### New module checklist

1. **Add to `Cargo.toml` workspace members** (if a new crate).
2. **Declare TCB color** by editing `.idd-config.json` `core_paths` if BLUE.
3. **CI script update obligations** — extend the relevant BLUE-scoped
   scripts; for crypto-side sub-modules, model the new CI gate on
   `ci_check_kes_sum_compatibility.sh` shape (4 guards: corpus +
   throwaway-comment present; no committed key files; upstream
   import only inside `#[cfg(test)]`; prefix-byte / magic-byte
   convention match).
4. **Add contract banner** (BLUE) to every `.rs` file.
5. **Add deny attributes** to `lib.rs` (BLUE).
6. **New canonical types:** add a `[[rules]]` block under family `T`
   in the invariant registry, plus a round-trip test. For new
   crypto-side authority rules, append `DC-CRYPTO-0X` /
   `CN-CRYPTO-0X` with bidirectional cross-ref to consumed rules
   (`T-DET-01`, `T-KEY-01`, `OP-OPS-04`, `DC-CRYPTO-03..05`).
7. **New operator-action probe binary:** (not applicable for
   cryptographic-primitive clusters — the ground-truth corpus is
   `#[cfg(test)]`).
8. **Cross-cluster obligation:** (N-P added none; the cluster doc's
   §2 "Out of scope" enumerates the future-cluster items
   directly).
9. **Cluster scope-edge:** document the deliberate scope-down in
   the cluster doc's §2 + the closure record. N-P's "mlocked memory
   + `CompactSum6Kes` + VRF/cold deferred" is the canonical
   example.
10. **Run `cargo test --workspace` and the full CI script suite.**

### Phase 4 anticipated additions

- **PHASE4-N-P — FULLY CLOSED at this HEAD** (mechanical close;
  BLUE-owned Sum6KES algorithm): `ade_crypto::kes_sum/` BLUE
  module tree + closed `KesAlgorithm` trait + closed
  `KesParseError` + closed `KesError` + `KesSecret::from_blue_signing_key`
  RED-internal constructor + cardano-cli loader accept path +
  cardano-cli ground-truth corpus + 4-guard CI script +
  `cardano-crypto` demoted to `#[cfg(test)]` oracle for KES.
- **PHASE4-N-O — FULLY CLOSED** (carried — Ade-native KES key-gen).
- **PHASE4-N-M-* — FULLY CLOSED** (carried — seed-import,
  bootstrap-anchor, WAL, admission orchestrator, evidence reducer,
  frame-reassembly, schedule wiring, follow-mode closure).
- **PHASE4-N-L — FULLY CLOSED** (carried).
- **PHASE4-N-L-LIVE — FULLY CLOSED** (carried).
- **PHASE4-N-K — FULLY CLOSED** (carried).
- **PHASE4-N-J / N-I / N-H / N-G / N-C / N-E /
  PROPOSAL-PROCEDURES-DECODE** — all FULLY CLOSED (carried).
- **NEW future cluster — mlocked secret memory (TBD-OPS)** *(declared
  out-of-scope in N-P)*: a future RED cluster wrapping `sodium_mlock`
  pages around `KesSecret.inner` + cold-key custody.
- **NEW future cluster — `CompactSum6Kes`** *(declared out-of-scope
  in N-P)*: 192-byte signature form for internal node-to-node
  bandwidth optimization. **Not bounty-priority.**
- **NEW future cluster — VRF BLUE migration** *(declared
  out-of-scope in N-P)*: Ade-owned `VrfDraft03` paralleling
  `kes_sum/`. N9 expansion past KES.
- **NEW future cluster — cold Ed25519 BLUE migration** *(declared
  out-of-scope in N-P)*: Ade-owned cold key custody beyond the
  current `ColdSigningKey::from_bytes_zeroizing` ed25519-dalek
  wrapper.
- **NEW future cluster — ChainDb persistence of partially-evolved
  KES keys** *(operator-side tooling, out-of-scope in N-P)*.
- All N-L-anchored future cluster candidates carry forward (live
  pass `RO-LIVE-03` superseded by N-L-LIVE / N-M-C / N-M-SCHED
  live evidence; TLS / authenticated transport; N2C local protocols
  session driver; peer-sharing + tx-submission live half; snapshot
  schema migration v1 → v2 tooling; metrics + observability;
  snapshot eviction policy; multi-peer fork choice; N2C
  local-chain-sync receive surface; pre-Conway snapshot encoder).

**These placements are candidates** — user confirmation needed at
cluster entry.

---

## 6. Forbidden Patterns (per color)

### BLUE (universal IDD prohibitions; enforced by CI where marked)

- No `HashMap`, `HashSet`, `IndexMap`, `IndexSet`.
- No `SystemTime`, `Instant`, `std::time::*` clocks.
- No `rand::thread_rng`, `thread::spawn`.
- No `f32`, `f64`, floating-point arithmetic.
- No `std::fs`, `std::net`, `tokio`, `async fn`.
- No `anyhow`; `unwrap`/`expect`/`panic` denied at the lint level.
- No `unsafe` outside an explicit allowlist.
- No `#[cfg(feature = ...)]` semantic gating.
- No signing patterns in BLUE.
- No re-hashing of `canonical_bytes` or re-encoded bytes — wire bytes only.
- No construction of `PreservedCbor` outside `ade_codec`.
- No raw CBOR decoding in any BLUE crate except `ade_codec` and the
  single allowlisted file `crates/ade_plutus/src/evaluator.rs`.
- No `pallas_*` reference outside `ade_plutus`.
- **(N-A specific)** Carried.
- **(N-B specific)** Carried.
- **(B1 / B2 / B3 / B4 / B5 specific)** Carried.
- **(OQ5 / COMMITTEE / DREP / ENACTMENT-COMMITTEE-WRITEBACK)** Carried.
- **(N-E specific — closed BLUE chokepoint `mempool_ingress`)** Carried.
- **(PP specific — closed BLUE sub-grammar `decode_proposal_procedures`)** Carried.
- **(N-C-S1..S7 specific)** All carried.
- **(N-G-S1..S4 specific)** All carried.
- **(N-H-S1..S6 specific)** All carried.
- **(N-I-S1..S6 specific)** All carried.
- **(N-J-S1..S8 specific)** All carried.
- **(N-K specific)** No new BLUE.
- **(N-L specific)** No new BLUE.
- **(N-L-LIVE / N-M-* / N-O specific)** Per their cluster docs;
  HEAD_DELTAS narrates.
- **(N-P specific — NEW BLUE module tree `ade_crypto::kes_sum/`)**
  - MUST NOT import `cardano_crypto::kes::*` outside `#[cfg(test)]`
    (N9 — KES-scoped; CI-defended by
    `ci_check_kes_sum_compatibility.sh` Guard 3).
  - MUST NOT use `String` in any error variant
    (`KesError` / `KesParseError` are `u32` / `usize` /
    `&'static str` only).
  - MUST NOT expose public byte accessors for any `SigningKey`
    type — `Sum0SigningKey` and `SumSigningKey<D>` carry hot
    secret material.
  - MUST hand-roll `Drop` for any new secret-bearing type
    (best-effort zeroize via `ZeroizingSeed` on sub-seed buffers;
    CI-defended by `ci_check_private_key_custody.sh` with the new
    `kes_sum/` whitelist).
  - MUST match Haskell `cardano-base` `expand_seed` prefix bytes
    (left = `0x01`, right = `0x02`); NOT `cardano-crypto` Rust
    1.0.8 prefixes (`0x00` / `0x01`). CI-defended by
    `ci_check_kes_sum_compatibility.sh` Guard 4.
  - MUST NOT add heuristic period inference. `current_period_of_signing_key`
    returns exactly one `u32` or a closed `KesParseError`
    (DC-CRYPTO-09).
  - MUST NOT silently accept period > 63 tree shapes.
  - MUST NOT construct an upstream `cardano_crypto::kes::SumSigningKey`
    via `unsafe`, `transmute`, vendored `pub(crate)` access, or
    fork-only constructors (N9 — load-bearing).

### GREEN (carried + N-P additions)

- (carried bullets per prior revision, plus N-L-LIVE / N-M-* / N-O
  GREEN additions per their cluster docs)
- **(`ade_crypto::kes_sum::cardano_cli_corpus`, NEW in N-P,
  `#[cfg(test)]`)** Hex-literal `&[u8; 608]` SKEY + `&[u8; 32]`
  VKEY corpus. MUST be preceded by `// TEST ONLY: throwaway
  deterministic fixture generated for Sum6KES` per fixture (OQ4 —
  `feedback_no_credential_leaks`; CI-defended by
  `ci_check_kes_sum_compatibility.sh` Guard 1). MUST NOT commit
  `.skey` envelope files anywhere under `crates/ade_crypto/`
  (Guard 2).
- **(`ade_crypto::kes_sum::tests`, NEW in N-P, `#[cfg(test)]`)**
  35 unit tests including the 64-period chain, expanded-skey
  round-trip at every period, signature round-trip at every
  period, negative tests across 8 wrong-size payloads + 3
  structural defect classes, cardano-cli ground-truth corpus
  cross-impl agreement (3 fixtures), and the
  `cardano-crypto`-Rust-divergence-documentation test. May import
  `cardano_crypto::kes::*` (the `#[cfg(test)]` oracle role per
  N9 KES-scope).

### RED (carried + N-P additions)

- (carried bullets per prior revision, plus N-L-LIVE / N-M-* / N-O
  RED additions per their cluster docs)
- **(`ade_runtime::producer::signing`, extended in N-P)** Hosts
  `pub(super) fn KesSecret::from_blue_signing_key(inner: <Sum6Kes
  as KesAlgorithm>::SigningKey, current_period: KesPeriod) -> Self`.
  MUST keep visibility `pub(super)` — `producer::keys` is the sole
  call site; widening to `pub` permits bypassing the BLUE
  deserializer's structural validation. `KesSecret.inner` is now
  the BLUE-owned signing key (`<Sum6Kes as KesAlgorithm>::SigningKey`);
  `from_bytes_zeroizing` is retired (compat shim removed in N-P S5);
  `from_seed_at_period` (Ade-native flow) and `from_blue_signing_key`
  (cardano-cli flow) are the two construction paths.
- **(`ade_runtime::producer::keys::load_kes_signing_key_skey`,
  extended in N-P)** MUST gate on `payload.len() ==
  Sum6Kes::SIGNING_KEY_SIZE` (608) before calling the BLUE
  deserializer. Wrong-size payloads → `KeyLoadError::UnsupportedExpandedKesKeyFormat`
  (preserves N-O surface). Structural defects →
  `KeyLoadError::KesParse(KesParseError)` (NEW N-P variant). MUST
  NOT construct a `KesSecret` via any path other than
  `from_blue_signing_key` for cardano-cli flavor payloads.
  CI-defended by `ci_check_kes_envelope_closed.sh` Guard 2
  (narrowed).
- **(`ade_runtime/Cargo.toml`, narrowed in N-P)** `cardano-crypto`
  declaration MUST NOT include the `kes-sum` feature
  (`features = ["vrf-draft03", "dsign"]`). The N-P close removes
  `kes-sum`; reintroducing it is a discipline violation.

### Project-specific additions

- **No commits of credentials, hostnames, IPs, private keys** —
  enforced by `ci_check_no_secrets.sh`. **N-P:** the cardano-cli
  ground-truth corpus is hex-literal `&[u8; 608]` constants in
  `cardano_cli_corpus.rs` (`#[cfg(test)]`) with the mandatory
  throwaway-fixture comment per fixture; no `.skey` files committed
  anywhere under `crates/ade_crypto/` (OQ4 rule; Guard 2 of
  `ci_check_kes_sum_compatibility.sh`).
- **No `Phase 4 internal-mode mock network`** — Tier 1 surfaces
  must be exercised against real cardano-node peers. **N-P:** the
  cardano-cli ground-truth corpus is captured from a real
  `cardano-cli 11.0.0.0` invocation (3 throwaway-key fixtures);
  this is the live oracle for the Haskell-equivalence claim.
- **No collapsing wire and canonical bytes** — dual-authority rule.
- **No Tier 5 surface without a stated rationale**.
- **No "we'll match it later" stubs on Tier 1 surfaces** — Tier 1
  closure is hard-gated. **N-P:** the cardano-cli expanded-skey
  loader was a fail-closed Tier-5-adjacent surface in N-O; N-P
  closes it cleanly to "Tier-1 Haskell-equivalent BLUE algorithm
  + Tier-1 canonical serde grammar". No "we'll match it later"
  carve-out remains.

---

## Cross-references

- CODEMAP: `docs/ade-CODEMAP.md` — module-by-module authority table,
  upstream of this document. **Cross-reference check at this HEAD:**
  CODEMAP at HEAD `d62c2bc` reflects the N-L state; CODEMAP
  regeneration against `d6f3399` should add the new BLUE module
  tree `ade_crypto::kes_sum/` (6 production files + 2
  `#[cfg(test)]` files), surface the `ade_runtime`'s `cardano-crypto`
  feature narrowing (drops `kes-sum`), record the new
  `KesSecret::from_blue_signing_key` constructor seam, and bump the
  test inventory + canonical-type count.
- Invariant registry: `docs/ade-invariant-registry.toml` — rule
  families incl. T / CN / DC / OP / RO. **N-P flipped to `enforced`
  (newly introduced):** `DC-CRYPTO-08`
  (`ci_script = ci/ci_check_kes_sum_compatibility.sh,
  ci/ci_check_private_key_custody.sh`); `DC-CRYPTO-09`
  (`ci_script = ci/ci_check_kes_sum_compatibility.sh`).
  **Cleared `open_obligation`:** `OP-OPS-04` (was "cardano-cli
  expanded path deferred to PHASE4-N-P"); `DC-CRYPTO-07` (was
  "fail-closed always until PHASE4-N-P"; statement narrowed to
  describe the new accept-608-valid + fail-close-others policy).
  **Strengthened:** `DC-CRYPTO-03` / `DC-CRYPTO-04` /
  `DC-CRYPTO-05` each gain `strengthened_in += PHASE4-N-P`. Total:
  264 entries.
- Phase 4 cluster plan: `docs/active/phase_4_cluster_plan.md`.
- Tier doctrine: `docs/active/CE-79_gate_statement.md` and
  `docs/active/CE-79_tier5_addendum.md`.
- Cluster N-A / N-B / N-D / N-E / N-C / N-G / N-H / N-I / N-J /
  N-K / N-L / N-L-LIVE / N-M-A / N-M-A1.1 / N-M-B / N-M-C / N-M-FRAG /
  N-M-SCHED / N-M-FOLLOW / N-O / B1 / B2 / B3 / B4 / B5 /
  OQ5-CREDENTIAL-FIDELITY / COMMITTEE-CRED-FIDELITY /
  DREP-VOTE-FIDELITY / ENACTMENT-COMMITTEE-FIDELITY /
  ENACTMENT-COMMITTEE-WRITEBACK / PROPOSAL-PROCEDURES-DECODE:
  all closed; cluster docs carried.
- **Cluster PHASE4-N-P (CLOSED at this HEAD; mechanical wire-layer
  closure of the cardano-cli expanded KES seam)**: the cluster
  doc + closure record at
  `docs/clusters/completed/PHASE4-N-P/{cluster,CLOSURE}.md`. SHIPS
  the BLUE `ade_crypto::kes_sum/` module tree (`mod`, `single`,
  `sum`, `hash`, `period`, `errors`); ships the cardano-cli
  ground-truth corpus + 35 unit tests under `#[cfg(test)]`; ships
  the `KesSecret::from_blue_signing_key` RED-internal constructor;
  closes 2 obligations (`OP-OPS-04.open_obligation` +
  `DC-CRYPTO-07.open_obligation`); flips 2 new registry rules to
  `enforced` (`DC-CRYPTO-08`, `DC-CRYPTO-09`); strengthens 3
  carried (`DC-CRYPTO-03`, `DC-CRYPTO-04`, `DC-CRYPTO-05`); adds
  one CI script (`ci_check_kes_sum_compatibility.sh`) + narrows
  one (`ci_check_kes_envelope_closed.sh` Guard 2); demotes
  `cardano-crypto` to a `#[cfg(test)]` oracle for KES; drops the
  `kes-sum` feature from `ade_runtime`'s `cardano-crypto`
  declaration; preserves the proof obligation doc
  (`period-from-zeroed-sum6-tree-shape-proof.md`).
- **Future cluster — mlocked secret memory (TBD-OPS)** *(declared
  out-of-scope in N-P)*.
- **Future cluster — `CompactSum6Kes`** *(declared out-of-scope in
  N-P; not bounty-priority)*.
- **Future cluster — VRF (`VrfDraft03`) BLUE migration** *(declared
  out-of-scope in N-P; N9 expansion past KES)*.
- **Future cluster — cold Ed25519 BLUE migration** *(declared
  out-of-scope in N-P)*.
- **Future cluster — ChainDb persistence of partially-evolved KES
  keys across process restarts** *(operator-side tooling, declared
  out-of-scope in N-P)*.
- All N-L-anchored future cluster candidates carry forward (TLS /
  authenticated transport; N2C local protocols session driver;
  peer-sharing + tx-submission live half; snapshot schema migration
  v1 → v2 tooling; metrics + observability; snapshot eviction
  policy; multi-peer fork choice; N2C local-chain-sync receive
  surface; pre-Conway snapshot encoder).
