# Conway governance-state accumulation corpus (PHASE4-B5, DC-LEDGER-09)

Backs `crates/ade_ledger/tests/gov_state_corpus.rs`. The certificates are built
in-test as typed `ConwayCert` values (and minimal CBOR for decode-layer
adversarial cases), so this directory holds only this README.

## What is mechanically closed (CE-5 / CE-6)

1. **Positive (synthetic):** a real-shaped Conway governance-cert sequence (vote
   delegation → committee hot-key authorization → DRep registration) accumulates
   into the correct `ConwayGovState` — `vote_delegations[cred] = drep`,
   `committee_hot_keys[hot] = cold`, `drep_expiry[cred] = current_epoch +
   drep_activity` — under a controlled base state and `GovCertEnv`.
2. **Replay:** the accumulation is byte-identical across two runs, asserted over
   the canonical gov-state fingerprint surface (`fingerprint.governance` and
   `.combined`) — `T-DET-01`.
3. **Adversarial (no false accept):**
   - a DRep register/update with the env absent fails fast
     (`MissingDRepActivityParam`) — never a defaulted expiry;
   - decode-layer hazards (unknown tag ≥ 19, truncated array) reject before any
     gov apply; a removed tag (5/6) is not a governance mutation;
   - a double committee resignation is idempotent and deterministic.

The block-path wiring (`accumulate_tx_certs`) is additionally covered in
`crates/ade_ledger/src/rules.rs` (`gov_accumulation_applies_drep_registration_into_gov_state`,
`gov_apply_error_halts_accumulation`): the governance half is applied into
`gov_state`, and a gov apply error halts the block transition fail-closed.

## Fingerprint migration (T-DET-01, intentional)

`gov_state` is now cert-accumulated and carried forward through `apply_block`
(PHASE4-B5-S3), rather than nulled at every block. Separately, the params
fingerprint gained `drep_activity` (PHASE4-B5-S1, Conway-deposit tag extended
2 → 3). Both are deliberate, documented migrations.

## Environment-blocked open obligation (NOT closed here)

The **real epoch-576 governance-state (VState)-vs-cardano-node oracle** is
environment-blocked, identical to the B4-S5 / B3-S5 constraint: the epoch-576
ledger-state / UMap snapshot was deleted post-extraction and is **not** in this
repo (see `corpus/cert_state/README.md` and
`corpus/validity/conway_epoch576/README.md`). Real Conway governance certs cannot
be replayed into gov-state at `track_utxo=true` here because the prior
delegation/registration state and the loaded `ConwayGovState` base would not
resolve from blocks/epochs absent locally.

Per the project tier doctrine, real-chain gov-state agreement for
`DC-LEDGER-09` is **reclassified environment-blocked**; the synthetic positive +
replay + adversarial gate above is the mechanical closure that B5 ships. This
mirrors `DC-LEDGER-08` / `DC-TXV-06` / `DC-TXV-03`, whose real oracles are the
same documented open obligation.
