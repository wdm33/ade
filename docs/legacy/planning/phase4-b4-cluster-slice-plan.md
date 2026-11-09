# Cluster/Slice Plan — Ade / PHASE4-B4 (Conway cert-state accumulation fail-closed path)

> **Status**: planning artifact, non-normative. Authority lives in
> `docs/ade-invariant-registry.toml` and `docs/planning/phase4-b4-invariants.md`.
> Produced via `/cluster-plan`. Distinct from the phase-level
> `docs/active/phase_4_cluster_plan.md`, which is **not** modified by this doc.

## Inputs
- `docs/planning/phase4-b4-invariants.md` — B4 sketch (B4-INV-1..3, decisions:
  era-unified, OQ-1 native apply, OQ-6 no config touch, separate cluster).
- `docs/ade-invariant-registry.toml` — NEW `DC-LEDGER-08` (declared); strengthens
  `DC-VAL-06`; anchored by `T-DET-01`.
- B3 (closed): `ade_codec::conway::cert::decode_conway_certs` (closed grammar,
  tags 0..18, `UnknownCertTag`, trailing-bytes-strict); `LedgerError` taxonomy
  (`From<CodecError>`, `EraInvalidCertificate`, `UnsupportedStateDependentDeposit`).
- `~/.claude/methodology/idd.md` Part I §§1–10, Part IV.

> **Detailed authority for this cluster is `docs/clusters/PHASE4-B4/cluster.md`**
> (per-variant effect table, owner-tagged action model, 5-slice breakdown,
> governance exit rule). This plan is the high-level index; where it is terser
> than the cluster doc, the cluster doc and the registry win.

## Cluster Index (Dependency Order)
1. **PHASE4-B4** — Conway cert-state accumulation fail-closed path — primary
   invariant: *for every era, certificate-state accumulation is era-dispatched,
   total over that era's certificate grammar, and fail-closed — no cert decode or
   apply error is swallowed as "non-fatal" when full state is present.* *(detailed below)*
2. **PHASE4-B5** — Conway governance certificate accumulation authority —
   *(declared, future)* — replays cert-derived updates into `ConwayGovState`
   (vote_delegations, committee, committee_hot_keys, drep_expiry) and proves
   equivalence against snapshot/oracle governance state. Consumes B4's enriched
   decoder + owner-tagged effects; this is where `MutateGovernanceState` effects
   become *applied* rather than owner-tagged-out-of-scope. Not implemented in B4.

(B4 + a declared B5. B3 is closed; **B4 consumes and completes the shared Conway
certificate decoder** — B3 used a deposit-accounting projection that drops
delegation/owner payloads (Finding A); B4 requires the same single decoder to
retain all cert-state/governance-owner payloads. No second Conway cert decoder is
allowed. Scoped **out** of B3 to keep authority crisp: B3 owns value-conservation
accounting; B4 owns delegation/pool cert-state accumulation; B5 owns governance.)

---

## Cluster PHASE4-B4 — Conway cert-state accumulation fail-closed path

**Primary invariant:**
> `process_block_certificates` either produces complete canonical `CertState` or a
> deterministic structured `LedgerError`. Conway certs decode through the closed
> Conway grammar (tags 0..18) via explicit era dispatch — never the Shelley
> 6-variant decoder, never reduced into the 7-variant Shelley `Certificate`. Every
> `ConwayCert` variant resolves to exactly one of {`CertState` mutation | explicit
> neutral | structured reject}. No decode or apply error is swallowed. A false
> accept (wrong cert-state silently constructed) is release-blocking.

**Grounding (load-bearing):** the swallow's "non-fatal during replay without full
UTxO state" rationale at `rules.rs:1433` is **false** — `process_block_certificates`
runs *only* at the two `track_utxo == true` call sites (`rules.rs:286`, `rules.rs:369`);
the swallow fires precisely when full state **is** present.

**Era surface (grounded):** `ade_codec` has exactly two cert grammars —
`shelley::cert::decode_certificates` (Shelley→Babbage, identical grammar) and
`conway::cert::decode_conway_certs` (Conway). Byron uses a non-`ShelleyBlock` path and
never reaches this function. "All eras unified" therefore = explicit era dispatch
selecting between these two decoders + their two apply functions.

**Anchors:** NEW `DC-LEDGER-08` (cert-state accumulation closure; declared);
strengthens `DC-VAL-06` (removes the `Err(_) => non-fatal` silent-skip); anchored by
`T-DET-01` (cert-state fingerprint).

**TCB partition:**
- **BLUE:** `ade_ledger::delegation` (NEW `apply_conway_cert`, `ConwayCertAction`,
  `CertApplyError`, `ConwayCertEnv`; `CertState` transitions);
  `ade_ledger::rules::process_block_certificates` (era→decoder dispatch —
  authoritative); reused: `ade_codec::conway::cert`, `ade_codec::shelley::cert`,
  existing `LedgerError` taxonomy.
- **GREEN:** `ade_testkit` cert-state positive + adversarial corpus harness + cert
  mutators.
- **RED:** corpus fixtures; snapshot-derived prior `CertState` + resolved state over
  the Conway-576 window.

**Entry conditions:**
- B3 closed: `decode_conway_certs` (tags 0..18, `UnknownCertTag`, trailing-bytes-strict)
  shipped; `LedgerError` carries `From<CodecError>`, `EraInvalidCertificate`,
  `UnsupportedStateDependentDeposit`.
- Snapshot loader exposes prior `CertState` + resolved state for Conway-576 at
  `track_utxo=true` (`project_snapshot_loader_done`).
- Current `process_block_certificates` uses the Shelley decoder + double swallow +
  ignored `_era` — the thing this cluster removes.

**Cluster Exit Criteria + per-variant table + action model + forbidden list +
replay obligations are resolved in `docs/clusters/PHASE4-B4/cluster.md`** (the
detailed authority). The high-level shape, post-`/cluster-doc`:

**5 CEs / 5 slices (dependency order):**

| Slice | Scope | CE | TCB |
|---|---|---|---|
| **B4-S1** | Complete the shared Conway cert decoder (`ade_codec`): `ConwayCert` + `decode_conway_certs` additively retain all owner payloads (delegation credential, pool id, DRep credential, vote target, committee fields); B3 deposit projection stays valid; single decoder authority. | CE-B4-1 | BLUE |
| **B4-S2** | Native owner-tagged apply model (`ade_ledger`): `ConwayCertAction` / `ConwayCertOutcome` / `apply_conway_cert`, total over 18 variants; composites (10/12/13) split across owners; no Shelley reduction; **no Conway cert is `Neutral`**. Unwired. | CE-B4-2 | BLUE |
| **B4-S3** | Era-dispatched decode boundary in `process_block_certificates`: `CardanoEra` dispatch, `_era` discard gone, decode errors fail closed, owner-tagged gov effects routed out-of-scope. | CE-B4-3 | BLUE |
| **B4-S4** | Remove swallowed apply errors: errors halt the block; no `Err(_)` swallow. | CE-B4-4 | BLUE |
| **B4-S5** | Cert-state replay + adversarial corpus + oracle for the B4-owned surface. | CE-B4-5 | GREEN+RED |

**Key resolved decisions** (full detail in the cluster doc):
- **Finding A (decoder completion):** the B3 decoder is a lossy deposit projection;
  B4 *completes* the single shared decoder (B4-S1), it does not add a second one.
- **Owner-tagged disposition:** governance-affecting certs (vote-deleg 9/10,
  committee 14/15, DRep 16/17/18) are decoded fully and **owner-tagged to
  `ConwayGovState`**, routed out-of-mutation-scope — **not** `Neutral` (owner
  exists → flattening forbidden), **not** `Unsupported`, **not** applied by B4.
- **Composite certs (10/12/13)** carry both a B4-owned `CertState` mutation and an
  owner-tagged gov effect; the apply outcome carries both (binary
  `{Applied|OwnerTaggedOutOfScope}` is insufficient).
- **Governance exit rule (CE-B4-5):** real Conway blocks may carry gov certs; they
  must be owner-tagged, never `Neutral`/`Unsupported`/applied;
  `UnsupportedUntilStateOwner` is unreachable on the real corpus (release-blocking).
- **Fingerprint:** B4-owned `CertState` fingerprint changes — intentional
  `T-DET-01` migration confirmed *correct* against the oracle, not merely stable.
- **Governance accumulation is PHASE4-B5**, not B4.

---

## Authority reminder
Planning aid only. Authority for rules belongs to `docs/ade-invariant-registry.toml`;
for mechanical acceptance, the named tests/CI. If this plan conflicts with the registry
or normative specs, those win.
