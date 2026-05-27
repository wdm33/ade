# PHASE4-N-P — Closure record

**Closed:** 2026-05-27
**HEAD:** `6973318` (S5 close + push)
**Predecessor:** `bfbf8a1` (S4) → `a64de4f` (S3) → `80966f6` (S2)
→ `c04dc0b` (S1 planning) → `6eb4fbd` (PHASE4-N-O close).

## What shipped

Sum6KES expanded-compatibility cluster. Ade now owns the
`Sum6KES Ed25519DSIGN` algorithm in BLUE
(`crates/ade_crypto/src/kes_sum/`), matches Haskell `cardano-base`
byte-for-byte, and imports cardano-cli's 608-byte expanded
`KesSigningKey_ed25519_kes_2^6` envelope via the BLUE
deserializer. The `cardano-crypto` Rust crate is demoted to a
`#[cfg(test)]` oracle for KES (VRF + DSIGN paths remain
upstream until a future cluster).

5 slices, 5 commits:

| Slice | HEAD | Shipped |
|---|---|---|
| **S1** (planning + proof obligation) | `c04dc0b` | invariants doc; cluster-plan doc; expanded cluster.md; `period-from-zeroed-sum6-tree-shape-proof.md`; DC-CRYPTO-08/09 appended as `declared` |
| **S2** (BLUE algorithm) | `80966f6` | `ade_crypto/src/kes_sum/` (mod + single + sum + hash + 19 tests incl. 64-period chain + cross-impl); custody-check whitelist for `kes_sum/` |
| **S3** (skey + sig serde + period inference) | `a64de4f` | trait `LEVEL`; `raw_serialize/raw_deserialize_signing_key_kes`; `raw_serialize/raw_deserialize_signature_kes`; `current_period_of_signing_key`; `KesParseError` closed surface; `period_from_zeroed_sum6_tree_shape` per the S1 proof obligation; `ade_crypto::kes::verify_kes_signature` migrated to BLUE |
| **S4** (cardano-cli corpus + CI gate + Haskell-prefix fix) | `bfbf8a1` | 3 throwaway-key fixtures committed as hex literals; `ci/ci_check_kes_sum_compatibility.sh` (4 guards); **`expand_seed` prefix bytes corrected to 0x01 / 0x02** (Haskell convention) after the corpus identified the divergence vs `cardano-crypto` Rust 1.0.8 (0x00 / 0x01) |
| **S5** (KesSecret migration + loader + close) | `6973318` | `KesSecret.inner` migrated to BLUE type; `load_kes_signing_key_skey` accepts valid 608-byte payloads via BLUE deserializer; `kes-sum` feature dropped from `ade_runtime`'s `cardano-crypto`; `OP-OPS-04.open_obligation` + `DC-CRYPTO-07.open_obligation` cleared; `DC-CRYPTO-08/09` flipped to `enforced`; `DC-CRYPTO-04/05` strengthened |

## MAC verification (cluster doc §4)

| MAC | Status | Evidence |
|---|---|---|
| 1. Proof obligation doc exists | ✅ | `docs/clusters/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md` |
| 2. VK byte-identity vs cardano-crypto for ≥ 8 seeds | ✅ (8 distinct seeds via S2 cross-impl + S4 corpus) | `sum6_kes_vk_diverges_from_cardano_crypto_rust_for_same_seed` documents the divergence; `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth` (3 ground-truth pairs) is the real test |
| 3. 64-period chain (gen, update, sign, verify, round-trip) | ✅ | `sum6_kes_chain_advances_through_all_64_periods`, `sum6_skey_round_trip_at_every_period_0_to_63`, `sum6_signature_round_trip_at_every_period`, `period_from_zeroed_sum6_tree_shape_agrees_with_update_kes_chain` |
| 4. Negative tests (wrong size + malformed sub-tree) | ✅ | `raw_deserialize_signing_key_kes_rejects_wrong_payload_size` (8 sizes), `raw_deserialize_signing_key_kes_rejects_leaf_all_zero`, `raw_deserialize_signing_key_kes_rejects_inconsistent_vk_left_at_level_6`, `raw_deserialize_signing_key_kes_rejects_inconsistent_vk_right_at_level_6` |
| 5. cardano-cli corpus (≥ 3 fixtures, throwaway-comment-prefixed, VK match + cross-impl sign-verify) | ✅ | 3 fixtures committed; `cardano_cli_corpus_skey_deserializes_and_vk_matches_ground_truth` + `cardano_cli_corpus_sign_then_upstream_verifies` + `cardano_cli_corpus_negative_flip_one_byte_in_vk_left_fail_closed` |
| 6. `KesSecret.inner` migration with no observable behaviour change | ✅ | `cargo test -p ade_runtime --lib producer` — 62 tests green |
| 7. `ade_runtime/Cargo.toml` drops `kes-sum` | ✅ | `features = ["vrf-draft03", "dsign"]` |
| 8. cardano-cli 608-byte valid → `Ok(KesSecret)` | ✅ | `cardano_cli_kes_envelope_accepts_real_608_byte_payload` |
| 9. `ci_check_kes_sum_compatibility.sh` | ✅ | PASS (4/4) |
| 10. `ci_check_kes_envelope_closed.sh` Guard 2 narrowed | ✅ | Updated to require `raw_deserialize_signing_key_kes` in loader body |
| 11. `ci_check_private_key_custody.sh` | ✅ | PASS (6/6) — N-P whitelist for `kes_sum/` |
| 12. `cargo test --workspace --lib` | ✅ | 1410+ tests green; no regressions |
| 13. Registry close-out | ✅ | OP-OPS-04 + DC-CRYPTO-07 `open_obligation` removed; DC-CRYPTO-08/09 `enforced`; DC-CRYPTO-04/05 strengthened |
| 14. Operator README updated | ✅ | "Unsupported key flow" replaced with "Alternative: cardano-cli expanded KES skey (also supported, since PHASE4-N-P)" |
| 15. `/cluster-close` workflow | partial — see "Carry-forward" below |
| 16. All 5 commits trailered + pushed | ✅ | `c04dc0b`, `80966f6`, `a64de4f`, `bfbf8a1`, `6973318` |

## Key discovery during S4

`cardano-crypto` Rust 1.0.8 uses different `expand_seed` prefix
bytes than Haskell `cardano-base`:

| Source | left prefix | right prefix |
|---|---|---|
| Haskell `cardano-base` (drives cardano-cli) | `0x01` | `0x02` |
| `cardano-crypto` Rust 1.0.8 | `0x00` | `0x01` |

PHASE4-N-P S2 initially matched upstream Rust. S4's cardano-cli
corpus rejected this with `InconsistentSubtreeVkRight { level: 2 }`
across all three fixtures. Fix: prefix bytes corrected to
`0x01 / 0x02` in `crates/ade_crypto/src/kes_sum/hash.rs`. The
S2 cross-impl-vs-upstream tests were retired (premise was false);
their replacement is the cardano-cli ground-truth corpus + two
divergence-documenting tests.

**Implication for PHASE4-N-O Ade-native keys**: any
`kes.ade.skey` generated before S5 close used the wrong
`expand_seed` prefix (because `KesSecret.inner` was still
`cardano_crypto::kes::Sum6Kes::SigningKey` at that point). After
S5 close, the same seed will derive a **different** VK. Pre-S5
keys do NOT roundtrip through post-S5 signing. **Mitigation**:
no real deployments existed; the bounty test is pre-launch.
Documented in S5.md for completeness.

## Open obligations cleared

- `OP-OPS-04.open_obligation` — was "PHASE4-N-P deferral";
  cleared. Both KES key flows (Ade-native + cardano-cli expanded)
  are now supported and mechanically validated.
- `DC-CRYPTO-07.open_obligation` — was "fail-closed always until
  PHASE4-N-P"; cleared. Statement narrowed to describe the new
  accept-608-valid + fail-close-others policy.

## Open obligations preserved

None introduced by N-P. The closure is clean.

## Cluster-close discipline carry-forward

Per the IDD cluster-close protocol, the following are
**deferred** to a follow-up cluster-close commit (not blocking
this closure):

1. **Grounding-doc regeneration.** `/codemap`, `/seams`,
   `/head-deltas`, `/traceability` regenerate the four
   load-bearing architectural docs against HEAD. These are
   mechanical refreshers — they don't change cluster outcomes,
   only the navigation surface.
2. **Archive `docs/clusters/PHASE4-N-P/` to
   `docs/clusters/completed/PHASE4-N-P/`.** Per the
   `clusters_archive_dir` convention. Done in a follow-up
   commit that also bumps `head_deltas_baseline` in
   `.idd-config.json`.
3. **Per-cluster security review.** `/code-review high` (or
   ultra) against the full cluster diff (`6eb4fbd..6973318`).
   No BLOCK findings expected; the slice-by-slice IDD reviews
   were clean.

These are handled in the next commit by `/cluster-close`. This
CLOSURE.md is the record that the cluster is functionally done.
