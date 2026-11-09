# PHASE4-B4 — Conway cert-state accumulation fail-closed path

> Invariant sketch (IDD Part I). Planning artifact — no clusters, slices, or
> code yet. Produced 2026-05-21. Anchor rule: **DC-LEDGER-08** (proposed,
> appended to `docs/ade-invariant-registry.toml`). Strengthens **DC-VAL-06**.

## Grounding (what the code does today)

`process_block_certificates` (`crates/ade_ledger/src/rules.rs:1383-1466`)
accumulates delegation/pool `CertState` from each tx body's key-4 cert array.
It is reached **only** at the two `track_utxo == true` call sites
(`rules.rs:286`, `rules.rs:369`). It contains three fail-open swallows:

1. **`rules.rs:1422`** — Conway cert bytes are decoded with the **Shelley
   6-variant decoder** `ade_codec::shelley::cert::decode_certificates`.
   Conway-only tags (7..18: `reg`/`unreg`, DRep, committee, vote-deleg, gov)
   have no Shelley arm. The `_era` parameter is **ignored** — the decoder is
   hard-wired to Shelley regardless of era.
2. **`rules.rs:1433-1436`** — `apply_cert` errors swallowed, justified as
   "non-fatal during replay *without full UTxO state*."
3. **`rules.rs:1440-1442`** — decode errors swallowed entirely.

**Sharp finding — the swallow rationale is self-contradictory.** Justification
(2) claims "without full UTxO state," but this code only executes when
`track_utxo == true` (the `else` branch at `rules.rs:288`/`rules.rs:371` clones
state and never calls this function). The swallow runs precisely when full
state *is* present. The original excuse no longer holds.

**Second finding — type gap.** `apply_cert` (`delegation.rs:114`) accepts only
the 7-variant Shelley `Certificate`. The shared decoder yields `ConwayCert`
(tags 0..18). Consuming `decode_conway_certs` here requires a Conway→cert-state
apply path or a total `ConwayCert → {Shelley apply | neutral | reject}`
mapping. **This is the central planning question, not an implementation
detail.**

This is a distinct authority surface from B3: DC-TXV-06 closed the cert
*classification* path (`cert_classify.rs`) feeding **value conservation**. B4
closes the cert *state accumulation* path feeding **delegation/pool/reward
state**. Same decoder, different consumer, different invariant.

## Decisions taken at sketch time

- **Era scope: all eras unified.** B4 routes every era through explicit
  era→decoder dispatch, removing the `_era` discard universally rather than
  patching only the Conway arm. This eliminates the era-blind decoder-selection
  pattern entirely (see INV-2, OQ-2).
- **OQ-1 — native Conway apply, no lossy adapter (RESOLVED).** The authoritative
  apply function consumes Conway-native meaning:

  ```
  apply_conway_cert(state: CertState, cert: ConwayCert, env: ConwayCertEnv)
      -> Result<CertState, CertApplyError>
  ```

  Reducing `ConwayCert` (tags 0..18) back into the 7-variant Shelley
  `Certificate` recreates the exact semantic compression that produced this bug
  class — an incompatibility risk requiring proof, with no upside. A mapping
  layer is permitted **only** as an internal, total, explicit helper:

  ```
  ConwayCert -> ConwayCertAction
    ApplyConway(...)
    Neutral(...)
    Reject(NotValidInEra | Malformed | Unsupported)
  ```

  B4 is explicitly **not** "decode Conway then squeeze into Shelley."
- **OQ-6 — no `.idd-config.json` entry-count touch (RESOLVED).** Registry
  authority is `docs/ade-invariant-registry.toml`, not a human-readable count
  string. No `.idd-config.json` update in B4 unless a mechanical registry-count
  check actually fails. There is no `registry_count` field in the TOML and no
  check reads the doc string, so updating it would be cosmetic churn.
- **Cluster posture: B4 is a separate ledger-cert-state fail-closed cluster**,
  not absorbed into B3. Core invariant restated for the cluster: *for every era,
  certificate-state accumulation is era-dispatched, total over that era's
  certificate grammar, and fail-closed — no cert decode/apply error may be
  swallowed as "non-fatal" when full state is present.*

## 1. What must always be true

- **B4-INV-1 (closure).** Cert-state accumulation over a block's certs is a
  closed, total transition: it yields either complete canonical `CertState`
  *or* a deterministic structured reject. No third "partial / best-effort"
  outcome.
- **B4-INV-2 (decoder unification, all eras).** The cert-state path decodes
  certs through the era-correct closed decoder selected by explicit
  era→decoder dispatch — Conway certs through
  `ade_codec::conway::cert::decode_conway_certs` (tags 0..18), never the
  Shelley 6-variant decoder. `era` ceases to be `_era` for every era.
- **B4-INV-3 (error propagation).** A decode error or an `apply` error during
  accumulation propagates as a structured `LedgerError` (the existing
  `From<CodecError>`, `EraInvalidCertificate`, `UnsupportedStateDependentDeposit`
  surface), never `Ok(...)`.
- **Strengthens DC-VAL-06** — removes a live `Err(_) => { /* non-fatal */ }`
  silent-skip in BLUE.
- **Total mapping** — every `ConwayCert` variant resolves to exactly one
  cert-state effect: a real `CertState` mutation, an explicit neutral
  (governance/DRep certs that don't touch delegation/pool state), or a
  structured reject. No dropped variant.

## 2. What must never be possible

- A malformed cert array silently leaving `cert_state` unchanged (swallow #3).
- An `apply_cert`/apply error silently dropped, leaving `cert_state`
  mid-accumulation but reported as success (swallow #2).
- A Conway-only tag (7..18) decoded by the Shelley decoder and dropped as
  unmatched (bug #1).
- An unknown tag (≥19) or removed tag (5/6 in Conway) producing `Ok`
  accumulation rather than reject.
- Era-blind decoder selection — using one decoder for all eras.
- Non-determinism: `HashMap` iteration, wall-clock, or RNG influencing
  accumulated `CertState`.

## 3. What must remain identical across executions

- The accumulated `CertState` (delegation map, pool params, deposits,
  retirements) for a given block + prior state — bit-for-bit.
- The reject decision and its structured error class for any
  malformed/unknown/era-invalid cert — identical class, identical position.
- Decoder dispatch: same era ⇒ same decoder ⇒ same parse.

## 4. What must be replay-equivalent

- Replaying the same ordered blocks at `track_utxo == true` must produce a
  byte-identical `CertState` sequence (and identical `LedgerState` fingerprint
  downstream).
- A new adversarial corpus of malformed / unknown-tag / removed-tag /
  truncated Conway cert arrays must replay to byte-identical structured
  rejects.
- A positive corpus of real Conway cert-bearing blocks (epoch-576 window) must
  replay to byte-identical accepted `CertState` — the no-regression guard,
  since today's swallow may be masking real accumulation that B4 starts
  enforcing.

## 5. State transitions in scope

```
accumulate_block_certs :
  (prior: CertState, input: ShelleyBlock @ era, params)
    → Result<(CertState, ∅effects), LedgerError>

decode_certs_for_era :
  (era, cert_bytes)
    → Result<Vec<ConwayCert | Certificate>, CodecError>     // Conway ⇒ decode_conway_certs

apply_conway_cert :
  (prior: CertState, cert: ConwayCert, params, idx)
    → Result<CertState, LedgerError>                        // total over tags 0..18:
                                                            //   delegation/pool ⇒ mutate
                                                            //   DRep/committee/gov/vote ⇒ Neutral (explicit no-op)
                                                            //   removed 5/6 ⇒ EraInvalidCertificate
                                                            //   unaccountable state-dependent ⇒ UnsupportedStateDependentDeposit
```

`process_block_certificates` becomes total: any internal `Err` short-circuits
the whole block transition (fail-fast, IDD §8).

## 6. TCB color hypothesis

| Element | Color | Note |
|---|---|---|
| `decode_conway_certs` (reused, B3-S2) | **BLUE** | already BLUE |
| era→decoder dispatch in `process_block_certificates` | **BLUE** | authoritative; removes the `_era` discard |
| `apply_conway_cert` / ConwayCert→CertState mapping | **BLUE** | authoritative state transition |
| structured `LedgerError` propagation | **BLUE** | reuses existing taxonomy |
| adversarial + positive corpora | **RED** (fixtures) → **GREEN** (drivers) | parity with B3-S5/S6 |
| CI closure gate (`ci/ci_check_*`) | **GREEN/CI** | parallels `ci_check_conway_cert_classification_closed.sh` |

No new RED in the authoritative path. **Open color question:** whether
`apply_conway_cert` lives in `delegation.rs` (BLUE) or whether a
ConwayCert→Shelley-Certificate bridge belongs in a thin GREEN adapter —
resolve at cluster-plan.

## 7. Open questions (resolve before / during cluster-plan)

1. **Apply path (load-bearing) — RESOLVED.** Native `apply_conway_cert`
   consuming Conway-native meaning; no lossy `ConwayCert → Shelley Certificate`
   adapter. Internal `ConwayCert → ConwayCertAction` mapping permitted only if
   total + explicit. See "Decisions taken" above.
2. **Pre-Conway eras (decided: unified).** All eras route through explicit
   era→decoder dispatch. Remaining sub-question: confirm the Shelley–Babbage
   decoder (`decode_certificates`) is already closed for its tag set so the
   unification adds dispatch without changing those eras' verdicts.
3. **Neutral cert semantics.** Which Conway variants legitimately leave
   `CertState` unchanged (DRep registration, committee auth/resign, votes, gov
   actions) vs. which must mutate cert-state. Needs a per-variant
   cert-state-effect table (parallel to B3's classification table).
4. **No-regression risk.** Does enabling real Conway cert accumulation
   (currently partly swallowed) change any existing replay fingerprint at
   epoch-576? If so, that is an intentional `T-DET-01` fingerprint migration to
   call out — same concern B3-S1 hit.
5. **Anchor rule.** New rule **DC-LEDGER-08** (cert-state accumulation closure)
   vs. folding into DC-TXV-06. Recommendation taken: new rule — different
   state, different consumer, different code locus.
6. **Registry mechanics — RESOLVED.** No `.idd-config.json` entry-count touch
   in B4 unless a mechanical registry-count check fails. Registry authority is
   the TOML, not the human-readable count string. See "Decisions taken" above.

## Post-cluster-doc resolutions (2026-05-21)

Resolved while writing `docs/clusters/PHASE4-B4/cluster.md`:

- **Finding A — shared decoder is lossy.** `decode_conway_certs` is a B3
  deposit-purpose projection that drops the delegation/pool/DRep payloads
  `apply_conway_cert` needs. B4 **completes** the single shared decoder additively
  (B4-S1); no second decoder. B3 deposit reads remain a valid projection.
- **Finding B — owner-tagged, outside B4 mutation scope.** Governance-affecting
  certs (vote-deleg 9/10, committee 14/15, DRep 16/17/18) have owners in
  `ConwayGovState`, but it is snapshot-loaded, not cert-accumulated. B4 owns only
  delegation/pool `CertState`; gov certs are decoded fully and
  **owner-tagged to `ConwayGovState`**, routed out-of-mutation-scope — not
  `Neutral` (flattening forbidden; owner exists), not `Unsupported`, not applied.
- **Composite certs (10/12/13)** split across owners — apply outcome carries both a
  `CertState` mutation and an owner-tagged effect.
- **No Conway cert is `Neutral`** — every defined tag has an owner; `Neutral` maps
  to ∅ in Conway.
- **PHASE4-B5 (declared)** owns wiring the owner-tagged effects into applied
  `ConwayGovState` — not B4.

## Next step

Slices: `/slice-doc B4-S1` (complete the shared decoder) → S2..S5. Detailed
authority is `docs/clusters/PHASE4-B4/cluster.md`. The B4 stub in
`docs/planning/phase4-b3-cluster-slice-plan.md` is cross-linked as superseded.
