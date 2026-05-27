# HEAD Deltas — Ade

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Regenerate via `/head-deltas <baseline>`. Baseline is declared in
> `.idd-config.json` (`head_deltas_baseline`).

> Baseline: `6eb4fbd` (PHASE4-N-O close — Ade-native KES key-gen + cardano-cli expanded skey fail-closed, 2026-05-27 11:39 +0700)
> HEAD: `d6f3399` (PHASE4-N-P close — archive + CLOSURE.md + head_deltas_baseline bump, 2026-05-27 13:19 +0700)
> 6 commits, 32 files changed, +4,850 / −289 lines

> **Baseline shift note.** This regen narrows the baseline from the
> prior `d62c2bc` (PHASE4-N-K close, drove the N-L narrative) to
> `6eb4fbd` (PHASE4-N-O close — the previous cluster's final commit).
> HEAD_DELTAS now narrates **only** the PHASE4-N-P cluster: the
> Ade-owned BLUE `Sum6KES Ed25519DSIGN` algorithm + cardano-cli
> expanded `KesSigningKey_ed25519_kes_2^6` import path, shipped
> across 5 slices (S1 → S5). The intermediate clusters
> N-L-LIVE / N-M-A / N-M-A1.1 / N-M-B / N-M-C / N-M-FRAG /
> N-M-SCHED / N-M-FOLLOW / N-O each had their own HEAD_DELTAS
> regen at their respective closes; their narratives are preserved
> in the archived cluster docs under `docs/clusters/completed/`
> and in the SEAMS / CODEMAP / TRACEABILITY companions.
> `.idd-config.json` `head_deltas_baseline` was bumped from
> `d62c2bc` to `6973318` (the PHASE4-N-P S5 close, which is the
> last load-bearing commit of N-P; `d6f3399` is the chore-only
> archive + bump commit). Per existing convention each regen
> baselines at the **previous** cluster's close, so this doc
> baselines at `6eb4fbd` (N-O close) — the config baseline marks
> where the **next** cluster's narrative will start.

> **Cluster summary.** PHASE4-N-P ships the Ade-owned BLUE
> `Sum6KES Ed25519DSIGN` algorithm + cardano-cli expanded skey
> import, closing the open obligation that PHASE4-N-O explicitly
> deferred. 5 slices, 5 feature commits + 1 chore-archive commit.
> 1 new BLUE module added (`ade_crypto::kes_sum`, 8 source files),
> 5 RED files modified (loader + signer + producer Cargo.toml +
> node binary + testkit), 1 new CI gate added
> (`ci_check_kes_sum_compatibility.sh`) + 2 existing scripts
> updated (`ci_check_kes_envelope_closed.sh`,
> `ci_check_private_key_custody.sh`). 2 new registry rules
> introduced + enforced (`DC-CRYPTO-08`, `DC-CRYPTO-09`); 4
> existing rules strengthened (`DC-CRYPTO-04`, `DC-CRYPTO-05`,
> `DC-CRYPTO-07`, `OP-OPS-04`); 2 open obligations cleared
> (`OP-OPS-04.open_obligation` and `DC-CRYPTO-07.open_obligation`
> were both the same "PHASE4-N-P deferral" carry-forward). 1 new
> closed sum type added (`KesParseError` with 6 variants); 1 new
> KES algorithm error sum (`ade_crypto::kes_sum::KesError`,
> 5 variants). `cardano-crypto` is demoted to a `#[cfg(test)]`
> oracle for KES in `ade_crypto`, and the `kes-sum` Cargo feature
> is dropped from `ade_runtime`'s `cardano-crypto` declaration —
> VRF + DSIGN paths continue to use upstream unchanged (out of
> N-P scope). Key discovery during S4: `cardano-crypto` Rust
> 1.0.8 uses `0x00`/`0x01` `expand_seed` prefix bytes; Ade matches
> Haskell `cardano-base` (`0x01`/`0x02`) — Ade matches cardano-cli
> ground-truth byte-for-byte. No removals; the cluster is purely
> additive to the existing module graph aside from the
> `cardano-crypto` import-set narrowing.

---

## 1. Commit Log

Verbatim from `git log --oneline --no-merges 6eb4fbd..HEAD`. The
PHASE4-N-P slices are committed one-per-slice (not collapsed into
a single cluster-close commit, unlike the N-L convention) so each
slice ID is recoverable from the commit log directly.

| Hash | Type | Summary |
|------|------|---------|
| `d6f3399` | chore | chore(cluster-close): PHASE4-N-P close — archive + CLOSURE.md + head_deltas_baseline bump |
| `6973318` | feat | feat(crypto+runtime): PHASE4-N-P S5 — KesSecret BLUE migration + cardano-cli loader accept + cluster close |
| `bfbf8a1` | feat | feat(crypto+ci): PHASE4-N-P S4 — cardano-cli corpus + Haskell-prefix fix + CI gate |
| `a64de4f` | feat | feat(crypto): PHASE4-N-P S3 — Sum6KES expanded skey + signature serde + period inference |
| `80966f6` | feat | feat(crypto): PHASE4-N-P S2 — Ade-owned BLUE Sum6KES algorithm |
| `c04dc0b` | docs | docs(planning+cluster): PHASE4-N-P planning artifacts + S1 proof obligation |

6 commits total: 1 `docs` (S1 planning + proof obligation), 4
`feat` (S2..S5), 1 `chore` (cluster archive). No `fix` /
`refactor` / `test` commits in this window — the cluster is a
single linear feature stream.

---

## 2. New Modules

| Module | Color | Purpose | Key sub-paths | Added in (cluster/slice) |
|--------|-------|---------|---------------|--------------------------|
| `ade_crypto::kes_sum` | **BLUE** | Ade-owned, from-first-principles reimplementation of the Cardano `Sum6KES Ed25519DSIGN` algorithm. Closed `KesAlgorithm` trait over the recursive `Sum_n` chain (`Sum0 = SingleKes<Ed25519>`, `Sum_n = SumKes<Sum_{n-1}>` for n = 1..6). Pure: no I/O, no wall clock, no `HashMap`, no floats, no RNG; every function is total or returns a closed `KesError` / `KesParseError` variant. Byte-identical to Haskell `cardano-base`'s `Sum6KES Ed25519DSIGN` for `derive_verification_key`, `gen_key_kes_from_seed_bytes`, `update_kes` (all 64 periods), `sign_kes`, `raw_serialize_signing_key_kes`, `raw_serialize_signature_kes`, and their `raw_deserialize_*` inverses; matches cardano-cli ground-truth corpus byte-for-byte. `VerificationKey` pinned to `[u8; 32]` (Ed25519 leaf pk + Blake2b256 sub-tree hashes); `SigningKey` carries hot secret material with hand-rolled `Drop` (via `ZeroizingSeed` per-field on every Sum level + Sum0's leaf seed). Period is uniquely inferable from which sub-seeds are zeroed in the deserialized tree (closed `period_from_zeroed_sum6_tree_shape` per the S1 proof obligation). `cardano-crypto` is `#[cfg(test)]` only inside `kes_sum/` (cross-impl divergence-documenting tests + nothing else). | `mod.rs` (`KesAlgorithm` trait, `Sum1Kes..Sum6Kes` aliases, `KesError`), `single.rs` (`Sum0Kes`, `Sum0SigningKey`, `Sum0Signature` — Ed25519 leaf), `sum.rs` (`SumKes<D>`, `SumSigningKey<D>` w/ `ZeroizingSeed` per-field Drop, `SumSignature<D>` `(sigma_d, vk0, vk1)` tuple, `raw_serialize/raw_deserialize_signing_key_kes`, `raw_serialize/raw_deserialize_signature_kes`, `current_period_of_signing_key`), `hash.rs` (`expand_seed` w/ Haskell-prefix `0x01`/`0x02`, `hash_pair` Blake2b256 vk concatenation), `period.rs` (`period_from_zeroed_sum6_tree_shape` — the closed period-inference fn), `errors.rs` (`KesParseError` closed 6-variant sum: `WrongPayloadSize`, `LeafSignKeyAllZero`, `InconsistentSubtreeVkLeft { level }`, `InconsistentSubtreeVkRight { level }`, `LevelOutOfRange { level }`, `InvalidEd25519SignatureLength`), `cardano_cli_corpus.rs` (`#[cfg(test)]` — 3 throwaway 608-byte SKEY + VKEY fixtures generated by `cardano-cli 11.0.0.0`), `tests.rs` (`#[cfg(test)]` — 35 unit tests covering recurrence sizes, 64-period chain, round-trips, negative shapes, ground-truth corpus, two divergence-documenting tests vs `cardano-crypto` Rust 1.0.8) | PHASE4-N-P / S2 (algorithm) + S3 (skey/sig serde + period inference) + S4 (corpus + Haskell-prefix fix) |

No new workspace crates. Workspace member count unchanged.
`ade_crypto::kes_sum` is added under the existing BLUE `ade_crypto`
crate; it sits alongside `ade_crypto::kes` (the upstream-wrapper
verification API — already BLUE) and is wired into `ade_crypto::kes::verify_kes`
in S3 (PHASE4-N-P S3 migrated the public `verify_kes` entry to
the BLUE algorithm).

Cross-reference: the new module must be reflected in CODEMAP
under §BLUE (`ade_crypto::kes_sum`). The current
`docs/ade-CODEMAP.md` predates N-P and does not mention `kes_sum`
— regenerate via `/codemap`.

---

## 3. Modules Modified

| Module | Scope | Key changes |
|--------|-------|-------------|
| `ade_crypto::lib` | +1 line | Adds `pub mod kes_sum;` declaration. Pure module-graph addition. |
| `ade_crypto::kes` | +6 / −9 lines | S3 migration: `verify_kes` and `verify_kes_signature` now route through the BLUE `ade_crypto::kes_sum::Sum6Kes` instead of `cardano_crypto::kes::Sum6Kes`. Signature parsing surface tightened: `raw_deserialize_signature_kes` returns a `Result` in our impl (vs. `Option` upstream) — error mapped to the existing `CryptoError::MalformedSignature`. Function bodies untouched; no public surface change observable to callers. |
| `ade_runtime::producer::signing` | +43 / −20 lines | S5 migration: `KesSecret.inner` type changed from `<cardano_crypto::kes::Sum6Kes as KesAlgorithm>::SigningKey` to `<ade_crypto::kes_sum::Sum6Kes as KesAlgorithm>::SigningKey`. New constructor `KesSecret::from_blue_signing_key(inner, current_period)` (`pub(super)`, used by the cardano-cli loader). `verification_key_fingerprint` simplified — our BLUE algorithm returns `[u8; 32]` directly (no `raw_serialize_verification_key_kes` intermediate copy). `kes_sign` / `kes_update` call signatures simplified (BLUE algorithm takes `&SigningKey`/`SigningKey` without the unit context, `period: u32` rather than `u64`). `SigningError::CardanoCrypto` variant retained for VRF (still upstream); new `SigningError::AdeKesSum(KesError)` variant added for the migrated KES path. `Drop` policy comment updated — per-field `ZeroizingSeed` Drop carries the load now. |
| `ade_runtime::producer::keys` | +44 / −15 lines | S5: `load_kes_signing_key_skey` policy flip. PHASE4-N-O fail-closed every cardano-cli expanded payload via `KeyLoadError::UnsupportedExpandedKesKeyFormat`; PHASE4-N-P S5 narrows that variant to wrong-size-only and routes structurally-valid 608-byte payloads through `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes`, constructing a `KesSecret` via the new `from_blue_signing_key` constructor with `current_period_of_signing_key`. New `KeyLoadError::KesParse(KesParseError)` variant for the structurally-invalid-but-right-size case. 3 new tests (`cardano_cli_kes_envelope_accepts_real_608_byte_payload`, `cardano_cli_kes_envelope_rejects_synthetic_608_byte_payload`, `cardano_cli_kes_envelope_rejects_608_byte_leaf_zero_payload`); 1 renamed (`..._rejects_608_byte_payload` → `..._rejects_synthetic_608_byte_payload`). |
| `ade_runtime::Cargo.toml` | +6 / −3 lines | S5: dropped `"kes-sum"` from the `cardano-crypto` feature list. After N-P, `ade_runtime` declares `cardano-crypto = { features = ["vrf-draft03", "dsign"] }` — KES is fully BLUE-owned via `ade_crypto::kes_sum` (a workspace path dependency, already declared). VRF + DSIGN remain upstream out-of-scope concerns. |
| `ade_node::key_gen` | +1 line | One new branch in `classify_err`: `KeyLoadError::KesParse(_) => "cardano-cli expanded KES skey parse error"`. Additive; no other change. |
| `ade_testkit::producer::reference_vectors` | +30 / −17 lines | KES reference-set materialization migrated to the BLUE algorithm. Function signatures adjusted to N-P's `u32`-period + no-unit-context API. The `cardano_crypto::kes::Sum6Kes` import is removed; `cardano_crypto::vrf::VrfDraft03` remains (VRF is out of N-P scope). Module doc spells out the load-bearing implication: the reference signatures **have changed at the byte level vs the N-C-era frozen set** because Ade's `expand_seed` prefix now matches Haskell (0x01/0x02) rather than `cardano-crypto` Rust 1.0.8 (0x00/0x01); the new frozen set matches cardano-cli ground truth. |
| `docs/ade-invariant-registry.toml` | +60 / −24 lines | 2 new rules appended (`DC-CRYPTO-08`, `DC-CRYPTO-09`, both `enforced`); 4 existing rules rewritten + `strengthened_in += "PHASE4-N-P"` applied (`DC-CRYPTO-04`, `DC-CRYPTO-05`, `DC-CRYPTO-07`, `OP-OPS-04`); 2 `open_obligation` fields cleared (the carry-forwards on `DC-CRYPTO-07` and `OP-OPS-04`, both of which read "PHASE4-N-P deferral" at baseline). Closure record fully reflected at HEAD. |
| `docs/active/op-ops-04-ade-native-kes-flow.md` | +110 / −68 lines | Operator-facing README rewrite. Title generalized ("KES operator flow (Ade-native + cardano-cli expanded)"); the "Unsupported key flow" section retired and replaced with "Alternative: cardano-cli expanded KES skey (also supported, since PHASE4-N-P)" + a "Why this changed in PHASE4-N-P" rationale block. Audience and source attribution updated; the bounty-facing claim boundary is now "both flows supported and mechanically validated against cardano-cli ground truth". |
| `.idd-config.json` | +1 / −1 lines | `head_deltas_baseline` bumped from `d62c2bc` to `6973318` (PHASE4-N-P S5 close). The `_head_deltas_baseline_doc` companion key was **not** updated in this bump and still describes the N-L narrative — see Anomalies §. |

No other source modules were touched in the `crates/` tree. The
cluster is **almost purely additive** to the existing module graph:
one new BLUE sub-tree (`ade_crypto::kes_sum`, 8 files), 7 modified
RED/BLUE files (loader + signer + lib.rs + kes.rs + reference
vectors + key_gen + Cargo.toml), one CI script added + 2 modified.
The single non-additive change is the `kes-sum` Cargo feature drop
on `ade_runtime`'s `cardano-crypto` dependency — a deliberate
demotion (cardano-crypto KES is no longer in `ade_runtime`'s
production graph; it remains under `#[cfg(test)]` in `ade_crypto`
for cross-impl validation only).

The deleted `docs/clusters/PHASE4-N-P/cluster.md` (and the 7
sibling slice/proof docs that moved with it to
`docs/clusters/completed/PHASE4-N-P/`) appear as rename pairs in
`git diff --name-status`. These are routine cluster-close archival
moves, not deletions of new content.

---

## 4. Feature Flags

No Cargo `[features]` table is declared in `ade_crypto`,
`ade_runtime`, or any other workspace crate at baseline or at HEAD.
No new Ade-side feature flags introduced; no existing flags
modified.

**The single Cargo-feature change in this cluster is on an
upstream dependency**, not on an Ade workspace crate:

| Crate dep | Feature | Module | Status |
|-----------|---------|--------|--------|
| `cardano-crypto` | `kes-sum` | `ade_runtime/Cargo.toml` | **Removed** (S5). `ade_runtime`'s `cardano-crypto` features narrow from `["vrf-draft03", "kes-sum", "dsign"]` to `["vrf-draft03", "dsign"]`. The deliberate intent: cardano-crypto's KES surface is no longer compiled into the `ade_runtime` production graph. KES routing is now via `ade_crypto::kes_sum` (BLUE, in-workspace). |
| `cardano-crypto` | `kes-sum` | `ade_crypto` (test-only oracle) | **Test-only retention**. cardano-crypto is gated `#[cfg(test)]` inside `ade_crypto::kes_sum/` (via `cardano_cli_corpus.rs` and `tests.rs`); the production code path in `ade_crypto/src/kes.rs` no longer imports `cardano_crypto::kes::*`. CI gate `ci/ci_check_kes_sum_compatibility.sh` Guard 3 mechanically asserts the `#[cfg(test)]` confinement. |

No coupling between the two: the `ade_runtime` drop is a pure
build-graph narrowing; the `ade_crypto` `#[cfg(test)]` confinement
is a CI-enforced structural discipline.

The cluster also adds three closed BLUE constants that pin
deterministic algorithm sizes (referenced here for completeness;
they are not Cargo features):

| Constant | Module | Purpose | Status |
|----------|--------|---------|--------|
| `Sum6Kes::SIGNING_KEY_SIZE = 608` | `ade_crypto::kes_sum` | Canonical Sum6KES expanded signing-key payload size (Haskell `rawSerialiseSignKeyKES (Sum6KES Ed25519DSIGN)`). Cross-checked against cardano-cli ground-truth corpus. | **New** since baseline |
| `Sum6Kes::SIGNATURE_SIZE = 448` | `ade_crypto::kes_sum` | Canonical Sum6KES signature payload size (`SUM6_KES_SIG_LEN` in `ade_crypto::kes`; same value). | **New** since baseline |
| `Sum6Kes::TOTAL_PERIODS = 64` | `ade_crypto::kes_sum` | Canonical period count for the depth-6 Sum chain (`2^6`). | **New** since baseline |

---

## 5. CI Checks

### PHASE4-N-P Sum6KES compatibility — 1 new script + 2 extended (`ci/ci_check_*.sh` 89th)

| Check | Status | What it checks |
|-------|--------|----------------|
| `ci/ci_check_kes_sum_compatibility.sh` | **New** (S4) — script 89 | Enforces `DC-CRYPTO-08` + `DC-CRYPTO-09`. **Guard 1**: cardano-cli ground-truth corpus exists at `crates/ade_crypto/src/kes_sum/cardano_cli_corpus.rs` with ≥ 3 `pub(super) const SKEY{N}: &[u8; 608]` fixtures, each preceded by a `"TEST ONLY: throwaway deterministic fixture generated for Sum6KES"` comment. **Guard 2**: no `.skey` envelope files committed under `crates/ade_crypto/`. **Guard 3 (N9)**: `cardano_crypto` is imported in `crates/ade_crypto/src/**` only inside `#[cfg(test)]` blocks — no production-code path may re-introduce the upstream KES surface in `ade_crypto`. **Guard 4**: the `expand_seed` prefix bytes in `kes_sum/hash.rs` are `0x01` / `0x02` (Haskell convention), NOT `0x00` / `0x01` (`cardano-crypto` Rust 1.0.8). A literal-byte check surfaces drift faster than the cardano-cli corpus test would. |
| `ci/ci_check_kes_envelope_closed.sh` | **Modified** (S5) | Original PHASE4-N-O body asserted the cardano-cli loader return `UnsupportedExpandedKesKeyFormat` for every payload and **not** construct a `KesSecret`. PHASE4-N-P S5 narrows: (a) `UnsupportedExpandedKesKeyFormat` is still asserted (for the size-mismatch path); (b) **new**: `raw_deserialize_signing_key_kes` must be called inside the loader body (the BLUE structural validator drives every accept); (c) `KesSecret::from_bytes_zeroizing` / `from_seed_at_period` are still forbidden in the loader body (those would bypass the structural validator). |
| `ci/ci_check_private_key_custody.sh` | **Modified** (S2) | Original PHASE4-N-C body asserts no `*SigningKey` types are defined in BLUE and no `cardano-crypto` signing API is called in BLUE production code. PHASE4-N-P extends both guards with an explicit `KES_SUM_DIR` whitelist: files under `crates/ade_crypto/src/kes_sum/` are exempt because that directory legitimately hosts the Ade-owned `Sum0SigningKey` / `SumSigningKey<D>` types and recursively calls our own `Sum_n::sign_kes`. The N9 hard prohibition (no upstream-shim in production) is enforced by `ci_check_kes_sum_compatibility.sh` Guard 3 — the two scripts compose. |

Total CI script count: **88 → 89** (`ci/ci_check_*.sh`). 1 new
script; 2 modified scripts; no removals — the cluster strictly
appends + extends. (Other files under `ci/` such as
`build_consensus_inputs_bundle.sh`, `mithril_restore_preprod_peer.sh`,
and `git-hooks/` are unchanged.)

TRACEABILITY cross-reference: the new script
`ci_check_kes_sum_compatibility.sh` appears as a `ci_script` on 4
distinct rules at HEAD (`DC-CRYPTO-04`, `DC-CRYPTO-05`,
`DC-CRYPTO-07`, `DC-CRYPTO-08`, `DC-CRYPTO-09`, `OP-OPS-04`); the
modified `ci_check_kes_envelope_closed.sh` retains its edge to
`DC-CRYPTO-07` + `OP-OPS-04`; the modified
`ci_check_private_key_custody.sh` retains its broader custody
edges (`DC-CRYPTO-03..05`, `OP-OPS-04`, `T-KEY-01`). Re-traced via
`ci/ci_check_constitution_coverage.sh` — expected to pass at HEAD.

---

## 6. Canonical Type Registry Delta

n/a — `.idd-config.json` `canonical_type_registry` is null.
Canonical-type rules live inline in the invariant registry under
family `T`.

**PHASE4-N-P introduced new closed sum types** in support of the
BLUE Sum_n algorithm + the loader's structural-validation path:

- `ade_crypto::kes_sum::KesError` — closed BLUE algorithm error
  surface (5 variants: `InvalidSeedLength { expected, actual }`,
  `PeriodOutOfRange { period, max_period }`, `VerificationFailed`,
  `KeyExpired`, `Ed25519(&'static str)`). No raw key bytes in any
  variant; `Ed25519` carries only a static-string detail.
- `ade_crypto::kes_sum::KesParseError` — closed BLUE parse-time
  error surface (6 variants: `WrongPayloadSize { actual }`,
  `LeafSignKeyAllZero`, `InconsistentSubtreeVkLeft { level }`,
  `InconsistentSubtreeVkRight { level }`, `LevelOutOfRange { level }`,
  `InvalidEd25519SignatureLength { actual }`). Per the S1 proof
  obligation §5, every variant carries only `u32`/`usize`
  primitives — no hex, no seed material.
- `ade_crypto::kes_sum::Sum0SigningKey` — BLUE Ed25519 leaf
  signing key, hand-rolled `Drop` zeroes the 32-byte seed.
- `ade_crypto::kes_sum::Sum0Signature` — 64-byte Ed25519 signature
  newtype (closed leaf signature shape).
- `ade_crypto::kes_sum::SumSigningKey<D>` — recursive Sum-level
  signing-key generic, per-field `ZeroizingSeed` Drop on each of
  the 6 levels' `r1_seed` field.
- `ade_crypto::kes_sum::SumSignature<D>` — closed
  `(sigma_d, vk0, vk1)` tuple at every Sum level.

Plus one additive variant on each of two previously-closed
RED-shell sums:

- `ade_runtime::producer::keys::KeyLoadError::KesParse(KesParseError)`
  — additive extension. All existing match sites updated
  (`ade_node::key_gen::classify_err`). Existing variants
  byte-identical.
- `ade_runtime::producer::signing::SigningError::AdeKesSum(KesError)`
  — additive extension. Existing `CardanoCrypto` variant retained
  for VRF (still upstream). All call-sites in `kes_sign`,
  `kes_update`, `KesSecret::from_bytes_zeroizing` re-pointed to
  the new variant; no surface broken.

Plus the canonical authority sites that are now SOLE-authority for
KES:

- `ade_crypto::kes_sum::Sum6Kes` — SOLE BLUE Sum6KES algorithm in
  the workspace (CN-CRYPTO authority moves from upstream wrapper
  into our BLUE).
- `ade_crypto::kes::verify_kes` (existing) — SOLE KES-signature
  verification entry point; now routes through the BLUE algorithm.
- `ade_runtime::producer::keys::load_kes_signing_key_skey`
  (existing) — SOLE cardano-cli expanded-skey loader; now routes
  608-byte payloads through the BLUE deserializer.

**Removals: 0** (expected under append-only discipline).

Exact whole-project type recount belongs to the TRACEABILITY regen
that follows this HEAD_DELTAS.

---

## 7. Normative Rule Delta

The project's invariant registry tracks structured rules (TOML),
not prose normative-doc rules; this section reports on it.

- Rules at baseline (`6eb4fbd:docs/ade-invariant-registry.toml`): **262**
- Rules at HEAD (`HEAD:docs/ade-invariant-registry.toml`): **264**
- Net additions: **2** (`DC-CRYPTO-08`, `DC-CRYPTO-09`).
- Removals: **0** (expected under append-only discipline; clean).

- **New rules (2) at HEAD:**
  - **`DC-CRYPTO-08` `enforced`** — Ade-owned Sum6KES algorithm is
    Haskell-equivalent. `ade_crypto::kes_sum::Sum6Kes` is byte-
    identical to Haskell `cardano-base`'s `Sum6KES Ed25519DSIGN`
    for `derive_verification_key`, `gen_key_kes_from_seed_bytes`,
    `update_kes` (all 64 periods), `sign_kes`. Cross-impl
    validation against the cardano-cli ground-truth corpus is
    mechanically enforced under `#[cfg(test)]`. The N9 hard
    prohibition (no upstream-shim in production via unsafe /
    transmute / vendored pub(crate) access / fork-only
    constructors) is enforced by
    `ci/ci_check_kes_sum_compatibility.sh` Guard 3. 18 tests
    listed. `ci_check_kes_sum_compatibility.sh` + `ci_check_private_key_custody.sh`.
  - **`DC-CRYPTO-09` `enforced`** — Sum6KES expanded signing-key
    serde and period inference. `raw_serialize_signing_key_kes` /
    `raw_deserialize_signing_key_kes` byte-identical to Haskell's
    `rawSerialiseSignKeyKES` / `rawDeserialiseSignKeyKES`. Closed
    608-byte on-disk format; any other size → `WrongPayloadSize`.
    Period uniquely inferable from zeroed sub-seeds (no heuristic;
    closed-error otherwise). Round-trip preserves period for every
    period 0..=63. Malformed sub-trees → closed `KesParseError`
    variant; no best-effort guesswork. 15 tests listed.
    `ci_check_kes_sum_compatibility.sh`.

- **Strengthenings recorded at HEAD by PHASE4-N-P:**
  - **`DC-CRYPTO-04.strengthened_in += "PHASE4-N-P"`** — KES
    signing transcript equivalence statement extended: "byte-
    identical to **Haskell cardano-base's Sum6KES reference**"
    (was previously implicit "reference"), and "after PHASE4-N-P
    S5 the algorithm is BLUE-owned". Cross-impl agreement now
    mechanically validated against cardano-cli ground truth via
    `DC-CRYPTO-08`. New test `cardano_cli_corpus_sign_then_upstream_verifies`
    added to the rule's `tests` array. `ci_script` extended with
    `ci_check_kes_sum_compatibility.sh`.
  - **`DC-CRYPTO-05.strengthened_in += "PHASE4-N-P"`** — KES
    evolution discipline statement extended: "after PHASE4-N-P
    S5 the underlying algorithm (`update_kes`) is BLUE-owned;
    per-field zeroize on Drop of consumed sub-seeds is implemented
    via `ZeroizingSeed` (DC-CRYPTO-08)". New test
    `zeroizing_seed_drop_overwrites_bytes`. `code_locus` extended
    with `crates/ade_crypto/src/kes_sum/sum.rs (SumKes::update_kes + ZeroizingSeed Drop)`.
    `ci_script` extended with `ci_check_kes_sum_compatibility.sh`.
  - **`DC-CRYPTO-07.strengthened_in = ["PHASE4-N-P"]`** —
    cardano-cli envelope rule fully restated: from "fail-closed
    always" (PHASE4-N-O) to "accept structurally-valid 608-byte
    payloads via BLUE deserializer; fail-close all other shapes
    via closed `KesParseError` variants". `open_obligation`
    cleared (was the "PHASE4-N-P deferral" carry-forward).
    `tests` array expanded to 6 entries; `ci_script` extended
    with `ci_check_kes_sum_compatibility.sh`.
  - **`OP-OPS-04.strengthened_in += "PHASE4-N-P"`** (now
    `["PHASE4-N-O", "PHASE4-N-P"]`) — operator-supplied keys rule
    extended to declare both KES key flows (Ade-native +
    cardano-cli expanded) supported and routing the cardano-cli
    path through the Ade-owned BLUE deserializer. `open_obligation`
    cleared (was the "PHASE4-N-P deferral" carry-forward; matches
    the `DC-CRYPTO-07` clearance — the same deferral text was
    duplicated in both rules at baseline). `tests` and
    `ci_script` arrays grew to reflect the new cardano-cli
    loader tests + the new compatibility CI script.

- **Open obligations status at HEAD:**
  - **`OP-OPS-04.open_obligation`** — **CLEARED**. Was: "PHASE4-N-P
    deferral only: cardano-cli's 608-byte expanded
    `KesSigningKey_ed25519_kes_2^6` payload is not yet importable".
    Now the rule statement directly enumerates both supported flows.
  - **`DC-CRYPTO-07.open_obligation`** — **CLEARED**. Was: "PHASE4-N-P
    deferral: cardano-cli expanded `Sum6KES` import becomes
    supported when `ade_crypto::kes_sum` ships". Now the rule
    statement directly describes the accept-608-valid policy.
  - **`RO-LIVE-01.open_obligation`** = `blocked_until_operator_peer_available`
    — carried forward from PHASE4-N-G. Unchanged.
  - **`RO-LIVE-02.open_obligation`** = `blocked_until_operator_peer_available`
    — carried forward from PHASE4-N-H. Unchanged.
  - **`RO-LIVE-03.open_obligation`** = `blocked_until_operator_peer_available`
    — carried forward from PHASE4-N-L. Unchanged.
  - **`CN-CONS-06.open_obligation`** = `blocked_until_operator_stake_available`
    — carried forward from PHASE4-N-C. Unchanged.
  - **`DC-STORE-09.open_obligation`** = `snapshot_schema_migration_follow_on_cluster`
    — carried forward from PHASE4-N-K. Unchanged.
  - (`DC-EVIDENCE-01` and `RO-LIVE-05` were flipped to `enforced`
    earlier in PHASE4-N-M-SCHED + N-M-FOLLOW; not relevant to this
    cluster.)

---

## Anomalies and Cross-Reference Warnings

- **No canonical-type or invariant-rule removals.** Append-only
  discipline preserved across the cluster.
- **No conventional-commits violations.** All 6 commits follow
  `<type>(<scope>): <subject>` shape — 1 `docs(planning+cluster)`,
  4 `feat(<scope>)`, 1 `chore(cluster-close)`.
- **Stale config doc comment.** `.idd-config.json`'s
  `_head_deltas_baseline_doc` companion comment was **not**
  updated when `head_deltas_baseline` was bumped from `d62c2bc`
  to `6973318`. The comment still reads "PHASE4-N-L baseline …
  HEAD_DELTAS narrates the PHASE4-N-L cluster". Cosmetic; no
  mechanical effect. Reconcile at the next regen-touch of the
  config file. The closure commit `d6f3399` introduced this
  staleness — the bump touched the keyed value but not the
  adjacent `_*_doc` companion.
- **Pre-S5 Ade-native KES keys are byte-incompatible with
  post-S5 signing.** The S4 cardano-cli ground-truth discovery
  forced the `expand_seed` prefix correction from
  `0x00`/`0x01` (cardano-crypto Rust 1.0.8) to `0x01`/`0x02`
  (Haskell `cardano-base`). Any `kes.ade.skey` envelope generated
  by `ade_node --mode key_gen_kes` **before** S5 close used the
  wrong prefix (because `KesSecret.inner` was still
  `cardano_crypto::kes::Sum6Kes::SigningKey` at that point); the
  same seed will now derive a **different** VK. Mitigation: no
  real deployments existed at S5 close; the bounty test is
  pre-launch. Documented in
  `docs/clusters/completed/PHASE4-N-P/S5.md` and
  `docs/clusters/completed/PHASE4-N-P/CLOSURE.md` § "Key
  discovery during S4".
- **Retired S2 cross-impl-vs-upstream tests.** S2 originally
  shipped tests asserting our impl agreed with `cardano-crypto`
  Rust 1.0.8 byte-for-byte. S4 disproved that premise (the
  upstream Rust crate uses the wrong prefix). The S2 tests were
  retired; replacement is the cardano-cli ground-truth corpus
  + two **divergence-documenting** tests
  (`sum6_kes_seed_expansion_diverges_from_cardano_crypto_rust_1_0_8`,
  `sum6_kes_vk_diverges_from_cardano_crypto_rust_for_same_seed`).
  This is a deliberate inversion: we now mechanically assert the
  divergence rather than the agreement. Not a discipline gap;
  load-bearing for keeping the prefix correct.
- **CODEMAP cross-reference**: the new `ade_crypto::kes_sum`
  BLUE module must appear in CODEMAP §BLUE. The current
  `docs/ade-CODEMAP.md` contains no mention of `kes_sum`
  — regen via `/codemap`. Likely also stale on the N-L / N-M
  surface (the existing doc references the PHASE4-N-H S1 era
  closure-script entries); a full regen is in order.
- **SEAMS cross-reference**: `KesError` and `KesParseError` are
  new closed BLUE error sums (closed registries); `KesAlgorithm`
  is a closed trait surface for the Sum_n chain (2 associated types + 6
  associated consts + 11 methods). SEAMS should classify them
  under closed registries / closed traits. Regen via `/seams` if
  absent.
- **TRACEABILITY cross-reference**: the 1 new CI script + 2
  modified scripts (§5) and the 2 new rules + 4 strengthenings
  (§7) must appear in TRACEABILITY. If absent at the next read,
  regen via `/traceability`.
- **Honest-scope note (BLUE algorithm).** `ade_crypto::kes_sum`
  ships only what cardano-base's `Sum6KES Ed25519DSIGN` ships:
  the algorithm + serde + period inference + cross-impl
  validation. VRF (`VrfDraft03`) and cold-key signing (Ed25519
  DSIGN) remain on upstream `cardano-crypto` until a separate
  future cluster picks them up. This is a deliberate scope cap,
  not a discipline gap — the cluster-doc and
  `docs/active/op-ops-04-ade-native-kes-flow.md` both flag it.
- **Honest-scope note (Cargo demotion).** `cardano-crypto` is
  demoted to `#[cfg(test)]` only **inside `ade_crypto`** and
  feature-narrowed (no `kes-sum` feature) **inside `ade_runtime`**.
  It is still a compile-time dependency of `ade_runtime`'s
  production graph (for VRF + DSIGN) and of `ade_testkit`. A
  full removal of cardano-crypto from production is out of
  scope for N-P and is a separate future cluster.

---

## Generation Notes

This regen was produced by `/head-deltas 6eb4fbd` against the
PHASE4-N-P close working tree at `HEAD = d6f3399`. The baseline
was set to `6eb4fbd` (PHASE4-N-O close) per the established
per-cluster cadence — each grounding regen baselines at the
previous cluster's close so the narrative stays narrow and
reviewable per-cluster. `.idd-config.json` `head_deltas_baseline`
itself was bumped earlier in `d6f3399` from `d62c2bc` to
`6973318` (the PHASE4-N-P S5 close — i.e. where the **next**
cluster's narrative will start); that bump is intentional and
distinct from this doc's baseline choice. Future regens should
continue to baseline at the previous cluster's close.

Mechanical inputs:
- `git log --oneline --no-merges 6eb4fbd..HEAD` → §1 (6 commits).
- `git diff --name-status 6eb4fbd..HEAD` → §2 + §3.
- `git diff --numstat 6eb4fbd..HEAD -- crates/<crate>/` → §3 scope column.
- `crates/ade_runtime/Cargo.toml` diff → §4 (no Ade-side Cargo
  features changed; cardano-crypto `kes-sum` feature dropped).
- `ls ci/ci_check_*.sh` vs `git ls-tree -r --name-only 6eb4fbd ci/`
  → §5 (88 → 89; 2 modified, 1 new).
- `git diff 6eb4fbd -- docs/ade-invariant-registry.toml` + entry
  count (`grep -c '^\[\[rules\]\]'`) → §7 (262 → 264; 2 new rules,
  4 strengthenings, 0 removals, 2 open obligations cleared).
- `docs/clusters/completed/PHASE4-N-P/{cluster,CLOSURE,S1..S5}.md`
  → cluster-summary header, Modules Modified narrative,
  registry-delta cross-checks, S4 prefix-discovery anomaly.
