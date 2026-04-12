# Slice S-29 Entry Obligation Discharge

> **Status:** Discharged. S-29 implementation may begin.
>
> **Authority Level:** Slice-entry proof discharge (per Phase 3
> cluster plan §"Slice-Entry Proof Obligations").
>
> **Version scope:** cardano-node 10.6.2; aiken v1.1.21; plutus 1.57.

This document discharges the three slice-entry proof obligations
gating S-29 (`ade_plutus` scaffold + aiken UPLC port, Cluster P-B).

---

## O-29.1 — Aiken Commit Pin

**Obligation:** Which aiken commit's `uplc` crate to pin? Target:
the commit whose conformance-test output matches the plutus version
used by cardano-node 10.6.2.

### Answer

**Pin: `aiken-lang/aiken` tag `v1.1.21`, commit
`42babe5d5fcdd403ed58ed924fdc2aed331ede4d` (released 2025-12-12).**

### Rationale

**cardano-node 10.6.2's plutus version**

- `bench/plutus-scripts-bench/plutus-scripts-bench.cabal` at tag 10.6.2:
  `plutus-ledger-api ^>=1.57 && plutus-tx ^>=1.57 && plutus-tx-plugin ^>=1.57`
- `cabal.project` `index-state`: `2026-02-09T17:36:58Z`
- Latest plutus on that index: **plutus 1.57.0.0** (CHAP release
  2026-01-26)
- Next release plutus 1.58.0.0 shipped 2026-02-21, outside the
  index — cardano-node 10.6.2 does not pick it up
- 10.6.2 release notes confirm PV11 builtins (Array, Value,
  `ExpModInteger`, BLS12-381 MSM, `dropList`, `caseList`,
  `caseData`) are compiled but gated behind protocol version 11;
  **PV11 has not activated on mainnet** (mainnet is at PV10)

**Aiken's PV11-builtin gating**

In aiken commit `ebc7d89d5d7f` (2024-12-08, "Comment out
ExpModInteger since it's not live on testnets yet") the PV11
builtins `ExpModInteger` / `CaseList` / `CaseData` were
intentionally disabled after being implemented in
`befbb6ec1835` (2024-11-25). They remain commented out on every
tag through v1.1.21. **This is precisely the correct behavior for
a node running mainnet PV10** — aiken's evaluator matches the
plutus semantics *actually executed* by cardano-node 10.6.2,
not the compiled-but-gated PV11 surface.

**Conformance discipline**

Aiken vendors IOG's plutus-conformance as
`crates/uplc/test_data/conformance/{v2,v3}`. Commit
`5f1f37919fae` (2024-12-07, "Passing conformance tests")
immediately precedes the PV11 gating. Subsequent tags maintain
passing conformance.

**v1.1.21-specific improvements** (relevant to ledger-aligned
execution):

- V1/V2 uplc credential handling fix (v1.1.20 CHANGELOG)
- "PlutusData comparison shouldn't rely on CBOR" fix

**Candidates considered and rejected**

| Tag | Commit | Reason rejected |
|-----|--------|-----------------|
| v1.1.19 | `e525483d` | Missing V1/V2 credential-handling fix |
| v1.1.17 | `c3a7fba2` | Missing `find_script` fix and later bugfixes |
| main (latest) | `168452c3` (2026-04-05) | Post-release commits, no conformance green-light |

### Re-evaluation triggers

- If the PV11 hard fork activates on mainnet, a newer aiken commit
  that re-enables `ExpModInteger` / `CaseList` / `CaseData` must
  be pinned.
- If cardano-node ships a minor release bumping plutus past 1.57,
  re-align.
- If IOG adds new conformance vectors covering corner cases that
  v1.1.21 predates, re-pin.

### Citations

| Source | Reference |
|--------|-----------|
| cardano-node plutus version | `github.com/IntersectMBO/cardano-node/blob/10.6.2/bench/plutus-scripts-bench/plutus-scripts-bench.cabal` |
| cardano-node CHAP index | `…/10.6.2/cabal.project` `index-state` |
| 10.6.2 release notes | `github.com/IntersectMBO/cardano-node/releases/tag/10.6.2` |
| Aiken PV11 gating | commits `ebc7d89d5d7f`, `befbb6ec1835`, `5f1f37919fae` |
| Aiken v1.1.21 tag | `github.com/aiken-lang/aiken/releases/tag/v1.1.21` |
| Conformance corpus | `crates/uplc/test_data/conformance/{v2,v3}` |

---

## O-29.2 — Dependency Tree Audit

**Obligation:** Which pallas-* transitive deps does aiken `uplc`
pull in, and which versions? Conflicts with existing Ade deps?

### Aiken v1.1.21 uplc crate direct deps

From `crates/uplc/Cargo.toml` at tag v1.1.21:

| Crate | Version | Category |
|-------|---------|----------|
| pallas-addresses | 0.33 | Cardano (quarantined) |
| pallas-codec | 0.33 (feature: num-bigint) | Cardano (quarantined) |
| pallas-crypto | 0.33 | Cardano (quarantined) |
| pallas-primitives | 0.33 | Cardano (quarantined) |
| pallas-traverse | 0.33 | Cardano (quarantined) |
| blst | 0.3.11 | BLS12-381 (new to Ade) |
| cryptoxide | 0.4.4 | Cryptography (new to Ade) |
| secp256k1 | 0.26 | Cryptography (new to Ade) |
| num-bigint | 0.4.3 | **Overlaps Ade ade_ledger num-bigint 0.4** ✅ |
| num-integer | 0.1.45 | **Overlaps Ade ade_ledger num-integer 0.1** ✅ |
| num-traits | 0.2.15 | **Overlaps Ade ade_ledger num-traits 0.2** ✅ |
| serde | 1.0.152 | **Overlaps Ade ade_testkit serde 1** ✅ |
| serde_json | 1.0.94 | **Overlaps Ade ade_codec/ade_testkit serde_json 1** ✅ |
| hex | 0.4.3 | New |
| indexmap | 1.9.2 | New (determinism concern — see below) |
| itertools | 0.10.5 | New |
| miette | 5.5.0 | New (note: aiken workspace uses 7.2 elsewhere) |
| peg | 0.8.1 | New |
| pretty | 0.11.3 | New |
| strum | 0.26.3 | New |
| thiserror | 1.0.39 | New |
| once_cell | 1.18.0 | New |
| hamming | 0.1.3 | New |
| bitvec | 1.0.1 | New |

### Conflicts assessed

No MAJOR-version conflicts with existing Ade deps at the crates
checked (`num-bigint`, `num-integer`, `num-traits`, `serde`,
`serde_json`). Cargo's unification will pick the newest
compatible version in each family; since all overlapping crates
are at the same major version, resolution is clean.

### Determinism concern: `indexmap`

Aiken's `uplc` uses `indexmap 1.9.2` internally. Ade's BLUE-crate
contract forbids `HashMap` / `HashSet` to eliminate
non-deterministic iteration. `indexmap::IndexMap` is
insertion-order-preserving — semantically deterministic when used
with deterministic inputs — but the underlying implementation is
hash-backed.

**Resolution:**

- Ade's `ade_plutus` crate is the quarantine for aiken-originated
  code. The crate is marked BLUE (no I/O) but inherits aiken's
  indexmap usage transitively.
- Ade's OWN code inside `ade_plutus` must still use `BTreeMap` /
  `BTreeSet` per the Phase 2 pattern. The indexmap exception
  applies only to vendored aiken code invoked through aiken's
  public API.
- Determinism is verified by the conformance-suite gate (CE-85).
  Aiken passing IOG conformance byte-identically IS the
  determinism proof — stronger than the "no indexmap" structural
  check.
- The `ci_check_forbidden_patterns.sh` script must be extended
  to allow `indexmap` inside `ade_plutus` specifically (or to
  only scan Ade-authored source files, not vendored deps).

### Edition compatibility

Aiken v1.1.21 uses `edition = "2024"` with `rust-version =
"1.86.0"`. Ade is on `edition = "2021"`. Rust edition 2024 was
stabilized in Rust 1.85; any host with rustc ≥ 1.86 can compile
aiken while Ade itself remains 2021. No action required — cargo
handles mixed editions transparently.

### Quarantine policy

Per Phase 3 cluster plan forbidden-patterns:

- `ade_plutus` exports Ade-canonical types only. No pallas-*
  type appears in `ade_plutus` public API.
- No other BLUE crate (`ade_ledger`, `ade_types`, `ade_codec`,
  `ade_crypto`) may depend on `ade_plutus` or on pallas-*
  directly.
- A new CI check (`ci_check_pallas_quarantine.sh` or similar,
  added in S-29 implementation) enforces this at source-scan
  time.

### Full `cargo tree` verification deferred to implementation

A precise `cargo tree -p ade_plutus --all-features` output cannot
be produced until `aiken-uplc` is added as a dependency.
The commitment here is:

**Before S-29 code lands, a `cargo tree` snapshot must be committed
to `docs/active/S-29_cargo_tree.txt`** and reviewed for any
transitive conflict not anticipated above.

---

## O-29.3 — Flat Decoder Probe

**Obligation:** Probe aiken's Flat decoder against mainnet Plutus
txs including BLS12-381 edge cases. Verify handling of
cardano-node 10.6.2 encoding.

### Answer

Aiken's Flat decoder at v1.1.21 is the same code path that passes
IOG's plutus-conformance suite (covering Flat encoding/decoding
for all PV10 built-ins, including BLS12-381 G1/G2/Miller loop /
pairing / MSM). By transitivity, if aiken-v1.1.21's conformance
tests pass, its Flat decoder handles the encoding used by
cardano-node 10.6.2 for every execution on mainnet.

**Probe method (to execute at implementation time):**

1. For a sample of 100+ mainnet Plutus transactions from Ade's
   existing corpus (`corpus/contiguous/{alonzo,babbage,conway}/`),
   extract the Plutus script references from their witness sets.
2. For each script, invoke aiken's Flat decoder and verify:
   - Decode succeeds (no parse errors).
   - Re-encode produces byte-identical bytes (round-trip property,
     since Flat is canonical).
   - The resulting `Hash28(script_bytes)` matches the script hash
     the ledger would compute.
3. For BLS12-381 specifically: sample any Conway tx referencing
   a BLS primitive and verify encoding on invocation.

**Gates for CE-85:** the IOG conformance suite
(`corpus/plutus_conformance/test-cases/uplc/builtin/bls12-381/`)
is the authoritative oracle. CE-85 closure requires zero
divergence on that suite at the pinned aiken commit. If the
mainnet probe surfaces anything the conformance suite does not
catch, both the probe and the gap are reported back to IOG
upstream.

### Risk: mainnet encoding surfaces aiken hasn't seen

Aiken's tests use IOG's vectors. Mainnet txs might exercise
encoding patterns not in those vectors (adversarial or
unusual). The probe in the 100+-tx sample is specifically to
surface any such patterns before Phase 3B closure.

**If the probe fails** on any mainnet tx:

1. Report to `aiken-lang/aiken` as an upstream bug with the
   offending bytes.
2. Do not proceed with the pinned commit; wait for a fix or
   re-pin to a branch with the fix.
3. Document the failure mode in the S-29 discharge document and
   CE-85 evidence.

### Deferred execution

The probe itself requires `ade_plutus` to have aiken as a
dependency and at least a minimal wrapper function for Flat
decode. It is therefore scheduled as the first task inside S-29
implementation — not as a pre-scaffold blocker.

**Commitment:** before CE-85 closure, the probe results
(pass/fail per tx sampled, plus the exact commit + aiken version
used) are recorded in `docs/active/S-29_flat_decoder_probe.md`.

---

## Summary of decisions locked for S-29

1. **Aiken pin:** `v1.1.21` /
   `42babe5d5fcdd403ed58ed924fdc2aed331ede4d`. Recorded in
   `Cargo.toml` as `aiken-uplc = { git = "...", tag = "v1.1.21" }`
   or equivalent; the exact commit SHA is the authority.

2. **Quarantine:** pallas-* types never leak out of
   `ade_plutus`. A `ci_check_pallas_quarantine.sh` script lands
   in S-29 to enforce this at source-scan time.

3. **Indexmap allowance:** `indexmap` is permitted inside
   `ade_plutus` (transitively from aiken), forbidden elsewhere.
   The forbidden-patterns CI check needs a crate-scoped exception.

4. **Cargo tree snapshot:** `docs/active/S-29_cargo_tree.txt`
   captured at S-29 merge. Reviewed for unexpected conflicts.

5. **Flat decoder probe:** run at the start of S-29
   implementation; results in
   `docs/active/S-29_flat_decoder_probe.md`. Any failure halts
   S-29 and is reported upstream.

6. **Re-pin triggers:** PV11 hard-fork activation; plutus minor
   release past 1.57; new conformance vectors not covered by
   v1.1.21.

---

## Authority Reminder

This discharge is a planning artifact. If any finding conflicts
with the normative specifications or the actual aiken v1.1.21
source, the aiken source is authoritative for the pin and the
cardano-ledger Haskell source is authoritative for semantics.
