# SLICE P6 (P4-S2) — store-semantics version gate

> **SCOPED, NOT IMPLEMENTED.** P4 follow-up #1. Turns *"mysterious hash mismatch after upgrade"* into
> *"this store was produced under incompatible semantics; re-bootstrap or run a proven migration."*
>
> P5 (`45c2b942`) prevents the bug class from recurring in LIVE computation. This slice addresses the
> orthogonal failure mode: a DURABLE store produced under an older semantic contract, recovered by a
> newer binary.

## Intent

Make semantic store compatibility **mechanical instead of remembered**. A binary must not silently
recover from a durable store whose authoritative epoch / ledger / frozen-leadership semantics were
produced under an older semantic contract.

```
today:    old store semantics + new binary semantics -> opaque hash mismatch
required: old store semantics + new binary semantics -> typed version rejection / re-bootstrap
```

## This is a GENERALIZATION of a proven mechanism, not an invention

Reconnaissance changed the shape of this slice. The exact mechanism already exists in
`crates/ade_runtime/src/chaindb/persistent.rs`, bound to one hardcoded semantic axis:

```rust
// MEM-OPT-UTXO-DISK S1.5b: fingerprint-version gate. This store embeds v2 fingerprints in its
// WAL/anchor; a v1 (or unversioned) store CANNOT be replayed by a v2 node -- fail CLOSED
// (no silent mixed-version replay, no upgrade). `found` 0 = the marker is absent.
if fp_version != FINGERPRINT_VERSION {
    return Err(ChainDbError::FingerprintVersionMismatch { expected: FINGERPRINT_VERSION, found: fp_version });
}
```

It already has every property this slice requires: a store-level `META` table, fail-closed on
mismatch, **absent marker reads as 0 and is rejected**, no silent upgrade, typed error. It even
encodes the correct asymmetry — in the same function, the *encoding* version migrates forward
(`version < SCHEMA_VERSION` ⇒ upgrade on next write) while the *semantics* version does not:

| axis | on older marker |
|---|---|
| `SCHEMA_VERSION` (byte layout) | upgrade on next write — forward-compatible |
| `FINGERPRINT_VERSION` (meaning) | **hard fail, no upgrade** |

**Encoding may migrate. Semantics may not.** That is exactly the required behaviour, already proven
in production code. This slice widens the semantic axis from "fingerprint computation" to the
authoritative-semantics surface as a whole.

## The gap the existing markers leave

Four per-object schema versions exist — `FROZEN_LEADERSHIP_SCHEMA_VERSION=6`,
`SEED_CINPUT_SCHEMA_VERSION=6`, `BOOTSTRAP_RUPD_SCHEMA_VERSION=3`,
`RECOVERED_ANCHOR_POINT_SCHEMA_VERSION=1` — plus the store-level `SCHEMA_VERSION` and
`FINGERPRINT_VERSION`.

**Every one of them versions the ENCODING of bytes, not the SEMANTICS that produced the values inside
them.** This is precisely why P4 was invisible:

> **P3 changed no byte layout at all.** Every object still decoded cleanly at v6. The store was
> structurally valid and semantically stale, and nothing in the system could express that state.

The existing versions answer *"can I parse these bytes?"*. The missing one answers *"were these bytes
produced by rules I still implement?"*

## Tier classification

| tier | statement |
|---|---|
| **true** | Replay equivalence requires the persisted authority store and the binary's semantics to agree on the meaning of the bytes. |
| **derived** | Cardano epoch/era and reward semantics must not be recovered from a store produced under incompatible venue/epoch rules. |
| **release** | Version-gate tests + CI must reject stale stores before recovery proceeds. |
| **operational** | The operator-facing instruction may be "re-bootstrap this store" — but that is remediation, not the semantic invariant. |

## Required behaviour

```
store_semantics_version == binary_required_semantics_version   -> recovery may proceed
store_semantics_version <  binary_required_semantics_version   -> typed terminal (RebootstrapRequired)
missing version marker                                          -> typed terminal (explicit legacy rejection)
future / unknown version                                        -> typed terminal
```

No silent migration unless there is a **sealed migration proof**.

## Hard prohibitions

- No best-effort recovery from stale authority bytes.
- No fallback to old `slot_to_epoch` behaviour (it is deleted; DC-LEDGER-13 keeps it deleted).
- No implicit migration.
- No "warn and continue".
- No CLI flag that weakens the semantic gate.
- No feature-flagged authoritative semantics.

## Design content this slice must resolve

### 1. The marker must be PER-ARTIFACT, not one marker for the store

Measured, not assumed: the three authority artifacts are opened from **`snapshot_dir`**, not the data
directory (`node_lifecycle.rs:571` reduced checkpoint, `:599` accumulator), while `chain.db` and the
WAL come from the data directory. They can therefore arrive from **different provenance** — and
routinely do: the P4 investigation runs paired `--snapshot-dir preview-snapshot-1376` with
`--data-dir ade-r2-live`.

So a single marker in `chain.db`'s `META` is insufficient; a stale accumulator or reduced checkpoint
could be paired with a current ChainDb and pass. Both siblings already have their own `META_TABLE`
(`epoch_acc_meta`, `reduced_meta`), so each can carry the marker.

**Requirement:** every authority artifact carries the marker, and they must agree with the binary
**and with each other**. Cross-artifact agreement is the part a single marker cannot express.

### 2. What the version covers — and what it deliberately does not

**Semantics-bearing** (marker required): the WAL (its `post_fp` chain is a function of ledger
semantics), ledger snapshots, the epoch accumulator, the reduced UTxO checkpoint, and the derived
sidecars.

**Semantics-free**: the raw block bytes in `chain.db`. These are canonical wire input — the same bytes
under any semantics.

This split is operationally load-bearing. "Re-bootstrap" sounds catastrophic (12.9 GB, hours), but the
*expensive* part — network sync — is semantics-free and retained. Only derived state must be rebuilt.
A re-derive-from-retained-blocks path is therefore possible later (**out of scope here**, but the
split should be preserved so it stays possible).

The new version must be **orthogonal to `FINGERPRINT_VERSION`**, not folded into it: the two change
independently, and conflating them forces spurious re-bootstraps.

### 3. Making the bump mechanical — the hard part

A version constant that must be *remembered* gets forgotten exactly once, which is all it takes. Three
candidate triggers were considered:

| trigger | verdict |
|---|---|
| **(a)** CI fails if any file in a declared semantics-bearing set changes without a version bump | Catches P3. Noisy — a comment edit trips it. |
| **(b)** Lockfile of content hashes over a declared semantics-bearing surface; drift requires EITHER a version bump OR an explicit lockfile update with justification | Catches P3. Converts "remember to think about it" into "explicitly declare this change semantics-neutral". **Recommended.** |
| **(c)** Behavioural: pin a golden replay-corpus fingerprint; changed output ⇒ semantics changed | **Would have MISSED P3.** |

**(c) deserves emphasis because it is the intuitive choice and it fails.** P3's bug did not alter the
mainnet corpus at all — preview was affected "by numeric accident, not by correctness", and preprod
was not in the corpus. A behavioural trigger measured against a mainnet-shaped corpus is structurally
blind to venue-geometry defects, which is the entire P3/P4 family. If a behavioural trigger is used it
must be **multi-venue**, which is the separate venue-parity work.

Recommendation: **(b)**, optionally strengthened later by a multi-venue (c).

### 4. Where the check runs

`init_or_check_schema` runs on **every** `PersistentChainDb::open`, which is strictly stronger than
checking inside `warm_start_recovery`: a new reader that opens the store gets the gate for free. That
largely satisfies acceptance item 7 structurally rather than by convention. The sibling artifacts need
the equivalent check in their own `open`.

## Mechanical acceptance criteria (draft)

| CE | Criterion |
|---|---|
| **CE-P6-1** | An authority store stamps the current semantics version at creation |
| **CE-P6-2** | The marker is verified before the accumulator / eview / frozen-leadership authority is used |
| **CE-P6-3** | A **missing** marker fails closed with a typed terminal (legacy rejection) |
| **CE-P6-4** | An **older** marker fails closed with a typed re-bootstrap-required error |
| **CE-P6-5** | A **future/unknown** marker fails closed |
| **CE-P6-6** | A fresh current store recovers normally (the gate is not vacuous) |
| **CE-P6-7** | CI guard: a new authoritative store reader cannot bypass the version check |
| **CE-P6-8** | Cross-artifact agreement: a current ChainDb paired with a stale accumulator or stale reduced checkpoint fails closed |
| **CE-P6-9** | The bump trigger is mechanical: a mutation to the semantics-bearing surface without a bump or a justified lockfile update fails CI |
| **CE-P6-10** | Negative-tested — every gate above is proven to FAIL when its violation is introduced |

## OPEN DECISION — this rejects every store that exists today

Every current store predates the marker, so on the first open after this lands, **all of them fail
closed**. That includes the preprod store currently following live (~2 h of sync at time of writing)
and any preview store.

Two options:

1. **Reject unmarked stores, no stamp tool.** Clean, no hole. Costs a re-bootstrap of every live store.
2. **Provide an explicit operator stamp** ("I certify this store was produced under current
   semantics"). Not *implicit* migration, so it does not violate the prohibition list as written — and
   for a preprod store built entirely post-P3 the certification would in fact be true.

**Recommendation: option 1.** "I'm sure this store is fine" is exactly the judgement that failed in
P4 — the store looked structurally perfect and was three epochs stale. A stamp tool re-introduces
precisely the human judgement the gate exists to replace. The cost is bounded by the semantics-free
split above: block bytes are retained, so re-bootstrap re-derives rather than re-syncs.

This is the one decision that needs an explicit call before implementation.

## Scope exclusions

- **The `apply_epoch_boundary_full` mainnet denominator** (DC-LEDGER-13 allowlist) is NOT folded in.
  It is a reward-semantics change and belongs in its own slice. It enters this slice only if fixing it
  *determines* a version bump — i.e. only as a consumer of this mechanism, never as part of building it.
- **Re-derive-from-retained-blocks** recovery: out of scope; only the artifact split that keeps it
  possible is in scope.
- **Multi-venue behavioural parity testing**: out of scope; noted as the strengthening path for the
  bump trigger.

## Proposed registry IDs

`DC-STORE-10` (the gate), `DC-STORE-11` (cross-artifact agreement), `DC-STORE-12` (mechanical bump
discipline). Next free in the family is 10.
