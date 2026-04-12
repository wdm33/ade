# CE-79 Four-Tier Gate Statement

> **Exit criterion (verbatim from cluster plan):**
>
> Four-tier gate statement documented: for every correctness claim made
> about the Ade ledger, state which tier of the gate applies — `true`
> (mechanically enforced property), `derived` (established against the
> non-Plutus corpus), `release` (scope of the certification shipped),
> or `non-goal` (explicitly out of scope). The gate applies
> uniformly across all seven eras (Byron through Conway).

**Status: DOCUMENTED.** This document IS the gate statement.

**Version-scoped to:** cardano-node 10.6.2 (git rev `0d697f14`),
ouroboros-consensus-cardano 0.26.0.3, mainnet magic 764824073.

---

## 1. Purpose

Correctness claims about a ledger implementation vary in strength.
A claim that is mechanically enforced by the compiler is stronger
than a claim verified against a corpus, which in turn is stronger
than a claim about what is certified to be shippable, which is
stronger than a claim about what is not pursued at all. Without an
explicit gate, readers cannot tell which kind of claim they are
looking at.

The four-tier gate establishes the vocabulary. Each CE in the
constitution registry declares which tier(s) apply to its
closure statement.

---

## 2. The Four Tiers

### Tier 1 — `true` (Mechanically Enforced Property)

A property guaranteed by the code itself, independent of any test
or corpus. Established by the type system, by `#![deny(...)]`
attributes, by CI scripts that reject any violation at build time,
or by pure-function boundaries that make violations impossible
to express.

A `true` claim does not depend on any specific input. It holds
for every possible input, by construction.

**Examples in Ade:**

| Property | Mechanism | Enforcement surface |
|----------|-----------|---------------------|
| Determinism in BLUE crates | No `HashMap`/`HashSet`, no `SystemTime`, no `rand`, no floats, no `fs`, no `net`, no `async` | `ci_check_forbidden_patterns.sh` + `#![deny(clippy::float_arithmetic)]` |
| BLUE → RED boundary | No BLUE crate depends on any RED crate | `ci_check_dependency_boundary.sh` |
| `apply_block` purity | Pure function signature: `(&LedgerState, era, &[u8]) -> Result<LedgerState, _>` | Type system |
| Conservation via `Value` algebra | `Value::sub` returns `Result`; underflow surfaces as `LedgerError` | Type system + tests |
| Canonical serialization | All persisted/hashed data goes through `ade_codec::cbor` writers; no ad-hoc encoding | Module header contract + `ci_check_module_headers.sh` |
| No wire-byte forgery | Hash-critical paths consume `PreservedCbor<T>.wire_bytes()` only | `ci_check_hash_uses_wire_bytes.sh` |
| No credential leaks | No hostnames, IPs, keys in tracked files | `ci_check_no_secrets.sh` |

### Tier 2 — `derived` (Non-Plutus Corpus Equivalence)

A property empirically established against the archived oracle
corpus (cardano-node 10.6.2 snapshots and contiguous block streams).
Holds for every tested input; extrapolates to the population of
non-Plutus mainnet traffic by construction of the corpus but is
not a formal proof over all possible inputs.

A `derived` claim is stronger than anecdote (it covers a curated
population) but weaker than `true` (it does not rule out untested
inputs). If a new cardano-node release changes the corpus, the
`derived` claim must be re-verified against the new corpus.

**Examples in Ade:**

| Property | Corpus surface | Evidence |
|----------|----------------|----------|
| CBOR round-trip fidelity | All 10,500 contiguous blocks | `ci_check_cbor_round_trip.sh` |
| Verdict agreement on non-Plutus blocks | 10,500 blocks across 7 eras | `ci_check_differential_divergence.sh` (layer 1) |
| Reward formula exactness | Allegra 236→237 per-pool comparison | `epoch_oracle_comparison.rs::precise_boundary_comparison_eta_diagnosis` |
| Conway governance ratification/enactment | Conway 528/536/576 boundaries | `epoch_oracle_comparison.rs::conway_*` |
| HFC translation semantics | 22/22 encoding-independent fields at Shelley→Allegra | `translation_summary_proof.rs` |
| Boundary state fingerprint stability | 12 proof-grade snapshots | `ci_check_differential_divergence.sh` (layer 2) |
| Stateful determinism | All 7 eras, single + multi-block | `ci_check_ledger_determinism.sh` |
| Cryptographic primitives | Blake2b-256/224 / Ed25519 / VRF / KES golden vectors | `ci_check_crypto_vectors.sh` |

### Tier 3 — `release` (Scope of Certification)

The explicit description of what the current release is certified
to handle correctly. Narrower than `derived` — it names the exact
set of preconditions, version scopes, and residuals that remain
acceptable.

A `release` claim is what the project commits to externally. It
answers: if someone ships Ade at this commit, what have they
been promised?

**Current release certification:**

- **Non-Plutus block validation only.** Alonzo/Babbage/Conway blocks
  containing Plutus scripts reach `ScriptVerdict::NotYetEvaluated` and
  are flagged "script-execution-deferred" rather than accepted or
  rejected. This is deliberate (see Tier 4).
- **Version-scoped to cardano-node 10.6.2.** Any upstream release that
  changes on-disk encoding (e.g. `TablesCodecVersion1` at 10.7.0),
  protocol parameters, or semantic behavior invalidates the release
  claim until the corpus is re-extracted and Tier 2 evidence is re-gathered.
- **Epoch boundary correctness at snapshot precision.** CE-71 closes
  with 0-lovelace reward formula delta. CE-72 closes with documented
  irreducible residuals at snapshot comparison precision limits
  (Alonzo 164 ADA reserves, Alonzo 4,046 ADA treasury, Conway 1.3 ADA
  treasury) — all traceable to PV≤6 registration-set timing and
  per-member rounding, not to formula error.
- **HFC translation at semantic equivalence.** CE-73 certifies that
  all 6 HFC translations produce oracle-equivalent state at the
  22-field comparison surface. Byte-parity with the Haskell on-disk
  `ExtLedgerState` CBOR is not pursued (see Tier 4).
- **Boundary-level differential agreement.** CE-75 certifies
  fingerprint-identical state at the 12 proof-grade boundaries.
  Per-block state-hash agreement requires a live differential
  harness and is not part of the current release.

### Tier 4 — `non-goal` (Explicit Out of Scope)

Properties the project has explicitly decided not to pursue.
Includes the rationale so future contributors can reason about
whether the decision still holds.

A `non-goal` claim is as important as a `true` claim: it tells
readers what the project does not promise, which prevents false
inference from the release.

**Current non-goals:**

| Non-goal | Rationale |
|----------|-----------|
| Partial Plutus execution | Half-evaluated scripts introduce a failure mode (validation diverges from oracle on a subset of transactions) that is worse than no evaluation at all. Plutus lands as Phase 3 with a full UPLC evaluator, or not at all. |
| Byte-parity with Haskell `ExtLedgerState` on-disk CBOR | The Haskell on-disk format is an implementation artifact of `cardano-binary`, not a spec. It changes across releases (e.g. `TablesCodecVersion1` at 10.7.0). Ade uses its own canonical fingerprint format (`ade_ledger::fingerprint`) which achieves the same divergence-detection goal without version coupling. |
| Per-block live differential harness (in-repo) | Live differential against a running Haskell node requires integration with an external runtime (ShadowBox or equivalent). In-repo, Ade closes differential agreement at the boundary-snapshot level; per-block live differential is addressed by an external harness when downstream need arises. |
| Full mainnet sync | Phase 2 certifies ledger rules on a curated corpus. Continuous mainnet sync (Phase 6) requires networking, chain selection, block production, and consensus code not yet in Ade. |
| `ade_core` as a non-empty crate | `ade_core` was reserved for shared abstractions that did not materialize. It remains a placeholder. The actual functional core lives in `ade_ledger`. |

---

## 3. How Claims Reference the Gate

Every closure statement in `constitution_registry.toml` cites the
applicable tier(s). Example:

```toml
[exit_criteria.CE-71]
status = "closed"
tier = "derived"
evidence = "epoch_oracle_comparison.rs::precise_boundary_comparison_eta_diagnosis"
residuals = "Alonzo 164 ADA reserves (PV<=6 registration-set timing, snapshot-comparison precision)"
version_scope = "cardano-node 10.6.2"
```

A reader of the registry can tell immediately whether a claim is
a mechanical invariant (`true`), a corpus-verified property
(`derived`), a shipped promise (`release`), or an exclusion
(`non-goal`).

---

## 4. What This Gate Is Not

- **It is not a formal verification claim.** `true` means
  mechanically enforced by code and CI, not proved in a theorem
  prover. A bug in the enforcement surface (CI script, type
  signature) breaks the `true` claim.
- **It is not a consensus-safety claim.** Correctness against a
  curated corpus does not imply safety in a live adversarial
  network. Consensus is Phase 4.
- **It is not a certification for block producers.** A block
  producer running Ade would need the full shell (networking,
  chain selection, signing), which is not in the current release.
- **It is not a static gate.** Every upstream Haskell release can
  move `release` boundaries. `derived` evidence must be re-gathered
  against the new corpus. `non-goal` decisions can be revisited if
  downstream need emerges (e.g., live differential when a block
  producer use case appears).

---

## 5. Authority Reminder

This document is a planning and communication aid.

Correctness requirements remain defined by:

- the project constitution
  (`ade_replay_first_constitutional_node_plan_v1.md` §2–§4b),
- `01_core_determinism_and_contract.md`,
- `classification_table.md`,
- and the CI enforcement surface.

If there is ever a conflict between a tier classification in this
document and the normative specifications, the normative
specifications and CI enforcement are authoritative.
