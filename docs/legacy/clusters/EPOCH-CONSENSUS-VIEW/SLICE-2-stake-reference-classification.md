# EPOCH-CONSENSUS-VIEW — Slice 2 (scope): typed stake-reference classification (era-gated)

> **Status:** SCOPED (2026-06-20, pre-code). The SECOND slice of EPOCH-CONSENSUS-VIEW. Pure BLUE. Independently testable WITHOUT a leader slot (CIP-19 vectors + a Conway pointer fixture + real preview address samples). The mechanism (the cluster) stays UNAPPROVED beyond what each slice proves; no code until this doc is accepted.

## Purpose (the per-output attribution primitive)
Given a decoded `Address` and the active era, extract the typed **stake reference** of an output — the per-output primitive every stake aggregation depends on — through a **typed decoder + era-gated classifier**, never a fixed byte offset. This is the riskiest correctness surface of the cluster (Conway pointer retirement, key/script/reward/null per era), isolated and oracle-able on its own BEFORE the heavier aggregation + replay-window wiring (slice 3). This slice does NOT sum stake, does NOT touch the transient store, does NOT wire anything live.

## Binding attribution principle (from the design record)
Stake attribution is derived ONLY from a fully decoded canonical address form. **No fixed byte offset is authoritative across address variants or eras.** The `[29..57]` layout holds only for base addresses; pointer/enterprise/reward/Byron differ, and pointer attribution is era-gated. A byte-offset shortcut would be silently wrong on exactly the cases that matter.

## Grounded done-vs-net-new
- **DONE (reuse):** `ade_types::address::Address` (`address/mod.rs:13`) — the 5 forms `Base|Pointer|Enterprise|Byron|Reward`, each carrying raw bytes, discriminated by header byte; `Address::as_bytes()`; `address::Credential{KeyHash|ScriptHash}`; `shelley::cert::StakeCredential{KeyHash|ScriptHash}` (`cert.rs:48`). The `Address` enum already splits the 5 forms — but carries only raw bytes; it does NOT extract the staking part.
- **NET-NEW (this slice):** a typed `StakeRef` enum + a total, deterministic, era-gated `classify_stake_ref(address, era) -> StakeRefOutcome` that extracts the staking part, plus fail-closed handling of malformed address bytes.

## The classification contract (BLUE, era-gated)
The classifier is a pure total function:

> Given **canonical address bytes** and a **canonical era / protocol-version context**, classification returns ONE deterministic typed result:
> `StakeRefClass = Base(StakeCredential) | Pointer(PointerRef) | Null | Reject(MalformedAddress)`.
> **No classifier result directly changes stake totals** — it is a per-output reference, consumed later by the aggregation (slice 3).

The four results are semantically distinct and must stay distinguishable:
- **`Base(StakeCredential)`** — a base address's explicit staking credential (key/script per header bit 5), decoded from the canonical form.
- **`Pointer(PointerRef)`** — a **decoded pointer reference, UNRESOLVED**. It carries only the `(slot, txIx, certIx)` coordinates. It does **NOT** imply a stake credential, a stake contribution, or any eligibility — resolution (the `Ptr → credential` pointer-map lookup) and any contribution decision are **slice 3**. Pre-Conway a pointer address yields `Pointer(coords)`; **at Conway (PV9+) pointer stake is retired**, so the classifier yields `Null` directly (the address is spendable, contributes 0).
- **`Null`** — a valid NON-staking form (enterprise types 6–7, Byron type 8): all eras, no staking part, contributes nothing. A semantic result for *valid* input.
- **`Reject(MalformedAddress)`** — structurally invalid / under-length / bad-header bytes. **Distinct from `Null`**: `Null` is "valid form, no stake"; `Reject` is "not a valid address." Never collapse a malformed input to `Null`.

| Address form (header type) | Result | Era gating |
|---|---|---|
| Base 0–3 | `Base(Credential)` (key/script per bit 5) | all eras |
| Pointer 4–5 | `Pointer(coords)` (unresolved, implies nothing) | **pre-Conway**; **Conway (PV9+) → `Null`** (pointer stake retired) |
| Enterprise 6–7 | `Null` | all eras |
| Byron 8 | `Null` (bootstrap, non-delegable) | all eras |
| Reward 14–15 | classified for decoder completeness (see role restriction below) | all eras |
| malformed / under-length | `Reject(MalformedAddress)` | all eras |

**Reward-address role restriction (no reward-account smuggling).** A reward address (types 14–15) is a withdrawal target / staking-credential form, NOT an ordinary UTxO payment address. This slice classifies it for **decoder completeness only**; it must NOT be treated as an ordinary UTxO stake contribution. Whether a reward address can even appear as a `TxOut` payment address on the relevant ledger path is an entry obligation (below); until slice 3 proves the ledger rule, a reward-address-as-output is **prohibited from contributing output stake** (fail-closed or excluded, not summed). This prevents reward-account semantics leaking into per-output attribution.

## Invariants (proposed DC-EVIEW-02)
- **One deterministic typed result:** `classify(canonical_bytes, era_ctx) -> Base | Pointer | Null | Reject` — total, pure, deterministic. No HashMap/wall-clock/rand/float.
- **No total mutation:** no classifier result directly changes stake totals; classification is reference extraction only (aggregation is slice 3).
- **Typed-decode-only:** classification routes through the typed decoder; no fixed byte offset is the contract (a CI gate forbids a bare `[29..57]`-style slice as the attribution source).
- **Era/PV is bound consensus context:** the era / protocol-version used for the gate is a TYPED consensus-context input already bound to the block being processed — NEVER inferred from the raw address bytes, local config, wall-clock, or a caller-selected flag. (The pointer-retirement gate is exactly where a loose context source would produce divergent results.)
- **Era-gated pointer retirement:** a pointer address classifies to `Pointer(coords)` pre-Conway and to `Null` at/after Conway (PV9). Mechanically tested both ways against the bound context.
- **`Pointer` implies nothing:** a `Pointer` result is an unresolved coordinate, never a credential / contribution / eligibility.
- **Null forms:** enterprise + Byron classify to `Null` in all eras.
- **Malformed stays distinguishable:** a malformed / under-length / bad-header address (including a malformed-but-prefix-valid one) is `Reject(MalformedAddress)`, NEVER silently `Null`, never a panic. `Null` = valid non-staking form; `Reject` = invalid input.
- **Reward not smuggled:** a reward address is not treated as ordinary UTxO output stake (the role restriction above).

## Entry obligations (answer before code)
1. **Existing decoder completeness:** does anything already split a base address into payment `[1..29]` + staking `[29..57]`, a reward address `[1..29]`, and the pointer varints? Confirm key-vs-script discrimination (header bit 4 = payment-is-script; bit 5 = stake-is-script) and the base-128 varint decode (slot/txIx/certIx, with the `Word32/Word16/Word16` bound on the target ledger). Reuse vs build.
2. **Era/PV authority (load-bearing):** the era / protocol-version that drives the pointer-retirement gate MUST be a TYPED consensus-context input already bound to the block being classified — NOT inferred from the raw address bytes, local config, wall-clock, or a caller-selected flag. Identify that bound context source (`era_schedule.locate(slot).era` / the block's protocol params) and whether retirement keys on the **era** or the **protocol major version** (PV9 per Conway spec §9.1.2). Thread it explicitly; no ambient default. This is the obligation most able to create divergent results if sourced loosely.
3. **Malformed handling:** the `Reject(MalformedAddress)` shape + the policy — does a malformed address abort the materialization (fail-closed) or get recorded as a typed `Reject` the caller must handle? (Default: a typed `Reject`, never silent `Null`; the caller fails closed.) Confirm a malformed-but-prefix-valid address (a recognised header byte but a truncated/oversized body) is `Reject`, not `Null`.
4. **Reward-address role / ledger rule:** does the relevant ledger path permit a reward address (types 14–15) as a `TxOut` PAYMENT address at all? If NOT, the classifier still decodes it (decoder completeness) but it must be prohibited from ordinary UTxO output-stake contribution until slice 3 proves the ledger rule — never summed as output stake here. Confirm the rule; do not assume.

## MAC (mechanical acceptance — all hermetic, no leader slot)
1. **CIP-19 vectors:** known address bytes for each of base(×4 key/script combos), pointer, enterprise, reward(key/script), Byron → the expected `StakeRefClass` result.
2. **Conway pointer retirement (the deliberate fixture):** the SAME pointer-address bytes classify to `Pointer(coords)` under a pre-Conway bound context and to `Null` under a Conway+ bound context — proving the era gate keys on the bound consensus context, not the bytes.
3. **`Pointer` implies nothing:** the `Pointer(coords)` result carries only coordinates — assert it exposes no credential / contribution / eligibility (a type-level + value-level check).
4. **Null forms:** enterprise + Byron → `Null` in every era.
5. **Base key vs script + reward key vs script:** header bit 5 / bit 4 discrimination correct.
6. **Malformed → `Reject`, distinct from `Null`:** truncated / wrong-length / bad-header bytes → `Reject(MalformedAddress)`, never `Null`, never panic. **Explicitly include a malformed-but-prefix-valid case** (a recognised header byte with a truncated/oversized body) and assert it is `Reject`, NOT `Null` — the two results must stay distinguishable.
7. **Reward not summed as output stake:** a reward-address output (if the ledger path even permits one) is excluded from / fails closed on ordinary output-stake contribution — never classified as an ordinary `Base` contribution.
8. **Real preview samples:** classify a batch of real addresses from the preview UTxO dump without `Reject`; assert the distribution is sane (overwhelmingly `Base`, the expected handful of `Enterprise`/`Null`), as a smoke that the decoder handles live data.
9. **CI gate:** `ci/ci_check_eview_stake_ref_classification.sh` — asserts the era-gated pointer-retirement test exists, the typed-decode-only guard (no bare offset as the attribution source), the era/PV-from-bound-context guard (the classifier signature takes a typed context, not ambient state), and the malformed-distinct-from-Null test.

## Differential oracle
The FULL pool-distribution oracle match (vs `cardano-cli query stake-distribution` / `stake-snapshot` / `leadership-schedule`) requires aggregation and belongs to **slice 3**. Slice 2's oracle is CIP-19 test vectors + the Conway pointer fixture + real-sample classification — the classification is validated in isolation first.

## Hard prohibitions / non-goals
- NO stake aggregation / per-pool sum (slice 3); NO pointer→credential resolution (slice 3); NO reward-balance arithmetic (slice 3).
- NO transient-store wiring, NO bounded-replay-window wiring (slice 3); NO live wiring; NO `track_utxo=true` on the live path.
- NO fixed byte offset as the attribution contract.
- NO `EpochConsensusView` emission yet (that is the aggregation + projection slice).

## Where it sits
Slice 1 (committed `85fbc04f`, DC-EVIEW-01) proved the disposable transient substrate. Slice 2 (this) proves the per-output stake-reference classification — the riskiest correctness piece — in isolation. Slice 3 then aggregates (classify → resolve → sum per registered+delegated pool, + reward balances) over a bounded replay window USING the slice-1 transient store, emits the bound `EpochConsensusView`, and validates against the cardano-cli oracle.
