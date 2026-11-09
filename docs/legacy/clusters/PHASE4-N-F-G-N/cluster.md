# PHASE4-N-F-G-N — WarmStart forge eta0 from the recovered seed-epoch consensus input (T-REC-04 / DC-CINPUT-03)

> **Grounded in a proven bug (G-N is the fix; diagnosis done).** The C1 rerun + a forge instrument (since
> reverted) proved Ade forged its block-0 header VRF over **`eta0 = Nonce::ZERO`** while the C1 follower
> verifies over the genesis nonce **`953a4c34…`** → `VRFKeyBadProof`. The VRF variant, proof format (80-byte
> draft-03), key, and input *construction* are all correct (Ade's draft-03 `verify_praos_vrf` verifies real
> mainnet + preprod Conway headers — both draft-03; draft-13 was measured dead). The defect is purely the
> recovered nonce. S1-recon then found the deeper cause: the persisted seed-epoch sidecar **omits eta0**.
> Grounding: `[[project_phase4_c1_genesis_rehearsal_live_state]]`.

## §1 Primary invariant (T-REC-04, with DC-CINPUT-03)
**T-REC-04 (true):** the WarmStart forge `eta0` (`chain_dep.epoch_nonce`) MUST come from the
imported/recovered **consensus input**, never from a snapshot placeholder and never from genesis. A
snapshot-seeded `Nonce::ZERO` must never reach the forge / self_accept path. Authoritative recovered state
must be explicit, persisted, replayable, and comparable — eta0 is one such recovered consensus input, carried
as an explicit recovered artifact, not hidden inside the snapshot.

**DC-CINPUT-03 (derived):** the Praos VRF leader/header input (`praos_vrf_input(slot, eta0)`) uses the
Cardano epoch nonce carried by the recovered `SeedEpochConsensusInputs` — so a forged Conway header verifies
under a real peer's `mkInputVRF(slot, eta0)`.

## §2 The defect (proven, not assumed)
The actual chain:
```
admission imports eta0 (LiveConsensusInputsCanonical.epoch_nonce = 953a4c34, from consensus-inputs.json)
  → merge/persist DROPS eta0 (SeedEpochConsensusInputs omits an epoch_nonce field)
    → --mode node WarmStart has NO recoverable eta0
      → the snapshot's chain_dep Nonce::ZERO (seeded at admission/bootstrap.rs:164) reaches the forge
        → forge signs the header VRF over ZERO → C1 follower rejects (VRFKeyBadProof)
```
`SeedEpochConsensusInputs` (`ade_ledger/src/seed_consensus_inputs.rs`) carries `anchor_fp, epoch_no,
active_slots_coeff, total_active_stake, pool_distribution` — **no `epoch_nonce`**. So at WarmStart the lineage
restore returns the leader-schedule inputs but not eta0; the only recovered chain_dep is the snapshot's ZERO.
`bootstrap_initial_state` (bootstrap.rs:234) returns that ZERO chain_dep to forge/self_accept. The
`admission/bootstrap.rs` comment (198-200) predicted it: *"before FRAG + tag-24 unwrap landed, no block ever
reached header validity, so the ZERO-nonce chain_dep was masked."* Self-accept trap: Ade's validator reads
the same recovered ZERO → producer/validator agree; the C1 uses `953a4c34` → rejects. Proven:
`FORGE-DIAG sched eta0=[00,…,00]`, real pool vk `12c1ceb0…`, 80-byte draft-03 proof.

## §3 The fix — persist eta0 in the seed-epoch sidecar, then overlay at WarmStart
Keep the authority split: **snapshot = ledger/chain skeleton; the seed-epoch consensus sidecar = the Praos
consensus inputs, INCLUDING eta0.** Concretely:
1. **Add `epoch_nonce` to `SeedEpochConsensusInputs`** — eta0 is a seed-epoch consensus input that belongs
   there; its omission is the defect. Versioned CBOR (bump the sidecar schema version; CN-CINPUT-01 sole
   encoder) — fail-closed on the schema change.
2. **Persist it from `LiveConsensusInputsCanonical`** — the admission `merge` carries the imported
   `epoch_nonce` into the persisted sidecar.
3. **Recover it during WarmStart** — `bootstrap_initial_state` reads the sidecar's `epoch_nonce`.
4. **Overlay onto `PraosChainDepState`** — set the recovered `chain_dep.epoch_nonce` (+ seed-epoch
   `evolving_nonce`, equal to eta0 at the seed epoch) from the recovered sidecar, in the single
   `bootstrap_initial_state` authority. NOT "replace snapshot with bundle"; NO VRF code; NO genesis-derived
   nonce; NO manual/C1-only override; NO hiding eta0 only in the snapshot.

**Safety call (fail-closed, not backward-compat fallback):** an existing/old sidecar that lacks `epoch_nonce`
MUST fail closed — a structured `SeedEpochConsensusInputsMissingEpochNonce` (or schema/version mismatch),
NEVER a default-to-zero. That prevents the exact bug surviving as a compat fallback. The existing C1 staged
store (whose sidecar predates eta0) is therefore regenerated; an old store must not be silently accepted as
forge-capable.

## §6 TCB color
A versioned change to a persisted consensus-input record (`ade_ledger` codec — closed CBOR grammar,
fail-closed on schema) + recovery assembly in `ade_runtime::bootstrap` (deterministic field reconciliation).
The forge consumes the corrected `chain_dep` through the UNCHANGED BLUE Praos VRF path. No VRF crypto, no new
authority.

## §7 Slices
| Slice | Scope | CE | Registry | Status |
|---|---|---|---|---|
| **S1** | Add `epoch_nonce` to `SeedEpochConsensusInputs` (versioned, fail-closed); persist it via the admission merge; recover + overlay onto `chain_dep` at WarmStart; regression (persist→recover round-trip + ZERO snapshot → recovered eta0 == 953a4c34) + fail-closed test | CE-G-N-1 | T-REC-04 + DC-CINPUT-03 → enforced | done |
| **S2** | Regenerate the C1 staged store (re-run admission) + live C1 confirmation: forge eta0 == `953a4c34`; follower verifies header VRF (no `VRFKeyBadProof`) + proceeds past `DownloadedHeader` | CE-G-N-2 | operator-gated | done — live-confirmed 2026-06-04 |

## §8 Cluster Exit Criteria (CE-G-N-1, mechanical)
1. `SeedEpochConsensusInputs` gains an `epoch_nonce` field.
2. The sidecar encoder/decoder is versioned (schema-version bump) and fail-closed for the schema change.
3. The admission `merge` persists `epoch_nonce` from `LiveConsensusInputsCanonical`.
4. WarmStart recovery applies the recovered sidecar `epoch_nonce` to the forge `chain_dep`.
5. Regression: imported eta0 = `953a4c34…`; snapshot `chain_dep = Nonce::ZERO`; the persisted sidecar
   round-trips eta0; the recovered forge `chain_dep.epoch_nonce == 953a4c34…` (and `evolving_nonce` likewise).
6. An old/missing sidecar `epoch_nonce` fails closed (`SeedEpochConsensusInputsMissingEpochNonce` or
   version/schema mismatch) — never a default-to-zero. Explicit migration only if mechanically safe.
7. The C1 staged store is regenerated; an old store must not be silently accepted as forge-capable.

**CE-G-N-2 (operator-gated):** the regenerated-store C1 rerun shows forge `eta0 == 953a4c34` and **no
`VRFKeyBadProof`** (a downstream failure would be a NEW blocker). `blocked_until_operator_c1_genesis_
successor_rehearsal`; no RO-LIVE flip; no acceptance claim without the follower log through `correlate`.

## §9 Replay obligations
Persist→recover of eta0 is deterministic + replay-equivalent (same canonical inputs ⇒ byte-identical sidecar
⇒ same recovered chain_dep). Covered by the S1 round-trip + recovery regression tests.

## §10 Invariants
- **Adds:** `T-REC-04` (WarmStart forge eta0 from the recovered consensus input, never snapshot-ZERO/genesis)
  + `DC-CINPUT-03` (Praos VRF input uses the recovered sidecar eta0), declared → enforced at S1.
- **Preserves / cross-ref:** `DC-EPOCH-03` (seed-epoch eta0 frozen — G-N supplies the *correct* recovered
  value it freezes), `CN-CINPUT-01` (sole sidecar encoder — versioned here), `DC-CINPUT-01/02a/02b`,
  `CN-FORGE-04` (Praos VRF), `T-REC-01/02/03`, `CN-WIRE-10/11`, `DC-NODE-09`, `RO-LIVE-01` (no flip),
  `OP-OPS-04/05` (operator extraction path).

## §11 Forbidden during this cluster (hard boundaries)
- **no genesis-derived nonce; no manual / C1-only override** — eta0 comes from the recovered sidecar (sourced
  from the imported bundle).
- **no VRF variant change** (draft-03 is correct).
- **no private-only branch** — the corrected recovery is the default `--mode node` WarmStart.
- **no hiding eta0 only in the snapshot** — it is an explicit recovered consensus input (option (b) rejected).
- **no default-to-zero / accept-both fallback** — old sidecars without eta0 fail closed.
- **no weakening of `self_accept`** — it stays strict, now reading the correct recovered nonce.
- **no RO-LIVE flip; no acceptance claim** without the follower log through `correlate`.

## §12 Tiering
- **true:** authoritative recovered state explicit/persisted/replayable/comparable (T-REC-04).
- **derived:** Praos VRF input uses the Cardano epoch nonce from `SeedEpochConsensusInputs` (DC-CINPUT-03).
- **release:** the C1 rerun must show no `VRFKeyBadProof` before any rehearsal acceptance claim (CE-G-N-2,
  under RO-LIVE-01 — no flip).
- **operational:** the operator bundle sources eta0 from the C1/preprod node extraction path
  (`import_live_consensus_inputs`); never hand-authored.

## §13 Cluster close (2026-06-04) — NARROW CLAIM
**Claim (what G-N proves):** the WarmStart-recovered forge `eta0` (`chain_dep.epoch_nonce`) now comes from
the persisted seed-epoch consensus input (`SeedEpochConsensusInputs.epoch_nonce`, v2), NOT `Nonce::ZERO`; an
old v1 sidecar fails closed; and a real C1 cardano-node follower's header VRF verification proceeds PAST
`VRFKeyBadProof`.

**Live evidence (S2, 2026-06-04; C1 cardano-node 11.0.1 follower, magic 42):**
- old v1 store → WarmStart fails closed (exit 42, `SeedConsensusSidecarDecode(Structural "outer array has wrong field count"); failing closed … no bundle fallback`).
- regenerated v2 store → `--mode node` recovers (no fail-closed) + forges block 0 (slot 107405).
- follower: **`VRFKeyBadProof` count = 0**; `WarmToHot → ChainSync.Client.FoundIntersection → DownloadedHeader (slot 107405) → BlockFetch AddedFetchRequest…CompletedBlockFetch` (fetched Ade's full block-0 body).

**Mechanical:** `T-REC-04` + `DC-CINPUT-03` enforced; `ci/ci_check_warmstart_eta0_overlay.sh`; regression
`warm_start_overlays_recovered_eta0_onto_chain_dep_g_n` + the seed-consensus / bootstrap-A3b / merge /
genesis / mithril / genesis_pinning / node_sync / node_lifecycle suites pass. **NO VRF crypto/variant change.**

**Explicitly NOT claimed:** block adopted by the follower; C1 genesis rehearsal complete; RO-LIVE flip;
preprod/bounty success. Adoption was NOT confirmed — Ade crashed feed-side ~2.4 s after the body fetch,
before the follower logged adoption; no acceptance claim without the follower log through `correlate`.

**New blocker (separate — NOT in G-N):** Ade crashed fail-closed (exit 43) on its FEED/receive side —
`relay run-loop sync step failed (Receive(Validity(Body(Decoding(DecodingError { offset: 0, reason: UnexpectedType })))))` — `run_node_sync` receiving a block from `:3010` and failing to decode the BODY. Scoped
as **PHASE4-N-F-G-O — Feed-side block-body decode compatibility** (evidence-first: capture the exact received
bytes, then fix the shared receive decoder; likely a tag-24 / framing mismatch, but do not fix from hypothesis).
