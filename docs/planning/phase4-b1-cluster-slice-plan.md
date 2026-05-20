# Cluster/Slice Plan — Ade / Workstream B

> **Status**: planning artifact, non-normative. Authority lives in
> `docs/ade-invariant-registry.toml` and the invariant sketch
> `docs/planning/phase4-b1-invariants.md`. Produced via `/cluster-plan`.

## Inputs

- `docs/planning/phase4-b1-invariants.md` — B1 invariant sketch (incl. §8
  investigation results: consensus-input provenance + fail-closed audit).
- `~/.claude/methodology/idd.md` Part I §§1–10, Part IV.
- `docs/ade-invariant-registry.toml` — `DC-VAL-01..06` (declared).
- `~/.claude/methodology/templates/cluster.md`.

## Cluster Index (Dependency Order)

1. **PHASE4-B1** — Full block validity agreement — primary invariant: a block
   is Valid iff consensus header authority ∧ ledger body authority both accept
   it, and that verdict equals the reference on valid **and** invalid blocks.
   **(detailed below)**
2. **PHASE4-B2** — Tx validity agreement / mempool — *(future; opens with its
   own `/invariants` sketch)* — depends on B1.
3. **PHASE4-B3** — Block production on preprod — *(future)* — depends on B1, B2.
4. **PHASE4-B4** — Sync from genesis/Mithril → tip — *(future)* — depends on
   B1 + N-D.

Only **PHASE4-B1** is sketched and planned here. B2–B4 are placeholders.

Workstream B is the bounty-winning path (validity + block production);
PHASE4-N-B shipped the consensus kernel, and B1 is the dependency root that
turns it into real block-validity agreement.

---

## Cluster PHASE4-B1 — Full block validity agreement

**Primary invariant:**
> `block_validity(LedgerState, PraosChainDepState, EraSchedule, LedgerView,
> block_cbor)` is a pure, total function whose verdict (Valid / Invalid+reason)
> equals the reference cardano-node verdict — proven on a positive corpus (real
> Conway-577 blocks) **and** a mandatory adversarial corpus (fail-closed
> rejection of malformed/malicious blocks).

**Anchors:** NEW `DC-VAL-01..06`; strengthens `DC-LEDGER-02` (no-false-accept /
CE-88 lineage), `CN-EPOCH-01`, `DC-CONSENSUS-02`, `CN-CONS-04`, `DC-CRYPTO-01`,
`T-ENC-01`, `T-DET-01`.

**TCB partition:**
- **BLUE:** `ade_ledger::consensus_view` (production `LedgerView` projection);
  `ade_ledger::block_validity` (composition transition + verdict/error
  taxonomies); header-completeness additions in `ade_core::consensus` +
  `ade_crypto::kes` wiring.
- **GREEN:** `ade_testkit::validity` (positive + adversarial corpus harness +
  mutators).
- **RED:** consensus-input extractor (snapshot `state` CBOR tail-scan, VRF-key
  un-skip, S3/disk loading) extending the snapshot loader.

**Entry conditions:**
- PHASE4-N-B closed: `validate_and_apply_header`, `PraosChainDepState`,
  `LedgerView` trait, `EraSchedule`, VRF/leader/nonce/op-cert kernel exist.
- Phases 1–3 closed: `apply_block_with_verdicts` (full body validation incl.
  fail-closed Ed25519 + Byron bootstrap witness verification, preserved-bytes
  hashing — confirmed in sketch §8).
- Consensus-input provenance resolved (sketch §8): eta0 extractable by a
  proven tail-scan of the snapshot `state` CBOR; eta0(576)=`d4d5f9dc…`,
  eta0(577)=`19674ecf…` already captured; ledger-state dumps carry VRF keys
  + stake; reference node binary present (pre-Conway, not required).
- `ade_crypto::kes` exists for KES signature verification (B1-S5).

**Cluster Exit Criteria (CI-verifiable):**

| CE | Check | Closed by |
|---|---|---|
| **CE-B1-1** | Production `LedgerView` is a pure projection of `LedgerState` (set-snapshot stake + pool VRF keys + asc) + epoch nonce; returns correct `(σ, total_active_stake, vrf_key, asc)` for Conway 577. Unit + corpus tests. | B1-S2 (inputs from B1-S1) |
| **CE-B1-2** | `block_validity` composes header ∧ body with header-before-body fail-fast ordering; body-hash binding is wired (proven by a negative test, not a defined-but-unwired validator). | B1-S4 (types from B1-S3) |
| **CE-B1-3** | Positive agreement — Ade marks every real Conway-577 block Valid (oracle = on-chain inclusion); the verdict stream replays byte-identically across two runs. | B1-S6 |
| **CE-B1-4** | Negative agreement — Ade rejects the adversarial corpus (bad witness/key/sig sizes, fabricated witness, bignum overflow, altered body-vs-header-hash, future slot, era-mismatched VRF, **bad KES sig**, missing required signer) with fail-closed structured reasons matching the spec-defined invalidity class. | B1-S5 (header completeness) + B1-S7 (corpus) |
| **CE-B1-5** | Total state evolution — Valid → evolved `(LedgerState', PraosChainDepState')`; Invalid → unchanged states + reason; no partial mutation. | B1-S4 + B1-S6 (replay) |

**Oracle policy:** the *positive* oracle is **on-chain inclusion** (every real
Conway-577 block is valid by definition — no node query needed). The *negative*
oracle is **spec-defined invalidity** (mutations that violate a specific
ledger/consensus rule); reference-node confirmation is best-effort, since the
bundled cardano-node 8.12.2 cannot run Conway.

**Slices:**

- **B1-S1** — Consensus-input extractor + canonical Conway-577 corpus —
  *invariant:* nonce / VRF-key / stake / asc are canonical inputs extracted
  deterministically (proven `82 01 5820` tail-scan + un-skip the VRF field in
  the loader), never guessed; absence is fail-fast — *addresses:* CE-B1-1
  (inputs), CE-B1-3 (corpus) — *TCB:* RED + GREEN. Includes the 5-nonce
  **field-order cross-validation** before trusting the `epochNonce` label.
- **B1-S2** — Production `LedgerView` projection — *invariant:* pure, total,
  `BTreeMap`-backed projection of `LedgerState` (set-snapshot E−2 stake, pool
  VRF keys, pparams asc) + epoch nonce; no rederivation, no `HashMap` —
  *addresses:* CE-B1-1 — *TCB:* BLUE. Entry obligation: confirm
  `ade_ledger → ade_core` dependency is acyclic.
- **B1-S3** — `BlockValidityVerdict` / `BlockValidityError` / `FieldError`
  closed taxonomies + canonical encoding — *invariant:* closed reason surface,
  no `String` / `#[non_exhaustive]` / `Box<dyn>` — *addresses:* substrate for
  CE-B1-2..5 — *TCB:* BLUE.
- **B1-S4** — `block_validity` composition transition — *invariant:* Valid iff
  header ∧ body accept; header-before-body fail-fast; body-hash binding wired;
  total state evolution (Invalid → unchanged states) — *addresses:* CE-B1-2,
  CE-B1-5 — *TCB:* BLUE.
- **B1-S5** — Header-validity completeness — *invariant:* KES signature
  verification (`ade_crypto::kes`) + era-correct VRF domain + fail-closed
  field/size checks (`expect_size` helper) so header forgeries are rejectable —
  *addresses:* CE-B1-4 (header half), `DC-VAL-06` — *TCB:* BLUE.
- **B1-S6** — Positive agreement corpus + replay — *invariant:* every real
  Conway-577 block → Valid; verdict stream byte-identical across two runs —
  *addresses:* CE-B1-3, CE-B1-5 — *TCB:* GREEN + RED.
- **B1-S7** — Adversarial / negative agreement corpus — *invariant:* every
  malformed/malicious block (body + header mutations) → Invalid with the
  correct fail-closed reason class — *addresses:* CE-B1-4 — *TCB:* GREEN
  (mutators) + RED.

**Forbidden during this cluster** (inherits cluster + global doctrine):
- Fail-open length/size guards (`if X.len()==K {check} else {skip}`) on any
  authority path — `DC-VAL-06`.
- Defined-but-unwired checks; tautological/no-op guards.
- Re-encoding for body/tx hashing instead of preserved wire bytes —
  `T-ENC-01`.
- `LedgerView` rederiving a stake snapshot; using mark/go instead of set —
  `DC-CONSENSUS-02`, `CN-EPOCH-01`.
- A "trust the body / skip header" path in the authoritative verdict (the
  follow-bridge's RED peer-trusted shortcut must not leak here) — `DC-VAL-02`.
- `HashMap`/`HashSet`, float, wall-clock in BLUE.
- Positive-only test coverage — the adversarial corpus is mandatory.

**Replay obligations:**
- **New canonical types:** `BlockValidityVerdict`, `BlockValidityError`,
  `FieldError`, production `LedgerView` impl, the B1 corpus schema.
- **New authoritative transition:** `block_validity` (composes existing
  authorities; introduces no new ambient state).
- **New replay corpus:**
  - `corpus/validity/conway_epoch577/` — positive: `LedgerState` +
    `PraosChainDepState{epoch_nonce=eta0(577)}` + `EraSchedule` + blocks +
    on-chain-inclusion verdicts.
  - `corpus/validity/adversarial/` — negative: mutated blocks +
    spec-invalidity reason classes.
  - Both replay byte-identically; anchored by `T-DET-01`, `DC-LEDGER-02`.

**Invariants strengthened by this cluster:**

| Slice | Strengthens / introduces |
|---|---|
| B1-S1 | `DC-VAL-01` (inputs are canonical, never guessed) |
| B1-S2 | `DC-CONSENSUS-02`, `CN-EPOCH-01`, `DC-VAL-01` |
| B1-S3 | `DC-VAL-02/04/05/06` (closed taxonomies) |
| B1-S4 | `DC-VAL-02`, `DC-VAL-03`, `DC-VAL-05`, `CN-CONS-04` |
| B1-S5 | `DC-VAL-06`, `DC-CRYPTO-01`, `T-ENC-01` |
| B1-S6 | `DC-VAL-04`, `T-DET-01`, `DC-LEDGER-02` |
| B1-S7 | `DC-VAL-04`, `DC-VAL-06`, `DC-LEDGER-02` (no-false-accept) |

**Open items to resolve at `/cluster-doc PHASE4-B1`:**
- Dependency direction for `block_validity` placement (`ade_ledger → ade_core`,
  expected acyclic — verify; alternative: a thin `ade_validity` composition
  crate).
- 5-nonce field-order cross-validation method (B1-S1).
- Confirm `ade_crypto::kes` exposes a usable verify for B1-S5; if it proves out
  of reach, it becomes an explicit documented CE-B1-4 gap (flagged, never
  silently dropped).
- Epoch alignment: Conway-577 blocks need eta0(577) (captured) + set-snapshot
  frozen at epoch 575 — confirm the ledger-state dump carries the right
  snapshot generation.

## Authority reminder

Planning aid only. Authority for rules belongs to
`docs/ade-invariant-registry.toml`; for mechanical acceptance, the named
tests/CI. If this plan conflicts with the registry or normative specs, those
win.
