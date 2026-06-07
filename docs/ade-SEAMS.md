# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **458 canonical types**, **142 CI checks** at HEAD (`6363683e`, PHASE4-N-AE.F — receive idempotency at the durable-admit chokepoint: a hash-exact already-have no-op in `pump_block` that survives the post-adoption echo, `DC-NODE-16`).
> Reads the CODEMAP (`docs/ade-CODEMAP.md` — **458 canonical types / 141 CI / 340 rules**, pinned at `a76672b9`,
> the PHASE4-N-AE close) for the module list + TCB colors, and the invariant registry
> (`docs/ade-invariant-registry.toml` — **341 entries** at HEAD `6363683e`: 209 enforced / 20 partial / 112 declared)
> for the rule IDs that gate each closed surface. **Count reconciliation (load-bearing — read honestly): this SEAMS
> and the registry AGREE at HEAD (458 / 142 / 341); the on-disk CODEMAP LAGS at `a76672b9` (458 / 141 / 340) because
> AE.F is a hygiene/correctness follow-on that adds NO crate, NO module, NO canonical type, and NO TCB-color change —
> the module inventory + colors SEAMS reads from CODEMAP are unaffected, so CODEMAP was not regenerated (refresh-on-next-cluster
> item, not a discipline gap).** N-AE.F added **no new crate, no new module, and no TCB color change**: the gate lives
> inside the EXISTING inventoried RED module `ade_runtime::forward_sync::pump` (`pump_block`); there is **ZERO BLUE diff**
> (the additive `DecodedBlock.prev_hash` field was the N-AE BLUE change, already on disk; AE.F touches no BLUE file).
> **TCB-color note (carried from the N-AE close, per CODEMAP, still accurate):** the chain-sync server change
> (`DC-PROTO-10`, `crates/ade_network/src/chain_sync/server.rs`) is **BLUE** — `chain_sync/` is a BLUE `core_paths`
> submodule and the file carries the BLUE banner; the forge-admissibility classifier is GREEN-by-function inside RED
> `ade_node::node_sync`; the serve projection + node wiring + the AE.F idempotency gate are RED.
> **N-AE.F opened NO new ingress surface, NO new registry (closed or extensible), NO new version-gated contract,
> NO new closed enum, and NO new plugin/extension point.** It adds a single deterministic guard INSIDE the existing
> RED durable-admit chokepoint `pump_block`: immediately after `decode_block` and BEFORE the BLUE chokepoint reducer,
> it queries `db.get_block_by_hash(&decoded.block_hash)` and, if `Some(stored)` AND `stored.slot == decoded.slot`,
> returns `Ok(None)` (an idempotent no-op — no reducer step, no WAL append, no tip change). It mints **+1 rule**
> (`DC-NODE-16`, enforced) + **+1 CI gate** (`ci_check_receive_idempotency.sh`). The gate is a plain conditional +
> early return over the already-closed `ChainDb` membership query — **NOT** a new closed enum, **NOT** a new attach
> point: a DIFFERENT block (different hash) at/before the last-applied slot still falls through to the UNCHANGED BLUE
> header authority and fails closed (`SlotBeforeLastApplied`), and `pump_block` stays the sole durable tip authority.
> **SEAMS is therefore MATERIALLY UNCHANGED from the N-AE pin — AE.F adds NO new seam.** This refresh re-pins
> HEAD/counts and records the single chokepoint-guard tightening against the seam it touches (§2 the forged-block /
> received-block durable-admit domain; §5 +1 CI gate, 141 → 142; §6 the new RED idempotency-chokepoint prohibition;
> §7 examined for a candidate seam and NONE found). The N-AE through G-A history below is carried unchanged.
>
> ### PHASE4-N-AE.F (`6363683e`) — receive idempotency at the durable-admit chokepoint (`DC-NODE-16`)

>
> **This regeneration is a single-slice follow-on refresh, applied directly to the on-disk SEAMS.** The prior
> on-disk SEAMS was pinned at the PHASE4-N-AE close (`a76672b9` / **458** canonical types / **141** CI / **340**
> rules). It is brought current to HEAD `6363683e` (the *AE.F receive idempotency* commit — **458** canonical types
> / **142** CI / **341** rules). It folds in **PHASE4-N-AE.F** — the post-CE-A5 echo fix. After the real
> cardano-node 11.0.1 relay `AddedToCurrentChain` Ade's forged block 17 (the CE-A5 manifest, venue c2ae18), the
> relay re-announced that block BACK to Ade over Ade's follow link; the BLUE header authority correctly rejected it
> as `SlotBeforeLastApplied { last: 421, attempted: 421 }`, terminating the continuous run (exit 43) AFTER
> adoption. AE.F makes Ade no-op that echo so a long-running C2-LOCAL / preprod relay survives its own served tip
> coming back.
>
> **N-AE.F opened NO new seam.** The `a76672b9..6363683e` span touches **ONE source file — RED, already inventoried,
> with ZERO new `struct`/`enum`:** `crates/ade_runtime/src/forward_sync/pump.rs` (the RED durable-admit chokepoint
> `pump_block`, which already performs ChainDb/WAL I/O). **No new crate, no new module, no new registry file, no
> closed-enum addition, no version-gated contract bump, and NO BLUE diff.** The seam-relevant change — recorded
> under §2 (the forged-block / received-block durable-admit domain gains an idempotent fast-path BEFORE the BLUE
> reducer; the data-only/authoritative split is unchanged — the guard is a deterministic durable-store read, not a
> new authority), §5 (+1 CI gate, 141 → 142), §6 (the new RED idempotency-chokepoint prohibition), and §7 (NO new
> candidate seam) — is:
>
> - **The receive-idempotency chokepoint guard (AE.F — `DC-NODE-16`, enforced).** In RED
>   `ade_runtime::forward_sync::pump::pump_block`, immediately after `decode_block` and BEFORE the BLUE chokepoint
>   reducer (`forward_sync_step` / `validate_and_apply_header`), the chokepoint queries
>   `db.get_block_by_hash(&decoded.block_hash)`; if `Some(stored)` AND `stored.slot == decoded.header_input.slot`
>   it returns `Ok(None)` — an **idempotent no-op**: no reducer step, no `WalEntry::AdmitBlock`, no tip change, so
>   the post-state (ledger, chain_dep, ChainDb tip, WAL length) is IDENTICAL and the WAL never records the
>   re-announce (replay-equivalent — `T-REC-05` / `DC-WAL-02` preserved). The decision key is **HASH equality vs
>   the durable store**, never slot alone: a DIFFERENT block (different hash) at/before the last-applied slot
>   returns `None` from `get_block_by_hash`, falls through to the UNCHANGED BLUE header authority, and fails closed
>   (`SlotBeforeLastApplied` / `BlockNoOutOfOrder` — AE-F-INV-2, the fail-closed boundary). No skip-past, no
>   fork-choice (`DC-CONS-03` untouched — AE-F-INV-3). **This is a deterministic conditional + early return over
>   the already-closed `ChainDb` membership query, NOT a new closed enum, NOT a registration surface, NOT a second
>   tip-advance path** — `pump_block` stays the sole durable tip authority (`DC-NODE-12`).
> - **TCB-placement refinement (load-bearing, vs the AE.F invariants sketch).** The `/invariants` sketch
>   (`docs/planning/phase4-n-ae-f-echo-idempotency-invariants.md` §5) proposed a BLUE `ReceiveOutcome::AlreadyHave`
>   reducer variant. The implementation instead places the gate at the **RED** chokepoint with **NO BLUE change and
>   no new reducer input**: `get_block_by_hash` is a DETERMINISTIC durable-store query (not nondeterminism), so the
>   already-have decision is correctly RED orchestration over a deterministic read, and the BLUE authority is left
>   untouched and still fail-closes every block that reaches it. This is the right FC/IS split — the chokepoint
>   already owns the ChainDb/WAL I/O; the guard adds one read + one early return, no new BLUE canonical type or
>   reducer variant (458 canonical types unchanged).
>
> **Registry → 341 rules** (209 enforced / 20 partial / 112 declared). **ONE NEW** (`introduced_in =
> "PHASE4-N-AE"`): `DC-NODE-16` (`tier = derived`, `enforced`, `ci_script = ci/ci_check_receive_idempotency.sh`,
> `cross_ref = [DC-NODE-12, DC-PROTO-09, DC-PROTO-10, DC-CONS-03, T-REC-05, DC-WAL-02]`) — receive idempotency at
> the durable-admit chokepoint. **No rule strengthened, no rule weakened** by AE.F (the BLUE authority is
> unchanged; the existing replay/durability rules are preserved, not re-tagged). **NET +1 CI gate (141 → 142):**
> `ci_check_receive_idempotency.sh` — fences the gate as hash-keyed (`get_block_by_hash(&decoded.block_hash)`,
> never slot-only), gated-before-the-reducer (the `return Ok(None)` precedes `forward_sync_step`), and
> slot-consistent (`stored.slot == decoded.header_input.slot`). DC-NODE-16 mints a dedicated gate (it is NOT
> test-only).
>
> **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** N-AE.F makes the receive path no-op a
> byte-identical already-applied block re-announced by the peer — and **nothing more**. There is **NO BLUE change**
> (the BLUE header authority `validate_and_apply_header` / `block_validity` is UNCHANGED and still fail-closes every
> block that reaches it; 458 canonical types unchanged); **NO new closed/extensible registry**; **NO version-gated
> contract or wire-grammar change**; **NO fork-choice / multi-producer intake** (`DC-CONS-03` untouched — the skip
> is exact-match idempotency, never chain selection, never a skip-past a gap, never accepting a better chain); the
> idempotent skip is gated on **HASH equality against the durable store**, never slot alone, so the
> `SlotBeforeLastApplied` fail-close stays the fence for any DIFFERENT block at/before the last-applied slot
> (AE-F-INV-2); and there is **NO second durable tip-advance path** — `pump_block` remains the sole durable tip
> authority (`DC-NODE-12`), and the no-op advances no tip at all. **NO RO-LIVE flip** — surviving the post-adoption
> echo is a continuous-run precondition for long-running C2-LOCAL / preprod operation, NOT operator-witnessed
> bounty acceptance (only a committed `ba02_evidence::correlate` manifest over the preprod tip advances
> `RO-LIVE-01`, which stays `partial` / operator-gated).
>

> ### PHASE4-N-AE cluster close (`a76672b9`) — recover→serve continuity + forge admissibility (`DC-NODE-15` / `DC-NODE-14` / `DC-CONS-24` / `DC-PROTO-10`)

>
> **This regeneration is a single-cluster cluster-close refresh, applied directly to the on-disk SEAMS.** The
> prior on-disk SEAMS was pinned at the PHASE4-N-AC close (`1d54abb4` / **458** canonical types / **138** CI /
> **336** rules). It is brought current to HEAD `a76672b9` (the *AE.E chain-sync server FindIntersect cursor* commit
> — the PHASE4-N-AE close — **458** canonical types / **141** CI / **340** rules). It folds in the **single closed
> cluster PHASE4-N-AE** (slices AE.A / AE.B / AE.C / AE.E — a `--mode node` forge must produce a peer-ADOPTABLE
> successor: it is admissible only when the durable servable tip equals the followed peer tip, its parent
> byte-equals that peer-visible tip, every claimed forge parent is servable-or-peer-intersectable in the durable
> served lineage, and a non-Origin FindIntersect leaves the chain-sync server's read cursor AT the intersect). The
> headline is that a real cardano-node 11.0.1 relay `AddedToCurrentChain` an Ade-forged successor (CE-A5 manifest,
> venue c2ae18, forged block 17 @ slot 421, issuerHash `a1ed4e04` == blake2b-224(pool1 cold VK)).
>
> **N-AE opened NO new seam.** The whole `25ddeebd..a76672b9` span (PHASE4-N-AC was already on disk; the AE work
> spans the PHASE4-N-AD durability-proof close + the four AE slices) touches **five** source files — **TWO under the BLUE `core_paths`, both adding ZERO new `struct`/`enum`**:
> `crates/ade_ledger/src/block_validity/header_input.rs` (additive read-only field `DecodedBlock.prev_hash`, see
> below) + `crates/ade_network/src/chain_sync/server.rs` (BLUE producer chain-sync server — an existing-field
> `last_announced` cursor write, the AE.E fix); and THREE RED/GREEN: `crates/ade_node/src/node_lifecycle.rs`
> + `crates/ade_node/src/node_sync.rs` (RED wiring + the GREEN-by-fn forge-admissibility classifier) +
> `crates/ade_runtime/src/network/served_chain_projection.rs` (RED serve projection — the proof-gated FindIntersect
> branch). **No new crate, no new module, no new registry file, no closed-enum addition to any wire grammar, no
> version-gated contract bump.** The seam-relevant changes — recorded under §1 (the producer-forge pipeline gains a
> fail-closed forge-admissibility *step* BEFORE leadership, and the N2N serve surface's FindIntersect resolution is
> tightened; **no pipeline step is reordered and no new ingress surface appears**), §2 (the forged-block-serving
> domain's data-only/authoritative split is unchanged — the new branch is still data-only serve over the durable
> ChainDb projection; the crash-recovery domain gains the AE.C prior-fp seed), §3 (**NO new closed or extensible
> registry** — the FindIntersect-only projection is analysed below as a candidate extension point and REJECTED),
> §4 (**NO frozen-contract / wire-format / hash-algorithm change** — the closed `Point` enum + chain-sync message
> set + tag-24 envelope + block codec are reused UNCHANGED), §5 (+3 CI gates, 138 → 141), §6 (the new RED/GREEN
> forge-admissibility + no-synthetic-anchor-bytes prohibitions), and §7 (NO new candidate seam — the AE.B
> projection is a COMPLETION of the existing serve projection, not a new attach point) — are:
>
> - **The forge-admissibility gate (AE.A — `DC-NODE-15` / `DC-CONS-24`, enforced).** A closed classifier sibling to
>   `forge_epoch_admission` (the `DC-EPOCH-03` precedent), computed BEFORE leadership in RED `ade_node::node_sync`:
>   a forge is admissible iff `durable_servable_tip.{hash, block_no} == followed_peer_tip.{hash, block_no}`, else
>   it fails closed to a typed `ForgeRefused::NotCaughtUp { local_servable_tip, followed_peer_tip, reason }` (a
>   structured value, NOT a log line). The `recovered.tip` fallback as a forge base is REMOVED. The followed-peer
>   tip is converted to a structured, replayable admissibility input (`FollowedPeerTipSignal`) — it may *prevent* a
>   forge, it is **never** a chain selector (it never reaches `select_best_chain` / `chain_selector`). The new
>   sums (`NotCaughtUpReason`, `ForgeFollowedTipAdmission`, `ForgeRefused`, `NodeForgeOutcome`, `FollowedPeerTipSignal`)
>   are **closed RED/GREEN error/signal sums in `ade_node::node_sync`** — NOT canonical types, NOT registries, NOT
>   wire grammar.
> - **The recovered-anchor serve projection (AE.B — `DC-NODE-14`, enforced; Option B locked).** RED
>   `ChainDbServedSource::intersect` gains a SECOND, proof-gated branch: it projects the `prev_hash` of the
>   EARLIEST servable `StoredBlock` (a private helper `earliest_servable_block_prev_hash`, bounded — reads exactly
>   the earliest block, `DC-SERVEMEM-01`) as a **FindIntersect-only** point, **iff** a real servable successor
>   exists. It **never serves bytes** for that point (`get_block_by_hash` / `serve_range` stay empty → BlockFetch
>   refuses structurally; no synthetic `StoredBlock`, no synthetic CBOR). Recover-only (no `StoredBlock`) ⇒ no
>   projection (fail-closed). A `PrevHash::Genesis` root ⇒ no projection. **This is the candidate seam flagged for
>   examination — see the dedicated note at the end of this subsection. It is a CLOSED, proof-gated read-only
>   projection rule, NOT an extension point.**
> - **The chain-sync server FindIntersect cursor fix (AE.E — `DC-PROTO-10`, enforced — the actual CE-A5 closer).**
>   BLUE `producer_chain_sync_serve` in `ade_network::chain_sync::server` (a BLUE `core_paths` submodule): after answering `IntersectFound(point)`,
>   the server's read cursor `last_announced` IS that point — so the next `RequestNext` serves `next_after(point)`
>   (the successor the client rolls onto), never `next_after(None)` (the chain start = block 0, which a client
>   intersecting at its own tip rejects as `UnexpectedBlockNo(tip+1)(0)`). `last_announced = Block → Some((slot,
>   hash)) | Origin → None` (Origin keeps the cursor `None` — serve-from-start is correct for Origin-sync clients,
>   which is why earlier producer-serve clusters passed). **This is a server-internal cursor-state fix — the
>   chain-sync wire grammar (the `ChainSyncMessage` set, the closed `Point` enum) is UNCHANGED;** `FindIntersect` /
>   `IntersectFound` / `RequestNext` / `RollForward` are the existing messages, used unchanged.
> - **The recover→follow WAL prior-fp seed (AE.C — strengthens `DC-WAL-02` + `T-REC-05`).** RED
>   `ade_node::node_lifecycle`: the live `ForwardSyncState` prior-fp is seeded from `fingerprint(&state.ledger)`
>   (the recovered ledger tip being extended), never zero — so the first followed `AdmitBlock` chains from the
>   WAL-tail post_fp and a recover→followed store warm-starts replay-equivalently (it was failing `ChainBreak`,
>   exit 42). No new accept-break/skip path is added to WAL `verify_chain` / `replay_from_anchor` — the fix seeds
>   the chain correctly, it does NOT loosen recovery. **No new seam — a correctness fix to the existing crash-recovery
>   provenance fold.**
> - **The lone additive BLUE symbol: `DecodedBlock.prev_hash`** (`ade_ledger::block_validity::header_input` —
>   already PARSED from the header, now EXPOSED read-only so the RED serve projection can prove the earliest
>   servable block's parent). **This is an additive read-only field, NOT a new canonical type and NOT a new seam**
>   (458 canonical types unchanged). `T-KEY-01` / no-signing-in-BLUE and the FC/IS partition are preserved; the
>   BLUE block-validity / fork-choice / forge authorities are reused UNCHANGED (`fork_choice` / `select_best_chain`
>   is NOT on this path — it stays the follow/`chain_selector` authority, untouched; AE.A adds **no** fork-choice;
>   multi-producer fork-choice is an explicitly-deferred separate cluster, Gap 1).
>
> **Registry → 340 rules** (208 enforced / 20 partial / 112 declared). **FOUR NEW** (all `introduced_in =
> "PHASE4-N-AE"`): `DC-NODE-15` (forge admissibility = `durable_servable_tip == followed_peer_tip` else structured
> `ForgeRefused::NotCaughtUp`; enforced), `DC-CONS-24` (forged parent hash byte-equals the peer-visible selected
> tip; enforced), `DC-NODE-14` (every claimed forge parent is servable-or-peer-intersectable in the durable served
> lineage — partial after AE.A's followed-tip clause, ENFORCED after AE.B's recovered-anchor clause), `DC-PROTO-10`
> (chain-sync server FindIntersect cursor; enforced — the CE-A5 closer). **Strengthened (registry ground truth —
> `strengthened_in += "PHASE4-N-AE"` on exactly these 7):** `DC-EPOCH-03` (a sibling fail-closed forge-admissibility
> boundary), `CN-CONS-06` (the AE.E cursor fix — the served successor a peer rolls onto), `CN-CONS-07` (serve
> provenance now also fences sanctioned-writer-only anchor materialization / intersect-point-without-bytes),
> `DC-WAL-02` (AE.C: the first followed `AdmitBlock.prior_fp` chains from the recovered ledger-tip fingerprint on the
> live recover→follow path), `DC-NODE-05` (forge subordinate to the sync spine — the forge-admissibility gate makes
> the forge WAIT for the followed tip), `T-REC-05` (replay determinism extends to the recovered→followed→forged
> served chain + AE.C recover→follow→warm-start), `DC-CONS-23` (extend-only now also fences the no-synthetic-anchor-
> bytes rule). **No rule weakened.** _(The AE.B recovered-anchor intersect projection TIGHTENS the `DC-NODE-13`
> serve-projection surface, but the registry does NOT carry a `strengthened_in += "PHASE4-N-AE"` tag on `DC-NODE-13`
> — the cluster doc §10 prose named it under "Strengthens", the registry is canonical, and the new `DC-NODE-14`
> recovered-anchor clause is the formally-introduced rule; `DC-NODE-13` is cross-referenced, not re-tagged.)_ **NET +3 CI gates (138 → 141):** `ci_check_forge_followed_tip_admission.sh`
> (AE.A — no `recovered.tip` forge-base fallback; forge fires only when `durable_servable_tip == followed_peer_tip`;
> typed `ForgeRefused::NotCaughtUp`; the peer-tip signal never reaches `select_best_chain`), `ci_check_recovered_anchor_intersectable.sh`
> (AE.B — FindIntersect-only + proof-gated + NO synthetic bytes for the projected anchor), and
> `ci_check_recover_follow_wal_lineage.sh` (AE.C — both prior-fp seeds are `fingerprint(&state.ledger)`, never zero;
> no new WAL accept-break/skip path). The containment gates (`ci_check_node_run_loop_containment.sh`,
> `ci_check_served_chain_projection.sh`, `ci_check_loop_planner_closed.sh`) remain green — no containment invariant
> relaxed.
>
> **Candidate-seam analysis (the AE.B FindIntersect-only projection) — examined and REJECTED as a new seam.** The
> task flagged the serve projection's new intersect branch as a possible extension point. It is **not** one, for
> three reasons. (1) **No registration path.** There is no `register_*` / `add_*` / plugin-trait / `Box<dyn Trait>`
> / map-of-handlers — the branch is a single closed `match`/`for` over the already-closed `Point` enum inside one
> RED method; nothing pluggable enters at runtime. (2) **It is proof-gated and read-only, not open-within-constraints.**
> The projection advertises a point **iff** the earliest servable `StoredBlock`'s `prev_hash` equals an offered
> point (Ade can *prove* it holds a real servable successor); it serves **no bytes**; recover-only and
> `PrevHash::Genesis` cases fail closed. That is the shape of a CLOSED behavior (like the §3 closed registries), the
> opposite of an extensible registry's "grows at runtime within constraints." (3) **It COMPLETES an existing seam
> rather than opening one.** The serve projection (`ChainDbServedSource`, the `--mode node` read-only serve view of
> the durable ChainDb) is the SAME surface already documented in §1/§2 since N-U S3 / `DC-NODE-13`; AE.B closes the
> recovered-anchor clause of `DC-NODE-14` ON that surface. New work still attaches to the serve surface the same way
> it did before (extend the durable ChainDb via the sanctioned writers `pump_block` / `bootstrap_initial_state`; the
> projection is a consequence, not an attach point). It is therefore recorded as a TIGHTENING of the `DC-NODE-13`
> serve-projection surface (cross-referenced — the registry does NOT carry `strengthened_in += "PHASE4-N-AE"` on
> `DC-NODE-13`; the new `DC-NODE-14` recovered-anchor clause is the formally-introduced rule) plus a strengthening of
> `CN-CONS-07` / `DC-CONS-23`, NOT a new §3 entry and NOT a new §7 candidate.
>
> **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** N-AE makes a `--mode node` forge produce a
> peer-ADOPTABLE successor and tightens the serve/chain-sync path so a real relay can FindIntersect at the forged
> parent and roll forward onto it — and **nothing more**. There is **NO new BLUE authority or canonical type** (the
> only NEW BLUE symbol is the additive read-only field `DecodedBlock.prev_hash`; the AE.E `chain_sync/server.rs`
> change is an existing-field cursor write, not a new symbol; 458 unchanged); **NO new closed/extensible
> registry**; **NO version-gated contract or wire-grammar change** (the closed `Point` enum + chain-sync message set
> are reused unchanged — DC-PROTO-10 is a server-internal cursor-state fix, AE.E touches no grammar); **NO
> fork-choice / multi-producer intake** (Gap 1 — explicitly a separate future cluster); **no synthetic servable
> `StoredBlock` bytes** from snapshot/ledger/block-number/hash (hash-critical paths require preserved original
> bytes); and **no second durable tip-advance path** (forged + received blocks still become durable ONLY via
> `pump_block`). The CE-A5 manifest (a real cardano-node `AddedToCurrentChain` an Ade-forged block) is the
> recover→follow→forge→serve→ADOPT path proven END-TO-END LIVE on the non-producing-relay C2-LOCAL venue — it is a
> non-promotable rehearsal venue that **flips no RO-LIVE rule on its own** (only a committed `ba02_evidence::correlate`
> manifest over the preprod tip advances `RO-LIVE-01`).
>

> ### PHASE4-N-AC cluster close (`1d54abb4`) — KES signing evolves the operator key to the current period (`DC-CRYPTO-10`)

>
> **This regeneration is a single-cluster cluster-close refresh, applied directly to the on-disk SEAMS.** The
> prior on-disk SEAMS was pinned at the PHASE4-N-AB close (`c6e7fafe` / **458** canonical types / **137** CI /
> **335** rules). It is brought current to HEAD `1d54abb4` (the *Close PHASE4-N-AC* commit — **458** canonical
> types / **138** CI / **336** rules). It splices the **single closed cluster PHASE4-N-AC** (the RED producer
> signing shell now EVOLVES the operator KES signing key FORWARD to the requested KES period BEFORE signing, via
> the EXISTING deterministic Sum6KES `kes_update` primitive; fail-closed backwards / beyond-lifetime). **N-AC is
> RED-only — ZERO BLUE diff** (458 canonical types unchanged; the two source files touched in the whole
> `c6e7fafe..1d54abb4` span are BOTH RED — `crates/ade_runtime/src/producer/producer_shell.rs` (the RED producer
> key-custody shell — NEW method `kes_sign_header_advancing`) + `crates/ade_node/src/produce_mode.rs` (the RED
> forge — its single real KES sign now calls the new method); `git diff c6e7fafe..1d54abb4` over the BLUE
> `core_paths` trees touches **no** file and adds **zero** `struct`/`enum` lines — the BLUE Sum6KES algorithm +
> KES verifier + forge eligibility + wire rules are reused UNCHANGED; `T-KEY-01` "no signing in BLUE" preserved).
> The seam-relevant change — recorded under §1 (the producer-forge pipeline's single real KES-sign step now
> evolves-then-signs), §2 (the KES signing-key custody domain's signing-operation row + rule), §3 (no new closed
> enum / no version-gated contract — the change REUSES the EXISTING closed `SigningError` variants
> `EvolutionBackwards` / `EvolutionExhausted` + the branded `KesSignature` / `UnsignedHeaderPreImage`), §4 (no
> frozen-contract / wire-format / hash-algorithm change), §5 (one NEW gate, 137 → 138, + a `strengthened_in` note
> on `CN-KES-HEADER-01`), §6 (the RED no-stale-period-sign / verbatim-period / fail-closed-both-directions
> prohibition + the per-cluster honest-scope bullet), and §7 (NO new candidate — a COMPLETION/STRENGTHENING of an
> already-closed RED signing seam; plus the genesis-window / KES-period coupling honest-scope finding) — is:
>
> - **NEW RED method `ProducerShell::kes_sign_header_advancing(&mut self, period: u32, preimage: &UnsignedHeaderPreImage)
>   -> Result<KesSignature, ShellSignError>`** (`ade_runtime::producer::producer_shell` (RED, `//! RED`);
>   **DC-CRYPTO-10**) — `self.kes_advance_to(period)?` THEN `self.kes_sign_header(period, preimage)`. `kes_advance_to
>   -> kes_update` is the EXISTING deterministic Sum6KES forward evolution: `period == current` → no-op evolution
>   (the `while current < to` loop does nothing) then sign (existing period-0 signing UNCHANGED); `period > current`
>   (in window) → forward evolution then sign; `period < current` → fail closed `Signing(EvolutionBackwards)` (a
>   destroyed past period cannot be re-signed — forward-secrecy); `period` beyond the key lifetime / unreachable by
>   sequential evolution → fail closed `Signing(EvolutionExhausted)` (`SUM6_MAX_PERIOD = 63`). The `period` is
>   passed **VERBATIM** (no `period ± N` mutation). After a successful `kes_advance_to`, `current == period`, so
>   `kes_sign_header` signs at the current period (no `KesPeriodNotCurrent`). The `&self` `kes_sign_at` /
>   `kes_sign_header` REMAIN for callers that manage periods themselves.
> - **The forge's SINGLE real KES sign now evolves-then-signs.** `ade_node::produce_mode::run_real_forge_inner`
>   step 3 calls `kes_sign_header_advancing` instead of the raw `kes_sign_header`; on `Err(_)` it still returns the
>   closed `CoordinatorEvent::ForgeFailed`. The forge's existing `kes_period_in_window` pre-check is preserved (it
>   also rejects `period > opcert_last`). So the forge now works across the opcert's KES window, not only the
>   minted-at-0 period (the gap the item-4 C1 re-run surfaced — every period-1 leader slot was failing
>   `KesPeriodNotCurrent { requested: 1, current: 0 }`).
> - **NO new closed enum, NO new canonical type, NO version-gated contract bump.** Evolution + signing REUSE the
>   EXISTING closed `SigningError` variants `EvolutionBackwards` / `EvolutionExhausted` and the branded
>   `KesSignature` / `UnsignedHeaderPreImage` types. The Sum6KES `kes_update` evolution is the EXISTING primitive
>   (reused, not re-implemented); the wire format / envelope / hash algorithms / canonical-type set are UNCHANGED.
>
> **Registry → 336 rules** (ONE NEW, `DC-CRYPTO-10`, `tier = derived`, `introduced_in = "PHASE4-N-AC"`, `enforced`,
> `ci_script = ci/ci_check_kes_evolution_before_sign.sh`, `cross_ref = [CN-KES-HEADER-01, T-KEY-01, DC-CRYPTO-04,
> DC-CRYPTO-09, CN-FORGE-03]`). **ONE strengthened** (`strengthened_in += "PHASE4-N-AC"`): `CN-KES-HEADER-01` (the
> KES-signs-the-real-unsigned-header-pre-image invariant now ALSO covers period evolution — the signed pre-image is
> at the EVOLVED current period, not a stale minted-at-0 period). **No rule weakened.** **NET +1 CI gate (137 ->
> 138): +1 NEW** `ci_check_kes_evolution_before_sign.sh` — pins that the forge real KES sign uses
> `kes_sign_header_advancing` (NOT the raw `kes_sign_header` / `kes_sign_at`), that `kes_sign_header_advancing`
> calls `kes_advance_to(period)` VERBATIM before signing (no `period ± N`), and that `kes_update` retains BOTH
> fail-closed guards (`EvolutionBackwards` + `EvolutionExhausted`); the standing `ci_check_no_signing_in_blue.sh`
> remains the BLUE `T-KEY-01` fence.
>
> **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** N-AC makes the RED signing shell EVOLVE
> the operator KES key to the requested period before signing — and **nothing more**. The Sum6KES evolution is the
> EXISTING deterministic `kes_update` primitive (reused, not re-implemented); signing stays **RED** (`T-KEY-01`
> preserved — no signing in BLUE); the BLUE Sum6KES algorithm + KES verifier + forge eligibility + wire rules are
> UNCHANGED. There is **NO new BLUE canonical type or authority** (`kes_sign_header_advancing` is a RED `&mut self`
> method; 458 unchanged); **NO new closed enum / version-gated contract** (the existing `SigningError` variants +
> branded `KesSignature` / `UnsignedHeaderPreImage` are reused); the period is passed **VERBATIM** (no manual
> `± N`); the evolution is **fail-closed both directions** (backwards / beyond-lifetime → structured error + NO
> signature, never a key override); and there is **NO RO-LIVE flip** — a forged + self-accepted period-1 block a
> real cardano-node DOWNLOADS with no KES/parse rejection is a wire-layer / header-acceptance event, NOT
> operator-witnessed bounty acceptance (`RO-LIVE-01` stays `partial` / operator-gated). **Genesis-window / KES-period
> coupling finding (recorded honestly, NOT a gap):** `slotsPerKESPeriod = 129600` equals `3k/f = 3·2160/0.05 =
> 129600`, so KES period 1 begins exactly when the Cardano genesis density window closes — a **from-genesis C1
> rehearsal can therefore never show forge-at-period-1 AND follower-adopt simultaneously** (period 0 = window open
> / no evolution exercised; period 1+ = evolution exercised but the follower rejects `CandidateTooSparse`, which is
> **genesis-density-window-limited, KES-INDEPENDENT**, NOT a KES rejection). **Cross-period end-to-end forge→adopt
> is proven on the dense C2 tip path** (the real bounty path — current preprod tip, no genesis window), NOT a net
> reset (which returns to period 0 and re-proves only the narrow period-0 case). It is the live-readiness fix
> surfaced by the pre-RO-LIVE hardening queue (item-4 finding → item-5 fix).
>
> ### PHASE4-N-AB cluster close (`c6e7fafe`) — outbound mux segmentation (`CN-SESS-05`)

>
> **This regeneration is a single-cluster cluster-close refresh, applied directly to the on-disk SEAMS.** The
> prior on-disk SEAMS was pinned at the PHASE4-N-AA close (`b0365df0` / **458** canonical types / **136** CI /
> **334** rules). It is brought current to HEAD `c6e7fafe` (the *Close PHASE4-N-AB* commit — **458** canonical
> types / **137** CI / **335** rules). It splices the **single closed cluster PHASE4-N-AB** (outbound mux
> segmentation — the GREEN `session::core` `handle_outbound` now SEGMENTS an outbound mini-protocol payload larger
> than the mux SDU limit `MAX_PAYLOAD` into ordered `<= MAX_PAYLOAD` frames, the outbound inverse of the
> `CN-SESS-04` inbound reassembly Ade already performs). **N-AB is GREEN-only — ZERO BLUE diff** (458 canonical
> types unchanged; the ONLY source file touched in the whole `b0365df0..c6e7fafe` span is the GREEN session reducer
> `ade_network::session::core` (`crates/ade_network/src/session/core.rs`); `mux::frame` and every other BLUE
> submodule are reused **unchanged**). It pairs with PHASE4-N-AA (item 1 bounded *how many* blocks a peer can pull
> per request; this makes *one large* block — a Conway block exceeding 64 KiB — actually transmittable). The
> seam-relevant additions — recorded under §1 (the outbound inverse of the §1 N2N inbound reassembly), §3 (no new
> closed enum — REUSES the closed `SessionError::OutboundPayloadTooLarge` + the `AcceptedMiniProtocol` registry +
> the single `encode_inner_frame` authority), §4 Frozen (the symmetric outbound fixed bound), §5 (one NEW gate,
> 136 → 137), §6 (the GREEN no-truncation / single-encoder / fixed-bound prohibition), and §7 (NO new candidate — a
> completion of an already-closed seam's outbound direction) — are:
>
> - **`handle_outbound` (in GREEN `ade_network::session::core`) now SEGMENTS an outbound mini-protocol payload**
>   (`CN-SESS-05`): a payload with `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` is split into ordered
>   `<= MAX_PAYLOAD` chunks (`payload.chunks(MAX_PAYLOAD)`), each encoded via the SAME single-frame
>   `encode_inner_frame` authority (which wraps `mux::frame::encode_frame` and keeps its own strict `MAX_PAYLOAD`
>   guard — never a second / parallel frame encoder), and emitted in order; every segment carries the SAME
>   mini-protocol id + mode + the SAME single captured `timestamp` input (GREEN — no per-segment clock read); an
>   empty payload still emits exactly one (empty) frame; `concat(segment payloads) == payload` exactly
>   (byte-preserving, lossless, ordered — DC-CONS-17 preserved). It is the **outbound inverse of the `CN-SESS-04`
>   inbound `drain_protocol_items` reassembly**: a conformant peer (and Ade's own inbound CN-SESS-04 path)
>   reconstructs the original payload.
> - **NEW fixed defensive GREEN constant `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB`** (`ade_network::session::core`,
>   `core.rs:60`; **CN-SESS-05**) — the outbound payload ceiling, **symmetric with the receive-side
>   `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` (DC-LIVEMEM-01)**. A payload above it fails closed
>   (`SessionError::OutboundPayloadTooLarge { len }`) **before** any segmentation. A **FIXED, closed,
>   non-configurable** bound — **NO CLI / env / config escape, no unbounded mode, no partial-success send** — that a
>   later hardening slice may **tighten** (a strengthening of `CN-SESS-05`) but may NEVER make tunable / unbounded.
>   A **GREEN constant, NOT a BLUE canonical type** (458 unchanged). Recorded in §4 Frozen alongside
>   `DC-LIVEMEM-01`'s `MAX_REASSEMBLY_TAIL_BYTES` / `MAX_WIRE_PUMP_LOOKAHEAD` and `DC-SERVEMEM-01`'s
>   `MAX_SERVE_RANGE_BLOCKS`.
> - **NO new closed enum, NO new canonical type, NO new authority.** Segmentation REUSES the existing closed
>   `SessionError::OutboundPayloadTooLarge` variant (`session::event`, `event.rs:177` — not added by N-AB), the
>   closed `AcceptedMiniProtocol` registry (mini-protocol id + mode unchanged), and the single `encode_inner_frame`
>   → `mux::frame::encode_frame` per-frame authority. `MAX_OUTBOUND_PAYLOAD_BYTES` is a GREEN closed constant.
>
> **Registry → 335 rules** (ONE NEW, `CN-SESS-05`, `tier = derived`, `introduced_in = "PHASE4-N-AB"`, `enforced`,
> `ci_script = ci/ci_check_outbound_segmentation.sh`, `cross_ref = [CN-SESS-04, DC-LIVEMEM-01, CN-WIRE-08,
> DC-SERVEMEM-01, DC-CONS-17]`). **TWO strengthened** (`strengthened_in += "PHASE4-N-AB"`): `CN-SESS-04` (the
> inbound per-mini-protocol reassembly invariant now has its symmetric outbound inverse — receive+send symmetry; the
> reassembly contract is a round-trip, segment-then-reassemble is byte-identical) and `DC-SERVEMEM-01` (the bounded
> serve can now actually transmit a large in-range block — the bounded serve range and the outbound segmentation
> compose). **No rule weakened.** **NET +1 CI gate (136 -> 137): +1 NEW** `ci_check_outbound_segmentation.sh`
> (fences the single `encode_inner_frame` authority, the `encode_inner_frame` `MAX_PAYLOAD` guard,
> `handle_outbound` segmentation, and the fixed non-configurable `MAX_OUTBOUND_PAYLOAD_BYTES` bound).
>
> **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** N-AB makes the outbound wire path SEGMENT
> large payloads into legal mux frames — and **nothing more**. It is the **GREEN, deterministic** inverse of the
> `CN-SESS-04` inbound reassembly; it transmits bytes the producer already produced, it does **NOT** decide which
> block is served, **NOT** advance a durable tip, and **NOT** make any peer-acceptance claim. There is **NO new BLUE
> canonical type** (`MAX_OUTBOUND_PAYLOAD_BYTES` is a GREEN const; 458 unchanged); **NO second frame encoder**
> (`encode_inner_frame` stays the single single-frame authority — `mux::frame` reused unchanged); **NO new closed
> enum** (`SessionError::OutboundPayloadTooLarge` reused); **NO truncation / drop / reorder** (lossless,
> byte-preserving, ordered); **NO partial-success send** (a message either fully segments+emits or fails closed);
> **NO per-segment clock** (one captured `timestamp` input reused — determinism preserved); the upper bound is a
> **fixed, non-configurable** ceiling that fails closed above it (`OutboundPayloadTooLarge`), never an unbounded
> outbound buffer; and there is **NO RO-LIVE flip** — segmenting bytes != operator-witnessed peer acceptance
> (`RO-LIVE-01` stays `partial` / operator-gated; confirming a real cardano-node demux accepts the reused-per-segment
> SDU timestamp is the live leg, fenced behind the unflipped RO-LIVE obligations). It is pre-RO-LIVE hardening item 2.
>
> ### PHASE4-N-AA cluster close (`5c9f6cf6` / close `b0365df0`) — bounded peer-driven serve range (`DC-SERVEMEM-01`)

>
> **This regeneration is a single-cluster cluster-close refresh, applied directly to the on-disk SEAMS.** The
> prior on-disk SEAMS was pinned at the post-PHASE4-N-U gate-hygiene span (`999199f8` / **458** canonical types /
> **135** CI / **333** rules). It is brought current to the CODEMAP regenerated at the N-AA close (`5c9f6cf6` /
> **458** canonical types / **136** CI / **334** rules; `b0365df0` is the close commit one past it). It splices
> the **single closed cluster PHASE4-N-AA** (bounded peer-driven serve range — the `--mode node` serve path bounds
> per-request work and fails closed on oversized / inverted / undecodable ranges). **N-AA is RED-only — ZERO BLUE
> diff** (458 canonical types unchanged; no BLUE struct/enum/fn added — the lone BLUE-tree touch in the close span
> is a `// Core Contract:` comment header on `ade_ledger::block_validity::mod.rs`, no type/authority/export change).
> It is the **serve-side analog of `DC-LIVEMEM-01`** (the receive-side bounded-memory cap, N-F-G-E) and closes the
> **PHASE4-N-U cross-slice security-review MEDIUM finding** (peer-driven serve resource amplification). The
> seam-relevant additions — all classified under §3 Closed (closed RED enum / struct / fixed defensive constant),
> with §3 Extensible (the durable-chain serve impl now BOUNDED), §4 Frozen (the symmetric serve-side fixed cap),
> §5 (one NEW gate, 135 → 136), §6 (the RED no-unbounded-serve prohibition), and §7 (NO new candidate — a bounding
> of an already-closed seam) updated — are:
>
> - **NEW closed RED enum `ServeRangeOutcome { Served(Vec<(SlotNo, Hash32, Vec<u8>)>) | Empty | CapExceeded | ReadError }`**
>   (`ade_runtime::network::served_chain_projection` (RED, `//! RED`); **DC-SERVEMEM-01**) — the four internal
>   reasons the bounded BlockFetch serve distinguishes; **every non-`Served` variant maps to an empty `Vec` → the
>   BLUE reducer's wire `NoBlocks`** (`CapExceeded` on an oversized range, decided BY THE BOUND before any decode;
>   `Empty` on an out-of-chain or inverted window; `ReadError` on an in-range block the single BLUE `decode_block`
>   cannot authenticate — the serve never emits a block it cannot authenticate). **NOT `#[non_exhaustive]`; no
>   catch-all.** A RED serve-read outcome, **NOT** a dispatch (the single `dispatch_server_frame_event_to_outbound`
>   is unchanged — DC-NODE-07 preserved), **NOT** canonical-counted. A closed fail-closed reason vocabulary (a
>   surface REDUCTION), NOT an extension point. Classified under §3 Closed.
> - **NEW closed RED value struct `CappedSlotRange { blocks: Vec<(SlotNo, Vec<u8>)>, truncated: bool }`**
>   (`ade_runtime::chaindb::types` (RED); **DC-SERVEMEM-01**) — the result of the bounded hash-free read primitive
>   `range_bytes_capped(from, to, max)`: `blocks` holds at most `max` `(slot, bytes)` pairs slot-ascending,
>   `truncated = true` when the requested range held MORE than `max` (the per-request cap was exceeded). A RED
>   storage value type, **NOT** canonical-counted. A closed value type (a surface REDUCTION), NOT an extension point.
> - **NEW fixed defensive constant `MAX_SERVE_RANGE_BLOCKS = 256`** (`ade_runtime::network::served_chain_projection`;
>   **DC-SERVEMEM-01**) — the per-request serve-range cap, **symmetric with the receive-side
>   `MAX_WIRE_PUMP_LOOKAHEAD = 256` (DC-LIVEMEM-01)**. A **FIXED, closed, non-configurable** bound — **NO CLI / env
>   / config escape, no unbounded mode** — that a later hardening slice may **tighten** (a strengthening of
>   `DC-SERVEMEM-01`) but may NEVER make tunable / unbounded. A defensive **implementation** bound, NOT a Cardano
>   semantic parameter (it does not change which chain is served, only the per-request work ceiling). Recorded in §4
>   Frozen alongside `DC-LIVEMEM-01`'s `MAX_WIRE_PUMP_LOOKAHEAD` / `MAX_REASSEMBLY_TAIL_BYTES`.
> - **BOUNDS the existing closed serve seam — `ChainDbServedSource` reads via the bounded hash-free ChainDb
>   primitives `range_bytes_capped` + `last_block_bytes`** (NEW `ChainDb` trait methods; **DC-SERVEMEM-01**) instead
>   of the unbounded `iter_from_slot` / O(N) `chaindb.tip()` (which materialized the full `from..tip` range and
>   recovered each hash via a per-block `SLOT_BY_HASH` scan — O(N²)). The block hash is derived from each block's
>   own bytes via the single BLUE `decode_block` authority (NO second hash authority, NO `SLOT_BY_HASH` reference on
>   the serve path). The trusted recovery / rollback `iter_from_slot` / `tip` internals are **doc-fenced (not
>   peer-driven), unchanged**. **This is a bounding of the DC-NODE-13 durable-chain serve seam — it opens NO new
>   seam.** A security-review MEDIUM (`5c9f6cf6`) added an inverted-range (`from > to`, incl. `from = u64::MAX`)
>   guard on both `ChainDb` impls so a peer controlling both endpoints gets an empty result, never a panic.
>
> **Registry → 334 rules** (ONE NEW, `DC-SERVEMEM-01`, `tier = derived`, `introduced_in = "PHASE4-N-AA"`,
> `enforced`, `ci_script = ci/ci_check_serve_range_bounded.sh`, `cross_ref = [DC-LIVEMEM-01, DC-NODE-13, DC-NODE-07,
> CN-CONS-07, DC-CONS-17]`). **TWO strengthened** (`strengthened_in += "PHASE4-N-AA"`): `DC-NODE-13` (the `--mode
> node` durable-chain serve projection is now BOUNDED) and `DC-LIVEMEM-01` (extended from the receive-side to a
> symmetric serve-side bound — a cross-ref, not a code change to G-E). **No rule weakened.** **NET +1 CI gate (135 →
> 136): +1 NEW** `ci_check_serve_range_bounded.sh` (fences the serve to the bounded hash-free primitives — no
> `iter_from_slot` / `chaindb.tip()` — the fixed non-configurable cap, and `decode_block`-only hash derivation).
>
> **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** N-AA is a **defensive resource bound on the
> read-only serve path** — and **nothing more**. The cap is an **implementation bound, NOT a Cardano semantic
> parameter** and **cannot be disabled at runtime**. There is **NO new canonical / BLUE type** (`CappedSlotRange` +
> `ServeRangeOutcome` are RED storage/serve value types, NOT canonical-counted); **NO second hash authority** (the
> serve derives the hash via the single BLUE `decode_block`; the bounded primitive is hash-FREE); **NO new dispatch
> authority** (the single `dispatch_server_frame_event_to_outbound` is unchanged — DC-NODE-07 preserved); the
> trusted recovery / rollback `iter_from_slot` internals are **OUT OF SCOPE** (doc-fenced, unchanged); and there is
> **NO RO-LIVE flip** — bounding serve work ≠ operator-witnessed peer acceptance (`RO-LIVE-01` stays `partial` /
> operator-gated). It is pre-RO-LIVE hardening item 1.
>
> ### Post-N-U gate-hygiene span (`4e358e92` → `999199f8`, 2026-06-05) — DC-NODE-06 handoff-fence REPOINT + green-means-green CI sweep

>
> **This regeneration is a FOCUSED single-span refresh** (applied directly to the on-disk SEAMS, no CODEMAP
> regenerate). It brings SEAMS current from its last pin (`4e358e92`, the PHASE4-N-U close-doc refresh) to HEAD
> (`999199f8`), splicing the post-N-U-close gate-hygiene span. **CODEMAP was intentionally NOT regenerated** — it
> is structurally current (unchanged module inventory; **458** canonical types / **135** CI / **333** rules) and
> stays pinned at `4e358e92`; this refresh reads it as-is. **Counts are UNCHANGED across the span** (458 / 135 /
> 333): **NO new BLUE type, NO new surface, NO new version-gated contract, NO net CI gate** (the 11 gates the span
> touches are all MODIFIED in place — count stays 135). The two source-file touches
> (`ade_ledger/src/block_validity/mod.rs`, `ade_runtime/src/seed_import/importer.rs`) are pure **doc-comment**
> hygiene (stale comments reconciled to the code a gate greps); **no semantic code change, no invariant weakened.**
>
> **The ONE material SEAMS change — the DC-NODE-06 handoff-fence REPOINT (`60deecf3`).** The gate
> `ci/ci_check_served_chain_handoff_fence.sh` was **repointed** (not retired — the CI count stays **135**, avoiding
> churn in the just-refreshed grounding docs). It **NO LONGER** fences the PHASE4-N-F-G-B *“`SelfAcceptedHandoff`
> → `ServedChainHandle::push_atomic` accumulator”* handoff on the `--mode node` spine: **N-U S3 RETIRED that
> mechanism** in favor of serve-as-projection (DC-NODE-13), and the N-U close STRANDED the gate (its “every
> node-spine `push_atomic(` fed by `into_accepted()`” check inverted — N-U removed the very `push_atomic` it
> required; the set-based close-pass gate-diff masked this). It now fences the **EVOLVED DC-NODE-06
> durable-provenance serve**: the `--mode node` spine serve sources **ONLY** `ServedChainSource::DurableChainDb`
> (the durable ChainDb read-only projection, `ade_runtime::network::served_chain_projection::ChainDbServedSource`)
> over `Arc<dyn ChainDb>`, with **NO retired non-durable serve ingress** — no `push_atomic(` / `served_chain_admit(`
> / `ServedChainHandle` / `SelfAcceptedHandoff` handoff channel on the node spine. DC-NODE-06's **DEEPER**
> invariant (*only validated/admitted bytes may be served on the node spine*) is **PRESERVED and STRENGTHENED**
> (durable-provenance per the CN-CONS-07 restatement; it now SURVIVES RESTART). Owners stay
> `{node_lifecycle.rs, node_sync.rs}`; complementary to `ci_check_served_chain_projection.sh` (the DC-NODE-13
> projection-SHAPE gate) — this gate is the serve-PROVENANCE angle. The registry (`DC-NODE-06`) was already
> reconciled at `60deecf3` (`ci_script += ci_check_served_chain_projection.sh`; `code_locus` repointed to the
> durable-projection serve source; `strengthened_in += PHASE4-N-U`; rule count UNCHANGED at 333).
>
> **The G-B handoff MECHANISM is now SUPERSEDED** (by N-U S3 / DC-NODE-13) **wherever it described the `--mode
> node` serve path** — §1 *self-accept→serve handoff* surface, §2 *self-accept→serve handoff* domain, and the §3
> `SelfAcceptedHandoff` registry entry are annotated below. **The `SelfAcceptedHandoff` carrier itself is
> RETAINED** — `--mode produce` (CN-PROD-04) is a SEPARATE serve authority that legitimately keeps the carrier +
> `ServedChainHandle` (`ServedChainSource::Snapshot`). **Historical / carried narrative references to this gate in
> past-cluster (G-B / G-C / G-D / G-E / G-H) delta sections below are pin-correct for their spans and are LEFT
> as-is** — only the CURRENT-state descriptions are updated.
>
> **Green-means-green.** The other 10 gates the span touches (`999199f8` repaired 10 pre-existing gate-vs-code
> drifts; `e92b40b7` made `ci_check_no_secrets.sh` actually run past `ARG_MAX` + tuned IP false positives) are
> drift/hygiene fixes only — **no invariant weakened**. The full CI sweep is now **135 / 0 GREEN**
> (`ls ci/ci_check_*.sh | wc -l` = 135, every gate passes).
>
> ### PHASE4-N-U cluster close (`4e358e92`) — forged-block durability: own-tip durable admit + warm-start recovery + serve-as-projection of the durable ChainDb

>
> **This regeneration is a single-cluster cluster-close refresh.** The prior on-disk SEAMS was pinned at the
> PHASE4-N-F-G-K…G-R catch-up (`65954fa3` / **458** canonical types / **134** CI / **328** rules). It is brought
> current to the CODEMAP regenerated at the same HEAD (`4e358e92` / **458** canonical types / **135** CI / **333**
> rules), splicing the **single closed cluster PHASE4-N-U** (forged-block durability — full producer own-tip
> advance). **The net BLUE delta is ZERO new canonical types and ZERO new BLUE authorities** — the lone BLUE touch
> is the additive `ade_ledger::block_validity::header_input::block_header_bytes(block_cbor: &[u8]) -> Result<&[u8],
> BlockValidityError>`, factored out of the existing `accepted_block_header_bytes` (which now delegates
> `block_header_bytes(accepted.as_bytes())`) — the SAME single DC-CONS-18 header-projection authority, generalized
> from `&AcceptedBlock` to `&[u8]` so the durable-chain serve projection can read `StoredBlock.bytes` directly; no
> new logic, no new type, no parallel splitter. Everything else is **RED wiring** on the `--mode node` spine. The
> seam-relevant additions — classified under §3 Closed / §3 Extensible (a SECOND impl of an existing closed trait
> seam) / §7 (one declared seam CLOSED, one prior seam RETIRED) — are:
>
> - **NEW CLOSED read-source selector `ServedChainSource { Snapshot | DurableChainDb }`** (`ade_runtime::network::serve_dispatch`
>   (RED); **DC-NODE-07 PRESERVED**) — a closed 2-variant enum read by the SINGLE serve-dispatch authority
>   `dispatch_server_frame_event_to_outbound`. `Snapshot(&ServedChainView)` is the `--mode produce` in-memory
>   accumulator path; `DurableChainDb(&dyn ChainDb)` is the `--mode node` durable-chain projection (NEW in N-U).
>   **ONE dispatch, two read sources** — the serve authority does not fork; the source enum selects which read-side
>   the single dispatch reads from. **A closed read-source selector (a surface REDUCTION), NOT an extension point.**
>   Classified under §3 Closed; no new serve authority, no second dispatch (`ci_check_served_chain_projection.sh`).
> - **NEW SECOND IMPL of the closed serve seams — `ChainDbServedSource`** (`ade_runtime::network::served_chain_projection`
>   (RED, `//! RED`); **DC-NODE-13**) — a NEW RED module implementing the EXISTING BLUE serve seams
>   `ServedHeaderLookup` (`ade_network::chain_sync::server`) / `ServedRangeLookup` (`ade_network::block_fetch::server`)
>   over the durable ChainDb (`iter_from_slot` / `get_block_by_hash` / `tip`). It is a **second impl of those trait
>   seams ALONGSIDE the produce-mode `ServedChainLookups`** (the snapshot impl, `ade_runtime::producer::served_chain_lookups`).
>   These two trait seams remain **THE closed read-side extension surface for the serve reducers** — a new served
>   read-side attaches by implementing them (see §3 Extensible). `ChainDbServedSource` serves `stored.bytes`
>   VERBATIM (no re-encode — DC-CONS-17), reuses the single `block_header_bytes` header-projection authority
>   (DC-CONS-18) + `decode_block` (NO parallel splitter, NO `AcceptedBlock` reconstruction). Provenance is
>   structural (CN-CONS-07 serve clause): every byte is `StoredBlock.bytes` from the durable ChainDb whose sole
>   production writers are `pump_block` (DC-NODE-12) + the validated `bootstrap_initial_state` — serving cannot
>   leak a byte that did not clear `block_validity`.
> - **REUSES the single durable tip authority — `admit_forged_block_durably` → `pump_block`** (`ade_node::node_sync`
>   (RED); **DC-NODE-12 / DC-CONS-23 / DC-WAL-04**) — the forge's own self-accepted block (`accepted.into_bytes()`,
>   byte-identical to the self-accept input — no re-encode, I-10) is fed into the SAME `forward_sync::pump_block`
>   chokepoint received blocks use (extend-only `admit_via_block_validity` → StoreBlockBytes → AppendWal →
>   AdvanceTip, durable-before-tip). The forge gains **NO** direct tip-advance path (`pump_block` stays the sole
>   durable tip authority), **NO** `NodeBlockSource` variant, **NO** new `WalEntry` variant (it reuses
>   `WalEntry::AdmitBlock`); a stale-tip forge fails closed via header-position / `prior_fp`, never an own-block
>   override. **NOT a new chokepoint** — a REUSE of the closed `pump_block` durable tip authority.
> - **WarmStart recovery extended — forged-tip recovery + no-orphan drop** (`ade_node::{node_sync, node_lifecycle}`
>   (RED); **T-REC-05 / DC-WAL-04**) — production `warm_start_recovery` gains WAL-tail reconciliation +
>   forward-replay-from-sub-tip-snapshot (reusing the BLUE `bootstrap_initial_state` forward-replay authority), so
>   a forged-block durable tip (which captures no snapshot-at-tip — rides DC-STORE-07) recovers byte-identically;
>   a torn forge-admit crash drops the un-WAL'd orphan; a recovered fp ≠ WAL-tail `post_fp` is a deterministic
>   fail-fast (`WarmStartWalTailFingerprintMismatch`). T-REC-05 is **test-enforced** (the kill-recover
>   fingerprint-equality tests `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay` +
>   `warm_start_drops_orphan_block_above_wal_tail`; `T-REC-05.ci_script = ""`).
>
> **Registry → 333 rules** (FIVE NEW, all `introduced_in = "PHASE4-N-U"`, all `enforced`: `DC-NODE-12` (own-forged
> durable admit via the single `pump_block` chokepoint), `DC-CONS-23` (extend-only durable admit / stale-tip
> fail-closed), `DC-WAL-04` (forged WAL `prior_fp` chaining + no-orphan recovery), `T-REC-05` (**tier = true**,
> forged-tip recovery byte-identity + replay-equivalence — test-enforced, `ci_script = ""`), `DC-NODE-13`
> (serve-as-projection of the durable chain). TWO strengthened: `CN-CONS-07` (serve-provenance clause →
> durable-provenance), `DC-NODE-11` (monotone-serve-gate MECHANISM superseded by serve-as-projection; the
> invariant *a follower sees a stable, coherent served block 0* is preserved + strengthened — it now survives
> restart). **No rule weakened.** **NET +1 CI gate (134 → 135): +2 NEW** (`ci_check_forged_durable_admit_via_pump.sh`
> S1 / DC-NODE-12 + DC-CONS-23 + DC-WAL-04; `ci_check_served_chain_projection.sh` S3 / DC-NODE-13), **−1 RETIRED**
> (`ci_check_served_chain_stability.sh`, the G-R DC-NODE-11 monotone-serve-gate mechanism superseded). The S1 also
> EXTENDED `ci_check_node_run_loop_containment.sh` in place (allow-list for the one `admit_forged_block_durably`
> driver call; no count change).
>
> **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** N-U makes own-forged blocks **durable**,
> **recoverable**, and **coherently servable** — and **nothing more**. There is **NO second durable tip-advance
> path** (the forged block goes THROUGH `pump_block`; no forge-specific `put_block` / `AdvanceTip` / `rollback`);
> **NO admit-time fork-choice** (the durable admit is extend-only — `select_best_chain` / `fork_choice` are NOT on
> this path; DC-CONS-03 stays the follow/`chain_selector` authority, cross-ref only); **NO re-encode**
> (`accepted.into_bytes()` verbatim); **NO `AcceptedBlock` reconstruction** in the serve projection (it reads raw
> `stored.bytes` + `block_header_bytes(&[u8])`); **NO re-validation shortcut** (provenance is structural: durable
> ChainDb ⇐ `pump_block` ⇐ `block_validity`); **NO new BLUE authority or canonical type**; and **NO RO-LIVE flip**
> — durability + coherent serve ≠ operator-witnessed peer acceptance (`RO-LIVE-01` stays `partial` /
> operator-gated). **Honest drift (recorded at close, NOT a gap):** the S2 §8-named gate
> `ci_check_forged_tip_recovery.sh` and the §8-named test `forge_two_clean_runs_byte_identical` were **not created
> literally** — S2 enforced T-REC-05 + DC-WAL-04(no-orphan) via the kill-recover fingerprint-equality tests named
> above (in `cargo test -p ade_node`), and the registry records `T-REC-05.ci_script = ""` (test-enforced). The
> invariants are enforced; the §8 CE artifact names drifted during S2.
>
> ### PHASE4-N-F-G-K … G-R + C1 evidence (catch-up, `65954fa3`) — serve-side real-node compat + feed decode/view fidelity + forge-successor + monotone serve gate + recovered eta0
>
> **This regeneration is a multi-cluster CATCH-UP refresh.** The prior on-disk SEAMS was pinned at the
> PHASE4-N-F-G-J close (`550eec3a` / **457** canonical types / **126** CI / **319** rules). It is brought current
> to the CODEMAP regenerated at the same HEAD (`65954fa3` / **458** canonical types / **134** CI / **328** rules),
> splicing the **eight closed clusters G-K → G-R + the C1 genesis-successor rehearsal evidence**. **The WHOLE
> span adds ONLY closed sums + version-gated fields — NO new extensible / negotiated / plugin / runtime-registered
> surface (verified two ways: (a) `git diff --name-only 550eec3a..65954fa3` touches exactly the BLUE `core_paths`
> files `ade_network/src/{handshake/version_table.rs, chain_sync/server.rs, codec/{chain_sync,primitives}.rs}` +
> `ade_ledger/src/{seed_consensus_inputs.rs, consensus_view.rs}`, the rest RED/GREEN/test; (b) the net BLUE delta
> is ONE canonical type (`ArrayHead`, G-M), two additive BLUE fns (`encode_n2n_version_params` G-L,
> `decode_array_head_two_form` G-M), and one additive BLUE record-field + version-gate bump
> (`SeedEpochConsensusInputs.epoch_nonce` + `SEED_CINPUT_SCHEMA_VERSION 1 → 2`, G-N)).** The eight headline rules
> (G-N ships two) are all `enforced`; registry **319 → 328** (+9); CI **126 → 134** (+8, one gate per cluster).
> The seam-relevant additions — each classified under §3 Closed / §4 Frozen / §4 Version-gated, **none
> extensible** — are:
>
> - **NEW CLOSED BLUE enum `ArrayHead = Definite(u64) | Indefinite`** (`ade_network::codec::primitives` (BLUE);
>   the `codec/` BLUE submodule count **38 → 39**, total **457 → 458**; **CN-WIRE-11 / G-M**) — the two-form CBOR
>   array-head grammar for real cardano-node ChainSync `FindIntersect`. A real cardano-node sends the points list
>   as a CBOR **INDEFINITE-length** array (`0x9f … 0xff`); the closed BLUE fn `decode_array_head_two_form` accepts
>   BOTH the definite (`9f…`) and indefinite forms, and on `Indefinite` the caller consumes the matching `0xff`
>   break. **SCOPED to ChainSync `FindIntersect`** — it is NOT a general definite/indefinite decoder for the rest
>   of the closed wire grammar (the rest stays definite-only); **no catch-all**. Paired with the
>   `chain_sync::server` `Origin → IntersectFound[Origin]` reply (the universal common ancestor; matches the real
>   node, does NOT widen the served chain). Pinned against captured cardano-node 11.0.1 FindIntersect/IntersectFound
>   fixtures. A surface REDUCTION for real-node compat, **NOT an extension point.** Classified under §3 Closed /
>   §4 Frozen; fenced by `ci_check_chainsync_findintersect_compat.sh`.
> - **VERSION-GATED field `SeedEpochConsensusInputs.epoch_nonce: Nonce`** behind `SEED_CINPUT_SCHEMA_VERSION 1 → 2`
>   (`ade_ledger::seed_consensus_inputs` (BLUE); **T-REC-04 + DC-CINPUT-03 / G-N**) — persists the recovered Praos
>   eta0 in the seed-epoch sidecar so the WarmStart forge VRF input is `praos_vrf_input(slot, epoch_nonce)` (a
>   recovered follower no longer fails `VRFKeyBadProof`). **A versioned-GATE evolution, NOT a new open surface:**
>   the field is additive ONLY behind the schema bump; `SeedEpochConsensusInputs` stays a single closed canonical
>   type with the SOLE codec (no `Default`, no `#[non_exhaustive]`, `BTreeMap`-ordered, byte-canonical); a **v1
>   sidecar that omitted `epoch_nonce` decodes as `UnknownVersion`, NEVER a default-to-zero eta0** (verified:
>   `seed_consensus_inputs.rs` `SEED_CINPUT_SCHEMA_VERSION = 2`, the decoder rejects `version != 2` as
>   `UnknownVersion`, and a v1-shape test pins the fail). Classified under §4 Version-gated; fenced by
>   `ci_check_warmstart_eta0_overlay.sh`. **NO new canonical TYPE** (458 unchanged — a field on an existing closed
>   record).
> - **FROZEN per-version N2N `versionData` wire encoding `encode_n2n_version_params(version, network_magic)`**
>   (`ade_network::handshake::version_table` (BLUE); an additive BLUE **fn**, no struct/enum — 458 unchanged;
>   **CN-WIRE-10 / G-L**) — the serve-side N2N handshake was emitting a `versionData` a real cardano-node rejected
>   (`HandshakeDecodeError NodeToNodeV_15 'unknown encoding: TInt 1'`). The single per-version `versionData`
>   encoder emits, for V11..=V15, the 4-field `NodeToNodeVersionData` `[networkMagic,
>   initiatorAndResponderDiffusionMode, peerSharing, query]` (and the extended V16+ shape) — so a real
>   cardano-node accepts the serve-side handshake at NodeToNodeV_15. **The N2N version table is a CLOSED / FROZEN
>   contract:** this is the single per-version encoder over the **unchanged closed `N2N_SUPPORTED` version SET** —
>   **NO version-set widening, NO runtime negotiation of meaning** (a successful handshake is a wire-layer event,
>   not peer acceptance). Classified under §4 Frozen (real-node-compat strengthening of the closed version table);
>   fenced by `ci_check_n2n_handshake_versiondata_authority.sh`.
> - **REUSES the FROZEN tag-24 authority — feed-side BlockFetch tag-24 unwrap** (`ade_runtime::admission::wire_pump`
>   (RED); **CN-WIRE-12 / G-O**) — the live admission feed was handing a peer's BlockFetch `MsgBlock` payload to
>   the BLUE decoder WITHOUT first stripping its tag-24 (`0xd8 0x18`) CBOR-in-CBOR envelope (`UnexpectedType`). G-O
>   makes the feed-side WirePump strip it via the **single `ade_codec::unwrap_tag24` authority BEFORE the BLUE
>   decode** (mirroring the already-correct `admission::runner` + `ade_core_interop::follow` paths). **There is NO
>   second `unwrap_tag24` / parallel tag-24 parser** — the single `CN-WIRE-08` (N-X) authority is reused. NOT a new
>   serializer / parallel envelope / extension point; a decode-boundary fix. Classified under §4 Frozen (reuses
>   the frozen tag-24 authority); fenced by `ci_check_feed_tag24_unwrap.sh`.
> - **RED `ade_node` orchestration deltas — no new attach point** (`G-K` `DC-NODE-09`, `G-P` `DC-CINPUT-04`, `G-Q`
>   `DC-NODE-10`, `G-R` `DC-NODE-11`; all RED, NO BLUE change, 458 unchanged): **G-K** decouples the `--mode node`
>   serve-listener lifetime from feed end (the serve sibling, spawned OUTSIDE `run_relay_loop`, stays up serving
>   past feed-end so a follower keeps fetching) — `ci_check_node_serve_lifetime.sh`; **G-P** sources the feed-side
>   header-validation view (the leader-threshold stake distribution for Steps 5 + 7) from the recovered consensus
>   surface (`PoolDistrView::from_seed_epoch_consensus_inputs`) so a valid live block 0 is ingested
>   (`VerificationFailed` gone) — `ci_check_feed_leader_threshold_view.sh`; **G-Q** derives the forge-SUCCESSOR
>   position `(last_admitted_block_no + 1, PrevHash::Block(last_admitted_hash))` from the evolved admitted spine
>   state (not the stale recovered tip) — the first stable node, block 1+, reusing the G-J `forge_header_position`
>   convention, no new variant / `NodeBlockSource` — `ci_check_forge_successor_evolved_spine.sh`; **G-R** adds a
>   **monotone serve gate** (`node_lifecycle::serve_gate_admits(highest_served_block_no, candidate_block_no)`) so
>   the served tip never regresses once block 0 is served (a candidate that would move the served chain backward,
>   or re-serve an already-served height with a divergent hash, is refused) — a genesis-successor follower adopts
>   a **stable served block 0** — `ci_check_served_chain_stability.sh`. Each is **RED orchestration wiring inside
>   `node_lifecycle` / `node_sync`**, threading evolved-spine / recovered-view / serve-gate state through existing
>   signatures — **NO new closed enum / `NodeBlockSource` / `CoordinatorEvent` / `Mode` variant, NO new `--mode
>   node` flag, NO new BLUE authority, NO plugin/negotiated surface.** `run_relay_loop`'s containment is
>   byte-unchanged; the serve sibling stays read-only over the `ServedChainView`. Classified under §3 Closed
>   (closed RED vocabulary / fences); each backs a `derived` `enforced` rule.
> - **C1 genesis-successor rehearsal evidence (carried — `129d25ac`, `65954fa3`):** docs/evidence + CI only, **NO
>   crate changed**. A real Haskell follower on the C1 private testnet adopts Ade's stable served block 0,
>   captured at `docs/evidence/c1-genesis-rehearsal-{follower.log,peer-accept.jsonl,manifest.toml}` + the run-2
>   reproduction + the reproduction runbook; `65954fa3` broadens the rehearsal-manifest schema gate
>   (`ci_check_rehearsal_manifest_schema.sh`) IN PLACE to ALSO cover the c1 rehearsal manifests (a gate-scope
>   broadening, NOT a new gate; CI count unchanged at 134). It is fenced by the non-promotable
>   `PrivateRehearsalManifest` envelope (`is_rehearsal` / `not_bounty_evidence` literals, distinct rehearsal home).
>
> **Registry → 328 rules** (NEW `DC-NODE-09` (G-K), `CN-WIRE-10` (G-L), `CN-WIRE-11` (G-M), `T-REC-04` (G-N,
> **tier = true**) + `DC-CINPUT-03` (G-N), `CN-WIRE-12` (G-O), `DC-CINPUT-04` (G-P), `DC-NODE-10` (G-Q),
> `DC-NODE-11` (G-R) — all `enforced`; no rule weakened). **126 → 134 CI** (one new gate per cluster:
> `ci_check_node_serve_lifetime.sh` + `ci_check_n2n_handshake_versiondata_authority.sh` +
> `ci_check_chainsync_findintersect_compat.sh` + `ci_check_warmstart_eta0_overlay.sh` + `ci_check_feed_tag24_unwrap.sh`
> + `ci_check_feed_leader_threshold_view.sh` + `ci_check_forge_successor_evolved_spine.sh` +
> `ci_check_served_chain_stability.sh`; the C1 evidence commit broadened `ci_check_rehearsal_manifest_schema.sh` in
> place — no count change).
>
> **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** The whole G-K…G-R span is **serve-side
> real-node compat + feed decode/view fidelity + forge-successor continuity + a monotone serve gate + an explicit
> recovered eta0** — and **nothing more**. The `ArrayHead` indefinite-length acceptance is **SCOPED to ChainSync
> `FindIntersect`** (NOT a general definite/indefinite decoder); the `Origin → IntersectFound[Origin]` reply
> matches the real node and **does NOT widen the served chain** beyond the Origin intersect. The `versionData`
> encoder is over the **unchanged closed `N2N_SUPPORTED` version SET** (no widening, no runtime negotiation); a
> successful handshake is a wire-layer event, **NOT peer acceptance**. The feed tag-24 unwrap **reuses the single
> `CN-WIRE-08` authority** (no parallel parser). The `epoch_nonce` schema bump is **version-GATED** (a v1 sidecar
> fails closed as `UnknownVersion`, never silently zero-eta0); the WarmStart overlay supplies the recovered eta0
> to the forge VRF input but **does NOT make the recovered sidecar a runtime authority for nonce evolution** (the
> BLUE `consensus::nonce` transitions stay undriven on the forge path, DC-EPOCH-03). The forge-successor position
> is faithful to the admitted spine but the forge still holds **no `ChainDb` handle** — it advances **no durable
> tip** (the "first stable node, block 1+" claim is in-memory spine continuity past block 0, **NOT** durable
> block-1+ persistence — that is N-U). The monotone serve gate makes the served tip monotone so a follower adopts
> a stable block 0 — it is a **served-chain STABILITY** fix on the read-only serve path, **NOT** a durable-tip
> advance and **NOT** a peer-acceptance claim. **NO RO-LIVE flip** across the entire span — `RO-LIVE-01` stays
> `partial` / operator-gated; the C1 evidence is a **rehearsal REPRODUCTION**, NOT a bounty-completion claim
> (private C1 acceptance ≠ bounty completion; preview/preprod acceptance = the single bounty deliverable). **NO
> new BLUE authority beyond the one closed `ArrayHead` enum + the two additive BLUE fns + the one version-gated
> field; NO new extensible / negotiated / plugin surface; NO new `--mode node` flag / `NodeBlockSource` /
> `CoordinatorEvent` / `Mode` variant.** `run_relay_loop`'s containment is byte-unchanged.
>
> > **NOTE — the G-H serve-to-peer §1/§3/§5 bodies are still carried-only (the G-K…G-R serve deltas are spliced).**
> > The intervening **PHASE4-N-F-G-H** cluster (the NEW RED `ade_runtime::network::serve_dispatch`,
> > `ade_node::node_lifecycle::{run_node_serve_task, bind_serve_listener, ServeStartError}`, the additive BLUE fn
> > `n2n_supported_for_magic`, `DC-NODE-07`, the gates `ci_check_single_serve_dispatch_authority.sh` +
> > `ci_check_serve_listener_magic_aware.sh`) is documented in the CODEMAP carried delta but its serve-to-peer §1
> > surface + §3 `ServeStartError` row + §5 serve-gate rows are **NOT** spliced into the §1–§7 bodies below — the
> > G-K…G-R serve-side deltas (G-K lifetime decouple, G-L `versionData`, G-M FindIntersect/Origin, G-R monotone
> > serve gate) EXTEND that G-H serve sibling, and are folded into §3/§4/§5/§6 + the per-cluster note above. The
> > header counts (458 / 134 / 328) are the true HEAD values and DO include G-H's contributions; a fuller G-H §1
> > serve-to-peer surface catch-up is the recommended follow-on. **Treat the §1–§7 bodies as current through
> > G-R for everything EXCEPT the G-H §1 serve-to-peer surface block.**
>
> ### PHASE4-N-F-G-J (closed, `550eec3a`) — PrevHash null/hash32 wire authority + position-aware header rule + genesis-successor cold-start forge on the `--mode node` spine
>
> **This regeneration is a scoped DELTA-REFRESH at the PHASE4-N-F-G-J close.** It brings the counts and the
> closed-surface tables current with the CODEMAP regenerated at the same HEAD (`550eec3a` / **457** canonical
> types / **126** CI checks / **319** rules), and splices the N-F-G-J closed-surface deltas. **G-J adds only
> CLOSED sums + ONE NEW BLUE module — NO extensible / negotiated surface (verified).** The seam-relevant
> additions are:
>
> - **NEW CLOSED BLUE sum `PrevHash = Genesis | Block(Hash32)`** (`ade_types::shelley::block`, **CN-WIRE-09**) —
>   the header `prev_hash` wire grammar `$hash32 / null` (cardano-ledger `PrevHash = GenesisHash | BlockHash`),
>   replacing the prior flat `prev_hash: Hash32` field (**456 → 457** canonical types). Closed 2-variant; **NOT
>   `#[non_exhaustive]`**. ONE **POSITION-BLIND** BLUE codec authority (`ade_codec::shelley::block::decode_prev_hash`
>   decodes `null -> Genesis` / `hash32 -> Block` as a pure function of the CBOR token, NEVER `block_number`; the
>   `ShelleyHeaderBody` `AdeEncode` writes `Genesis -> write_null` / `Block(h) -> write_bytes_canonical`) + ONE
>   **POSITION-AWARE** validator authority (`ade_ledger::block_validity::header_position::check_header_position`).
>   The `null` grammar is scoped to **header_body ONLY** — it MUST NOT leak into the chain-sync/block-fetch
>   `Point`/`Tip` codec (`Point::Origin` stays `array(0)`); a frozen contract fenced by
>   `ci_check_prevhash_single_wire_authority.sh`. A surface REDUCTION, NOT an extension point. Classified under
>   §3 Closed / §4 Frozen.
> - **NEW CLOSED GREEN emit-only scheduling vocabulary (CN-NODE-04, S1):** `ade_node::live_log::sched_event`
>   `NodeSchedEvent` (closed 5-variant) + `FeedReason` (`NoBlockAvailable | CleanEmpty | UnknownDisconnected`) +
>   `ForgeOutcome` (closed 4-variant), with the byte-deterministic JSONL encoder `live_log::sched_writer`. **None
>   is `#[non_exhaustive]`; no catch-all / `Other` variant; no stringly-typed field.** EMIT-ONLY: the GREEN
>   planner never constructs or reads a `NodeSchedEvent`; the relay loop emits them around the planner call +
>   the `LoopStep` arms (recording feed/forge decisions WITHOUT changing them). Allow-list + negative tests;
>   fenced by `ci_check_node_sched_events_emit_only.sh` (the planner must never name the vocabulary). A closed
>   diagnostic vocabulary (a surface REDUCTION), NOT an extension point. Classified under §3 Closed.
> - **ADDITIVE closed-variant extensions (no surface widening):** `BlockValidityError::HeaderPositionInvalid`
>   folds into the **UNCHANGED** closed `BlockRejectClass::HeaderInvalid` (NO new reject class);
>   `NodeForgeError::RecoveredTipMissingBlockNo` (a fail-closed cold-start edge). Both are additive variants on
>   existing closed sums, no wildcard.
> - **NEW BLUE module `ade_ledger::block_validity::header_position`** (added under the existing `block_validity`
>   authority — the SINGLE position-rule authority `check_header_position` + the `HeaderPositionInvalid` variant;
>   a fn + a variant, NO struct/enum). Called by `decode_block` BEFORE the header authority. It **MUST NOT** be
>   duplicated and **MUST NOT** live in `ade_codec`. See §5 Module Addition Rules.
> - **UNCHANGED closed surfaces (note they did NOT gain variants):** `ForgeEpochAdmission` (the off-epoch guard
>   authority), `NodeBlockSource` (closed 2-variant — the cold-start is a new code PATH through the existing
>   `NodeBlockSource`, NOT a new variant), `Mode` / the CLI flag allow-list (unchanged — no private-only flag,
>   fenced by `ci_check_node_path_fidelity.sh`), `CoordinatorEvent` (the cold-start reuses the existing forge
>   events).
> - **S5 rehearsal reuses existing surfaces VERBATIM:** `ba02_evidence::correlate` + `rehearsal_evidence::PrivateRehearsalManifest`
>   (no new evidence type, no manifest-schema change); the genesis-rehearsal home
>   `phase4-n-f-g-j-genesis-rehearsal-*.toml` is covered by the extended `ci_check_rehearsal_manifest_schema.sh`.
>
> **Registry → 319 rules** (NEW `CN-WIRE-09` + `DC-NODE-08` + `CN-NODE-04`, all `tier = derived`, `enforced`;
> `CN-REHEARSAL-FIDELITY-01` gains `strengthened_in += "PHASE4-N-F-G-J"`; no rule weakened). **121 → 126 CI**
> (NEW `ci_check_node_sched_events_emit_only.sh` + `ci_check_prevhash_single_wire_authority.sh` +
> `ci_check_genesis_successor_reachability.sh`).
>
> **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** The raw byte codec is **POSITION-BLIND**
> (the `Genesis|Block` decision is the CBOR token's, never `block_number`'s). The position-AWARE coupling
> (`block_number 0 <=> Genesis`) lives in EXACTLY ONE BLUE authority (`check_header_position`), called by
> `decode_block`, surfacing `HeaderPositionInvalid` (coarse class `HeaderInvalid` — no new `BlockRejectClass`);
> it MUST NOT live in `ade_codec` and MUST NOT be duplicated. No all-zero `Hash32`, no anchor fingerprint, and
> no Shelley genesis hash may stand in for the genesis predecessor — it is structurally CBOR null (the producer
> `prev_hash` migrated `Hash32 -> PrevHash` end to end, deleting the all-zero stand-in at its source in
> `ChainEvolution::prev_hash()`). The node-spine cold-start first-block reachability (`DC-NODE-08`) is **scoped
> to the hermetic cold-start execution** (NO `genesis_forged` latch); the recovered seed-epoch lineage gates
> **PERMISSION** to forge from the genesis-successor position, it is **NOT** the source of the prev_hash bytes;
> the forge engine holds **no `ChainDb` handle**, so it advances **no durable tip** (durable block-1+ = N-U). The
> S1 scheduling-event vocabulary is **emit-only**. **NO RO-LIVE flip** (`RO-LIVE-01` stays `partial` /
> operator-gated); the cold-start eligibility signal is general (forge-configured + valid recovered base),
> **never a private-only / C1-only flag**.
>
> > **NOTE — the G-H cluster is NOT yet reflected in this SEAMS body.** This file was last fully regenerated at
> > the **G-D** close (`6bd60c80`); the intervening **PHASE4-N-F-G-H** cluster (node-spine live serve-to-peer —
> > the NEW RED `ade_runtime::network::serve_dispatch`, `ade_node::node_lifecycle::run_node_serve_task` /
> > `bind_serve_listener` / `ServeStartError`, the additive BLUE fn `n2n_supported_for_magic`, `DC-NODE-07`, and
> > the gates `ci_check_single_serve_dispatch_authority.sh` + `ci_check_serve_listener_magic_aware.sh`) is
> > documented in the CODEMAP carried delta but is **NOT** spliced into the §1–§7 bodies below. This delta-refresh
> > was scoped to **G-J** only (per the regeneration constraint). The header counts above (457 / 126 / 319) are the
> > true HEAD values and DO include G-H's contributions; a fuller G-H surface catch-up (the serve-to-peer §1
> > surface + the §3 `ServeStartError` row + the §5 serve gates) is the recommended follow-on. **Treat the §1–§7
> > bodies as current through G-J for everything EXCEPT the G-H serve-to-peer surface.**
>
> **This regeneration is a scoped INCREMENTAL catch-up through ONE cluster.** The prior file was generated
> at the PHASE4-N-F-G-E close (header `6f848825` / 119 CI checks / 314 rules — live-feed bounded memory before
> authoritative decode/apply on the `--mode node` spine). It is brought current through **PHASE4-N-F-G-D**
> (closing now at `6bd60c80` — a **private-testnet accepted-block bounty DRY-RUN harness** on the same
> `--mode node` spine). **G-D is a bounty DRY-RUN harness — NOT the bounty deliverable.** It adds a
> **path-faithful, non-promotable** rehearsal apparatus and does **NOT** enforce that a C1 run has succeeded
> (the rehearsal-schema gate is **vacuous until a real operator-produced manifest is committed**). The
> seam-relevant additions are **two NEW closed/fenced ATTACH POINTS + two NEW fences — NOT new extension
> points**: (1) a **NEW non-promotable rehearsal-evidence surface** — GREEN `ade_node::rehearsal_evidence`
> (`PrivateRehearsalManifest` WRAPS a correlate-produced `Ba02Manifest` in a structurally distinct,
> NON-PROMOTABLE envelope; closed **1-variant** `RehearsalVenue { PrivateTestnetC1 }`; the SOLE ctor
> `from_correlate_outcome` returns `None` on `NoEvidence`; `to_canonical_toml` ALWAYS emits `is_rehearsal = true`
> + `not_bounty_evidence = true` as LITERALS) + RED `ade_node::rehearsal_pass` (file I/O that REUSES
> `ba02_pass::correlate_peer_log_file` VERBATIM — no alternate correlator — and writes ONLY to the rehearsal
> home `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`), gated by the NEW
> `ci_check_rehearsal_manifest_schema.sh` (vacuous-until-committed; scans the active + archived G-C bounty homes,
> fail-closed; THREE non-promotability barriers); and (2) a **NEW path-fidelity fence**
> `ci_check_node_path_fidelity.sh` pinning the `--mode node` accepted-block path (no new `--mode node` argv flag
> vs the pinned **28-flag** closed allow-list; no from-genesis consensus-inputs constructor; the node path
> sources consensus inputs ONLY via the shared `import_live_consensus_inputs`). **No new BLUE authority / canonical
> type, no new `--mode node` flag, no new `NodeBlockSource` / `CoordinatorEvent` / `Mode` variant, no binary
> wiring.** **No BLUE crate was modified** — the 456 canonical-type total is unchanged (the NEW
> `ade_node::rehearsal_evidence` is GREEN-by-content and the NEW `ade_node::rehearsal_pass` is RED — neither is
> canonical-counted; both live in `ade_node`, not a BLUE `core_paths` entry). **Registry → 315 rules** (NEW
> `CN-REHEARSAL-FIDELITY-01`, `tier = release`, `enforced`; two coupled clauses — path fidelity + evidence
> non-promotability); **119 → 121 CI** (NEW `ci_check_node_path_fidelity.sh` + `ci_check_rehearsal_manifest_schema.sh`).
>
> **Boundary language (load-bearing — do NOT soften / do NOT broaden).** G-D enforces that the private C1
> dry-run is a **path-faithful, non-promotable rehearsal HARNESS** — and nothing more. (1) **PATH FIDELITY:** the
> C1 private dry-run uses the SAME `--mode node` accepted-block path as preview/preprod
> (`import_live_consensus_inputs` → forge → self-accept → sibling-serve → block-fetch → peer log → `correlate`),
> with NO private-only flag / branch / bootstrap authority / from-genesis constructor; the only differences are
> operator-controlled INPUTS (a private genesis whose stake makes Ade win slots fast) + the evidence LABEL. A
> private-only helper that would make the rehearsal pass where the same condition would fail on preview/preprod
> is a **shared-path bug to fix in the shared path, never special-cased.** (2) **EVIDENCE NON-PROMOTABILITY:** any
> private-testnet manifest is clearly marked rehearsal / private-testnet, stored ONLY under the rehearsal home
> (`docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`, never a bounty home), sha256-bound to a real Haskell
> peer log, correlate-produced (`ba02_evidence::correlate` is the SOLE acceptance-evidence constructor), and flips
> **NO RO-LIVE rule**. The C1 dry-run is **NOT a runtime node mode** and is **not wired into any binary arm** —
> the live C1 execution stays `blocked_until_operator_c1_net_executed`, exercised only by the env-gated RED test
> `crates/ade_node/tests/node_c1_dry_run_rehearsal.rs::node_c1_dry_run_rehearsal_live` (`ADE_LIVE_C1_DRY_RUN=1`,
> skipped in CI). G-D does **NOT** enforce that a C1 run has succeeded; the rehearsal gate is **vacuous until a
> real operator-produced manifest is committed**. There is **NO RO-LIVE flip** (`RO-LIVE-01` stays `partial`;
> `RO-LIVE-06` stays schema-only / `enforced`; neither gains a G-D strengthening), **NO bounty / preview / preprod
> completion claim**. Private C1 acceptance ≠ bounty completion; preview/preprod acceptance = the single bounty
> deliverable, captured separately. The G-A..G-E forge / serve / live-feed / containment surfaces are
> **unchanged**; the three containment / handoff / memory fences (`ci_check_node_run_loop_containment.sh`,
> `ci_check_served_chain_handoff_fence.sh`, `ci_check_live_feed_memory_bounds.sh`) are **byte-unchanged**.
>
> ### PHASE4-N-F-G-D (closing, `6bd60c80`) — private-testnet accepted-block bounty DRY-RUN harness on the `--mode node` spine
>
> N-F-G-D introduced / extended (the two NEW surfaces are CLOSED / fenced ATTACH POINTS — classified under §3
> Closed / §4 Frozen; NOT new extension points). G-D adds a path-faithful, non-promotable bounty DRY-RUN
> **harness**, not a runtime mode:
>
> - **NEW CLOSED non-promotable surface (GREEN, `ade_node::rehearsal_evidence` — `//! GREEN`):** the
>   `PrivateRehearsalManifest` (S2) — a structurally distinct, NON-PROMOTABLE rehearsal envelope WRAPPING a
>   correlate-produced `ba02_evidence::Ba02Manifest` payload (the SAME proof the bounty BA-02 path produces).
>   Closed **1-variant** `RehearsalVenue { PrivateTestnetC1 }` (a non-private venue is **unrepresentable** — a
>   rehearsal is never preprod / preview). The **SOLE constructor** `from_correlate_outcome(&BA02Outcome,
>   RehearsalEnvelope) -> Option<Self>` wraps `BA02Outcome::Ba02Manifest` and returns `None` on `NoEvidence`
>   (nothing to wrap, nothing to write) — so a rehearsal manifest is **ALWAYS correlate-produced**; there is **NO**
>   path from raw operator input or a `NoEvidence` outcome to a manifest. `to_canonical_toml` is pure and **ALWAYS
>   emits `is_rehearsal = true` + `not_bounty_evidence = true` as LITERALS** — the type **cannot represent a
>   non-rehearsal**. `REHEARSAL_MANIFEST_SCHEMA_VERSION = 1`. Pure / deterministic (no I/O / clock / rand / float /
>   `HashMap`); promotable-shape GREEN. `session/`-style GREEN-by-content (lives in `ade_node`, NOT a BLUE
>   `core_paths` entry), so **NOT canonical-counted**. **A CLOSED constructor-fenced non-promotable envelope (a
>   surface REDUCTION), NOT an extensible registry / plugin point.** Backs `CN-REHEARSAL-FIDELITY-01` clause 2.
>   Gate: `ci_check_rehearsal_manifest_schema.sh`.
> - **NEW RED rehearsal-evidence file I/O (RED, `ade_node::rehearsal_pass` — `//! RED`):** the file I/O over the
>   GREEN `PrivateRehearsalManifest` (S2). `correlate_peer_log_file_into_rehearsal(ade, peer_log_path, envelope)
>   -> io::Result<Option<PrivateRehearsalManifest>>` **REUSES `ba02_pass::correlate_peer_log_file` VERBATIM** (no
>   alternate correlator), then wraps a correlate-produced payload in the envelope; `Ok(None)` iff `correlate`
>   returned `NoEvidence`; a missing/unreadable file fails closed (`io::Error`).
>   `write_private_rehearsal_manifest(&PrivateRehearsalManifest, out_path) -> io::Result<()>` accepts **ONLY a
>   `PrivateRehearsalManifest`** (the argument type IS the gate — only an already-constructed one is writable).
>   Registered `pub mod rehearsal_pass;` in `lib.rs`. Constructs no evidence, synthesizes no acceptance, uses no
>   alternate correlator. **RED file I/O over an UNCHANGED closed evidence vocabulary — NO new closed enum /
>   registry / plugin point.** Backs `CN-REHEARSAL-FIDELITY-01` clause 2. Gate:
>   `ci_check_rehearsal_manifest_schema.sh`.
> - **REUSED VERBATIM (RED, `ade_node::ba02_pass` — UNCHANGED):** `rehearsal_pass` calls
>   `ba02_pass::correlate_peer_log_file` (→ GREEN `ba02_evidence::correlate`, the SOLE `Ba02Manifest` ctor)
>   verbatim — there is **NO alternate correlator**. The G-C `ba02_pass` evidence I/O is byte-unchanged by G-D.
> - **NEW path-fidelity FENCE (CI, `ci_check_node_path_fidelity.sh`):** the `--mode node` accepted-block PATH
>   FIDELITY fence (S1, CN-REHEARSAL-FIDELITY-01 clause 1). **Guard (a):** the `crates/ade_node/src/cli.rs` argv
>   flag-literal set equals the pinned closed **28-flag** allow-list — G-D adds none; a private-only / venue flag
>   (e.g. `--private-net`, `--from-genesis`, `--devnet`, `--rehearsal`) would change the set and trip the guard.
>   **Guard (b):** no fn whose name carries BOTH `genesis` and `consensus` exists (a from-genesis consensus-inputs
>   constructor; line comments stripped first so prose cannot self-trip) AND `node_lifecycle.rs` sources consensus
>   inputs via the shared `import_live_consensus_inputs`. The C1 dry-run differs from the preprod pass **only in
>   operator INPUTS + the evidence LABEL — never in code.**
> - **NEW rehearsal-schema FENCE (CI, `ci_check_rehearsal_manifest_schema.sh`):** vacuous-until-committed (S2;
>   hardened in S4, CN-REHEARSAL-FIDELITY-01 clause 2). When a committed
>   `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml` is present it verifies the closed **12-field** schema +
>   `schema_version == 1` + `is_rehearsal = true` + `not_bounty_evidence = true` + a `venue` of `private-testnet*`
>   + `peer_log_file_sha256 == sha256(the committed peer-log fixture)`. **THREE non-promotability barriers:** (i)
>   the distinct `docs/evidence/` rehearsal home; (ii) the explicit rehearsal markers; (iii) a cross-check (S4
>   hardening) that **NO** rehearsal marker (`^is_rehearsal` / `^not_bounty_evidence`) appears in any `.toml` under
>   EITHER bounty home (active `docs/clusters/PHASE4-N-F-G-C/` AND archived
>   `docs/clusters/completed/PHASE4-N-F-G-C/`) — built from the EXISTING-homes list first (no `[[ -d ]]`
>   whole-check skip), **fail-closed** on a scan error (grep rc≥2) over an existing home. When NO manifest is
>   committed (the typical state — the C1 dry-run is `blocked_until_operator_c1_net_executed`), the gate is
>   **vacuously satisfied**.
> - **NEW runbook (S3):** `docs/evidence/phase4-n-f-g-d-private-rehearsal-README.md` — the C1 dry-run execution
>   runbook, a provable **STRICT SUBSET** of the G-C preprod operator-pass runbook (same `--mode node` path /
>   `--peer` feed / operator-key flow / peer-log capture / `correlate` / NoEvidence fail-closed; ONLY
>   `venue=private-testnet-c1` + operator genesis stake + the `PrivateRehearsalManifest` envelope differ — the
>   strict-subset property is ANCHORED by the S1 path-fidelity fence).
>
> **Registry delta (N-F-G-D):** **315 rules** (314 → 315). **NEW `CN-REHEARSAL-FIDELITY-01`** (`tier = release`,
> `status = enforced` at this close; `introduced_in = "PHASE4-N-F-G-D"`; two coupled clauses — (1) path fidelity +
> (2) evidence non-promotability; `ci_script = ci/ci_check_node_path_fidelity.sh, ci/ci_check_rehearsal_manifest_schema.sh`;
> 7 tests). **No rule weakened; no RO-LIVE strengthening bump** (G-D does not advance the bounty deliverable).
> **Two new CI gates** (119 → 121): `ci_check_node_path_fidelity.sh` + `ci_check_rehearsal_manifest_schema.sh`.
>
> **Governance note (N-F-G-D).** G-D upholds the load-bearing structural lines. **A HARNESS, not an acceptance
> claim:** G-D ships a path-fidelity fence + a non-promotable rehearsal-evidence surface + a dry-run runbook + an
> operator-gated execution scaffold, but **claims no peer acceptance and no bounty completion** — `RO-LIVE-01`
> stays `partial`, `RO-LIVE-06` enforces schema + mechanics only, and the rehearsal gate is vacuous-until-committed.
> **Path fidelity is mechanically fenced:** `ci_check_node_path_fidelity.sh` forbids any new `--mode node` flag
> (28-flag pinned allow-list) and any from-genesis consensus-inputs constructor — the C1 dry-run differs from
> preprod only in operator inputs + the evidence label, never in code. **Evidence is non-promotable:** `correlate`
> stays the SOLE `Ba02Manifest` ctor; the `PrivateRehearsalManifest` type cannot represent a non-rehearsal; three
> independent barriers (distinct home, explicit markers, bounty-home leak cross-check) keep a rehearsal manifest
> out of the bounty path. **Closed surfaces stay closed:** `RehearsalVenue` is a closed 1-variant enum;
> `NodeBlockSource` / `CoordinatorEvent` / `Mode` are unchanged; no new BLUE authority / canonical type / argv flag
> / bootstrap. **Containment is untouched:** the relay-loop containment gate, the served-chain handoff fence, and
> the live-feed memory bounds are byte-unchanged. **No overclaim:** private C1 acceptance ≠ bounty completion; the
> live C1 execution stays `blocked_until_operator_c1_net_executed`.
>
> ### PHASE4-N-F-G-E (closed, `6f848825`) — live-feed bounded memory before authoritative decode/apply on the `--mode node` spine
>
> N-F-G-E introduced / extended (all NEW surfaces are CLOSED / fenced — classified under §3 Closed / §4 Frozen;
> NOT new extension points). This is a surface REDUCTION (two fail-closed memory bounds in front of the BLUE
> decode path), NOT a new attach point:
>
> - **NEW CLOSED const + additive CLOSED-enum variant (GREEN, `ade_network::session::{core, event}`):**
>   `MAX_REASSEMBLY_TAIL_BYTES = 16 * 1024 * 1024` (`session/core.rs:49`). After `drain_protocol_items` drains
>   every COMPLETE item, an incomplete per-mini-protocol reassembly tail whose `buf.len() > MAX_REASSEMBLY_TAIL_BYTES`
>   (`core.rs:249`) returns the NEW `SessionError::ReassemblyBufferOverflow { protocol, len, cap }`
>   (`event.rs:195`) — **fail closed, drop the peer; no silent truncation, no partial decode**. The cap fires
>   **BEFORE** the BLUE `ade_codec` decode path. `SessionError` gains the variant **additively** — there is **NO
>   wildcard**; the SOLE exhaustive consumer `ade_runtime::network::mux_pump::session_err_to_halt` (`mux_pump.rs:278`)
>   maps it → `PeerHaltReason::ChainSyncDecodeError` (`mux_pump.rs:302-303`). Deterministic (pure `buf.len()`
>   check; the GREEN no-clock/rand/float/HashMap contract is preserved). `session/` is GREEN-by-content (NOT a
>   BLUE `ade_network` submodule `core_path`), so the new variant is **NOT canonical-counted**. **A closed memory
>   bound + a closed additive enum variant (a surface REDUCTION), NOT a new extension point.** Backs
>   `DC-LIVEMEM-01`. Gate: `ci_check_live_feed_memory_bounds.sh`.
> - **NEW CLOSED const (RED, `ade_node::node_sync`):** `MAX_WIRE_PUMP_LOOKAHEAD = 256` (`node_sync.rs:58`).
>   `pump_lookahead` stops the opportunistic `try_recv` drain at the cap (`node_sync.rs:126`,
>   `lookahead.len() >= MAX_WIRE_PUMP_LOOKAHEAD`), so the existing bounded `mpsc` (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`)
>   **back-pressures** the pump. Content-blind; the verdict-decoupled `NodeBlockSource` contract (closed 2-variant
>   `{WirePump, InMemory}`) + arrival order are **unchanged** — this is a depth cap on the existing opaque
>   `VecDeque<Vec<u8>>` lookahead, **not a new variant / source / verdict**. **A closed lookahead-depth bound (a
>   surface REDUCTION), NOT a new extension point.** Backs `DC-LIVEMEM-01`. Gate:
>   `ci_check_live_feed_memory_bounds.sh`.
> - **CHANGED (RED, `ade_runtime::network::mux_pump`):** `session_err_to_halt` maps the new
>   `SessionError::ReassemblyBufferOverflow { .. }` → `PeerHaltReason::ChainSyncDecodeError` (drop the peer). The
>   exhaustive `match` (no wildcard) is what makes the additive `SessionError` variant a closed extension.
> - **NEW CI gate** `ci_check_live_feed_memory_bounds.sh` (DC-LIVEMEM-01) — verifies BOTH bounds are CLOSED
>   LITERAL constants AND not wired to CLI / env / config (the no-escape-hatch guard 3; line comments are stripped
>   first so the doc-comments naming "CLI / env / config" do not self-trip). **Both caps are closed literals, NOT
>   tunables.**
>
> **Registry delta (N-F-G-E):** **314 rules** (313 → 314). **NEW `DC-LIVEMEM-01`** (`tier = derived`,
> operational-hardening — live-feed peer-driven memory bounded before authoritative decode/apply) at
> `status = enforced`. **No rule weakened.** **One new CI gate** (118 → 119): `ci_check_live_feed_memory_bounds.sh`.
>
> **Governance note (N-F-G-E).** G-E upholds the load-bearing structural lines. **Operational hardening, not an
> authority change:** `DC-LIVEMEM-01` is `tier = derived` — explicitly NOT BLUE consensus law (the rule text says
> so); it bounds shell/glue memory, never a chain fact. **The cap fires before the BLUE decode path** (`ade_codec`
> byte-unchanged) — no silent truncation, no partial decode, no unbounded fallback. **Closed source/enum surfaces
> stay closed:** the reassembly cap fails closed via an *additive* closed-enum variant
> (`SessionError::ReassemblyBufferOverflow`, no wildcard, one exhaustive consumer); `NodeBlockSource` stays the
> 2-variant `{WirePump, InMemory}`; both bounds are closed literal constants with no runtime escape hatch.
> **Containment is untouched:** the serve/forge/containment fences (`ci_check_node_run_loop_containment.sh` +
> `ci_check_served_chain_handoff_fence.sh`) are byte-unchanged. **No overclaim:** the claim is bounded memory
> before authoritative decode/apply — NOT network DoS resistance, NOT peer fairness, NOT BA-02 readiness; no
> `RO-LIVE` flip.
>
> ### PHASE4-N-F-G-C (closed, `351d46bc`) — live WirePump feed + BA-02 evidence wiring on the `--mode node` spine
>
> **Boundary language (carried — N-F-G-C — do not soften).** G-C closes the **MECHANICAL live-feed + evidence
> scaffolding ONLY**. The `--mode node` `On` arm is **live-feed-wireable**: when `--peer` is supplied,
> `spawn_live_wire_pump_source` builds a LIVE `NodeBlockSource::WirePump` by reusing the closed `dial_for_admission`
> + `run_admission_wire_pump` (the runtime `AdmissionPeerEvent` feeds the `WirePump` arm directly — no bridge, no
> reimplemented wire authority, no new variant). With a live feed the forge is observable when the feed is
> Continuing and a due leader slot is reached; **peer ACCEPT is NOT claimed here — it is operator-gated**
> (`RO-LIVE-01` partial / `blocked_until_operator_stake_available`), proven only by an operator-captured peer log
> through `ba02_evidence::correlate`. The BA-02 evidence I/O (`ba02_pass`) reads a real operator-captured
> peer-log file and runs it through the SOLE `Ba02Manifest` constructor (`correlate`); `write_ba02_manifest`
> accepts ONLY a `Ba02Manifest`, so a written manifest is always correlate-produced; **no synthetic manifest is
> committed** (the gate is vacuous-until-committed and sha256-binds the fixture). Ade self-accept /
> `ForgeSucceeded` / served-block / wire success **≠ peer acceptance**. With NO `--peer` the empty source still
> halts before any `ForgeTick` (forge-CAPABLE, not observable). The durable tip still advances only via
> `run_node_sync → pump_block` (no second bootstrap, CN-NODE-01); `run_node_sync` is UNMODIFIED;
> `ci_check_node_run_loop_containment.sh` is **byte-unchanged**. There is **NO new BLUE authority / canonical
> type, no new `NodeBlockSource` variant, no new `CoordinatorEvent` variant**, and the bounty acceptance
> criterion (an operator-witnessed accepted block) is **NOT** satisfied by this cluster. _(N-F-G-E added two
> closed memory caps in front of this live feed; it changed neither the live-feed wiring, the serve mechanism, nor
> any BLUE authority.)_
>
> N-F-G-C introduced / extended (all NEW surfaces are CLOSED / fenced ATTACH POINTS — classified under §3 Closed
> / §4 Frozen; NOT new extension points; carried unchanged from the G-C close):
>
> - **NEW node-spine ATTACH POINT that REUSES existing closed wire infra (RED, `ade_node::node_lifecycle`):**
>   the live WirePump feed (S1) — `spawn_live_wire_pump_source(peer_addrs: &[String], network_magic: u32,
>   recovered_tip: Option<&ChainTip>) -> NodeBlockSource`. RED wiring ONLY: it reuses the closed
>   `ade_runtime::admission::{dial_for_admission, run_admission_wire_pump}` VERBATIM (no reimplementation, no new
>   wire authority) and feeds their `AdmissionPeerEvent` output DIRECTLY into the `WirePump` arm via
>   `NodeBlockSource::from_wire_pump(rx)` (the node spine consumes the runtime event type — no bridge). Each
>   `--peer` is dialed in a `tokio::spawn` whose `AdmissionPeerEvent` output is fed into a **bounded** `mpsc`
>   (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`); the `start_point` is the recovered tip (`Point::Block`) or
>   `Point::Origin`. The `On` arm wires it iff `--peer` is supplied (`let mut source = if live_feed_wired {
>   spawn_live_wire_pump_source(...) } else { NodeBlockSource::in_memory(Vec::new()) }`). **The live source is a
>   FILL of the closed 2-variant `NodeBlockSource` {WirePump, InMemory} (NOT a new variant), adds no second
>   tip-advance path, carries no verdict.** Honest-scope (C3, mirrors `admission::bootstrap::
>   spawn_wire_pumps_for_admission`): an unparseable addr or a `dial_for_admission` failure is logged-and-dropped
>   — never fatal, never a fabricated address, never a silent tip graft; if no peer yields a live pump the feed
>   ends and the loop halts clean (same outcome as the empty source). **This CLOSES the prior §7 candidate #0
>   "live network serve + operator-peer surface (G-C)" on the CONSUME/FEED side** — the live feed is now wired.
>   Gate: `ci_check_served_chain_handoff_fence.sh` BROADENED in place (S1); `ci_check_node_run_loop_containment.sh`
>   byte-unchanged.
> - **CHANGED (RED, fn-visibility promotion — reuse, no reimpl, `ade_node::admission::bootstrap`):**
>   `build_n2n_version_table` is promoted `fn` → `pub(crate) fn` (S1) so `node_lifecycle::spawn_live_wire_pump_source`
>   can REUSE the existing N2N version-table builder instead of reimplementing it. No behavior change.
> - **NEW RED BA-02 evidence-I/O module (RED, `ade_node::ba02_pass` — `//! RED`):** operator-pass BA-02 evidence
>   file I/O (S2). `correlate_peer_log_file(ade: &AdeForgeRecord, peer_log_path: &Path) -> io::Result<BA02Outcome>`
>   reads the operator-captured peer-accept JSONL file → GREEN `ba02_evidence::parse_peer_accept_events`
>   (allow-list) → `ba02_evidence::correlate` (the SOLE `Ba02Manifest` ctor); a missing/unreadable file fails
>   closed (`io::Error`), never a synthesized acceptance. `write_ba02_manifest(manifest: &Ba02Manifest, out_path:
>   &Path) -> io::Result<()>` accepts ONLY a `Ba02Manifest` (which only `correlate`'s exact-match arm
>   constructs), so a written manifest is ALWAYS correlate-produced. Registered `pub mod ba02_pass;` in `lib.rs`.
>   **NO new closed enum / registry** — two file-I/O fns over the pre-existing closed `Ba02Manifest` / `BA02Outcome`
>   / `PeerAcceptEvent` / `NoEvidenceReason` vocabulary; the GREEN `ba02_evidence` correlator is **UNCHANGED**.
>   Constructs no evidence, derives no acceptance, never coerces a non-acceptance line. Reached by no binary arm
>   at this HEAD (exercised by `crates/ade_node/tests/node_operator_pass_ba02.rs` — incl. the env-gated
>   `node_operator_pass_ba02_live`). Backs the BA-02 leg of `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01`.
> - **GREEN (BA-02 evidence correlator — UNCHANGED, `ade_node::ba02_evidence`):** `correlate` stays the SOLE
>   `Ba02Manifest` constructor; the closed `BA02Outcome` / `PeerAcceptEvent` / `NoEvidenceReason` / versioned
>   `Ba02Manifest` surface is byte-unchanged. G-C only *consumes* it from the new RED `ba02_pass` I/O wrapper.
> - **NEW CI gate** `ci_check_ba02_evidence_manifest_schema.sh` (S2, RO-LIVE-06 BA-02 evidence; mirrors
>   `ci_check_operator_evidence_manifest_schema.sh`) — when a committed `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml`
>   manifest is present, it verifies the closed **8-field** schema (`schema_version`, `block_hash`, `slot`,
>   `peer_log_file`, `peer_log_file_sha256`, `peer_log_capture_command`, `peer_log_filter`, `accept_event_kind`),
>   `schema_version == 1`, and that `peer_log_file_sha256` matches the actual SHA-256 of the committed peer-log
>   fixture it binds. When NO manifest is committed (the typical state — the live operator pass is
>   `blocked_until_operator_stake_available`), the gate is **vacuously satisfied** (no manifest, no claim). This is
>   **the no-synthetic-manifest enforcer**: a hand-authored manifest with no real fixture, or a tampered fixture,
>   FAILS.
> - **BROADENED CI gate (in place, NOT new)** `ci_check_served_chain_handoff_fence.sh` (S1, DC-NODE-06
>   serve-ingress clause) — the OWNERS set is broadened from `{node_lifecycle.rs}` to `{node_lifecycle.rs,
>   node_sync.rs}` so the fence still holds if the serve owner moves; guard (3) is flipped from a 3-name
>   **deny-list** to an **ALLOW-LIST** (every node-spine unbounded handoff channel MUST carry `SelfAcceptedHandoff`
>   — any other token is a violation — and at least one `UnboundedSender<SelfAcceptedHandoff>` must be present).
>   Guards (1) every `push_atomic(` fed by `into_accepted()` and (2) no direct `served_chain_admit(` on the node
>   spine are unchanged. A **net tightening**. The CI count does NOT change for this file (modified in place).
> - **NEW runbook** `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` (S2) — the operator-pass execution
>   runbook for the live BA-02 leg (how to capture the peer-accept log, what `correlate` requires, and the
>   no-synthetic-manifest discipline). NEW test file `crates/ade_node/tests/node_operator_pass_ba02.rs`
>   (`correlate_wired_to_operator_peer_log`, `correlate_from_operator_log_file_is_deterministic`, env-gated
>   `node_operator_pass_ba02_live`); the live-feed reachability/serve legs are
>   `node_sync::tests::live_wire_pump_feed_reaches_forge_tick` +
>   `forge_succeeds::live_feed_forge_serve_loopback_returns_forged_block`.
>
> **Registry delta (N-F-G-C):** stays **313 rules** — **no new rule**. `RO-LIVE-01` (stays `partial` — its
> `open_obligation` now records that G-C wired the MECHANICAL half; the LIVE half stays
> `blocked_until_operator_stake_available`), `RO-LIVE-06` (stays `enforced` — schema + mechanics only; LIVE BA-02
> IS NOT CLAIMED), `CN-OPERATOR-EVIDENCE-01` (stays `enforced` — `code_locus` now cites the new runbook + the new
> gate), `DC-NODE-06` (stays `enforced` — `tests` gains `live_feed_forge_serve_loopback_returns_forged_block`)
> each gain `strengthened_in += "PHASE4-N-F-G-C"`. **One new CI gate** (117 → 118):
> `ci_check_ba02_evidence_manifest_schema.sh`. _(These registry edits are uncommitted in the working tree at this
> regen — the cluster-close strengthening lands alongside this doc.)_
>
> **Governance note (N-F-G-C).** G-C upholds the load-bearing structural lines. **Mechanical scaffolding, not an
> acceptance claim:** G-C wires a live feed and the BA-02 evidence I/O, but **claims no peer acceptance** —
> `RO-LIVE-01` stays `partial` / `blocked_until_operator_stake_available`, `RO-LIVE-06` enforces schema +
> mechanics only, and the new gate is vacuous-until-committed. **Reuse, not reimplementation:** the live feed
> reuses the closed `dial_for_admission` + `run_admission_wire_pump` verbatim (no new wire authority);
> `build_n2n_version_table` is reused via a fn-visibility promotion. **Closed source stays closed:**
> `NodeBlockSource` is still the 2-variant `{WirePump, InMemory}` — the live source is a *fill* of the existing
> arm via `from_wire_pump`, not a new variant; the durable tip still advances only via `run_node_sync →
> pump_block`; `ci_check_node_run_loop_containment.sh` is byte-unchanged. **`correlate` is the sole `Ba02Manifest`
> ctor:** `ba02_pass::write_ba02_manifest` accepts ONLY a `Ba02Manifest`, so a written manifest is always
> correlate-produced from a real peer log; no synthetic manifest is committed; Ade self-accept / served-block /
> wire success are NOT evidence. **No overclaim:** no new BLUE authority, no new canonical type, no new
> `NodeBlockSource` / `CoordinatorEvent` variant, and the bounty acceptance criterion is NOT satisfied here.
>
> ### PHASE4-N-F-G-B (closed, `339cccb1`) — self-accept→serve handoff on the `--mode node` spine
>
> N-F-G-B introduced / extended (all NEW surfaces are CLOSED / fenced — classified under §3 Closed / §4 Frozen):
>
> - **NEW CLOSED surface (GREEN, `ade_runtime::producer::self_accepted_handoff`):** `SelfAcceptedHandoff` (S1)
>   — a constructor-fenced newtype over the BLUE `ade_ledger::producer::AcceptedBlock`. Private field
>   `accepted: AcceptedBlock`; **SOLE constructor** `from_self_accepted(AcceptedBlock) -> Self`; accessors
>   `accepted(&self) -> &AcceptedBlock` + `into_accepted(self) -> AcceptedBlock`; `#[derive(Debug, Clone,
>   PartialEq)]`. There is **NO** constructor from a raw `Vec<u8>`, a `ForgedBlockArtifact` (`artifact.bytes`
>   is never an `AcceptedBlock` source — re-deriving the token would breach CN-FORGE-01; the carrier holds the
>   ORIGINAL token), a `CoordinatorEvent`, a self-declared acceptance flag, or a peer verdict — so "hand the
>   serve task an artifact that was not BLUE self-accepted" is **type-unrepresentable**. Pure / deterministic
>   (no I/O, clock, rand, float); the carrier wraps the original token verbatim and never re-validates or
>   re-derives it. Registered in `producer/mod.rs` (`pub mod self_accepted_handoff;`). **A CLOSED
>   constructor-fenced carrier (a surface REDUCTION), NOT an extensible registry.** Backs `DC-NODE-06`. Gate:
>   `ci_check_served_chain_handoff_fence.sh` (S3; broadened in place by G-C S1).
> - **NEW fenced seam (RED, `ade_node::node_lifecycle`):** the self-accept→serve handoff seam — `ForgeActivation`
>   gains a private `handoff_tx: Option<mpsc::UnboundedSender<SelfAcceptedHandoff>>` + a `with_handoff_sender(tx)
>   -> Self` builder (S2). The dispatcher `run_node_lifecycle_inner` (`On` arm) `tokio::spawn`s a **sibling**
>   served-chain admit task that drains the typed channel and admits via the single
>   `ServedChainHandle::push_atomic(handoff.into_accepted())` — the **SOLE** node-spine `push_atomic` site. The
>   relay-loop body forwards each surfaced handoff as a best-effort typed `tx.send(h)` only; it holds **only** the
>   `Sender` — never a `ServedChainHandle`, never `push_atomic`, never a served-chain mutation — so the relay-loop
>   containment gate stays byte-unchanged. **A typed channel seam + a single reused serve authority, NOT an
>   extension point.** Gate: `ci_check_served_chain_handoff_fence.sh` (the relay-loop gate
>   `ci_check_node_run_loop_containment.sh` is byte-unchanged).
> - **CHANGED (RED, `ade_node::produce_mode::run_real_forge`):** split into a thin wrapper + a private
>   **byte-identical** `run_real_forge_inner`; the public signature is now `(CoordinatorEvent,
>   Option<ade_ledger::producer::AcceptedBlock>)` (S1). The inner fn writes the BLUE self-accepted token into the
>   out-param ONLY on the success path, so `Some(..)` is structurally reachable iff the event is `ForgeSucceeded`;
>   every fail-closed branch leaves it `None`. The `produce_mode` `BroadcastBlock` caller takes `.0` (its serve
>   path advances via `ChainEvolution::advance` and ignores the surfaced token — functionally unaffected). The
>   closed `CoordinatorEvent` / `ForgeSucceeded` surface is **unchanged** — **no new variant**.
> - **CHANGED (RED, `ade_node::node_sync::forge_one_from_recovered`):** now returns `(CoordinatorEvent,
>   Option<SelfAcceptedHandoff>)` (S1); wraps the token surfaced by `run_real_forge` via
>   `self_accepted.map(SelfAcceptedHandoff::from_self_accepted)` (`None` on `ForgeNotLeader` / `ForgeFailed` / the
>   off-epoch fail-closed branch). The token is the ORIGINAL from BLUE `self_accept` (CN-FORGE-01), never
>   re-derived from `artifact.bytes`.
> - **NEW CI gate** `ci_check_served_chain_handoff_fence.sh` (S3, CE-G-B-3 / DC-NODE-06 serve-ingress clause) —
>   _(broadened in place by G-C S1; current detail in the G-C bullet above and the CI table below.)_
>
> **Registry delta (N-F-G-B):** stayed **313 rules** — **no new rule** (`DC-NODE-06` already existed as the G-A
> forward sketch). **`DC-NODE-06`** (self-accept→serve handoff, shape B) flipped **`declared` → `enforced`**.
> `CN-PROD-04` + `CN-CONS-07` gained `strengthened_in += "PHASE4-N-F-G-B"`. **One new CI gate** (116 → 117).
>
> **Governance note (N-F-G-B).** G-B upholds the load-bearing structural lines. **Serve authority, not a
> relaxation:** the relay-loop body performs no serve / admit / gossip / block-fetch / durable-tip mutation — the
> handoff from the loop to the sibling task is a **typed channel send** of a constructor-fenced self-accepted
> artifact, so `ci_check_node_run_loop_containment.sh` stays semantically unchanged. **Only self-accepted
> artifacts serve:** the typed `SelfAcceptedHandoff`'s sole provenance is the BLUE `AcceptedBlock` from
> `self_accept` (private-field constructor fence, CN-FORGE-01). **Single serve mutation:** served-chain mutation
> happens only via the single `ServedChainHandle::push_atomic`, fed only by `into_accepted()`. **No overclaim:**
> no new BLUE authority, no new canonical type, no new `CoordinatorEvent` variant, no live feed, and **no
> peer-acceptance / BA-02 / RO-LIVE claim** (peer ACCEPT stays operator-gated; RO-LIVE-01 partial). `DC-NODE-06`
> flipped declared→enforced. _(G-C does NOT alter the serve mechanism — it only adds the live feed source and the
> BA-02 evidence I/O around it.)_
>
> ### PHASE4-N-F-G-A (closed, `80dac1f7`) — forge fidelity on the `--mode node` spine
>
> N-F-G-A introduced / extended (all CLOSED — classified under §3 Closed / §4 Frozen+version-gated):
>
> - **NEW CLOSED surface (GREEN-by-content, `ade_runtime::clock`):** `SlotAlignmentError` — a **1-variant**
>   closed enum (`BeforeGenesisAnchor`, `crates/ade_runtime/src/clock.rs:99`) carried by the NEW pure
>   `checked_millis_to_slot(tick_millis, start_millis, start_slot, slot_length_ms) -> Result<SlotNo,
>   SlotAlignmentError>` (S3). It returns `Err(BeforeGenesisAnchor)` for `tick_millis < start_millis` — the
>   exact before-anchor case the saturating `millis_to_slot` masks to `start_slot` — otherwise the EXACT
>   `millis_to_slot` result. Pure integer arithmetic; no wall-clock, no float. The saturating `millis_to_slot`
>   is left intact for the non-forge callers. **A surface REDUCTION (a closed fail-closed boundary), NOT an
>   extension point.** Gate: the before-anchor fail-closed wiring is asserted via the S3 unit tests + the
>   node-path tests (the `ci_check_clock_seam.sh` seam check is unchanged). DC-EPOCH-03-adjacent (the clock
>   guard on the forge path).
> - **NEW CLOSED surface (GREEN-by-content, `ade_runtime::consensus_inputs::protocol_params`):**
>   `ProtocolParamsParseError` (`crates/ade_runtime/src/consensus_inputs/protocol_params.rs:55`; incl.
>   `InexactRational { field: &'static str }` + `JsonShape`) — the closed error set of the NEW cardano-cli
>   `query protocol-parameters` JSON parser `parse_protocol_parameters_json(json, network_magic) ->
>   Result<ProtocolParameters, ProtocolParamsParseError>` (S2a). It converts the oracle's protocol-parameters
>   JSON (the `protocol_params_json` preimage) into a canonical BLUE `ade_ledger::pparams::ProtocolParameters`
>   so the recovered ledger carries the **current** protocol version + modeled parameters. **No float path
>   (hard rule):** rational unit-interval / non-negative-interval literals are preserved as
>   `serde_json::value::RawValue` strings and converted to exact `ade_ledger::rational::Rational` by integer
>   decimal/scientific parsing — no `f64`, no `as f64`, no serde float deserialization; a literal that cannot
>   be represented exactly fails closed (`InexactRational`). **A surface REDUCTION (a closed RED-parse →
>   BLUE-type pipeline), NOT an extension point.** Gate: `ci_check_recovered_ledger_pparams_sourced.sh`
>   (install-side, S2a).
> - **NEW CLOSED surface (GREEN-by-content, `ade_runtime::consensus_inputs::canonical`):**
>   `ForgeCurrentPParamsError` — a **3-variant** closed enum (`PreimageAbsent` / `BindMismatch` /
>   `Parse(ProtocolParamsParseError)`, `crates/ade_runtime/src/consensus_inputs/canonical.rs:129`) carried by
>   the NEW `LiveConsensusInputsCanonical::require_forge_current_pparams() -> Result<ProtocolParameters,
>   ForgeCurrentPParamsError>` accessor (S2a). `LiveConsensusInputsCanonical` now carries `protocol_params_json:
>   Option<String>` **OUTSIDE** the frozen 15-field canonical fingerprint (the fingerprint already commits to
>   `protocol_params_hash`, so the preimage is a non-fingerprinted carry — it does not alter the bundle
>   fingerprint). `require_forge_current_pparams()` requires the preimage be present (`PreimageAbsent`),
>   hash-bind via `blake2b_256` to the fingerprinted `protocol_params_hash` (`BindMismatch`), and parse exactly
>   (`Parse`). **A hash-bound accessor, NOT an extension point.** Gate:
>   `ci_check_recovered_ledger_pparams_sourced.sh`.
> - **NEW CLOSED surface (GREEN-by-function inside a RED module, `ade_node::node_sync`):**
>   `ForgeEpochAdmission` — a **2-variant** closed enum (`WithinSeedEpoch` / `OffEpoch { located, seed }`,
>   `crates/ade_node/src/node_sync.rs:355`) carried by the NEW pure `forge_epoch_admission(slot, era_schedule,
>   seed_epoch) -> ForgeEpochAdmission` guard (S4), derived solely via the BLUE `EraSchedule::locate`. Within
>   the seed epoch ⇒ `WithinSeedEpoch`; any other located epoch ⇒ `OffEpoch { Some(e), seed }`; a slot that does
>   not locate ⇒ `OffEpoch { None, seed }`. `forge_one_from_recovered` calls it **before** `query_leader_schedule`,
>   so an off-epoch forge fails closed before leadership / KES signing. **A closed classifier vocabulary
>   (an off-epoch slot is an error, never a third variant), NOT an extension point.** Gate:
>   `ci_check_node_forge_single_epoch_fail_closed.sh` (S4, CE-G-A-4 / DC-EPOCH-03).
> - **CLOSED surface DELIBERATELY NOT EXTENDED (GREEN, `ade_runtime::producer::coordinator`):** the S4
>   off-epoch path routes to the EXISTING closed **9-variant** `CoordinatorEvent::ForgeNotLeader`
>   (`SlotTick` / `ForgeSucceeded` / `ForgeNotLeader` / `ForgeFailed` / `PeerConnected` / `PeerDisconnected` /
>   `LedgerSnapshotUpdated` / `BroadcastDrained` / `Shutdown`) — **NO new variant was added** for off-epoch.
>   An off-epoch forge is surfaced as a "not leader" outcome through the existing closed vocabulary, keeping
>   the event set closed and additively-stable.
> - **EXTENDED (RED, `ade_node::operator_forge`):** retires the `parse_simple_opcert_json` /
>   `parse_simple_genesis_json` stubs **on the node path** and uses the REAL cardano-cli closed-contract
>   parsers `ade_runtime::producer::opcert_envelope::parse_opcert_envelope` +
>   `ade_runtime::producer::genesis_parser::parse_shelley_genesis` (S2). The genesis reuse extracts
>   clock/KES/network constants ONLY — never a starting-state source. Same `OperatorForgeError` shape (6
>   variants); the `OpcertParse`/`GenesisParse` `&'static str` details now come from the real parsers. NO new
>   BLUE authority, NO plugin seam.
> - **EXTENDED (RED, `ade_node::node_lifecycle`):** the `On`-arm `ForgeTick` consumes the checked clock guard
>   (S3 — `checked_millis_to_slot`; before-anchor fails closed, recorded in `last_slot_alignment_fail:
>   Option<SlotAlignmentError>`) and sources `protocol_version` + `pparams` from the recovered CURRENT ledger
>   view (S2 — `node_lifecycle::forge_constants_from_pparams`), not produce-path defaults.
>   `run_relay_loop`'s containment is **semantically unchanged** (the new boundaries sit inside the existing
>   fence).
> - **EXTENDED (RED, `ade_node::admission::{seed_to_snapshot, bootstrap}`):** `build_seed_ledger` installs the
>   supplied **current** `ProtocolParameters` (not `::default()`) into the recovered ledger at seed/import time
>   (S2a); the forge-capable caller passes the oracle-bound parameters via `require_forge_current_pparams`.
>   Warm-start preserves them.
> - **NEW (GREEN, `ade_testkit::consensus::genesis_pinning`):** a `#[cfg(test)]` genesis-consistency pinning
>   harness (S1) that reads the committed private-net Ade-as-leader reference fixture (S1b), builds the
>   recovered seed-epoch surface, drives the **REAL** `bootstrap_initial_state` warm-start, and pins Ade's
>   recovered values + leader-eligibility inputs against the genesis-derived reference. Non-authoritative test
>   infrastructure; **evidence input, never runtime authority**.
> - **Four NEW CI gates** (S1/S2/S2a/S4): `ci_check_genesis_consistency_fixture_present.sh`,
>   `ci_check_recovered_ledger_pparams_sourced.sh`, `ci_check_node_forge_real_cli_ingress.sh`,
>   `ci_check_node_forge_single_epoch_fail_closed.sh`.
>
> **Registry delta (N-F-G-A):** **313 rules** at HEAD (311 → 313). **NEW `DC-EPOCH-03`** (single-epoch forge
> fail-closed on the `--mode node` spine) at `status = enforced`. **NEW `DC-NODE-06`** (self-accept→serve handoff,
> shape B) at `status = declared` — a **forward sketch** for the NEXT sub-cluster G-B (since flipped to
> `enforced` by G-B). Seven `strengthened_in += "PHASE4-N-F-G-A"` bumps. **Four new CI gates** (112 → 116).
>
> **Governance note (N-F-G-A).** The load-bearing structural lines hold. The forge stays **subordinate**: the
> GREEN planner still learns only whether a slot is *due* (content-blind `ForgeSlotStatus`), never who is a
> leader; the single recovered `BootstrapState` is the forge base (no second bootstrap, CN-NODE-01); the
> single durable tip-advance path remains `run_node_sync → pump_block`; the forge advances no tip and is
> **self-accept-only**. The two new boundaries are **fail-closed walls, not silent saturations**. The S1 fixture
> is **evidence input, never runtime authority**. `protocol_params` carries **no float path**. The
> `protocol_params_json` preimage stays **OUTSIDE** the frozen 15-field `LiveConsensusInputsCanonical`
> fingerprint — a non-fingerprinted additive carry, **NOT a fingerprint-schema change** (§4). `DC-NODE-06` is a
> *declared* sketch (the serve handoff is the next sub-cluster G-B).
>
> ### PHASE4-N-F-F (closed, `4eb7610`) — operator-key ingress + forge-on flip on the relay spine
>
> N-F-F introduced / extended (all CLOSED — classified under §3 Closed / §4 Frozen):
>
> - **NEW CLOSED surface (GREEN, `ade_node::forge_intent`):** the pure tri-state forge-intent classifier —
>   the total decision "may `--mode node` forge?" as a function of which operator-key CLI flags are *present*
>   (never of their contents). Closed **2-variant** `ForgeIntent { On(ForgePaths), Off }`, presence-validated
>   `ForgePaths` (paths only — no secrets, no contents), closed **1-variant** `ForgeIntentError::PartialKeySet
>   { present, missing }` (static flag-name strings only — no path/secret bytes). `classify_forge_intent(cold,
>   kes, vrf, opcert, genesis: Option<&Path>) -> Result<ForgeIntent, ForgeIntentError>` is pure/total over all
>   2⁵ presence combinations (all five ⇒ `On`; none ⇒ `Off`; any partial ⇒ `PartialKeySet`), binding the
>   partial arm by name (no wildcard). No I/O, no secret; promotable to BLUE. **A surface REDUCTION (a closed
>   classifier), NOT an extension point.** Gate `ci_check_forge_intent_closed.sh`. CN-NODE-03 (intent half).
> - **NEW CLOSED surface (RED, `ade_node::operator_forge`):** the **single named `--mode node` operator-material
>   ingress site**. `load_operator_producer_shell(&ForgePaths) -> Result<ProducerShell, OperatorForgeError>`
>   reuses the existing cold/vrf/kes loaders (no reimpl) and `ProducerShell::init` (which enforces the
>   KES-period-vs-opcert freshness bound, CN-PROD-02). `build_operator_forge_material(&ForgePaths) ->
>   Result<OperatorForgeMaterial, OperatorForgeError>` returns the custody shell + `GenesisAnchor` + `pool_id =
>   Hash28(blake2b_224(cold_vk))` (the one named derivation) + clock-seam anchors. Closed **6-variant**
>   `OperatorForgeError` (`ColdKeyLoad` / `VrfKeyLoad` / `KesKeyLoad` / `OpcertParse` / `ShellInit` /
>   `GenesisParse`). `OperatorForgeMaterial` is deliberately NOT `Debug` / `Serialize`. **This is a RED-parse →
>   BLUE-structural-validate → canonical-type ingress reusing existing loaders — NO new BLUE authority, NO
>   plugin seam.** _(G-A S2 retired the `parse_simple_*` stubs on this path for the real cardano-cli parsers.)_
>   Gate `ci_check_operator_forge_no_secret_leak.sh`. CN-NODE-03 (custody half).
> - **EXTENDED (RED, `ade_node::node_lifecycle`):** the `--mode node` arm classifies forge intent and flips
>   the binary forge path from N-F-E's always-`None` to a real `Some`/`None` decision keyed off operator-key
>   flag presence. NO Mithril call, NO second bootstrap (CN-NODE-01 preserved). _(G-A extends the `On` arm's
>   `ForgeActivation` assembly + the `forge_one_from_recovered` fence; G-C extends the `On` arm's source
>   selection (live feed vs empty) only; neither alters the intent classifier or the custody-confined ingress
>   surface.)_ CN-NODE-03 / CN-NODE-01.
> - **NEW CI gate** `ci_check_forge_intent_closed.sh` (CN-NODE-03 intent half). **NEW CI gate**
>   `ci_check_operator_forge_no_secret_leak.sh` (CN-NODE-03 custody half). **MODIFIED-in-place CI gate**
>   `ci_check_node_binary_uses_single_bootstrap.sh` (CN-NODE-01 — `ReceiveState::new` owner allow-list
>   `{node.rs, node_lifecycle.rs}`).
>
> **Registry delta (N-F-F):** **311 rules** at the N-F-F close. **NEW `CN-NODE-03`** at `status = enforced`.
> `strengthened_in += "PHASE4-N-F-F"` on four rules. Two new CI gates + one modified in place.
>
> ### PHASE4-N-F-E (closed, `cd2484f`) — forge-tick on the relay run-loop spine
>
> N-F-E introduced / extended (all CLOSED — classified under §3 Closed / §4 Frozen):
>
> - **CLOSED, additively extended (GREEN, `ade_node::run_loop_planner`):** the pure `LoopStep` vocabulary went
>   **3 → 4 variants** (`+ ForgeTick`), and a NEW content-blind `ForgeSlotStatus { Due, NotDue }` planner input
>   was added. A NEW pure `forge_slot_status(last_forged_slot, current_slot)` monotonic guard (the ONLY fn in
>   the module that observes a `SlotNo`) emits `Due` at most once per `SlotNo` and never for a past slot. These
>   are **CE-not-law additively-evolvable closed planner vocabularies** (like `WalEntry` / chaindb
>   `SCHEMA_VERSION`) — **NOT new plugin/extension seams.** DC-NODE-05.
> - **CLOSED (RED, `ade_node::node_lifecycle`):** NEW `pub struct ForgeActivation<'a>` — the **opt-in
>   forge-activation bundle** threaded into `run_relay_loop` (now `forge: Option<&mut ForgeActivation<'_>>`).
>   When `Some`, the loop's `ForgeTick` branch derives the slot via the **clock seam**, reuses
>   `CoordinatorState::kes_period_for_slot`, and calls **exactly ONE** fenced
>   `node_sync::forge_one_from_recovered`. It advances NO durable tip and serves/admits/gossips nothing.
>   **This is a closed, opt-in activation surface, not an extension point.** _(G-A S3 derives the `ForgeTick`
>   slot via `checked_millis_to_slot`; G-A adds `protocol_version` + `last_slot_alignment_fail` fields; G-B adds
>   the opt-in `handoff_tx`.)_ CN-NODE-02 / DC-SYNC-02 / DC-NODE-05.
> - **Gate evolution (the seam-closure mechanism, NO new gate):** `ci_check_node_run_loop_containment.sh`
>   permits **exactly one fenced `forge_one_from_recovered`** while RETAINING every
>   tip/serve/admit/`run_real_forge`/second-bootstrap prohibition and ADDING no-serve tokens. A **net
>   tightening.** _(G-A / G-B / G-C left this gate byte-/semantically unchanged.)_
>
> **Honest scope (N-F-E, carried into N-F-F / N-F-G-A / N-F-G-B):** relay-only + strictly hermetic; a **live
> unbounded peer** for the relay loop was the **RO-LIVE-01 follow-on** (the live FEED is now wired by N-F-G-C;
> the operator-witnessed live ACCEPT remains the gating follow-on).
>
> ---
>
> **The PHASE4-N-F-C surface stands below, carried forward.** PHASE4-N-F-C wires the single `--mode node`
> Ade node lifecycle and is proven through evidence closure. Its SEAMS deltas were almost entirely surface
> REDUCTIONS — new CLOSED surfaces, not new extension points — and it CLOSED the consume-side seed-epoch
> consensus-input seam that N-F-A had left open.
>
> N-F-C introduced / extended (all NEW surfaces are CLOSED — classified under §3 Closed / §4 Frozen):
>
> - **CLOSED (RED, `ade_node::cli`)** the `Mode` run-mode enum is a **5-variant CLOSED set**
>   `{WireOnly, Admission, KeyGenKes, Produce, Node}` — **no `#[non_exhaustive]`**, and `main.rs` dispatch has
>   **no wildcard arm**. The `Node` variant is the N-F-C addition. Gate: `ci_check_node_mode_closure.sh`.
> - **CLOSED (RED, `ade_node::node_sync`)** `NodeBlockSource` — a **verdict-decoupled** peer-block source:
>   closed 2-variant enum (`WirePump` / `InMemory`) whose `next_block` yields **only** ordered block bytes,
>   skips `TipUpdate`, ends on `Disconnected`. It NEVER derives / surfaces / depends on a verdict. _(N-F-G-C
>   FILLS the `WirePump` arm via `from_wire_pump` from a LIVE `--peer` dial — a fill, NOT a new variant.)_
> - **CLOSED (GREEN, `ade_node::ba02_evidence`)** the BA-02 peer-acceptance evidence vocabulary — closed sums
>   `PeerAcceptEvent` (2), `PeerAcceptSource` (3), `NoEvidenceReason` (4), `BA02Outcome` (2) — plus the
>   **versioned** `Ba02Manifest` (`BA02_MANIFEST_SCHEMA_VERSION = 1`). `correlate` is the **SOLE**
>   `Ba02Manifest` constructor. Gate: `ci_check_ba02_evidence_closed.sh` (+ N-F-G-C
>   `ci_check_ba02_evidence_manifest_schema.sh`). **BA-02 is satisfied NOWHERE.**
> - **CLOSURE of the N-F-A consume-side seam (CN-CINPUT-03 / DC-CINPUT-02b).** The node-lifecycle forge path
>   `node_sync::forge_one_from_recovered` may attach ONLY via the recovered surface — it projects leadership
>   via `PoolDistrView::from_seed_epoch_consensus_inputs` and fails closed when none. Fenced by
>   `ci_check_consensus_input_provenance.sh` guard (d). _(N-F-E/N-F-F/N-F-G-A/N-F-G-B/N-F-G-C carry this fence
>   unchanged.)_
> - **Lifecycle-owner rule (RED, `ade_node::node_lifecycle`)** — THE single `--mode node` recovered-state
>   lifecycle owner (`PHASE4-N-F-C-LIFECYCLE-OWNER`). Both arms route initial state through the single
>   `bootstrap_initial_state` authority. Gates: `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh`
>   (CN-NODE-01) + `ci_check_node_sync_via_pump.sh` (DC-SYNC-01 driver containment).
>
> **What the binary `Node` arm actually runs (precise wiring honesty, updated through N-F-G-C).** `main()`
> routes `Mode::Node → run_node_lifecycle → run_node_lifecycle_inner`. The arm is **fully wired + durable for
> bootstrap + recovery** (FirstRun Mithril bootstrap; WarmStart WAL-replay recovery). Both arms produce the
> **single recovered `BootstrapState`**. **N-F-F forge-intent flip:** the arm calls `classify_forge_intent`
> over flag *presence*. `Off` ⇒ `run_relay_loop(…, None)` (byte-identical to N-F-E relay); `On(paths)` ⇒
> `operator_forge::build_operator_forge_material → coordinator_init → ForgeActivation → run_relay_loop(…,
> Some(&mut activation))`; `PartialKeySet` ⇒ `NodeLifecycleError::ForgeKeyIngress` →
> `EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44`. **N-F-G-A forge constants:** the `On` arm's `ForgeActivation`
> derives `protocol_version` + `pparams` from the recovered current ledger view (S2), ingests config through
> the real `parse_opcert_envelope` + `parse_shelley_genesis` (S2), installs the oracle-bound current pparams
> at seed/import (S2a), derives the `ForgeTick` slot via `checked_millis_to_slot` (S3, before-anchor
> fail-closed), and fails an off-epoch forge closed via `forge_epoch_admission` before leadership (S4).
> **N-F-G-B self-accept→serve handoff:** the `On` arm spawns a **sibling** served-chain admit task draining a
> typed `mpsc<SelfAcceptedHandoff>`; each `ForgeTick`, `forge_one_from_recovered` returns `(CoordinatorEvent,
> Option<SelfAcceptedHandoff>)` and the loop does a best-effort `tx.send(h)` (typed channel send only — never a
> `ServedChainHandle` / `push_atomic` in the loop body). **N-F-G-C live feed (NEW):** the `On` arm wires `let
> mut source = if !cli.peer_addrs.is_empty() { spawn_live_wire_pump_source(&cli.peer_addrs, network_magic,
> state.tip.as_ref()) } else { NodeBlockSource::in_memory(Vec::new()) }` — a LIVE `NodeBlockSource::WirePump`
> from `--peer` (reusing the closed admission dial + pump; bounded `mpsc`, cap 64; recovered-tip or
> `Point::Origin` start). **Empty-source honesty (no-`--peer` arm):** with no `--peer` the empty source halts the
> loop on iteration 1 BEFORE any `SyncOnce` or `ForgeTick`; the `On` arm is **forge-CAPABLE but not observable**
> without a live feed. With a live feed the forge is observable when the feed is Continuing and a due leader slot
> is reached, but **peer ACCEPT stays operator-gated** (RO-LIVE-01 partial). **N-F-G-C BA-02 evidence I/O
> (NEW):** the RED `ade_node::ba02_pass` module is the operator-pass evidence file I/O (`correlate_peer_log_file`
> + `write_ba02_manifest`), reached by no binary arm at this HEAD (tested only). The L6 BA-02 evidence
> correlator (`ba02_evidence`) remains a GREEN library surface, now *consumed* by the RED `ba02_pass` wrapper.
> **BA-02 is satisfied nowhere.**
>
> **Governance note (N-F-C).** The single `bootstrap_initial_state` authority is the sole initial-state path
> on both lifecycle arms (CN-NODE-01); the recovered surface is consumed for leadership ONLY via the closed
> `PoolDistrView::from_seed_epoch_consensus_inputs` projection. The `Mode` sum stays closed with no wildcard
> dispatch. **No BLUE crate was modified by N-F-C / N-F-D / N-F-E / N-F-F / N-F-G-A / N-F-G-B / N-F-G-C** (the
> 456 canonical-type total is unchanged; all this code lands in the RED `ade_node` + RED `ade_runtime` + GREEN
> `ade_testkit`).
>
> ---
>
> **The PHASE4-N-F-A surface stands below, carried forward.** PHASE4-N-F-A is the **recovered seed-epoch
> consensus-input CAPABILITY** cluster (A1–A4) — a closed canonical record with a SOLE codec, persisted as a
> fingerprint-bound sidecar and reconstructable through verified warm-start. It was a capability cluster, NOT
> production wiring; PHASE4-N-F-C wired production consumption.
>
> N-F-A introduced / extended:
>
> - **BLUE** `ade_ledger::seed_consensus_inputs` (NEW, A1) — the closed `SeedEpochConsensusInputs`
>   recovered-state record + its **SOLE** version-gated, byte-canonical codec (`SEED_CINPUT_SCHEMA_VERSION =
>   1`) + the closed 6-variant `SeedConsensusInputsError`. **CN-CINPUT-01**.
> - **BLUE** `ade_ledger::wal` (EXTENDED, A3a) — the **additive** `WalEntry::SeedEpochConsensusInputsImported`
>   variant at append-only **wire tag 3** that does not participate in the `AdmitBlock` fp-chain. **DC-CINPUT-01**.
>   `WalEntry` stays a CE-not-law surface.
> - **BLUE** `ade_ledger::consensus_view` (EXTENDED, A4) — `PoolDistrView::from_seed_epoch_consensus_inputs`
>   (pure field-map). **DC-CINPUT-02a** (enforced; CONSUMED by the node-lifecycle forge path since N-F-C under
>   DC-CINPUT-02b — exercised by the N-F-E forge tick, the N-F-F `On` arm, and the N-F-G-A current-constants
>   forge tick).
> - **GREEN** `ade_runtime::seed_consensus_merge` (NEW, A2) — deterministic no-I/O merge; fail-closed, never a
>   zero-hash fill.
> - **RED** `ade_runtime::{bootstrap, genesis_bootstrap, mithril_bootstrap, seed_consensus_provenance, chaindb}`
>   (EXTENDED, A2/A3) — the warm-start restore + sidecar tail + the anchor-fp-keyed `chaindb` namespace (redb
>   `SCHEMA_VERSION` v2 → **v3**).
> - **NEW CI gate** `ci_check_consensus_input_provenance.sh` (**CN-CINPUT-02**, enforced) — a data-flow-resistant
>   containment gate; the populate-side fence (N-F-C added guard (d), the consume-side fence).
>
> **Four structural decisions remain load-bearing for SEAMS:** (1) the **single `bootstrap_initial_state`
> authority** fronts produce-mode cold-start, the Conway-genesis path, the Mithril provenance path (N-Y), the
> Mithril production-bootstrap composition (N-Z), the N-F-A warm-start restore, AND both N-F-C lifecycle arms
> — **no `GenesisAnchor` / `MithrilAnchor` trait or plugin seam exists**. (2) The **two-driver split** (GREEN
> reducer / RED pump). (3) **`WalEntry` stays a CE-not-law** surface. (4) The **redb `chaindb` `SCHEMA_VERSION`
> is a versioned gate** (v3), not a frozen contract.
>
> **Cluster-doc location.** The PHASE4-N-F-G-C cluster doc + slice docs (S1, S2) live under
> `docs/clusters/PHASE4-N-F-G-C/`; on close they archive under `docs/clusters/completed/`. The predecessor
> **PHASE4-N-F-G-B** set (cluster + S1/S2/S3) is archived under `docs/clusters/completed/PHASE4-N-F-G-B/`; the
> **PHASE4-N-F-G-A** set is archived under `docs/clusters/completed/PHASE4-N-F-G-A/`. Every prior closed
> cluster doc — the **PHASE4-N-F-A / N-F-C / N-F-D / N-F-E / N-F-F / N-F-G-A / N-F-G-B** sets, the entire **N-Q /
> N-R-\* / N-S-\*** set, the **N-M-\*** sub-trees, **N-O**, **N-P**, **N-T**, **N-V**, **N-W**, **N-X**, **N-Y**,
> **N-Z** — is archived under `docs/clusters/completed/`.

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative pipelines. Ade
> is a Cardano node, not a request/response service — its "external surfaces" are the
> N2N/N2C wire, operator-supplied key/genesis/opcert files, the cardano-cli UTxO seed
> dump, the cardano-cli `query protocol-parameters` JSON (N-F-G-A), the Mithril snapshot
> manifest (N-Y), the Mithril production-bootstrap composition (N-Z), the Conway genesis
> file (N-Y), the operator-captured peer-accept JSONL log (BA-02 evidence, N-F-C / wired
> N-F-G-C), and argv. Each reduces to a canonical BLUE type (or a closed evidence outcome)
> before any authoritative transition. There is **no HTTP/gRPC/message-bus ingress**
> (confirmed absent — not a gap).
>
> **N-F-G-C note (live feed + BA-02 evidence I/O):** the `--mode node` `On` arm is now
> **live-feed-wireable** from `--peer`. The same N2N inbound wire surface (below) is the live
> feed: `spawn_live_wire_pump_source` reuses the closed `ade_runtime::admission::{dial_for_admission,
> run_admission_wire_pump}` VERBATIM and feeds their `AdmissionPeerEvent` output into the closed
> `NodeBlockSource::WirePump` arm via `from_wire_pump` (a FILL of the existing arm — NOT a new
> ingress *protocol* surface, NOT a new variant, NOT a new wire authority). It introduces **no new
> reduction target** — the live block bytes traverse the EXACT same pipeline as the N2N inbound wire
> surface. The BA-02 evidence path (operator-captured peer-accept JSONL) reduces through the SOLE
> `correlate` constructor to the closed `BA02Outcome` (now reachable via RED file I/O in `ba02_pass`,
> the only G-C-added I/O); a missing/unreadable file fails closed (`io::Error`), never a synthesized
> acceptance.
>
> **N-F-G-A note:** the `--mode node` forge path sources REAL forge constants. The cardano-cli protocol-
> parameters JSON (the `protocol_params_json` preimage) is ingested by the GREEN `consensus_inputs::protocol_params`
> parser into a canonical BLUE `ProtocolParameters` (no float — exact `Rational` or fail closed), hash-bound to
> the fingerprinted `protocol_params_hash` via the GREEN `require_forge_current_pparams` accessor; opcert + genesis
> ingest through the REAL `parse_opcert_envelope` + `parse_shelley_genesis` (genesis for clock/KES/network
> constants only). The wall-clock that drives the forge tick enters through the existing RED **clock seam** —
> now via the checked `checked_millis_to_slot` (a before-anchor tick fails closed `SlotAlignmentError`, never
> saturates to `start_slot`); only the canonical `SlotNo` crosses into the GREEN planner / the fenced forge
> call. **No external ingress *protocol* surface is added** (it is operator files + argv flags + the oracle's
> pparams JSON preimage carried in the consensus-inputs bundle). The off-epoch guard (`forge_epoch_admission`)
> reduces `(slot, era_schedule, seed_epoch)` to the closed `ForgeEpochAdmission` before leadership / KES signing.

### Surface: N2N inbound wire (received blocks/headers/txs; the LIVE `--mode node` feed since N-F-G-C)

```
Surface: N2N mini-protocol traffic over TCP+mux (RED ade_runtime::network::{n2n_listener, mux_pump, n2n_dialer};
  the --mode node On-arm LIVE feed reuses ade_runtime::admission::{dial_for_admission, run_admission_wire_pump} via
  ade_node::node_lifecycle::spawn_live_wire_pump_source — N-F-G-C)
Reduces to: decoded mini-protocol messages → tag-24-stripped inner bytes → PreservedCbor<T> → DecodedBlock (BLUE ade_codec)
Pipeline (fixed; steps may not be reordered or shortcut):
  1. mux::frame::decode_frame                       (BLUE — single frame-decode authority)
  2. session::core::step                            (GREEN — partial-frame buffer + payload reassembly + closed AcceptedMiniProtocol registry)
  3. per-mini-protocol *_transition reducer         (BLUE — chain_sync / block_fetch / etc.)
  3a. tag-24 strip (N-X)                             (BLUE — decompose_blockfetch_block / decompose_rollforward_header delegate to ade_codec::unwrap_tag24; RED admission::runner / follow call ade_codec::unwrap_tag24 directly — no hand-rolled parse)
  3b. (N-F-G-E) memory bound BEFORE decode           (GREEN session::core: after drain_protocol_items drains every COMPLETE item, an incomplete per-mini-protocol reassembly tail over MAX_REASSEMBLY_TAIL_BYTES = 16 MiB ⇒ SessionError::ReassemblyBufferOverflow ⇒ mux_pump::session_err_to_halt drops the peer — fail-closed, no silent truncation, no partial decode; closed literal const, no CLI/env/config escape hatch. This is a fail-closed BOUND in front of step 4, NOT a new pipeline step. DC-LIVEMEM-01)
  4. ade_codec decode_block_envelope / decode_*     (BLUE — sole PreservedCbor construction site, over the verbatim tag-24-stripped inner bytes)
  5. ade_ledger::receive::reducer / mempool_ingress (BLUE — header→body bridge / wire-ingress chokepoint)
  6. forward_sync::reducer → forward_sync::pump (N-Y)  (GREEN admit-plan over the BLUE admit chokepoint → RED durability-ordered driver; AdvanceTip only after StoreBlockBytes + AppendWal ack)
  7. block_validity / tx_validity / admission        (BLUE verdict; GREEN admission compares already-authoritative outputs)
N-F-G-C LIVE-FEED PATH (the --mode node On arm; CONSUME side of the prior §7 candidate, now wired):
  L1. spawn_live_wire_pump_source(&cli.peer_addrs, network_magic, recovered_tip)  (RED — REUSES dial_for_admission + run_admission_wire_pump VERBATIM; build_n2n_version_table reused via pub(crate); start_point = recovered tip Point::Block or Point::Origin)
  L2. per --peer: tokio::spawn { dial_for_admission(addr) → run_admission_wire_pump(...) → bounded mpsc::channel(LIVE_WIRE_PUMP_CHANNEL_CAP = 64) }  (an unparseable addr / dial failure is logged-and-dropped — C3 honest-scope; non-fatal; no fabricated addr, no tip graft)
  L2a. (N-F-G-E) pump_lookahead depth cap            (RED node_sync: the opportunistic try_recv drain stops at MAX_WIRE_PUMP_LOOKAHEAD = 256, so the bounded mpsc (cap 64) back-pressures the pump; content-blind; the closed 2-variant NodeBlockSource + arrival order are unchanged; closed literal const, no CLI/env/config escape hatch — DC-LIVEMEM-01)
  L3. NodeBlockSource::from_wire_pump(rx)            (RED — a FILL of the closed WirePump arm; NOT a new variant; yields ordered Block bytes ONLY, skips TipUpdate, ends on Disconnected — DC-SYNC-01)
  L4. → run_node_sync → pump_block                  (the SAME single durable tip-advance — step 6 above; no second tip-advance, no verdict)
Cross-surface state sharing: the served ServedChainSnapshot (read by both serve and broadcast paths);
  the per-peer outbound map (PerPeerOutbound) is keyed by PeerId — no cross-peer byte leakage.
  The tag-24 unwrap step (N-X) is the SAME shared ade_codec authority used by the serve path's wrap step.
  The forward-sync persisted ChainDb + FileWalStore are the same stores the recovery path (recovery::restart)
  reconciles on warm-start (DC-WAL-*; WalTailFingerprintMismatch fail-fast). N-F-C: the SAME stores the
  --mode node lifecycle (node_lifecycle + node_sync) opens; pump_block gains its FIRST production caller
  in node_sync::run_node_sync. N-F-D: the relay loop (run_relay_loop) is the live-run owner that drives
  run_node_sync each iteration. N-F-G-C: the --mode node On arm now feeds run_node_sync from a LIVE
  NodeBlockSource::WirePump (reusing the admission dial + pump) when --peer is supplied — the live feed is
  the SAME N2N surface; with no --peer the empty source halts the loop before any SyncOnce/ForgeTick. The WAL
  is also where the N-F-A additive SeedEpochConsensusInputsImported (tag 3) provenance lives — disjoint from
  the AdmitBlock fp-chain.
Rule (N-F-G-C live feed; ci_check_served_chain_handoff_fence.sh BROADENED + ci_check_node_run_loop_containment.sh
  byte-unchanged): the live feed is a REUSE of the closed dial + pump (no new wire authority, no reimpl); the
  source stays the closed 2-variant NodeBlockSource (a from_wire_pump FILL, NOT a new variant); the tip advances
  ONLY via run_node_sync → pump_block (no second tip-advance); a dial/parse failure is logged-and-dropped, never
  fatal / fabricated / a tip graft.
HONEST SCOPE: G-C wires the MECHANICAL live feed ONLY. With a live feed the forge is observable when the feed is
  Continuing and a due leader slot is reached, but peer ACCEPT is NOT claimed here — it is operator-gated
  (RO-LIVE-01 partial / blocked_until_operator_stake_available), proven only by an operator-captured peer log
  through ba02_evidence::correlate. Ade self-accept / served-block / wire success != peer acceptance. BA-02 is
  satisfied nowhere.
N-F-G-E (bounded memory before decode/apply): the two caps above (reassembly tail + lookahead depth) bound
  peer-driven memory on this live feed BEFORE authoritative decode/apply — a NARROW claim. NOT full network DoS
  resistance, NOT peer fairness, NOT BA-02/live-evidence readiness. Precision: the reassembly check is post-extend
  (single-buffer transient peak cap + one <=64 KiB frame ~16.06 MiB); ProtoBuffers holds ~10 INDEPENDENT
  per-protocol buffers each capped (per-connection aggregate ~10x single-buffer, still O(constant)/connection).
  Per-connection-COUNT / peer-fairness limits are a SEPARATE out-of-scope surface.
OUTBOUND INVERSE (N-AB, CN-SESS-05): the SEND direction of step 2 mirrors this inbound reassembly. The GREEN
  session::core::handle_outbound SEGMENTS an outbound mini-protocol payload larger than the mux SDU limit
  MAX_PAYLOAD (= u16::MAX = 65535) into ordered <= MAX_PAYLOAD frames via the SAME single encode_inner_frame ->
  mux::frame::encode_frame per-frame authority (each chunk satisfies encode_inner_frame's strict MAX_PAYLOAD
  guard), so a >64 KiB Conway block-fetch Block can be sent; every segment keeps the SAME mini-protocol id + mode
  + the SAME single captured timestamp input (GREEN — no per-segment clock); concat(segment payloads) == payload
  exactly (byte-preserving, lossless, ordered — DC-CONS-17), so Ade's own inbound CN-SESS-04 reassembly (and a
  conformant peer) reconstructs the original. A payload above the FIXED, non-configurable MAX_OUTBOUND_PAYLOAD_BYTES
  = 16 MiB (symmetric with the inbound MAX_REASSEMBLY_TAIL_BYTES, DC-LIVEMEM-01) fails closed BEFORE any
  segmentation (SessionError::OutboundPayloadTooLarge — the existing closed variant, reused, NOT added). This is
  the OUTBOUND inverse of the inbound reassembly above, NOT a new pipeline step / encoder / closed enum; no
  truncation / drop / reorder / partial-success send. NO BLUE change, NO RO-LIVE flip (RO-LIVE-01 stays partial).
SERVE-SIDE FINDINTERSECT (N-AE.E, DC-PROTO-10): on the producer SERVE direction (the inverse of this feed), the BLUE
  ade_network::chain_sync::server::producer_chain_sync_serve FindIntersect handler now sets its read cursor
  last_announced TO the resolved intersect point after replying IntersectFound, so the next RequestNext serves
  next_after(intersect) (the successor the client rolls onto), never next_after(None) = block 0 (which a client
  intersecting at its own tip rejected as UnexpectedBlockNo(tip+1)(0)). last_announced = Block -> Some((slot,hash))
  | Origin -> None (Origin keeps serve-from-start, correct). This is a server-INTERNAL cursor-state fix — the
  closed Point enum + the ChainSyncMessage grammar (FindIntersect/IntersectFound/RequestNext/RollForward) are
  UNCHANGED, NOT a new pipeline step / message / negotiated surface. The recovered-anchor INTERSECTABILITY it
  composes with (AE.B, DC-NODE-14) is recorded under the serve-projection surface + §3 (ChainDbServedSource).
```

### Surface: operator-captured peer-accept log → BA-02 evidence (RED file I/O wired N-F-G-C over the GREEN N-F-C correlator)

```
Surface: operator-captured peer-accept JSONL log file (RED ade_node::ba02_pass over GREEN ade_node::ba02_evidence)
Reduces to: closed PeerAcceptEvent set → BA02Outcome {Ba02Manifest | NoEvidence} via correlate (the SOLE Ba02Manifest ctor)
Pipeline (RED file read → allow-list parse → exact-match correlate; fail-closed on a missing/unreadable file):
  1. correlate_peer_log_file(ade, peer_log_path)        (RED ba02_pass — std::fs read; a missing/unreadable file ⇒ io::Error, NEVER a NoEvidence and NEVER a manifest)
  2. parse_peer_accept_events                            (GREEN ba02_evidence — ALLOW-LIST: only `peer_served_block` / `peer_chain_tip` discriminators → PeerAcceptEvent; every weaker/unknown/malformed line DROPPED, never coerced)
  3. AdeForgeRecord (forged hash + slot)                 (read VERBATIM from the BLUE-minted forge record — NEVER recomputed; no new BLUE authority)
  4. correlate(ade, peer_log)                            (GREEN — pure/total/deterministic, HASH-PRIMARY; emits BA02Outcome::Ba02Manifest ONLY on an exact forged-hash↔peer-accept match at the matching chain point, no conflicting signal; else NoEvidence{reason})
  5. write_ba02_manifest(&Ba02Manifest, out)             (RED ba02_pass — the ARGUMENT TYPE is the gate: it accepts ONLY a Ba02Manifest, which only correlate's exact-match arm constructs; there is NO path emitting a manifest from a NoEvidence outcome or from raw operator input, so a written manifest is ALWAYS correlate-produced)
Cross-surface state sharing: NONE. GREEN evidence comparing already-authoritative outputs; forges nothing,
  admits nothing, persists no node state. ba02_pass is RED file I/O ONLY — it constructs no evidence, derives no
  acceptance, never coerces a non-acceptance line. A Ba02Manifest is a CLAIM ABOUT authority, not authority.
Rule (RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01, ci_check_ba02_evidence_closed.sh + ci_check_ba02_evidence_manifest_schema.sh):
  correlate is the ONLY Ba02Manifest constructor; write_ba02_manifest accepts ONLY a Ba02Manifest; NO self-evidence
  token (ForgeSucceeded / self_accept / block_received / served-block / wire-success / agreement_verdict / "agreed")
  may be an acceptance source; NO committed synthetic manifest. The new schema gate verifies the closed 8-field
  manifest + schema_version == 1 + peer_log_file_sha256 == sha256(committed fixture) WHEN a CE-G-C-LIVE_*.toml is
  committed, and is VACUOUSLY satisfied when none is (the typical state). The versioned Ba02Manifest
  (BA02_MANIFEST_SCHEMA_VERSION = 1) is a version-GATED contract (§4).
HONEST SCOPE (N-F-G-C): G-C wires the BA-02 evidence I/O (ba02_pass) but is reached by NO binary arm at this HEAD
  (tested only — incl. the env-gated node_operator_pass_ba02_live). A real BA-02 result needs a REAL
  operator-captured peer log naming the EXACT Ade-forged hash, through correlate, against a peer that can grant
  leadership (C1 private testnet or C2 preprod with provisioned stake). BA-02 is satisfied NOWHERE at this HEAD;
  RO-LIVE-01 stays partial / operator-gated.
```

### Surface: --mode node forge constants (current protocol-parameters source + checked clock + off-epoch guard, N-F-G-A)

```
Surface: --mode node forge-constant fidelity (GREEN ade_runtime::consensus_inputs::{protocol_params, canonical} + GREEN ade_runtime::clock::checked_millis_to_slot + GREEN-by-fn ade_node::node_sync::forge_epoch_admission)
Reduces to: oracle pparams JSON preimage → current ProtocolParameters (hash-bound) + a checked SlotNo (before-anchor fails closed) + a ForgeEpochAdmission verdict (off-epoch fails closed before leadership)
Pipeline (fixed; the forge constants are sourced REAL, hash-bound, and the two boundaries fail closed inside the existing forge fence):
  S2a-1. require_forge_current_pparams()             (GREEN — protocol_params_json carried OUTSIDE the frozen 15-field fingerprint; preimage absent ⇒ ForgeCurrentPParamsError::PreimageAbsent)
  S2a-2. blake2b_256 bind                            (GREEN — blake2b_256(protocol_params_json) != fingerprinted protocol_params_hash ⇒ BindMismatch)
  S2a-3. parse_protocol_parameters_json(json, magic) (GREEN — exact decimal/scientific → ade_ledger::rational::Rational by INTEGER arithmetic; no f64/serde-float; a non-exact literal ⇒ ProtocolParamsParseError::InexactRational; bad shape ⇒ JsonShape; wrapped as ForgeCurrentPParamsError::Parse)
  S2a-4. install at seed/import                      (RED admission::{seed_to_snapshot, bootstrap} — build_seed_ledger installs the CURRENT ProtocolParameters into the recovered ledger, never ProtocolParameters::default(); warm-start preserves it)
  S2-5.  operator config ingress                     (RED operator_forge — real parse_opcert_envelope + parse_shelley_genesis; parse_simple_* retired on the node path; genesis ⇒ clock/KES/network constants ONLY)
  S3-6.  checked_millis_to_slot(tick, start, slot0, len)  (GREEN — before-anchor tick (tick < start) ⇒ SlotAlignmentError::BeforeGenesisAnchor recorded in last_slot_alignment_fail; otherwise the EXACT millis_to_slot result; no float, no wall-clock; saturating millis_to_slot left intact for non-forge callers)
  S4-7.  forge_epoch_admission(slot, era_schedule, seed_epoch)  (GREEN-by-fn — via the BLUE EraSchedule::locate; within seed epoch ⇒ WithinSeedEpoch; any other / unlocatable ⇒ OffEpoch; called BEFORE query_leader_schedule, so an off-epoch forge fails closed before leadership / KES signing; drives NO NonceInput::EpochBoundary / CandidateFreeze nonce promotion)
  8.     (off-epoch outcome)                          (routes through the EXISTING closed 9-variant CoordinatorEvent::ForgeNotLeader — NO new variant added)
Cross-surface state sharing: the recovered current ProtocolParameters installed at seed/import is the SAME
  parameter record the warm-start recovery preserves and the On-arm ForgeActivation carries (protocol_version +
  pparams). The checked clock seam is the SAME RED clock seam the N-F-E/N-F-F forge tick uses (only a SlotNo
  crosses; SystemTime/Instant/float never cross into GREEN/BLUE). The off-epoch guard reuses the BLUE
  EraSchedule::locate the rest of the consensus core uses. NONE of these boundaries advance a tip, serve,
  admit, or gossip.
Rule (DC-EPOCH-03 / CE-G-A-1/2/2a/4; ci_check_recovered_ledger_pparams_sourced.sh + ci_check_node_forge_real_cli_ingress.sh + ci_check_node_forge_single_epoch_fail_closed.sh + ci_check_genesis_consistency_fixture_present.sh):
  the recovered ledger pparams are sourced from the oracle preimage (never defaulted/genesis-initial); the
  config ingress uses the REAL parsers (no parse_simple_* on the node path); the off-epoch guard runs BEFORE
  leadership via EraSchedule::locate (no fabricated epoch, no nonce promotion); protocol_params has NO float
  path (exact Rational or fail closed); checked_millis_to_slot MUST NOT saturate a before-anchor tick (fail
  closed); the protocol_params_json preimage stays OUTSIDE the 15-field canonical fingerprint. run_relay_loop's
  containment is SEMANTICALLY UNCHANGED — the new boundaries sit inside the existing forge fence.
HONEST SCOPE: this is forge-fidelity HARDENING — real config, current pparams, two fail-closed boundaries. It
  does NOT serve / admit / gossip / advance a durable tip; the forge stays subordinate + self-accept-only; on
  the empty binary source the loop halts before any ForgeTick (forge-CAPABLE, NOT observable — RO-LIVE-01).
  The S1 genesis-consistency fixture is evidence input, never runtime authority. BA-02 satisfied nowhere.
```

### Surface: --mode node operator-key ingress (the forge-intent classifier + operator-material site, N-F-F; real parsers N-F-G-A)

```
Surface: operator-key CLI flags for --mode node (GREEN ade_node::forge_intent + RED ade_node::operator_forge)
Reduces to: ForgeIntent {On(ForgePaths) | Off} (presence classification) → OperatorForgeMaterial (custody shell + GenesisAnchor + pool_id + clock anchors) → ForgeActivation
Pipeline (fixed; presence is classified PURELY first, then a single named RED ingress site, then activation assembly):
  1. GREEN classify_forge_intent(cold, kes, vrf, opcert, genesis: Option<&Path>)  (pure/total over all 2^5 flag-PRESENCE combinations; never observes file contents; all five present ⇒ On(ForgePaths); none ⇒ Off; any partial subset ⇒ Err(ForgeIntentError::PartialKeySet{present, missing}) — static flag-name strings only)
  2a. Off  → run_relay_loop(…, None)                 (the EXACT N-F-E relay, verbatim — keys absent ⇒ exact relay-only; the planner is fed NotDue and never returns ForgeTick)
  2b. PartialKeySet → NodeLifecycleError::ForgeKeyIngress → EXIT_NODE_FORGE_KEY_INGRESS_FAILED = 44  (partial ⇒ fail closed; never a silent relay fallback, never a missing/zero/fabricated key)
  2c. On(paths) → operator_forge::build_operator_forge_material(&paths)  (the single named RED operator-material ingress site)
       i.   load_operator_producer_shell(&paths)      (RED — reuses load_cold_signing_key_skey / load_vrf_signing_key_skey / load_kes_skey_any_format; no reimpl)
       ii.  BLUE structural validators                (KES via ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes — byte layout IS the validator; ProducerShell::init enforces the opcert shape + KES-period-vs-opcert freshness bound, CN-PROD-02)
       iii. pool_id = Hash28(blake2b_224(cold_vk))     (the ONE named derivation — never fabricated)
       iv.  (N-F-G-A S2) REAL parse_opcert_envelope + parse_shelley_genesis  (the parse_simple_* stubs are RETIRED on the node path; genesis extracts clock/KES/network constants ONLY — NOT a bootstrap source, NOT a new semantic genesis authority)
       v.   (N-F-G-A S2) protocol_version + pparams from the recovered CURRENT ledger view  (current_forge_constants_from_recovered — NO LONGER produce-path ProtocolParameters::default() defaults)
  3. coordinator_init → ForgeActivation → run_relay_loop(…, Some(&mut activation))  (keys present ⇒ forge-capable, built on the single recovered BootstrapState; N-F-G-C: the relay loop's source is a LIVE NodeBlockSource::WirePump when --peer is supplied)
Cross-surface state sharing: the OperatorForgeMaterial.shell (ProducerShell) is RED-confined key custody —
  OperatorForgeMaterial is NOT Debug/Serialize; no byte accessor / serialization / logging; the private
  KES/VRF/cold material is passed only to the fenced forge handoff (forge_one_from_recovered) and NEVER
  copied into the GREEN CoordinatorState, the planner, any node/loop state, or any persisted/logged/
  hashed-for-evidence/replay surface. The forge base is the SAME single recovered BootstrapState that
  seeds the relay spine (no second bootstrap — CN-NODE-01; the recovered state outlives both the
  ForwardSyncState and the ForgeActivation). pool_id + the genesis public anchors feed coordinator_init.
Rule (CN-NODE-03, ci_check_forge_intent_closed.sh + ci_check_operator_forge_no_secret_leak.sh + ci_check_node_forge_real_cli_ingress.sh):
  ingress is STRICTLY RED-parse → BLUE-structural-validator → canonical-type, reusing the existing loaders +
  (N-F-G-A) the REAL opcert/genesis parsers — NO new BLUE authority, NO parser reimpl, NO plugin/trait seam,
  NO second forge codepath, NO new BLUE crate change. ForgeIntent is the closed two-variant set (no third
  "partial" variant); classify_forge_intent is the sole entry, binding the partial arm by name (no wildcard).
  The N-F-E forge containment gate stays SEMANTICALLY UNCHANGED (still exactly one fenced
  forge_one_from_recovered; no run_real_forge / serve / admit / gossip / broadcast / block-fetch / durable-tip
  mutation) — N-F-F/N-F-G-A/N-F-G-B/N-F-G-C may ADD ingress / fidelity / handoff / live-feed gates but MUST NOT
  relax forge containment.
HONEST SCOPE: this is operator-key INGRESS + activation wiring + (N-F-G-A) real config ingress — it makes the
  binary forge-CAPABLE with real keys + real constants. N-F-G-C makes the On arm live-feed-wireable (--peer);
  with a live feed the forge becomes observable at a due leader slot, but peer ACCEPT stays operator-gated
  (RO-LIVE-01). On the empty-source binary path the loop still halts before any ForgeTick. NO serve / admit /
  gossip / durable-tip / BA-02 / RO-LIVE acceptance claim. Mithril is untouched (bootstrap accelerator, NOT a
  forge/validation shortcut).
```

### Surface: --mode node relay run-loop (the live-run owner, N-F-D; forge-tick N-F-E; forge-on flip N-F-F; checked clock + off-epoch guard N-F-G-A; self-accept→serve handoff N-F-G-B; live feed N-F-G-C; forge-on-followed-tip admissibility N-AE)

```
Surface: --mode node relay run loop (RED ade_node::node_lifecycle::run_relay_loop; the single live-run owner)
Reduces to: per-iteration step over the closed GREEN LoopStep vocabulary; the tip advances ONLY via run_node_sync → pump_block
Pipeline (fixed; GREEN plans the iteration, RED performs effects, BLUE authority stays behind the seams):
  0. (entry) both --mode node arms converge here     (FirstRun bootstrap + WarmStart recovery → run_relay_loop; no print-and-exit. N-F-F: the arm classifies forge intent FIRST — Off ⇒ forge:None; On ⇒ forge:Some(operator-material ForgeActivation with N-F-G-A current pparams + protocol_version); PartialKeySet ⇒ fail closed. N-F-G-C: the source is a LIVE NodeBlockSource::WirePump (spawn_live_wire_pump_source) when --peer is supplied, else the empty in_memory source)
  1. (top of iteration) reset pending_slot = None     (N-F-E: a skipped/failed path can never forge for a stale slot)
  1a. (forge-on path only) clock seam → SlotNo         (N-F-E: RED Clock::next_tick → millis_to_slot → SlotNo; N-F-G-A S3: now via checked_millis_to_slot — a before-anchor tick fails closed SlotAlignmentError::BeforeGenesisAnchor instead of saturating to start_slot; only SlotNo crosses; DC-NODE-03/DC-NODE-05)
  1b. (forge-on path only) forge_slot_status guard      (N-F-E: GREEN pure monotonic guard → ForgeSlotStatus {Due|NotDue}; at most once per SlotNo, never a past slot)
  2. GREEN plan_loop_step(loop_state, sync_status, forge_slot_status, shutdown)  (→ closed LoopStep {SyncOnce | ForgeTick | Idle | HaltCleanly}; content-blind; total table)
  3a. SyncOnce  → run_node_sync → pump_block            (DC-SYNC-01/DC-SYNC-02: the SOLE durable tip-advance; durable-before-advance; UNMODIFIED since N-F-C; N-F-G-C feeds it from the LIVE WirePump source)
  3b. ForgeTick → exactly one forge_one_from_recovered  (N-F-E: reuses kes_period_for_slot; N-F-G-A S4: forge_epoch_admission runs BEFORE query_leader_schedule — an off-epoch slot fails closed before leadership/KES signing; recovered-surface leadership; advances NO durable tip; serves/admits/gossips NOTHING in the loop body; updates last_forged_slot only on a real attempt; records into in-memory hermetic_forge_outcomes. N-F-G-J: the arm derives (block_number, prev_hash) via the GREEN node_sync::forge_header_position(selected_tip.as_ref()) — the ONE cold-start convention: None => (0, PrevHash::Genesis); Some => (last_block_no+1, PrevHash::Block(tip.hash)); a tip-without-height edge fails closed NodeForgeError::RecoveredTipMissingBlockNo — never .unwrap_or(1). When ChainDb::tip() AND the recovered tip are BOTH None, the GREEN node_lifecycle::may_cold_start_forge(selected_tip_present=false, has_recovered_lineage, feed_eligible) permits forging block 0 + PrevHash::Genesis through the SAME run_real_forge → self_accept → SelfAcceptedHandoff path S3 proved — DC-NODE-08; the forge holds NO ChainDb handle, advances NO durable tip — scoped to the hermetic cold-start, no genesis_forged latch. N-AE: a forge-admissibility gate (a GREEN-by-fn classifier sibling to forge_epoch_admission, computed BEFORE leadership) now refuses the forge unless durable_servable_tip.{hash,block_no} == followed_peer_tip.{hash,block_no} — fail-closed to a typed ForgeRefused::NotCaughtUp {local_servable_tip, followed_peer_tip, reason}; the recovered.tip fallback as a forge base is REMOVED; the forged successor's prev_hash byte-equals the followed peer tip hash and block_no == followed_tip.block_no + 1 — DC-NODE-15 + DC-CONS-24; the followed-peer-tip signal is a structured admissibility input (FollowedPeerTipSignal), NEVER a chain selector — it never reaches select_best_chain/chain_selector)
  3b''. (N-F-G-J) run_relay_loop_with_sched emits the closed GREEN NodeSchedEvent diagnostics to stderr around the planner call + the LoopStep arms  (EMIT-ONLY — records the feed/forge scheduling decision WITHOUT altering it; FeedReason {NoBlockAvailable | CleanEmpty | UnknownDisconnected}; ForgeOutcome {Succeeded | NotLeader | Failed | NoTipAvailable}; the planner never names the vocabulary — ci_check_node_sched_events_emit_only.sh, CN-NODE-04)
  3b'. (N-F-G-B) forge_one_from_recovered now returns (CoordinatorEvent, Option<SelfAcceptedHandoff>)  (RED — Some iff ForgeSucceeded; the loop does a best-effort TYPED tx.send(h) to the sibling served-chain admit task — a typed channel send ONLY; the loop body holds ONLY the mpsc Sender, never a ServedChainHandle / push_atomic / served_chain_admit, so ci_check_node_run_loop_containment.sh stays byte-unchanged)
  3c. Idle      → cancellation-safe wait                (select on source-readiness or shutdown; the only branch that awaits across a cancellation boundary)
  3d. HaltCleanly → exit, on-disk state recoverable
Cross-surface state sharing: shares the persistent ChainDb + FileWalStore with the forward-sync pump +
  warm-start restore; ForgeActivation borrows the recovered BootstrapState (the SOLE leadership source),
  the CoordinatorState (genesis-anchor host for the REUSED kes_period_for_slot), the ProducerShell
  (key custody — hermetic/fenced at N-F-E; REAL operator material at N-F-F; current pparams/protocol_version
  at N-F-G-A), and the Clock seam (checked at N-F-G-A). The forge tick shares NO served-chain / outbound map
  IN THE LOOP BODY. N-F-G-B: ForgeActivation gains an opt-in handoff_tx: Option<mpsc::UnboundedSender<
  SelfAcceptedHandoff>> (set via with_handoff_sender); the dispatcher On arm tokio::spawns a SIBLING served-chain
  admit task that drains the typed channel and admits via the single ServedChainHandle::push_atomic(into_accepted())
  — the served-chain mutation lives in the SIBLING task, NEVER in the loop body. N-F-G-C: the source handed in is
  a LIVE NodeBlockSource::WirePump fed by spawn_live_wire_pump_source (reusing the closed admission dial + pump)
  when --peer is supplied — the SAME N2N inbound surface; the source is still the closed 2-variant enum (a fill,
  not a new variant); the durable tip still advances ONLY via run_node_sync → pump_block.
Rule (CN-NODE-02 / DC-SYNC-02 / DC-NODE-05 / DC-EPOCH-03 / DC-NODE-06 / DC-NODE-15 / DC-CONS-24, ci_check_node_run_loop_containment.sh + ci_check_loop_planner_closed.sh + ci_check_node_forge_single_epoch_fail_closed.sh + ci_check_served_chain_handoff_fence.sh + ci_check_forge_followed_tip_admission.sh):
  the relay-loop body advances the tip ONLY via run_node_sync; references NO run_real_forge / correlate( /
  Ba02Manifest / second-bootstrap path, NO direct manual tip-mutation token, and (N-F-E) NO serve token
  (served_chain_admit / push_atomic / OutboundCommand / broadcast / block_fetch); it may have EXACTLY ONE
  fenced forge_one_from_recovered( call (CE-E-4). The GREEN planner emits only the closed LoopStep
  vocabulary and cannot express an authority decision; SlotNo is observable ONLY in the pure
  forge_slot_status guard (banned in plan_loop_step). N-F-G-A: the forge tick derives its slot via the checked
  guard (before-anchor fail-closed) and fails an off-epoch slot closed (before leadership) — run_relay_loop's
  body containment is SEMANTICALLY UNCHANGED (the new boundaries sit inside the existing fence). N-F-G-B: the
  loop body forwards the surfaced Option<SelfAcceptedHandoff> as a TYPED mpsc tx.send(h) ONLY — the served-chain
  push_atomic / served_chain_admit live in the SIBLING task, so the loop body still references NO serve token and
  ci_check_node_run_loop_containment.sh is byte-/semantically UNCHANGED. N-F-G-C: the live feed enters as the
  closed NodeBlockSource handed to run_relay_loop (a from_wire_pump FILL) — the run loop body is UNCHANGED (the
  source-selection happens in the dispatcher before the loop); ci_check_node_run_loop_containment.sh is
  byte-unchanged. The sibling serve ingress is fenced by the BROADENED ci_check_served_chain_handoff_fence.sh
  (owners {node_lifecycle.rs, node_sync.rs}; guard-3 allow-list: every node-spine unbounded handoff channel
  MUST carry SelfAcceptedHandoff; every push_atomic( fed by into_accepted(); no direct served_chain_admit(). N-AE: the ForgeTick gate is a TIGHTENING inside the existing fence (it can only PREVENT a forge, never select a chain) — ci_check_forge_followed_tip_admission.sh fences no recovered.tip forge-base fallback, the typed ForgeRefused::NotCaughtUp, and that the peer-tip signal never reaches select_best_chain; the relay-loop body containment + the closed LoopStep planner are byte-/semantically UNCHANGED.
HONEST SCOPE: N-F-G-C wires the MECHANICAL live feed; the binary is forge-CAPABLE with real keys + real
  constants, and with a LIVE --peer feed the forge becomes observable at a due leader slot — but peer ACCEPT is
  operator-gated (RO-LIVE-01 partial). With no --peer the empty source halts before any ForgeTick. No BA-02 /
  serve-to-peer / gossip / durable-forge claim.
```

### Surface: --mode node self-accept→serve handoff (the typed handoff seam, N-F-G-B) — MECHANISM SUPERSEDED on the `--mode node` spine by N-U S3 / DC-NODE-13

> **SUPERSEDED on the `--mode node` spine (N-U S3 / DC-NODE-13, repoint `60deecf3`).** The serve-handoff
> MECHANISM described in this block — surfacing the self-accepted `AcceptedBlock` → `SelfAcceptedHandoff` carrier →
> typed channel → sibling task → `ServedChainHandle::push_atomic` — is **RETIRED on the `--mode node` spine.** N-U
> S3 replaced it with **serve-as-projection of the durable ChainDb** (`ServedChainSource::DurableChainDb`; see the
> §1 *--mode node lifecycle* / §2 *forged-block serving* N-U notes): the node spine no longer feeds an in-memory
> accumulator — it serves a READ-ONLY projection of the durable chain, whose bytes entered ONLY through the
> validated durable admit (`pump_block`, DC-NODE-12) + the trusted seed (`bootstrap_initial_state`). The forge's
> own block reaches the served view by becoming DURABLE (`admit_forged_block_durably` → `pump_block`), NOT by a
> serve handoff. The handoff-fence gate `ci_check_served_chain_handoff_fence.sh` is REPOINTED accordingly (see §5).
> **The `SelfAcceptedHandoff` carrier is RETAINED for `--mode produce` (CN-PROD-04, `ServedChainSource::Snapshot`)
> — a SEPARATE serve authority.** The block below is preserved as the historical N-F-G-B mechanism; the
> CURRENT `--mode node` serve provenance is durable-projection.

```
Surface: self-accept→serve handoff on the --mode node spine (GREEN ade_runtime::producer::self_accepted_handoff::SelfAcceptedHandoff + RED ade_node::{node_sync::forge_one_from_recovered, node_lifecycle sibling serve task})
Reduces to: a BLUE self-accepted AcceptedBlock (the ORIGINAL token from self_accept) → typed SelfAcceptedHandoff carrier → typed mpsc<SelfAcceptedHandoff> channel send → sibling task → the single ServedChainHandle::push_atomic(into_accepted())
Pipeline (fixed; the forge SURFACES the BLUE token, the loop FORWARDS a typed channel send, the SIBLING admits via the single push_atomic — the loop body never mutates the served chain):
  1. BLUE self_accept                                  (inside run_real_forge — the gate; no ForgeSucceeded without an Accepted AcceptedBlock; CN-PROD-04)
  2. run_real_forge → (CoordinatorEvent, Option<AcceptedBlock>)  (RED — thin wrapper + byte-identical run_real_forge_inner writes the BLUE token into the out-param ONLY on success; Some iff ForgeSucceeded; every fail-closed branch None; the closed CoordinatorEvent surface is UNCHANGED — no new variant)
  3. forge_one_from_recovered → (CoordinatorEvent, Option<SelfAcceptedHandoff>)  (RED — wraps via self_accepted.map(SelfAcceptedHandoff::from_self_accepted); the ORIGINAL token, NEVER re-derived from artifact.bytes — CN-FORGE-01)
  4. relay-loop body: best-effort tx.send(h)           (RED node_lifecycle — a TYPED channel send only; the loop holds ONLY the Sender — never a ServedChainHandle, never push_atomic, never a served-chain mutation; ci_check_node_run_loop_containment.sh stays byte-unchanged)
  5. sibling served-chain admit task (tokio::spawn)    (RED node_lifecycle On arm — drains the typed mpsc<SelfAcceptedHandoff> and calls the SINGLE node-spine ServedChainHandle::push_atomic(handoff.into_accepted()))
  6. BLUE served_chain_admit (inside push_atomic)      (the SOLE entry into the served index; only self-accepted blocks — CN-PROD-04; push_atomic wraps it in watch::Sender::send_modify, no torn snapshot)
Cross-surface state sharing: the ServedChainHandle is the SAME single served-admit authority the produce-mode
  forge→serve path uses (CN-PROD-04 / N-R-B/N-T); the served chain is mutated ONLY via push_atomic. The handoff
  carrier wraps the SAME BLUE AcceptedBlock minted by self_accept. N-F-G-C: a self-accepted forged block is now
  served byte-identically over IN-PROCESS block-fetch on the live-feed path
  (live_feed_forge_serve_loopback_returns_forged_block); a served block leaving the node OVER THE WIRE to a peer
  that ACCEPTS it is the operator-gated RO-LIVE-01 leg.
Rule (DC-NODE-06, ci_check_served_chain_handoff_fence.sh [BROADENED, G-C] + ci_check_node_run_loop_containment.sh):
  (1) every node-spine push_atomic( is fed by into_accepted(); (2) NO direct served_chain_admit( on the node
  spine (served-chain mutation only via the single push_atomic); (3) [G-C ALLOW-LIST] every node-spine unbounded
  handoff channel MUST carry SelfAcceptedHandoff (never <Vec<u8>> / <ForgedBlockArtifact> / <bool>), and at least
  one UnboundedSender<SelfAcceptedHandoff> must be present. SelfAcceptedHandoff's SOLE constructor takes a BLUE
  AcceptedBlock (private field) — there is NO raw-bytes / artifact / event / flag / verdict constructor, so a
  non-self-accepted artifact is type-unrepresentable as a handoff. The token is the ORIGINAL from self_accept
  (CN-FORGE-01), never re-derived from artifact.bytes. The relay-loop body forwards a TYPED channel send only —
  ci_check_node_run_loop_containment.sh is byte-/semantically UNCHANGED.
HONEST SCOPE: this is SERVE-AUTHORITY (G-B); N-F-G-C adds the LIVE FEED around it. NO peer-acceptance / BA-02 /
  RO-LIVE acceptance claim (peer ACCEPT stays operator-gated; RO-LIVE-01 partial). BA-02 satisfied nowhere.
```

### Surface: producer-mode forge → serve → broadcast (the live producer half)

```
Surface: --mode produce slot loop (RED ade_node::produce_mode + GREEN producer::coordinator)
Reduces to: ForgedBlock → AcceptedBlock (BLUE self_accept) → ServedChainSnapshot → tag-24-wrapped wire bytes
Pipeline (fixed; the BLUE-then-RED-then-BLUE composition of run_real_forge):
  1. bootstrap_initial_state                        (RED/GREEN — sole forge-state source; N-T; fronts genesis/Mithril cold-start, N-Y/N-Z; produce_mode passes SeedEpochConsensusSource::NotRequired — N-F-A)
  1a. era guard (N-W)                                (RED — non-Praos era fail-closes to ForgeFailureReason::UnsupportedProducerEra before any forge)
  2. RED vrf_prove over expected_vrf_input.alpha_bytes()  (operator VRF key; alpha comes from the BLUE LeaderScheduleAnswer — no RED-side era dispatch; N-W)
  3. BLUE verify_and_evaluate_leader(era, …) → LeaderCheckVerdict  (ade_core::consensus::leader_check; era-correct Praos construction; N-R-A + N-W)
  4. RED kes_sign_header_advancing(period, UnsignedHeaderPreImage)  (N-S-A pre-image; N-AC EVOLVES the operator KES key to `period` via the deterministic Sum6KES kes_update THEN signs ONLY the branded pre-image at the evolved current period; fail-closed EvolutionBackwards / EvolutionExhausted; period passed verbatim; DC-CRYPTO-10)
  5. GREEN assemble_tick
  6. BLUE forge_block → encode_block_envelope       (single canonical block encoder, storage-form [era, block]; N-V)
  7. BLUE self_accept                               (gate — no ForgeSucceeded without Accepted; N-F-G-B: run_real_forge now SURFACES this original BLUE AcceptedBlock token as the .1 of (CoordinatorEvent, Option<AcceptedBlock>) — Some iff ForgeSucceeded — never re-derived from artifact.bytes; the produce_mode BroadcastBlock caller takes .0 and is functionally unaffected)
  8. ChainEvolution::advance(self)                  (GREEN linear typestate; token only via self_accept; N-T)
  9. ServedChainHandle::push_atomic                 (single served-admit authority; N-R-B/N-T)
 10. BLUE serve composition (N-X)                   (block_fetch::server emits compose_blockfetch_block(storage [era, block]) = tag24(bytes([era, block]));
                                                     chain_sync::server emits compose_rollforward_header(era, header_cbor) = [era_tag, tag24(bytes(header_cbor))])
 11. OutboundCommand → MuxPump                      (typed relay; no byte tunnel; N-S-B)
Cross-surface state sharing: ChainEvolution threads each forge's post-state into the next
  forge's base; ServedChainSnapshot is shared with the N2N serve path; the per-peer outbound
  map is shared with the listener. The serve step's tag-24 wrap is the SAME ade_codec authority
  the receive path uses to unwrap (CN-WIRE-08). produce_mode's KES/VRF/cold/opcert loaders are
  the SAME loaders reused by the N-F-F operator-forge ingress (no reimpl). N-F-G-B: run_real_forge's
  surfaced BLUE AcceptedBlock is the SAME token the node-spine self-accept→serve handoff (above) wraps into a
  SelfAcceptedHandoff; the ServedChainHandle::push_atomic is the SAME single served-admit authority both paths use.
N-F-A/N-F-C/N-F-E/N-F-F/N-F-G-A fence (populate-side AND consume-side enforced): produce_mode is the forge-time
  consensus-input path (import_live_consensus_inputs + pool_distr_view_from_consensus_inputs +
  --consensus-inputs-path). It MUST pass SeedEpochConsensusSource::NotRequired and MUST NOT build / put
  the seed-epoch sidecar nor append its WAL provenance (CN-CINPUT-02 populate-side). produce_mode stays
  DIAGNOSTIC and does NOT consume the recovered surface. The recovered-surface CONSUME-side seam is
  CLOSED on the SEPARATE node-lifecycle forge path: node_sync::forge_one_from_recovered projects the
  leadership view ONLY via PoolDistrView::from_seed_epoch_consensus_inputs(&recovered.…), and may NOT
  fabricate a SeedEpochConsensusInputs literal or name the bundle tokens
  (ci_check_consensus_input_provenance.sh guard (d); CN-CINPUT-03 / DC-CINPUT-02b). The N-F-E forge tick,
  the N-F-F On arm, and the N-F-G-A current-constants forge tick reach that same fenced forge path from
  the relay loop's ForgeTick branch.
```

### Surface: seed-epoch sidecar warm-start (recovered consensus inputs — N-F-A; WIRED in N-F-C)

```
Surface: warm-start restore of the recovered seed-epoch consensus inputs (RED ade_runtime::bootstrap::restore_seed_epoch_consensus_inputs, inside the bootstrap_initial_state authority)
Reduces to: anchor-fp-keyed sidecar bytes (SnapshotStore::get_seed_epoch_consensus_inputs) → verified SeedEpochConsensusInputs → BootstrapState.seed_epoch_consensus_inputs: Option<SeedEpochConsensusInputs>
Pipeline (fixed; the RED-read / BLUE-verify chain; fail-closed on every step):
  1. SeedEpochConsensusSource discriminant           (RequiredFromRecoveredProvenance(provenance) ⇒ restore; NotRequired ⇒ None — the only two modes; the node-lifecycle WarmStart arm passes RequiredFromRecoveredProvenance; produce_mode + every other caller passes NotRequired)
  2. RED get_seed_epoch_consensus_inputs(anchor_fp)  (the only RED step — reads the anchor-fp-keyed sidecar bytes; absent ⇒ BootstrapError::SeedConsensusSidecarMissing)
  3. BLUE blake2b_256 bind                            (re-hash the read bytes; != provenance.sidecar_hash ⇒ SeedConsensusHashMismatch)
  4. BLUE decode_seed_epoch_consensus_inputs          (the A1 SOLE decoder; version-gated, byte-canonical; failure ⇒ SeedConsensusSidecarDecode)
  5. BLUE anchor/epoch binding                        (decoded anchor_fp/epoch_no != provenance ⇒ SeedConsensusBindingMismatch)
  6. BLUE byte-identity re-encode                     (re-encode != input ⇒ SeedConsensusHashMismatch)
Cross-surface state sharing: the same SnapshotStore + WAL the forward-sync pump, recovery::restart, the
  N-F-C node lifecycle, and the N-F-D relay loop use. The recovered BootstrapState this produces is the
  SOLE leadership source the N-F-E forge tick / N-F-F On-arm / N-F-G-A current-constants tick projects
  (PoolDistrView::from_seed_epoch_consensus_inputs) AND the single forge base (no second bootstrap, CN-NODE-01).
N-F-C WIRING: the production restart path is wired — ade_node::node_lifecycle (--mode node) drives this
  restore on its WarmStart arm via list_seed_epoch_consensus_anchor_fps discovery → WAL replay →
  bootstrap_initial_state(RequiredFromRecoveredProvenance). The CONSUME-side fence
  (ci_check_consensus_input_provenance.sh guard (d): CN-CINPUT-03 / DC-CINPUT-02b) keeps the forge path
  from fabricating the record. N-F-G-A note: the recovered ledger now also carries the oracle-bound CURRENT
  ProtocolParameters (installed at seed/import, S2a) — warm-start preserves it. N-F-G-C note: the recovered tip
  this produces is the start_point spawn_live_wire_pump_source dials from (Point::Block) on the live-feed path.
```

### Surface: --mode node lifecycle (FirstRun / WarmStart — the real Ade node, N-F-C)

```
Surface: --mode node (RED ade_node::node_lifecycle; THE single PHASE4-N-F-C-LIFECYCLE-OWNER)
Reduces to: on-disk state → closed NodeStart {FirstRun | WarmStart} → verified BootstrapState through the single bootstrap_initial_state authority → forge-intent classify (N-F-F) → source select (N-F-G-C) → run_relay_loop (N-F-D)
Pipeline (fixed; classification is a PURE function of on-disk state):
  1. open persistent ChainDb (PersistentChainDb) + FileWalStore
  2. classify_start(has_tip, has_snapshots)            (pure → NodeStart::FirstRun | NodeStart::WarmStart)
  3a. FirstRun  → first_run_mithril_bootstrap          (Mithril-only; bootstrap_from_mithril_snapshot; verify_mithril_binding fail-closed BEFORE any state admitted; persists seed-epoch sidecar + WAL provenance; NO genesis/bundle/cold/graft fallback)
  3b. WarmStart → warm_start_recovery                  (list_seed_epoch_consensus_anchor_fps discovery → WAL replay → bootstrap_initial_state(RequiredFromRecoveredProvenance) verify chain)
  4. (N-F-F) classify_forge_intent over flag presence  (Off ⇒ forge:None; On(paths) ⇒ build_operator_forge_material → coordinator_init → ForgeActivation → forge:Some; PartialKeySet ⇒ NodeLifecycleError::ForgeKeyIngress = exit 44)
  4a. (N-F-G-A) the On-arm ForgeActivation carries current pparams + protocol_version  (from forge_constants_from_pparams; real opcert/genesis ingress; checked clock; off-epoch guard inside forge_one_from_recovered)
  4b. (N-F-G-C) source select  (--peer present ⇒ spawn_live_wire_pump_source → LIVE NodeBlockSource::WirePump via from_wire_pump, reusing the closed admission dial + pump; else NodeBlockSource::in_memory(Vec::new()) — the empty source)
  5. (N-F-D) both arms converge into run_relay_loop    (the single live-run owner; the tip advances ONLY via run_node_sync → pump_block; N-F-F: forge:None on the default binary path, forge:Some on the operator-key On arm; N-F-G-C: the chosen source is handed in)
Cross-surface state sharing: shares the single bootstrap_initial_state authority with produce_mode +
  the Conway-genesis + Mithril production-bootstrap paths; shares the persistent ChainDb + FileWalStore
  with the forward-sync pump + warm-start restore + the relay loop. The recovered BootstrapState is BOTH
  the spine base AND (On arm) the forge base — one recovered state, no second bootstrap. N-F-G-C: the live
  feed reuses the SAME admission dial + pump the wire-only / admission modes use (no reimpl). Closed fail-closed
  NodeLifecycleError (incl. RelaySync, N-F-D; ForgeKeyIngress, N-F-F; MissingFlag("--network-magic") on the
  live-feed arm, N-F-G-C).
Rule (CN-NODE-01, ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh + ci_check_node_binary_uses_single_bootstrap.sh):
  exactly one module carries the PHASE4-N-F-C-LIFECYCLE-OWNER marker; both arms route through the SINGLE
  bootstrap_initial_state authority — no second bootstrap/recovery/storage-init path, no genesis/bundle/cold/
  graft fallback, no recover_node_state overclaim. ReceiveState::new is legitimate only in the lifecycle-owner
  files {node.rs, node_lifecycle.rs} (N-F-F owner allow-list). A new mode that needs initial state MUST obtain
  it via this one authority. The N-F-F On arm reuses the SAME recovered state as the forge base; N-F-G-C's live
  feed introduces NO second bootstrap (the tip still advances only via run_node_sync → pump_block).
```

### Surface: --mode node sync source (verdict-decoupled peer-block source, N-F-C; readiness extended N-F-D; LIVE WirePump fill N-F-G-C)

```
Surface: NodeBlockSource (RED ade_node::node_sync; closed 2-variant enum {WirePump | InMemory})
Reduces to: ordered peer-block BYTES only (AdmissionPeerEvent::Block → Vec<u8>); NEVER a verdict
Pipeline (the verdict-decoupled source contract, E1/E2; N-F-D readiness signal; N-F-G-C live fill):
  1. next_block() selects ONLY AdmissionPeerEvent::Block, in arrival order
  1a. AdmissionPeerEvent::TipUpdate is SKIPPED            (a comparison input for admission's verdict loop, not a block, not a sync tip authority)
  1b. AdmissionPeerEvent::Disconnected / closed channel ENDS the feed (a clean disconnect is not a tip authority)
  1c. (N-F-D) content-blind readiness                     (has_work_ready / is_ended / wait_ready via non-blocking try_recv; next_block is non-blocking drain-then-None; never awaited across a shutdown cancellation boundary; the WirePump arm fills an opaque content-blind lookahead VecDeque<Vec<u8>> from the channel)
  1d. (N-F-G-C) from_wire_pump(rx: mpsc::Receiver<AdmissionPeerEvent>)  (the LIVE constructor of the WirePump arm — fed by spawn_live_wire_pump_source reusing the closed admission dial + pump; a FILL of the existing arm, NOT a new variant)
  2. (driver) run_node_sync feeds bytes to forward_sync::pump_block — its production caller — durable StoreBlockBytes + AppendWal BEFORE AdvanceTip (DC-SYNC-01), then a tip checkpoint via PersistentSnapshotCache
Cross-surface state sharing: the same persistent ChainDb + FileWalStore the lifecycle owner + warm-start
  restore + the relay loop use; the captured tip checkpoint is the exact PersistentSnapshotCache artifact warm_start_recovery reads back.
Rule (DC-SYNC-01 / DC-SYNC-02, ci_check_node_sync_via_pump.sh): the source yields ordered block bytes and
  NOTHING else (no derive_verdict / run_admission / follow); run_node_sync advances the tip ONLY via
  pump_block (no follower-as-sync, no verdict-as-sync, no manual put_block/AdvanceTip/rollback_to_slot).
  run_node_sync is UNMODIFIED in production by N-F-D / N-F-E / N-F-F / N-F-G-A / N-F-G-C (#[cfg(test)] additions
  + N-F-G-A's forge_epoch_admission GREEN-by-fn guard; N-F-G-C added the from_wire_pump constructor + the
  live_wire_pump_feed_reaches_forge_tick reachability test). The WirePump arm stays a closed verdict-decoupled
  contract — a FILL from a LIVE dial, NOT a plugin point for alternative sources.
HONEST SCOPE: run_node_sync is driven on the live run path ONLY by the N-F-D relay loop (SyncOnce);
  forge_one_from_recovered is reached ONLY via the N-F-E ForgeTick branch when a ForgeActivation is
  supplied. N-F-G-C: with --peer, the WirePump arm is fed a LIVE unbounded cardano-node peer (the prior
  RO-LIVE-01 feed-side follow-on, now wired); with no --peer the empty source halts before any SyncOnce/ForgeTick.
  Peer ACCEPT stays operator-gated (RO-LIVE-01 partial).
```

### Surface: BA-02 peer-acceptance evidence (GREEN correlator N-F-C; RED file I/O wired N-F-G-C)

```
Surface: operator-captured peer-accept JSONL log (GREEN ade_node::ba02_evidence::parse_peer_accept_events; RED file I/O ade_node::ba02_pass, N-F-G-C)
Reduces to: closed PeerAcceptEvent set → BA02Outcome {Ba02Manifest | NoEvidence} via correlate
Pipeline (allow-list parse → exact-match correlate; the SOLE Ba02Manifest constructor):
  1. (N-F-G-C) ba02_pass::correlate_peer_log_file       (RED — std::fs read of the operator-captured file; a missing/unreadable file ⇒ io::Error fail-closed, NEVER a synthesized acceptance)
  2. parse_peer_accept_events                            (GREEN ALLOW-LIST: only `peer_served_block` / `peer_chain_tip` discriminators → PeerAcceptEvent; every weaker/unknown/malformed line DROPPED, never coerced)
  3. AdeForgeRecord::from_forge_artifact                 (reads the BLUE-minted forged hash + slot VERBATIM from ForgedBlockArtifact — NEVER recomputed; no new BLUE authority)
  4. correlate(ade, peer_log)                            (GREEN pure/total/deterministic, HASH-PRIMARY; emits BA02Outcome::Ba02Manifest ONLY on an exact forged-hash↔peer-accept match at the matching chain point, no conflicting signal; else NoEvidence{reason})
  5. (N-F-G-C) ba02_pass::write_ba02_manifest            (RED — accepts ONLY a Ba02Manifest, so a written manifest is ALWAYS correlate-produced — no path emits one from NoEvidence or raw operator input)
Cross-surface state sharing: NONE. GREEN evidence comparing already-authoritative outputs; ba02_pass is RED file
  I/O ONLY (constructs no evidence, derives no acceptance, never coerces a non-acceptance line). A Ba02Manifest is
  a CLAIM ABOUT authority, not authority.
Rule (RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01, ci_check_ba02_evidence_closed.sh + ci_check_ba02_evidence_manifest_schema.sh):
  correlate is the ONLY Ba02Manifest constructor; write_ba02_manifest accepts ONLY a Ba02Manifest; NO self-evidence
  token (ForgeSucceeded / self_accept / block_received / served-block / wire-success / agreement_verdict / "agreed")
  may be an acceptance source; NO committed synthetic *ba02* manifest. The new schema gate (8 fields +
  schema_version == 1 + peer_log_file_sha256 cross-check) is vacuous-until-committed. The versioned Ba02Manifest
  (BA02_MANIFEST_SCHEMA_VERSION = 1) is a version-GATED contract (§4).
HONEST SCOPE: G-C wires the RED file I/O (ba02_pass) over the UNCHANGED GREEN correlator, reached by NO binary arm
  at this HEAD (tested only). Synthetic logs prove the mechanics only and CANNOT satisfy BA-02. BA-02 is satisfied
  NOWHERE at this HEAD (incl. N-F-G-C's live-feed scaffolding); RO-LIVE-01 remains partial/operator-gated — a real
  BA-02 result needs an operator-captured peer log through correlate against a peer that can grant leadership
  (C1 private testnet or C2 preprod with provisioned stake).
```

### Surface: operator file ingress (KES skey / opcert / Shelley genesis / UTxO seed dump)

```
Surface: operator-supplied files (RED ade_runtime::producer::{keys, opcert_envelope, genesis_parser}, seed_import)
Reduces to: Sum6Kes signing key (via BLUE deserializer) / OperationalCert / GenesisAnchor / canonical seed entries
Pipeline:
  1. RED parse text/JSON/CBOR envelope               (closed parser per file type; structured fail-closed error)
  2. BLUE structural validator                       (e.g. Sum6Kes::raw_deserialize_signing_key_kes — byte layout is the validator)
  3. canonical type handed to the BLUE core          (never raw bytes)
Cross-surface state sharing: GenesisAnchor + opcert public metadata feed the producer coordinator;
  KES/VRF/cold private material is RED-confined and never enters GREEN CoordinatorState.
N-F-F note (CLOSED — was the prior §7 candidate #1): --mode node NOW ingests real operator keys. The
  ingress is a NEW dedicated surface (ade_node::operator_forge, see the operator-key-ingress surface above)
  that REUSES these exact loaders (load_cold_signing_key_skey / load_vrf_signing_key_skey /
  load_kes_skey_any_format) in a RED-parse → BLUE-structural-validate → canonical-type pipeline — no new
  BLUE authority, no parser reimpl, no plugin seam. Key custody stays RED-confined to ProducerShell
  (OperatorForgeMaterial not Debug/Serialize; no byte accessor / serialization / logging). The forge-on flip
  is opt-in via operator-key flag presence (CN-NODE-03).
N-F-G-A note: on the --mode node path the operator_forge ingress now uses the REAL parse_opcert_envelope +
  parse_shelley_genesis (the parse_simple_* stubs are RETIRED there) — same RED-parse → BLUE-validate pipeline,
  no new BLUE authority (ci_check_node_forge_real_cli_ingress.sh). The cardano-cli query protocol-parameters
  JSON is a NEW operator-sourced preimage (carried in the consensus-inputs bundle) parsed by the GREEN
  consensus_inputs::protocol_params parser into the canonical BLUE ProtocolParameters (no float).
N-F-G-C note: a NEW operator-supplied INPUT enters here — the operator-captured peer-accept JSONL log, read by
  the RED ba02_pass::correlate_peer_log_file and reduced through the GREEN correlate to the closed BA02Outcome
  (see the BA-02 surface above). It is evidence input, never runtime authority; a missing/unreadable file fails
  closed (io::Error). No new wire-feed file type — the live feed reuses the closed admission dial + pump.
```

### Surface: Mithril snapshot manifest — provenance binding (N-Y)

```
Surface: Mithril snapshot manifest JSON (RED ade_runtime::mithril_import::json::parse_mithril_manifest_json)
Reduces to: RawMithrilManifest → SeedProvenance::Mithril{..} + MithrilManifestReport → (BLUE) verify_mithril_binding verdict
Pipeline (fixed; the RED-then-BLUE provenance binding):
  1. RED parse_mithril_manifest_json                 (SOLE manifest-JSON parser → RawMithrilManifest; fail-closed MithrilManifestError; NO semantic decision)
  2. RED import_mithril_manifest                     (maps into the closed SeedProvenance::Mithril + MithrilManifestReport; NEVER re-verifies the STM multisig)
  3. BLUE verify_mithril_binding(report, anchor)     (the SOLE authority deciding whether a Mithril anchor binds; cross-checks {network_magic, genesis_hash, certified_point, certificate_hash}; fails closed with MithrilImportError)
Cross-surface state sharing: the report side (manifest) and the anchor side MUST originate
  independently — verify_mithril_binding is NOT a tautological self-check (CN-MITHRIL-01).
N-F-F/N-F-G-A/N-F-G-C note: Mithril is UNTOUCHED — it stays the bootstrap/recovery layer (a bootstrap
  accelerator), NOT a validation/forge shortcut. The operator-key ingress + the live feed do NOT call Mithril
  and do NOT create a second bootstrap path (CN-NODE-01; the forge base is the single recovered BootstrapState).
```

### Surface: Mithril production-bootstrap composition (N-Z, extended N-F-A sidecar tail; first non-test caller N-F-C)

```
Surface: bootstrap_from_mithril_snapshot (RED ade_runtime::mithril_bootstrap; composition-only — NO standalone argv flag; N-F-C first non-test caller = the --mode node FirstRun arm)
Reduces to: (MithrilSeedPointInputs, seed (LedgerState, PraosChainDepState), manifest_bytes) → MithrilBootstrapOutput { ledger, chain_dep, tip, anchor } (+ N-F-A sidecar put + WAL provenance append)
Pipeline (fixed; the call-order is CI-pinned by ci_check_mithril_seed_point_independence.sh):
  1. RED import_mithril_manifest_from_bytes(manifest_bytes)  (→ MithrilProvenanceImport { provenance, report }; fail-closed MithrilBootstrapError::Import; NO semantic decision)
  2. RED mint(MintInputs{ seed_slot/seed_block_hash/network_magic/genesis_hash/… from MithrilSeedPointInputs (operator-INDEPENDENT origin); seed_provenance = import.provenance })  (→ BootstrapAnchor; seed_point comes ONLY from the operator inputs, NEVER the manifest — DC-MITHRIL-02)
  3. BLUE verify_mithril_binding(&import.report, &anchor)    (the SOLE binding authority; fail-closed MithrilBootstrapError::Binding BEFORE any storage init; CN-MITHRIL-01 strengthened — verify-before-bootstrap)
  4. RED bootstrap_initial_state(BootstrapInputs{ …, genesis_initial: Some((seed_ledger, seed_chain_dep)) , seed_epoch_consensus_source: NotRequired })  (the single closed bootstrap authority; never a parallel storage-init path; CN-NODE-01)
  5. RED sidecar tail (N-F-A; success path only, after binding passes)  (GREEN merge_seed_epoch_consensus_inputs → A1 encode → put_seed_epoch_consensus_inputs(anchor_fp, …) DURABLE → append_seed_epoch_provenance (WAL tag 3) COMMIT POINT; the composer WRITES the sidecar, never consumes one; CN-CINPUT-02)
Cross-surface state sharing: shares the single bootstrap authority with produce_mode + the
  Conway-genesis path + the N-F-C node lifecycle. The operator's MithrilSeedPointInputs origin and the
  manifest origin MUST stay structurally independent (DC-MITHRIL-02). The N-F-A sidecar tail puts to the
  anchor-fp-keyed SnapshotStore namespace (disjoint from the slot-keyed snapshot space) THEN appends WAL
  provenance — a crash between leaves an unrecorded sidecar, which warm-start treats as "not imported"
  and fails closed.
```

### Surface: Conway genesis cold-start (N-Y, extended N-F-A sidecar tail)

```
Surface: Conway genesis config (RED ade_runtime::genesis_bootstrap::bootstrap_from_conway_genesis)
Reduces to: ConwayGenesisConfig → (LedgerState, PraosChainDepState) → BootstrapInputs.genesis_initial (+ N-F-A sidecar put + WAL provenance append)
Pipeline (fixed; the RED-read / BLUE-transform / single-authority composition):
  1. RED genesis_parser file read/parse              (shelley/Conway genesis JSON → ConwayGenesisConfig)
  2. BLUE genesis_initial_state(&ConwayGenesisConfig) (pure Conway-only transform; fail-closed GenesisSourceError::NonConwayEra)
  3. RED route through bootstrap_initial_state       (genesis pair enters ONLY via BootstrapInputs.genesis_initial; records SeedProvenance::CardanoCliJson; seed_epoch_consensus_source: NotRequired; never a second storage-init authority)
  4. RED sidecar tail (N-F-A)                         (GREEN merge → A1 encode → put_seed_epoch_consensus_inputs DURABLE → append_seed_epoch_provenance (WAL tag 3) COMMIT POINT; WRITES the sidecar, never consumes one; CN-CINPUT-02)
Cross-surface state sharing: shares the single bootstrap authority with produce_mode + the
  N-Z Mithril production-bootstrap composition. bootstrap_from_mithril_snapshot is the
  symmetric Mithril-path twin of this entry; both gained the same &mut dyn WalStore + sidecar tail.
  NOTE (N-F-C): the --mode node FirstRun arm is Mithril-ONLY (it routes through
  bootstrap_from_mithril_snapshot, NOT this genesis path — no genesis fallback on the node lifecycle).
  NOTE (N-F-F/N-F-G-A): the operator-key ingress reuses producer::genesis_parser (N-F-G-A: the REAL
  parse_shelley_genesis) for clock/KES ANCHOR EXTRACTION ONLY — NOT a bootstrap source and NOT a new
  semantic genesis authority.
```

### Surface: argv (closed mode set)

```
Surface: command line (RED ade_node::cli — Cli / ProduceCli / AdmissionCli / KeyGenKesCli)
Reduces to: a 5-variant CLOSED Mode enum {wire_only, admission, key_gen_kes, produce, node}
  (Mode::parse; NOT #[non_exhaustive]; main.rs dispatch has NO wildcard arm; ci_check_node_mode_closure.sh)
Pipeline: argv → Cli → mode driver. --mode produce requires --json-seed + --consensus-inputs-path;
  --mode node (N-F-C) requires --snapshot-dir + --wal-dir, and on FirstRun the documented-extraction
  inputs (--json-seed + --consensus-inputs-path + --mithril-manifest-path, Mithril-bound — NEVER forge inputs).
  --mode node (N-F-F) ADDITIONALLY accepts the OPTIONAL operator-key flags (cold/KES/VRF skey + opcert +
  genesis-file) classified by classify_forge_intent over PRESENCE: all five ⇒ forge-on, none ⇒ relay-only,
  any partial ⇒ fail closed (exit 44). --mode node (N-F-G-C) ADDITIONALLY accepts the OPTIONAL --peer flag(s):
  present ⇒ a LIVE NodeBlockSource::WirePump feed (requires --network-magic, else MissingFlag fail-closed);
  absent ⇒ the empty in_memory source (halts before any SyncOnce/ForgeTick).
Cross-surface state sharing: none.
N-F-C: the `node` variant is the addition; main() routes Mode::Node → run_node_lifecycle. Adding a
  Mode variant is a SURFACE REDUCTION (closed taxonomy), not an extension point — a new variant forces a
  main.rs compile error until an explicit (wildcard-free) arm is added (ci_check_node_mode_closure.sh).
N-F-D/N-F-E/N-F-G-A: NO new argv flag for the relay loop / forge tick / forge fidelity. N-F-F: the operator-key
  flags are an OPTIONAL ingress on the existing --mode node arm (no new Mode variant). N-F-G-C: --peer is an
  OPTIONAL live-feed flag on the existing --mode node arm (no new Mode variant) — present ⇒ the forge tick
  becomes reachable in production once a leader slot is due (RO-LIVE-01); absent ⇒ the default empty-source path.
N-F-G-D: NO new argv flag — the --mode node accepted-block PATH is pinned to a closed 28-flag allow-list by
  ci_check_node_path_fidelity.sh (a private-only / venue flag — e.g. --private-net / --from-genesis / --devnet /
  --rehearsal — would trip the fence). The C1 private-testnet dry-run uses the SAME path as preview/preprod,
  differing ONLY in operator INPUTS (a private genesis whose stake makes Ade win slots fast) + the evidence
  LABEL (a non-promotable PrivateRehearsalManifest); there is NO from-genesis consensus-inputs constructor —
  the node path sources consensus inputs ONLY via the shared import_live_consensus_inputs (CN-REHEARSAL-FIDELITY-01).
```

**Rule:** New ingress attaches by producing the canonical BLUE type's bytes and entering
the **same** pipeline. A new mini-protocol attaches through `session::core::step` + a BLUE
`*_transition` reducer + a closed `AcceptedMiniProtocol` registry entry — and on the SEND side
its large outbound payloads are SEGMENTED by the SAME `session::core::handle_outbound` into
ordered `<= MAX_PAYLOAD` frames via the single `encode_inner_frame` authority (N-AB / CN-SESS-05;
the outbound inverse of CN-SESS-04 inbound reassembly — never a second frame encoder, never a
runtime-configurable / unbounded payload cap; `> MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` fails closed).
A new operator
file type attaches as a RED parser feeding a BLUE structural validator. **A new bootstrap
seed source (like Mithril or genesis) attaches by populating `BootstrapInputs.genesis_initial`
and routing through the single `bootstrap_initial_state` authority — NEVER via a new
`*Anchor` trait / plugin seam, and never via a parallel storage-init path** (CN-MITHRIL-01 /
CN-NODE-01 / DC-GENESIS-SRC-01 / DC-MITHRIL-02). **A recovered-state surface (like the N-F-A
seed-epoch consensus inputs) is populated ONLY on the verified-bootstrap composition path
(put-then-WAL-append) and is read back ONLY by the warm-start restore inside
`bootstrap_initial_state` — the producer / forge-time path may not populate it (CN-CINPUT-02
populate-side); the CONSUME-side wiring is CLOSED in PHASE4-N-F-C, fenced by
`ci_check_consensus_input_provenance.sh` guard (d) (CN-CINPUT-03 / DC-CINPUT-02b);
`produce_mode` stays diagnostic.** **A new live-run step attaches to the relay loop as a closed
`LoopStep` variant + a content-blind GREEN planner input + a fenced RED branch (N-F-D / N-F-E
pattern) — the planner emits a closed vocabulary and cannot decide authority; the loop body
advances the tip ONLY via `run_node_sync` and (if it forges) reaches EXACTLY ONE fenced
`forge_one_from_recovered`, serving/admitting/gossiping nothing (CN-NODE-02 / DC-NODE-05).**
**A LIVE feed for the `--mode node` spine (N-F-G-C) attaches by REUSING the closed
`ade_runtime::admission::{dial_for_admission, run_admission_wire_pump}` and FILLING the closed
`NodeBlockSource::WirePump` arm via `from_wire_pump` — NEVER a new `NodeBlockSource` variant, a
new wire authority, a reimplemented dial/pump, a second tip-advance, or a verdict; a dial/parse
failure is logged-and-dropped, never fatal / fabricated / a tip graft (the broadened
`ci_check_served_chain_handoff_fence.sh` + the byte-unchanged `ci_check_node_run_loop_containment.sh`).**
**Operator signing-material ingress (N-F-F) attaches as a closed presence-classifier (`forge_intent`,
GREEN) + a single named RED ingress site (`operator_forge`) that REUSES the existing
cold/vrf/kes/opcert loaders — NO new BLUE authority, NO parser reimpl, NO plugin seam, NO second
forge codepath; key custody stays RED-confined to `ProducerShell`.** **A forge-fidelity boundary
(N-F-G-A) attaches INSIDE the existing forge fence — a current-protocol-parameters source as a
GREEN RED-parse → BLUE-`ProtocolParameters` pipeline (no float; hash-bound to the fingerprinted
`protocol_params_hash`; the preimage carried OUTSIDE the frozen 15-field fingerprint), a checked
clock→slot guard that fails closed before-anchor (never saturates), and an off-epoch guard derived
via the BLUE `EraSchedule::locate` that fails the forge closed before leadership — each a closed
fail-closed surface, NOT a new extension point or a new BLUE authority; `run_relay_loop`'s
containment stays SEMANTICALLY UNCHANGED (DC-EPOCH-03 — `ci_check_recovered_ledger_pparams_sourced.sh`
+ `ci_check_node_forge_real_cli_ingress.sh` + `ci_check_node_forge_single_epoch_fail_closed.sh` +
`ci_check_genesis_consistency_fixture_present.sh`).** **A self-accept→serve handoff (N-F-G-B) attaches
ONLY through the constructor-fenced `SelfAcceptedHandoff` carrier (sole ctor `from_self_accepted`,
no raw-bytes/artifact/event/flag/verdict ctor) into a sibling served-chain admit task whose sole
mutation is the single `ServedChainHandle::push_atomic` fed by `into_accepted()`; the relay-loop body
forwards a typed `mpsc<SelfAcceptedHandoff>` send only (DC-NODE-06 / CN-PROD-04 /
`ci_check_served_chain_handoff_fence.sh`).** **BA-02 evidence I/O (N-F-G-C) attaches as RED file I/O
(`ba02_pass`) over the UNCHANGED GREEN `correlate` — `correlate` stays the SOLE `Ba02Manifest`
constructor; `write_ba02_manifest` accepts ONLY a `Ba02Manifest`; no self-evidence acceptance source;
no committed synthetic manifest (RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01 —
`ci_check_ba02_evidence_manifest_schema.sh`, vacuous-until-committed + sha256-bound).** **A bounty DRY-RUN
rehearsal (N-F-G-D) attaches as a path-faithful, NON-PROMOTABLE apparatus that adds NO new `--mode node` argv
flag (the `cli.rs` flag set equals the pinned 28-flag closed allow-list) and NO from-genesis consensus-inputs
constructor — the C1 private dry-run uses the SAME accepted-block path as preview/preprod, sourcing consensus
inputs ONLY via the shared `import_live_consensus_inputs`; a condition that would fail on preprod is a
shared-path bug, never special-cased. Its evidence attaches as a GREEN constructor-fenced
`PrivateRehearsalManifest` (sole ctor `from_correlate_outcome`, `None` on `NoEvidence`; `to_canonical_toml`
emits `is_rehearsal`/`not_bounty_evidence` literals) over the UNCHANGED `correlate`, written by a RED
`rehearsal_pass` that REUSES `ba02_pass::correlate_peer_log_file` VERBATIM — stored ONLY under the rehearsal home
`docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`, sha256-bound, never under / referenced by a bounty home,
flipping NO RO-LIVE rule (CN-REHEARSAL-FIDELITY-01 — `ci_check_node_path_fidelity.sh` +
`ci_check_rehearsal_manifest_schema.sh`, vacuous-until-committed).** A new `--mode`
that needs initial state MUST obtain it via the single `bootstrap_initial_state` authority (CN-NODE-01).
New ingress **may not** introduce a second `PreservedCbor` construction site, a second
block-envelope encoder, a second era→leader-VRF-input construction (CN-FORGE-04), a second
`wrap_tag24` / `unwrap_tag24` definition or a hand-rolled tag-24 parse in RED (CN-WIRE-08), a
direct-transport write that bypasses `OutboundCommand`, a forward-sync path that advances the tip
before the durability writes ack (DC-SYNC-01), a second bootstrap/storage-init authority
(CN-NODE-01 / DC-GENESIS-SRC-01), a Mithril manifest parser other than `parse_mithril_manifest_json`
(CN-MITHRIL-01), a Mithril-bootstrap composition that drills into the manifest import to source the
anchor `seed_point` (DC-MITHRIL-02), a second `SeedEpochConsensusInputs` codec (CN-CINPUT-01), a
forge-time path that populates the seed-epoch sidecar / appends its WAL provenance (CN-CINPUT-02), a
`Mode` enum variant without an explicit wildcard-free `main.rs` dispatch arm (CN-NODE-MODE-01), a
second `--mode node` lifecycle owner or a lifecycle arm bypassing `bootstrap_initial_state`
(CN-NODE-01), a node-sync driver that advances the tip by any path other than `pump_block`
(DC-SYNC-01), a second `NodeBlockSource` variant / a reimplemented dial-pump / a live feed that
advances a second tip (N-F-G-C — `from_wire_pump` is a FILL of the closed `WirePump` arm; the dial +
pump are REUSED verbatim), a second `Ba02Manifest` constructor / a self-evidence acceptance source / a
committed synthetic BA-02 manifest (RO-LIVE-06), a second live-run owner or a relay-loop body that
advances the tip / forges / serves / admits / gossips outside the closed seams (CN-NODE-02 /
DC-SYNC-02), a GREEN planner that observes a `SlotNo` in `plan_loop_step` or emits a non-closed
`LoopStep` (DC-NODE-05), a node-lifecycle forge path that fabricates a `SeedEpochConsensusInputs`
literal or names a forge-time bundle token (CN-CINPUT-03 / DC-CINPUT-02b), a second operator-material
ingress site / a third `ForgeIntent` variant / a partial-key-set silent relay fallback / private
KES-VRF-cold bytes escaping `ProducerShell` (CN-NODE-03), a node-spine served-chain mutation outside
the single `push_atomic` fed by `into_accepted()` / a handoff channel typed other than
`UnboundedSender<SelfAcceptedHandoff>` (DC-NODE-06), **a `parse_simple_*` parser reintroduced on the
`--mode node` forge path (CE-G-A-2 — `ci_check_node_forge_real_cli_ingress.sh`), a recovered ledger
carrying `ProtocolParameters::default()` / genesis-initial pparams instead of the oracle preimage
(CE-G-A-2a — `ci_check_recovered_ledger_pparams_sourced.sh`), a `forge_epoch_admission` guard reordered
AFTER `query_leader_schedule` / a fabricated candidate epoch / a `NonceInput::EpochBoundary`|`CandidateFreeze`
nonce promotion on the forge path (DC-EPOCH-03 — `ci_check_node_forge_single_epoch_fail_closed.sh`), a
`protocol_params` float path, a `checked_millis_to_slot` that saturates a before-anchor tick, or a
`protocol_params_json` preimage folded INTO the 15-field canonical fingerprint (CE-G-A-1/2/2a/4), a new
`--mode node` argv flag beyond the pinned 28-flag allow-list / a from-genesis consensus-inputs constructor / a
node path that sources consensus inputs by anything other than the shared `import_live_consensus_inputs`
(CN-REHEARSAL-FIDELITY-01 clause 1), a `PrivateRehearsalManifest` constructed from anything but a
correlate-produced payload / an alternate correlator in `rehearsal_pass` / a rehearsal manifest written outside
the rehearsal home or referenced by a bounty home / a rehearsal manifest that flips a RO-LIVE rule
(CN-REHEARSAL-FIDELITY-01 clause 2).**

---

## 2. Data-Only vs. Authoritative Layers

### Domain: live `--mode node` block feed — REUSED dial/pump vs. closed source (RED reuse / closed fill; N-F-G-C)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Live-feed wiring (REUSE, no reimpl)** | `ade_node::node_lifecycle::spawn_live_wire_pump_source` (NEW, S1) | RED | Builds a LIVE `NodeBlockSource::WirePump` from the operator-supplied `--peer`. REUSES the closed `ade_runtime::admission::{dial_for_admission, run_admission_wire_pump}` VERBATIM (no reimplementation, no new wire authority) + the `pub(crate)`-promoted `build_n2n_version_table`. Each peer dialed in a `tokio::spawn` → bounded `mpsc` (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`) → `from_wire_pump(rx)`. A dial/parse failure is logged-and-dropped (C3). Decides nothing; moves bytes from a reused dial into a closed source. |
| **Reused dial authority** | `ade_runtime::admission::dial_for_admission` | RED | The SAME admission dial the wire-only / admission modes use — REUSED verbatim. UNCHANGED. |
| **Reused wire pump** | `ade_runtime::admission::run_admission_wire_pump` | RED | The SAME admission wire pump — REUSED verbatim; emits `AdmissionPeerEvent`. UNCHANGED. |
| **Closed verdict-decoupled source (FILL, not a new variant)** | `ade_node::node_sync::NodeBlockSource` (`from_wire_pump`, closed 2-variant `{WirePump, InMemory}`) | RED | The `WirePump` arm is FILLED from the live `AdmissionPeerEvent` channel; yields ordered `Block` bytes ONLY, skips `TipUpdate`, ends on `Disconnected`. NEVER a verdict. A FILL of the existing arm, NOT a new variant. |
| **Authoritative durable apply (UNMODIFIED)** | `ade_runtime::forward_sync::pump_block` (via `node_sync::run_node_sync`) | RED + BLUE admit | The SOLE durable tip-advance the live bytes feed; `StoreBlockBytes`/`AppendWal`-before-`AdvanceTip` (DC-SYNC-01). UNMODIFIED — no second tip-advance. |

**Rule (N-F-G-C live feed; `ci_check_served_chain_handoff_fence.sh` BROADENED + `ci_check_node_run_loop_containment.sh`
byte-unchanged):** the live feed is **REUSE, not reimplementation** — the dial + pump are the closed admission
authorities, reused verbatim; `build_n2n_version_table` is reused via a fn-visibility promotion. The source stays
the **closed 2-variant `NodeBlockSource`** — `from_wire_pump` is a FILL of the existing `WirePump` arm, **NOT a
new variant / plugin point / new wire authority**. The durable tip advances ONLY via `run_node_sync → pump_block`
(no second tip-advance). A dial/parse failure is logged-and-dropped — never fatal / fabricated / a tip graft.
**None of these chokepoints move.** **Honest scope:** G-C wires the MECHANICAL live feed ONLY — with a live feed
the forge is observable at a due leader slot, but **peer ACCEPT is operator-gated** (RO-LIVE-01 partial), proven
only by an operator-captured peer log through `correlate`. Ade self-accept / served-block / wire success ≠ peer
acceptance. BA-02 satisfied nowhere.

### Domain: BA-02 operator-pass evidence — RED file I/O vs. GREEN correlator vs. BLUE-minted hash (N-F-C; I/O wired N-F-G-C)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only evidence file I/O** | `ade_node::ba02_pass` (`correlate_peer_log_file`, `write_ba02_manifest`; NEW, S2 — `//! RED`) | RED | Operator-pass BA-02 evidence file I/O. `correlate_peer_log_file` reads the operator-captured peer-accept JSONL → GREEN `parse_peer_accept_events` + `correlate`; a missing/unreadable file ⇒ `io::Error` (fail-closed), never a synthesized acceptance. `write_ba02_manifest` accepts **ONLY a `Ba02Manifest`** (which only `correlate`'s exact-match arm constructs). Constructs no evidence, derives no acceptance, never coerces a non-acceptance line. **NO new closed enum / registry.** |
| **Evidence correlator (compares already-authoritative outputs) — UNCHANGED** | `ade_node::ba02_evidence` (`parse_peer_accept_events`, `correlate`, closed `BA02Outcome` / `PeerAcceptEvent` / `PeerAcceptSource` / `NoEvidenceReason` / versioned `Ba02Manifest`) | GREEN | COMPARES the BLUE-minted forged hash (read verbatim, never recomputed) against an operator-captured peer-accept signal; `correlate` is the **SOLE** `Ba02Manifest` constructor (hash-primary; exact-match arm only). Forges/admits/persists nothing. UNCHANGED by G-C — only *consumed* by the new RED `ba02_pass` wrapper. RO-LIVE-06. |
| **Authoritative forge record source** | `ade_node::ba02_evidence::AdeForgeRecord::from_forge_artifact` | GREEN | Reads the BLUE-minted forged hash + slot VERBATIM from `ForgedBlockArtifact` — NEVER recomputed; no new BLUE authority. |

**Rule (RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01; `ci_check_ba02_evidence_closed.sh` +
`ci_check_ba02_evidence_manifest_schema.sh`):** the RED `ba02_pass` is **file I/O ONLY** over the UNCHANGED GREEN
correlator; **all evidence semantics live in GREEN `correlate`** — the SOLE `Ba02Manifest` constructor (no
self-evidence acceptance source: not `ForgeSucceeded` / `self_accept` / `block_received` / served-block /
wire-success / `agreement_verdict` / "agreed"). `write_ba02_manifest` accepts ONLY a `Ba02Manifest`, so a written
manifest is **always correlate-produced** from a real peer log; **no synthetic manifest is committed** (the new
gate is vacuous-until-committed; when a `CE-G-C-LIVE_*.toml` is committed it verifies the 8-field schema +
`schema_version == 1` + `peer_log_file_sha256 == sha256(fixture)`). **The `correlate` chokepoint never moves.**
**Honest scope:** `ba02_pass` is reached by NO binary arm at this HEAD (tested only — incl. the env-gated
`node_operator_pass_ba02_live`). BA-02 is satisfied NOWHERE at this HEAD; a real result needs an operator-captured
peer log through `correlate` against a peer that can grant leadership (C1/C2). RO-LIVE-01 stays partial/operator-gated.

### Domain: private-testnet rehearsal evidence — RED file I/O vs. GREEN non-promotable envelope vs. the REUSED GREEN correlate (N-F-G-D)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only rehearsal file I/O (REUSE, no alternate correlator)** | `ade_node::rehearsal_pass` (`correlate_peer_log_file_into_rehearsal`, `write_private_rehearsal_manifest`; NEW, S2 — `//! RED`) | RED | The private-testnet rehearsal-evidence file I/O. `correlate_peer_log_file_into_rehearsal` REUSES `ba02_pass::correlate_peer_log_file` **VERBATIM** (→ the GREEN `correlate`, the SOLE `Ba02Manifest` ctor) — **no alternate correlator** — then wraps a correlate-produced payload in the GREEN envelope; `Ok(None)` iff `correlate` returned `NoEvidence`; a missing/unreadable file ⇒ `io::Error` (fail-closed). `write_private_rehearsal_manifest` accepts **ONLY a `PrivateRehearsalManifest`** (the argument type IS the gate). Constructs no evidence, synthesizes no acceptance. **NO new closed enum / registry.** |
| **Non-promotable evidence envelope (compares nothing new — wraps the SAME proof)** | `ade_node::rehearsal_evidence` (`PrivateRehearsalManifest`, closed 1-variant `RehearsalVenue { PrivateTestnetC1 }`, `RehearsalEnvelope`, `REHEARSAL_MANIFEST_SCHEMA_VERSION = 1`) | GREEN | WRAPS a correlate-produced `ba02_evidence::Ba02Manifest` (the SAME proof the bounty path produces) in a structurally distinct, NON-PROMOTABLE envelope. SOLE ctor `from_correlate_outcome` returns `None` on `NoEvidence`; `to_canonical_toml` ALWAYS emits `is_rehearsal = true` + `not_bounty_evidence = true` as LITERALS — the type **cannot represent a non-rehearsal**. Pure / deterministic; no I/O / clock / rand / float / `HashMap`. Forges/admits/persists nothing. `CN-REHEARSAL-FIDELITY-01` clause 2. |
| **Acceptance authority (REUSED VERBATIM — UNCHANGED)** | `ade_node::ba02_evidence::correlate` (via `ba02_pass::correlate_peer_log_file`) | GREEN | `correlate` stays the **SOLE** `Ba02Manifest` constructor (hash-primary; exact-match arm only). UNCHANGED by G-D — only *consumed* (transitively) through the reused `ba02_pass` wrapper. Ade self-accept / served bytes / wire success ≠ acceptance — only the Haskell peer log through `correlate` is (the allow-list is inherited verbatim). |

**Rule (CN-REHEARSAL-FIDELITY-01 clause 2; `ci_check_rehearsal_manifest_schema.sh`):** the rehearsal apparatus
is **REUSE, not reimplementation** — `rehearsal_pass` calls `ba02_pass::correlate_peer_log_file` (the SOLE
correlate path) **verbatim**; there is **NO alternate correlator**. **All acceptance semantics still live in the
UNCHANGED GREEN `correlate`** — the SOLE `Ba02Manifest` constructor; the `PrivateRehearsalManifest` merely WRAPS a
correlate-produced payload in a NON-PROMOTABLE envelope (sole ctor `from_correlate_outcome`, `None` on
`NoEvidence`; `is_rehearsal`/`not_bounty_evidence` literals — the type cannot represent a non-rehearsal).
`write_private_rehearsal_manifest` accepts ONLY a `PrivateRehearsalManifest`, so a written rehearsal manifest is
**always correlate-produced**, stored ONLY under the rehearsal home (`docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml`),
sha256-bound, **never under / referenced by a bounty home** (three barriers: distinct home, explicit markers, a
fail-closed cross-check over the active + archived G-C bounty homes), and flips **NO RO-LIVE rule**. **The
`correlate` chokepoint never moves.** **Honest scope:** `rehearsal_pass` is reached by NO binary arm at this HEAD
(tested only — incl. the env-gated `node_c1_dry_run_rehearsal_live`, `ADE_LIVE_C1_DRY_RUN`). **A bounty DRY-RUN
harness — NOT the bounty deliverable.** Private C1 acceptance ≠ bounty completion; the live C1 execution stays
`blocked_until_operator_c1_net_executed`. BA-02 is satisfied NOWHERE; preview/preprod acceptance is the single
bounty deliverable, captured separately.

### Domain: forge-constant fidelity — current-pparams source + checked clock + off-epoch guard (GREEN parse / GREEN accessor / BLUE authority-behind-seams; N-F-G-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only current-pparams parser** | `ade_runtime::consensus_inputs::protocol_params` (`parse_protocol_parameters_json`, closed `ProtocolParamsParseError`) | GREEN | The cardano-cli `query protocol-parameters` JSON parser. Converts the oracle's pparams JSON preimage into a canonical BLUE `ade_ledger::pparams::ProtocolParameters`. **No float path** — rational literals preserved as `RawValue` strings → exact `ade_ledger::rational::Rational` by INTEGER arithmetic; a non-exact literal fails closed (`InexactRational`). Produces the parameter record but is NOT its author; decides nothing semantic. Promotable to BLUE. |
| **Hash-bound current-pparams accessor** | `ade_runtime::consensus_inputs::canonical::require_forge_current_pparams` (closed 3-variant `ForgeCurrentPParamsError`) | GREEN | `LiveConsensusInputsCanonical` carries `protocol_params_json: Option<String>` **OUTSIDE** the frozen 15-field fingerprint (the fingerprint commits to `protocol_params_hash`). The accessor requires the preimage present (`PreimageAbsent`), `blake2b_256`-binds it to the fingerprinted `protocol_params_hash` (`BindMismatch`), and parses exactly (`Parse`). A hash-bound gate, not the authority. |
| **Checked clock→slot guard** | `ade_runtime::clock::checked_millis_to_slot` (closed 1-variant `SlotAlignmentError`) | GREEN | A before-anchor tick (`tick_millis < start_millis`) fails closed `BeforeGenesisAnchor` instead of saturating to `start_slot`; otherwise the EXACT `millis_to_slot` result. Pure integer arithmetic; no float, no wall-clock. The saturating `millis_to_slot` is left intact for non-forge callers. A fail-closed boundary, not an authority. |
| **Off-epoch admission guard** | `ade_node::node_sync::forge_epoch_admission` (closed 2-variant `ForgeEpochAdmission`) | GREEN-by-fn (inside RED `node_sync`) | Derived SOLELY from `(slot, era_schedule, seed_epoch)` via the BLUE `EraSchedule::locate`. Within the seed epoch ⇒ `WithinSeedEpoch`; any other / unlocatable ⇒ `OffEpoch`. Called BEFORE `query_leader_schedule` inside `forge_one_from_recovered`. A pure fail-closed guard, not the epoch authority. |
| **Authoritative epoch locator** | `ade_core::consensus::era_schedule::EraSchedule::locate` | BLUE | The single BLUE epoch locator the off-epoch guard derives the candidate epoch from. No fabricated epoch math. UNCHANGED by G-A. |
| **Authoritative ProtocolParameters / Rational** | `ade_ledger::pparams::ProtocolParameters` + `ade_ledger::rational::Rational` | BLUE | The canonical parameter model + exact-integer rational arithmetic the GREEN parser produces into. Pre-existing BLUE types; UNCHANGED by G-A (no float in `rational`). |
| **Current-pparams install (recovered ledger)** | `ade_node::admission::{seed_to_snapshot, bootstrap}` (`build_seed_ledger`) | RED | Installs the supplied CURRENT `ProtocolParameters` (never `::default()`) into the recovered ledger at seed/import; warm-start preserves it. The forge-capable caller passes the oracle-bound parameters via `require_forge_current_pparams`. |
| **Off-epoch outcome (deliberately NOT extended)** | `ade_runtime::producer::coordinator::CoordinatorEvent` (9-variant) | GREEN | An off-epoch forge routes through the EXISTING `CoordinatorEvent::ForgeNotLeader` — **NO new variant added**. The closed 9-variant event set stays additively stable. |

**Rule (DC-EPOCH-03 / CE-G-A-1/2/2a/4):** forge fidelity is **real config + current pparams + two fail-closed
boundaries inside the existing forge fence**. The GREEN parser/accessor/guards are **data-only / fail-closed
boundaries** — all semantic authority (the `ProtocolParameters` model, exact `Rational` arithmetic, the
`EraSchedule::locate` epoch locator) stays BLUE. The recovered ledger pparams are sourced from the oracle
preimage (never defaulted); the config ingress uses the REAL parsers (no `parse_simple_*` on the node path);
the off-epoch guard runs BEFORE leadership via `EraSchedule::locate` (no fabricated epoch, no nonce
promotion); `protocol_params` has **no float path** (exact `Rational` or fail closed); `checked_millis_to_slot`
**fails closed before-anchor** (never saturates); the `protocol_params_json` preimage stays **OUTSIDE** the
15-field canonical fingerprint. **None of these chokepoints move**; `run_relay_loop`'s containment is
**semantically unchanged**. This is **forge-fidelity HARDENING** — it does NOT serve / admit / gossip /
advance a durable tip; the forge stays **subordinate + self-accept-only**; the empty binary source halts
before any `ForgeTick` (forge-CAPABLE, NOT observable — RO-LIVE-01). The S1 genesis-consistency fixture is
**evidence input, never runtime authority**. **BA-02 satisfied nowhere.**

### Domain: operator-key ingress + forge-on flip (GREEN classifier / RED ingress / single-authority reuse; N-F-F, real parsers N-F-G-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Pure forge-intent classifier** | `ade_node::forge_intent` (`classify_forge_intent`, closed `ForgeIntent { On(ForgePaths), Off }`, `ForgePaths`, `ForgeIntentError::PartialKeySet`) | GREEN | The total, content-blind decision "may `--mode node` forge?" — a pure function of which operator-key CLI flags are *present* (never their contents). All five present ⇒ `On`; none ⇒ `Off`; any partial ⇒ `PartialKeySet` (static flag-name strings only). No I/O, no secret; promotable to BLUE. Decides nothing semantic; a partial set is an *error*, never an *intent*. CN-NODE-03 (intent half). |
| **Operator-material ingress site (data-only loader reuse)** | `ade_node::operator_forge` (`load_operator_producer_shell`, `build_operator_forge_material`, closed 6-variant `OperatorForgeError`, `OperatorForgeMaterial`) | RED | The SINGLE named `--mode node` operator-material ingress site. REUSES the existing cold/vrf/kes loaders + (N-F-G-A) the REAL `parse_opcert_envelope` / `parse_shelley_genesis` (no reimpl) → BLUE structural validators (`Sum6Kes::raw_deserialize_signing_key_kes`; `ProducerShell::init` enforces the opcert shape + KES-period-vs-opcert freshness bound, CN-PROD-02) → canonical types. `pool_id = Hash28(blake2b_224(cold_vk))` (the one named derivation); genesis reused for clock/KES/network anchors only. `OperatorForgeMaterial` is NOT `Debug`/`Serialize`. NO new BLUE authority, NO plugin seam, NO second forge codepath. CN-NODE-03 (custody half). |
| **Authoritative KES structural validator** | `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes` | BLUE | Byte layout IS the validator for the 608-byte cardano-cli skey envelope. UNCHANGED. |
| **Authoritative shell-init freshness bound** | `ade_ledger`/`ade_runtime` `ProducerShell::init` (reused) | BLUE+RED | Enforces the opcert shape + the KES-period-vs-opcert freshness bound (CN-PROD-02). UNCHANGED — reused, not reimplemented. |
| **Single bootstrap / forge-base authority (reused)** | `ade_runtime::bootstrap::bootstrap_initial_state` (via the lifecycle owner) | GREEN-by-content (+RED restore) | The `On` arm's forge base is the SAME single recovered `BootstrapState` that seeds the relay spine — no second bootstrap, no Mithril call, no second recovered state (CN-NODE-01). N-F-G-A: the recovered ledger now carries the oracle-bound current `ProtocolParameters`. |

**Rule (CN-NODE-03 / CN-NODE-01):** operator-key ingress is **STRICTLY RED-parse → BLUE-structural-validate
→ canonical-type**, reusing the existing loaders + (N-F-G-A) the real opcert/genesis parsers. The GREEN
classifier decides forge *intent* over flag *presence* and nothing semantic; the RED ingress site holds key
custody confined to `ProducerShell` (`OperatorForgeMaterial` not `Debug`/`Serialize`; no byte accessor /
serialization / logging; no copy into the GREEN coordinator / planner / loop / persisted / logged /
hashed-for-evidence / replay surfaces). **No new BLUE authority, no parser reimpl, no plugin/trait seam, no
second forge codepath, no new BLUE crate change.** A partial key set fails closed
(`NodeLifecycleError::ForgeKeyIngress` → exit 44). `pool_id` is derived in ONE named place, never fabricated.
The forge base is the single recovered `BootstrapState` (no second bootstrap). **The forge containment from
N-F-E stays SEMANTICALLY UNCHANGED** — N-F-F/N-F-G-A/N-F-G-C may ADD ingress/fidelity/live-feed gates but MUST
NOT relax it. **None of these chokepoints move.** **Honest scope:** the binary is forge-CAPABLE with real keys +
real constants; N-F-G-C makes the On arm live-feed-wireable (observable forge at a due leader slot) but peer
ACCEPT stays operator-gated (RO-LIVE-01). No serve-to-peer / admit / gossip / durable-tip / BA-02 / RO-LIVE
claim. Mithril untouched.

### Domain: live relay run-loop (GREEN planner / RED driver / BLUE authority-behind-seams; N-F-D, forge-tick N-F-E, forge-on flip N-F-F, fidelity N-F-G-A, live feed N-F-G-C)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Pure lifecycle planner** | `ade_node::run_loop_planner` (`plan_loop_step`, closed `LoopStep`, content-blind `SyncStatus` + `ForgeSlotStatus`, the pure `forge_slot_status` guard) | GREEN | Decides each iteration's `LoopStep` from content-blind inputs over a total table. Emits ONLY `{SyncOnce, ForgeTick, Idle, HaltCleanly}` — **cannot express an authority decision**. `forge_slot_status` is the ONLY fn here that observes a `SlotNo`. With `ForgeSlotStatus::NotDue` the table collapses to the N-F-D relay mapping. Decides nothing semantic. UNCHANGED by N-F-G-A / N-F-G-C. CN-NODE-02 / DC-NODE-05. |
| **Live-run driver (the single owner)** | `ade_node::node_lifecycle::run_relay_loop` | RED | Both `--mode node` arms converge here. Performs effects per the GREEN plan: `SyncOnce → run_node_sync` (the SOLE durable tip-advance; N-F-G-C feeds it from the LIVE WirePump source), `ForgeTick →` exactly one fenced `forge_one_from_recovered` (advances no tip, serves/admits/gossips nothing in the body), `Idle →` cancellation-safe wait, `HaltCleanly →` exit. **Body containment SEMANTICALLY UNCHANGED across N-F-E → N-F-F → N-F-G-A → N-F-G-B → N-F-G-C** — the source-selection happens in the dispatcher BEFORE the loop; the run loop body is byte-unchanged. CN-NODE-02 / DC-SYNC-02 / DC-NODE-05. |
| **Opt-in forge-activation bundle** | `ade_node::node_lifecycle::ForgeActivation<'a>` (N-F-E; REAL operator material N-F-F; current pparams + checked-clock fail field N-F-G-A; opt-in `handoff_tx` N-F-G-B) | RED | The closed opt-in bundle (`forge: Option<&mut ForgeActivation>`): borrows the `Clock` seam (checked at N-F-G-A), the `CoordinatorState`, the recovered `BootstrapState` (SOLE leadership source + forge base), the `ProducerShell` (key custody), `pool_id` / current `pparams` / `protocol_version` / `last_slot_alignment_fail` / the opt-in `handoff_tx: Option<mpsc::UnboundedSender<SelfAcceptedHandoff>>` (N-F-G-B). `None` ⇒ exact N-F-D/N-F-E relay. Decides nothing semantic. |
| **Live-feed source builder (REUSE)** | `ade_node::node_lifecycle::spawn_live_wire_pump_source` (N-F-G-C) | RED | Builds the LIVE `NodeBlockSource::WirePump` from `--peer` by reusing the closed `dial_for_admission` + `run_admission_wire_pump` (a FILL, not a new variant); handed to `run_relay_loop`. Decides nothing; moves bytes from a reused dial into a closed source. |
| **Authoritative durable apply** | `ade_runtime::forward_sync::pump_block` (carried fwd, N-Y) | RED + BLUE admit | The durability-ordered driver `run_node_sync` feeds; the BLUE admit chokepoint + `StoreBlockBytes`/`AppendWal`-before-`AdvanceTip` invariant live here. UNMODIFIED. |
| **Authoritative forge handoff** | `ade_node::node_sync::forge_one_from_recovered` (carried fwd, N-F-C; N-F-G-A S4 off-epoch guard; N-F-G-B token surfacing) | RED (driver) → BLUE | Projects leadership ONLY via `PoolDistrView::from_seed_epoch_consensus_inputs`; eligibility is the BLUE leader-schedule. **N-F-G-A** calls `forge_epoch_admission` BEFORE `query_leader_schedule`. **N-F-G-B** returns `(CoordinatorEvent, Option<SelfAcceptedHandoff>)`. CN-CINPUT-03 / DC-CINPUT-02b / DC-EPOCH-03 / DC-NODE-06. |

**Rule (CN-NODE-02 / DC-SYNC-02 / DC-NODE-05 / DC-EPOCH-03 / DC-NODE-06):** the GREEN planner decides iteration
shape over a closed, content-blind vocabulary and cannot decide authority; the RED driver performs effects but
advances the tip ONLY via `run_node_sync → pump_block` and (if forging) reaches EXACTLY ONE fenced
`forge_one_from_recovered`, serving/admitting/gossiping nothing in the body; BLUE authority stays behind the
existing seams. The wall-clock enters ONLY through the RED `Clock` seam (now checked — before-anchor fails
closed), and only a `SlotNo` crosses into GREEN/BLUE. The off-epoch guard fails the forge closed before
leadership (N-F-G-A). The N-F-G-B handoff is a typed `tx.send` only (served-chain mutation in the SIBLING task).
**N-F-G-C leaves `run_relay_loop`'s body containment byte-unchanged** — the live-feed source-selection happens in
the dispatcher before the loop; the source handed in is the closed `NodeBlockSource` (a `from_wire_pump` FILL).
The forge is **subordinate** to the sync spine — a forged block is a local self-accept artifact, served only via
the fenced sibling `push_atomic`, never a tip advance. **None of these chokepoints move.** **Honest scope:** the
binary is forge-CAPABLE with real keys + real constants; with a LIVE `--peer` feed (N-F-G-C) the forge becomes
observable at a due leader slot, but peer ACCEPT is operator-gated (RO-LIVE-01 partial); BA-02 satisfied nowhere.

### Domain: recovered seed-epoch consensus inputs (N-F-A; CONSUMED in N-F-C, exercised by the N-F-E forge tick + the N-F-F On arm + the N-F-G-A current-constants tick)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative canonical record + SOLE codec** | `ade_ledger::seed_consensus_inputs` (`SeedEpochConsensusInputs`, `encode_/decode_seed_epoch_consensus_inputs`, `SEED_CINPUT_SCHEMA_VERSION = 1`) | BLUE | The closed recovered-state record + its SOLE version-gated, byte-canonical encoder/decoder pair. `decode_*` fail-closes on unknown version, wrong shape, short hash, non-canonical / duplicate pool-map keys, trailing bytes (closed 6-variant `SeedConsensusInputsError`). No second codec. (CN-CINPUT-01.) |
| **Data-only merge glue** | `ade_runtime::seed_consensus_merge::merge_seed_epoch_consensus_inputs` | GREEN | Lifts a verified-bootstrap two-map `LiveConsensusInputsCanonical` into the BLUE single-map record; fail-closed (closed 2-variant `SeedConsensusMergeError`) — never a zero-hash fill. Produces the BLUE record but is NOT its author; decides nothing semantic. |
| **Data-only WAL provenance appender** | `ade_runtime::seed_consensus_provenance::append_seed_epoch_provenance` | RED | `blake2b_256` of the EXACT A1 sidecar bytes → `WalEntry::SeedEpochConsensusInputsImported` (tag 3) append. Allowed only at the two verified-bootstrap composition sites; called only AFTER the durable sidecar put. |
| **Data-only sidecar store** | `ade_runtime::chaindb::SnapshotStore::{put,get,list}_seed_epoch_consensus_*` | RED | Anchor-fp-keyed sidecar namespace **disjoint** from the slot-keyed snapshot space; idempotent on identical bytes; redb `seed_cinputs_by_anchor_fp` table, `SCHEMA_VERSION = 3`. N-F-C added the read-only `list_seed_epoch_consensus_anchor_fps` discovery method. No semantic decision. |
| **Authoritative warm-start restore** | `ade_runtime::bootstrap::restore_seed_epoch_consensus_inputs` (inside `bootstrap_initial_state`) | RED read + BLUE verify | The `get_seed_epoch_consensus_inputs` read is the only RED step; bind → decode → anchor/epoch binding → byte-identity verification is BLUE, fail-closed via the 5 `BootstrapError::SeedConsensus*` variants. **MUST NOT** fall back to the forge-time bundle. (DC-CINPUT-01.) Wired on the `--mode node` WarmStart arm. |
| **Authoritative replay fold** | `ade_ledger::wal::replay_from_anchor` (`ReplayOutcome`, `RecoveredBootstrapProvenance`) | BLUE | Folds the additive tag-3 entry into `ReplayOutcome.recovered_provenance` (at most one; `DuplicateProvenance` / `ProvenanceAnchorMismatch` fail closed) **without** disturbing the `AdmitBlock` fp-chain. Pure. |
| **Authoritative projection (consumed since N-F-C; exercised by the N-F-E forge tick + N-F-F On arm + N-F-G-A tick)** | `ade_ledger::consensus_view::PoolDistrView::from_seed_epoch_consensus_inputs` | BLUE | Pure field-map projecting the recovered record into the leadership `PoolDistrView` (off-epoch ⇒ `None`; no zero-hash fallback). The SOLE leadership source the node-lifecycle forge handoff consumes (DC-CINPUT-02a + DC-CINPUT-02b / CN-CINPUT-03). |
| **Consume-side forge handoff** | `ade_node::node_sync::forge_one_from_recovered` | RED (driver) | Builds the forge base ENTIRELY from recovered state + the selected tip: projects leadership ONLY via `PoolDistrView::from_seed_epoch_consensus_inputs`, fails closed (`NodeForgeError::MissingRecoveredConsensusInputs`) when none, and MUST NOT fabricate a `SeedEpochConsensusInputs` literal or name a forge-time bundle token. The CONSUME-side fence — CN-CINPUT-03 / DC-CINPUT-02b, guard (d). **N-F-G-A** adds the off-epoch `forge_epoch_admission` guard BEFORE leadership (DC-EPOCH-03). _Production code otherwise UNMODIFIED._ |
| **Read-only anchor discovery** | `ade_runtime::chaindb::SnapshotStore::list_seed_epoch_consensus_anchor_fps` | RED | Returns persisted anchor lineages in ascending order. Discovery is NOT proof; the warm-start verify chain is the authority. Sole caller `node_lifecycle::warm_start_recovery`. |

**Rule (CN-CINPUT-01 / -02 / -03 / DC-CINPUT-01 / -02a / -02b):** the recovered seed-epoch consensus inputs
are a **closed canonical type with a SOLE codec**. The RED/GREEN shells merge, encode, put, and append
provenance; **all semantic decisions (decode, binding verification, the leadership projection) live in BLUE**.
Population is **contained** to the verified-bootstrap composition path; the forge-time path MUST NOT build /
put the sidecar nor append its WAL provenance (CN-CINPUT-02). The warm-start restore + replay fold live inside
the **single `bootstrap_initial_state` authority** and the BLUE `wal::replay_from_anchor` — **neither
chokepoint moves.** The consume side is wired (the `--mode node` WarmStart arm +
`node_sync::forge_one_from_recovered`) behind the consume-side fence (guard (d)). **N-F-G-A note:** the forge
tick now also runs the off-epoch `forge_epoch_admission` guard (via `EraSchedule::locate`) before leadership;
the projection stays the SOLE leadership source. `produce_mode` stays diagnostic and still passes
`SeedEpochConsensusSource::NotRequired`.

### Domain: node lifecycle + BA-02 evidence (N-F-C; BA-02 file I/O wired N-F-G-C)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Lifecycle owner (single authority router)** | `ade_node::node_lifecycle` (`run_node_lifecycle`, `classify_start`, `first_run_mithril_bootstrap`, `warm_start_recovery`, `run_relay_loop`) | RED | THE single `--mode node` recovered-state lifecycle owner (`PHASE4-N-F-C-LIFECYCLE-OWNER`). Classifies FirstRun vs WarmStart as a PURE function of on-disk state; both arms route initial state through the SINGLE `bootstrap_initial_state` authority and converge into `run_relay_loop` (N-F-D). N-F-F adds the forge-intent classify + `Some`/`None` flip; N-F-G-A sources the On-arm constants from the recovered current view; N-F-G-C selects the live `WirePump` source (`--peer`) vs the empty source. Decides nothing semantic; never a second bootstrap/recovery/storage-init path (CN-NODE-01). |
| **Verdict-decoupled block source** | `ade_node::node_sync::NodeBlockSource` + `run_node_sync` | RED | Yields ordered peer-block BYTES only (skips `TipUpdate`, ends on `Disconnected`; N-F-D added a content-blind readiness signal; N-F-G-C added `from_wire_pump` — a FILL of the WirePump arm); `run_node_sync` is the production caller of `forward_sync::pump_block` (durable-before-tip, DC-SYNC-01). NEVER carries/derives a verdict. Driven on the live path by the N-F-D relay loop. |
| **Authoritative durable apply** | `ade_runtime::forward_sync::pump_block` (carried fwd, N-Y) | RED + BLUE admit | The durability-ordered driver the source feeds; the BLUE admit chokepoint + `StoreBlockBytes`/`AppendWal`-before-`AdvanceTip` invariant live here, not in the source. |
| **Evidence correlator (compares already-authoritative outputs)** | `ade_node::ba02_evidence` (`parse_peer_accept_events`, `correlate`) | GREEN | COMPARES the BLUE-minted forged hash (read verbatim, never recomputed) against an operator-captured peer-accept signal; `correlate` is the SOLE `Ba02Manifest` constructor. Forges/admits/persists nothing. RO-LIVE-06. _N-F-G-C: now CONSUMED by the RED `ba02_pass` file-I/O wrapper (still reached by no binary arm; tested only)._ |
| **Evidence file I/O (N-F-G-C)** | `ade_node::ba02_pass` (`correlate_peer_log_file`, `write_ba02_manifest`) | RED | Reads the operator-captured peer-log file into the GREEN `correlate`; `write_ba02_manifest` accepts ONLY a `Ba02Manifest`. A missing/unreadable file fails closed (`io::Error`). Constructs no evidence; reached by no binary arm at this HEAD. RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01. |

**Rule (CN-NODE-01 / DC-SYNC-01 / RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01):** the lifecycle owner ROUTES (single
`bootstrap_initial_state` authority on both arms); the block source is data-only (bytes, never a verdict; the
N-F-G-C live `WirePump` fill stays a closed verdict-decoupled contract); the durable apply + admit authority stay
in `pump_block`; the BA-02 correlator is GREEN evidence whose SOLE constructor admits no self-evidence acceptance
source and emits no committed synthetic manifest, and the N-F-G-C RED `ba02_pass` is file I/O ONLY over it
(`write_ba02_manifest` accepts ONLY a `Ba02Manifest`). **None of these chokepoints move.** **Honest scope:**
`ba02_evidence` + `ba02_pass` are reached by no binary arm (tested only); `node_sync` is driven on the live path
only by the N-F-D relay loop (SyncOnce — now from a LIVE feed with `--peer`) + the forge tick (ForgeTick, with a
`ForgeActivation`). **BA-02 is satisfied NOWHERE at this HEAD; RO-LIVE-01 remains partial/operator-gated.**

### Domain: bootstrap seed provenance (N-Y, extended N-Z + N-F-A; UNTOUCHED by N-F-F / N-F-G-A / N-F-G-C)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only Mithril shell** | `ade_runtime::mithril_import::{json, importer}` | RED | `parse_mithril_manifest_json` is the SOLE manifest-JSON parser; `import_mithril_manifest` maps it into the closed `SeedProvenance::Mithril` + `MithrilManifestReport`. No semantic decision; never re-verifies the STM multisig. |
| **Mithril production-bootstrap composition** *(N-Z; +N-F-A sidecar tail; N-F-C first non-test caller)* | `ade_runtime::mithril_bootstrap::bootstrap_from_mithril_snapshot` | RED | **Composition-only**: import → mint anchor from the operator-independent `MithrilSeedPointInputs` → BLUE `verify_mithril_binding` fail-closed BEFORE storage init → single `bootstrap_initial_state` (NotRequired) → N-F-A sidecar tail. Symmetric with `bootstrap_from_conway_genesis`. No new authority, no new `SeedProvenance` variant, no CLI surface. Closed `MithrilBootstrapError`. **N-F-F/N-F-G-A/N-F-G-C do NOT call this** — operator-key ingress / forge fidelity / live feed is not a bootstrap. |
| **Data-only genesis shell** | `ade_runtime::genesis_bootstrap::bootstrap_from_conway_genesis` + `producer::genesis_parser` | RED | Reads + parses the Conway genesis file; routes the transform through the single bootstrap authority; runs the same sidecar tail. NOT on the N-F-C node lifecycle (FirstRun is Mithril-only). The N-F-F/N-F-G-A ingress reuses `genesis_parser` (N-F-G-A: the REAL `parse_shelley_genesis`) for clock/KES/network anchor extraction ONLY. |
| **Authoritative binding predicate** | `ade_ledger::bootstrap_anchor::binding::verify_mithril_binding` | BLUE | The sole authority deciding whether a Mithril anchor binds — a pure predicate cross-checking `{network_magic, genesis_hash, certified_point, certificate_hash}`; fails closed with `MithrilImportError`. |
| **Authoritative genesis transform** | `ade_ledger::genesis_source::genesis_initial_state` | BLUE | The pure Conway-only transform; fail-closed `GenesisSourceError::NonConwayEra`. |
| **Single bootstrap chokepoint** | `ade_runtime::bootstrap::bootstrap_initial_state` | GREEN-by-content (+ RED A3b restore) | The ONE authority all initial state flows through — `genesis_bootstrap`, the N-Y Mithril provenance path, the N-Z composition, the N-F-A warm-start restore, AND both N-F-C lifecycle arms all route here. Returns the named `BootstrapState`; the `SeedEpochConsensusSource` discriminant selects cold-start vs. warm-start restore. The N-F-F On arm reuses the resulting recovered `BootstrapState` as the forge base — NEVER a second bootstrap. |

**Rule (CN-MITHRIL-01 / CN-NODE-01 / DC-GENESIS-SRC-01 / DC-MITHRIL-02):** the RED shells parse bytes and
produce reports/configs / mint anchors; **all** semantic decisions live in BLUE. All initial state routes
through the **single** `bootstrap_initial_state` authority. **There is NO `GenesisAnchor` / `MithrilAnchor`
trait or plugin seam.** `verify_mithril_binding` MUST NOT be tautological. New seed-source support adds a RED
parse/map shell + (if a new authoritative decision is needed) a BLUE predicate/transform + (for production
wiring) a composition-only RED twin of `bootstrap_from_{conway_genesis, mithril_snapshot}`; **the
`bootstrap_initial_state` chokepoint never moves.** **N-F-F/N-F-G-A/N-F-G-C note:** Mithril stays the
bootstrap/recovery layer untouched — a bootstrap accelerator, not a validation/forge shortcut; operator-key
ingress / forge fidelity / the live feed neither calls Mithril nor creates a second bootstrap.

### Domain: network forward-sync (durable-before-tip, N-Y; first production driver N-F-C; driven by the N-F-D loop; LIVE feed N-F-G-C)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Effect-plan reducer** | `ade_runtime::forward_sync::reducer` (`forward_sync_step`, `AdmitPlan::durable`) | GREEN-by-content | Composes the BLUE admit chokepoint and emits the closed `SyncEffect` plan. The private `AdmitPlan::durable` is the **sole** `AdvanceTip` emitter and fixes the durable-before-tip order — an out-of-order plan is structurally inexpressible. |
| **Durability-ordered driver** | `ade_runtime::forward_sync::pump` (`pump_block`) | RED | Applies the reducer's `SyncEffect` plan in order against the persistent `ChainDb` + `FileWalStore` + snapshot writer; refuses to advance the tip before `StoreBlockBytes` + `AppendWal` return Ok — `PumpError::TipBeforeDurable`. Its production caller is `node_sync::run_node_sync`, driven each iteration by the N-F-D relay loop's `SyncOnce`. N-F-G-C: the bytes it folds now come from a LIVE `NodeBlockSource::WirePump` (reusing the closed admission dial + pump) when `--peer` is supplied. |

**Rule (DC-SYNC-01 / DC-SYNC-02):** the GREEN reducer decides the effect plan; the RED pump applies it in
durable order. This GREEN-reducer / RED-pump split mirrors the `ade_network::session` (GREEN) /
`ade_runtime::network::mux_pump` (RED) split. `AdvanceTip` is unreachable before `StoreBlockBytes` +
`AppendWal` (`ci_check_forward_sync_chokepoint_only.sh`). N-F-C's `run_node_sync` advances the tip ONLY via
`pump_block` (`ci_check_node_sync_via_pump.sh`); N-F-D's relay loop drives `run_node_sync` each iteration;
N-F-G-C feeds it a LIVE peer (the feed-side RO-LIVE-01 follow-on, now wired) without changing the
single-`AdvanceTip`-emitter chokepoint. New sync logic adds `SyncEffect` variants + reducer arms; **the
single-`AdvanceTip`-emitter chokepoint never moves.** An **acceptance-criterion** seam, not a registry-law surface.

### Domain: crash recovery (N-Y, extended N-F-A provenance fold; production restart wired N-F-C; loop-as-replay N-F-D/N-F-E/N-F-G-A)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Recovery wiring (test-only entry)** | `ade_runtime::recovery::restart::recover_node_state` | RED | Composes the EXISTING authorities (`WalStore::read_all` + BLUE `wal::replay_from_anchor` + `rollback_to_slot`) to reconcile the ChainDb to the WAL tail. No second recovery engine. Fail-fast on `WalTailFingerprintMismatch`. Still the test-only secondary entry — the PRODUCTION restart path is the N-F-C WarmStart arm. |
| **Production restart path** | `ade_node::node_lifecycle::warm_start_recovery` | RED | The WarmStart arm: anchor-lineage discovery → WAL replay → restore through the single `bootstrap_initial_state(RequiredFromRecoveredProvenance)` authority. No second recovery engine; fail-closed. Converges into `run_relay_loop`. |

**Rule (recovery-contract / DC-WAL-*; restart wired N-F-C; loop-as-replay N-F-D/N-F-E):** recovery composes
existing authorities; it never re-implements replay or rollback (`ci_check_recovery_contract.sh`). The N-F-A
`ReplayOutcome` additively carries the recovered seed-epoch provenance without disturbing the `AdmitBlock`
fp-chain. **N-F-D/N-F-E extend replay-equivalence to continuous operation (T-REC-03):** the same
recovered/bootstrapped state + the same ordered canonical block feed + the same deterministic loop inputs +
(N-F-E) the same injected clock-tick + shutdown schedule produce byte-identical authoritative outputs.
**N-F-G-A note:** the S1 genesis-consistency pinning harness drives the REAL `bootstrap_initial_state`
warm-start against the committed Ade-as-leader reference fixture and pins the recovered values — the fixture
is **evidence input, never runtime authority**. `warm_start_recovery` is the single production restart owner
(CN-NODE-01).

### Domain: N2N tag-24 wire envelope (N-X)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sole byte wrap/unwrap authority** | `ade_codec::cbor::tag24::{wrap_tag24, unwrap_tag24}` | BLUE | The single workspace authority that wraps inner bytes in a tag-24 (`0xd8 0x18`) envelope and strips it. `unwrap_tag24` returns a zero-copy borrow (no re-encode); fails closed with `TagEnvelopeError`. Each defined exactly once. |
| **BlockFetch composition** | `ade_network::codec::block_fetch::{compose,decompose}_blockfetch_block` | BLUE | A served `MsgBlock` payload = `tag24(bytes([era, block]))` — era **inside** the wrap; **Conway = storage index 7**. |
| **ChainSync composition** | `ade_network::codec::chain_sync::{compose,decompose}_rollforward_header, chain_sync_wire_era_index}` | BLUE | A served `RollForward` header = `[era_tag, tag24(bytes(header_cbor))]` — era_tag **outside** the wrap; **CONSENSUS era index, Conway = 6 = storage − 1**. |
| **Serve emitters** | `ade_network::block_fetch::server` / `chain_sync::server` | BLUE | Emit composed (tag-24-wrapped) bytes — never a bare `[era, block]` / bare header. |
| **RED consumers (migrated)** | `ade_node::admission::runner` + `ade_core_interop::follow` | RED | Strip a peer's tag-24 envelope via `ade_codec::unwrap_tag24`; no local parse. _(N-F-G-C: the live `--mode node` feed reuses `admission::runner`'s closed pump path verbatim.)_ |

**Rule (CN-WIRE-08):** one tag-24 byte authority + per-protocol composition layered over it. The two N2N
surfaces use **different era-index schemes** (BlockFetch storage Conway = 7; ChainSync consensus Conway = 6 =
storage − 1), pinned byte-identically against cardano-node 11.0.1 captures. No hand-rolled tag-24 parse in
RED. **The wrap/unwrap chokepoint never moves.**

### Domain: block codec (decode + encode)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative ingress** | `ade_codec::cbor::envelope::decode_block_envelope` + per-era `decode_*_block` | BLUE | Sole `PreservedCbor` construction site; operates over the verbatim tag-24-stripped inner bytes on the wire path (N-X). |
| **Authoritative egress (N-V)** | `ade_codec::cbor::envelope::encode_block_envelope` | BLUE | The single block-envelope encoder; emits storage-form `[era, block]` (Conway = discriminant 7, head `82 07`). |
| **Producer consumer** | `ade_ledger::producer::forge::forge_block` | BLUE | Wraps forged output via `encode_block_envelope`. |

**Rule (CN-FORGE-03, strengthened N-X):** one block-envelope grammar in both directions; forge and validate
share it. The on-wire serve form is the N-X tag-24 composition over this storage-form. **The encode/decode
chokepoint pair never moves.**

### Domain: leader-eligibility VRF input (N-W)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sole era→construction authority** | `ade_core::consensus::vrf_cert::leader_vrf_input(era, slot, eta0)` | BLUE | The single place selecting a Praos vs TPraos leader-eligibility VRF construction; returns the closed `ExpectedVrfInput`. |
| **Era-correct range-extension** | `ade_core::consensus::vrf_cert::leader_value_for` | BLUE | Praos `praos_leader_value` vs TPraos identity, dispatched on the `ExpectedVrfInput` variant. |
| **Leader-schedule producer** | `ade_core::consensus::leader_schedule::query_leader_schedule` | BLUE | Builds `LeaderScheduleAnswer.expected_vrf_input` via `leader_vrf_input`. Called by `forge_one_from_recovered` AFTER the N-F-G-A off-epoch guard. |
| **RED prove-step consumer** | `ade_node::produce_mode::run_real_forge` | RED | Proves over `answer.expected_vrf_input.alpha_bytes()`; non-Praos era fail-closes. Reused by `node_sync::forge_one_from_recovered` (the N-F-E/N-F-F/N-F-G-A forge tick's BLUE engine). |

**Rule (CN-FORGE-04):** exactly one VRF transcript authority per era/protocol version; the Praos producer
alpha MUST equal the validator alpha. No both-alphas fallback. **The era→VRF construction chokepoint never
moves.**

### Domain: KES signing-key custody (real operator KES ingestion in `--mode node` since N-F-F)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only loader (shared)** | `ade_runtime::producer::keys::load_kes_signing_key_skey` / `produce_mode::load_kes_skey_any_format` | RED | Reads the 608-byte cardano-cli skey envelope. `load_kes_skey_any_format` is `pub(crate)` (N-F-F) and REUSED verbatim by `operator_forge::load_operator_producer_shell` — no reimpl. |
| **Authoritative deserializer** | `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes` | BLUE | Byte layout is the structural validator. UNCHANGED. |
| **Authoritative algorithm** | `ade_crypto::kes_sum` | BLUE | Ade-native Sum6KES, byte-identical to Haskell `cardano-base`. UNCHANGED. |
| **Signing operation** | `ade_runtime::producer::signing` / `producer_shell::{kes_sign_header, kes_sign_header_advancing}` | RED | Sole key-custody surface; signs only the branded `UnsignedHeaderPreImage`, emitting the branded `KesSignature`. **N-AC (DC-CRYPTO-10): the forge's single real KES sign now calls `kes_sign_header_advancing`** — `kes_advance_to(period)? then kes_sign_header(period, preimage)` — EVOLVING the operator KES key FORWARD to the requested period via the EXISTING deterministic Sum6KES `kes_update` BEFORE signing, so signing works across the opcert's KES window (not only the minted-at-0 period); fail-closed `Signing(EvolutionBackwards)` (backwards / forward-secrecy) / `Signing(EvolutionExhausted)` (beyond `SUM6_MAX_PERIOD = 63`); the `period` is passed VERBATIM. The `&self` `kes_sign_at` / `kes_sign_header` REMAIN for period-managing callers. |
| **`--mode node` operator-material ingress (N-F-F; real parsers N-F-G-A)** | `ade_node::operator_forge::{load_operator_producer_shell, build_operator_forge_material}` | RED | The single named `--mode node` operator-material ingress site: REUSES the KES/VRF/cold loaders + (N-F-G-A) the real `parse_opcert_envelope` / `parse_shelley_genesis` → `ProducerShell::init` → `OperatorForgeMaterial` (custody shell, not `Debug`/`Serialize`). Key custody RED-confined to `ProducerShell`; passed only to the fenced `forge_one_from_recovered`, never copied/logged/serialized/hashed-for-evidence (CN-NODE-03). |

**Rule (CN-NODE-03):** the RED loader may not call `KesSecret::from_*` inside `load_kes_signing_key_skey` /
`operator_forge` — only the BLUE deserializer path. Signing is RED-confined; BLUE never signs. **N-F-F:
`--mode node` ingests REAL operator KES/VRF/cold/opcert material** via the single named `operator_forge`
ingress site, which REUSES the existing loaders (N-F-G-A: + the real opcert/genesis parsers; no reimpl, no
new BLUE authority, no plugin seam) and keeps custody confined to `ProducerShell`. The forge tick still
reuses `CoordinatorState::kes_period_for_slot`. **The custody/signing chokepoint never moves.** **N-AC
(DC-CRYPTO-10): the forge's single real KES sign EVOLVES the key to the chain's current KES period before
signing** (`kes_sign_header_advancing` = `kes_advance_to(period)? then kes_sign_header(period, …)`), using the
EXISTING deterministic Sum6KES `kes_update` (idempotent at the current period; fail-closed backwards /
beyond-lifetime); the period is passed VERBATIM; signing stays RED; the BLUE Sum6KES algorithm + KES verifier are
UNCHANGED. NO new BLUE authority / canonical type; NO new closed enum (the existing `SigningError` variants +
branded `KesSignature` / `UnsignedHeaderPreImage` are reused). `CN-KES-HEADER-01` `strengthened_in +=
"PHASE4-N-AC"` (the real-pre-image sign now works across KES periods, not only the minted one)
(`ci_check_kes_evolution_before_sign.sh`).

### Domain: leader eligibility (RED/BLUE split)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **VRF proof producer** | `ade_node::produce_mode` (prove-step) | RED | Produces the VRF proof/output over the BLUE answer's `expected_vrf_input.alpha_bytes()`. |
| **Authoritative evaluator** | `ade_core::consensus::leader_check::verify_and_evaluate_leader` | BLUE | Verifies + evaluates eligibility from canonical inputs only; emits the closed `LeaderCheckVerdict`. |

**Rule (CN-FORGE-02):** BLUE never sees the VRF/KES/cold keys; the evaluator has no
`LedgerView`/`EraSchedule`/`ChainDepState`/clock/storage/RED dep. The RED/BLUE split never moves.

### Domain: self-accept→serve handoff — typed carrier vs. single serve authority (GREEN carrier / RED sibling task / BLUE admit-behind-seams; N-F-G-B) — `--mode node` serve usage SUPERSEDED by N-U S3 / DC-NODE-13

> **SUPERSEDED on the `--mode node` spine (N-U S3 / DC-NODE-13, repoint `60deecf3`).** The `--mode node` serve no
> longer routes a forged block through this typed carrier → sibling `push_atomic` mechanism; it serves a READ-ONLY
> projection of the durable ChainDb (`ServedChainSource::DurableChainDb` over `Arc<dyn ChainDb>` — see the §2
> *forged-block serving* domain's N-U note and the durable-projection serve row). A forged block reaches the served
> view by becoming DURABLE (`admit_forged_block_durably` → `pump_block`, DC-NODE-12), not by a serve handoff. The
> fence `ci_check_served_chain_handoff_fence.sh` is REPOINTED to the durable-provenance serve (§5). **The carrier +
> `ServedChainHandle` below are RETAINED for `--mode produce` (CN-PROD-04, `ServedChainSource::Snapshot`) — a
> SEPARATE serve authority.** The table + rule below are the historical N-F-G-B `--mode node` mechanism.

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only handoff carrier (constructor-fenced)** | `ade_runtime::producer::self_accepted_handoff::SelfAcceptedHandoff` (sole ctor `from_self_accepted`, accessors `accepted()` / `into_accepted()`) | GREEN | A constructor-fenced newtype over the BLUE `ade_ledger::producer::AcceptedBlock`. Private field; the SOLE constructor takes an `AcceptedBlock` (itself producible only by BLUE `self_accept` returning `Ok`). NO raw-bytes / `ForgedBlockArtifact` / `CoordinatorEvent` / flag / verdict constructor — a non-self-accepted artifact is type-unrepresentable as a handoff. Pure; carries the ORIGINAL token verbatim, never re-validates or re-derives it. Decides nothing; moves a token. |
| **Forge token surfacing** | `ade_node::produce_mode::run_real_forge` (+ `run_real_forge_inner`) / `ade_node::node_sync::forge_one_from_recovered` | RED | `run_real_forge` returns `(CoordinatorEvent, Option<AcceptedBlock>)` — `Some` iff `ForgeSucceeded` (the out-param is written only on the success path). `forge_one_from_recovered` returns `(CoordinatorEvent, Option<SelfAcceptedHandoff>)`, wrapping via `self_accepted.map(SelfAcceptedHandoff::from_self_accepted)`. The token is the ORIGINAL from BLUE `self_accept` (CN-FORGE-01), never re-derived from `artifact.bytes`. |
| **Typed handoff channel + sibling serve task** | `ade_node::node_lifecycle` (`ForgeActivation.handoff_tx: Option<mpsc::UnboundedSender<SelfAcceptedHandoff>>`, `with_handoff_sender`, the `tokio::spawn` sibling admit task) | RED | The relay-loop body forwards each surfaced handoff as a best-effort TYPED `tx.send(h)` ONLY — it holds ONLY the `Sender`, never a `ServedChainHandle` / `push_atomic` / served-chain mutation (so `ci_check_node_run_loop_containment.sh` stays byte-unchanged). The dispatcher On arm spawns a SIBLING task that drains the typed channel and admits via the single `ServedChainHandle::push_atomic(handoff.into_accepted())` — the SOLE node-spine `push_atomic` site. _(N-F-G-C BROADENED the fence's owner set to `{node_lifecycle.rs, node_sync.rs}` + flipped guard-3 to an allow-list.)_ |
| **Atomic publisher (reused)** | `ade_runtime::producer::served_chain_handle::push_atomic` | RED (GREEN-by-content glue) | The SAME single served-admit authority `produce_mode` uses; wraps `served_chain_admit` in `watch::Sender::send_modify` (no torn snapshot). REUSED unchanged — fed ONLY by `into_accepted()` on the node spine. |
| **Authoritative admit (reused, behind seams)** | `ade_ledger::producer::served_chain::served_chain_admit` | BLUE | The SOLE entry into the served index; only self-accepted blocks (CN-PROD-04). UNCHANGED — reached only via `push_atomic`. |

**Rule (DC-NODE-06 / CN-PROD-04 / CN-FORGE-01):** the self-accept→serve handoff is a **typed channel seam +
the single reused serve authority** — NO new BLUE authority, NO new canonical type, NO new `CoordinatorEvent`
variant. **Only a BLUE self-accepted artifact can be served:** the `SelfAcceptedHandoff`'s sole provenance is
the `AcceptedBlock` from `self_accept` (private-field constructor fence); raw bytes / a failed outcome / a flag
/ a peer verdict are type-unrepresentable. The relay-loop body does a **typed `tx.send` only** — served-chain
mutation lives in the SIBLING task via the single `push_atomic` fed by `into_accepted()`
(`ci_check_served_chain_handoff_fence.sh`, BROADENED by G-C: owners `{node_lifecycle.rs, node_sync.rs}`; every
node-spine `push_atomic(` fed by `into_accepted()`; no direct `served_chain_admit(`; the allow-list requires
every node-spine unbounded handoff channel to carry `SelfAcceptedHandoff`). **None of these chokepoints move**;
`ci_check_node_run_loop_containment.sh` is byte-/semantically unchanged. **Honest scope:** N-F-G-C adds the
LIVE FEED around this serve mechanism; a self-accepted block is served byte-identically over IN-PROCESS
block-fetch (`live_feed_forge_serve_loopback_returns_forged_block`), but a served block reaching a peer that
ACCEPTS it is the operator-gated RO-LIVE-01 leg. **No peer-acceptance / BA-02 / RO-LIVE claim; BA-02 satisfied
nowhere.**

### Domain: forged-block serving (data-only serve vs. authoritative admit)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative admit** | `ade_ledger::producer::served_chain::served_chain_admit` | BLUE | Sole entry into the served index; only self-accepted blocks (CN-PROD-04). |
| **Atomic publisher** | `ade_runtime::producer::served_chain_handle::push_atomic` | RED (GREEN-by-content glue) | Wraps `served_chain_admit` in `watch::Sender::send_modify` — no torn snapshot. |
| **Read-side serve (snapshot — `--mode produce`)** | `ade_network::block_fetch::server::producer_block_fetch_serve` over `ade_runtime::producer::served_chain_lookups::ServedChainLookups` | BLUE reducer / RED impl | Serves a `RequestRange` only if endpoints + every intervening block are present; emits the tag-24 composition (N-X). The snapshot impl of the closed serve seams. |
| **Read-side serve (durable projection — `--mode node`, N-U; BOUNDED N-AA; recovered-anchor intersect N-AE.B)** | `ade_network::{chain_sync,block_fetch}::server` over `ade_runtime::network::served_chain_projection::ChainDbServedSource` | BLUE reducers / RED impl | The `--mode node` served view as a READ-ONLY projection of the durable ChainDb; serves `stored.bytes` verbatim, reuses `block_header_bytes` + `decode_block` (DC-NODE-13). A SECOND impl of the same closed serve seams. **N-AA (DC-SERVEMEM-01): the peer-driven BlockFetch read is now BOUNDED** — it reads via the bounded **hash-free** primitives `range_bytes_capped` + `last_block_bytes` (NOT the unbounded `iter_from_slot` / O(N) `chaindb.tip()`), caps each request at the fixed non-configurable `MAX_SERVE_RANGE_BLOCKS = 256`, derives the hash via the single BLUE `decode_block` (no second hash authority, no `SLOT_BY_HASH` on the serve path), and fails closed via the closed `ServeRangeOutcome` (`CapExceeded` / `Empty` / `ReadError`) → wire `NoBlocks`. **N-AE.B (DC-NODE-14): `ChainDbServedSource::intersect` gains a SECOND, proof-gated branch** — it advertises the `prev_hash` of the EARLIEST servable `StoredBlock` (the private bounded helper `earliest_servable_block_prev_hash`, reads exactly one block) as a **FindIntersect-ONLY** point IFF a real servable successor exists, so a relay that already holds the recovered/forged PARENT can FindIntersect there and roll forward onto the real successor; it serves **NO bytes** for that point (`get_block_by_hash` / `serve_range` / BlockFetch refuse structurally — no synthetic `StoredBlock`, no synthetic CBOR), and recover-only (no `StoredBlock`) / `PrevHash::Genesis` fail closed (no projection). A CLOSED, proof-gated read-only behavior — NOT a registration / extension surface. |

**Rule:** a forged block is visible to peers only after `push_atomic`; the read-side serve is data-only over
the BLUE `ServedChainSnapshot`. The serve emitter wraps via the single tag-24 authority before bytes reach a
peer (CN-WIRE-08). **N-F-E/N-F-F/N-F-G-A note:** the relay-loop forge tick **body** does NOT touch this serve
path — `served_chain_admit` / `push_atomic` are forbidden in the relay-loop body, even on the forge-CAPABLE On
arm. **N-F-G-B note (DC-NODE-06 ENFORCED):** the forge tick SURFACES the original BLUE self-accepted
`AcceptedBlock` and the loop body forwards it as a TYPED `mpsc<SelfAcceptedHandoff>` send to a dispatcher-spawned
SIBLING served-chain admit task; the sibling task — NOT the loop body — admits via the single `push_atomic`
(fed by `into_accepted()`). The loop body still references no serve token, so containment is byte-unchanged. See
the self-accept→serve handoff domain above. **N-F-G-C note:** a self-accepted block is now served byte-identically
over IN-PROCESS block-fetch on the live-feed path; a served block leaving the node over the wire to a peer that
ACCEPTS it is the operator-gated RO-LIVE-01 leg (peer ACCEPT NOT claimed here).

**N-U note (DC-NODE-12 / DC-NODE-13):** the forge's OWN block now becomes **durable** — the relay-loop `ForgeTick` arm routes the self-accepted block through the fenced `node_sync::admit_forged_block_durably` → the single `pump_block` durable chokepoint (extend-only; the durable tip advances; the next forge builds N+1 on it; a stale-tip forge fails closed). The `--mode node` SERVE path is now a READ-ONLY **projection of the durable ChainDb** (`ChainDbServedSource` over `ServedChainSource::DurableChainDb`), read by the single `dispatch_server_frame_event_to_outbound` (DC-NODE-07 preserved — one dispatch, two read sources), NOT the retired G-R in-memory `ServedChainView` accumulator + `serve_gate_admits` monotone gate. The forge still advances NO durable tip DIRECTLY (`pump_block` is the sole durable tip authority); durability + coherent serve ≠ peer acceptance (`RO-LIVE-01` stays operator-gated). The snapshot serve (`ServedChainSnapshot` + `push_atomic`) above stays the `--mode produce` path; N-U does NOT move it. **N-AA note (DC-SERVEMEM-01):** the `--mode node` durable-projection serve READ is now BOUNDED — each peer BlockFetch range is read via the bounded hash-free `range_bytes_capped` + `last_block_bytes`, capped at the fixed `MAX_SERVE_RANGE_BLOCKS = 256` (symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD`), failing closed on an oversized / inverted / undecodable range; the single `dispatch_server_frame_event_to_outbound` (DC-NODE-07), the hash authority (single `decode_block`), within-cap byte-identity (DC-CONS-17), and the WRITE side (`pump_block`) are all unchanged. RED-only, 0 BLUE diff, no RO-LIVE flip.

**N-AE.B note (DC-NODE-14 — recovered-anchor intersectability; Option B locked):** after a recover-behind + follow, the live follow stores the followed lineage (incl. the peer tip) as servable `StoredBlock`s via `pump_block`, so the followed tip is FindIntersect-able for free (AE.A's followed-tip clause). For the forge PARENT specifically — the recovered/forged point a relay must stand on to adopt Ade's successor — `ChainDbServedSource::intersect` projects the `prev_hash` of the EARLIEST servable `StoredBlock` as a **FindIntersect-ONLY**, **proof-gated** point (advertised IFF Ade holds a real servable successor whose `prev_hash` equals an offered point), serving **NO bytes** for it (Option A — materialize anchor bytes — was explicitly NOT taken: hash-critical paths require preserved original bytes, and the relay already has the parent block). This is the recovered-anchor clause that flips DC-NODE-14 to `enforced`; it composes with the AE.E chain-sync server FindIntersect cursor fix (DC-PROTO-10) so the relay rolls forward onto the forged successor rather than block 0. **This is a TIGHTENING of the existing DC-NODE-13 read-only projection (cross-referenced — the registry carries no `strengthened_in += "PHASE4-N-AE"` on DC-NODE-13; the formally-introduced rule is the new DC-NODE-14 recovered-anchor clause) plus a strengthening of CN-CONS-07 / DC-CONS-23, NOT a new seam or registry** — new work still attaches to the serve surface by extending the durable ChainDb via the sanctioned writers `pump_block` / `bootstrap_initial_state`; the projection is a consequence, not an attach point. Proven LIVE: a real cardano-node 11.0.1 relay `AddedToCurrentChain` an Ade-forged successor (CE-A5 manifest, venue c2ae18). RED-only on the projection (the lone BLUE change is the additive read-only `DecodedBlock.prev_hash`); no RO-LIVE flip on the rehearsal venue.

---

## 3. Closed vs. Extensible Registries

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `ServedChainSource` *(NEW, N-U / DC-NODE-13 — serve BOUNDED N-AA / DC-SERVEMEM-01; DC-NODE-07 PRESERVED)* | `ade_runtime::network::serve_dispatch` (RED) | 2 (`Snapshot(&ServedChainView)` / `DurableChainDb(&dyn ChainDb)`) | The closed read-source selector read by the SINGLE serve-dispatch authority `dispatch_server_frame_event_to_outbound`. `Snapshot` is the `--mode produce` in-memory `ServedChainSnapshot`/`ServedChainView` accumulator path; `DurableChainDb` is the `--mode node` durable-chain projection (NEW in N-U — `ChainDbServedSource`). **ONE dispatch, TWO read sources** — the serve authority does NOT fork; the source enum selects which read-side the single dispatch reads from (DC-NODE-07 preserved — `ci_check_served_chain_projection.sh` pins exactly one `dispatch_server_frame_event_to_outbound`). **NOT `#[non_exhaustive]`**; the dispatch `match` is total with NO wildcard. **A closed read-source selector (a surface REDUCTION), NOT an extension point.** Backs **DC-NODE-13**. A new read source = a `ServedChainSource` variant + a `dispatch_server_frame_event_to_outbound` arm (no wildcard) + a strengthening of **DC-NODE-13** (`ci_check_served_chain_projection.sh`); the single serve dispatch MUST NOT be duplicated, and the new read source MUST implement the same closed serve seams (`ServedHeaderLookup` / `ServedRangeLookup`). |
| `ServeRangeOutcome` *(NEW, N-AA / DC-SERVEMEM-01)* | `ade_runtime::network::served_chain_projection` (RED, `//! RED`) | 4 (`Served(Vec<(SlotNo, Hash32, Vec<u8>)>)` / `Empty` / `CapExceeded` / `ReadError`) | The closed internal outcome of the BOUNDED `--mode node` BlockFetch serve read. **Every non-`Served` variant maps to an empty `Vec` → the BLUE reducer's wire `NoBlocks`** — `CapExceeded` on an oversized range (decided BY THE FIXED BOUND `MAX_SERVE_RANGE_BLOCKS`, **before** any decode), `Empty` on an out-of-chain or inverted (`from > to`) window, `ReadError` on an in-range block the single BLUE `decode_block` cannot authenticate (the serve never emits a block it cannot authenticate). **NOT `#[non_exhaustive]`; no catch-all.** It is a serve-READ outcome, **NOT a dispatch** (the single `dispatch_server_frame_event_to_outbound` is unchanged — DC-NODE-07 preserved); lives in `ade_runtime` (not a BLUE `core_paths` entry), so **NOT canonical-counted**. **A closed fail-closed reason vocabulary (a surface REDUCTION), NOT an extension point.** Backs **DC-SERVEMEM-01**. A new reason = a `range_bytes_with_outcome` arm (no wildcard) + every non-`Served` arm still mapping to `NoBlocks` + a strengthening of **DC-SERVEMEM-01** (`ci_check_serve_range_bounded.sh`); the fixed cap MUST NOT become configurable, and no second block-hash authority may be introduced. |
| `CappedSlotRange` *(NEW, N-AA / DC-SERVEMEM-01)* | `ade_runtime::chaindb::types` (RED) | closed value struct (`blocks: Vec<(SlotNo, Vec<u8>)>` / `truncated: bool`) | The result of the bounded **hash-free** ChainDb read primitive `range_bytes_capped(from, to, max)`: `blocks` holds at most `max` `(slot, bytes)` pairs in slot-ascending order; `truncated = true` when the requested range contained MORE than `max` blocks (the per-request serve cap was exceeded) — the serve uses `truncated` to fail closed (`ServeRangeOutcome::CapExceeded`) and to distinguish "cap exceeded" from "genuinely empty". A RED storage value type, **NOT canonical-counted**. **A closed value struct (a surface REDUCTION), NOT an extension point.** Backs **DC-SERVEMEM-01**. A new field = a struct addition behind the bounded-read contract + a strengthening of **DC-SERVEMEM-01**; the primitive MUST stay hash-free (no `SLOT_BY_HASH` scan) and read at most `max + 1` blocks lazily. |
| `MAX_SERVE_RANGE_BLOCKS` (closed serve-range cap) *(NEW, N-AA / DC-SERVEMEM-01)* | `ade_runtime::network::served_chain_projection` (RED) | closed literal const `256` | The per-request `--mode node` serve-range cap, **SYMMETRIC with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD = 256` (DC-LIVEMEM-01)**. A **CLOSED LITERAL constant — a defensive implementation bound, NOT a Cardano semantic parameter** (it does not change which chain is served, only the per-request work ceiling); **NO runtime / CLI / env / config override, no unbounded mode** (the no-escape-hatch surface reduction, `ci_check_serve_range_bounded.sh`). A future hardening slice may **tighten** it (a strengthening of **DC-SERVEMEM-01**), but may NEVER make it a tunable / unbounded. Recorded in §4 Frozen alongside `MAX_REASSEMBLY_TAIL_BYTES` / `MAX_WIRE_PUMP_LOOKAHEAD`. |
| Chain-sync server FindIntersect cursor *(NEW behavior, N-AE.E / DC-PROTO-10)* | `ade_network::chain_sync::server` (BLUE — a `core_paths` submodule; `producer_chain_sync_serve` FindIntersect handler) | server-INTERNAL `last_announced` cursor state; **the closed `Point` enum + the `ChainSyncMessage` grammar are UNCHANGED** | After answering `IntersectFound(point)` the producer chain-sync server sets its read cursor `last_announced` TO the resolved intersect — `Block → Some((slot,hash))` / `Origin → None` — so the next `RequestNext` serves `next_after(point)` (the successor the client rolls onto), never `next_after(None)` = block 0 (which a client intersecting at its own tip rejected as `UnexpectedBlockNo(tip+1)(0)`; Origin keeps `None` = serve-from-start, correct — why earlier Origin-sync producer-serve clusters passed). **This is a server-INTERNAL cursor-STATE fix, NOT a wire-grammar change** — `FindIntersect` / `IntersectFound` / `RequestNext` / `RollForward` are the EXISTING messages, the closed `Point` enum (`Point::Origin` stays `array(0)`, `Point::Block` stays `[slot, hash32]`) is reused UNCHANGED, and no message / variant / negotiated surface is added. **NOT an extension point — a correctness tightening of the existing closed chain-sync server.** Backs **DC-PROTO-10** (test-enforced: `producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`; `ci_script = ""`). A change to the cursor rule = a strengthening of **DC-PROTO-10**; the closed `Point` enum + `ChainSyncMessage` grammar MUST NOT be widened, and a non-Origin intersect MUST set the cursor (never serve block 0 to a tip-pointed client). |
| Recovered-anchor FindIntersect projection (cross-ref §2) *(NEW behavior, N-AE.B / DC-NODE-14)* | `ade_runtime::network::served_chain_projection` (RED — `ChainDbServedSource::intersect` second branch + private `earliest_servable_block_prev_hash`) | a proof-gated **FindIntersect-ONLY** read-only behavior over the EXISTING closed serve seams; **no new enum / no new serve dispatch** | `ChainDbServedSource::intersect` advertises the `prev_hash` of the EARLIEST servable `StoredBlock` (the forge PARENT) as a FindIntersect point **IFF** Ade holds a real servable successor whose `prev_hash` equals an offered point; it serves **NO bytes** for that point (`get_block_by_hash` / `serve_range` / BlockFetch refuse structurally — no synthetic `StoredBlock`, no synthetic CBOR), and recover-only (no `StoredBlock`) / `PrevHash::Genesis` fail closed (no projection). Option A (materialize anchor bytes) was explicitly NOT taken. **This is a CLOSED, proof-gated read-only PROJECTION behavior over the existing `DurableChainDb` serve source (DC-NODE-13) — NOT a new registry, NOT an extension point** (no `register_*` / plugin / handler map; a single closed `match`/`for` over the already-closed `Point` enum). Backs **DC-NODE-14** (the recovered-anchor clause); it TIGHTENS the **DC-NODE-13** serve-projection surface (cross-ref — NOT formally `strengthened_in`-tagged in the registry) and is a strengthening of **CN-CONS-07** / **DC-CONS-23**. The lone additive BLUE symbol it depends on — the read-only `DecodedBlock.prev_hash` (already parsed) — adds NO canonical type (458 unchanged). `ci_check_recovered_anchor_intersectable.sh` fences FindIntersect-only + proof-gated + no synthetic bytes. New work still attaches to the serve surface by extending the durable ChainDb via the sanctioned writers `pump_block` / `bootstrap_initial_state`; this projection is a consequence, not an attach point. |
| `ArrayHead` *(NEW, N-F-G-M / CN-WIRE-11)* | `ade_network::codec::primitives` (BLUE) | 2 (`Definite(u64)` / `Indefinite`) | The closed two-form CBOR array-head grammar for real cardano-node ChainSync `FindIntersect`. The closed BLUE fn `decode_array_head_two_form(protocol, data, offset)` decodes BOTH the definite (`9f…`/`98…`) and the indefinite (`0x9f … 0xff`) forms; on `Indefinite` the caller consumes the matching `0xff` break. **NOT `#[non_exhaustive]`.** The codec/ BLUE submodule count `38 → 39` (total **457 → 458**) — the LONE new BLUE canonical type in the whole G-K…G-R span. **SCOPED to ChainSync `FindIntersect`** (`decode_find_intersect_points` consumes it; the rest of the closed wire grammar stays definite-only — `ArrayHead` / `decode_array_head_two_form` is NOT a general definite/indefinite decoder); **no catch-all**. Paired with the `chain_sync::server` `Origin → IntersectFound[Origin]` reply (the universal common ancestor — matches the real node, does NOT widen the served chain). Pinned against captured cardano-node 11.0.1 FindIntersect/IntersectFound fixtures. **A closed BLUE sum (a surface REDUCTION for real-node wire compat), NOT an extension point.** Backs **CN-WIRE-11**. New variant = a `decode_array_head_two_form` arm + a strengthening of **CN-WIRE-11** (`ci_check_chainsync_findintersect_compat.sh`); the indefinite acceptance MUST stay scoped to FindIntersect, and the `Origin` reply MUST NOT widen the served chain beyond the Origin intersect. |
| Per-version N2N `versionData` encoding (`encode_n2n_version_params`) *(NEW fn, N-F-G-L / CN-WIRE-10)* | `ade_network::handshake::version_table` (BLUE) | the SINGLE per-version `versionData` encoder over the **unchanged closed `N2N_SUPPORTED` version SET** (V11..=V15 → the 4-field `[networkMagic, diffusionMode, peerSharing, query]`; V16+ → the extended shape) | The single per-version N2N `versionData` wire encoding — for V11..=V15 the 4-field `NodeToNodeVersionData` (`diffusionMode = true`, `peerSharing = NoPeerSharing(0)`, `query = false`), for V16+ the extended shape — so a real cardano-node accepts the serve-side handshake at NodeToNodeV_15 (fixes `HandshakeDecodeError NodeToNodeV_15 'unknown encoding: TInt 1'`). An additive BLUE **fn** (no struct/enum — 458 unchanged) over the unchanged closed `N2N_SUPPORTED` version SET. **NO version-set widening, NO runtime negotiation of meaning** (a successful handshake is a wire-layer event, NOT peer acceptance — `RO-LIVE-01` stays `partial`). **A closed per-version wire encoder (a surface REDUCTION / real-node-compat strengthening of the closed version table), NOT an extension point.** Backs **CN-WIRE-10**. A new per-version shape = a `encode_n2n_version_params` arm + a strengthening of **CN-WIRE-10** (`ci_check_n2n_handshake_versiondata_authority.sh` — the single per-version encoder + the per-version field shape, pinned against the real-node handshake fixture); the closed `N2N_SUPPORTED` version SET MUST NOT be widened and no runtime negotiation of meaning may be introduced. |
| Feed-side BlockFetch tag-24 unwrap (REUSES the frozen authority) *(N-F-G-O / CN-WIRE-12)* | `ade_runtime::admission::wire_pump` (RED) | REUSES the single `ade_codec::unwrap_tag24` (CN-WIRE-08) authority; **NO new serializer / parallel parser** | The feed-side WirePump strips the peer's BlockFetch `MsgBlock` tag-24 (`0xd8 0x18`) CBOR-in-CBOR envelope via the single `ade_codec::unwrap_tag24` authority **BEFORE** the BLUE decode (so the live feed decodes the inner `[era, block]` cleanly — `UnexpectedType` gone), mirroring the already-correct `admission::runner` + `ade_core_interop::follow` paths. **There is NO second `unwrap_tag24` / hand-rolled tag-24 parse** — the single `CN-WIRE-08` (N-X) authority is reused; a non-tag-24 / malformed payload fails closed. **A closed decode-boundary reuse (a surface REDUCTION over the frozen tag-24 authority), NOT a new closed enum / parallel envelope / extension point.** Backs **CN-WIRE-12**. Adding a feed-side unwrap site = a strengthening of **CN-WIRE-12** (`ci_check_feed_tag24_unwrap.sh` — the feed-side WirePump unwraps via `ade_codec::unwrap_tag24` before decode, and adds no hand-rolled tag-24 parse); no parallel tag-24 authority may be introduced. |
| ~~`serve_gate_admits` monotone serve gate~~ **RETIRED (N-U S3 — superseded by serve-as-projection, DC-NODE-13)** *(was N-F-G-R / DC-NODE-11)* | ~~`ade_node::node_lifecycle`~~ — REMOVED from source (only retirement comments + the retired-test marker remain at `node_lifecycle.rs:276` / `:1767`) | — | **RETIRED.** N-U S3 supersedes the monotone serve-gate MECHANISM with serve-as-projection: the `--mode node` served view is now a READ-ONLY projection of the **durable** ChainDb (`ChainDbServedSource` over `ServedChainSource::DurableChainDb`), and the durable chain is extend-only (DC-CONS-23), so it holds exactly one block 0 by construction and a follower fetches coherent history A→B (never B without A) — **more strongly** than the in-memory `serve_gate_admits` predicate, and serving now **survives restart** (the durable ChainDb is recovered, T-REC-05; the accumulator was not). The DC-NODE-11 INVARIANT (*a follower sees a stable, coherent served block 0*) is **preserved + strengthened**, not weakened; its `ci_script` migrated `ci_check_served_chain_stability.sh` (RETIRED) → `ci_check_served_chain_projection.sh`. The pure predicate `serve_gate_admits` + its in-module test are gone from source. See the durable-chain serve projection in §3 Extensible + the §2 forged-block serving domain. |
| Forge-successor position from the evolved admitted spine *(NEW, N-F-G-Q / DC-NODE-10)* | `ade_node::node_sync` (RED — `forge_one_from_recovered` threading) | reuses the closed G-J `forge_header_position` convention; **NO new variant / `NodeBlockSource`** | After the genesis-successor block 0 is admitted, the NEXT forge derives `(block_number, prev_hash) = (last_admitted_block_no + 1, PrevHash::Block(last_admitted_hash))` from the **evolved admitted spine state** (the position the just-admitted block established), NOT the stale recovered tip — yielding the first stable node (block 1+, no successor crash). It reuses the SINGLE G-J `forge_header_position` convention; the forge still holds NO `ChainDb` handle (advances no durable tip — durability is N-U). **A closed RED forge-position threading (a surface REDUCTION), NOT an extension point.** Backs **DC-NODE-10**. A change to the successor position = a strengthening of **DC-NODE-10** (`ci_check_forge_successor_evolved_spine.sh` — the successor forge position is sourced from the evolved admitted spine, not the stale recovered tip); no new variant, no second forge codepath, no RO-LIVE flip. |
| Feed header-validation view from the recovered consensus surface *(NEW, N-F-G-P / DC-CINPUT-04)* | `ade_node::node_lifecycle` (RED — feed header/ledger Step-5/7 validation) | sources the leader-threshold view via the BLUE `PoolDistrView::from_seed_epoch_consensus_inputs`; **NO new closed enum** | The feed's header-step + ledger Step-5/7 validation derives its leader-threshold stake-distribution view (+ the VRF-keyhash / ASC / total-active-stake inputs) from the **recovered consensus surface** (`from_seed_epoch_consensus_inputs`, the same projection the forge handoff consumes), NOT a defaulted / empty distribution — so a valid live block 0 is ingested (`VerificationFailed` gone). The verdict stays BLUE (Steps 5 + 7 are BLUE authorities); G-P only sources the VIEW. **A closed RED view-provenance threading (a surface REDUCTION), NOT an extension point.** Backs **DC-CINPUT-04**. A change to the feed view = a strengthening of **DC-CINPUT-04** (`ci_check_feed_leader_threshold_view.sh` — the feed header-validation view is derived from the recovered consensus surface, never a defaulted / empty distribution). |
| Serve-listener lifetime decoupled from feed end *(NEW, N-F-G-K / DC-NODE-09)* | `ade_node::node_lifecycle` (RED — the serve sibling on the `On` arm) | RED lifetime fence; **NO new closed enum / variant** | The `--mode node` serve listener's lifetime is **independent of the live feed's end** — the serve sibling (`run_node_serve_task`, spawned OUTSIDE `run_relay_loop`) stays up serving past feed-end, so a follower mid-fetch keeps fetching (validated against the live C1 follower reaching `:3002`). RED, read-only over the self-accepted `ServedChainView`; admits nothing, advances no durable tip, makes no peer-acceptance claim. **A closed RED serve-lifetime fence (a surface REDUCTION), NOT an extension point.** Backs **DC-NODE-09**. A change to the serve lifetime = a strengthening of **DC-NODE-09** (`ci_check_node_serve_lifetime.sh` — the serve listener's lifetime is independent of the feed source's end); `run_relay_loop`'s containment is byte-unchanged (the serve task lives outside it). |
| `PrevHash` *(NEW, N-F-G-J S2)* | `ade_types::shelley::block` (BLUE) | 2 (`Genesis` = CBOR null / `Block(Hash32)` = hash32) | The closed Cardano header `prev_hash` wire grammar `$hash32 / null` (cardano-ledger `PrevHash = GenesisHash / BlockHash`), replacing the prior flat `prev_hash: Hash32` field (**456 → 457** canonical types). **NOT `#[non_exhaustive]`.** ONE **POSITION-BLIND** BLUE codec authority — `ade_codec::shelley::block::decode_prev_hash` decodes `null -> Genesis` / `hash32 -> Block` as a pure function of the CBOR token (NEVER `block_number`); the `ShelleyHeaderBody` `AdeEncode` writes `Genesis -> write_null` / `Block(h) -> write_bytes_canonical`. The `null` grammar is scoped to **header_body ONLY** — it MUST NOT leak into the chain-sync/block-fetch `Point`/`Tip` codec (`Point::Origin` stays `array(0)`). The position-AWARE coupling (`block_number 0 <=> Genesis`) lives in the SEPARATE single BLUE `ade_ledger::block_validity::header_position::check_header_position`, NEVER in `ade_codec`. **A closed BLUE sum (a surface REDUCTION over the wire grammar), NOT an extension point.** Backs **CN-WIRE-09**. New variant = a `decode_prev_hash` arm + an `AdeEncode` arm + a strengthening of **CN-WIRE-09** (`ci_check_prevhash_single_wire_authority.sh`); the codec stays POSITION-BLIND, the null grammar stays header_body-scoped, and the single wire + single position authority MUST NOT be duplicated. |
| `NodeSchedEvent` + `FeedReason` + `ForgeOutcome` *(NEW, N-F-G-J S1)* | `ade_node::live_log::sched_event` (GREEN) | `NodeSchedEvent` 5 (`FeedUnavailable{reason}` / `ForgeTickConsidered` / `ForgeTickSkipped{reason}` / `ForgeAttempted` / `ForgeResult{outcome}`); `FeedReason` 3 (`NoBlockAvailable` / `CleanEmpty` / `UnknownDisconnected`); `ForgeOutcome` 4 (`Succeeded` / `NotLeader` / `Failed` / `NoTipAvailable`) | The closed `--mode node` feed/forge **scheduling-event vocabulary**, byte-deterministically JSONL-encoded by `live_log::sched_writer`. **None is `#[non_exhaustive]`; there is deliberately NO catch-all / `Other` variant and no stringly-typed field** (`reason`/`outcome` are the closed enums; `as_str` is the stable discriminator). **EMIT-ONLY:** the GREEN planner never constructs or reads a `NodeSchedEvent`; the relay loop (`run_relay_loop_with_sched`) emits them around the planner call + the `LoopStep` arms, recording the scheduling decision WITHOUT altering it (`FeedReason::eligible` mirrors the planner's forge-eligibility predicate, but the event is observation only). Allow-list + negative tests. Lives in `ade_node` (NOT a BLUE `core_paths` entry), so **NOT canonical-counted**. **A closed diagnostic vocabulary (a surface REDUCTION), NOT an extension point.** Backs **CN-NODE-04**. New event/reason/outcome = an exhaustive-`match` arm in the encoder (compile error until wired + allow-listed) + a negative test + a strengthening of **CN-NODE-04** (`ci_check_node_sched_events_emit_only.sh` — the planner must never name the vocabulary). |
| `BlockValidityError::HeaderPositionInvalid` *(NEW additive variant, N-F-G-J S3)* | `ade_ledger::block_validity` (BLUE) | additive variant `{ block_number, expected, actual }`-shape on the closed `BlockValidityError` sum | The closed fail-closed signal raised by the single BLUE position authority `check_header_position` (`block_number 0 <=> Genesis`). Added **additively, no wildcard**; it folds into the **UNCHANGED** closed `BlockRejectClass::HeaderInvalid` (verdict.rs — **NO new reject class**). Surfaced by `decode_block` BEFORE the header authority. **A closed additive enum variant (a surface REDUCTION), NOT an extension point.** Backs **CN-WIRE-09 / DC-NODE-08**. New variant = a `check_header_position` arm + a `BlockRejectClass` mapping arm (folding into an existing coarse class) + a strengthening of **CN-WIRE-09**; non-secret position primitives only. |
| `BlockRejectClass` *(DELIBERATELY NOT EXTENDED, N-F-G-J S3)* | `ade_ledger::block_validity::verdict` (BLUE) | UNCHANGED — `HeaderPositionInvalid` maps to the existing `HeaderInvalid` coarse class | The closed coarse block-reject taxonomy. **S3 reused the existing `HeaderInvalid` for the position-rule failure — NO new variant added** (`verdict.rs`: `BlockValidityError::HeaderPositionInvalid { .. } => BlockRejectClass::HeaderInvalid`). A position-rule violation is surfaced through the existing coarse class, keeping the set additively stable. New variant = a strengthening of the verdict contract; closed, no wildcard. |
| `NodeForgeError::RecoveredTipMissingBlockNo` *(NEW additive variant, N-F-G-J S4)* | `ade_node::node_sync` (RED) | additive variant on the closed `NodeForgeError` sum (now 2: `MissingRecoveredConsensusInputs` / `RecoveredTipMissingBlockNo`) | The closed cold-start fail-closed edge: a `Some(tip)` without a block height fails closed (`forge_header_position` returns `Err(RecoveredTipMissingBlockNo)`), **never `.unwrap_or(1)`**. **A closed additive enum variant (a surface REDUCTION), NOT an extension point.** Backs **DC-NODE-08**. New variant = a `forge_header_position` arm + a strengthening of **DC-NODE-08 / CN-CINPUT-03**. |
| `RehearsalVenue` *(NEW, N-F-G-D S2)* | `ade_node::rehearsal_evidence` (GREEN-by-content) | 1 (`PrivateTestnetC1`) | The closed rehearsal-venue tag on the non-promotable `PrivateRehearsalManifest`. **A non-private venue is unrepresentable** — a rehearsal is NEVER preprod / preview, so a private-testnet manifest can never masquerade as bounty evidence by venue. Lives in `ade_node` (NOT a BLUE `core_paths` entry), so **NOT canonical-counted**. **A closed 1-variant enum (a surface REDUCTION), NOT an extension point.** New variant = a `to_str`/serialize arm + a strengthening of **CN-REHEARSAL-FIDELITY-01** clause 2 (`ci_check_rehearsal_manifest_schema.sh`); a `venue` of `private-testnet*` is the enforced literal, and a non-rehearsal venue must stay unrepresentable. |
| `PrivateRehearsalManifest` *(NEW, N-F-G-D S2)* | `ade_node::rehearsal_evidence` (GREEN-by-content) | constructor-fenced struct (`ba02: Ba02Manifest` + `venue: RehearsalVenue` + `peer_log_file` + `peer_log_file_sha256`; `REHEARSAL_MANIFEST_SCHEMA_VERSION = 1`) over the existing closed `Ba02Manifest` | The NON-PROMOTABLE private-testnet rehearsal envelope WRAPPING a correlate-produced `Ba02Manifest` (the SAME proof the bounty BA-02 path produces). **SOLE constructor** `from_correlate_outcome(&BA02Outcome, RehearsalEnvelope) -> Option<Self>` wraps `BA02Outcome::Ba02Manifest` and returns `None` on `NoEvidence` — so a rehearsal manifest is **ALWAYS correlate-produced**; there is **NO** path from raw operator input / `NoEvidence` to a manifest. `to_canonical_toml` ALWAYS emits `is_rehearsal = true` + `not_bounty_evidence = true` as **LITERALS** — the type **cannot represent a non-rehearsal**. Pure / deterministic; no I/O / clock / rand / float / `HashMap`. Lives in `ade_node`, so **NOT canonical-counted**. **A CLOSED constructor-fenced non-promotable envelope (a surface REDUCTION), NOT an extensible registry / plugin point.** Backs `CN-REHEARSAL-FIDELITY-01` clause 2. A new field / schema bump = a struct addition + a `REHEARSAL_MANIFEST_SCHEMA_VERSION` bump + a strengthening of **CN-REHEARSAL-FIDELITY-01** (`ci_check_rehearsal_manifest_schema.sh`, 12-field schema); **no raw-operator-input / `NoEvidence` ctor may be introduced, and the type MUST stay incapable of serializing a non-rehearsal.** |
| `SessionError::ReassemblyBufferOverflow` *(NEW, N-F-G-E)* | `ade_network::session::event` (GREEN-by-content) | additive variant `{ protocol, len, cap }` on the closed `SessionError` enum | The closed fail-closed signal for an incomplete per-mini-protocol reassembly tail over `MAX_REASSEMBLY_TAIL_BYTES`. Added **additively** — there is **NO wildcard**; the SOLE exhaustive consumer `ade_runtime::network::mux_pump::session_err_to_halt` maps it → `PeerHaltReason::ChainSyncDecodeError` (drop the peer). The cap fires **BEFORE** the BLUE `ade_codec` decode path — no silent truncation, no partial decode. `session/` is GREEN-by-content (NOT a BLUE `ade_network` submodule path), so the variant is **NOT canonical-counted**. **A closed additive enum variant (a surface REDUCTION / fail-closed bound), NOT an extension point.** New variant = a `SessionError` arm + a `session_err_to_halt` arm (no wildcard) + a strengthening of **DC-LIVEMEM-01** (`ci_check_live_feed_memory_bounds.sh`). |
| `MAX_REASSEMBLY_TAIL_BYTES` (closed memory bound) *(NEW, N-F-G-E)* | `ade_network::session::core` (GREEN-by-content, `core.rs:49`) | closed literal const `16 * 1024 * 1024` (16 MiB) | The per-mini-protocol reassembly-tail cap. After `drain_protocol_items` drains every COMPLETE item, `buf.len() > MAX_REASSEMBLY_TAIL_BYTES` ⇒ `SessionError::ReassemblyBufferOverflow` (fail closed, drop the peer). A **CLOSED LITERAL constant — a defensive implementation bound, NOT a Cardano semantic parameter**; **NO runtime / CLI / env / config override** (the no-escape-hatch surface reduction, `ci_check_live_feed_memory_bounds.sh` guard 3). A future hardening slice may **tighten** it (a strengthening of **DC-LIVEMEM-01**), but may NEVER make it a tunable / unbounded. **(N-AB)** its symmetric OUTBOUND counterpart is `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` (below; CN-SESS-05) — the outbound `handle_outbound` segmentation cap. |
| `MAX_WIRE_PUMP_LOOKAHEAD` (closed lookahead-depth bound) *(NEW, N-F-G-E)* | `ade_node::node_sync` (RED, `node_sync.rs:58`) | closed literal const `256` | The WirePump opportunistic-drain depth cap. `pump_lookahead` stops the `try_recv` drain at the cap (`node_sync.rs:126`), so the existing bounded `mpsc` (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`) **back-pressures** the pump. Content-blind; the verdict-decoupled `NodeBlockSource` (closed 2-variant `{WirePump, InMemory}`) + arrival order are **unchanged** (a depth cap on the existing opaque `VecDeque<Vec<u8>>`, NOT a new variant / source / verdict). A **CLOSED LITERAL constant — a defensive implementation bound**; **NO runtime / CLI / env / config override** (`ci_check_live_feed_memory_bounds.sh` guard 3). A future hardening slice may **tighten** it (a strengthening of **DC-LIVEMEM-01**), never make it a tunable / unbounded. |
| `MAX_OUTBOUND_PAYLOAD_BYTES` (closed outbound payload bound) *(NEW, N-AB / CN-SESS-05)* | `ade_network::session::core` (GREEN-by-content, `core.rs:60`) | closed literal const `16 * 1024 * 1024` (16 MiB) | The OUTBOUND mini-protocol payload ceiling, **SYMMETRIC with the receive-side `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` (DC-LIVEMEM-01)**. `session::core::handle_outbound` SEGMENTS a payload with `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` into ordered `<= MAX_PAYLOAD` frames (the outbound inverse of CN-SESS-04 inbound reassembly); a payload ABOVE the cap fails closed with the **existing** closed `SessionError::OutboundPayloadTooLarge { len }` (REUSED, **NOT a new variant**) **before** any segmentation. A **CLOSED LITERAL constant — a defensive implementation bound**; **NO runtime / CLI / env / config override, no unbounded mode, no partial-success send** (`ci_check_outbound_segmentation.sh`). `session/` is GREEN-by-content (NOT a BLUE `ade_network` submodule path), so the const is **NOT canonical-counted** (458 unchanged). A future hardening slice may **tighten** it (a strengthening of **CN-SESS-05**), but may NEVER make it a tunable / unbounded. **No new closed enum** — segmentation reuses the single `encode_inner_frame` → `mux::frame::encode_frame` authority + the closed `AcceptedMiniProtocol` registry. |
| `NodeBlockSource` *(N-F-C; readiness extended N-F-D; LIVE WirePump FILL N-F-G-C)* | `ade_node::node_sync` (RED) | 2 (`WirePump` / `InMemory`) | The **verdict-decoupled** ordered peer-block source: `next_block` yields ONLY `AdmissionPeerEvent::Block` bytes, SKIPS `TipUpdate`, ends on `Disconnected`. N-F-D added a content-blind readiness signal. **N-F-G-C added `from_wire_pump(rx)` — a LIVE FILL of the existing `WirePump` arm fed by `spawn_live_wire_pump_source` (reusing the closed admission dial + pump VERBATIM); NOT a new variant, NOT a new wire authority, NOT a plugin point.** NEVER carries a verdict. A closed single-method contract. New variant = a `next_block` arm + a strengthening of **DC-SYNC-01 / DC-SYNC-02**; a new source must REUSE the closed dial/pump (never reimplement), FILL an existing arm, advance no second tip, and carry no verdict. **N-F-G-J added NO `NodeBlockSource` variant — the genesis-successor cold-start is a new code PATH (the both-`None`-tip arm) through the EXISTING closed source, not a new variant.** |
| `ba02_pass` evidence I/O *(NEW, N-F-G-C S2)* | `ade_node::ba02_pass` (RED — `//! RED`) | 2 fns (`correlate_peer_log_file`, `write_ba02_manifest`); **NO new closed enum / registry** | The RED operator-pass BA-02 evidence file I/O over the pre-existing closed `Ba02Manifest` / `BA02Outcome` / `PeerAcceptEvent` / `NoEvidenceReason` vocabulary. `correlate_peer_log_file` reads the operator-captured peer-log file → the GREEN `correlate` (the SOLE `Ba02Manifest` ctor); `write_ba02_manifest` accepts **ONLY a `Ba02Manifest`** — so a written manifest is ALWAYS correlate-produced. **A CLOSED file-I/O wrapper (a surface REDUCTION over the existing closed evidence vocabulary), NOT a new closed enum / registry / plugin point.** A missing/unreadable file fails closed (`io::Error`). Backs the BA-02 leg of `RO-LIVE-06` / `CN-OPERATOR-EVIDENCE-01`. Gate: `ci_check_ba02_evidence_manifest_schema.sh` (the no-synthetic-manifest enforcer; vacuous-until-committed + 8-field schema + `peer_log_file_sha256` cross-check). Adding an evidence-I/O fn = a strengthening of RO-LIVE-06; **no new acceptance source, no path emitting a manifest from `NoEvidence` / raw operator input.** |
| `SelfAcceptedHandoff` *(N-F-G-B S1)* | `ade_runtime::producer::self_accepted_handoff` (GREEN) | constructor-fenced newtype (1 private field `accepted: AcceptedBlock`; SOLE ctor `from_self_accepted`; accessors `accepted()` / `into_accepted()`) | The typed carrier moving a BLUE self-accepted forged block from the forge path to the sibling serve task. Its **SOLE constructor** takes a BLUE `ade_ledger::producer::AcceptedBlock` (itself producible only by BLUE `self_accept` returning `Ok`); the field is private. There is **NO** constructor from a raw `Vec<u8>`, a `ForgedBlockArtifact` (`artifact.bytes` is never a token source — re-deriving would breach CN-FORGE-01; the carrier holds the ORIGINAL token), a `CoordinatorEvent`, a self-declared acceptance flag, or a peer verdict — so handing the serve task a non-self-accepted artifact is **type-unrepresentable**. A **CLOSED constructor-fenced carrier (a surface REDUCTION), NOT an extensible registry / plugin point.** **N-U S3 / DC-NODE-13 note (repoint `60deecf3`):** the `--mode node` serve usage of this carrier (the G-B `SelfAcceptedHandoff` → `push_atomic` accumulator handoff) is **SUPERSEDED** by serve-as-projection of the durable ChainDb — on the node spine a forged block reaches the served view by becoming DURABLE (`admit_forged_block_durably` → `pump_block`), not via this carrier. **The carrier is RETAINED for `--mode produce` (CN-PROD-04, `ServedChainSource::Snapshot`) — a SEPARATE serve authority** — so it still Backs `DC-NODE-06` (now via durable-provenance on `--mode node`; via the snapshot accumulator on `--mode produce`). A change to the carried type / a new accessor = a strengthening of **DC-NODE-06 / CN-PROD-04 / CN-FORGE-01** (`ci_check_served_chain_handoff_fence.sh`, REPOINTED N-U S3 to the durable-provenance serve); **no raw-bytes / artifact / event / flag / verdict constructor may be introduced**. |
| `SlotAlignmentError` *(N-F-G-A S3)* | `ade_runtime::clock` (GREEN-by-content) | 1 (`BeforeGenesisAnchor`) | The closed fail-closed boundary carried by `checked_millis_to_slot`. A before-anchor tick (`tick_millis < start_millis`) is an *error*, never a saturation to `start_slot`. A **surface REDUCTION (a closed fail-closed wall)**, NOT a plugin/extension point. New variant = a `checked_millis_to_slot` arm + a strengthening of **DC-EPOCH-03**. |
| `ProtocolParamsParseError` *(N-F-G-A S2a)* | `ade_runtime::consensus_inputs::protocol_params` (GREEN-by-content) | closed sum (incl. `JsonShape` / `InexactRational { field: &'static str }`) | The closed error set of the cardano-cli `query protocol-parameters` JSON parser. **No float path** — a rational literal that cannot be represented exactly by integer arithmetic fails closed (`InexactRational`); a bad shape ⇒ `JsonShape`. Carries only non-secret `&'static str` field tags. A **surface REDUCTION (a closed RED-parse → BLUE-`ProtocolParameters` pipeline)**, NOT an extension point. New variant = a `parse_protocol_parameters_json` arm + a strengthening of **CE-G-A-2a** (`ci_check_recovered_ledger_pparams_sourced.sh`); non-secret primitives only; **no float path may be introduced**. |
| `ForgeCurrentPParamsError` *(N-F-G-A S2a)* | `ade_runtime::consensus_inputs::canonical` (GREEN-by-content) | 3 (`PreimageAbsent` / `BindMismatch` / `Parse(ProtocolParamsParseError)`) | The closed error set of `require_forge_current_pparams`. `LiveConsensusInputsCanonical` carries `protocol_params_json: Option<String>` **OUTSIDE** the frozen 15-field canonical fingerprint (which commits to `protocol_params_hash`); the accessor requires the preimage present (`PreimageAbsent`), `blake2b_256`-binds it to the fingerprinted hash (`BindMismatch`), and parses exactly (`Parse`). A hash-bound accessor, NOT an extension point. New variant = a strengthening of **CE-G-A-2a**; **the preimage MUST stay OUTSIDE the 15-field canonical CBOR fingerprint** (no fingerprint-schema change). |
| `ForgeEpochAdmission` *(N-F-G-A S4)* | `ade_node::node_sync` (GREEN-by-fn inside RED `node_sync`) | 2 (`WithinSeedEpoch` / `OffEpoch { located, seed }`) | The closed off-epoch admission verdict carried by `forge_epoch_admission`, derived SOLELY from `(slot, era_schedule, seed_epoch)` via the BLUE `EraSchedule::locate`. An off-epoch / unlocatable slot is an *error* (`OffEpoch`), never a third variant. Called BEFORE `query_leader_schedule` inside `forge_one_from_recovered`. A **closed classifier vocabulary**, NOT an extension point. New variant = a `forge_epoch_admission` arm + a strengthening of **DC-EPOCH-03** (`ci_check_node_forge_single_epoch_fail_closed.sh` — must use `EraSchedule::locate`, no fabricated epoch, no nonce promotion). |
| `CoordinatorEvent` *(DELIBERATELY NOT EXTENDED, N-F-G-A S4)* | `ade_runtime::producer::coordinator` (GREEN) | 9 (`SlotTick` / `ForgeSucceeded` / `ForgeNotLeader` / `ForgeFailed` / `PeerConnected` / `PeerDisconnected` / `LedgerSnapshotUpdated` / `BroadcastDrained` / `Shutdown`) | The closed coordinator event set. **S4 reused the existing `ForgeNotLeader` for the off-epoch outcome — NO new variant added.** N-F-G-B surfaced the self-accepted token via a **sibling return component**, NOT a new event variant. N-F-G-C added **NO** `CoordinatorEvent` variant; N-F-G-D added **NO** `CoordinatorEvent` variant. An off-epoch forge is surfaced as a "not leader" outcome through the closed vocabulary, keeping the set additively stable. New variant = a strengthening of **DC-PROD-01**; closed JSONL vocab, no free-form reason strings, no key material. |
| `ForgeIntent` *(N-F-F)* | `ade_node::forge_intent` (GREEN) | 2 (`On(ForgePaths)` / `Off`) | The closed tri-state forge-intent classification the `--mode node` arm keys its forge-on flip off. `classify_forge_intent` is the SOLE entry; **NO third "partial" variant** — a partial key set is `Err(ForgeIntentError::PartialKeySet)`. Pure/total over all 2⁵ flag-PRESENCE combinations (never observes contents). A **CE-not-law additively-closed classifier** (like `WalEntry` / `LoopStep`). New variant = a `classify_forge_intent` arm (bound by name, no wildcard) + a `node_lifecycle` dispatch arm + a strengthening of **CN-NODE-03** (`ci_check_forge_intent_closed.sh`). |
| `ForgeIntentError` *(N-F-F)* | `ade_node::forge_intent` (GREEN) | 1 (`PartialKeySet { present, missing }`) | The closed forge-intent classify error — carries ONLY static CLI flag-name strings (`&'static str`), never a supplied path string, never key material. New variant = a strengthening of **CN-NODE-03**; static flag-name strings only (`ci_check_forge_intent_closed.sh`). |
| `OperatorForgeError` *(N-F-F)* | `ade_node::operator_forge` (RED) | 6 (`ColdKeyLoad` / `VrfKeyLoad` / `KesKeyLoad` / `OpcertParse` / `ShellInit` / `GenesisParse`) | The closed operator-material ingress error sum — one variant per reused-loader / structural-validator step. Carries no path/key bytes (`OpcertParse`/`GenesisParse` hold `&'static str` detail only). _(N-F-G-A: details now come from the REAL `parse_opcert_envelope` / `parse_shelley_genesis`.)_ New variant = a strengthening of **CN-NODE-03 / OP-OPS-04**; non-secret primitives only (`ci_check_operator_forge_no_secret_leak.sh`). |
| `OperatorForgeMaterial` *(N-F-F)* | `ade_node::operator_forge` (RED) | closed struct (`shell: ProducerShell` / `genesis: GenesisAnchor` / `pool_id: Hash28` / `pparams` / `protocol_version` / `anchor_millis` / `start_slot` / `slot_length_ms`) | The operator-material forge inputs. **Deliberately NOT `Debug`/`Serialize`** (holds the custody `ProducerShell`); no byte accessor / serialization / logging. `pool_id` derived in ONE named place (`blake2b_224(cold_vk)`). A **CE-not-law additively-closed struct**. A new field = a struct addition behind the closed ingress contract + a strengthening of **CN-NODE-03** (`ci_check_operator_forge_no_secret_leak.sh`). |
| `LoopStep` *(EXTENDED 3→4, N-F-E)* | `ade_node::run_loop_planner` (GREEN) | 4 (`SyncOnce` / `ForgeTick` / `Idle` / `HaltCleanly`) | The closed live-run iteration vocabulary the GREEN planner emits. **N-F-E added `ForgeTick`** (3→4). It **cannot express an authority decision**. A **CE-not-law additively-evolvable closed planner enum** (like `WalEntry`). New variant = a `plan_loop_step` arm + a fenced RED `run_relay_loop` branch + a strengthening of **CN-NODE-02 / DC-NODE-05** (`ci_check_loop_planner_closed.sh` + `ci_check_node_run_loop_containment.sh`). |
| `ForgeSlotStatus` *(N-F-E)* | `ade_node::run_loop_planner` (GREEN) | 2 (`Due` / `NotDue`) | The **content-blind** forge-slot planner input. The planner learns only whether a slot is *due*, NEVER who is a leader (eligibility is BLUE inside `forge_one_from_recovered`). Derived by the pure `forge_slot_status` monotonic guard (the only `SlotNo`-observing fn in the module). New variant = a `plan_loop_step` arm + a strengthening of **DC-NODE-05** (`ci_check_loop_planner_closed.sh`). |
| `ForgeActivation` *(N-F-E; real operator material N-F-F; current pparams N-F-G-A; opt-in `handoff_tx` N-F-G-B)* | `ade_node::node_lifecycle` (RED) | closed opt-in struct (`clock` / `coordinator_state` / `recovered` / `shell` / `pool_id` / `pparams` / `protocol_version` / `anchor_millis` / `start_slot` / `slot_length_ms` / `last_slot_alignment_fail` / `handoff_tx: Option<mpsc::UnboundedSender<SelfAcceptedHandoff>>` / private `last_forged_slot`+`pending_slot` / `hermetic_forge_outcomes`) | The **opt-in forge-activation bundle** threaded into `run_relay_loop` as `forge: Option<&mut ForgeActivation>`. `Some` activates exactly one fenced `forge_one_from_recovered` per `ForgeTick`, advancing no durable tip and serving/admitting/gossiping nothing in the loop body; `None` reproduces N-F-D relay. N-F-G-B: the opt-in `handoff_tx` carries the surfaced `SelfAcceptedHandoff` to the sibling serve task (set via `with_handoff_sender`). **A closed activation surface, NOT an extension point.** A new field = a struct addition behind the closed activation contract + a strengthening of **DC-NODE-05 / DC-NODE-06**. |
| `Mode` (run-mode set) *(N-F-C)* | `ade_node::cli` (RED) | 5 (`WireOnly` / `Admission` / `KeyGenKes` / `Produce` / `Node`) | The CLOSED `--mode` taxonomy. **NOT `#[non_exhaustive]`**; `Mode::parse` + `main.rs` dispatch are total with **NO wildcard arm**. New variant = a `Mode::parse` arm + an explicit wildcard-free `main.rs` arm + a strengthening of **CN-NODE-MODE-01** (`ci_check_node_mode_closure.sh`). _(N-F-F/N-F-G-A/N-F-G-C added NO `Mode` variant — the operator-key + `--peer` flags are OPTIONAL ingress on the existing `--mode node` arm; N-F-G-D added NO `Mode` variant and NO new argv flag — the `cli.rs` flag set is pinned to a 28-flag closed allow-list by `ci_check_node_path_fidelity.sh`, and the C1 dry-run is a HARNESS, not a runtime mode.)_ _(N-F-G-J added NO `Mode` variant and NO new argv flag — the cold-start forge fires through the existing `--mode node` arm; the CLI flag allow-list is unchanged and stays fenced by `ci_check_node_path_fidelity.sh`.)_ |
| `PeerAcceptEvent` *(N-F-C; consumed via `ba02_pass` N-F-G-C)* | `ade_node::ba02_evidence` (GREEN) | 2 (`PeerServedBlock` / `PeerChainTip`) | The CLOSED **allow-list** of peer-acceptance signals; `parse_peer_accept_events` recognizes ONLY these two discriminators. UNCHANGED by N-F-G-C (only consumed by the new RED `ba02_pass` I/O). New variant = a parser allow-list arm + a strengthening of **RO-LIVE-06**. |
| `PeerAcceptSource` *(N-F-C)* | `ade_node::ba02_evidence` (GREEN) | 3 (`ServedBlock` / `ChainTip` / `ServedBlockAndChainTip`) | The closed typed provenance of the accepting signal. New variant = a `correlate` source arm + a strengthening of RO-LIVE-06. |
| `NoEvidenceReason` *(N-F-C)* | `ade_node::ba02_evidence` (GREEN) | 4 (`NoPeerAccept` / `HashMismatch` / `ChainPointMismatch` / `ConflictingPeerSignals`) | The closed reason sum for `BA02Outcome::NoEvidence` — NoEvidence is the DEFAULT. New variant = a `correlate` classify arm + a strengthening of RO-LIVE-06. |
| `BA02Outcome` *(N-F-C; consumed via `ba02_pass` N-F-G-C)* | `ade_node::ba02_evidence` (GREEN) | 2 (`Ba02Manifest(Ba02Manifest)` / `NoEvidence { reason }`) | The closed correlation outcome. `correlate` is the **SOLE** `Ba02Manifest` constructor; no self-evidence acceptance source; no committed synthetic manifest (**RO-LIVE-06**). UNCHANGED by N-F-G-C — `ba02_pass::write_ba02_manifest` accepts ONLY the `Ba02Manifest` arm. |
| `Ba02Manifest` schema *(N-F-C; schema gate N-F-G-C)* | `ade_node::ba02_evidence` (GREEN) | versioned struct — `BA02_MANIFEST_SCHEMA_VERSION = 1` | A **version-GATED** canonical evidence manifest (see §4); SOLE constructor `correlate`'s exact-match arm (RO-LIVE-06). N-F-G-C added the committed-manifest schema gate `ci_check_ba02_evidence_manifest_schema.sh` (8 fields + `schema_version == 1` + `peer_log_file_sha256` cross-check; vacuous-until-committed). |
| `NodeLifecycleError` *(N-F-C; +RelaySync N-F-D; +ForgeKeyIngress N-F-F; +MissingFlag live-feed N-F-G-C)* | `ade_node::node_lifecycle` (RED) | closed sum (incl. `RelaySync`, `ForgeKeyIngress(String)`, `MissingFlag(&'static str)`) | The closed fail-closed lifecycle-owner error set (Mithril-only, no genesis/bundle/cold/graft fallback). N-F-F added `ForgeKeyIngress(String)` (→ exit 44); N-F-G-C uses `MissingFlag("--network-magic")` on the live-feed arm. New variant = a strengthening of **CN-NODE-01 / CN-NODE-02 / CN-NODE-03**. |
| `NodeStart` *(N-F-C)* | `ade_node::node_lifecycle` (RED) | 2 (`FirstRun` / `WarmStart`) | The closed start classification — a PURE function of on-disk state. No third "ambiguous" mode. New variant = a strengthening of CN-NODE-01. |
| `NodeSyncError` *(N-F-C)* | `ade_node::node_sync` (RED) | 2 (`Pump(String)` / `Capture(String)`) | The closed sync-driver fail-closed halt set. New variant = a strengthening of **DC-SYNC-01 / DC-SYNC-02**. |
| `NodeForgeError` *(N-F-C; exercised by the N-F-E forge tick + N-F-F On arm + N-F-G-A tick)* | `ade_node::node_sync` (RED) | 1 (`MissingRecoveredConsensusInputs`) | The closed forge-handoff fail-closed set: a forge over a base that carries NO recovered seed-epoch record is unrepresentable. New variant = a strengthening of **CN-CINPUT-03 / DC-CINPUT-02b / DC-NODE-05**. |
| `SeedEpochConsensusInputs` *(N-F-A; +`epoch_nonce` field, schema `1 → 2` N-F-G-N)* | `ade_ledger::seed_consensus_inputs` (BLUE) | closed canonical record; **version-gated** behind `SEED_CINPUT_SCHEMA_VERSION = 2` (now a 7-field outer record) | The recovered seed-epoch consensus-input record with a **SOLE** encoder/decoder pair. **N-F-G-N added the additive BLUE field `epoch_nonce: Nonce`** (the recovered Praos eta0 for `epoch_no`) and **bumped `SEED_CINPUT_SCHEMA_VERSION 1 → 2`** — the field is additive ONLY behind the bump; a **v1 sidecar that omitted `epoch_nonce` decodes as `UnknownVersion`, NEVER a default-to-zero eta0** (verified: the decoder rejects `version != 2` as `UnknownVersion`; a v1-shape test pins the fail). `decode_*` rejects any version != the constant fail-closed, and rejects a structurally-valid-but-non-canonical buffer. No `Default`, no `#[non_exhaustive]`, `BTreeMap`. **A closed canonical record evolving behind a VERSION GATE (a version-gated field add, NOT a new TYPE — 458 unchanged), NOT an extension point.** Backs **CN-CINPUT-01** (strengthened by **T-REC-04 / DC-CINPUT-03**). New field / version = a `decode_*` arm + a `SEED_CINPUT_SCHEMA_VERSION` bump + a strengthening of **CN-CINPUT-01**; the recovered eta0 MUST be carried verbatim (never default-to-zero) and the v1→v2 gate MUST fail closed (`ci_check_warmstart_eta0_overlay.sh`). No second codec. |
| `SeedConsensusInputsError` *(N-F-A)* | `ade_ledger::seed_consensus_inputs` (BLUE) | 6 (`MalformedCbor` / `UnknownVersion` / `Structural` / `NonCanonicalMapOrder` / `DuplicatePoolKey` / `TrailingBytes`) | The closed `decode_*` failure set. New variant = a strengthening of **CN-CINPUT-01**; non-secret primitives only; MUST fail closed. |
| `SeedConsensusMergeError` *(N-F-A; carries recovered eta0 N-F-G-N)* | `ade_runtime::seed_consensus_merge` (GREEN) | 2 (`PoolMissingVrfKeyhash` / `PoolMissingStake`) | A pool present in exactly one source map fails closed here, **never a zero-hash fill**. **N-F-G-N:** the merge now also carries the recovered Praos `epoch_nonce` (eta0) verbatim into the BLUE record's new `epoch_nonce` field — a pure field-pass, **never default-to-zero** (DC-CINPUT-03 forbids a zero-eta0 fallback; the BLUE codec's v1→v2 gate backs it). New variant = a strengthening of the merge contract (CN-CINPUT-02). No catch-all. |
| `SeedEpochConsensusSource` *(N-F-A; CONSUME-side wired N-F-C)* | `ade_runtime::bootstrap` (RED) | 2 (`NotRequired` / `RequiredFromRecoveredProvenance(RecoveredBootstrapProvenance)`) | The input-mode discriminant for warm-start. The `--mode node` WarmStart arm passes `RequiredFromRecoveredProvenance`; its construction is contained to {lifecycle owner, `bootstrap.rs`}. New variant = a strengthening of DC-CINPUT-01. |
| `BootstrapError` (N-F-A new variants) | `ade_runtime::bootstrap` (RED) | +5 (`SeedConsensusProvenanceMissing` / `SeedConsensusSidecarMissing` / `SeedConsensusHashMismatch` / `SeedConsensusBindingMismatch` / `SeedConsensusSidecarDecode`) | The fail-closed warm-start-restore failure set; MUST NOT fall back to the forge-time bundle. New variant = a strengthening of **DC-CINPUT-01**; non-secret primitives only. |
| `MithrilBootstrapError` *(N-Z; +N-F-A SeedConsensus* variants)* | `ade_runtime::mithril_bootstrap` (RED) | 3 base + N-F-A `SeedConsensus*` | The closed RED-composition error sum — one variant per composed step. No catch-all/`String`; the binding step is the SOLE semantic decision (BLUE). |
| `MithrilSeedPointInputs` *(N-Z)* | `ade_runtime::mithril_bootstrap` (RED) | closed struct | The **operator-provided, structurally-independent** seed-point origin (DC-MITHRIL-02). A new attested field = a struct addition + a strengthening of DC-MITHRIL-02. |
| `MithrilBootstrapOutput` *(N-Z)* | `ade_runtime::mithril_bootstrap` (RED) | closed struct (`ledger` / `chain_dep` / `tip` / `anchor`) | A new field = a struct addition behind the composition contract. |
| `SeedProvenance` *(N-Y; UNCHANGED through N-F-G-C)* | `ade_ledger::bootstrap_anchor::anchor` (BLUE) | 2 (`CardanoCliJson` / `Mithril { … }`) | **Version-gated** behind `ANCHOR_SCHEMA_VERSION = 2`. Closed — no open/wildcard variant. New variant = a `decode_bootstrap_anchor` arm + an `ANCHOR_SCHEMA_VERSION` bump + a strengthening of **CN-ANCHOR-01 / DC-ANCHOR-01**. |
| `MithrilImportError` *(N-Y)* | `ade_ledger::bootstrap_anchor::binding` (BLUE) | 5 | The closed `verify_mithril_binding` failure set. New variant = a strengthening of **CN-MITHRIL-01 / DC-MITHRIL-01**; MUST fail closed. |
| `MithrilManifestReport` *(N-Y)* | `ade_ledger::bootstrap_anchor::binding` (BLUE) | closed struct | A new attested field = a struct addition + a strengthening of the binding predicate's cross-check. |
| `GenesisSourceError` *(N-Y)* | `ade_ledger::genesis_source` (BLUE) | 1 load-bearing (`NonConwayEra`) | `genesis_initial_state` is Conway-only. New variant = a strengthening of **DC-GENESIS-SRC-01**. |
| `SyncEffect` *(N-Y)* | `ade_runtime::forward_sync::reducer` (GREEN-by-content) | 4 (`StoreBlockBytes` / `AppendWal` / `CommitCheckpoint` / `AdvanceTip`) | The closed forward-sync effect plan. `AdvanceTip` is unreachable before `StoreBlockBytes` + `AppendWal`. New variant = a reducer arm + a pump apply-step + a strengthening of **DC-SYNC-01**. |
| `MithrilManifestError` *(N-Y)* | `ade_runtime::mithril_import::importer` (RED) | closed sum | The closed manifest-JSON parse failure set. No semantic decision. |
| `PumpError` *(N-Y)* | `ade_runtime::forward_sync::pump` (RED) | closed sum (incl. `TipBeforeDurable`) | New variant = a strengthening of **DC-SYNC-01**. |
| `NodeRecoveryError` *(N-Y)* | `ade_runtime::recovery::restart` (RED) | closed sum (incl. `WalTailFingerprintMismatch`) | A WAL-tail fingerprint divergence fails fast. New variant = a strengthening of the recovery contract / **DC-WAL-***. |
| `BlockVerdict` (observable surface) *(N-Y)* | `ade_testkit::harness::sync_diff` (GREEN) | 2 (`Admitted` / `Rejected`) | Compared on observable surfaces only (DC-COMPAT-01). New variant = a strengthening of **DC-COMPAT-01 / RO-SYNC-EVIDENCE-01**. |
| `RegressionFixtureViolation` *(N-Y)* | `ade_testkit::harness::sync_diff` (GREEN) | closed sum | New variant = a strengthening of **RO-SYNC-EVIDENCE-01**. |
| `TagEnvelopeError` *(N-X)* | `ade_codec::cbor::tag24` (BLUE) | 4 (`NotTag24` / `NotByteString` / `Truncated` / `TrailingBytes`) | New variant = a strengthening of **CN-WIRE-08**; non-secret offset/length primitives only. |
| `ExpectedVrfInput` *(N-W)* | `ade_core::consensus::vrf_cert` (BLUE) | 2 (`Praos([u8;32])` / `Tpraos([u8;41])`) | The 2-variant enum IS the protocol-family tag. New variant = a `leader_vrf_input` arm + a strengthening of **CN-FORGE-04**. |
| `LeaderCheckVerdict` *(N-R-A)* | `ade_core::consensus::leader_check` (BLUE) | 2 (`Eligible` / `NotEligible`) | New variant = a strengthening of **CN-FORGE-02**; `NotEligible` carries only a bounded fingerprint. |
| `ForgeFailureReason` *(extended N-W)* | `ade_runtime::producer::producer_log` (GREEN) | closed sum incl. `UnsupportedProducerEra` | New variant = a strengthening of **CN-FORGE-04 / DC-PROD-01**. |
| `OutboundCommand` *(N-S-B)* | `ade_runtime::network::outbound_command` (RED) | typed `ChainSyncServerMsg` / `BlockFetchServerMsg` | New variant = a new typed mini-protocol reply. **No `Vec<u8>` byte tunnel** (CN-OUTBOUND-RELAY-01). |
| `DispatchError` *(N-S-B)* | `ade_node::produce_mode` + `ade_runtime::network::n2n_server` (RED) | closed sum | No `String`/catch-all variant (CN-PEER-OUTBOUND-MAP-01). |
| `ChainEvolutionError` *(N-T)* | `ade_runtime::producer::chain_evolution` (GREEN) | closed sum | New variant = a strengthening of **DC-PROD-03**. |
| `BroadcastPushError` *(N-T)* | `ade_node::produce_mode` (RED) | closed sum | New variant = a strengthening of **CN-PROD-04**. |
| `ProducerLogEvent` *(N-Q)* | `ade_runtime::producer::producer_log` (GREEN) | closed JSONL vocab | New variant = a strengthening of **DC-PROD-01**. No free-form reason strings, no key material. |
| `GenesisParseError` *(N-R-C; reused on the node path N-F-G-A)* | `ade_runtime::producer::genesis_parser` (RED) | closed sum | New variant = a strengthening of **CN-GENESIS-01**. The N-F-G-A `operator_forge` ingress reuses `parse_shelley_genesis` (this error type) on the node path. |
| `OpCertParseError` *(N-R-C; reused on the node path N-F-G-A)* | `ade_runtime::producer::opcert_envelope` (RED) | closed sum | New variant = a strengthening of **CN-OPCERT-01**. The N-F-G-A `operator_forge` ingress reuses `parse_opcert_envelope` (this error type) on the node path. |
| `UnsignedHeaderPreImageError` *(N-S-A)* | `ade_ledger::block_validity::unsigned_header_pre_image` (BLUE) | closed sum | New variant = a strengthening of **DC-KES-HEADER-01**. |
| `SigningError` / branded `KesSignature` + `UnsignedHeaderPreImage` *(KES signing surface; REUSED — NOT extended — by N-AC)* | `ade_runtime::producer::{signing, producer_shell}` (RED) + `ade_ledger::block_validity::unsigned_header_pre_image` (BLUE) | closed sum + branded newtypes | **N-AC (DC-CRYPTO-10): adds NO new closed enum / variant and NO version-gated contract.** The new RED `kes_sign_header_advancing` evolves-then-signs by REUSING the EXISTING closed `SigningError` variants `EvolutionBackwards` (backwards / forward-secrecy) + `EvolutionExhausted` (beyond `SUM6_MAX_PERIOD = 63`) and the EXISTING branded `KesSignature` / `UnsignedHeaderPreImage` types — the closed-surface set is unchanged. New variant on `SigningError` = a strengthening of **DC-CRYPTO-04..09 / CN-KES-HEADER-01**; non-secret primitives only (no key bytes in errors). |
| `AcceptedMiniProtocol` *(N-L; outbound segmentation REUSES it, N-AB)* | `ade_network::session` (GREEN) | closed registry | New mini-protocol = a registry entry + a `match` arm with **no wildcard accept**. **N-AB (CN-SESS-05):** the outbound `session::core::handle_outbound` segmentation carries the SAME mini-protocol id + mode across every segment of a message — it REUSES this closed registry (no new variant, no new mini-protocol) and the single `encode_inner_frame` per-frame authority. |
| `KesError` / `KesParseError` *(N-P)* | `ade_crypto::kes_sum::errors` (BLUE) | 5 / 6 variants | New variant = a strengthening of **DC-CRYPTO-08/09**; non-secret primitives only. |
| Operator-evidence manifest TOML schema *(N-S-C)* | `ci_check_operator_evidence_manifest_schema.sh` + `docs/clusters/completed/PHASE4-N-S-C/cluster.md` | closed key set | Any committed `CE-N-S-LIVE_*.toml` MUST conform (CN-OPERATOR-EVIDENCE-01). |
| BA-02 operator-pass manifest TOML schema *(NEW, N-F-G-C)* | `ci_check_ba02_evidence_manifest_schema.sh` + `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml` | closed 8-key set (`schema_version` / `block_hash` / `slot` / `peer_log_file` / `peer_log_file_sha256` / `peer_log_capture_command` / `peer_log_filter` / `accept_event_kind`) | Any committed `CE-G-C-LIVE_*.toml` MUST conform (`schema_version == 1`) AND its `peer_log_file_sha256` MUST match the actual SHA-256 of the committed peer-log fixture it binds (CN-OPERATOR-EVIDENCE-01 / RO-LIVE-06). **Vacuously satisfied when none is committed** (the typical state — the live operator pass is `blocked_until_operator_stake_available`). The no-synthetic-manifest enforcer. |
| Private-testnet rehearsal manifest TOML schema *(NEW, N-F-G-D)* | `ci_check_rehearsal_manifest_schema.sh` + `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml` | closed 12-key set (`schema_version` / `venue` / `is_rehearsal` / `not_bounty_evidence` / `peer_log_file` / `peer_log_file_sha256` / `forged_block_hash_hex` / `slot` / `network_magic` / `peer_accept_source` / `peer` / `matched_block_hash_hex`) | Any committed `phase4-n-f-g-d-private-rehearsal-*.toml` MUST conform (`schema_version == 1`, `is_rehearsal = true`, `not_bounty_evidence = true`, `venue` of `private-testnet*`) AND its `peer_log_file_sha256` MUST match the actual SHA-256 of the committed peer-log fixture (CN-REHEARSAL-FIDELITY-01 clause 2). **THREE non-promotability barriers:** the distinct `docs/evidence/` rehearsal home; the explicit rehearsal markers; and a fail-closed cross-check that NO rehearsal marker appears in any `.toml` under EITHER bounty home (active `docs/clusters/PHASE4-N-F-G-C/` AND archived `docs/clusters/completed/PHASE4-N-F-G-C/`). **Vacuously satisfied when none is committed** (the typical state — the C1 dry-run is `blocked_until_operator_c1_net_executed`). The no-synthetic-rehearsal-manifest + no-bounty-home-leak enforcer. A rehearsal manifest is NOT bounty evidence and flips NO RO-LIVE rule. |
| Sync-evidence manifest schema *(N-Y)* | `ci_check_sync_evidence_manifest_schema.sh` + `corpus/sync/regressions/` | closed key set | Mirrors the operator-evidence pattern; vacuously satisfied until a manifest is committed (RO-SYNC-EVIDENCE-01, **partial**). |
| `CardanoEra` + Conway cert / governance / withdrawal enums | `ade_types::{era, conway::*}` + `ade_codec::conway::*` | closed | New era / cert / gov-action = a versioned gate (DC-LEDGER-08/09/10/11). `is_praos()` classifies exactly {Babbage, Conway}. |
| Consensus message + verdict enums | `ade_core::consensus`, `ade_ledger::block_validity` / `tx_validity` | closed | `ci_check_consensus_closed_enums.sh` — `match` with no wildcard. |
| JSONL event vocabularies (admission / wire-only / live-log) | `ade_node::{admission_log, live_log}`, `ade_runtime::admission` | closed | New event = a strengthening of the owning DC rule; allow-list + negative tests. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|----------------|
| Served read-side trait seams `ServedHeaderLookup` / `ServedRangeLookup` *(impls extended N-U; durable impl BOUNDED N-AA)* | `ade_network::chain_sync::server::ServedHeaderLookup` + `ade_network::block_fetch::server::ServedRangeLookup` (BLUE reducers) — impls in `ade_runtime` | **THE closed read-side extension surface for the serve reducers.** A new served read-side attaches by IMPLEMENTING these two BLUE trait seams; the BLUE serve reducers + the single `dispatch_server_frame_event_to_outbound` are reused UNCHANGED. **Two impls exist at HEAD:** the produce-mode snapshot impl `ade_runtime::producer::served_chain_lookups::ServedChainLookups` (over `ServedChainSnapshot`) and the NEW N-U durable-chain impl `ade_runtime::network::served_chain_projection::ChainDbServedSource` (over the durable ChainDb — DC-NODE-13). A new impl MUST serve already-durable / already-self-accepted bytes VERBATIM (no re-encode — DC-CONS-17), reuse the single `block_header_bytes` (DC-CONS-18) + `decode_block` (NO parallel splitter, NO `AcceptedBlock` reconstruction), and be selected through a closed `ServedChainSource` variant (NOT a `Box<dyn _>` registry / negotiated surface) — provenance MUST stay structural (durable ChainDb ⇐ `pump_block` / self-accept). **N-AA (DC-SERVEMEM-01): the `--mode node` `ChainDbServedSource` BlockFetch read is now BOUNDED** — it reads via the bounded hash-free ChainDb primitives `range_bytes_capped` + `last_block_bytes` (NOT `iter_from_slot` / `chaindb.tip()`), caps each peer request at the fixed non-configurable `MAX_SERVE_RANGE_BLOCKS = 256`, and fails closed (`ServeRangeOutcome`) on an oversized / inverted / undecodable range; a NEW serve impl MUST likewise read via bounded primitives and cap per request (no unbounded materialization, no `SLOT_BY_HASH` scan). `ci_check_served_chain_projection.sh` + `ci_check_serve_range_bounded.sh`. |
| Ade-native WAL (append-only) | `ade_runtime::wal` (GREEN-by-content) + `ade_ledger::wal::event` (BLUE encoder/decoder) | Append-only; committed entries are never mutated (`ci_check_wal_append_only.sh`). **`WalEntry` is a deliberately CE-not-law surface** — additively evolvable behind the WAL schema (append-only wire tags; `AdmitBlock` = 0, `SeedEpochConsensusInputsImported` = 3, tags 1/2 reserved). An acceptance criterion, NOT a frozen registry-law enum. |
| Seed-epoch sidecar store (anchor-fp-keyed) *(N-F-A; consumed N-F-C)* | `ade_runtime::chaindb::SnapshotStore::{put,get,list}_seed_epoch_consensus_*` | A new entry is `put` only on the verified-bootstrap composition path, keyed by `anchor_fp` in a namespace disjoint from the slot-keyed snapshot space; idempotent on identical bytes (redb `seed_cinputs_by_anchor_fp` table, `SCHEMA_VERSION = 3`). N-F-C consumes it via `list_seed_epoch_consensus_anchor_fps` + `get_seed_epoch_consensus_inputs` on the WarmStart arm. The forge-time path may NOT `put` here (CN-CINPUT-02). |
| `PerPeerOutbound` map *(N-S-B)* | `ade_runtime::network::outbound_command` — `Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>` | Grows at runtime; **`BTreeMap`, not `HashMap`** — deterministic iteration; no cross-peer byte leakage (CN-PEER-OUTBOUND-MAP-01, DC-OUTBOUND-FIFO-01). |
| `OpCertCounterMap` | `ade_core::consensus::praos_state` (BLUE) | Grows as op-certs are observed; deterministic ordering. |
| `ServedChainSnapshot` (served blocks) | `ade_ledger::producer::served_chain` (BLUE) | Grows via `served_chain_admit` only; `push_atomic` is the sole publisher. (The N-F-E/N-F-F/N-F-G-A relay-loop forge tick does NOT publish here; N-F-G-B publishes via the sibling task's single `push_atomic` fed by `into_accepted()`.) |
| `MempoolState` (admitted txs) | `ade_ledger::mempool` (BLUE) | Grows via `mempool_ingress` → `admit` only; sorted/deduplicated. |
| Seed entries (imported UTxO) | `ade_runtime::seed_import` (GREEN-by-content) | Grows at import time from a cardano-cli UTxO dump; canonical decoders only. |
| Persisted ChainDb (synced blocks) *(N-Y; first production driver N-F-C; driven by the N-F-D loop; LIVE feed N-F-G-C; own-forged blocks N-U; serve-read source N-U)* | `ade_runtime::chaindb` via `forward_sync::pump` (write) + `ChainDbServedSource` (read, N-U) | Grows via the forward-sync pump applying the GREEN reducer's `SyncEffect` plan in durable order; the tip advances only after `StoreBlockBytes` + `AppendWal` ack (DC-SYNC-01). N-F-C's `node_sync::run_node_sync` is the first production driver; the N-F-D relay loop drives it each `SyncOnce` iteration; N-F-G-C feeds it a LIVE `--peer` source. **N-U: the SAME `pump_block` chokepoint now also durably admits the forge's OWN self-accepted block** (via `node_sync::admit_forged_block_durably`; extend-only, durable-before-tip — DC-NODE-12 / DC-CONS-23 / DC-WAL-04), so the producer own-tip advances. The sole production WRITERS stay `pump_block` + the validated `bootstrap_initial_state` (CN-CONS-07). **N-U also makes the durable ChainDb the `--mode node` SERVE-READ source** — `ChainDbServedSource` projects it READ-ONLY through the serve seams (DC-NODE-13); a forged-block durable tip recovers byte-identically on restart (T-REC-05) and serving survives restart. **N-AA bounds that serve-read** — the peer-driven BlockFetch range is read via the bounded hash-free `range_bytes_capped` + `last_block_bytes` and capped at the fixed `MAX_SERVE_RANGE_BLOCKS = 256`, failing closed on an oversized / inverted / undecodable range (DC-SERVEMEM-01); the WRITE side (`pump_block` + `bootstrap_initial_state`) is unchanged. |
| Sync regression fixtures *(N-Y)* | `corpus/sync/regressions/` | Each discovered Haskell observable-surface mismatch is committed as a named regression fixture (RO-SYNC-EVIDENCE-01). |
| Sum_n KES family | `ade_crypto::kes_sum` (BLUE) | A new `Sum_n` attaches as an internal type-alias step; the `KesAlgorithm` trait surface does not change. |
| Per-protocol tag-24 compositions *(N-X)* | `ade_network::codec::{block_fetch, chain_sync}` | A new CBOR-in-CBOR composition attaches as a `compose_*` / `decompose_*` pair delegating to the single `ade_codec::{wrap_tag24, unwrap_tag24}` authority (CN-WIRE-08). |
| Bootstrap-source production compositions *(N-Z; +N-F-A sidecar tail)* | `ade_runtime::{genesis_bootstrap, mithril_bootstrap}` | A new bootstrap-source production entry attaches as a **composition-only RED twin** of `bootstrap_from_{conway_genesis, mithril_snapshot}`: import/parse + (if a point is attested) mint the anchor from an operator-independent origin + verify-before-bootstrap (fail-closed) + route through the single `bootstrap_initial_state` authority + the N-F-A sidecar tail. **No new authority, no new `*Anchor` trait/plugin, no new `SeedProvenance` variant unless the source genuinely differs** (CN-MITHRIL-01 / CN-NODE-01 / DC-MITHRIL-02 / CN-CINPUT-02). |

> **Note (N-U realizes a SECOND impl of an EXISTING closed extension point — it adds NO new extensible /
> negotiated / plugin surface).** The N-U serve projection `ChainDbServedSource` is a **second impl of the
> ALREADY-CLOSED serve read-side trait seams** `ServedHeaderLookup` / `ServedRangeLookup` (the produce-mode
> `ServedChainLookups` is the first) — selected through the **closed** `ServedChainSource { Snapshot |
> DurableChainDb }` enum read by the SINGLE `dispatch_server_frame_event_to_outbound`. There is **no `Box<dyn
> _>` registry, no runtime-registered serve handler, no negotiated surface, no second serve dispatch** (DC-NODE-07
> preserved). The durable admit `node_sync::admit_forged_block_durably` is a **REUSE** of the single closed
> `pump_block` durable tip authority — NOT a new chokepoint, NO new `NodeBlockSource` / `WalEntry` /
> `CoordinatorEvent` / `Mode` variant, NO new `--mode node` flag, NO new BLUE authority or canonical type. The
> lone BLUE touch (`block_header_bytes(&[u8])`) is an additive fn factored out of `accepted_block_header_bytes`
> (the SAME DC-CONS-18 authority). N-U RETIRES the G-R `serve_gate_admits` monotone-gate mechanism (superseded by
> serve-as-projection — see the RETIRED row in §3 Closed). The serve-seam row + the extended Persisted-ChainDb
> row above are the N-U extensible entries; the `ServedChainSource` selector belongs in the Closed table.
>
> **Note (G-K … G-R is NOT a new extension point — the span adds ONLY closed sums + version-gated fields).** The
> entire G-K…G-R catch-up span introduces **NO extensible / negotiated / plugin / runtime-registered surface.**
> The lone new BLUE canonical type, the closed 2-variant `ArrayHead` (G-M), is a wire-grammar REDUCTION decoded
> by the scoped `decode_array_head_two_form` (FindIntersect-only — NOT a general definite/indefinite decoder);
> the `Origin → IntersectFound[Origin]` reply matches the real node and does NOT widen the served chain. The G-L
> `encode_n2n_version_params` is the single per-version `versionData` encoder over the **unchanged closed**
> `N2N_SUPPORTED` version SET (no widening, no runtime negotiation). The G-N `SeedEpochConsensusInputs.epoch_nonce`
> is a **version-GATED** additive field behind `SEED_CINPUT_SCHEMA_VERSION 1 → 2` (a v1 sidecar fails closed
> `UnknownVersion`, never zero-eta0) — NOT a new type, NOT an open variant. The G-O feed tag-24 unwrap **REUSES**
> the single `ade_codec::unwrap_tag24` (CN-WIRE-08) authority — NO parallel parser. The G-K/G-P/G-Q/G-R deltas are
> **RED orchestration** inside `node_lifecycle` / `node_sync` (serve-lifetime decouple, feed header-validation
> view, forge-successor position, monotone serve gate) — closed RED fences / pure predicates, **NO new closed
> enum / `NodeBlockSource` / `CoordinatorEvent` / `Mode` variant, NO new `--mode node` flag, NO new BLUE
> authority.** There is **no plugin trait, no `Box<dyn _>`, no runtime-registered handler, no negotiated surface.**
> They belong in the Closed table above (and §4 Frozen / §4 Version-gated), not in the Extensible table.
>
> **Note (N-F-G-J is NOT a new extension point — G-J adds ONLY closed sums + one BLUE module).** G-J introduces
> **NO extensible / negotiated surface.** The new closed BLUE sum `PrevHash` is a 2-variant wire grammar (not
> `#[non_exhaustive]`, single POSITION-BLIND codec authority + single POSITION-AWARE validator authority); the
> new GREEN `NodeSchedEvent` / `FeedReason` / `ForgeOutcome` are closed emit-only diagnostic enums (no catch-all,
> no stringly field, planner-never-names them); the additive `BlockValidityError::HeaderPositionInvalid` /
> `NodeForgeError::RecoveredTipMissingBlockNo` are closed-enum variants (no wildcard); the new BLUE module
> `ade_ledger::block_validity::header_position` is a single position-rule authority (a fn + a variant), not a
> registry/plugin point. The genesis-successor cold-start is a new code PATH through the UNCHANGED closed
> `NodeBlockSource`, not a new variant. **No plugin trait, no `Box<dyn _>`, no runtime-registered handler, no
> new `--mode node` flag, no new `NodeBlockSource` / `CoordinatorEvent` / `Mode` / `BlockRejectClass` variant.**
> They belong in the Closed table above, not here.
>
> **Note (N-F-G-D is NOT a new extension point).** The N-F-G-D rehearsal surfaces are **CLOSED / constructor-fenced
> ATTACH POINTS**, not new extensible registries: `RehearsalVenue` is a closed **1-variant** enum (a non-private
> venue is unrepresentable); `PrivateRehearsalManifest` is a constructor-fenced non-promotable envelope (sole ctor
> `from_correlate_outcome`, `None` on `NoEvidence`; `is_rehearsal`/`not_bounty_evidence` literals) WRAPPING the
> existing closed `Ba02Manifest`; `rehearsal_pass` is RED file I/O that REUSES `ba02_pass::correlate_peer_log_file`
> **verbatim** (no alternate correlator). There is **no plugin trait, no `Box<dyn _>`, no runtime-registered
> handler, no new BLUE authority, no new `--mode node` flag, and no new `NodeBlockSource` / `CoordinatorEvent` /
> `Mode` variant** — the path-fidelity fence `ci_check_node_path_fidelity.sh` pins the closed 28-flag allow-list +
> bars a from-genesis consensus-inputs constructor, and `correlate` stays the SOLE `Ba02Manifest` constructor.
> They belong in the Closed table above, not here.
>
> **Note (N-F-G-E is NOT a new extension point).** The N-F-G-E surfaces are **surface REDUCTIONS / fail-closed
> memory bounds**, not new extensible registries: `MAX_REASSEMBLY_TAIL_BYTES` (16 MiB) and `MAX_WIRE_PUMP_LOOKAHEAD`
> (256) are **closed literal constants** with **NO runtime / CLI / env / config override**; the additive
> `SessionError::ReassemblyBufferOverflow` variant is a closed-enum extension (no wildcard, one exhaustive
> consumer). They cap memory in front of the UNCHANGED closed decode path / the UNCHANGED closed verdict-decoupled
> `NodeBlockSource` — no plugin trait, no `Box<dyn _>`, no runtime-registered handler, no new BLUE authority, no
> new `NodeBlockSource` / `CoordinatorEvent` variant. They belong in the Closed table above, not here.
>
> **Note (N-F-G-C is NOT a new extension point).** The N-F-G-C live feed is a **REUSE** of the closed
> `ade_runtime::admission::{dial_for_admission, run_admission_wire_pump}` (no reimpl, no new wire authority) that
> **FILLS** the closed `NodeBlockSource::WirePump` arm via `from_wire_pump` — it is **NOT a new `NodeBlockSource`
> variant / plugin point / source-selection trait**; the durable tip still advances only via `run_node_sync →
> pump_block`. The N-F-G-C `ba02_pass` module is **RED file I/O over the UNCHANGED closed BA-02 evidence
> vocabulary** — it introduces **no new closed enum / registry**, and `correlate` stays the SOLE `Ba02Manifest`
> constructor. Neither belongs in the Extensible table.
>
> **Note (N-F-G-A is NOT an extension point).** The N-F-G-A forge-fidelity surfaces are **surface REDUCTIONS /
> fail-closed boundaries**, not new extensible registries: `SlotAlignmentError` / `ProtocolParamsParseError` /
> `ForgeCurrentPParamsError` / `ForgeEpochAdmission` are closed error/classifier enums; the
> `consensus_inputs::protocol_params` parser is a single RED-parse → BLUE-`ProtocolParameters` pipeline; the
> `protocol_params_json` preimage is a non-fingerprinted additive carry on `LiveConsensusInputsCanonical`. They
> introduce no plugin trait, no `Box<dyn _>` collection, no runtime-registered handler, no new BLUE authority,
> and `CoordinatorEvent` was deliberately NOT extended (the off-epoch outcome reuses `ForgeNotLeader`). They
> belong in the Closed table above, not here. **Likewise N-F-F** (operator-key ingress) is a surface REDUCTION
> (a closed classifier + a single named ingress site reusing existing loaders), and **N-F-G-B**
> (`SelfAcceptedHandoff` + the sibling serve task) is a closed constructor-fenced carrier + a typed channel seam
> — not new extensible registries.

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **ChainSync `FindIntersect` two-form array-head grammar is closed + SCOPED (N-F-G-M, CN-WIRE-11 — load-bearing;
  do NOT soften / do NOT broaden).** The header points-list array-head is the closed BLUE sum `ArrayHead =
  Definite(u64) | Indefinite` (`ade_network::codec::primitives`, the lone new BLUE canonical type in the span,
  457 → 458). `decode_array_head_two_form` accepts BOTH the definite (`9f…`/`98…`) and indefinite (`0x9f … 0xff`)
  forms; on `Indefinite` the caller consumes the matching `0xff` break. The indefinite acceptance is **SCOPED to
  ChainSync `FindIntersect`** (`decode_find_intersect_points`) — it is **NOT** a general definite/indefinite
  decoder; the rest of the closed wire grammar stays definite-only, and there is **no catch-all**. The paired
  `chain_sync::server` `Origin → IntersectFound[Origin]` reply matches the real cardano-node (the universal
  common ancestor) and **MUST NOT widen the served chain** beyond the Origin intersect. Pinned byte-identically
  against captured cardano-node 11.0.1 FindIntersect/IntersectFound fixtures
  (`ci_check_chainsync_findintersect_compat.sh`).
- **Per-version N2N `versionData` wire encoding is the single encoder over the closed version SET (N-F-G-L,
  CN-WIRE-10 — load-bearing).** `encode_n2n_version_params(version, network_magic)` is the SINGLE per-version
  `versionData` encoder (`ade_network::handshake::version_table`, an additive BLUE fn — 458 unchanged): for
  V11..=V15 the 4-field `NodeToNodeVersionData` `[networkMagic, initiatorAndResponderDiffusionMode, peerSharing,
  query]` (`diffusionMode = true`, `peerSharing = NoPeerSharing(0)`, `query = false`), for V16+ the extended
  shape — so a real cardano-node accepts the serve-side handshake at NodeToNodeV_15 (fixes `HandshakeDecodeError
  NodeToNodeV_15 'unknown encoding: TInt 1'`). It is the single encoder over the **unchanged closed
  `N2N_SUPPORTED` version SET** — **NO version-set widening, NO runtime negotiation of meaning**; a successful
  handshake is a wire-layer event, **NOT** peer acceptance (`RO-LIVE-01` stays `partial`). Pinned against the
  real-node handshake fixture (`ci_check_n2n_handshake_versiondata_authority.sh`).
- **Feed-side BlockFetch tag-24 unwrap REUSES the single frozen tag-24 authority (N-F-G-O, CN-WIRE-12 —
  load-bearing).** The feed-side WirePump (`ade_runtime::admission::wire_pump`) strips the peer's BlockFetch
  `MsgBlock` tag-24 (`0xd8 0x18`) envelope via the single `ade_codec::unwrap_tag24` (CN-WIRE-08) authority
  **BEFORE** the BLUE decode (so the inner `[era, block]` decodes cleanly — `UnexpectedType` gone), mirroring the
  already-correct `admission::runner` + `ade_core_interop::follow` paths. **There is NO second `unwrap_tag24` /
  hand-rolled tag-24 parse** — the single frozen authority is reused; a non-tag-24 / malformed payload fails
  closed (`ci_check_feed_tag24_unwrap.sh`).
- **`--mode node` serve listener lifetime is decoupled from feed end + the served tip is monotone (N-F-G-K +
  N-F-G-R, DC-NODE-09 + DC-NODE-11 — load-bearing).** The serve sibling (`run_node_serve_task`, spawned OUTSIDE
  `run_relay_loop`) stays up serving **past feed-end** (its lifetime is independent of the feed source's end —
  `ci_check_node_serve_lifetime.sh`), and once block 0 (the genesis successor) is served the served tip is
  **monotone** — `node_lifecycle::serve_gate_admits(highest_served_block_no, candidate_block_no)` refuses any
  candidate that would regress the served chain (`candidate_block_no <= highest_served_block_no`) or re-serve an
  already-served height with a divergent hash, so a genesis-successor follower adopts a **stable served block 0**
  (`ci_check_served_chain_stability.sh`). The serve sibling is **read-only** over the `ServedChainView` — it
  admits nothing, advances **no durable tip** (no `ChainDb` handle), and makes **no peer-acceptance claim**
  (`RO-LIVE-01` stays `partial`); `run_relay_loop`'s containment is byte-unchanged (the serve task lives outside
  it).
- **Forge-successor position is sourced from the evolved admitted spine + the feed header-validation view from
  the recovered consensus surface (N-F-G-Q + N-F-G-P, DC-NODE-10 + DC-CINPUT-04 — load-bearing).** After the
  genesis-successor block 0 is admitted, the NEXT forge derives `(block_number, prev_hash) = (last_admitted_block_no
  + 1, PrevHash::Block(last_admitted_hash))` from the **evolved admitted spine state** (not the stale recovered
  tip), reusing the single G-J `forge_header_position` convention — the first stable node, block 1+, no successor
  crash (`ci_check_forge_successor_evolved_spine.sh`); the forge holds **no `ChainDb` handle** (advances no
  durable tip — that is N-U). The feed's header-step + ledger Step-5/7 validation derives its leader-threshold
  stake-distribution view from the recovered consensus surface (`PoolDistrView::from_seed_epoch_consensus_inputs`),
  **never a defaulted / empty distribution** — so a valid live block 0 is ingested (`VerificationFailed` gone —
  `ci_check_feed_leader_threshold_view.sh`); the verdict stays BLUE (Steps 5 + 7 are BLUE authorities). **NO
  RO-LIVE flip.**
- **Header `prev_hash` is the closed `$hash32 / null` wire grammar with a SINGLE POSITION-BLIND codec authority +
  a SINGLE POSITION-AWARE validator authority (N-F-G-J S2/S3, CN-WIRE-09 — load-bearing; do NOT soften / do NOT
  broaden).** The Shelley-and-later header `prev_hash` field is the closed sum `PrevHash = Genesis | Block(Hash32)`
  (cardano-ledger `PrevHash = GenesisHash / BlockHash`); `Genesis` is CBOR null, `Block(h)` is a 32-byte hash32.
  The raw byte codec is **POSITION-BLIND**: `ade_codec::shelley::block::decode_prev_hash` decodes `null -> Genesis`
  / `hash32 -> Block` as a pure function of the CBOR token and the `ShelleyHeaderBody` `AdeEncode` writes
  `Genesis -> write_null` / `Block(h) -> write_bytes_canonical` — **NEVER consulting `block_number`**. The
  position-AWARE coupling (`block_number 0 <=> Genesis`) lives in EXACTLY ONE BLUE authority,
  `ade_ledger::block_validity::header_position::check_header_position`, called by `decode_block` BEFORE the header
  authority, surfacing `BlockValidityError::HeaderPositionInvalid` (coarse class `HeaderInvalid` — **no new
  `BlockRejectClass`**); it **MUST NOT** live in `ade_codec` and **MUST NOT** be duplicated. The `null` grammar is
  scoped to **header_body ONLY** — it **MUST NOT** leak into the chain-sync/block-fetch `Point`/`Tip` codec
  (`Point::Origin` stays `array(0)`). No all-zero `Hash32`, no anchor fingerprint, and no Shelley genesis hash may
  stand in for the genesis predecessor — it is structurally CBOR null (the producer `prev_hash` migrated
  `Hash32 -> PrevHash` end to end, deleting the all-zero stand-in at its source in `ChainEvolution::prev_hash()`).
  `ci_check_prevhash_single_wire_authority.sh` fences the single wire authority + the header_body-only null scope.
- **Genesis-successor cold-start forge is scoped to the hermetic execution; the recovered lineage gates PERMISSION,
  not the prev_hash bytes (N-F-G-J S4, DC-NODE-08 — load-bearing).** On the `--mode node` `On` arm, the
  `LoopStep::ForgeTick` branch derives `(block_number, prev_hash)` via the GREEN
  `node_sync::forge_header_position(Option<&ChainTip>)` — the ONE cold-start convention: `None => (0,
  PrevHash::Genesis)`; `Some => (last_block_no+1, PrevHash::Block(tip.hash))`; a tip-without-height edge fails
  closed `NodeForgeError::RecoveredTipMissingBlockNo` (never `.unwrap_or(1)`). When `ChainDb::tip()` AND the
  recovered tip are BOTH `None`, the GREEN `node_lifecycle::may_cold_start_forge(selected_tip_present=false,
  has_recovered_lineage, feed_eligible)` permits forging block 0 + `PrevHash::Genesis` through the SAME
  `run_real_forge → self_accept → SelfAcceptedHandoff → ServedChainView` path S3 proved. The cold-start fires
  **within the hermetic execution** (NO `genesis_forged` latch); the recovered seed-epoch lineage gates
  **PERMISSION** to forge from the genesis-successor position, it is **NOT** the source of the prev_hash bytes
  (which are structurally null); the forge engine holds **no `ChainDb` handle**, so it advances **no durable
  tip** (durable block-1+ progression is N-U). **NO RO-LIVE flip** (`RO-LIVE-01` stays `partial`); the cold-start
  eligibility signal is general (forge-configured + valid recovered base), **never a private-only / C1-only
  flag**. (`ci_check_genesis_successor_reachability.sh`.)
- **`--mode node` feed/forge scheduling-event vocabulary is closed + emit-only (N-F-G-J S1, CN-NODE-04).** The
  GREEN `ade_node::live_log::{sched_event, sched_writer}` `NodeSchedEvent` / `FeedReason` / `ForgeOutcome` are
  closed enums (none `#[non_exhaustive]`; no catch-all / `Other`; no stringly field), byte-deterministically
  JSONL-encoded. **EMIT-ONLY:** the GREEN planner never constructs or reads them; `run_relay_loop_with_sched`
  records the feed/forge scheduling decisions WITHOUT changing them. `ci_check_node_sched_events_emit_only.sh`
  enforces the planner never names the vocabulary; allow-list + negative tests.
- **Live-feed peer-driven memory is bounded BEFORE authoritative decode/apply by two closed literal caps
  (N-F-G-E, DC-LIVEMEM-01 — load-bearing; do NOT soften / do NOT broaden).** Two closed constants, each
  fail-closed in front of the BLUE `ade_codec` decode path, with **NO runtime / CLI / env / config escape hatch**
  (`ci_check_live_feed_memory_bounds.sh` guard 3): (1) GREEN `ade_network::session::core`
  `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` — after `drain_protocol_items` drains every COMPLETE item, an incomplete
  per-mini-protocol reassembly tail over the cap returns the additive closed `SessionError::ReassemblyBufferOverflow`
  and `mux_pump::session_err_to_halt` drops the peer (no silent truncation, no partial decode); (2) RED
  `ade_node::node_sync` `MAX_WIRE_PUMP_LOOKAHEAD = 256` — `pump_lookahead` stops the opportunistic drain at the
  cap so the bounded `mpsc` (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`) back-pressures the pump. These are **defensive
  implementation bounds, NOT Cardano semantic parameters** — a future hardening slice may **tighten** them, but
  may NEVER disable them or set them unbounded. The claim is **NARROW**: bounded memory before authoritative
  decode/apply — **NOT** full network DoS resistance, **NOT** peer resource fairness, **NOT** BA-02 / live-evidence
  readiness (the per-connection aggregate ceiling across the ~10 independent per-protocol reassembly buffers and
  per-connection-COUNT / peer-fairness are a SEPARATE, out-of-scope surface). **NO BLUE change** (`ade_codec`
  byte-unchanged); **NO new `NodeBlockSource` / `CoordinatorEvent` variant**; the serve/forge/containment fences
  (`ci_check_node_run_loop_containment.sh` + `ci_check_served_chain_handoff_fence.sh`) are **byte-unchanged**; **no
  live-evidence / BA-02 / RO-LIVE flip or claim**. Precision (record, not overclaim): the reassembly check is
  post-extend, so a single buffer's transient peak is `cap + one <=64 KiB mux frame` (~16.06 MiB), and
  `ProtoBuffers` holds up to ~10 INDEPENDENT per-protocol buffers each capped separately (per-connection aggregate
  ~10× the single-buffer cap — still O(constant) per connection).
- **Peer-driven `--mode node` SERVE-range work is bounded BEFORE unbounded storage/CPU by a fixed defensive cap
  (N-AA, DC-SERVEMEM-01 — load-bearing; do NOT soften / do NOT broaden; the serve-side analog of the
  DC-LIVEMEM-01 receive-side bound above).** The `--mode node` durable-chain serve (`ChainDbServedSource`,
  DC-NODE-13) reads each peer BlockFetch range via the bounded **hash-free** ChainDb primitives `range_bytes_capped`
  + `last_block_bytes` (NOT the unbounded `iter_from_slot` / O(N) `chaindb.tip()` — which materialized the full
  `from..tip` range and recovered each hash via a per-block `SLOT_BY_HASH` scan, O(N²)), caps each request at the
  **fixed `MAX_SERVE_RANGE_BLOCKS = 256`** (SYMMETRIC with `MAX_WIRE_PUMP_LOOKAHEAD = 256`), and fails closed via
  the closed `ServeRangeOutcome` — `CapExceeded` on an oversized range (decided BY THE BOUND, before any decode),
  `Empty` on an out-of-chain / inverted (`from > to`) window, `ReadError` on an in-range block the single BLUE
  `decode_block` cannot authenticate; **every non-`Served` outcome maps to the wire `NoBlocks`**. The cap is a
  **defensive implementation bound, NOT a Cardano semantic parameter** (it does not change WHICH chain is served,
  only the per-request work ceiling) with **NO runtime / CLI / env / config escape hatch, no unbounded mode**
  (`ci_check_serve_range_bounded.sh`); a future hardening slice may **tighten** it, but may NEVER disable it or set
  it unbounded. The block hash is derived from each block's own bytes via the single BLUE `decode_block` authority
  (**NO second hash authority, NO `SLOT_BY_HASH` reference on the serve path**); the bounded primitive is hash-FREE.
  The single serve `dispatch_server_frame_event_to_outbound` is unchanged (**DC-NODE-07 preserved** —
  `ServeRangeOutcome` is an internal serve-read outcome, NOT a dispatch). Within-cap serving stays **byte-identical**
  to the pre-cluster behavior (**DC-CONS-17 preserved**). The trusted recovery / rollback `iter_from_slot` / `tip`
  internals are **OUT OF SCOPE** (doc-fenced, not peer-driven, unchanged). The claim is **NARROW**: a per-request
  serve-range bound on the read-only serve path — **NOT** full network DoS resistance, **NOT** peer-fairness, **NOT**
  a per-connection-COUNT / repeated-request limit (those remain SEPARATE, out-of-scope future hardening, as with
  DC-LIVEMEM-01), and **NOT** an RO-LIVE flip (`RO-LIVE-01` stays `partial` / operator-gated).
- **Outbound mini-protocol payloads are SEGMENTED into ordered mux frames by a fixed defensive bound
  (N-AB, CN-SESS-05 — load-bearing; do NOT soften / do NOT broaden; the outbound inverse of the CN-SESS-04 inbound
  reassembly and the symmetric outbound counterpart of the DC-LIVEMEM-01 receive-side `MAX_REASSEMBLY_TAIL_BYTES`).**
  The GREEN `ade_network::session::core::handle_outbound` segments a payload with
  `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` into ordered `<= MAX_PAYLOAD` chunks (`payload.chunks(MAX_PAYLOAD)`),
  each encoded via the SAME single-frame `encode_inner_frame` → `mux::frame::encode_frame` authority (`encode_inner_frame`
  keeps its strict `MAX_PAYLOAD` guard — **never a second / parallel frame encoder**); every segment carries the SAME
  mini-protocol id + mode + the SAME single captured `timestamp` input (GREEN — no per-segment clock); an empty payload
  emits exactly one (empty) frame; `concat(segment payloads) == payload` exactly (byte-preserving, lossless, ordered —
  **DC-CONS-17 preserved**), so Ade's own inbound CN-SESS-04 reassembly reconstructs the original. The
  **fixed `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB`** (SYMMETRIC with `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB`) is a **closed
  literal constant — a defensive implementation bound, NOT a Cardano semantic parameter** with **NO runtime / CLI /
  env / config escape hatch, no unbounded mode, no partial-success send**; a payload ABOVE it fails closed with the
  **existing** closed `SessionError::OutboundPayloadTooLarge { len }` (REUSED — **NOT a new variant / new closed
  enum**) BEFORE any segmentation (`ci_check_outbound_segmentation.sh`); a future hardening slice may **tighten** it,
  but may NEVER disable it or set it unbounded. There is **NO new BLUE canonical type** (`MAX_OUTBOUND_PAYLOAD_BYTES`
  is a GREEN const; 458 unchanged), **NO BLUE change** (`mux::frame` reused unchanged), and the claim is **NARROW**:
  segmenting bytes the producer already produced into legal mux frames — it does **NOT** decide which block is served,
  **NOT** advance a durable tip, and is **NOT** an RO-LIVE flip (`RO-LIVE-01` stays `partial` / operator-gated;
  confirming a real cardano-node demux accepts the reused-per-segment SDU timestamp is the live leg).
- **Live `--mode node` feed reuses the closed dial/pump + FILLS the closed source (N-F-G-C S1, load-bearing).**
  The live feed for the `--mode node` `On` arm is a **REUSE** of the closed `ade_runtime::admission::{dial_for_admission,
  run_admission_wire_pump}` (no reimplementation, no new wire authority) that **FILLS** the closed
  `NodeBlockSource::WirePump` arm via `from_wire_pump`. `NodeBlockSource` stays the **closed 2-variant `{WirePump,
  InMemory}`** — `from_wire_pump` is a FILL of the existing arm, **NOT a new variant, NOT a plugin point, NOT a
  source-selection trait**. The bounded `mpsc` (`LIVE_WIRE_PUMP_CHANNEL_CAP = 64`) bridges the reused pump's
  `AdmissionPeerEvent` into the source. The **durable tip advances ONLY via `run_node_sync → pump_block`** (no
  second tip-advance; `run_node_sync` is UNMODIFIED). A dial/parse failure is logged-and-dropped (C3 honest-scope)
  — never fatal / fabricated / a tip graft. `ci_check_node_run_loop_containment.sh` is **byte-unchanged**;
  `ci_check_served_chain_handoff_fence.sh` was **BROADENED in place** (owners `{node_lifecycle.rs, node_sync.rs}`;
  guard-3 allow-list — a net tightening). **No new BLUE authority, no new canonical type, no new `NodeBlockSource`
  / `CoordinatorEvent` variant.**
- **BA-02 evidence I/O is RED file I/O over the SOLE GREEN `correlate` constructor (N-F-G-C S2, load-bearing).**
  `correlate` stays the **SOLE `Ba02Manifest` constructor**. The RED `ade_node::ba02_pass::correlate_peer_log_file`
  reads the operator-captured peer-log file → `correlate`; `write_ba02_manifest` accepts **ONLY a `Ba02Manifest`**,
  so a written manifest is **always correlate-produced** from a real peer log. A missing/unreadable file fails
  closed (`io::Error`), never a synthesized acceptance. **No self-evidence acceptance source** (not `ForgeSucceeded`
  / `self_accept` / `block_received` / served-block / wire-success / `agreement_verdict` / "agreed"). **No synthetic
  manifest is committed** — `ci_check_ba02_evidence_manifest_schema.sh` is **vacuous-until-committed**, and when a
  `CE-G-C-LIVE_*.toml` is committed it verifies the closed **8-field** schema + `schema_version == 1` +
  `peer_log_file_sha256 == sha256(committed fixture)` (the no-synthetic-manifest enforcer). The versioned
  `Ba02Manifest` (`BA02_MANIFEST_SCHEMA_VERSION = 1`) is a version-GATED contract (below).
- **Self-accept→serve handoff fence (N-F-G-B S1-S3, DC-NODE-06 — ENFORCED; BROADENED N-F-G-C).** Only a BLUE
  self-accepted `AcceptedBlock` can be served on the `--mode node` spine. The typed carrier `SelfAcceptedHandoff`
  has a **private field + a SOLE constructor** `from_self_accepted(AcceptedBlock)` — there is **NO** raw-bytes /
  `ForgedBlockArtifact` / `CoordinatorEvent` / flag / verdict constructor, so a non-self-accepted artifact is
  type-unrepresentable as a handoff. The token is the ORIGINAL from BLUE `self_accept` (CN-FORGE-01), never
  re-derived from `artifact.bytes`. The relay-loop body forwards a **typed `mpsc<SelfAcceptedHandoff>` send only**;
  served-chain mutation happens in a **sibling** task via the single `ServedChainHandle::push_atomic` fed **only**
  by `into_accepted()`. **The serve ingress fence MUST hold** (`ci_check_served_chain_handoff_fence.sh`, CE-G-B-3;
  **BROADENED by N-F-G-C** — owners `{node_lifecycle.rs, node_sync.rs}`, guard-3 flipped to an allow-list: every
  node-spine `push_atomic(` fed by `into_accepted()`; no direct `served_chain_admit(`; every node-spine unbounded
  handoff channel typed `UnboundedSender<SelfAcceptedHandoff>`). `ci_check_node_run_loop_containment.sh` stays
  **byte-/semantically unchanged**. **No new BLUE authority, no new canonical type, no new `CoordinatorEvent`
  variant.**
- **`LiveConsensusInputsCanonical` 15-field fingerprint + the non-fingerprinted `protocol_params_json` carry
  (N-F-G-A S2a, load-bearing).** The bundle fingerprint is the **frozen 15-field** canonical CBOR of
  `LiveConsensusInputsCanonical` — it already commits to `protocol_params_hash`. The N-F-G-A
  `protocol_params_json: Option<String>` preimage is carried **OUTSIDE** that fingerprint: a **non-fingerprinted
  additive carry, NOT a fingerprint-schema change** — adding it does **not** alter the bundle fingerprint, the
  field count stays 15, and the canonical CBOR is byte-unchanged. `require_forge_current_pparams` re-binds the
  preimage to the fingerprinted `protocol_params_hash` via `blake2b_256` before parsing. **The preimage MUST stay
  OUTSIDE the 15-field canonical CBOR fingerprint** (`ci_check_recovered_ledger_pparams_sourced.sh`, CE-G-A-2a) —
  folding it INTO the fingerprint is a CI failure.
- **Current-protocol-parameters source has no float path (N-F-G-A S2a).** `parse_protocol_parameters_json`
  converts the oracle pparams JSON into the canonical BLUE `ProtocolParameters` using **exact integer
  arithmetic** on `serde_json::value::RawValue` strings → `ade_ledger::rational::Rational`; no `f64`, no `as
  f64`, no serde float deserialization. A literal that cannot be represented exactly fails closed
  (`ProtocolParamsParseError::InexactRational`). The recovered ledger carries the **current** parameters
  (never `ProtocolParameters::default()` / genesis-initial), installed at seed/import (CE-G-A-2a).
- **Checked clock→slot guard fails closed before-anchor (N-F-G-A S3).** `checked_millis_to_slot` returns
  `Err(SlotAlignmentError::BeforeGenesisAnchor)` for a before-anchor tick — it **MUST NOT saturate** to
  `start_slot` (the exact case the saturating `millis_to_slot` masks). No float, no wall-clock; the saturating
  `millis_to_slot` is left intact for non-forge callers. The forge `ForgeTick` derives its slot via the checked
  guard.
- **Off-epoch forge guard fails closed before leadership (N-F-G-A S4, DC-EPOCH-03).** `forge_one_from_recovered`
  calls `forge_epoch_admission` **before** `query_leader_schedule`; an off-epoch / unlocatable slot fails
  closed (`ForgeEpochAdmission::OffEpoch`) before leadership / KES signing. The candidate epoch is derived via
  the BLUE `EraSchedule::locate` (no fabricated epoch math); the node forge path drives **no**
  `NonceInput::EpochBoundary` / `CandidateFreeze` nonce promotion (cross-epoch production is a separate
  nonce-roll cluster). The off-epoch outcome routes through the existing `CoordinatorEvent::ForgeNotLeader`
  (no new event variant). (`ci_check_node_forge_single_epoch_fail_closed.sh`.)
- **Real cardano-cli config ingress on the node path (N-F-G-A S2).** The `--mode node` operator-forge ingress
  loads opcert + genesis through the REAL `parse_opcert_envelope` + `parse_shelley_genesis`; the `parse_simple_*`
  stubs are RETIRED on the node forge path (`ci_check_node_forge_real_cli_ingress.sh`, CE-G-A-2). The genesis
  reuse extracts clock/KES/network constants ONLY — never a starting-state source.
- **Genesis-consistency fixture is evidence-only (N-F-G-A S1).** The committed private-net Ade-as-leader
  reference fixture (`consensus-inputs.json` + `shelley-genesis.json` + `PROVENANCE.md`) is **evidence input,
  never runtime authority** — never a production source of eta0 / stake / ASC / VRF keyhash; the in-test
  sidecar pre-seed is `#[cfg(test)]`-confined. The fixture must be committed + well-formed + Ade-as-leader and
  carry NO secret key material (`ci_check_genesis_consistency_fixture_present.sh`, CE-G-A-1).
- **Operator-key ingress contract (N-F-F, CE-not-law)** — `--mode node` operator-material ingress is the SINGLE
  named `operator_forge` site reusing the existing loaders; key custody RED-confined to `ProducerShell`
  (`OperatorForgeMaterial` not `Debug`/`Serialize`); the forge-on flip is opt-in over flag presence
  (`ci_check_forge_intent_closed.sh` + `ci_check_operator_forge_no_secret_leak.sh`, CN-NODE-03).
- **`--mode` taxonomy** — the 5-variant closed `Mode` set with a wildcard-free `main.rs` dispatch arm per variant
  (`ci_check_node_mode_closure.sh`, CN-NODE-MODE-01). N-F-G-C added NO `Mode` variant (the `--peer` flag is
  OPTIONAL ingress on the existing `--mode node` arm).
- **Single `bootstrap_initial_state` authority** — all initial state (produce cold-start, Conway genesis, the
  Mithril provenance + production-bootstrap paths, the N-F-A warm-start restore, both N-F-C lifecycle arms)
  routes through this one chokepoint; no `*Anchor` trait/plugin seam (CN-NODE-01 / CN-MITHRIL-01). N-F-G-C's live
  feed introduces no second bootstrap.
- **Recovered eta0 is explicit + replayable + version-gated; the WarmStart overlay supplies it to the forge VRF
  input (N-F-G-N, T-REC-04 + DC-CINPUT-03 — load-bearing; do NOT soften / do NOT broaden).** The closed BLUE
  `SeedEpochConsensusInputs` gains the additive field `epoch_nonce: Nonce` (the recovered Praos eta0 for
  `epoch_no`) and its sole codec bumps `SEED_CINPUT_SCHEMA_VERSION 1 → 2`; the GREEN merge
  (`seed_consensus_merge`) carries the recovered eta0 verbatim into the field (never default-to-zero), and
  **WarmStart overlays it onto `chain_dep.epoch_nonce`** so the forge VRF input is
  `praos_vrf_input(slot, epoch_nonce)` (a recovered follower no longer fails `VRFKeyBadProof`). The schema bump is
  **VERSION-GATED**: a v1 sidecar that omitted `epoch_nonce` decodes as `UnknownVersion` — **NEVER a
  default-to-zero eta0** (`ci_check_warmstart_eta0_overlay.sh` pins the sidecar carries `epoch_nonce`, that
  WarmStart overlays it, and that the forge VRF input is `praos_vrf_input(slot, epoch_nonce)`). The WarmStart
  overlay supplies the recovered eta0 to the forge VRF input but does **NOT** make the recovered sidecar a
  runtime authority for nonce evolution — the BLUE `consensus::nonce` transitions stay undriven on the forge
  path (DC-EPOCH-03). `SeedEpochConsensusInputs` stays a single closed canonical type with the SOLE codec; the
  field is additive ONLY behind the version gate (NOT a new type — 458 unchanged, NOT an open/extensible
  variant).
- **`SeedEpochConsensusInputs` SOLE codec** — the A1 version-gated, byte-canonical, `BTreeMap`-ordered
  encoder/decoder; **`SEED_CINPUT_SCHEMA_VERSION = 2`** (N-F-G-N — the now-7-field outer record carrying
  `epoch_nonce`; a v1 sidecar decodes as `UnknownVersion`); no `Default` / `#[non_exhaustive]` (CN-CINPUT-01,
  strengthened by T-REC-04 / DC-CINPUT-03).
- **tag-24 wire envelope** — the single `ade_codec::cbor::tag24::{wrap_tag24, unwrap_tag24}` authority; the
  BlockFetch (storage Conway = 7) vs ChainSync (consensus Conway = 6) era-index schemes are pinned byte-identically
  against cardano-node 11.0.1 (CN-WIRE-08). N-F-G-C's live feed reuses `admission::runner`'s tag-24 unwrap verbatim.
- **block-envelope grammar** — the single `encode_block_envelope` / `decode_block_envelope` pair, storage-form
  `[era, block]`, Conway discriminant 7 (CN-FORGE-03).
- **era→leader-VRF-input construction** — the single `leader_vrf_input` authority per era/protocol; the Praos
  producer alpha equals the validator alpha (CN-FORGE-04).
- **Sum6KES algorithm + 608-byte cardano-cli skey envelope** — byte-identical to Haskell `cardano-base`;
  `raw_deserialize_signing_key_kes` is the structural validator (DC-CRYPTO-04..09).
- **Wire format / encoding**: `postcard` / CBOR per the pinned manifest versions; field order = wire order where
  applicable. **All 458 canonical types**: existing wire formats frozen; new types may be added behind a version
  gate (the lone canonical-type add in the G-K…G-R span is the closed `ArrayHead` enum, G-M — 457 → 458). **Hash algorithms**: `blake2b_256` / `blake2b_224` — algorithm immutable per version.

### Version-gated (can evolve across major versions)

- New variants in the closed message / event / classifier taxonomies (`Mode`, `LoopStep`, `CoordinatorEvent`,
  `SeedProvenance`, `SyncEffect`, `ExpectedVrfInput`, `BA02Outcome`, `PeerAcceptEvent`, `PrevHash` (N-F-G-J),
  `NodeSchedEvent` / `FeedReason` / `ForgeOutcome` (N-F-G-J), `ArrayHead` (N-F-G-M — the FindIntersect two-form
  array-head grammar), …) — each requires a new envelope/schema version +
  a wildcard-free dispatch arm + a registry-rule strengthening. **`PrevHash`** additionally requires the codec
  to stay POSITION-BLIND, the `null` grammar to stay header_body-scoped (out of the `Point`/`Tip` codec), and the
  single wire + single position authority to stay un-duplicated (CN-WIRE-09); **the `NodeSchedEvent` family** must
  stay emit-only with no catch-all variant and no planner reference (CN-NODE-04).
- The `Ba02Manifest` schema (`BA02_MANIFEST_SCHEMA_VERSION = 1`) — additions bump the schema version; the N-F-G-C
  `CE-G-C-LIVE_*.toml` operator-pass manifest schema (8 closed keys) evolves with it (RO-LIVE-06 /
  CN-OPERATOR-EVIDENCE-01).
- The redb `chaindb` `SCHEMA_VERSION` (currently **v3**, anchor-fp-keyed seed-epoch sidecar namespace) — a
  versioned gate, not a frozen contract (N-F-A).
- The `ANCHOR_SCHEMA_VERSION = 2` (`SeedProvenance`) + the **`SEED_CINPUT_SCHEMA_VERSION = 2`** (N-F-G-N — the
  7-field `SeedEpochConsensusInputs` carrying the additive `epoch_nonce: Nonce` eta0 field; a v1 sidecar decodes
  as `UnknownVersion`, never zero-eta0) decoders — version-gated. The schema bump is the canonical example of a
  **version-GATED field add**: the recovered eta0 is additive ONLY behind the `1 → 2` bump, fenced by
  `ci_check_warmstart_eta0_overlay.sh` (T-REC-04 / DC-CINPUT-03).
- Canonical type schema additions (new fields appended; sort/dedup + `BTreeMap` ordering invariants preserved).
- `WalEntry` wire tags (append-only: `AdmitBlock` = 0, `SeedEpochConsensusInputsImported` = 3; 1/2 reserved) — a
  CE-not-law additively-evolvable surface.
- The N-F-G-E memory bounds (`MAX_REASSEMBLY_TAIL_BYTES = 16 MiB`, `MAX_WIRE_PUMP_LOOKAHEAD = 256`), the N-AA
  serve-range cap (`MAX_SERVE_RANGE_BLOCKS = 256`, symmetric) AND the N-AB outbound segmentation cap
  (`MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB`, the symmetric outbound counterpart of `MAX_REASSEMBLY_TAIL_BYTES`) —
  closed literal constants that a future hardening slice may **tighten** (a strengthening of DC-LIVEMEM-01 /
  DC-SERVEMEM-01 / CN-SESS-05), but may NEVER make tunable / unbounded; they carry **no CLI / env / config
  override** at any version (`ci_check_live_feed_memory_bounds.sh` guard 3 / `ci_check_serve_range_bounded.sh` /
  `ci_check_outbound_segmentation.sh`).
- The N-F-G-D `PrivateRehearsalManifest` schema (`REHEARSAL_MANIFEST_SCHEMA_VERSION = 1`, closed 12-key TOML; the
  closed 1-variant `RehearsalVenue`) — a version-gated rehearsal-evidence envelope; additions bump the schema
  version. The type stays incapable of representing a non-rehearsal (`is_rehearsal`/`not_bounty_evidence`
  literals), correlate-produced, rehearsal-home-only, and flips NO RO-LIVE rule at any version
  (CN-REHEARSAL-FIDELITY-01 / `ci_check_rehearsal_manifest_schema.sh`).
- New CI checks (existing checks may be **tightened, never relaxed** — e.g. N-F-G-C BROADENED
  `ci_check_served_chain_handoff_fence.sh` in place, a net tightening; N-F-G-E added
  `ci_check_live_feed_memory_bounds.sh`; N-F-G-D added `ci_check_node_path_fidelity.sh` +
  `ci_check_rehearsal_manifest_schema.sh`; **N-F-G-K…G-R added 8** — one gate per cluster
  (`ci_check_node_serve_lifetime.sh`, `ci_check_n2n_handshake_versiondata_authority.sh`,
  `ci_check_chainsync_findintersect_compat.sh`, `ci_check_warmstart_eta0_overlay.sh`, `ci_check_feed_tag24_unwrap.sh`,
  `ci_check_feed_leader_threshold_view.sh`, `ci_check_forge_successor_evolved_spine.sh`,
  `ci_check_served_chain_stability.sh`) → **134**; the C1-evidence commit BROADENED
  `ci_check_rehearsal_manifest_schema.sh` in place again to ALSO cover the c1 rehearsal manifests — a net
  tightening, NOT a new file, CI count unchanged; **N-U** added +2 − 1 retired → **135**; **N-AA** added 1
  (`ci_check_serve_range_bounded.sh`, DC-SERVEMEM-01) → **136**; **N-AB** added 1
  (`ci_check_outbound_segmentation.sh`, CN-SESS-05) → **137**; **N-AC** added 1
  (`ci_check_kes_evolution_before_sign.sh`, DC-CRYPTO-10) → **138**; **N-AE** added 3
  (`ci_check_forge_followed_tip_admission.sh`, DC-NODE-15 + DC-CONS-24; `ci_check_recovered_anchor_intersectable.sh`,
  DC-NODE-14; `ci_check_recover_follow_wal_lineage.sh`, DC-WAL-02 + T-REC-05 strengthened) → **141**
  (DC-PROTO-10, the AE.E chain-sync server FindIntersect cursor fix, is test-enforced — `ci_script = ""` — so it
  mints no gate)).
- **N-AE added NO new version-gated CONTRACT.** The chain-sync wire grammar (the closed `Point` enum +
  `ChainSyncMessage` set) is reused UNCHANGED — DC-PROTO-10 is a server-INTERNAL `last_announced` cursor-state
  fix, not a schema/envelope/version bump; the AE.B recovered-anchor intersect is a proof-gated FindIntersect-ONLY
  read-only behavior over the EXISTING serve seams (no new schema). The only additive BLUE symbol is the read-only
  `DecodedBlock.prev_hash` field (already parsed from the header) — a struct-field addition under the existing
  frozen block codec, NOT a new wire-schema-versioned contract (458 canonical types unchanged); the closed
  `PrevHash` wire grammar it reads (`Genesis` / `Block`) is the N-F-G-J one, unchanged. No `*_SCHEMA_VERSION`
  bump, no new message-taxonomy variant, no new envelope.

---

## 5. Module Addition Rules

How new modules enter the workspace.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` crate, or a BLUE `ade_network` submodule path in `.idd-config.json` `core_paths`; `// Core Contract:` + `//! BLUE …` banner first line | `#![deny(unsafe_code)]`, `deny(unwrap_used / expect_used / panic / float_arithmetic)`; no `#[cfg(feature = …)]` semantic gating | Other BLUE modules only (`ade_types` ← `ade_codec`/`ade_crypto` ← `ade_core` ← `ade_ledger`/`ade_plutus`; `ade_network` BLUE submodules ← `ade_codec`+`ade_types`) | `ade_runtime`, `ade_node`, `ade_core_interop`, the RED half of `ade_network`; std runtime / I/O / clock / rand / `HashMap` / float / async |
| **GREEN** | `ade_testkit` crate, `ade_network::session`, or a GREEN-by-content sub-tree inside `ade_runtime` / `ade_node` (incl. `forward_sync::reducer`, `seed_consensus_merge` (N-F-A), `consensus_inputs::protocol_params` + `consensus_inputs::canonical::require_forge_current_pparams` (N-F-G-A), `clock::checked_millis_to_slot` (N-F-G-A), `ba02_evidence` (N-F-C), `producer::self_accepted_handoff` (N-F-G-B), `run_loop_planner` (N-F-D/N-F-E), `forge_intent` (N-F-F), `node_sync::forge_epoch_admission` (N-F-G-A, GREEN-by-fn), `rehearsal_evidence` (N-F-G-D, the non-promotable `PrivateRehearsalManifest` envelope), `live_log::{sched_event, sched_writer}` (N-F-G-J, the closed emit-only `NodeSchedEvent` / `FeedReason` / `ForgeOutcome` scheduling vocabulary + its byte-deterministic JSONL encoder), `node_sync::forge_header_position` + `node_lifecycle::may_cold_start_forge` (N-F-G-J, GREEN-by-fn — the cold-start header-position + permission decisions inside RED `ade_node`), `harness::sync_diff`, `consensus::genesis_pinning` (N-F-G-A, `#[cfg(test)]`)) with a `//! GREEN …` / `// GREEN` banner | Same deny attributes as BLUE; a purity CI gate per sub-tree (`run_loop_planner`: `ci_check_loop_planner_closed.sh`; `forge_intent`: `ci_check_forge_intent_closed.sh`; `protocol_params` + `require_forge_current_pparams`: `ci_check_recovered_ledger_pparams_sourced.sh`; `forge_epoch_admission`: `ci_check_node_forge_single_epoch_fail_closed.sh`; `self_accepted_handoff`: `ci_check_served_chain_handoff_fence.sh`; `genesis_pinning`: `ci_check_genesis_consistency_fixture_present.sh`; `rehearsal_evidence`: `ci_check_rehearsal_manifest_schema.sh`) | BLUE modules | RED modules in non-test deps; nondeterminism; secret material; float; participation in authoritative outputs |
| **RED** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_network::mux::transport` (incl. `forward_sync::pump`, `mithril_import`, `genesis_bootstrap`, `mithril_bootstrap` (N-Z), `seed_consensus_provenance` (N-F-A), `recovery::restart`, `node_lifecycle` (incl. `run_relay_loop` + `ForgeActivation` + `spawn_live_wire_pump_source`, N-F-D/N-F-E/N-F-G-A/N-F-G-C), `node_sync` (N-F-C), `ba02_pass` (N-F-G-C, the operator-pass BA-02 evidence I/O), `rehearsal_pass` (N-F-G-D, the rehearsal-evidence I/O reusing `ba02_pass::correlate_peer_log_file`), the N-F-G-J cold-start wiring in `node_lifecycle` (`may_cold_start_forge` permission gate, GREEN-by-fn) + `node_sync` (`forge_header_position` + the additive `NodeForgeError::RecoveredTipMissingBlockNo`), `operator_forge` (N-F-F; N-F-G-A real parsers), `admission::{seed_to_snapshot, bootstrap}` (N-F-G-A current-pparams install; N-F-G-C `build_n2n_version_table` `pub(crate)`); `*_mode.rs` for mode handlers); `//! RED …` banner | tokio/std/I/O allowed; the `Clock` seam is the SOLE wall-clock observation reachable from a relay-loop/orchestrator driver (N-F-G-A: the forge path uses the checked `checked_millis_to_slot`); key custody confined to `ProducerShell` | Any module | — (RED is the leaf) |

### New module checklist

1. Add to `Cargo.toml` `[workspace] members` (BLUE submodule paths: also add to `.idd-config.json`
   `core_paths`).
2. Apply the `// Core Contract:` + `//! BLUE|GREEN|RED` banner first line (`ci_check_module_headers.sh`).
3. BLUE/GREEN: inherit the deny attributes; pass `ci_check_forbidden_patterns.sh`,
   `ci_check_no_async_in_blue.sh`, `ci_check_no_semantic_cfg.sh`.
4. `ci_check_dependency_boundary.sh` rejects forbidden cross-color imports; `ci_check_pallas_quarantine.sh`
   confines `pallas-*` to `ade_plutus`.
5. New canonical types: add round-trip tests (`canonical_type_registry: null`; canonical-type rules live
   inline in registry family T).
6. New closed surface: add a `[[rules]]` entry + a CI gate; reference it by ID in the docs.
7. **New seed source: route through `bootstrap_initial_state` — NO `*Anchor` trait/plugin seam**
   (`ci_check_mithril_uses_bootstrap_initial_state.sh`). **A production-bootstrap composition attaches as a
   composition-only RED twin of `bootstrap_from_{conway_genesis, mithril_snapshot}`: verify-before-bootstrap,
   fail-closed, operator-independent `seed_point` origin** (`ci_check_mithril_seed_point_independence.sh`),
   **and (N-F-A) a sidecar tail that WRITES — never consumes — the recovered seed-epoch surface, populate-side
   only** (`ci_check_consensus_input_provenance.sh` — CN-CINPUT-02).
8. **New recovered/canonical record with a SOLE codec (like N-F-A):** put the type + its encoder/decoder in a
   single BLUE module; version-gate the decoder; keep it `BTreeMap`-ordered, byte-canonical, no `Default` /
   `#[non_exhaustive]` (CN-CINPUT-01-style).
9. **If a rule cites a moved/renamed source path: update its `code_locus`** —
   `ci_check_registry_code_locus_exists.sh` fails closed on any cited `crates/**.rs` / `ci/**.sh` path that
   does not exist on disk.
10. **New `--mode` (N-F-C rule):** (i) add the variant to the CLOSED `Mode` enum (not `#[non_exhaustive]`);
    (ii) add a `Mode::parse` arm + a wildcard-free `main.rs` arm (`ci_check_node_mode_closure.sh`); (iii) if it
    needs initial state, obtain it via the SINGLE `bootstrap_initial_state` authority (CN-NODE-01); (iv) if it
    forges, obtain consensus inputs ONLY via the recovered `SeedEpochConsensusInputs` →
    `PoolDistrView::from_seed_epoch_consensus_inputs` surface (CN-CINPUT-03 / DC-CINPUT-02b).
11. **New live-run step (N-F-D / N-F-E rule):** (i) add the variant to the closed GREEN `LoopStep` enum + a
    content-blind planner input + a total `plan_loop_step` arm — the planner observes a `SlotNo` ONLY in a
    dedicated pure guard (`ci_check_loop_planner_closed.sh`); (ii) add a fenced RED `run_relay_loop` branch
    that advances the tip ONLY via `run_node_sync` and (if it forges) reaches EXACTLY ONE
    `forge_one_from_recovered`, serving/admitting/gossiping nothing (`ci_check_node_run_loop_containment.sh`);
    (iii) take any wall-clock observation ONLY through the RED `Clock` seam (DC-NODE-03); (iv) make the step
    opt-in via a closed activation struct (`ForgeActivation`-style) — `None` MUST reproduce the prior relay
    behavior.
12. **New operator-material ingress (N-F-F rule):** (i) classify the *decision* as a PURE GREEN function of CLI
    flag PRESENCE over a closed enum, binding every combination by name, the error carrying only static
    flag-name strings (`ci_check_forge_intent_closed.sh`); (ii) ingest the material at a SINGLE named RED site
    that REUSES the existing loaders (no reimpl) in a RED-parse → BLUE-structural-validate → canonical-type
    pipeline — NO new BLUE authority, NO plugin/trait seam, NO second forge/bootstrap codepath; (iii) confine
    key custody to `ProducerShell` — the holder struct is NOT `Debug`/`Serialize`
    (`ci_check_operator_forge_no_secret_leak.sh`); (iv) derive any identity in ONE named place; (v) make
    activation opt-in over flag presence; (vi) reuse the SAME single recovered `BootstrapState` (CN-NODE-01);
    (vii) leave the prior forge-containment gate SEMANTICALLY UNCHANGED. CN-NODE-03.
13. **New forge-fidelity / fail-closed boundary (N-F-G-A rule):** (i) if it sources a forge constant from an
    oracle preimage, parse it at a single GREEN site with a closed error vocabulary, **no float path** (exact
    `Rational` or fail closed), and carry the preimage **OUTSIDE** the frozen bundle fingerprint, hash-binding
    it to a value the fingerprint already commits to (`ci_check_recovered_ledger_pparams_sourced.sh`); (ii)
    install the sourced constant at seed/import — **never a `::default()`** (CE-G-A-2a); (iii) if it ingests
    operator config, use the REAL parsers (`ci_check_node_forge_real_cli_ingress.sh`); (iv) if it is a
    fail-closed boundary on the forge path, make it an *error* (never a saturation or a third variant) and place
    it INSIDE the existing forge fence; (v) derive any epoch/era decision via the BLUE `EraSchedule::locate` and
    drive NO nonce promotion (DC-EPOCH-03, `ci_check_node_forge_single_epoch_fail_closed.sh`); (vi) any committed
    reference fixture is **evidence-only** (`ci_check_genesis_consistency_fixture_present.sh`).
14. **New self-accept→serve handoff (N-F-G-B rule):** (i) move a forged block to the serve task ONLY through a
    GREEN constructor-fenced carrier whose SOLE constructor takes a BLUE self-accepted `AcceptedBlock` (no
    raw-bytes / artifact / event / flag / verdict ctor); (ii) surface the ORIGINAL BLUE token (never re-derived
    from `artifact.bytes`, CN-FORGE-01); (iii) the relay-loop body forwards a TYPED channel send ONLY — no
    `ServedChainHandle` / `push_atomic` / `served_chain_admit` in the loop body; (iv) served-chain mutation lives
    in a sibling task via the single `push_atomic` fed ONLY by `into_accepted()`; (v) the handoff channel is typed
    `UnboundedSender<SelfAcceptedHandoff>` (never raw bytes / artifact / flag). DC-NODE-06 /
    `ci_check_served_chain_handoff_fence.sh`.
15. **New live `--mode node` feed (N-F-G-C rule):** (i) REUSE the closed `ade_runtime::admission::{dial_for_admission,
    run_admission_wire_pump}` VERBATIM (no reimpl, no new wire authority); (ii) FILL the closed
    `NodeBlockSource::WirePump` arm via `from_wire_pump` — NEVER add a `NodeBlockSource` variant / plugin point /
    source-selection trait; (iii) bridge via a BOUNDED `mpsc`; (iv) advance the durable tip ONLY via `run_node_sync
    → pump_block` (no second tip-advance; `run_node_sync` UNMODIFIED); (v) log-and-drop a dial/parse failure (never
    fatal / fabricated / a tip graft); (vi) leave `ci_check_node_run_loop_containment.sh` byte-unchanged. The
    served-chain handoff fence (`ci_check_served_chain_handoff_fence.sh`) may be BROADENED (net tightening), never
    relaxed.
16. **New operator-pass evidence I/O (N-F-G-C rule):** (i) it is RED file I/O over the UNCHANGED GREEN `correlate`
    — `correlate` stays the SOLE `Ba02Manifest` constructor; (ii) a manifest-writer accepts ONLY a `Ba02Manifest`
    (no path emitting one from `NoEvidence` / raw operator input); (iii) a missing/unreadable file fails closed
    (`io::Error`), never a synthesized acceptance; (iv) NO self-evidence acceptance source; (v) NO committed
    synthetic manifest — a committed-manifest schema gate is vacuous-until-committed + sha256-binds its fixture
    (`ci_check_ba02_evidence_manifest_schema.sh`, RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01).
17. **New peer-driven memory bound (N-F-G-E rule):** (i) bound the surface with a **CLOSED LITERAL constant** —
    NEVER a tunable wired to CLI / env / config (`ci_check_live_feed_memory_bounds.sh` guard 3; a future slice may
    only *tighten* it); (ii) the over-cap behavior MUST **fail closed** — drop-and-disconnect for an inbound
    surface (the reassembly cap → the additive closed `SessionError::ReassemblyBufferOverflow` → drop the peer) or
    back-pressure an existing bounded channel (the lookahead cap → the bounded `mpsc`) — NEVER a silent truncation,
    partial decode, fabricated block, or tip graft; (iii) place the bound **BEFORE** the BLUE `ade_codec` decode
    path / before any authoritative apply; (iv) if it signals via a closed enum, extend it **additively** with NO
    wildcard and handle the new variant in every exhaustive consumer; (v) leave the verdict-decoupled
    `NodeBlockSource` contract, the relay-loop containment gate, and the served-chain handoff fence
    byte-unchanged. DC-LIVEMEM-01.
18. **New wire-grammar sum with a position-aware coupling (N-F-G-J rule):** (i) model the wire field as a CLOSED
    BLUE sum (not `#[non_exhaustive]`); (ii) decode/encode it **POSITION-BLIND** in EXACTLY ONE `ade_codec`
    authority — a pure function of the CBOR token, NEVER of a structural index like `block_number`
    (`ci_check_prevhash_single_wire_authority.sh`, CN-WIRE-09); (iii) if a field's grammar (e.g. `null`) is
    scoped to one envelope, it MUST NOT leak into a sibling codec (`Point::Origin` stays `array(0)`); (iv) put any
    POSITION-AWARE coupling in a SEPARATE single BLUE authority (a NEW `block_validity` submodule like
    `header_position::check_header_position`), called from `decode_block` BEFORE the header authority — **never in
    `ade_codec`, never duplicated**; (v) surface a failure as an additive closed `BlockValidityError` variant that
    folds into an EXISTING coarse `BlockRejectClass` (no new reject class); (vi) delete any all-zero / fingerprint
    stand-in for the absent value at its source — it is structurally the wire-grammar's absent token. CN-WIRE-09 /
    DC-NODE-08.
19. **New cold-start forge reachability (N-F-G-J rule):** (i) derive `(block_number, prev_hash)` via ONE GREEN
    convention (`forge_header_position`): both-`None` tip => `(0, Genesis)`, `Some` => `(last+1, Block)`, a
    `Some`-without-height edge **fails closed** (never `.unwrap_or`); (ii) gate the cold-start with a GREEN
    permission predicate (`may_cold_start_forge`) keyed on the recovered lineage + forge-intent + a forge-eligible
    feed — the lineage gates PERMISSION, NOT the prev_hash bytes (which are structurally null); (iii) reuse the
    SAME `run_real_forge → self_accept → SelfAcceptedHandoff` path (no second forge codepath); (iv) hold NO
    `ChainDb` handle — advance no durable tip (durability is N-U); (v) scope it to the hermetic execution (no
    `genesis_forged` latch); (vi) keep the eligibility signal general (never a private-only / C1-only flag); (vii)
    any scheduling-event vocabulary is CLOSED + EMIT-ONLY (the planner never names it,
    `ci_check_node_sched_events_emit_only.sh`, CN-NODE-04). DC-NODE-08
    (`ci_check_genesis_successor_reachability.sh`).
20. **New real-cardano-node wire-compat surface (N-F-G-L / G-M / G-O rule):** (i) if the real node uses a wire
    shape Ade did not accept, model it as a CLOSED BLUE sum/encoder and pin it byte-identically against a
    committed cardano-node fixture — e.g. the two-form `ArrayHead = Definite(u64) / Indefinite` for FindIntersect
    (`ci_check_chainsync_findintersect_compat.sh`, CN-WIRE-11), or the single per-version `encode_n2n_version_params`
    `versionData` encoder (`ci_check_n2n_handshake_versiondata_authority.sh`, CN-WIRE-10); (ii) **SCOPE** a widened
    grammar to the one mini-protocol that needs it — the indefinite-length acceptance is FindIntersect-only, NOT a
    general decoder, with no catch-all; (iii) a reply that matches the real node (e.g. `Origin →
    IntersectFound[Origin]`) MUST NOT widen the served chain beyond what the protocol grants; (iv) the closed
    `N2N_SUPPORTED` version SET MUST NOT be widened and no runtime negotiation of meaning may be introduced — a
    successful handshake is a wire-layer event, NOT peer acceptance (`RO-LIVE-01` stays `partial`); (v) if a peer
    payload is tag-24-wrapped, strip it via the single `ade_codec::unwrap_tag24` (CN-WIRE-08) authority BEFORE the
    BLUE decode — NEVER a second / hand-rolled tag-24 parse (`ci_check_feed_tag24_unwrap.sh`, CN-WIRE-12).
21. **New recovered-record field for the forge (N-F-G-N rule):** (i) add the field ADDITIVELY to the closed BLUE
    record behind a `*_SCHEMA_VERSION` BUMP — a prior-version buffer MUST decode as `UnknownVersion`, **NEVER a
    default-to-zero / silent fill** (`ci_check_warmstart_eta0_overlay.sh`, T-REC-04 / DC-CINPUT-03); (ii) the GREEN
    merge carries the recovered value verbatim (never default); (iii) the WarmStart composition OVERLAYS the
    recovered value onto the forge input (e.g. `chain_dep.epoch_nonce` → the VRF input `praos_vrf_input(slot,
    epoch_nonce)`) — it supplies the recovered value to the forge, it does NOT make the recovered sidecar a runtime
    authority for the corresponding BLUE state transition (which stays undriven on the forge path, DC-EPOCH-03);
    (iv) keep the record a single closed canonical type with the SOLE codec (no `Default`, no `#[non_exhaustive]`,
    `BTreeMap`-ordered) — the field is additive ONLY behind the version gate, NOT a new TYPE.

### CI gates that enforce the boundary (142 total; the N-AE.F / N-AE / N-AC / N-AB / N-AA / N-U / N-F-G-K…G-R / N-F-G-J / N-F-G-D / N-F-G-E / N-F-G-C / N-F-G-B / N-F-G-A / N-F-F / N-F-D-E / N-F-C / N-F-A / N-Z / N-Y / producer / network set)

| Script | Enforces | Cluster |
|---|---|---|
| `ci_check_receive_idempotency.sh` *(NEW N-AE.F)* | **DC-NODE-16** — receive idempotency at the durable-admit chokepoint: in RED `ade_runtime::forward_sync::pump::pump_block`, immediately after `decode_block` and BEFORE the BLUE chokepoint reducer, the no-op is **HASH-keyed** (`get_block_by_hash(&decoded.block_hash)`, never slot-only); the `return Ok(None)` is **gated by that hit** (a byte-identical already-stored block — same slot, same hash — is skipped: no reducer step, no WAL append, no tip change, replay-equivalent); the slot is cross-checked (`stored.slot == decoded.header_input.slot`); a DIFFERENT block (different hash) at/before the last-applied slot returns `None`, reaches the UNCHANGED BLUE header authority, and fails closed (`SlotBeforeLastApplied`). **Non-vacuous** (the gate must find the hash-keyed query + the gated early return + the slot cross-check in the production region of `pump.rs`). No fork-choice, no skip-past (`DC-CONS-03` untouched); `pump_block` stays the sole durable tip authority (`DC-NODE-12`). | N-AE.F |
| `ci_check_forge_followed_tip_admission.sh` *(NEW N-AE A)* | **DC-NODE-15 + DC-CONS-24** — the `--mode node` `ForgeTick` `selected_tip` has **NO `recovered.tip` fallback** as a forge base; a forge fires only when the durable servable tip equals the followed peer tip (`hash` AND `block_no`), else it returns the typed `ForgeRefused::NotCaughtUp { local_servable_tip, followed_peer_tip, reason }` (a structured value, NOT a log line); the forged successor's `prev_hash` byte-equals the followed peer tip hash AND `block_no == followed_tip.block_no + 1` (parent identity is the canonical hash, never inferred from block number); and the followed-peer-tip signal NEVER reaches `select_best_chain` / `chain_selector` (admissibility-only, never a chain selector). The forge-admissibility sibling of `ci_check_node_forge_single_epoch_fail_closed.sh` (DC-EPOCH-03). | N-AE |
| `ci_check_recovered_anchor_intersectable.sh` *(NEW N-AE B)* | **DC-NODE-14 (anchor/parent clause)** — the recovered/forged parent is peer-intersectable via a **FindIntersect-ONLY**, **proof-gated** projection in `ChainDbServedSource::intersect` (`ade_runtime::network::served_chain_projection`): it advertises the `prev_hash` of the EARLIEST servable `StoredBlock` (the private bounded helper `earliest_servable_block_prev_hash`, reads exactly one block — DC-SERVEMEM-01) as an intersect point IFF a real servable successor exists; it serves **NO bytes** for that point (`get_block_by_hash` / `serve_range` / BlockFetch refuse structurally — no synthetic `StoredBlock`, no synthetic CBOR); recover-only (no `StoredBlock`) and `PrevHash::Genesis` fail closed (no projection). Option A (materialize anchor bytes) explicitly NOT taken. The serve-side completion of the DC-NODE-13 projection (`ci_check_served_chain_projection.sh`). | N-AE |
| `ci_check_recover_follow_wal_lineage.sh` *(NEW N-AE C)* | **DC-WAL-02 + T-REC-05 (strengthened)** — both `node_lifecycle` live `ForwardSyncState::new` prior-fp seeds are `fingerprint(&state.ledger)` (the recovered ledger tip being extended), NEVER `Hash32([0u8;32])` / zero / `default()` — so the first followed `AdmitBlock` chains from the WAL-tail post_fp and a recover→followed store warm-starts replay-equivalently (it was failing `ChainBreak`, exit 42); WAL `verify_chain` / `replay_from_anchor` carry NO new accept-break/skip path (the fix seeds the chain correctly, it does NOT loosen recovery). | N-AE |
| `ci_check_kes_evolution_before_sign.sh` *(NEW N-AC S1)* | **DC-CRYPTO-10** — the forge's SINGLE real KES sign (`ade_node::produce_mode::run_real_forge_inner` step 3) uses the EVOLVING `kes_sign_header_advancing` (NOT the raw `kes_sign_header` / `kes_sign_at`); `kes_sign_header_advancing` (`producer_shell`) calls `kes_advance_to(period)` VERBATIM BEFORE `kes_sign_header` (no `period + 1` / `period - 1` mutation); `kes_advance_to` → `kes_update` retains BOTH fail-closed guards (`EvolutionBackwards` on a backwards period / `EvolutionExhausted` beyond the key lifetime); and signing stays in RED `ade_runtime` (the standing `ci_check_no_signing_in_blue.sh` is the BLUE `T-KEY-01` fence — no `kes_sign` / `SigningKey` in BLUE). **Non-vacuous** (pre-S1 the forge used the raw `.kes_sign_header(`). The evolve-before-sign companion to `ci_check_unsigned_header_preimage_single_source.sh` (CN-KES-HEADER-01). | N-AC |
| `ci_check_outbound_segmentation.sh` *(NEW N-AB S1)* | **CN-SESS-05** — the GREEN `ade_network::session::core::handle_outbound` SEGMENTS an outbound mini-protocol payload `> MAX_PAYLOAD` into ordered `<= MAX_PAYLOAD` frames via the SINGLE `encode_inner_frame` → `mux::frame::encode_frame` per-frame authority — (a) NO second / alternate mux frame encoder in `session`; (b) `encode_inner_frame` still has its strict `MAX_PAYLOAD` guard; (c) `handle_outbound` owns segmentation (references `MAX_PAYLOAD` + `MAX_OUTBOUND_PAYLOAD_BYTES` + chunks); (d) `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` is a FIXED literal, NOT runtime-configurable (no CLI / env / config read, no unbounded mode); every segment keeps the same mini-protocol id + mode + the single captured timestamp (no per-segment clock); segmentation never truncates / drops / reorders; a payload above the fixed bound fails closed via the EXISTING `SessionError::OutboundPayloadTooLarge` (reused, no new variant). The outbound inverse of the `CN-SESS-04` inbound reassembly fenced by `ci_check_live_feed_memory_bounds.sh` (DC-LIVEMEM-01). **Non-vacuous** (pre-S1 HEAD had 0 `chunks(MAX_PAYLOAD)` + no `MAX_OUTBOUND_PAYLOAD_BYTES`). | N-AB |
| `ci_check_serve_range_bounded.sh` *(NEW N-AA S2)* | **DC-SERVEMEM-01** — the `--mode node` durable-chain serve projection (`ChainDbServedSource`, `ade_runtime::network::served_chain_projection`) reads each peer BlockFetch range via the bounded **hash-free** ChainDb primitives `range_bytes_capped` + `last_block_bytes` (NEVER the unbounded `iter_from_slot` / O(N) `chaindb.tip()`), does NO `SLOT_BY_HASH` / full-index scan, caps each request at the FIXED non-configurable `MAX_SERVE_RANGE_BLOCKS = 256` literal (no CLI / env / config read, no "unbounded" path), and derives each block's hash via the single BLUE `decode_block` / `block_header_bytes` (NO second hash authority, NO `SLOT_BY_HASH` on the serve path); an oversized / inverted / undecodable range fails closed via `ServeRangeOutcome` → wire `NoBlocks`. **Non-vacuous** (pre-S2 HEAD had 2 `iter_from_slot` + 1 `chaindb.tip()` serve calls its Guards 2/3 fire on; S2 has 0). The serve-side analog of `ci_check_live_feed_memory_bounds.sh` (DC-LIVEMEM-01). | N-AA |
| `ci_check_forged_durable_admit_via_pump.sh` *(NEW N-U S1)* | **DC-NODE-12 + DC-CONS-23 + DC-WAL-04** — the relay-loop `ForgeTick` arm routes the self-accepted forged block through the fenced `node_sync::admit_forged_block_durably` into the SINGLE `forward_sync::pump_block` durable chokepoint (extend-only `admit_via_block_validity` → StoreBlockBytes → AppendWal → AdvanceTip, durable-before-tip); the forge gains NO direct tip-advance path, NO `NodeBlockSource` variant, NO new `WalEntry` variant (reuses `WalEntry::AdmitBlock`); `accepted.into_bytes()` is byte-identical to the self-accept input (no re-encode, I-10); a stale-tip forge fails closed via header-position / `prior_fp`, never an own-block override. `pump_block` stays the sole durable tip authority. | N-U |
| `ci_check_served_chain_projection.sh` *(NEW N-U S3)* | **DC-NODE-13** — the `--mode node` served view is a READ-ONLY projection of the durable ChainDb: `ChainDbServedSource` (`ade_runtime::network::served_chain_projection`) implements the closed serve seams `ServedHeaderLookup` / `ServedRangeLookup` over `iter_from_slot` / `get_block_by_hash` / `tip`, selected through the closed `ServedChainSource::DurableChainDb` read by the SINGLE `dispatch_server_frame_event_to_outbound` (DC-NODE-07 preserved — exactly one serve dispatch); serves `stored.bytes` verbatim (no re-encode — DC-CONS-17), reuses the single `block_header_bytes` (DC-CONS-18) + `decode_block` (NO parallel splitter, NO `AcceptedBlock` reconstruction); a follower fetches coherent durable history A→B (never B without A) and serving survives restart. Supersedes the retired G-R `ci_check_served_chain_stability.sh` (DC-NODE-11 mechanism). | N-U |
| `ci_check_chainsync_findintersect_compat.sh` *(NEW N-F-G-M)* | **CN-WIRE-11** — the ChainSync `FindIntersect` points-list array-head is the closed sum `ArrayHead = Definite(u64) / Indefinite` decoded by the scoped `decode_array_head_two_form` (accepts the real-node indefinite-length list; SCOPED to FindIntersect — NOT a general definite/indefinite decoder; no catch-all); the `chain_sync::server` answers `Origin → IntersectFound[Origin]` (the universal common ancestor — does NOT widen the served chain). Pinned against captured cardano-node 11.0.1 FindIntersect/IntersectFound fixtures. | N-F-G-M |
| `ci_check_n2n_handshake_versiondata_authority.sh` *(NEW N-F-G-L)* | **CN-WIRE-10** — `encode_n2n_version_params` is the SINGLE per-version N2N `versionData` encoder over the unchanged closed `N2N_SUPPORTED` version SET (V11..=V15 → the 4-field `[networkMagic, diffusionMode, peerSharing, query]`; V16+ → the extended shape); pinned against the real-node handshake fixture. NO version-set widening, NO runtime negotiation. | N-F-G-L |
| `ci_check_warmstart_eta0_overlay.sh` *(NEW N-F-G-N)* | **T-REC-04 + DC-CINPUT-03** — the seed-epoch sidecar carries the recovered `epoch_nonce` (eta0) behind `SEED_CINPUT_SCHEMA_VERSION 1 → 2` (a v1 sidecar decodes as `UnknownVersion`, never zero-eta0); WarmStart overlays it onto `chain_dep.epoch_nonce` so the forge VRF input is `praos_vrf_input(slot, epoch_nonce)` (never a defaulted/zero eta0). | N-F-G-N |
| `ci_check_feed_tag24_unwrap.sh` *(NEW N-F-G-O)* | **CN-WIRE-12** — the feed-side WirePump (`ade_runtime::admission::wire_pump`) strips the peer's BlockFetch `MsgBlock` tag-24 envelope via the single `ade_codec::unwrap_tag24` (CN-WIRE-08) authority BEFORE the BLUE decode, and adds NO hand-rolled / parallel tag-24 parse. | N-F-G-O |
| `ci_check_feed_leader_threshold_view.sh` *(NEW N-F-G-P)* | **DC-CINPUT-04** — the feed header-validation view (leader-threshold stake distribution for header Step 5 + ledger Step 7) is derived from the recovered consensus surface (`PoolDistrView::from_seed_epoch_consensus_inputs`), never a defaulted / empty distribution; the verdict stays BLUE. | N-F-G-P |
| `ci_check_forge_successor_evolved_spine.sh` *(NEW N-F-G-Q)* | **DC-NODE-10** — the successor forge position is sourced from the evolved admitted spine state `(last_admitted_block_no + 1, PrevHash::Block(last_admitted_hash))`, NOT the stale recovered tip; reuses the single `forge_header_position` convention; advances no durable tip (no `ChainDb` handle); no RO-LIVE flip. | N-F-G-Q |
| ~~`ci_check_served_chain_stability.sh`~~ **RETIRED (N-U S3 — superseded by `ci_check_served_chain_projection.sh`)** *(was NEW N-F-G-R)* | **DC-NODE-11 (mechanism superseded)** — the G-R monotone serve gate `node_lifecycle::serve_gate_admits` is RETIRED: N-U S3 supersedes it with serve-as-projection (the durable chain is extend-only, so it holds exactly one block 0 by construction — DC-CONS-23 — and a follower fetches coherent history A→B more strongly than the in-memory predicate, surviving restart — DC-NODE-13). The DC-NODE-11 INVARIANT (*a follower sees a stable, coherent served block 0*) is preserved + strengthened; its `ci_script` migrated to `ci_check_served_chain_projection.sh`. NOT among the 135 active gates. | ~~N-F-G-R~~ → N-U |
| `ci_check_node_serve_lifetime.sh` *(NEW N-F-G-K)* | **DC-NODE-09** — the `--mode node` serve listener's lifetime (`run_node_serve_task`, spawned OUTSIDE `run_relay_loop`) is independent of the feed source's end — it stays up serving past feed-end so a follower mid-fetch keeps fetching; read-only over the `ServedChainView`, admits nothing, advances no durable tip, makes no peer-acceptance claim; `run_relay_loop` containment byte-unchanged. | N-F-G-K |
| `ci_check_prevhash_single_wire_authority.sh` *(NEW N-F-G-J S2)* | **CN-WIRE-09** — the header `prev_hash` is the closed sum `PrevHash = Genesis / Block(Hash32)` ($hash32/null); the SINGLE POSITION-BLIND BLUE codec authority is `ade_codec::shelley::block::decode_prev_hash` (decodes the CBOR token, NEVER `block_number`); the `null` grammar is scoped to header_body ONLY — it MUST NOT leak into the chain-sync/block-fetch `Point`/`Tip` codec (`Point::Origin` stays `array(0)`); no second wire authority, no all-zero/fingerprint stand-in for the genesis predecessor. | N-F-G-J |
| `ci_check_genesis_successor_reachability.sh` *(NEW N-F-G-J S4)* | **DC-NODE-08** — the both-`None`-tip `LoopStep::ForgeTick` arm derives `(0, PrevHash::Genesis)` via the GREEN `node_sync::forge_header_position` (a `Some`-without-height edge fails closed `NodeForgeError::RecoveredTipMissingBlockNo`, never `.unwrap_or(1)`) and forges block 0 through the SAME `run_real_forge → self_accept → SelfAcceptedHandoff` path; permission gated by the GREEN `node_lifecycle::may_cold_start_forge` (recovered lineage + forge-intent + forge-eligible feed); the forge holds NO `ChainDb` handle (no durable tip); scoped to the hermetic cold-start (no `genesis_forged` latch); the eligibility signal is general (never a private-only / C1-only flag). | N-F-G-J |
| `ci_check_node_sched_events_emit_only.sh` *(NEW N-F-G-J S1)* | **CN-NODE-04** — the GREEN `ade_node::live_log::{sched_event, sched_writer}` `NodeSchedEvent` / `FeedReason` / `ForgeOutcome` are CLOSED (none `#[non_exhaustive]`; no catch-all / `Other`; no stringly field) and EMIT-ONLY — the GREEN run-loop planner must NEVER name the vocabulary; the relay loop emits them around the planner call + the `LoopStep` arms, recording the scheduling decision WITHOUT altering it; allow-list + negative tests. | N-F-G-J |
| `ci_check_node_path_fidelity.sh` *(NEW N-F-G-D S1)* | **CN-REHEARSAL-FIDELITY-01 (clause 1, path fidelity)** — guard (a): the `crates/ade_node/src/cli.rs` argv flag-literal set equals the pinned closed **28-flag** allow-list (G-D adds none; a private-only / venue flag — `--private-net` / `--from-genesis` / `--devnet` / `--rehearsal` — trips it); guard (b): no fn whose name carries BOTH `genesis` and `consensus` (a from-genesis consensus-inputs constructor; line comments stripped first) AND `node_lifecycle.rs` sources consensus inputs via the shared `import_live_consensus_inputs`. The C1 dry-run differs from the preprod pass ONLY in operator INPUTS + the evidence LABEL — never in code. | N-F-G-D |
| `ci_check_rehearsal_manifest_schema.sh` *(NEW N-F-G-D S2; hardened S4)* | **CN-REHEARSAL-FIDELITY-01 (clause 2, evidence non-promotability)** — when a committed `docs/evidence/phase4-n-f-g-d-private-rehearsal-*.toml` is present, verify the closed 12-field schema + `schema_version == 1` + `is_rehearsal = true` + `not_bounty_evidence = true` + a `venue` of `private-testnet*` + `peer_log_file_sha256` == sha256(the committed peer-log fixture). THREE non-promotability barriers: the distinct `docs/evidence/` home; the rehearsal markers; and a fail-closed cross-check that NO rehearsal marker appears in any `.toml` under EITHER bounty home (active `docs/clusters/PHASE4-N-F-G-C/` AND archived `docs/clusters/completed/PHASE4-N-F-G-C/` — EXISTING-homes list built first, fail-closed on grep rc≥2). Vacuously satisfied when none committed (C1 dry-run `blocked_until_operator_c1_net_executed`). The no-synthetic-rehearsal-manifest + no-bounty-home-leak enforcer; flips NO RO-LIVE rule. | N-F-G-D |
| `ci_check_live_feed_memory_bounds.sh` *(NEW N-F-G-E S1)* | **DC-LIVEMEM-01** — both live-feed memory bounds are CLOSED LITERAL constants (`MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` in `session/core.rs`; `MAX_WIRE_PUMP_LOOKAHEAD = 256` in `node_sync.rs`) AND not wired to CLI / env / config (the no-escape-hatch guard 3; line comments stripped first so the doc-comments naming "CLI / env / config" do not self-trip). The reassembly cap fails closed via the additive `SessionError::ReassemblyBufferOverflow` (no wildcard; sole consumer `mux_pump::session_err_to_halt`). | N-F-G-E |
| `ci_check_ba02_evidence_manifest_schema.sh` *(NEW N-F-G-C S2)* | **RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01 (BA-02 manifest schema)** — when a committed `docs/clusters/PHASE4-N-F-G-C/CE-G-C-LIVE_*.toml` is present, verify the closed 8-field schema + `schema_version == 1` + `peer_log_file_sha256` == sha256(the committed peer-log fixture). Vacuously satisfied when none committed. The no-synthetic-manifest enforcer. | N-F-G-C |
| `ci_check_served_chain_handoff_fence.sh` *(N-F-G-B S3; REPOINTED N-U S3 close-correction `60deecf3` — DC-NODE-13)* | **DC-NODE-06 (EVOLVED durable-provenance serve)** — owners `{node_lifecycle.rs, node_sync.rs}`; the `--mode node` spine serve sources ONLY the durable ChainDb projection: (1) `ServedChainSource::DurableChainDb` is present; (2) the serve task takes the durable ChainDb read source (`run_node_serve_task` over `Arc<dyn ChainDb>`); (3) NO retired non-durable serve ingress on the node spine — no `push_atomic(`, no `served_chain_admit(`, no `ServedChainHandle`, no `SelfAcceptedHandoff` handoff channel. **REPOINTED, NOT retired** (CI count stays 135). The G-B *“`SelfAcceptedHandoff` → `push_atomic` accumulator”* handoff it formerly fenced is SUPERSEDED by N-U S3 serve-as-projection (DC-NODE-13); DC-NODE-06's deeper *only-validated-bytes-served* invariant is preserved + strengthened (now survives restart). Complementary to the DC-NODE-13 projection-SHAPE gate `ci_check_served_chain_projection.sh` (this is the serve-PROVENANCE angle). The `--mode produce` path (CN-PROD-04, `ServedChainSource::Snapshot`) is a SEPARATE serve authority — deliberately out of scope; it retains the `SelfAcceptedHandoff` carrier + `ServedChainHandle`. | N-F-G-B / N-U |
| `ci_check_genesis_consistency_fixture_present.sh` *(N-F-G-A S1)* | **CE-G-A-1** — the three S1b fixture files committed + well-formed + Ade-as-leader; NO secret key material. | N-F-G-A |
| `ci_check_recovered_ledger_pparams_sourced.sh` *(N-F-G-A S2a)* | **CE-G-A-2a** — recovered ledger `protocol_params` sourced from the oracle preimage (`require_forge_current_pparams`), never `::default()`; the `protocol_params_json` preimage OUTSIDE the 15-field fingerprint. | N-F-G-A |
| `ci_check_node_forge_real_cli_ingress.sh` *(N-F-G-A S2)* | **CE-G-A-2** — the `--mode node` operator-forge ingress uses the real `parse_opcert_envelope` + `parse_shelley_genesis`; fails closed on a `parse_simple_*` reintroduction. | N-F-G-A |
| `ci_check_node_forge_single_epoch_fail_closed.sh` *(N-F-G-A S4)* | **DC-EPOCH-03 / CE-G-A-4** — `forge_one_from_recovered` calls `forge_epoch_admission` BEFORE `query_leader_schedule`; candidate epoch via `EraSchedule::locate`; NO nonce promotion. | N-F-G-A |
| `ci_check_forge_intent_closed.sh` *(N-F-F)* | **CN-NODE-03 (intent half)** — closed two-variant `ForgeIntent`; `classify_forge_intent` sole entry; partial arm binds by name; `ForgeIntentError` static flag-name strings only. | N-F-F |
| `ci_check_operator_forge_no_secret_leak.sh` *(N-F-F; reuse scope extended N-F-G-A)* | **CN-NODE-03 (custody half) / OP-OPS-04** — `operator_forge` reuses the existing loaders (incl. N-F-G-A real parsers); `OperatorForgeMaterial` not `Debug`/`Serialize`; no private-key byte accessor / serialization / logging. | N-F-F |
| `ci_check_node_binary_uses_single_bootstrap.sh` *(MODIFIED-in-place N-F-F)* | **CN-NODE-01** — `ReceiveState::new` owner allow-list `{node.rs, node_lifecycle.rs}`. | N-F-F |
| `ci_check_loop_planner_closed.sh` *(N-F-D; EXTENDED N-F-E; UNCHANGED N-F-G-A/N-F-G-C)* | **CN-NODE-02 / DC-NODE-05** — the GREEN `run_loop_planner` emits only the closed `LoopStep` set, content-blind; the `SlotNo` ban scoped to `plan_loop_step`; `ForgeTick`/`ForgeSlotStatus` pinned. | N-F-D / N-F-E |
| `ci_check_node_run_loop_containment.sh` *(N-F-D; TIGHTENED N-F-E; UNCHANGED N-F-F/N-F-G-A/N-F-G-B/N-F-G-C; ALLOW-LIST EXTENDED in place N-U S1)* | **CN-NODE-02 / DC-SYNC-02 / DC-NODE-05 / DC-NODE-12** — the relay-loop body advances the tip ONLY via `run_node_sync`; references NO `run_real_forge` / `correlate(` / `Ba02Manifest` / second-bootstrap path; **exactly one** fenced `forge_one_from_recovered` (CE-E-4) with the no-serve tokens forbidden. **N-U S1 EXTENDED the allow-list in place** to admit the one `node_sync::admit_forged_block_durably` driver call (which itself routes through the single `pump_block` durable chokepoint — DC-NODE-12); a net tightening, NO file-count change. N-F-F/N-F-G-A/N-F-G-B/N-F-G-C left it byte-/semantically unchanged. | N-F-D / N-F-E / N-U |
| `ci_check_lifecycle_owner_uses_bootstrap_initial_state.sh` *(N-F-C)* | **CN-NODE-01** — exactly one `PHASE4-N-F-C-LIFECYCLE-OWNER`; FirstRun via `bootstrap_from_mithril_snapshot(` + WarmStart via `bootstrap_initial_state(RequiredFromRecoveredProvenance)`; no parallel/cold init, no fallback, no `recover_node_state(` overclaim. | N-F-C |
| `ci_check_node_sync_via_pump.sh` *(N-F-C)* | **DC-SYNC-01 (driver containment)** — `run_node_sync` advances the tip ONLY via `pump_block(`. (L5 `forge_one_from_recovered` excluded.) | N-F-C |
| `ci_check_ba02_evidence_closed.sh` *(N-F-C; reuse scope incl. `ba02_pass` N-F-G-C)* | **RO-LIVE-06 (BA-02 honesty)** — exactly one `Ba02Manifest` constructor (inside `correlate`); no self-evidence token as an acceptance source; no committed `docs/evidence/*ba02*` manifest. The N-F-G-C `ba02_pass` file I/O is consumed over this UNCHANGED correlator. | N-F-C |
| `ci_check_node_mode_closure.sh` *(N-F-C / N-Q)* | **CN-NODE-MODE-01** — pins the full 5-variant closed `Mode` set with a wildcard-free `main.rs` arm per variant. | N-F-C / N-Q |
| `ci_check_consensus_input_provenance.sh` *(N-F-A; guard (d) N-F-C)* | **CN-CINPUT-02 (populate)** + **CN-CINPUT-03 / DC-CINPUT-02b (consume, guard (d))**. | N-F-A / N-F-C |
| `ci_check_mithril_seed_point_independence.sh` *(N-Z)* | **DC-MITHRIL-02 + CN-MITHRIL-01** — `verify_mithril_binding(` precedes `bootstrap_initial_state(`; the `MintInputs` seed-point RHS never traces to a manifest-origin token. | N-Z |
| `ci_check_forward_sync_chokepoint_only.sh` *(N-Y)* | DC-SYNC-01 — durable-before-tip; `AdmitPlan` is the sole `AdvanceTip` emitter. | N-Y |
| `ci_check_mithril_uses_bootstrap_initial_state.sh` *(N-Y)* | CN-MITHRIL-01 — the Mithril path routes initial state through the single authority; no `*Anchor` trait/plugin seam. | N-Y |
| `ci_check_no_haskell_fingerprint_equality.sh` *(N-Y; scope incl. `genesis_pinning` N-F-G-A)* | DC-COMPAT-01 — the harness compares observable surfaces only, never an internal-ledger-fingerprint-vs-Haskell-hash equality. | N-Y |
| `ci_check_sync_evidence_manifest_schema.sh` *(N-Y)* | RO-SYNC-EVIDENCE-01 — closed sync-evidence manifest schema. | N-Y |
| `ci_check_recovery_contract.sh` *(strengthened N-Y)* | recovery-contract / DC-WAL-* — recovery composes existing authorities; fail-fast on `WalTailFingerprintMismatch`. | N-Y |
| `ci_check_registry_code_locus_exists.sh` *(`5db9aae`, extended `a2af041`)* | Registry↔source coherence — every cited `crates/**.rs` + `ci/**.sh` path must exist on disk. | post-N-Y |
| `ci_check_clock_seam.sh`, `ci_check_orchestrator_core_purity.sh` *(N-K; strengthened N-F-E)* | DC-NODE-03 — `clock.rs` is the SOLE `SystemTime::now()`/`Instant::now()` site in `ade_runtime`; the orchestrator/relay-loop core observes no clock/rand/`HashMap`/float — only a `SlotNo` crosses. (The N-F-G-A `checked_millis_to_slot` is pure, no float, no wall-clock.) | N-K / N-F-E |
| `ci_check_tag24_wire_authority.sh` | CN-WIRE-08 — single tag-24 wrap/unwrap authority. | N-X |
| `ci_check_producer_praos_vrf.sh` | CN-FORGE-04 — single era→leader-VRF-input authority. | N-W |
| `ci_check_leader_check_authority.sh` | CN-FORGE-02 — BLUE leader-check has no LedgerView/RED dep. | N-R-A |
| `ci_check_unsigned_header_preimage_single_source.sh` | CN-KES-HEADER-01 *(strengthened_in PHASE4-N-AC)* / DC-KES-HEADER-01 — single pre-image recipe. **N-AC strengthened CN-KES-HEADER-01**: the real-pre-image KES sign now ALSO covers period evolution (the signed pre-image is at the EVOLVED current period, not a stale minted-at-0 period) — fenced by the companion `ci_check_kes_evolution_before_sign.sh` (DC-CRYPTO-10). | N-S-A / N-AC |
| `ci_check_no_produce_mode_direct_transport_writes.sh` | CN-OUTBOUND-RELAY-01 — bytes only via `OutboundCommand` → `MuxPump`. | N-S-B |
| `ci_check_operator_evidence_manifest_schema.sh` | CN-OPERATOR-EVIDENCE-01 — closed evidence-manifest TOML schema. | N-S-C |
| `ci_check_produce_mode_uses_bootstrap_initial_state.sh` | CN-PROD-03 / N-T — `produce_mode` obtains initial state only via `bootstrap_initial_state`. | N-T |
| `ci_check_forge_decode_round_trip.sh`, `ci_check_no_independent_forge_codepath.sh` | CN-FORGE-03 — single forge codepath. | N-V |
| `ci_check_producer_coordinator_no_secrets.sh` | CN-PROD-02 — GREEN coordinator holds no secrets. | N-Q |

> **Count history:** … **N-F-F added 2** → 112, AND *modified one in place*. **N-F-G-A added 4**
> (`ci_check_genesis_consistency_fixture_present.sh`, `ci_check_recovered_ledger_pparams_sourced.sh`,
> `ci_check_node_forge_real_cli_ingress.sh`, `ci_check_node_forge_single_epoch_fail_closed.sh`) → **116**.
> **N-F-G-B added 1** (`ci_check_served_chain_handoff_fence.sh`) → **117**. **N-F-G-C added 1**
> (`ci_check_ba02_evidence_manifest_schema.sh`) AND *broadened `ci_check_served_chain_handoff_fence.sh` in place*
> (owners + guard-3 allow-list — a net tightening, NOT a new file) → **118**. **N-F-G-E added 1**
> (`ci_check_live_feed_memory_bounds.sh`) → **119**. **N-F-G-D added 2** (`ci_check_node_path_fidelity.sh` +
> `ci_check_rehearsal_manifest_schema.sh`) → **121** (the three containment / handoff / memory fences
> `ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_handoff_fence.sh`,
> `ci_check_live_feed_memory_bounds.sh` are byte-unchanged by G-D). **N-F-G-J added 3**
> (`ci_check_node_sched_events_emit_only.sh` (S1) + `ci_check_prevhash_single_wire_authority.sh` (S2) +
> `ci_check_genesis_successor_reachability.sh` (S4)) → **126** (the relay-loop containment / served-chain handoff /
> live-feed memory fences stay byte-unchanged; `ci_check_rehearsal_manifest_schema.sh` was EXTENDED in place to
> also cover the S5 `phase4-n-f-g-j-genesis-rehearsal-*.toml` home — a net tightening, not a new file).
> **N-F-G-K…G-R added 8** (one gate per cluster): `ci_check_node_serve_lifetime.sh` (G-K, DC-NODE-09) +
> `ci_check_n2n_handshake_versiondata_authority.sh` (G-L, CN-WIRE-10) +
> `ci_check_chainsync_findintersect_compat.sh` (G-M, CN-WIRE-11) + `ci_check_warmstart_eta0_overlay.sh` (G-N,
> T-REC-04 + DC-CINPUT-03) + `ci_check_feed_tag24_unwrap.sh` (G-O, CN-WIRE-12) +
> `ci_check_feed_leader_threshold_view.sh` (G-P, DC-CINPUT-04) + `ci_check_forge_successor_evolved_spine.sh`
> (G-Q, DC-NODE-10) + `ci_check_served_chain_stability.sh` (G-R, DC-NODE-11) → **126 → 134** (the relay-loop
> containment / served-chain handoff / live-feed memory fences stay byte-unchanged; the C1-evidence commit
> BROADENED `ci_check_rehearsal_manifest_schema.sh` in place AGAIN — to also cover the c1 rehearsal manifests —
> a net tightening, NOT a new file, CI count unchanged).
> **N-U added 1 net (134 → 135): +2 NEW** (`ci_check_forged_durable_admit_via_pump.sh` (S1, DC-NODE-12 +
> DC-CONS-23 + DC-WAL-04) + `ci_check_served_chain_projection.sh` (S3, DC-NODE-13)) **− 1 RETIRED**
> (`ci_check_served_chain_stability.sh`, the G-R DC-NODE-11 monotone-serve-gate mechanism superseded by
> serve-as-projection). N-U also EXTENDED `ci_check_node_run_loop_containment.sh` in place (allow-list for the
> one `admit_forged_block_durably` driver call — a net tightening, NO file-count change). The S2 §8-named
> `ci_check_forged_tip_recovery.sh` was **NOT created** — T-REC-05 is test-enforced (honest drift; `ci_script =
> ""`).
> **N-AA added 1 (135 → 136): +1 NEW** `ci_check_serve_range_bounded.sh` (S2, DC-SERVEMEM-01) — fences the
> `--mode node` durable-chain serve to the bounded hash-free primitives (no `iter_from_slot` / `chaindb.tip()`),
> the fixed non-configurable `MAX_SERVE_RANGE_BLOCKS` cap, and `decode_block`-only hash derivation. **0 retired,
> 0 modified-in-place** (the relay-loop containment / served-chain handoff / live-feed memory / serve-projection
> fences stay byte-unchanged). The serve-side analog of N-F-G-E's `ci_check_live_feed_memory_bounds.sh`.
> **N-AB added 1 (136 → 137): +1 NEW** `ci_check_outbound_segmentation.sh` (S1, CN-SESS-05) — fences the GREEN
> `session::core::handle_outbound` outbound segmentation: the single `encode_inner_frame` → `mux::frame::encode_frame`
> authority (no second / parallel frame encoder), the preserved `encode_inner_frame` `MAX_PAYLOAD` guard,
> `handle_outbound` chunk-into-`<= MAX_PAYLOAD`-frames behaviour, and the fixed non-configurable
> `MAX_OUTBOUND_PAYLOAD_BYTES` bound (no CLI / env / config escape, fail-closed via the EXISTING reused
> `SessionError::OutboundPayloadTooLarge`). **0 retired, 0 modified-in-place** (every other gate byte-unchanged;
> GREEN-only, 0 BLUE diff). The outbound inverse of N-F-G-E's `ci_check_live_feed_memory_bounds.sh` (DC-LIVEMEM-01).
> **N-AC added 1 (137 → 138): +1 NEW** `ci_check_kes_evolution_before_sign.sh` (S1, DC-CRYPTO-10) — pins that the
> forge's single real KES sign uses the evolving `kes_sign_header_advancing` (NOT the raw `kes_sign_header` /
> `kes_sign_at`), that it calls `kes_advance_to(period)` VERBATIM before signing (no `period ± N`), and that
> `kes_update` retains both fail-closed guards (`EvolutionBackwards` + `EvolutionExhausted`). **0 retired, 0
> modified-in-place** (every other gate byte-unchanged; RED-only, 0 BLUE diff — the standing
> `ci_check_no_signing_in_blue.sh` remains the BLUE `T-KEY-01` fence). The evolve-before-sign companion to
> `ci_check_unsigned_header_preimage_single_source.sh` (CN-KES-HEADER-01, `strengthened_in PHASE4-N-AC`).
> **N-AE added 3 (138 → 141): +3 NEW** `ci_check_forge_followed_tip_admission.sh` (AE.A, DC-NODE-15 + DC-CONS-24) +
> `ci_check_recovered_anchor_intersectable.sh` (AE.B, DC-NODE-14) + `ci_check_recover_follow_wal_lineage.sh` (AE.C,
> DC-WAL-02 + T-REC-05 strengthened). **0 retired, 0 modified-in-place** in count terms — the three containment
> gates (`ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_projection.sh`,
> `ci_check_loop_planner_closed.sh`) stay green and their allow-lists / expected call graphs may be updated for the
> new gate, but NO containment invariant is relaxed. RED/GREEN-only on the wiring side; the lone BLUE change is the
> additive read-only `DecodedBlock.prev_hash` field (no new canonical type). DC-PROTO-10 (AE.E, the chain-sync
> server FindIntersect cursor fix) is **test-enforced** (regression
> `producer_chain_sync_serve_find_intersect_sets_cursor_then_rolls_forward_past_it`; `ci_script = ""`, honest
> drift) — it mints no CI gate, so the +3 above is the full N-AE CI delta.
> **N-AE.F added 1 (141 → 142): +1 NEW** `ci_check_receive_idempotency.sh` (DC-NODE-16) — fences the receive-idempotency
> chokepoint guard in RED `ade_runtime::forward_sync::pump::pump_block`: the no-op is HASH-keyed
> (`get_block_by_hash(&decoded.block_hash)`, never slot-only), the `return Ok(None)` is gated by that hit and
> precedes the BLUE reducer (`forward_sync_step`), and the slot is cross-checked (`stored.slot ==
> decoded.header_input.slot`). **0 retired, 0 modified-in-place** (every other gate byte-unchanged; RED-only, 0 BLUE
> diff — the standing `ci_check_no_signing_in_blue.sh` / containment / served-projection / bounded-serve fences are
> all byte-unchanged). Unlike DC-PROTO-10 (test-enforced), DC-NODE-16 mints a dedicated gate.
> _(The G-H gates `ci_check_single_serve_dispatch_authority.sh` + `ci_check_serve_listener_magic_aware.sh` are part
> of the 141 total at HEAD but are NOT row-detailed in this table — see the G-H-gap note in the header.)_
> Earlier-cluster gates (N-A..N-P, the N-M-* set, the N-L wire-session set) are present in the 141 total; the full
> list is `ls ci/ci_check_*.sh` (= **141**).

---

## 6. Forbidden Patterns (per color)

- **BLUE:** no clock, rand, raw `HashMap`/`HashSet`/`IndexMap`, float, env access, network/filesystem, async
  runtime, locale-dependent ops, OS-dependent ordering. No signing (`ci_check_no_signing_in_blue.sh`). No
  `#[cfg(feature = …)]` semantic gating. No `PreservedCbor` construction outside `ade_codec`. No re-encode of
  wire bytes when hashing. No second era→leader-VRF-input construction (CN-FORGE-04). No second `wrap_tag24` /
  `unwrap_tag24` definition (CN-WIRE-08). No second bootstrap/storage-init authority (CN-NODE-01 /
  DC-GENESIS-SRC-01); no tautological Mithril binding check (CN-MITHRIL-01); `genesis_initial_state` is
  Conway-only. **(N-F-A) No second `SeedEpochConsensusInputs` encoder/decoder pair (CN-CINPUT-01 — SOLE codec);
  `decode_*` MUST be version-gated, byte-canonical, `BTreeMap`-ordered with no `Default` / `#[non_exhaustive]`.
  `PoolDistrView::from_seed_epoch_consensus_inputs` MUST be a pure field-map.** **(N-F-G-J) BLUE WAS extended by exactly ONE canonical type + ONE new module — the header `prev_hash` is now the
  closed sum `PrevHash = Genesis | Block(Hash32)` (`ade_types::shelley::block`, 456 → 457), decoded/encoded
  POSITION-BLIND by the SINGLE `ade_codec::shelley::block::decode_prev_hash` (NEVER `block_number`); the
  position-AWARE rule lives in the SINGLE NEW BLUE `ade_ledger::block_validity::header_position::check_header_position`
  (a fn + the additive `BlockValidityError::HeaderPositionInvalid` variant — no struct/enum), called by
  `decode_block`, folding into the UNCHANGED `BlockRejectClass::HeaderInvalid`. The `null` grammar MUST stay
  header_body-scoped (out of the `Point`/`Tip` codec); the single wire + single position authority MUST NOT be
  duplicated; no all-zero/fingerprint stand-in for the genesis predecessor (CN-WIRE-09).** **(N-F-F / N-F-G-A / N-F-G-B /
  N-F-G-C) No BLUE crate was modified — operator-key ingress + forge fidelity + the self-accept→serve handoff +
  the live feed + the BA-02 evidence I/O reuse the existing BLUE validators/authorities
  (`Sum6Kes::raw_deserialize_signing_key_kes`, `ProducerShell::init`'s freshness bound, the `ProtocolParameters`
  model, exact `Rational` arithmetic, `EraSchedule::locate`, `served_chain_admit`, the BLUE-minted forged hash)
  verbatim; no new BLUE authority. `ade_ledger::rational::Rational` MUST stay exact-integer (no float arithmetic).** **(N-F-G-K…G-R) BLUE gained exactly ONE canonical type + two additive fns + one
  version-gated field — and nothing else: the closed `ArrayHead = Definite(u64) | Indefinite`
  (`ade_network::codec::primitives`, 457 → 458) decoded by the scoped `decode_array_head_two_form` (FindIntersect-
  only — NOT a general definite/indefinite decoder; CN-WIRE-11); the additive fn `encode_n2n_version_params` (the
  single per-version N2N `versionData` encoder over the UNCHANGED closed `N2N_SUPPORTED` version SET — no
  widening, no runtime negotiation; CN-WIRE-10); and the additive field `SeedEpochConsensusInputs.epoch_nonce`
  behind the version-GATED `SEED_CINPUT_SCHEMA_VERSION 1 → 2` (a v1 sidecar MUST fail closed `UnknownVersion`,
  never zero-eta0; the SOLE codec stays `BTreeMap`-ordered, no `Default` / `#[non_exhaustive]`; T-REC-04 /
  DC-CINPUT-03). NO other BLUE change; `ade_ledger::rational::Rational` stays exact-integer.**
- **GREEN:** no nondeterminism; no participation in authoritative outputs. The `producer::coordinator` MUST NOT
  own/store private signing material; **its closed 9-variant `CoordinatorEvent` MUST stay additively stable —
  the N-F-G-A off-epoch outcome reuses `ForgeNotLeader`, the N-F-G-B token rides a sibling return component, and
  N-F-G-C adds no variant.** `ChainEvolution` (N-T) MUST NEVER mint `AcceptedBlock`. Closed vocabularies
  (`ProducerLogEvent`, `ForgeFailureReason`, `SyncEffect`, observable `BlockVerdict`, `LoopStep` /
  `ForgeSlotStatus`, `ForgeIntent`, `SlotAlignmentError` / `ProtocolParamsParseError` / `ForgeCurrentPParamsError`
  / `ForgeEpochAdmission`, the BA-02 `BA02Outcome` / `PeerAcceptEvent` / `NoEvidenceReason`, the N-F-G-J `NodeSchedEvent` /
  `FeedReason` / `ForgeOutcome`) — no open/wildcard variant. **(N-F-G-J) `live_log::{sched_event, sched_writer}`
  MUST stay EMIT-ONLY — the GREEN `run_loop_planner` MUST NEVER construct, read, or name the `NodeSchedEvent`
  vocabulary; the relay loop records scheduling decisions WITHOUT altering them; no catch-all / `Other` variant,
  no stringly field (CN-NODE-04, `ci_check_node_sched_events_emit_only.sh`). `node_sync::forge_header_position` +
  `node_lifecycle::may_cold_start_forge` MUST keep the cold-start convention pure (both-`None` => `(0, Genesis)`;
  `Some` => `(last+1, Block)`; a `Some`-without-height edge fails closed, never `.unwrap_or`) and gate PERMISSION
  only — never the prev_hash bytes (DC-NODE-08).** `forward_sync::reducer` (DC-SYNC-01): MUST NOT emit `AdvanceTip` before that block's `StoreBlockBytes`
  + `AppendWal`. **(N-F-A) `seed_consensus_merge` MUST fail closed on a pool in exactly one source map — NEVER a
  zero-hash fill.** **(N-F-C / N-F-G-C) `ba02_evidence` is evidence, not authority — it COMPARES already-authoritative
  outputs, MUST read the BLUE-minted forged hash VERBATIM, `correlate` MUST be the SOLE `Ba02Manifest`
  constructor; NO self-evidence acceptance source; NO committed synthetic manifest (RO-LIVE-06). N-F-G-C added the
  RED `ba02_pass` file-I/O wrapper over this UNCHANGED correlator — it constructs no evidence and `write_ba02_manifest`
  accepts ONLY a `Ba02Manifest`.** **(N-F-G-B) `producer::self_accepted_handoff::SelfAcceptedHandoff` MUST have a
  PRIVATE field + a SOLE constructor taking a BLUE `AcceptedBlock` — NO raw-bytes / `ForgedBlockArtifact` /
  `CoordinatorEvent` / flag / verdict constructor; it MUST carry the ORIGINAL token verbatim, never re-validating
  or re-deriving it (CN-FORGE-01 / DC-NODE-06).** **(N-F-D / N-F-E) `run_loop_planner` MUST observe a `SlotNo` ONLY
  in the dedicated `forge_slot_status` guard, emit only the closed `LoopStep` set, decide no authority.** **(N-F-F)
  `forge_intent` MUST observe only flag PRESENCE, emit only the closed 2-variant `ForgeIntent`, bind the partial
  arm by name (CN-NODE-03).** **(N-AB, CN-SESS-05) `ade_network::session::core::handle_outbound` MUST SEGMENT an
  outbound payload over `MAX_PAYLOAD` into ordered `<= MAX_PAYLOAD` frames via the single `encode_inner_frame`
  authority (no second frame encoder), byte-preservingly (no truncate / drop / reorder / partial-success), reusing
  one captured `timestamp` (no per-segment clock), failing closed via the existing `SessionError::OutboundPayloadTooLarge`
  at the FIXED non-configurable `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` (no CLI / env / config escape; the outbound
  inverse of the inbound reassembly bound below).** **(N-F-G-E) `ade_network::session::core` MUST fail closed on a reassembly tail over
  `MAX_REASSEMBLY_TAIL_BYTES` via the additive closed `SessionError::ReassemblyBufferOverflow` (NO wildcard; the
  bound fires BEFORE the BLUE decode path — no silent truncation, no partial decode); the cap MUST be a closed
  literal constant with NO CLI / env / config escape hatch (DC-LIVEMEM-01).** **(N-F-G-A) `consensus_inputs::protocol_params` MUST have NO float path; `require_forge_current_pparams`
  MUST keep the `protocol_params_json` preimage OUTSIDE the 15-field fingerprint + hash-bind before parsing;
  `checked_millis_to_slot` MUST fail closed before-anchor (never saturate); `forge_epoch_admission` MUST derive
  via `EraSchedule::locate` + drive no nonce promotion; `consensus::genesis_pinning` is `#[cfg(test)]` evidence,
  never runtime authority (DC-COMPAT-01).** `harness::sync_diff` (DC-COMPAT-01): MUST NOT compare Ade's internal
  ledger `fingerprint` to a Haskell hash. `lagging` ≠ success; wire success ≠ admission ≠ agreement ≠ peer ACCEPT. **(N-F-G-N) `seed_consensus_merge` MUST carry the recovered `epoch_nonce` (eta0) verbatim into
  the BLUE record's new field — NEVER a default-to-zero fill (DC-CINPUT-03; the BLUE codec's v1→v2 gate backs it).
  (N-F-G-R) `node_lifecycle::serve_gate_admits` MUST stay a pure monotone predicate (`serve_gate_admits(None, 0)`
  admits; a candidate `<=` the highest served height, or an already-served height with a divergent hash, is
  refused) — the served tip never regresses once block 0 is served (DC-NODE-11). (N-F-G-P) the feed
  header-validation view MUST derive its leader-threshold distribution from the recovered consensus surface
  (`PoolDistrView::from_seed_epoch_consensus_inputs`), never a defaulted / empty distribution — the verdict stays
  BLUE (DC-CINPUT-04). (N-F-G-Q) the forge-successor position MUST be sourced from the evolved admitted spine via
  the single `forge_header_position` convention, never the stale recovered tip (DC-NODE-10).**
- **RED:** no direct mutation of BLUE state; no construction of semantic types from raw bytes; no bypassing
  canonical validation. `produce_mode` emits outbound bytes only via `OutboundCommand`. The per-peer outbound
  map is `BTreeMap` (deterministic). Key custody confined to `producer::signing` / `producer_shell`.
  `run_real_forge` (N-W) MUST NOT perform RED-side era dispatch for the leader-VRF alpha. No hand-rolled tag-24
  parse (CN-WIRE-08). `forward_sync::pump` (DC-SYNC-01) MUST refuse to advance the tip before the durability
  writes ack. `mithril_import` MUST perform no semantic decision and route initial state through the single
  `bootstrap_initial_state` authority (CN-MITHRIL-01). `genesis_bootstrap` / `mithril_bootstrap` MUST route
  through the same authority — never a parallel storage-init path (CN-NODE-01 / DC-GENESIS-SRC-01); **(N-Z)**
  mint the anchor `seed_point` from the operator-independent `MithrilSeedPointInputs` ONLY, and run
  `verify_mithril_binding` fail-closed BEFORE `bootstrap_initial_state` (DC-MITHRIL-02). `recovery::restart`
  MUST compose the existing WAL-replay + rollback authorities and fail fast on `WalTailFingerprintMismatch`.
  **(N-F-A) `seed_consensus_provenance::append_seed_epoch_provenance` MUST `blake2b_256` the EXACT A1 bytes the
  composer `put`, only AFTER the durable sidecar put. `bootstrap::restore_seed_epoch_consensus_inputs` MUST fail
  closed on a missing sidecar / hash mismatch / non-canonical decode / binding mismatch / non-byte-identical
  re-encode, and MUST NOT fall back to the forge-time bundle. `produce_mode` MUST pass
  `SeedEpochConsensusSource::NotRequired` (CN-CINPUT-02).** **(N-F-C) `Mode` MUST stay closed with a wildcard-free
  `main.rs` arm per variant (CN-NODE-MODE-01). Exactly one `--mode node` lifecycle owner; both arms route through
  the SINGLE `bootstrap_initial_state` authority (CN-NODE-01). `NodeBlockSource` MUST yield ordered block bytes and
  NOTHING else; `run_node_sync` MUST advance the tip ONLY via `pump_block`. `node_sync::forge_one_from_recovered`
  MUST project leadership ONLY via `PoolDistrView::from_seed_epoch_consensus_inputs` (CN-CINPUT-03 /
  DC-CINPUT-02b).** **(N-F-D/E) Exactly one live-run owner; the relay-loop body MUST advance the tip ONLY via
  `run_node_sync`, reach EXACTLY ONE fenced `forge_one_from_recovered` per `ForgeTick`, advance NO durable tip,
  serve/admit/gossip NOTHING (CN-NODE-02 / DC-SYNC-02 / DC-NODE-05).** **(N-F-F) Operator-material ingress MUST
  land at the SINGLE named `operator_forge` site reusing the existing loaders — NO new BLUE authority, NO plugin
  seam, NO second forge codepath, NO Mithril call, NO second bootstrap; key custody RED-confined to
  `ProducerShell`; the forge-on flip opt-in over flag presence (CN-NODE-03).** **(N-F-G-A) The `--mode node`
  ingress MUST use the REAL parsers; recovered pparams MUST be sourced from the oracle preimage (never
  `::default()`); the forge tick MUST use `checked_millis_to_slot` + `forge_epoch_admission` BEFORE leadership;
  NO nonce promotion (DC-EPOCH-03). The N-F-E forge containment MUST stay SEMANTICALLY UNCHANGED.** **(N-F-G-B)
  The self-accept→serve handoff MUST move a forged block to the serve task ONLY through the constructor-fenced
  `SelfAcceptedHandoff` carrier; `run_real_forge` / `forge_one_from_recovered` MUST surface the ORIGINAL BLUE
  `AcceptedBlock` (`Some` iff `ForgeSucceeded`), never re-derived from `artifact.bytes` (CN-FORGE-01). The
  relay-loop body MUST forward a TYPED `mpsc<SelfAcceptedHandoff>` `tx.send` ONLY — no `ServedChainHandle`, no
  `push_atomic` / `served_chain_admit`, no served-chain mutation. Served-chain mutation MUST happen ONLY in the
  sibling `tokio::spawn` task via the single `ServedChainHandle::push_atomic`, fed ONLY by `into_accepted()`; the
  handoff channel MUST be typed `UnboundedSender<SelfAcceptedHandoff>` (DC-NODE-06 / CN-PROD-04,
  `ci_check_served_chain_handoff_fence.sh`).** **(N-F-G-C) The live `--mode node` feed MUST REUSE the closed
  `ade_runtime::admission::{dial_for_admission, run_admission_wire_pump}` VERBATIM (no reimpl, no new wire
  authority) and FILL the closed `NodeBlockSource::WirePump` arm via `from_wire_pump` — NEVER a new
  `NodeBlockSource` variant / plugin point / source-selection trait, NEVER a second tip-advance (`run_node_sync`
  UNMODIFIED), NEVER a dial/parse-failure that is fatal / fabricated / a tip graft (log-and-drop, C3). The bounded
  `mpsc` cap MUST stay bounded. `ci_check_node_run_loop_containment.sh` MUST stay byte-unchanged; the served-chain
  handoff fence may be BROADENED (net tightening), never relaxed. The RED `ba02_pass` MUST be file I/O ONLY over
  the UNCHANGED GREEN `correlate` — `correlate` the SOLE `Ba02Manifest` ctor; `write_ba02_manifest` accepts ONLY
  a `Ba02Manifest`; a missing/unreadable file fails closed; NO self-evidence acceptance source; NO committed
  synthetic manifest (RO-LIVE-06 / CN-OPERATOR-EVIDENCE-01, `ci_check_ba02_evidence_manifest_schema.sh`).**
  **(N-F-G-E) `ade_node::node_sync::pump_lookahead` MUST stop the opportunistic `try_recv` drain at a CLOSED
  LITERAL `MAX_WIRE_PUMP_LOOKAHEAD` (so the existing bounded `mpsc` back-pressures) — NO unbounded drain, NO
  CLI/env/config override (DC-LIVEMEM-01); `ade_runtime::network::mux_pump::session_err_to_halt` MUST handle the
  new `SessionError::ReassemblyBufferOverflow` variant in its exhaustive (no-wildcard) `match` and drop the peer.
  The closed verdict-decoupled `NodeBlockSource` contract, the relay-loop containment gate, and the served-chain
  handoff fence MUST stay byte-unchanged.** **(N-F-G-O) the feed-side WirePump (`ade_runtime::admission::wire_pump`) MUST strip
  the peer's BlockFetch `MsgBlock` tag-24 envelope via the single `ade_codec::unwrap_tag24` (CN-WIRE-08) authority
  BEFORE the BLUE decode — NEVER a second / hand-rolled tag-24 parse; a non-tag-24 / malformed payload fails
  closed (CN-WIRE-12, `ci_check_feed_tag24_unwrap.sh`). (N-F-G-K) the `--mode node` serve listener
  (`run_node_serve_task`, spawned OUTSIDE `run_relay_loop`) MUST have a lifetime independent of the feed source's
  end (stays up serving past feed-end) and stay read-only over the `ServedChainView` — admitting nothing,
  advancing no durable tip (DC-NODE-09). (N-F-G-R) the served tip MUST be monotone via
  `node_lifecycle::serve_gate_admits` once block 0 is served — no regression, no re-serve of an already-served
  height with a divergent hash (DC-NODE-11). (N-F-G-Q) the forge-successor position MUST derive `(block_number,
  prev_hash)` from the evolved admitted spine `(last_admitted_block_no + 1, PrevHash::Block(last_admitted_hash))`,
  never the stale recovered tip; the forge MUST hold NO `ChainDb` handle (no durable tip — N-U); NO new
  `NodeBlockSource` variant, NO second forge codepath, NO RO-LIVE flip (DC-NODE-10). (N-F-G-N) WarmStart MUST
  overlay the recovered `epoch_nonce` (eta0) from the v2 sidecar onto `chain_dep.epoch_nonce` (the forge VRF input
  is `praos_vrf_input(slot, epoch_nonce)`); a v1 sidecar MUST fail closed `UnknownVersion`, NEVER default eta0 to
  zero (T-REC-04 / DC-CINPUT-03). The whole G-K…G-R span MUST leave `run_relay_loop`'s containment + the
  served-chain handoff fence byte-unchanged.**
- **(N-AA, DC-SERVEMEM-01 — RED serve path) the peer-driven `--mode node` serve MUST be bounded and fail
  closed; it MUST NOT amplify peer-driven storage/CPU.** The `--mode node` durable-chain serve
  (`ChainDbServedSource`) MUST read each peer BlockFetch range via the bounded **hash-free** ChainDb
  primitives `range_bytes_capped` + `last_block_bytes` — **NEVER** the unbounded `iter_from_slot` / O(N)
  `chaindb.tip()` on the peer-driven serve path; MUST do **NO** `SLOT_BY_HASH` / full-index scan and **NO**
  per-request full-chain materialization (no unbounded `Vec` of peer-requested blocks); MUST cap each request
  at the **FIXED, non-configurable** `MAX_SERVE_RANGE_BLOCKS = 256` literal — **NO CLI / env / config escape
  hatch, no "unbounded" mode**; MUST derive each block's hash via the single BLUE `decode_block` /
  `block_header_bytes` (**NO second / duplicate block-hash authority, NO `SLOT_BY_HASH` on the serve path**);
  and MUST fail closed on an oversized (`CapExceeded`) / out-of-chain or inverted `from > to` (`Empty`) /
  undecodable in-range (`ReadError`) range, with **every non-`Served` `ServeRangeOutcome` mapping to the wire
  `NoBlocks`** (the serve MUST NOT emit a block it cannot authenticate via `decode_block`). The single serve
  `dispatch_server_frame_event_to_outbound` MUST stay the sole dispatch (**DC-NODE-07** — `ServeRangeOutcome`
  is an internal serve-read outcome, not a dispatch); within-cap serving MUST stay byte-identical
  (**DC-CONS-17**); and the trusted recovery / rollback `iter_from_slot` / `tip` internals stay doc-fenced /
  unchanged. A later slice may **tighten** the cap, never disable it (`ci_check_serve_range_bounded.sh`).**
- **(N-AB, CN-SESS-05 — GREEN session reducer) outbound segmentation MUST use the single frame encoder, must be
  byte-preserving, and must fail closed at the fixed bound.** The GREEN `ade_network::session::core::handle_outbound`
  MUST segment an outbound mini-protocol payload `> MAX_PAYLOAD` into ordered `<= MAX_PAYLOAD` frames via the SAME
  single `encode_inner_frame` → `mux::frame::encode_frame` per-frame authority — **NO second / alternate / parallel
  mux frame encoder** in `session`; `encode_inner_frame` MUST keep its strict `MAX_PAYLOAD` guard. It MUST **NOT**
  truncate, drop tail bytes, reorder, or partial-success send (a message either fully segments + emits or fails
  closed); `concat(segment payloads) == payload` exactly (**DC-CONS-17**). Every segment MUST carry the SAME
  mini-protocol id + mode + the **single captured `timestamp` input** — **NO per-segment clock call in GREEN**
  (determinism). A payload above the **FIXED, non-configurable** `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` literal
  (SYMMETRIC with the receive-side `MAX_REASSEMBLY_TAIL_BYTES`, DC-LIVEMEM-01) MUST fail closed with the EXISTING
  reused `SessionError::OutboundPayloadTooLarge` (**NO new closed enum / variant**) — **NO CLI / env / config escape
  hatch, no unbounded / runtime-configurable payload cap**. It MUST make **NO BLUE change** (`mux::frame` reused
  unchanged) and **NO RO-LIVE flip**. A later slice may **tighten** the bound, never disable it
  (`ci_check_outbound_segmentation.sh`).**
- **(N-AC, DC-CRYPTO-10 — RED signing shell) the forge's single real KES sign MUST evolve the operator key to the
  requested period before signing, fail-closed both directions, and never sign at a stale period.** The forge
  (`ade_node::produce_mode::run_real_forge_inner` step 3) MUST call the EVOLVING
  `ProducerShell::kes_sign_header_advancing(period, preimage)` — **NOT** the raw `kes_sign_header` / `kes_sign_at`
  (those `&self` methods REMAIN only for callers that manage periods themselves). `kes_sign_header_advancing` MUST
  call `kes_advance_to(period)` **VERBATIM** before `kes_sign_header` (**NO** `period + 1` / `period - 1` / `± N`
  mutation of the forge-computed period). `kes_advance_to` → `kes_update` (the EXISTING deterministic Sum6KES
  forward evolution — reused, **NOT** re-implemented) MUST stay idempotent at the current period (`while current <
  to`) and MUST retain BOTH fail-closed guards: `period < current` → `Signing(EvolutionBackwards)` (a destroyed
  past period cannot be re-signed — forward-secrecy), `period` beyond the key lifetime / `SUM6_MAX_PERIOD = 63` →
  `Signing(EvolutionExhausted)`. After a successful advance `current == period`, so the sign is at the current
  period (no `KesPeriodNotCurrent`); on a failed advance there is **NO signature** (never a key override). Signing
  MUST stay **RED** (`T-KEY-01` — no `kes_sign` / `SigningKey` in BLUE, `ci_check_no_signing_in_blue.sh`); the BLUE
  Sum6KES algorithm + KES verifier + forge eligibility + wire rules MUST stay UNCHANGED. **NO new BLUE authority /
  canonical type, NO new closed enum / version-gated contract** (the existing `SigningError` variants + branded
  `KesSignature` / `UnsignedHeaderPreImage` are reused), **NO C1-only key path** (the same `--mode node` forge path
  serves C1 and C2), and **NO RO-LIVE flip** (`RO-LIVE-01` stays `partial`). The forge's `kes_period_in_window`
  pre-check MUST be preserved (`ci_check_kes_evolution_before_sign.sh`; `CN-KES-HEADER-01` `strengthened_in +=
  "PHASE4-N-AC"`).**
- **(N-AE.F, DC-NODE-16 — RED durable-admit chokepoint) the receive-idempotency no-op MUST be hash-keyed against
  the durable store, gated before the BLUE reducer, and MUST NOT weaken the fail-closed boundary for a different
  block.** In RED `ade_runtime::forward_sync::pump::pump_block`, the already-have skip MUST query the durable store
  by **HASH** (`get_block_by_hash(&decoded.block_hash)`) — **NEVER** slot alone — and return `Ok(None)` (the
  idempotent no-op) **ONLY** when `Some(stored)` AND `stored.slot == decoded.header_input.slot`; the check MUST run
  AFTER `decode_block` and BEFORE the BLUE chokepoint reducer (`forward_sync_step` / `validate_and_apply_header`).
  The no-op MUST run **NO** reducer step, append **NOTHING** to the WAL, and change **NO** tip (the post-state —
  ledger / chain_dep / ChainDb tip / WAL length — MUST be identical and replay-equivalent; T-REC-05 / DC-WAL-02
  preserved). A **DIFFERENT** block (different hash) at/before the last-applied slot MUST **NOT** be short-circuited
  — it MUST reach the UNCHANGED BLUE header authority and fail closed (`SlotBeforeLastApplied` / `BlockNoOutOfOrder`
  — AE-F-INV-2). The skip MUST **NOT** skip *past* a gap, accept a *better* chain, or select among competing tips
  (no fork-choice — `DC-CONS-03` untouched, AE-F-INV-3). It MUST make **NO BLUE change** (the BLUE authority + the
  reducer inputs stay UNCHANGED — the sketch's BLUE `ReceiveOutcome::AlreadyHave` was NOT taken; the gate is RED
  orchestration over a deterministic durable-store read), introduce **NO** new closed enum / canonical type /
  version-gated contract, and add **NO** second durable tip-advance path (`pump_block` stays the sole durable tip
  authority — DC-NODE-12). **NO RO-LIVE flip** (`ci_check_receive_idempotency.sh`).**

### Project-specific additions (Ade)

- **Receive-idempotency honest scope + boundary (N-AE.F, load-bearing — do NOT soften / do NOT broaden):** N-AE.F
  makes the receive path NO-OP a byte-identical already-applied block re-announced by the peer (the post-CE-A5
  echo: after the relay adopted Ade's forged block 17 it served that block BACK over Ade's follow link, and the
  BLUE header authority rejected `SlotBeforeLastApplied{last=421,attempted=421}`, ending the continuous run with
  exit 43) — and **nothing more**. The discriminator is **HASH equality against the durable ChainDb**, never slot
  alone: `pump_block` queries `get_block_by_hash(&decoded.block_hash)` AFTER `decode_block` and BEFORE the BLUE
  reducer, returns `Ok(None)` ONLY on a same-hash-same-slot hit, and otherwise falls through UNCHANGED. The hard
  boundaries (all mechanically fenced by `ci_check_receive_idempotency.sh`): **(1) NO weakening of the fail-closed
  boundary** — a DIFFERENT block (different hash) at/before the last-applied slot still reaches the UNCHANGED BLUE
  header authority and fails closed `SlotBeforeLastApplied` (AE-F-INV-2). **(2) NO fork-choice / skip-past** — the
  skip is exact-match idempotency, never chain selection, never skipping a gap, never accepting a better chain
  (`DC-CONS-03` untouched, AE-F-INV-3). **(3) NO BLUE change** — the sketch's BLUE `ReceiveOutcome::AlreadyHave`
  variant was NOT taken; `get_block_by_hash` is a DETERMINISTIC durable-store query, so the gate is RED
  orchestration over a deterministic read, with no new reducer input and the BLUE authority untouched (458
  canonical types unchanged). **(4) NO second durable tip-advance path** — `pump_block` stays the sole durable tip
  authority (DC-NODE-12); the no-op advances no tip and appends nothing to the WAL (replay-equivalent — T-REC-05 /
  DC-WAL-02 preserved). **NO RO-LIVE flip** — surviving the echo is a long-running-relay precondition, NOT
  operator-witnessed bounty acceptance (`RO-LIVE-01` stays `partial` / operator-gated). DC-NODE-16 is `tier =
  derived` and `enforced`; no rule strengthened, no rule weakened. _(N-AE.F does not alter any wire / serve /
  forge / containment / bounded-memory surface; all those fences are byte-unchanged.)_
- **Recover→serve continuity + forge-admissibility honest scope + boundary (N-AE, load-bearing — do NOT soften /
  do NOT broaden):** N-AE makes a `--mode node` forge produce a peer-ADOPTABLE successor and tightens the
  serve/chain-sync path so a real relay can FindIntersect at the forged parent and roll forward onto it — and
  **nothing more**. The hard boundaries (from the PHASE4-N-AE cluster doc §11, all mechanically fenced):
  **(1) NO synthetic servable `StoredBlock` bytes** from snapshot / ledger / block-number / hash — hash-critical
  paths require preserved original bytes; the AE.B recovered-anchor projection is FindIntersect-ONLY and serves
  NO bytes (`ci_check_recovered_anchor_intersectable.sh`; Option A — materialize anchor bytes — was explicitly
  NOT taken). **(2) NO `recovered.tip` fallback as a forge base** — the forge waits for `durable_servable_tip ==
  followed_peer_tip` (`ci_check_forge_followed_tip_admission.sh`, DC-NODE-15). **(3) NO parent-hash inference
  from block number** — parent identity is the canonical hash (`block_no == followed_tip.block_no + 1` is an
  ADDITIONAL check, not the parent source; DC-CONS-24). **(4) NO peer-tip signal as a chain selector** — the
  followed-peer-tip signal (`FollowedPeerTipSignal`) may *prevent* a forge (admissibility), NEVER *select* the
  chain; it never reaches `select_best_chain` / `chain_selector` (fenced). **(5) NO fork-choice / multi-producer
  intake** — Gap 1 is an explicitly-deferred SEPARATE future cluster; the BLUE Praos `fork_choice` /
  `select_best_chain` authority (DC-CONS-03) stays the follow / `chain_selector` path, UNTOUCHED. **(6) NO second
  durable tip-advance path** — forged + received blocks still become durable ONLY via `pump_block` (DC-NODE-12).
  **(7) NO wire-grammar change** — DC-PROTO-10 is a server-INTERNAL `last_announced` cursor-state fix; the closed
  `Point` enum + `ChainSyncMessage` set are reused UNCHANGED. **NARROW — the only NEW BLUE symbol is the additive
  read-only `DecodedBlock.prev_hash` field** (the AE.E `chain_sync/server.rs` cursor fix is an existing-field write,
  not a new symbol; no new canonical type; 458 unchanged); the new
  `ForgeRefused` / `NotCaughtUpReason` / `ForgeFollowedTipAdmission` / `NodeForgeOutcome` / `FollowedPeerTipSignal`
  are closed RED/GREEN error/signal sums in `ade_node::node_sync`, NOT canonical types / registries; `T-KEY-01`
  + the FC/IS partition are preserved. **NO RO-LIVE flip** — the CE-A5 manifest (a real cardano-node
  `AddedToCurrentChain` an Ade-forged block, venue c2ae18) is a non-promotable C2-LOCAL rehearsal-venue event;
  only a committed `ba02_evidence::correlate` manifest over the preprod tip advances `RO-LIVE-01` (stays
  `partial` / operator-gated). DC-NODE-15 / DC-CONS-24 / DC-NODE-14 / DC-PROTO-10 are all `tier = derived` and
  `enforced`; DC-EPOCH-03 / CN-CONS-06 / CN-CONS-07 / DC-WAL-02 / DC-NODE-05 / T-REC-05 / DC-CONS-23
  `strengthened_in += "PHASE4-N-AE"` (the 7 the registry actually carries; the AE.B projection tightens the
  DC-NODE-13 serve-projection surface but DC-NODE-13 is cross-referenced, not re-tagged).
- **KES-signing-evolves-the-operator-key honest scope + boundary (N-AC, load-bearing — do NOT soften / do NOT
  broaden):** N-AC makes the RED producer signing shell EVOLVE the operator KES signing key FORWARD to the chain's
  current KES period BEFORE the single real KES sign — and **nothing more**. The new RED
  `ProducerShell::kes_sign_header_advancing` = `kes_advance_to(period)? then kes_sign_header(period, preimage)`,
  where `kes_advance_to` → `kes_update` is the EXISTING deterministic Sum6KES forward evolution (byte-for-byte
  cardano-base; reused, NOT re-implemented; idempotent at the current period; fail-closed `EvolutionBackwards`
  backwards / `EvolutionExhausted` beyond `SUM6_MAX_PERIOD = 63`). This closed the live-readiness gap the item-4
  C1 re-run surfaced: the forge's only real sign called the raw `kes_sign_header(kes_period, …)`, which requires
  `kes.current_period() == requested`, and nothing evolved the minted-at-0 operator key forward — so every
  period-1 leader slot failed `KesPeriodNotCurrent { requested: 1, current: 0 }`. **NARROW — signing stays RED**
  (`T-KEY-01` preserved); **the BLUE Sum6KES algorithm + KES verifier + forge eligibility + wire rules are
  UNCHANGED**; the period is passed **VERBATIM** (no `± N`); the evolution is **fail-closed both directions**
  (backwards / beyond-lifetime → structured error + NO signature, never a key override); **NO new BLUE authority /
  canonical type, NO new closed enum / version-gated contract** (the existing `SigningError` variants + branded
  `KesSignature` / `UnsignedHeaderPreImage` are reused); **NO C1-only key path** (same `--mode node` forge path for
  C1 and C2); and **NO RO-LIVE flip** — a forged + self-accepted period-1 block a real cardano-node DOWNLOADS with
  no KES/parse rejection is a wire-layer / header-acceptance event, NOT operator-witnessed bounty acceptance
  (`RO-LIVE-01` stays `partial` / operator-gated). **Genesis-window / KES-period coupling finding (honest, NOT a
  gap):** `slotsPerKESPeriod = 129600` equals `3k/f = 3·2160/0.05 = 129600`, so KES period 1 begins exactly when
  the Cardano genesis density window closes — a from-genesis C1 rehearsal can never show forge-at-period-1 AND
  follower-adopt simultaneously (period 0 = window open, no evolution exercised; period 1+ = evolution exercised
  but the follower rejects `CandidateTooSparse`, which is genesis-density-window-limited and **KES-INDEPENDENT**,
  not a KES rejection). **Cross-period end-to-end forge→adopt is the C2 dense-tip path** (the real bounty path —
  current preprod tip, no genesis window), NOT a net reset. Two pre-existing fail-closed INFO items are handed to
  C2 (recorded in the registry `evidence_notes`): (1) `kes_advance_to` zeroes the key on a FAILED advance —
  UNREACHABLE via the forge (the `kes_period_in_window` + `kes_period_for_slot` pre-check bounds the period),
  fail-safe; (2) the opcert window upper bound `opcert_start + 63` can diverge from the absolute Sum6KES ceiling 63
  when `opcert_start > 0` (real preprod opcerts) — still fail-closed (`EvolutionExhausted`), but the C2 config must
  derive `kes_max_period` from the absolute ceiling. DC-CRYPTO-10 is `tier = derived`; `CN-KES-HEADER-01`
  `strengthened_in += "PHASE4-N-AC"`. _(N-AC does not alter any wire / serve / live-feed / containment / bounded-memory
  surface; the relay-loop containment, served-chain handoff, and bounded-serve fences are byte-unchanged.)_
- **Serve-side real-node compat + feed decode/view fidelity + forge-successor + monotone serve gate + recovered
  eta0 honest scope + boundary (N-F-G-K … G-R, load-bearing — do NOT soften / do NOT broaden):** the whole
  G-K…G-R span is serve-side real-cardano-node compatibility + live-feed decode/view fidelity + forge-successor
  continuity + a monotone serve gate + an explicit recovered eta0 — and **nothing more**. **G-M** (`CN-WIRE-11`):
  the closed `ArrayHead = Definite(u64) | Indefinite` + `decode_array_head_two_form` widens the ChainSync
  `FindIntersect` array-head grammar to accept the real node's indefinite-length points list — **SCOPED to
  FindIntersect** (NOT a general definite/indefinite decoder; no catch-all); the `Origin → IntersectFound[Origin]`
  reply matches the real node and **does NOT widen the served chain**. **G-L** (`CN-WIRE-10`):
  `encode_n2n_version_params` is the single per-version `versionData` encoder over the **unchanged closed
  `N2N_SUPPORTED` version SET** (no widening, no runtime negotiation); a successful handshake is a wire-layer
  event, **NOT peer acceptance**. **G-O** (`CN-WIRE-12`): the feed-side WirePump strips the BlockFetch tag-24
  envelope via the single `ade_codec::unwrap_tag24` (CN-WIRE-08) authority before the BLUE decode — **NO parallel
  tag-24 parser**. **G-N** (`T-REC-04` + `DC-CINPUT-03`): the recovered eta0 is an additive `epoch_nonce` field
  behind the **version-GATED** `SEED_CINPUT_SCHEMA_VERSION 1 → 2` bump (a v1 sidecar fails closed `UnknownVersion`,
  never zero-eta0); WarmStart overlays it onto `chain_dep.epoch_nonce` so the forge VRF input is
  `praos_vrf_input(slot, epoch_nonce)`, but the recovered sidecar is **NOT** a runtime authority for nonce
  evolution (the BLUE `consensus::nonce` transitions stay undriven on the forge path, DC-EPOCH-03). **G-K /
  G-P / G-Q / G-R** are RED orchestration inside `node_lifecycle` / `node_sync` (serve-lifetime decouple; feed
  header-validation view from the recovered consensus surface; forge-successor position from the evolved admitted
  spine; monotone serve gate) — the forge holds **no `ChainDb` handle** (advances **no durable tip** — the "first
  stable node, block 1+" is in-memory spine continuity, durability is N-U); the serve sibling is **read-only** over
  the `ServedChainView` (admits nothing, makes no peer-acceptance claim). **NO RO-LIVE flip** across the entire
  span (`RO-LIVE-01` stays `partial` / operator-gated); the C1 evidence is a **rehearsal REPRODUCTION**, fenced
  by the non-promotable `PrivateRehearsalManifest`, **NOT** a bounty-completion claim (private C1 acceptance ≠
  bounty completion). The span adds **ONE BLUE canonical type** (`ArrayHead`, 457 → 458) + **two additive BLUE
  fns** (`encode_n2n_version_params`, `decode_array_head_two_form`) + **one version-gated BLUE field**
  (`SeedEpochConsensusInputs.epoch_nonce` + schema `1 → 2`) — **NO extensible / negotiated surface, no plugin
  trait, no `Box<dyn _>`, no runtime-registered handler, no new `--mode node` flag, no new `NodeBlockSource` /
  `CoordinatorEvent` / `Mode` variant.** **No rule weakened.** `run_relay_loop`'s containment is byte-unchanged.
- **PrevHash null/hash32 wire authority + position-aware header rule + genesis-successor cold-start forge honest
  scope + boundary (N-F-G-J, load-bearing — do NOT soften / do NOT broaden):** G-J makes the header `prev_hash`
  field the closed wire grammar `$hash32 / null` and discharges the position-aware coupling — and **nothing
  more**. The raw byte codec (`ade_codec::shelley::block::decode_prev_hash` + the `ShelleyHeaderBody` `AdeEncode`
  match) is **POSITION-BLIND**: it decodes `null -> Genesis` / `hash32 -> Block` as a pure function of the CBOR
  token, NEVER consulting `block_number`. The position-AWARE coupling (`block_number 0 <=> Genesis`) lives in
  EXACTLY ONE BLUE authority, `ade_ledger::block_validity::header_position::check_header_position`, called by
  `decode_block` BEFORE the header authority and surfacing `BlockValidityError::HeaderPositionInvalid` (coarse
  class `HeaderInvalid` — **no new `BlockRejectClass`**); it **MUST NOT** live in `ade_codec` and **MUST NOT** be
  duplicated. The `null` grammar is scoped to **header_body ONLY** — it MUST NOT leak into the chain-sync/
  block-fetch `Point`/`Tip` codec (`Point::Origin` stays `array(0)`). No all-zero `Hash32`, no anchor
  fingerprint, and no Shelley genesis hash may stand in for the genesis predecessor — it is structurally CBOR
  null (the producer `prev_hash` migrated `Hash32 -> PrevHash` end to end, deleting the all-zero stand-in at its
  source in `ChainEvolution::prev_hash()`). The node-spine cold-start first-block reachability (`DC-NODE-08`) is
  **scoped to the hermetic cold-start execution** (NO `genesis_forged` latch); the recovered seed-epoch lineage
  gates **PERMISSION** to forge from the genesis-successor position, it is **NOT** the source of the prev_hash
  bytes; the forge engine holds **no `ChainDb` handle**, so it advances **no durable tip** (durable block-1+
  progression is N-U). The S1 scheduling-event vocabulary (`NodeSchedEvent` / `FeedReason` / `ForgeOutcome`) is
  **closed + emit-only** — it records the relay loop's feed/forge decisions WITHOUT changing them, and the GREEN
  planner never names it. **NO RO-LIVE flip** (`RO-LIVE-01` stays `partial` / operator-gated); the cold-start
  eligibility signal is general (forge-configured + valid recovered base), **never a private-only / C1-only
  flag**. G-J adds **ONE BLUE canonical type** (`PrevHash`, 456 → 457) + **ONE NEW BLUE module**
  (`header_position`, a fn + a variant) + **closed GREEN diagnostic enums** + **two additive closed-enum
  variants** — **NO extensible / negotiated surface, no plugin trait, no new `--mode node` flag, no new
  `NodeBlockSource` / `CoordinatorEvent` / `Mode` / `BlockRejectClass` variant.** **No rule weakened.** S5 reuses
  the existing BA-02 + rehearsal surfaces VERBATIM (no new evidence type, no manifest-schema change). _(G-J does
  not alter the G-A..G-E forge / serve / live-feed / containment surfaces; the three containment / handoff /
  memory fences are byte-unchanged.)_
- **Bounty DRY-RUN rehearsal harness honest scope + boundary (N-F-G-D, load-bearing — do NOT soften / do NOT
  broaden):** G-D ships a **path-faithful, non-promotable bounty DRY-RUN harness — NOT the bounty deliverable.**
  **(1) Path fidelity:** the C1 private-testnet dry-run uses the SAME `--mode node` accepted-block path as
  preview/preprod (`import_live_consensus_inputs` → forge → self-accept → sibling-serve → block-fetch → peer log →
  `correlate`), with NO private-only flag / branch / bootstrap / from-genesis constructor (the `cli.rs` flag set is
  the pinned 28-flag closed allow-list); the only differences are operator INPUTS + the evidence LABEL — a
  condition that would fail on preprod is a shared-path bug, never special-cased. **(2) Evidence
  non-promotability:** the GREEN `PrivateRehearsalManifest` cannot represent a non-rehearsal (`is_rehearsal` /
  `not_bounty_evidence` literals; sole ctor `from_correlate_outcome`, `None` on `NoEvidence`); `correlate` stays
  the SOLE `Ba02Manifest` ctor (RED `rehearsal_pass` REUSES `ba02_pass::correlate_peer_log_file` verbatim, no
  alternate correlator); a rehearsal manifest lives ONLY under the rehearsal home, sha256-bound, never under /
  referenced by a bounty home (three barriers). **G-D does NOT enforce that a C1 run has succeeded** — the
  rehearsal-schema gate is **vacuous until a real operator-produced manifest is committed**; the C1 dry-run is
  **NOT a runtime node mode** and is **not wired into any binary arm** (exercised only by the env-gated RED test
  `node_c1_dry_run_rehearsal_live`, skipped in CI). **NO RO-LIVE flip** (`RO-LIVE-01` partial; `RO-LIVE-06`
  schema-only), **NO bounty / preview / preprod completion claim**; private C1 acceptance ≠ bounty completion (the
  live C1 execution stays `blocked_until_operator_c1_net_executed`). **No new BLUE authority / canonical type / argv
  flag / `NodeBlockSource` / `CoordinatorEvent` / `Mode` variant**; the relay-loop containment gate, the
  served-chain handoff fence, and the live-feed memory bounds are **byte-unchanged**.
- **Live-feed bounded-memory honest scope + boundary (N-F-G-E, load-bearing — do NOT soften / do NOT broaden):**
  N-F-G-E bounds **peer-driven memory on the live `--mode node` feed BEFORE authoritative decode/apply** — and
  nothing more. Two closed literal caps fail closed in front of the BLUE `ade_codec` decode path
  (`MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` → the additive closed `SessionError::ReassemblyBufferOverflow` → drop the
  peer; `MAX_WIRE_PUMP_LOOKAHEAD = 256` → the bounded `mpsc` back-pressures), with **NO CLI / env / config escape
  hatch**. The claim is **NARROW**: it is **NOT** full network DoS resistance, **NOT** peer resource fairness,
  **NOT** BA-02 / live-evidence readiness — other DoS surfaces (the per-connection aggregate ceiling across the
  ~10 independent per-protocol reassembly buffers, per-connection-COUNT / peer-fairness) remain future hardening
  slices. The bounds are **defensive implementation constants, NOT Cardano semantic parameters** (tighten-only).
  **NO BLUE change** (`ade_codec` byte-unchanged); **NO new `NodeBlockSource` / `CoordinatorEvent` variant**; the
  serve/forge/containment fences are **byte-unchanged**; **no live-evidence / BA-02 / RO-LIVE flip or claim**.
  `DC-LIVEMEM-01` is `tier = derived` (operational hardening, NOT BLUE consensus law). _(G-E does not alter the
  G-C live-feed wiring, the serve mechanism, or any BLUE authority.)_
- **Live-feed + BA-02-evidence honest scope + boundary (N-F-G-C, load-bearing — do not soften):** N-F-G-C closes
  the **MECHANICAL live-feed + evidence scaffolding ONLY**. The `--mode node` `On` arm is **live-feed-wireable**
  from `--peer` (reusing the closed admission dial + pump; a `from_wire_pump` FILL of the closed `WirePump` arm —
  no new variant, no new wire authority); with a live feed the forge is observable at a due leader slot. **Peer
  ACCEPT is NOT claimed — it is operator-gated** (`RO-LIVE-01` partial / `blocked_until_operator_stake_available`),
  proven only by an operator-captured peer log through `correlate`. The BA-02 evidence I/O (`ba02_pass`) reads a
  real operator-captured peer-log file through the SOLE `Ba02Manifest` constructor; `write_ba02_manifest` accepts
  ONLY a `Ba02Manifest`; **no synthetic manifest is committed** (the new gate is vacuous-until-committed +
  sha256-bound). **Ade self-accept / `ForgeSucceeded` / served-block / wire success ≠ peer acceptance.** With NO
  `--peer` the empty source still halts before any `ForgeTick`. The durable tip still advances only via
  `run_node_sync → pump_block`; `run_node_sync` UNMODIFIED; `ci_check_node_run_loop_containment.sh` byte-unchanged.
  **NO new BLUE authority / canonical type, no new `NodeBlockSource` / `CoordinatorEvent` variant.** **The bounty
  acceptance criterion (an operator-witnessed accepted block) is NOT satisfied by this cluster. BA-02 is satisfied
  nowhere.**
- **Self-accept→serve handoff honest scope + boundary (N-F-G-B, load-bearing — do not soften):** N-F-G-B is
  **serve-authority only** — it proves the serve *mechanism* on the `--mode node` spine admits **only** a BLUE
  self-accepted artifact, via the constructor-fenced `SelfAcceptedHandoff` carried into a **sibling** served-chain
  admit task whose sole mutation is the single `ServedChainHandle::push_atomic` fed by `into_accepted()`. The
  relay-loop body forwards a **typed channel send only**, so `ci_check_node_run_loop_containment.sh` is
  **byte-/semantically unchanged**. It introduces **no new BLUE authority / canonical type**, adds **no new
  `CoordinatorEvent` variant**, and makes **NO peer-acceptance / BA-02 / RO-LIVE acceptance claim**. `DC-NODE-06`
  is ENFORCED. _(N-F-G-C added the LIVE FEED around this serve mechanism; the serve mechanism is unchanged.)_
- **Forge-fidelity honest scope + boundary (N-F-G-A, load-bearing — do not soften):** N-F-G-A is
  **forge-fidelity hardening on the relay spine** — real cardano-cli config ingress (opcert + genesis),
  oracle-bound **current** `ProtocolParameters` (no longer `::default()`), a before-anchor clock→slot
  fail-closed guard (S3), and an off-epoch forge fail-closed guard *before* leadership / KES signing (S4). It
  does **NOT** serve / admit / gossip / advance a durable tip. The forge stays **subordinate + self-accept-only**;
  `run_relay_loop`'s containment is **semantically unchanged**. The two new boundaries are **fail-closed walls,
  not silent saturations**. `protocol_params` carries **no float path**. The S1 reference fixture is **evidence
  input, never runtime authority**. The `protocol_params_json` preimage stays **OUTSIDE** the frozen 15-field
  `LiveConsensusInputsCanonical` fingerprint (a non-fingerprinted additive carry).
- **Operator-key ingress honest scope + boundary (N-F-F, load-bearing — do not soften):** N-F-F is
  **operator-key ingress + the binary `Some`/`None` forge flip**. The binary is **forge-CAPABLE with real
  operator keys**; key custody is **RED-confined** to `ProducerShell`. **Mithril stays the bootstrap/recovery
  layer, untouched.**
- **Forge-tick honest scope + boundary (N-F-E, load-bearing):** N-F-E is a **hermetic, single-epoch,
  self-accept-only** forge-tick wiring cluster. **NO serve/broadcast/gossip; NO durable apply / tip mutation.**
- **Relay-run-loop honest scope (N-F-D):** the live relay run-loop is **relay-only** — N-F-G-C wires a **live
  unbounded peer** into the `WirePump` arm (the feed-side RO-LIVE-01 follow-on); the operator-witnessed live
  ACCEPT remains the gating follow-on.
- **Node-lifecycle honest scope (N-F-C):** PHASE4-N-F-C proves the node lifecycle mechanics through evidence
  closure; it does NOT claim live BA-02. `ba02_evidence` + the N-F-G-C `ba02_pass` I/O wrapper are reached by no
  binary arm (tested only).
- **Recovered-state surface is populate-contained AND consume-fenced (N-F-A populate / N-F-C consume; N-F-E +
  N-F-F + N-F-G-A exercise):** populated ONLY on the verified-bootstrap path; read back ONLY by the warm-start
  restore (CN-CINPUT-02). The forge-time `produce_mode` path may not populate them and stays diagnostic.
- **No new bootstrap-source plugin seam (N-Y hard rejection, carried into N-Z + N-F-A + N-F-C + N-F-F + N-F-G-A +
  N-F-G-C):** a new seed source attaches by populating `BootstrapInputs.genesis_initial` and routing through
  `bootstrap_initial_state` — NEVER via a `GenesisAnchor` / `MithrilAnchor` trait or plugin registry.
  **Operator-key ingress (N-F-F), forge fidelity (N-F-G-A), and the live feed (N-F-G-C) are NOT a bootstrap** —
  they reuse the single recovered `BootstrapState` as the forge base, call no Mithril, and create no second
  bootstrap.
- **Mithril seed-point independence (N-Z hard rule, DC-MITHRIL-02):** the anchor `seed_point` MUST originate
  from an operator-supplied origin structurally independent of the manifest; `verify_mithril_binding`
  cross-checks the two and fails closed; the binding must run before any storage init.
- **No synthetic forge state (N-T):** `produce_mode` MUST NOT construct `SyntheticForgeInputs`, a zero-stake
  `LeaderScheduleAnswer`, or an inline `LedgerState::new(...)` forge base.
- **No durability in the produce_mode forge path (N-U scope):** forged-block durability is deferred to N-U
  (§7). The network forward-sync durability (received blocks) DID land in N-Y; the N-F-E/N-F-F/N-F-G-A
  relay-loop forge tick advances NO durable tip (self-accept-only).
- **Bounded peer-driven memory (CLOSED by N-F-G-E — was the N-F-G-C-exposed §7 candidate #6):** the live
  `--mode node` feed's unbounded mux-reassembly tail (`ade_network::session::core`) + unbounded
  `WirePump.lookahead` (`ade_node::node_sync`) are now BOUNDED before authoritative decode/apply by two closed
  literal caps — `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` (fail-closed via the additive closed
  `SessionError::ReassemblyBufferOverflow` → drop the peer) + `MAX_WIRE_PUMP_LOOKAHEAD = 256` (the bounded `mpsc`
  back-pressures), each with NO CLI/env/config escape hatch, fenced by the NEW `ci_check_live_feed_memory_bounds.sh`
  (DC-LIVEMEM-01). **NARROW claim — bounded memory before decode/apply, NOT full network DoS resistance / peer
  fairness / BA-02 readiness** (per-connection aggregate + per-connection-COUNT / peer-fairness remain future
  hardening). The prior CBOR length-overflow remote-DoS (N-X) was already NOT reintroduced.
- **Bounded peer-driven SERVE range (CLOSED by PHASE4-N-AA — closes the N-U cross-slice security-review MEDIUM):**
  the `--mode node` durable-chain serve (`ChainDbServedSource`, DC-NODE-13) — which formerly read each peer
  BlockFetch range via the unbounded `iter_from_slot` (full `from..tip` materialization + per-block `SLOT_BY_HASH`
  scan, O(N²)) + an O(N) `chaindb.tip()` — is now BOUNDED before unbounded storage/CPU by the FIXED defensive cap
  `MAX_SERVE_RANGE_BLOCKS = 256` (symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD = 256`, DC-LIVEMEM-01),
  reading via the bounded hash-free `range_bytes_capped` + `last_block_bytes` and failing closed via the closed
  `ServeRangeOutcome` (`CapExceeded` / `Empty` / `ReadError` → wire `NoBlocks`), with NO CLI / env / config escape
  hatch, fenced by the NEW `ci_check_serve_range_bounded.sh` (DC-SERVEMEM-01). **NARROW claim — a per-request
  serve-range bound on the read-only serve path, NOT full network DoS resistance / peer fairness / per-connection-
  COUNT or repeated-request limits** (those remain SEPARATE, out-of-scope future hardening, as with DC-LIVEMEM-01).
  **RED-only, 0 BLUE diff; no second hash authority; DC-NODE-07 single dispatch preserved; within-cap serving
  byte-identical (DC-CONS-17); no RO-LIVE flip** (`RO-LIVE-01` stays `partial`). The trusted recovery / rollback
  `iter_from_slot` internals are out of scope (doc-fenced, not peer-driven).
- **Outbound mux segmentation (CLOSED by PHASE4-N-AB — pre-RO-LIVE hardening item 2):** the GREEN
  `ade_network::session::core` formerly had no symmetric OUTBOUND path — an outbound mini-protocol payload larger
  than the mux SDU limit `MAX_PAYLOAD` (e.g. a Conway block-fetch `Block` over 64 KiB) hit `encode_inner_frame`'s
  strict single-frame guard and FAILED, so the `--mode node` serve could not transmit a large block. N-AB makes
  `handle_outbound` SEGMENT a payload with `MAX_PAYLOAD < len <= MAX_OUTBOUND_PAYLOAD_BYTES` into ordered
  `<= MAX_PAYLOAD` frames via the SAME single `encode_inner_frame` → `mux::frame::encode_frame` authority — the
  outbound inverse of the `CN-SESS-04` inbound reassembly — byte-preservingly (`concat(segment payloads) ==
  payload`, DC-CONS-17), each segment carrying the SAME mini-protocol id + mode + the SAME single captured
  `timestamp` (GREEN, no per-segment clock); a payload above the FIXED, non-configurable
  `MAX_OUTBOUND_PAYLOAD_BYTES = 16 MiB` (symmetric with the receive-side `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB`,
  DC-LIVEMEM-01) fails closed via the EXISTING reused `SessionError::OutboundPayloadTooLarge` (no new closed enum),
  fenced by the NEW `ci_check_outbound_segmentation.sh` (`CN-SESS-05`, enforced). Pairs with N-AA (item 1 bounds
  *how many* blocks per request; this makes *one large* block sendable). **GREEN-only, 0 BLUE diff; no second frame
  encoder (`mux::frame` reused unchanged); no truncation / drop / reorder / partial-success send; no new canonical
  type (`MAX_OUTBOUND_PAYLOAD_BYTES` is a GREEN const); no RO-LIVE flip** (`RO-LIVE-01` stays `partial`; confirming
  a real cardano-node demux accepts the reused-per-segment SDU timestamp is the live leg, hardening item 4).
- **KES signing evolves the operator key to the current period (CLOSED by PHASE4-N-AC — live-readiness fix
  surfaced by the item-4 C1 re-run):** the forge's only real KES sign formerly called the raw
  `ProducerShell::kes_sign_header(kes_period, …)`, which requires `kes.current_period() == requested`, and nothing
  evolved the minted-at-0 cardano-cli operator key forward — so the moment the chain's KES period crossed 0 (slot
  ~249659 / `slotsPerKESPeriod = 129600`) every leader slot failed `KesPeriodNotCurrent { requested: 1, current:
  0 }` → `ForgeFailed` (`succeeded = 0`; the follower KeepAlive-timed-out). N-AC adds the RED
  `ProducerShell::kes_sign_header_advancing(period, preimage)` = `kes_advance_to(period)? then
  kes_sign_header(period, …)`, EVOLVING the operator KES key FORWARD to the requested period via the EXISTING
  deterministic Sum6KES `kes_update` (idempotent at the current period; fail-closed `EvolutionBackwards` backwards
  / `EvolutionExhausted` beyond `SUM6_MAX_PERIOD = 63`) BEFORE signing; the period is passed VERBATIM; the forge's
  single real sign is rewired to it, fenced by the NEW `ci_check_kes_evolution_before_sign.sh` (DC-CRYPTO-10,
  `enforced`; `CN-KES-HEADER-01` `strengthened_in += "PHASE4-N-AC"`). **NARROW: an evolve-then-sign fix on the RED
  signing shell — signing stays RED (`T-KEY-01`); the BLUE Sum6KES algorithm + KES verifier + forge eligibility +
  wire rules are UNCHANGED; NO new BLUE authority / canonical type / closed enum / version-gated contract (the
  existing `SigningError` variants + branded `KesSignature` / `UnsignedHeaderPreImage` are reused); fail-closed
  both directions (never a key override); NO C1-only path; and NO RO-LIVE flip** (`RO-LIVE-01` stays `partial`).
  **Genesis-window / KES-period coupling (honest, NOT a gap):** `slotsPerKESPeriod = 129600` equals `3k/f =
  3·2160/0.05 = 129600`, so KES period 1 begins exactly when the genesis density window closes — a from-genesis C1
  rehearsal can never show forge-at-period-1 AND follower-adopt simultaneously (the follower's `CandidateTooSparse`
  rejection is genesis-density-window-limited, KES-INDEPENDENT); **cross-period end-to-end forge→adopt is proven on
  the dense C2 tip path**, not a net reset. Two pre-existing fail-closed INFO items are handed to C2: the
  `kes_advance_to`-zeroes-on-failed-advance (UNREACHABLE via the forge; fail-safe) and the `opcert_start + 63` vs
  absolute-63 ceiling (C2 must derive `kes_max_period` from the absolute ceiling).
- **Registry `code_locus` must track source moves (`5db9aae`):** any rule citing a renamed/moved `crates/**.rs`
  or `ci/**.sh` path must have its `code_locus` updated; `ci_check_registry_code_locus_exists.sh` fails closed
  on a stale pointer.
- **`cardano_crypto::kes` is a `#[cfg(test)]` oracle only** under `crates/ade_crypto/src/**`. `pallas-*`
  confined to `ade_plutus`.
- **Commit-attribution override (CLAUDE.md):** this repo carries a model-attribution trailer on commit messages
  only (bounty requirement). Source comments, PRs, releases, issue comments still follow the global
  no-AI-attribution rule.
- **Grounding-doc → ade-atlas rebuild trigger (operational infra — NOT a code seam):** the downstream
  `ade-atlas` repo polls the grounding docs every 10 min. It attaches nothing to the node's authority surface.

---

## 7. Candidate & Not-Yet-Wired Seams (declared follow-ons — NOT closed)

> Surfaced honestly per IDD: these are **declared** future attach points, not closed surfaces. Each is named
> in a registry rule or a cluster CLOSURE record.
>
> **N-AE.F adds an idempotency guard INSIDE the existing RED durable-admit chokepoint — it OPENS NO new attach
> point, adds NO new closed/extensible registry, and surfaces NO new candidate seam.** PHASE4-N-AE.F makes the
> receive path NO-OP a byte-identical already-applied block re-announced by the peer (the post-CE-A5 echo):
> `ade_runtime::forward_sync::pump::pump_block` queries `get_block_by_hash(&decoded.block_hash)` AFTER
> `decode_block` and BEFORE the BLUE reducer, and returns `Ok(None)` on a same-hash-same-slot hit (`DC-NODE-16`,
> enforced; gate `ci_check_receive_idempotency.sh`). This is a deterministic conditional + early return over the
> already-closed `ChainDb` membership query — **not** a new closed enum, registration path, plugin trait, or
> ingress surface; it is a TIGHTENING of the existing forged-block / received-block durable-admit chokepoint
> (`pump_block`, the sole durable tip authority — `DC-NODE-12`). A DIFFERENT block (different hash) at/before the
> last-applied slot still falls through to the UNCHANGED BLUE header authority and fails closed
> (`SlotBeforeLastApplied`). N-AE.F is **RED-only (0 BLUE diff)** and does **NOT** flip any RO-LIVE rule — surviving
> the echo is a long-running-relay precondition, not operator-witnessed peer acceptance. All candidates (#0–#3, #5,
> the carried Gap-1 multi-producer fork-choice #7) and the already-retired #4 / #6 are carried UNCHANGED. **There
> is NO NEW candidate seam from N-AE.F** (an idempotency guard inside an existing chokepoint surfaces none).
>
> **N-AE TIGHTENS three existing authority paths (recover→serve continuity + forge admissibility) — it OPENS
> NO new attach point and adds NO new closed/extensible registry.** PHASE4-N-AE makes a `--mode node` forge
> produce a peer-ADOPTABLE successor: a forge-admissibility gate (`DC-NODE-15` / `DC-CONS-24`, enforced — forge
> iff `durable_servable_tip == followed_peer_tip`, else typed `ForgeRefused::NotCaughtUp`; forged parent
> byte-equals the peer-visible tip; the followed-peer-tip signal is admissibility-only, never a chain selector); a
> proof-gated **FindIntersect-only** recovered-anchor serve projection (`DC-NODE-14`, enforced — `ChainDbServedSource::intersect`
> advertises the earliest servable `StoredBlock`'s `prev_hash` IFF a real servable successor exists, serving NO
> bytes for it); a chain-sync server FindIntersect cursor fix (`DC-PROTO-10`, enforced — `last_announced` is set to
> the resolved intersect so the next `RequestNext` rolls forward onto the successor, not block 0); and a
> recover→follow WAL prior-fp seed (strengthens `DC-WAL-02` + `T-REC-05`). All four are TIGHTENINGS of existing
> seams (the producer-forge pipeline, the `--mode node` durable-ChainDb serve projection `DC-NODE-13`, the
> chain-sync server, and the crash-recovery WAL fold) — **none opens a new ingress surface, registry, or
> version-gated contract; the closed `Point` enum + chain-sync wire grammar are unchanged.** N-AE's lone BLUE
> change is the additive read-only `DecodedBlock.prev_hash` field (no new canonical type); the new
> `ForgeRefused` / `NotCaughtUpReason` / `ForgeFollowedTipAdmission` / `NodeForgeOutcome` / `FollowedPeerTipSignal`
> are closed RED/GREEN error/signal sums in `ade_node::node_sync` (not canonical types, not registries). N-AE does
> **NOT** flip any RO-LIVE rule — the CE-A5 manifest (a real cardano-node `AddedToCurrentChain` an Ade-forged
> block) is a non-promotable C2-LOCAL rehearsal-venue event; only a committed `correlate` manifest over the
> preprod tip advances `RO-LIVE-01`. **There is ONE genuinely-DEFERRED follow-on from N-AE — candidate #7
> below: multi-producer fork-choice (Gap 1), an explicitly-separate FUTURE cluster** (AE.A adds NO fork-choice;
> `select_best_chain` / `fork_choice` stays the follow/`chain_selector` authority, untouched). All prior candidates
> (#0–#3, #5) and the already-retired #4 / #6 are carried UNCHANGED.
>
> **N-AC COMPLETES / STRENGTHENS the existing RED KES signing seam (KES signing evolves the operator key) — it
> BROADENS NOTHING below and adds NO new candidate.** PHASE4-N-AC makes the forge's single real KES sign EVOLVE
> the operator KES key FORWARD to the chain's current KES period BEFORE signing (via the EXISTING deterministic
> Sum6KES `kes_update`; fail-closed backwards / beyond-lifetime; period passed VERBATIM) — `DC-CRYPTO-10`,
> enforced; gate `ci_check_kes_evolution_before_sign.sh`; `CN-KES-HEADER-01` `strengthened_in += "PHASE4-N-AC"`.
> This **COMPLETES / STRENGTHENS the existing RED KES signing surface (CN-KES-HEADER-01)** — it does NOT open a new
> ingress surface or extension point. N-AC is **RED-only (0 BLUE diff)**, adds **NO new extension point / closed
> enum / canonical type / version-gated contract** (signing + evolution REUSE the existing `SigningError` variants
> `EvolutionBackwards` / `EvolutionExhausted` + the branded `KesSignature` / `UnsignedHeaderPreImage`; the Sum6KES
> evolution is the existing `kes_update` primitive), and does **NOT** flip any RO-LIVE rule — a forged +
> self-accepted period-1 block a real cardano-node DOWNLOADS with no KES/parse rejection is a wire-layer /
> header-acceptance event, NOT operator-witnessed peer acceptance (`RO-LIVE-01` stays `partial` / operator-gated).
> **Genesis-window / KES-period coupling finding (honest, NOT a gap):** `slotsPerKESPeriod = 129600` equals `3k/f =
> 129600`, so a from-genesis C1 rehearsal can never show forge-at-period-1 AND follower-adopt simultaneously (the
> follower's `CandidateTooSparse` is genesis-density-window-limited, KES-INDEPENDENT) — cross-period end-to-end
> forge→adopt is the **C2 dense-tip path**. All candidates (#0–#3, #5) and the already-retired #4 / #6 are carried
> UNCHANGED. **There is NO NEW candidate seam from N-AC** (completing/strengthening an existing closed RED seam
> surfaces none).
>
> **N-AB CLOSES pre-RO-LIVE hardening item 2 (outbound mux segmentation) — it BROADENS NOTHING below and adds
> NO new candidate.** PHASE4-N-AB COMPLETES the OUTBOUND direction of the existing closed wire-protocol session
> seam (CN-SESS family): the GREEN `session::core::handle_outbound` now SEGMENTS an outbound mini-protocol payload
> `> MAX_PAYLOAD` into ordered `<= MAX_PAYLOAD` frames via the single `encode_inner_frame` authority — the outbound
> inverse of the `CN-SESS-04` inbound reassembly — failing closed above the FIXED `MAX_OUTBOUND_PAYLOAD_BYTES =
> 16 MiB` (`CN-SESS-05`, enforced; gate `ci_check_outbound_segmentation.sh`). This **CLOSES pre-RO-LIVE hardening
> item 2** (recorded under "Pre-RO-LIVE hardening" above, marked CLOSED). N-AB is **GREEN-only (0 BLUE diff)**,
> adds **NO new extension point / closed enum / canonical type** (it COMPLETES an already-closed seam's outbound
> direction — `MAX_OUTBOUND_PAYLOAD_BYTES` is a GREEN const, `SessionError::OutboundPayloadTooLarge` is reused, the
> single `encode_inner_frame` authority + the closed `AcceptedMiniProtocol` registry are reused), and does **NOT**
> flip any RO-LIVE rule — segmenting bytes ≠ operator-witnessed peer acceptance (`RO-LIVE-01` stays `partial` /
> operator-gated). All candidates (#0–#3, #5) and the already-retired #4 / #6 are carried UNCHANGED. **There is NO
> NEW candidate seam from N-AB** (completing an existing closed seam's outbound direction surfaces none).
>
> **N-AA CLOSES pre-RO-LIVE hardening item 1 (bounded peer-driven serve range) — it BROADENS NOTHING below and
> adds NO new candidate.** PHASE4-N-AA bounds the `--mode node` durable-chain serve (`ChainDbServedSource`,
> DC-NODE-13) against peer-driven resource amplification — the FIXED `MAX_SERVE_RANGE_BLOCKS = 256` cap +
> bounded hash-free reads (`range_bytes_capped` / `last_block_bytes`) + fail-closed `ServeRangeOutcome` →
> `NoBlocks` (`DC-SERVEMEM-01`, enforced; gate `ci_check_serve_range_bounded.sh`). This **CLOSES the PHASE4-N-U
> cross-slice security-review MEDIUM finding** (item recorded under "Pre-RO-LIVE hardening" below, now marked
> CLOSED). N-AA is **RED-only (0 BLUE diff)**, adds **NO new extension point** (it BOUNDS the already-closed
> DC-NODE-13 serve seam — it opens no new one; `CappedSlotRange` + `ServeRangeOutcome` are closed RED value
> types, the cap is a fixed non-configurable constant), and does **NOT** flip any RO-LIVE rule — bounding serve
> work ≠ operator-witnessed peer acceptance (`RO-LIVE-01` stays `partial` / operator-gated). The serve-side
> candidate #0 below stays gating on the operator-witnessed ACCEPT; all other candidates (#0–#3, #5) and the
> already-retired #4 / #6 are carried UNCHANGED. **There is NO NEW candidate seam from N-AA** (a defensive
> bounding of an existing closed seam surfaces none).
>
> **N-U CLOSES candidate #4 (forged-block durability) and RETIRES the G-R serve-gate mechanism — it BROADENS
> NOTHING below.** PHASE4-N-U ships forged-block durability: the forge's OWN self-accepted block becomes durable
> through the single `pump_block` chokepoint (`DC-NODE-12` / `DC-CONS-23` / `DC-WAL-04`), recovers byte-identically
> across restart + drops an un-WAL'd orphan (`T-REC-05`), and the `--mode node` served view becomes a READ-ONLY
> projection of the durable ChainDb (`DC-NODE-13`). So **candidate #4 below is now CLOSED** (marked inline).
> N-U **RETIRES the PHASE4-N-F-G-R `serve_gate_admits` monotone-serve-gate mechanism** (superseded by
> serve-as-projection — the durable chain is extend-only, so it holds exactly one block 0 by construction and a
> follower fetches coherent history A→B more strongly than the in-memory predicate, and serving survives
> restart; the `DC-NODE-11` invariant is preserved + strengthened, its gate migrated
> `ci_check_served_chain_stability.sh` → `ci_check_served_chain_projection.sh`). N-U adds **NO new extension
> point** (the durable admit REUSES `pump_block`; the serve projection is a SECOND impl of the already-closed
> serve trait seams selected through the closed `ServedChainSource` enum) and does **NOT** flip any RO-LIVE rule —
> durability + coherent serve ≠ operator-witnessed peer acceptance (`RO-LIVE-01` stays `partial` /
> operator-gated). The serve-side candidate #0 below stays gating on the operator-witnessed ACCEPT (the N-U
> serve projection makes the served view durable + restart-surviving, but the peer-ACCEPT leg is unchanged).
> Candidates #0–#3 and #5, and the already-retired #6, are carried UNCHANGED.
>
> **N-F-G-K … G-R RETIRE NOTHING and BROADEN NOTHING below.** The eight-cluster span adds ONLY closed sums +
> version-gated fields (the `ArrayHead` FindIntersect wire grammar + the `Origin` reply; the single per-version
> `versionData` encoder; the version-gated recovered `epoch_nonce`; the feed tag-24 reuse of the single CN-WIRE-08
> authority; and RED orchestration — serve-lifetime decouple, feed header-validation view, forge-successor
> position, monotone serve gate) — **no new extension point, no plugin/negotiated surface.** It does NOT close any
> §7 candidate and does **NOT** advance the bounty deliverable: `RO-LIVE-01` stays `partial` (the forge is
> self-accept-only, holds no `ChainDb` handle, and advances no durable tip — N-U owns durability; the serve
> sibling is read-only over the `ServedChainView`). The C1 genesis-successor rehearsal evidence (`129d25ac`,
> `65954fa3`) is a **rehearsal REPRODUCTION** of the same operator-gated DRY-RUN class as the G-D C1 dry-run —
> fenced by the non-promotable `PrivateRehearsalManifest`, under the IN-PLACE-broadened
> `ci_check_rehearsal_manifest_schema.sh` (now also covering the c1 rehearsal home), **NOT bounty evidence**,
> flipping no RO-LIVE rule (private C1 acceptance ≠ bounty completion). All candidates (#0–#5) and the
> already-retired #6 are carried UNCHANGED. **(The G-H §1 serve-to-peer surface is still NOT spliced into this
> body — see the G-H §1-gap note in the header; candidate #0 there is the closest related seam, and the G-K/G-L/
> G-M/G-R serve deltas EXTEND that G-H serve sibling.)**
>
> **N-F-G-J RETIRES NOTHING and BROADENS NOTHING below.** G-J adds ONLY closed sums + one BLUE module (the
> `PrevHash` wire grammar + the single `header_position` validator authority + the closed emit-only
> `NodeSchedEvent` vocabulary + two additive closed-enum variants + the genesis-successor cold-start PATH through
> the UNCHANGED closed `NodeBlockSource`) — **no new extension point, no plugin/negotiated surface.** It does NOT
> close any §7 candidate and does **NOT** advance the bounty deliverable: `RO-LIVE-01` stays `partial` (the
> cold-start forge is hermetic + self-accept-only, advancing no durable tip — N-U owns durability). The S5 C1
> genesis rehearsal reuses the existing rehearsal surfaces VERBATIM (no new evidence type), under the
> extended `ci_check_rehearsal_manifest_schema.sh`; it is the same operator-gated DRY-RUN class as the G-D C1
> dry-run (`blocked_until_operator_c1_net_executed`), NOT bounty evidence, flipping no RO-LIVE rule. All
> candidates (#0–#5) and the already-retired #6 are carried UNCHANGED. **(The G-H serve-to-peer surface is NOT
> spliced into this body — see the G-H-gap note in the header; candidate #0 there is the closest related seam.)**
>
> **N-F-G-D RETIRES NOTHING and BROADENS NOTHING below.** G-D adds a path-faithful, non-promotable bounty DRY-RUN
> **harness** (the rehearsal-evidence surface + the path-fidelity fence) — a HARNESS, not a closure of any
> candidate. It does **NOT** advance the bounty deliverable: `RO-LIVE-01` stays `partial`, `RO-LIVE-06` stays
> schema-only, and the rehearsal-schema gate is vacuous-until-committed. G-D adds **one new operator-pass
> execution gate** (the **C1 private-testnet dry-run**, recorded under "Operator-pass execution gates" below) —
> operator-gated (`blocked_until_operator_c1_net_executed`), exercised only by the env-gated RED test
> `node_c1_dry_run_rehearsal_live` (skipped in CI, NOT a runtime node mode), producing a clearly-marked rehearsal
> manifest, NOT bounty evidence, flipping no RO-LIVE rule. It is **DISTINCT** from the bounty operator pass (the C2
> preprod leg). All candidates (#0–#5) and the already-retired #6 are carried UNCHANGED.
>
> **N-F-G-E CLOSED the prior candidate #6** (bounded mux-reassembly tail + WirePump lookahead — the live-feed
> peer-driven memory surface that the G-C close added from its security-review MEDIUM). It is **RETIRED** from the
> list below: two closed literal caps (`MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` + `MAX_WIRE_PUMP_LOOKAHEAD = 256`),
> each fail-closed before authoritative decode/apply, with NO CLI/env/config escape hatch, fenced by the NEW
> `ci_check_live_feed_memory_bounds.sh` (DC-LIVEMEM-01). **G-E closed ONE specific live-feed memory surface — it
> is NOT full network DoS resistance, NOT peer resource fairness, NOT BA-02 / live-evidence readiness**; other DoS
> surfaces remain future hardening slices (e.g. the per-connection aggregate ceiling across the ~10 independent
> per-protocol reassembly buffers, and per-connection-COUNT / peer-fairness limits, are out of scope).
>
> **N-F-G-C PARTLY closed candidate #0** (the live network serve + operator-peer surface): the **live feed is
> wired** (consume side — a LIVE `NodeBlockSource::WirePump` from `--peer`, reusing the closed admission dial +
> pump) and the **operator-pass evidence path is scaffolded** (the runbook + the `ba02_pass` `correlate` wiring +
> the manifest-schema gate). **BUT the operator-witnessed live ACCEPT remains the gating follow-on** —
> `RO-LIVE-01` stays `partial` / `blocked_until_operator_stake_available`; `RO-LIVE-06` is NOT a live-BA-02 claim
> (schema + mechanics only). The other candidates (#0-#5) are carried UNCHANGED.

0. **Live network serve + operator-peer surface (PARTLY closed by N-F-G-C; the operator-witnessed ACCEPT remains
   the gating follow-on).** N-F-G-B CLOSED the self-accept→serve handoff (`DC-NODE-06` enforced). **N-F-G-C wired
   the live feed (consume side)** — when `--peer` is supplied, `spawn_live_wire_pump_source` builds a LIVE
   `NodeBlockSource::WirePump` (reusing the closed `dial_for_admission` + `run_admission_wire_pump`; a
   `from_wire_pump` FILL of the closed `WirePump` arm), so `LoopStep::ForgeTick` is reachable in production
   (`live_wire_pump_feed_reaches_forge_tick`) and a self-accepted forged block is served byte-identically over
   in-process block-fetch (`live_feed_forge_serve_loopback_returns_forged_block`). **N-F-G-C also scaffolded the
   operator-pass evidence path** — the `docs/evidence/phase4-n-f-g-c-operator-pass-README.md` runbook + the RED
   `ba02_pass::correlate_peer_log_file` wiring over the GREEN `correlate` + the no-synthetic-manifest gate
   `ci_check_ba02_evidence_manifest_schema.sh`. **What REMAINS gating:** the **operator-witnessed live ACCEPT** —
   a real operator-captured peer log through `correlate` against a peer that can grant leadership (C1 private
   testnet or C2 preprod with provisioned stake). `RO-LIVE-01` stays `partial` /
   `blocked_until_operator_stake_available`; `RO-LIVE-06` is NOT a live-BA-02 claim. _Confirm: the operator-pass
   execution is the next live leg; it MUST NOT relax the relay-loop containment / served-chain handoff fence, and
   a peer-ACCEPT claim stays gated on a real operator-captured peer log through `correlate` (no synthetic manifest)._
1. **Live unbounded peer → observable forge ACCEPT (RO-LIVE-01 follow-on — DECLARED, feed-side now wired by
   N-F-G-C).** N-F-G-C wired the **feed side** — a live unbounded cardano-node peer over the wire drives
   `run_node_sync` (the prior feed-side follow-on, now closed). What REMAINS is the **operator-witnessed
   observable forge ACCEPT** against a peer that can grant Ade leadership (operator-gated; `RO-LIVE-01` partial /
   `blocked_until_operator_stake_available`). _Confirm: does `NodeBlockSource` stay a closed verdict-decoupled
   contract (a `from_wire_pump` FILL, not a plugin point for alternative sources)? It does at this HEAD._
2. **Live BA-02 (RO-LIVE-06 follow-on — DECLARED, I/O scaffolded by N-F-G-C).** The schema + correlator mechanics
   are closed, and N-F-G-C wired the RED `ba02_pass` file I/O over the GREEN `correlate` (reached by no binary arm
   at this HEAD; tested only). A real BA-02 result needs a real operator-captured peer log naming the exact
   Ade-forged hash, run through `correlate`, against a peer that can grant leadership (operator-gated, distinct
   from RO-LIVE-01). Synthetic fixtures prove the mechanics only; the new gate is vacuous-until-committed +
   sha256-bound. BA-02 stays satisfied NOWHERE.
3. **Mithril import — remaining open obligations (RO-MITHRIL-IMPORT-01, still `partial`).** N-Z CLOSED item (b).
   Two seams remain deliberately NOT wired: **(a) seed-bytes-from-Mithril decode** and **(c) a committed
   reproducible Mithril fixture + CI/release evidence**. `bootstrap_from_mithril_snapshot` is composition-only
   with **NO standalone argv flag**. _(N-F-F/N-F-G-A/N-F-G-C left Mithril untouched.)_
4. **~~N-U — forged-block durability~~ — CLOSED by PHASE4-N-U (HEAD `4e358e92`).** WAL / ChainDB / warm-start for
   producer-**forged** blocks is now wired: (1) the forge's OWN self-accepted block (`accepted.into_bytes()`,
   byte-identical — no re-encode) is admitted durably through the SAME single `forward_sync::pump_block`
   chokepoint received blocks use, via the fenced `node_sync::admit_forged_block_durably` (extend-only,
   durable-before-tip; a stale-tip forge fails closed — `DC-NODE-12` / `DC-CONS-23` / `DC-WAL-04`; reuses
   `WalEntry::AdmitBlock`, NO new `NodeBlockSource` / `WalEntry` variant; `pump_block` stays the sole durable
   tip authority); (2) production `warm_start_recovery` recovers a forged-block durable tip byte-identically
   (forward-replay from the sub-tip snapshot, reusing the BLUE `bootstrap_initial_state` authority) + drops an
   un-WAL'd orphan + fails fast on a WAL-tail fingerprint divergence (`T-REC-05`, test-enforced —
   `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay` + `warm_start_drops_orphan_block_above_wal_tail`);
   (3) the `--mode node` served view is a READ-ONLY projection of the durable ChainDb (`ChainDbServedSource` over
   `ServedChainSource::DurableChainDb`, read by the single `dispatch_server_frame_event_to_outbound` — DC-NODE-07
   preserved — `DC-NODE-13`). Gates `ci_check_forged_durable_admit_via_pump.sh` (S1) +
   `ci_check_served_chain_projection.sh` (S3); the G-R `ci_check_served_chain_stability.sh` is RETIRED.
   **NARROW: durability + coherent serve only — NO admit-time fork-choice (extend-only; `select_best_chain` /
   `fork_choice` are NOT on this path), NO second durable tip-advance path, NO new BLUE authority / canonical
   type, and NO RO-LIVE flip** (durability ≠ operator-witnessed peer acceptance; `RO-LIVE-01` stays `partial`).
   _(The N-Y forward-sync durability already covered **received** blocks; N-U closes the producer-**forged** half.)_
5. **Sync-evidence live leg (N-Y — RO-SYNC-EVIDENCE-01, `partial`).** The snapshot→tip sync-evidence manifest
   schema is enforced but vacuously satisfied until a manifest is committed. An operator-witnessed execution
   gate, not a code seam.
6. **~~Bounded peer-driven memory on the now-live feed~~ — CLOSED by PHASE4-N-F-G-E (RETIRED from this list).**
   _(Added at the G-C close from the G-C per-cluster security-review MEDIUM; closed at HEAD `6f848825`.)_ The two
   surfaces — the per-mini-protocol mux-reassembly tail (`ade_network::session::core`) and the content-blind
   `WirePump.lookahead` (`ade_node::node_sync`, `VecDeque<Vec<u8>>`) — are now BOUNDED before authoritative
   decode/apply by **two closed literal caps**: `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` (fail-closed via the additive
   closed `SessionError::ReassemblyBufferOverflow` → `mux_pump::session_err_to_halt` drops the peer; no silent
   truncation, no partial decode) + `MAX_WIRE_PUMP_LOOKAHEAD = 256` (the bounded `mpsc`, cap 64, back-pressures
   the pump). Both carry **NO CLI / env / config escape hatch** and are fenced by the NEW
   `ci_check_live_feed_memory_bounds.sh` (DC-LIVEMEM-01, `enforced`). The over-cap behavior is fail-closed
   (drop-and-disconnect / back-pressure — never a silent tip graft or a fabricated block); the relay-loop
   containment gate + the verdict-decoupled `NodeBlockSource` contract are byte-unchanged. **Scope was NARROW —
   bounded memory before decode/apply only.** It is **NOT** full network DoS resistance, **NOT** peer resource
   fairness, **NOT** BA-02 / live-evidence readiness; the per-connection aggregate ceiling across the ~10
   independent per-protocol reassembly buffers and per-connection-COUNT / peer-fairness limits remain SEPARATE,
   out-of-scope future hardening slices.

7. **~~Pre-RO-LIVE hardening item 1 — bounded peer-driven serve range~~ — CLOSED by PHASE4-N-AA (HEAD `b0365df0`).**
   The `--mode node` durable-chain serve (`ChainDbServedSource`, DC-NODE-13) formerly read each peer BlockFetch
   range via the unbounded `iter_from_slot` (full `from..tip` materialization + a per-block `SLOT_BY_HASH` hash
   scan, O(N²)) plus an O(N) `chaindb.tip()` on every chain-sync request — a peer-driven resource-amplification
   path flagged by the **PHASE4-N-U cross-slice security review (MEDIUM)**. N-AA bounds it: (1) the serve reads via
   the bounded **hash-free** ChainDb primitives `range_bytes_capped` + `last_block_bytes` (NOT `iter_from_slot` /
   `chaindb.tip()`); (2) each request is capped at the **FIXED, non-configurable** `MAX_SERVE_RANGE_BLOCKS = 256`
   (symmetric with the receive-side `MAX_WIRE_PUMP_LOOKAHEAD = 256`, DC-LIVEMEM-01) — NO CLI / env / config escape,
   no unbounded mode; (3) the hash is derived from each block's own bytes via the single BLUE `decode_block`
   authority (NO second hash authority, NO `SLOT_BY_HASH` on the serve path); (4) an oversized / out-of-chain /
   inverted (`from > to`) / undecodable range fails closed via the closed `ServeRangeOutcome` (`CapExceeded` /
   `Empty` / `ReadError`) → wire `NoBlocks` (the serve never emits a block it cannot authenticate). Gate
   `ci_check_serve_range_bounded.sh` (DC-SERVEMEM-01, `enforced`); the security-review MEDIUM (`5c9f6cf6`) added the
   inverted-range guard on both `ChainDb` impls. **NARROW: a per-request serve-range bound on the read-only serve
   path — RED-only (0 BLUE diff); NO second hash authority; DC-NODE-07 single dispatch + DC-CONS-17 within-cap
   byte-identity preserved; NOT full network DoS resistance / peer fairness / per-connection-COUNT limits (SEPARATE,
   out-of-scope future hardening, as with DC-LIVEMEM-01); and NO RO-LIVE flip** (`RO-LIVE-01` stays `partial`). The
   trusted recovery / rollback `iter_from_slot` internals are doc-fenced (not peer-driven), unchanged. _(The
   serve-side analog of N-F-G-E's bounded receive-side memory; the two are symmetric — receive `MAX_WIRE_PUMP_LOOKAHEAD`
   / serve `MAX_SERVE_RANGE_BLOCKS`, both = 256.)_

8. **Multi-producer fork-choice (Gap 1 — DECLARED future cluster; explicitly out of N-AE).** PHASE4-N-AE closed
   Gap 2 (recover→serve continuity + forge admissibility — a `--mode node` forge now produces a peer-ADOPTABLE
   successor, proven LIVE by the CE-A5 manifest). **Gap 1 — fork-choice / multi-producer intake — is explicitly a
   SEPARATE future cluster.** At this HEAD the `--mode node` forge is *subordinate to the follow spine* (DC-NODE-05)
   and forges only on the followed tip; `select_best_chain` / `fork_choice` (the BLUE Praos fork-choice authority,
   DC-CONS-03) stays the follow / `chain_selector` path and is **untouched** — AE.A adds **NO** fork-choice, and
   the followed-peer-tip signal is admissibility-only, never a chain selector (`ci_check_forge_followed_tip_admission.sh`
   fences that it never reaches `select_best_chain`). The deferred work — selecting among competing producer chains
   / admit-time fork-choice on the `--mode node` spine — is its own cluster (recorded in the PHASE4-N-AE cluster doc
   §2/§11 and `docs/planning/c2-local-discovered-gaps.md`). _Confirm: when wired, fork-choice attaches to the
   EXISTING BLUE `fork_choice` / `select_best_chain` authority (it is NOT a new seam — the authority already
   exists); the followed-peer-tip admissibility signal MUST remain admissibility-only and MUST NOT be promoted into
   a chain selector._

### Operator-pass execution gates (schema enforced, execution blocked)

- **C1 private-testnet DRY-RUN (NEW, N-F-G-D — CN-REHEARSAL-FIDELITY-01; schema enforced, execution blocked)** —
  the rehearsal-manifest schema + the path-fidelity fence are enforced (`ci_check_rehearsal_manifest_schema.sh`,
  vacuous-until-committed + sha256-bound + the no-bounty-home-leak cross-check; `ci_check_node_path_fidelity.sh`,
  28-flag allow-list + no from-genesis constructor), but the C1 private-testnet dry-run EXECUTION is
  **`blocked_until_operator_c1_net_executed`**. The env-gated `node_c1_dry_run_rehearsal_live` harness
  (`ADE_LIVE_C1_DRY_RUN=1`) is a **RED test, skipped in CI, NOT a runtime node mode** and **not wired into any
  binary arm**; it produces a clearly-marked, NON-PROMOTABLE `PrivateRehearsalManifest` (`is_rehearsal` /
  `not_bounty_evidence` literals), **NOT bounty evidence**, and **flips NO RO-LIVE rule**. It is a **bounty
  DRY-RUN harness**, **DISTINCT** from the bounty operator pass (the C2 preprod leg below): private C1 acceptance
  ≠ bounty completion. It MUST NOT relax the relay-loop containment / served-chain handoff / live-feed memory
  fences, MUST NOT add a `--mode node` flag or a from-genesis constructor, and a manifest stays correlate-produced
  through `ba02_evidence::correlate` (the sole acceptance-evidence constructor), stored only under the rehearsal
  home.
- **CN-OPERATOR-EVIDENCE-01 / CN-CONS-06 / RO-LIVE-01** — the manifest schema is enforced (N-F-G-C added the
  BA-02 manifest-schema gate `ci_check_ba02_evidence_manifest_schema.sh`, vacuous-until-committed + sha256-bound),
  but C1 (private testnet) / C2 (preprod) operator-pass execution is `blocked_until_operator_pass_executed` /
  `blocked_until_operator_stake_available`. With CN-FORGE-04 (N-W), CN-WIRE-08 (N-X), CN-NODE-03 (N-F-F
  operator-key ingress), DC-EPOCH-03 (N-F-G-A forge fidelity), DC-NODE-06 (N-F-G-B self-accept→serve handoff)
  enforced, AND N-F-G-C's live feed + BA-02 evidence I/O wired, the producer forge composition is mechanically
  complete through the serve step AND the `--mode node` binary is forge-CAPABLE with real operator keys + real
  current constants + two fail-closed boundaries + a LIVE `--peer` feed (so the forge is observable at a due
  leader slot). The remaining blocker is the OPERATOR-PASS live leg itself — an operator-captured peer log
  through `correlate` against a peer that can grant leadership (RO-LIVE-01 / RO-LIVE-06).
- **RO-LIVE-06 (BA-02, N-F-C; I/O wired N-F-G-C)** — the evidence schema + correlator mechanics + the RED file
  I/O are enforced/wired, but a real BA-02 result is operator-gated. Synthetic fixtures CANNOT satisfy BA-02;
  the committed-manifest gate is vacuous-until-committed + sha256-bound. Distinct from RO-LIVE-01.

---

## Generation notes

- **Regenerated (single-slice follow-on refresh, PHASE4-N-AE.F) at HEAD `6363683e`** (`git rev-parse --short
  HEAD` — the *AE.F receive idempotency* commit), applied DIRECTLY to the on-disk SEAMS. The prior on-disk SEAMS
  was pinned at the PHASE4-N-AE close (`a76672b9` / **458** canonical types / **141** CI / **340** rules). This
  refresh folds in **PHASE4-N-AE.F** (the post-CE-A5 echo fix — receive idempotency at the durable-admit
  chokepoint, `DC-NODE-16`) and updates the counts to **458** canonical types / **142** CI / **341** rules.
  **N-AE.F added NO new ingress surface, NO new registry (closed or extensible), NO new version-gated contract, NO
  new closed enum, and NO new plugin/extension point** — it adds a single deterministic guard INSIDE the existing
  RED durable-admit chokepoint `pump_block`. **SEAMS is therefore MATERIALLY UNCHANGED from the N-AE pin — AE.F
  adds NO new seam** (the idempotency gate is an internal chokepoint guard, not an attach point). Registry → **341**
  (ONE NEW — `DC-NODE-16`, `tier = derived`, `introduced_in = "PHASE4-N-AE"`, `enforced`,
  `ci_script = ci/ci_check_receive_idempotency.sh`; no rule strengthened, no rule weakened); CI **141 → 142** (NET
  +1 NEW — `ci_check_receive_idempotency.sh`; 0 retired, 0 modified-in-place).
- **N-AE.F delta spot-checked at HEAD `6363683e` (grep/ls/git only — no `cargo`):** `git diff a76672b9..6363683e`
  touches **ONE** source `.rs` file — RED, already inventoried, with **ZERO new `struct`/`enum`:**
  `crates/ade_runtime/src/forward_sync/pump.rs` (the RED durable-admit chokepoint `pump_block`, which already does
  ChainDb/WAL I/O — its `//!`/header is RED). The added production lines are a `db.get_block_by_hash(&decoded.block_hash)`
  query immediately after `decode_block` and a gated `return Ok(None)` (when `stored.slot ==
  decoded.header_input.slot`), BEFORE the BLUE reducer step — a deterministic conditional + early return, NOT a new
  type. `git diff a76672b9..6363683e` over the BLUE `core_paths` trees touches **no** file and adds **zero** new
  `struct`/`enum` (verified by the change scope; 458 canonical types unchanged; the BLUE header authority
  `validate_and_apply_header` / `block_validity` is UNCHANGED and still fail-closes every block that reaches it).
  The NEW gate `ci_check_receive_idempotency.sh` is present; `ls ci/ci_check_*.sh | wc -l` = **142**; `grep -cE
  '^id = ' docs/ade-invariant-registry.toml` = **341** (the new `DC-NODE-16` present, `status = "enforced"`; 209
  enforced / 20 partial / 112 declared).
- **Cross-reference check (CODEMAP ↔ SEAMS ↔ registry) at the N-AE.F refresh — read honestly:** this SEAMS and the
  registry AGREE at HEAD `6363683e` (**458 / 142 / 341**: `ls ci/ci_check_*.sh | wc -l` = **142**; `grep -cE '^id =
  ' docs/ade-invariant-registry.toml` = **341**). The on-disk CODEMAP was **NOT regenerated** for AE.F — it stays
  pinned at the PHASE4-N-AE close (`a76672b9` / **458 / 141 / 340**) and does NOT carry `DC-NODE-16`. This is a
  deliberate, honest lag, **not a discipline gap**: AE.F adds NO crate, NO module, NO canonical type, and NO
  TCB-color change, so the module inventory + colors SEAMS reads from CODEMAP are unaffected (the one touched
  module, RED `ade_runtime::forward_sync::pump`, is already inventoried in the CODEMAP). The CODEMAP is a
  refresh-on-next-cluster item; it will pick up `DC-NODE-16` + the +1 CI gate + the `pump_block` idempotency-gate
  note at the next cluster close. No stale module references in this SEAMS; the lone module it newly cites for AE.F
  (`ade_runtime::forward_sync::pump`) is inventoried in the (lagging) CODEMAP.
- **Candidate seams surfaced for confirm/reject (this refresh):** **NONE — AE.F added none.** PHASE4-N-AE.F adds a
  hash-exact already-have idempotency guard INSIDE the existing RED durable-admit chokepoint `pump_block` (a
  deterministic `ChainDb` membership check returning `Ok(None)` for a re-announced block). It is **not** a new
  attach point: there is **no registration path** (no `register_*` / `add_*` / plugin trait / `Box<dyn Trait>` /
  handler map — it is a plain conditional + early return); it is **a tightening of an existing chokepoint**, not a
  new surface (`pump_block` is the SAME sole durable-admit / sole durable tip authority documented since N-Y /
  DC-NODE-12, and new work still attaches to the durable chain ONLY via that chokepoint + `bootstrap_initial_state`);
  and it introduces **no closed enum** (the decision is a conditional, not a sum type — the sketch's BLUE
  `ReceiveOutcome::AlreadyHave` variant was explicitly NOT taken). **No new ingress surface, no new closed/extensible
  registry, no new version-gated contract, no new extension/plugin point were introduced by N-AE.F; no
  human-judgment item is outstanding from this refresh.**
- **Regenerated (single-cluster CLUSTER-CLOSE refresh, PHASE4-N-AE) at HEAD `a76672b9`** (`git rev-parse
  --short HEAD` — the *AE.E chain-sync server FindIntersect cursor* commit, the PHASE4-N-AE close), applied
  DIRECTLY to the on-disk SEAMS. The prior on-disk SEAMS was pinned at the PHASE4-N-AC close (`1d54abb4` / **458**
  canonical types / **138** CI / **336** rules). This refresh folds in the **single closed cluster PHASE4-N-AE**
  (slices AE.A / AE.B / AE.C / AE.E — recover→serve continuity + forge admissibility) and updates the counts to
  **458** canonical types / **141** CI / **340** rules. **N-AE added NO new ingress surface, NO new registry
  (closed or extensible), NO new version-gated contract, and NO new plugin/extension point** — it TIGHTENS three
  existing authority paths (a forge-admissibility gate, `DC-NODE-15`; a proof-gated FindIntersect-only recovered-
  anchor serve projection, `DC-NODE-14`; a chain-sync server FindIntersect cursor fix, `DC-PROTO-10`). The closed
  `Point` enum and the chain-sync server wire grammar are UNCHANGED. **SEAMS is therefore MATERIALLY UNCHANGED
  from the N-AC pin** — this refresh re-pins HEAD/counts and records the three tightenings against the existing
  seams they touch. Registry → **340** (FOUR NEW — `DC-NODE-15` / `DC-CONS-24` / `DC-NODE-14` / `DC-PROTO-10`, all
  `introduced_in = "PHASE4-N-AE"`, all `enforced`; several strengthenings — `DC-NODE-13` / `CN-CONS-07` /
  `DC-CONS-23` / `DC-EPOCH-03` / `T-REC-05` / `DC-WAL-02` / `CN-CONS-06`; no rule weakened); CI **138 → 141** (NET
  +3 NEW — `ci_check_forge_followed_tip_admission.sh` / `ci_check_recovered_anchor_intersectable.sh` /
  `ci_check_recover_follow_wal_lineage.sh`; 0 retired, 0 modified-in-place; DC-PROTO-10 is test-enforced,
  `ci_script = ""`).
- **N-AE delta spot-checked at HEAD `a76672b9` (grep/ls/git only — no `cargo`):** `git diff 25ddeebd..a76672b9`
  touches **five** source `.rs` files — **TWO under the BLUE `core_paths`, with ZERO new `struct`/`enum` in either**:
  (1) `crates/ade_ledger/src/block_validity/header_input.rs` (its only added line is the read-only field
  `DecodedBlock.prev_hash`, populated from the already-parsed header — no decode/hash/determinism change), and
  (2) `crates/ade_network/src/chain_sync/server.rs` (BLUE — `chain_sync/` is a BLUE `core_paths` submodule + the
  file carries the `// Core Contract:` BLUE banner; the change is an existing-field write of `state.last_announced`,
  the AE.E cursor fix — `DC-PROTO-10`). The other three are RED/GREEN: `crates/ade_node/src/node_lifecycle.rs` +
  `crates/ade_node/src/node_sync.rs` (RED wiring + the GREEN-by-fn forge-admissibility classifier) +
  `crates/ade_runtime/src/network/served_chain_projection.rs` (RED serve projection). `git diff 25ddeebd..a76672b9`
  over the BLUE `core_paths` trees adds **zero** `^+(pub )?(struct|enum)` lines (verified mechanically; 458
  canonical types unchanged). The five new `(pub )?enum/struct` lines in the span — `NotCaughtUpReason`,
  `ForgeFollowedTipAdmission`, `ForgeRefused`, `NodeForgeOutcome`, `FollowedPeerTipSignal` — are ALL in RED
  `ade_node::node_sync` (closed error/signal sums, **NOT** canonical types, **NOT** registries, **NOT** wire
  grammar; 458 canonical types unchanged). `crates/ade_network/src/codec/chain_sync.rs` (the closed `Point` enum +
  chain-sync grammar) is **NOT in the diff** — DC-PROTO-10 is a server-internal `last_announced` cursor-state fix,
  not a grammar change. `served_chain_projection.rs` adds one private bounded helper
  (`earliest_servable_block_prev_hash`) + a second proof-gated branch to the EXISTING `ChainDbServedSource::intersect`
  (FindIntersect-only, serves no bytes) — a tightening of the existing read-only projection, not a new method or
  surface. The three new gates are present; `ls ci/ci_check_*.sh | wc -l` = **141**; `grep -cE '^id = '
  docs/ade-invariant-registry.toml` = **340** (the four new rules present; 208 enforced / 20 partial / 112
  declared).
- **Cross-reference check (CODEMAP ↔ SEAMS ↔ registry) at the N-AE close:** the on-disk CODEMAP was **regenerated
  at this same close** (`a76672b9` — **458** canonical types / **141** CI / **340** rules), so the **458 / 141 / 340**
  counts AGREE across CODEMAP, this SEAMS, and the registry (`ls ci/ci_check_*.sh | wc -l` = **141**; `grep -cE
  '^id = ' docs/ade-invariant-registry.toml` = **340**). Every module SEAMS references in the N-AE splice —
  `ade_ledger::block_validity::header_input`, `ade_network::chain_sync::server`, `ade_node::{node_lifecycle,
  node_sync}`, `ade_runtime::network::served_chain_projection` — is inventoried in the CODEMAP, and the CODEMAP's
  PHASE4-N-AE delta names the same four new rules (`DC-PROTO-10` / `DC-NODE-14` / `DC-NODE-15` / `DC-CONS-24`) + the
  same three new gates + the same 7 strengthenings (`DC-EPOCH-03` / `CN-CONS-06` / `CN-CONS-07` / `DC-WAL-02` /
  `DC-NODE-05` / `T-REC-05` / `DC-CONS-23`). **TCB-color cross-check (load-bearing): the CODEMAP carries an explicit
  "CLASSIFICATION CORRECTION" that the `chain_sync/server.rs` change (`DC-PROTO-10`) is BLUE** — `chain_sync/` is a
  BLUE `core_paths` submodule and the file opens with the `// Core Contract:` BLUE banner; this SEAMS follows that
  classification (the cluster-doc §6 prose called the AE.E reducer "GREEN", but CODEMAP + the file banner +
  `.idd-config.json` `core_paths` are the TCB-color authority, and they say BLUE). No stale module references; no
  drift vs CODEMAP unreconciled.
- **Candidate seams surfaced for confirm/reject (this cluster):** **ONE examined, REJECTED as a new seam.** The
  AE.B FindIntersect-only serve projection (`ChainDbServedSource::intersect`'s new proof-gated branch) was flagged
  as a possible extension point. It is **not** one: (1) **no registration path** — no `register_*` / `add_*` /
  plugin trait / `Box<dyn Trait>` / handler map; it is a single closed `match`/`for` over the already-closed
  `Point` enum inside one RED method. (2) **proof-gated + read-only, not open-within-constraints** — it advertises
  a point IFF the earliest servable `StoredBlock`'s `prev_hash` equals an offered point, serves NO bytes, and fails
  closed on recover-only / `PrevHash::Genesis`; that is a CLOSED behavior (like the §3 closed registries), the
  opposite of an extensible registry. (3) **it COMPLETES an existing seam** — the serve projection
  (`ChainDbServedSource`, the `--mode node` read-only serve view) is the SAME surface documented since N-U S3 /
  `DC-NODE-13`; AE.B closes the recovered-anchor clause of `DC-NODE-14` on it. It is therefore recorded as a
  TIGHTENING of the `DC-NODE-13` serve-projection surface (cross-referenced — the registry carries NO
  `strengthened_in += "PHASE4-N-AE"` on `DC-NODE-13`; the new `DC-NODE-14` recovered-anchor clause is the
  formally-introduced rule) plus a strengthening of `CN-CONS-07` / `DC-CONS-23`, **NOT** a new §3 registry and
  **NOT** a new §7 candidate. **No new ingress surface, no new closed/extensible registry, no new version-gated contract** were
  introduced by N-AE; **no human-judgment item is outstanding from this cluster.** (The Gap-1 multi-producer
  fork-choice is an explicitly-deferred separate FUTURE cluster — it is recorded under §7 as a not-yet-wired
  follow-on, not an N-AE seam.)
- **Regenerated (single-cluster CLUSTER-CLOSE refresh, PHASE4-N-AC) at HEAD `1d54abb4`** (`git rev-parse
  --short HEAD` — the *Close PHASE4-N-AC* commit), applied DIRECTLY to the on-disk SEAMS, downstream of the
  CODEMAP regenerated at the same close (`1d54abb4` — **458** canonical types / **138** CI / **336** rules). The
  prior on-disk SEAMS was pinned at the PHASE4-N-AB close (`c6e7fafe` / **458** / **137** / **335**). This refresh
  splices the **single closed cluster PHASE4-N-AC** (the RED producer signing shell evolves the operator KES key to
  the current period before signing — `DC-CRYPTO-10`) and updates the counts. **N-AC is RED-only — the net BLUE
  delta is ZERO new canonical types / authorities / fns / closed enums / version-gated contracts** (the two source
  files touched in the whole `c6e7fafe..1d54abb4` span are BOTH RED — `crates/ade_runtime/src/producer/producer_shell.rs`
  + `crates/ade_node/src/produce_mode.rs`; the BLUE Sum6KES algorithm + KES verifier + forge eligibility + wire
  rules reused unchanged). It COMPLETES / STRENGTHENS the existing RED KES signing seam (`CN-KES-HEADER-01`) — it
  opens **NO** new seam, plugin, closed enum, or version-gated contract. Registry → **336** (ONE NEW,
  `DC-CRYPTO-10`, `tier = derived`, `introduced_in = "PHASE4-N-AC"`, `enforced`,
  `ci_script = ci/ci_check_kes_evolution_before_sign.sh`; ONE strengthened — `CN-KES-HEADER-01`
  period-evolution-coverage; no rule weakened); CI **137 → 138** (NET +1: +1 NEW
  `ci_check_kes_evolution_before_sign.sh`; 0 retired, 0 modified-in-place).
- **N-AC delta spot-checked at HEAD `1d54abb4` (grep/ls/git only — no `cargo`):** `git diff c6e7fafe..1d54abb4`
  over the BLUE `core_paths` trees touches **no** file and adds **zero** `^+(pub )?(struct|enum|fn)` lines
  (RED-only — the two source files in the whole span are `crates/ade_runtime/src/producer/producer_shell.rs`
  (`//! RED`) + `crates/ade_node/src/produce_mode.rs` (`//! RED`), neither a BLUE `core_paths` entry). NEW RED
  method `ProducerShell::kes_sign_header_advancing(&mut self, period: u32, preimage: &UnsignedHeaderPreImage) ->
  Result<KesSignature, ShellSignError>` = `self.kes_advance_to(period)?` then `self.kes_sign_header(period,
  preimage)`; the forge's single real KES sign (`produce_mode::run_real_forge_inner` step 3) calls it instead of
  the raw `.kes_sign_header(`. `kes_advance_to` → `kes_update` is the EXISTING deterministic Sum6KES evolution
  (idempotent at the current period; fail-closed `EvolutionBackwards` / `EvolutionExhausted` — the existing
  `SigningError` variants, REUSED, not added). The NEW gate `ci_check_kes_evolution_before_sign.sh` is present;
  `ls ci/ci_check_*.sh | wc -l` = **138**; `grep -cE '^id = ' docs/ade-invariant-registry.toml` = **336** (the new
  `DC-CRYPTO-10` present). `kes_sign_header_advancing` is a RED `&mut self` method, **NOT canonical-counted** (458
  unchanged).
- **Cross-reference check (CODEMAP ↔ SEAMS) at the N-AC close:** the **458 / 138 / 336** counts match the CODEMAP
  header regenerated at the same close (`1d54abb4`) exactly. The two modules named in the N-AC splice —
  `ade_runtime::producer::producer_shell` (`kes_sign_header_advancing`, `kes_advance_to`) +
  `ade_node::produce_mode` (the rewired single real sign) — are inventoried there; the CODEMAP header's
  PHASE4-N-AC delta names the same one rule (`DC-CRYPTO-10`) + the +1 CI gate
  (`ci_check_kes_evolution_before_sign.sh`) + the same one strengthening (`CN-KES-HEADER-01`). No stale module
  references; no drift vs CODEMAP unreconciled.
- **Candidate seams surfaced for confirm/reject (this cluster):** **NONE NEW.** PHASE4-N-AC **completes /
  strengthens an already-closed RED seam** (the KES signing surface, CN-KES-HEADER-01) — it introduces no new
  ingress surface, no new closed / extensible registry (evolution + signing reuse the EXISTING closed `SigningError`
  variants `EvolutionBackwards` / `EvolutionExhausted` + the branded `KesSignature` / `UnsignedHeaderPreImage`; the
  Sum6KES `kes_update` evolution is the existing primitive), and no new version-gated contract. The evolve-before-sign
  composition has a single mechanical fence (`ci_check_kes_evolution_before_sign.sh`, DC-CRYPTO-10). No human-judgment
  item is outstanding from this cluster. (The honest-scope genesis-window / KES-period coupling — a from-genesis C1
  rehearsal cannot show forge-at-period-1 AND follower-adopt simultaneously; cross-period forge→adopt is the C2
  dense-tip path — is recorded under §6 Project-specific additions + §7 Pre-RO-LIVE hardening, and the two
  pre-existing fail-closed INFO items are handed to C2.)
- **Regenerated (single-cluster CLUSTER-CLOSE refresh, PHASE4-N-AB) at HEAD `c6e7fafe`** (`git rev-parse
  --short HEAD` — the *Close PHASE4-N-AB* commit), applied DIRECTLY to the on-disk SEAMS, downstream of the
  CODEMAP regenerated at the same close (`c6e7fafe` — **458** canonical types / **137** CI / **335** rules). The
  prior on-disk SEAMS was pinned at the PHASE4-N-AA close (`b0365df0` / **458** / **136** / **334**). This refresh
  splices the **single closed cluster PHASE4-N-AB** (outbound mux segmentation — `CN-SESS-05`) and updates the
  counts. **N-AB is GREEN-only — the net BLUE delta is ZERO new canonical types / authorities / fns / closed
  enums** (the ONLY source file touched in the whole `b0365df0..c6e7fafe` span is the GREEN session reducer
  `ade_network::session::core`; `mux::frame` and every other BLUE submodule reused unchanged). It COMPLETES the
  OUTBOUND direction of the existing closed wire-protocol session seam (the CN-SESS family) — it opens **NO** new
  seam, plugin, closed enum, or negotiated surface. Registry → **335** (ONE NEW, `CN-SESS-05`, `tier = derived`,
  `introduced_in = "PHASE4-N-AB"`, `enforced`, `ci_script = ci/ci_check_outbound_segmentation.sh`; TWO
  strengthened — `CN-SESS-04` receive+send-symmetry + `DC-SERVEMEM-01` bounded-serve-now-transmits-large-block; no
  rule weakened); CI **136 → 137** (NET +1: +1 NEW `ci_check_outbound_segmentation.sh`; 0 retired, 0
  modified-in-place).
- **N-AB delta spot-checked at HEAD `c6e7fafe` (grep/ls/git only — no `cargo`):** `git diff b0365df0..c6e7fafe`
  over the BLUE `core_paths` trees adds **zero** `^+(pub )?(struct|enum|fn)` lines (GREEN-only — the ONLY source
  file in the whole span is `crates/ade_network/src/session/core.rs`, a GREEN `session::core` submodule, NOT a
  BLUE `core_paths` entry). NEW fixed GREEN `const MAX_OUTBOUND_PAYLOAD_BYTES: usize = 16 * 1024 * 1024` at
  `crates/ade_network/src/session/core.rs:60`; `handle_outbound` segments via `payload.chunks(MAX_PAYLOAD)` at
  `:364` and fails closed above the bound at `:339`; `encode_inner_frame` (at `:404`) keeps its strict
  `MAX_PAYLOAD` guard at `:410-411` and remains the SOLE per-frame encoder wrapping the single
  `ade_network::mux::frame::encode_frame` authority (`mux/frame.rs:87`, unchanged in span). `SessionError::OutboundPayloadTooLarge { len }`
  at `crates/ade_network/src/session/event.rs:177` is the EXISTING closed variant (REUSED, not added). The NEW
  gate `ci_check_outbound_segmentation.sh` is present; `ls ci/ci_check_*.sh | wc -l` = **137**;
  `grep -cE '^id = ' docs/ade-invariant-registry.toml` = **335** (the new `CN-SESS-05` present).
  `MAX_OUTBOUND_PAYLOAD_BYTES` is a GREEN const, **NOT canonical-counted** (458 unchanged).
- **Cross-reference check (CODEMAP ↔ SEAMS) at the N-AB close:** the **458 / 137 / 335** counts match the CODEMAP
  header regenerated at the same close (`c6e7fafe`) exactly. The one module named in the N-AB splice —
  `ade_network::session::core` (`handle_outbound`, `MAX_OUTBOUND_PAYLOAD_BYTES`, the reused `encode_inner_frame`)
  — is inventoried there; the CODEMAP header's PHASE4-N-AB delta names the same one rule (`CN-SESS-05`) + the +1
  CI gate (`ci_check_outbound_segmentation.sh`) + the same two strengthenings (`CN-SESS-04` / `DC-SERVEMEM-01`).
  No stale module references; no drift vs CODEMAP unreconciled.
- **Candidate seams surfaced for confirm/reject (this cluster):** **NONE NEW.** PHASE4-N-AB **completes the
  OUTBOUND direction of an already-closed seam** (the CN-SESS wire-protocol session reducer) — it introduces no
  new ingress surface, no new closed / extensible registry (segmentation reuses the closed
  `SessionError::OutboundPayloadTooLarge` + the `AcceptedMiniProtocol` registry + the single `encode_inner_frame`
  authority; `MAX_OUTBOUND_PAYLOAD_BYTES` is a fixed non-configurable GREEN const), and no new version-gated
  contract. The outbound segmentation + the fixed bound have a single mechanical fence
  (`ci_check_outbound_segmentation.sh`, CN-SESS-05). No human-judgment item is outstanding from this cluster. (The
  ONE open obligation — confirming a real cardano-node demux accepts the reused-per-segment SDU timestamp — is the
  live leg, correctly fenced behind the unflipped RO-LIVE obligations as pre-RO-LIVE hardening item 4.)

- **Regenerated (single-cluster CLUSTER-CLOSE refresh, PHASE4-N-AA) at HEAD `b0365df0`** (`git rev-parse
  --short HEAD` — the *Close PHASE4-N-AA* commit), applied DIRECTLY to the on-disk SEAMS, downstream of the
  CODEMAP regenerated at the N-AA close (`5c9f6cf6` — **458** canonical types / **136** CI / **334** rules;
  `b0365df0` is the close commit one past it, with no further code/type/gate change). The prior on-disk SEAMS
  was pinned at the post-PHASE4-N-U gate-hygiene span (`999199f8` / **458** / **135** / **333**). This refresh
  splices the **single closed cluster PHASE4-N-AA** (bounded peer-driven serve range — `DC-SERVEMEM-01`) and
  updates the counts. **N-AA is RED-only — the net BLUE delta is ZERO new canonical types / authorities / fns**
  (the lone BLUE-tree touch in the close span is a `// Core Contract:` comment header on
  `ade_ledger/src/block_validity/mod.rs`, no type/authority/export change). It BOUNDS the existing closed
  DC-NODE-13 durable-chain serve seam — it opens **NO** new seam, plugin, or negotiated surface. Registry →
  **334** (ONE NEW, `DC-SERVEMEM-01`, `tier = derived`, `introduced_in = "PHASE4-N-AA"`, `enforced`,
  `ci_script = ci/ci_check_serve_range_bounded.sh`; TWO strengthened — `DC-NODE-13` serve-now-bounded +
  `DC-LIVEMEM-01` symmetric-serve-side cross-ref; no rule weakened); CI **135 → 136** (NET +1: +1 NEW
  `ci_check_serve_range_bounded.sh`; 0 retired, 0 modified-in-place).
- **N-AA delta spot-checked at HEAD `b0365df0` (grep/ls/git only — no `cargo`):** the close span over the BLUE
  `core_paths` trees adds **zero** `^+(pub )?(struct|enum|fn)` lines (RED-only — the lone BLUE-tree touch is
  the `// Core Contract:` comment header on `ade_ledger/src/block_validity/mod.rs`). NEW closed RED `pub enum
  ServeRangeOutcome { Served(Vec<(SlotNo, Hash32, Vec<u8>)>), Empty, CapExceeded, ReadError }` at
  `crates/ade_runtime/src/network/served_chain_projection.rs:57`, with the FIXED `const MAX_SERVE_RANGE_BLOCKS:
  usize = 256` at `:50`; NEW closed RED `pub struct CappedSlotRange { blocks: Vec<(SlotNo, Vec<u8>)>,
  truncated: bool }` at `crates/ade_runtime/src/chaindb/types.rs:46`; NEW `ChainDb` trait methods
  `range_bytes_capped` (`crates/ade_runtime/src/chaindb/mod.rs:103`) + `last_block_bytes` (`:114`). The serve
  reads via `range_bytes_capped` (`served_chain_projection.rs:102`) + the O(log N) `last_block_bytes` (`:198`)
  — **0** `iter_from_slot` / `chaindb.tip()` calls remain on the serve path (the only `tip()` is the serve's own
  `ServedHeaderLookup::tip()` in a test assertion). The NEW gate `ci_check_serve_range_bounded.sh` is present;
  `ls ci/ci_check_*.sh | wc -l` = **136**; `grep -cE '^id = ' docs/ade-invariant-registry.toml` = **334** (the
  new `DC-SERVEMEM-01` present). These RED storage/serve value types are **NOT canonical-counted** (458
  unchanged).
- **Cross-reference check (CODEMAP ↔ SEAMS) at the N-AA close:** the **458 / 136 / 334** counts match the
  CODEMAP header regenerated at the same close exactly. Every module named in the N-AA splice appears in that
  CODEMAP — `ade_runtime::network::served_chain_projection (ChainDbServedSource, ServeRangeOutcome,
  MAX_SERVE_RANGE_BLOCKS)`, `ade_runtime::chaindb::{types (CappedSlotRange), mod (range_bytes_capped /
  last_block_bytes)}` — all inventoried there; the CODEMAP header's PHASE4-N-AA delta names the same one rule
  (`DC-SERVEMEM-01`) + the +1 CI gate + the same two strengthenings (`DC-NODE-13` / `DC-LIVEMEM-01`). No stale
  module references; no drift vs CODEMAP unreconciled.
- **Candidate seams surfaced for confirm/reject (this cluster):** **NONE NEW.** PHASE4-N-AA is a defensive
  **bounding of an already-closed seam** (the DC-NODE-13 durable-chain serve) — it introduces no new ingress
  surface, no new closed/extensible registry beyond the closed RED value types (`ServeRangeOutcome` /
  `CappedSlotRange`) and the fixed non-configurable cap (`MAX_SERVE_RANGE_BLOCKS`), and no new version-gated
  contract. The serve-read primitives + cap have a single mechanical fence (`ci_check_serve_range_bounded.sh`,
  DC-SERVEMEM-01). No human-judgment item is outstanding from this cluster.

- **Focused single-span refresh (post-PHASE4-N-U gate hygiene) at HEAD `999199f8`** (`git rev-parse --short
  HEAD`), applied DIRECTLY to the on-disk SEAMS. Brings SEAMS current from its last pin (`4e358e92`, the
  PHASE4-N-U close-doc refresh) to HEAD, splicing the span `4e358e92..999199f8` (commits `60deecf3` DC-NODE-06
  handoff-fence repoint, `e92b40b7` `ci_check_no_secrets.sh` ARG_MAX/IP-false-positive fix, `999199f8` 10
  gate-vs-code drift repairs; the `7f00e75d` *Close PHASE4-N-U* commit in the span is administrative bookkeeping —
  the SEAMS-material content already landed in the `4e358e92` close-doc refresh). **CODEMAP was intentionally NOT
  regenerated** — structurally current, unchanged inventory, still pinned at `4e358e92`; read as-is. **Counts
  UNCHANGED across the span: 458 canonical types / 135 CI / 333 rules.** Verified gate-set unchanged:
  `git diff --name-status 4e358e92..999199f8 -- 'ci/ci_check_*.sh'` shows 11 gates, ALL `M` (modified) — zero
  added/deleted; `ls ci/ci_check_*.sh | wc -l` = **135**; `grep -c '^[[]rules[]][]]' docs/ade-invariant-registry.toml`
  = **333** (baseline and HEAD identical). The two source-file touches in the span
  (`ade_ledger/src/block_validity/mod.rs`, `ade_runtime/src/seed_import/importer.rs`) are pure doc-comment hygiene
  (stale comments reconciled to the code a gate greps) — no semantic code change, no invariant weakened.
- **The ONE material SEAMS delta: the DC-NODE-06 handoff-fence REPOINT (`60deecf3`).**
  `ci/ci_check_served_chain_handoff_fence.sh` is REPOINTED (not retired — CI count stays 135): it no longer fences
  the G-B *“`SelfAcceptedHandoff` → `push_atomic` accumulator”* handoff (N-U S3 RETIRED that mechanism for
  serve-as-projection, DC-NODE-13), and now fences the EVOLVED DC-NODE-06 durable-provenance serve — the `--mode
  node` spine serve sources ONLY `ServedChainSource::DurableChainDb` over `Arc<dyn ChainDb>`, with NO retired
  non-durable serve ingress (no `push_atomic` / `served_chain_admit` / `ServedChainHandle` / `SelfAcceptedHandoff`
  channel). DC-NODE-06's deeper *only-validated-bytes-served* invariant is preserved + strengthened (now survives
  restart). The registry was already reconciled at `60deecf3` (DC-NODE-06 `ci_script += ci_check_served_chain_projection.sh`;
  `code_locus` repointed; `strengthened_in += PHASE4-N-U`; rule count UNCHANGED at 333) — this pass reconciles the
  grounding-doc PROSE descriptions, as that registry note directs. Updated CURRENT-state surfaces: §1 *self-accept→serve
  handoff* (supersession banner), §2 *self-accept→serve handoff* domain (supersession banner), §3 `SelfAcceptedHandoff`
  registry entry (durable-provenance / `--mode produce`-retained framing), §5 gate-table row (repointed checks). The
  `SelfAcceptedHandoff` carrier is RETAINED for `--mode produce` (CN-PROD-04). **Historical references to this gate
  in past-cluster (G-B/G-C/G-D/G-E/G-H) delta sections are pin-correct for their spans and LEFT as-is** — only the
  CURRENT-state descriptions were updated.
- **Green-means-green: the full CI sweep is now 135 / 0 GREEN** (`ls ci/ci_check_*.sh | wc -l` = 135, every gate
  passes after the `999199f8` drift repairs + the `e92b40b7` no_secrets fix).
- **Candidate seams surfaced for confirm/reject (this span):** NONE NEW. The span introduced no new surface, no
  new closed/extensible registry, no new version-gated contract; the DC-NODE-06 repoint is a description/provenance
  reconciliation of an ALREADY-CLOSED seam (the durable-projection serve, classified at the N-U close under §2
  *forged-block serving* + §3 Closed `ServedChainSource` + §3 Extensible second-impl-of-closed-trait). No
  human-judgment item is outstanding from this span.

- Regenerated (single-cluster CLUSTER-CLOSE refresh, PHASE4-N-U) at HEAD `4e358e92`
  (`git rev-parse --short HEAD`), downstream of the CODEMAP regenerated at the same HEAD (**135** CI checks,
  **333** rules, **458** canonical types). The prior on-disk SEAMS was pinned at the **PHASE4-N-F-G-K…G-R
  catch-up** (`65954fa3` / 134 CI / 328 rules / 458 canonical types). This refresh splices the **single closed
  cluster PHASE4-N-U** (forged-block durability — full producer own-tip advance) and updates the counts.
  **The net BLUE delta is ZERO new canonical types + ZERO new BLUE authorities — verified NO new extensible /
  negotiated / plugin surface** (N-U realizes a SECOND impl of the ALREADY-CLOSED serve trait seams + a closed
  read-source enum). The lone BLUE touch is the additive fn
  `ade_ledger::block_validity::header_input::block_header_bytes(&[u8])`, factored out of
  `accepted_block_header_bytes` (which now delegates) — the SAME DC-CONS-18 header-projection authority, no new
  type, no parallel splitter. Registry → **333** (FIVE NEW, all `introduced_in = "PHASE4-N-U"`, all `enforced`:
  `DC-NODE-12`, `DC-CONS-23`, `DC-WAL-04`, `T-REC-05` (tier = true, test-enforced — `ci_script = ""`),
  `DC-NODE-13`; TWO strengthened: `CN-CONS-07` serve-provenance, `DC-NODE-11` mechanism superseded; no rule
  weakened); CI 134 → **135** (NET +1: +2 NEW `ci_check_forged_durable_admit_via_pump.sh` +
  `ci_check_served_chain_projection.sh`; −1 RETIRED `ci_check_served_chain_stability.sh`; N-U also extended
  `ci_check_node_run_loop_containment.sh` in place — no count change).
- **N-U delta spot-checked at HEAD `4e358e92` (grep/ls/git only — no `cargo`):**
  `git diff --name-only 65954fa3..4e358e92 -- crates/` touches exactly two BLUE `core_paths` files
  (`ade_ledger/src/block_validity/{header_input.rs, mod.rs}`), and that BLUE diff adds **zero**
  `^+(pub )?(struct|enum)` lines (the BLUE touch is the additive `pub fn block_header_bytes(block_cbor: &[u8])
  -> Result<&[u8], BlockValidityError>` at `header_input.rs:76`; `accepted_block_header_bytes` now delegates
  `block_header_bytes(accepted.as_bytes())` at `:58`). NEW RED `crates/ade_runtime/src/network/served_chain_projection.rs`
  (`//! RED`) `pub struct ChainDbServedSource` with `impl ServedHeaderLookup` (line 60) + `impl ServedRangeLookup`
  (line 124); the closed `pub enum ServedChainSource<'a> { Snapshot(&ServedChainView), DurableChainDb(&dyn
  ChainDb) }` at `crates/ade_runtime/src/network/serve_dispatch.rs:55`, read by the single
  `pub async fn dispatch_server_frame_event_to_outbound` (line 137; `match` arms for both variants, no wildcard).
  `crates/ade_node/src/node_sync.rs` the fenced `admit_forged_block_durably` routing through
  `forward_sync::pump_block` (line 360) + the recovery test
  `forge_kill_then_warm_start_recovers_same_tip_via_forward_replay` (`:1779`);
  `crates/ade_node/src/node_lifecycle.rs` the orphan-drop test `warm_start_drops_orphan_block_above_wal_tail`
  (`:2502`) + the `serve_gate_admits` RETIREMENT comments (`:276`, `:1767`) — the predicate + its in-module test
  are GONE from source. `ServedChainLookups` (the produce-mode snapshot impl of the same seams) is unchanged at
  `crates/ade_runtime/src/producer/served_chain_lookups.rs:31` (`impl ServedHeaderLookup` `:35`, `impl
  ServedRangeLookup` `:81`). The two NEW gates (`ci_check_forged_durable_admit_via_pump.sh`,
  `ci_check_served_chain_projection.sh`) present; the RETIRED `ci_check_served_chain_stability.sh` is ABSENT;
  `ls ci/ci_check_*.sh | wc -l` = **135**; `grep -cE '^id = ' docs/ade-invariant-registry.toml` = **333** (the
  five new rule IDs `DC-NODE-12`/`DC-WAL-04`/`T-REC-05`/`DC-CONS-23`/`DC-NODE-13` present). `run_relay_loop`
  containment is allow-list-EXTENDED (the one driver call) but otherwise byte-unchanged; the served-chain handoff
  fence is byte-unchanged.
- **Cross-reference check (CODEMAP ↔ SEAMS) at `4e358e92`:** the **458 / 135 / 333** counts match the CODEMAP
  header regenerated at the same HEAD exactly. Every module named in the N-U splice appears in that CODEMAP —
  `ade_ledger::block_validity::header_input (block_header_bytes)`, the NEW RED
  `ade_runtime::network::served_chain_projection (ChainDbServedSource)`, the closed
  `ade_runtime::network::serve_dispatch (ServedChainSource)`, `ade_node::{node_sync
  (admit_forged_block_durably), node_lifecycle (warm-start recovery)}` — all inventoried there; the CODEMAP
  header's PHASE4-N-U delta names the same five rules + the +2/−1 CI gates + the same RETIRED G-R
  `serve_gate_admits` mechanism. No stale module references; no drift vs CODEMAP unreconciled. **Honest drift
  (carried from the CODEMAP close note):** the S2 §8-named gate `ci_check_forged_tip_recovery.sh` + the §8-named
  test `forge_two_clean_runs_byte_identical` were NOT created literally — T-REC-05 is test-enforced via the
  kill-recover fingerprint-equality tests named above (`T-REC-05.ci_script = ""`); the invariants are enforced,
  the §8 CE artifact NAMES drifted during S2. **The carried G-K…G-R / G-J Generation-notes entries below are
  retained verbatim at their own pins (`65954fa3` / `550eec3a`) and are NOT rewritten — they describe those spans
  correctly; the historical `134` / `328` / `65954fa3` references inside the carried delta narratives + carried
  notes are pin-correct for their spans, not stale.**
- **`.idd-config.json` is NOT edited by this doc.** Its `head_deltas_baseline` is `65954fa3` (the G-K…G-R
  baseline) — the next `/head-deltas` measures the N-U span from there. The authoritative counts are the CODEMAP
  header + `grep -cE '^id = ' docs/ade-invariant-registry.toml` (= 333) + `ls ci/ci_check_*.sh | wc -l` (= 135);
  per the task constraint this regeneration does NOT modify `.idd-config.json`.
- NOTE: no `cargo build`/`test`/`check` was run during this regeneration (grep/ls/git only, per the task
  constraint).
- (Prior multi-cluster CATCH-UP entry, carried below.)
- Regenerated (multi-cluster CATCH-UP refresh, PHASE4-N-F-G-K…G-R + C1 evidence) at HEAD `65954fa3`
  (`git rev-parse --short HEAD`), downstream of the CODEMAP regenerated at the same HEAD (**134** CI checks,
  **328** rules, **458** canonical types). The prior on-disk SEAMS was pinned at the **PHASE4-N-F-G-J close**
  (`550eec3a` / 126 CI / 319 rules / 457 canonical types). This refresh splices the **eight closed clusters
  G-K → G-R + the C1 genesis-successor rehearsal evidence** and updates the counts. **The WHOLE span adds ONLY
  closed sums + version-gated fields — verified NO extensible / negotiated / plugin surface.** Net BLUE delta:
  **ONE** new canonical type (`ArrayHead = Definite(u64) | Indefinite`, `ade_network::codec::primitives`, G-M —
  457 → 458), **two** additive BLUE fns (`encode_n2n_version_params`, G-L; `decode_array_head_two_form`, G-M),
  and **one** version-gated BLUE field (`SeedEpochConsensusInputs.epoch_nonce` + `SEED_CINPUT_SCHEMA_VERSION 1 →
  2`, G-N). Registry → **328** (NEW `DC-NODE-09`, `CN-WIRE-10`, `CN-WIRE-11`, `T-REC-04`, `DC-CINPUT-03`,
  `CN-WIRE-12`, `DC-CINPUT-04`, `DC-NODE-10`, `DC-NODE-11` — all `enforced`; no rule weakened); CI 126 → **134**
  (one new gate per cluster; the C1-evidence commit broadened `ci_check_rehearsal_manifest_schema.sh` in place —
  no count change).
- **G-K…G-R delta spot-checked at HEAD `65954fa3` (grep/ls/git only — no `cargo`):**
  `git diff --name-only 550eec3a..65954fa3 -- crates/` touches the BLUE `core_paths` files
  `ade_network/src/{handshake/version_table.rs, chain_sync/server.rs, codec/chain_sync.rs, codec/primitives.rs}` +
  `ade_ledger/src/{seed_consensus_inputs.rs, consensus_view.rs}` (the rest RED/GREEN/test).
  `crates/ade_network/src/codec/primitives.rs` `pub enum ArrayHead { Definite(u64), Indefinite }` (line 64) +
  `pub fn decode_array_head_two_form` (line 73); `crates/ade_network/src/handshake/version_table.rs`
  `pub fn encode_n2n_version_params(version: u16, network_magic: u32) -> VersionParams` (line 88);
  `crates/ade_network/src/chain_sync/server.rs` `IntersectFound { point, tip }` variant + `Origin`-reply test;
  `crates/ade_ledger/src/seed_consensus_inputs.rs` `pub const SEED_CINPUT_SCHEMA_VERSION: u32 = 2` (line 42) +
  `pub epoch_nonce: Nonce` (line 61) + the decoder rejecting `version != 2` as `UnknownVersion` (a v1-shape test
  pins the fail); `crates/ade_runtime/src/admission/wire_pump.rs` strips the BlockFetch tag-24 wrapper via the
  single `ade_codec::unwrap_tag24` authority before decode (CN-WIRE-12, with a fail-closed test);
  `crates/ade_node/src/node_lifecycle.rs` `pub fn serve_gate_admits(highest_served_block_no: Option<u64>,
  candidate_block_no: u64) -> bool` (line 281, DC-NODE-11) + the feed header-validation view via
  `PoolDistrView::from_seed_epoch_consensus_inputs` (DC-CINPUT-04); `crates/ade_node/src/node_sync.rs` the
  forge-successor evolved-spine position (DC-NODE-10). The eight NEW gates
  (`ci_check_node_serve_lifetime.sh`, `ci_check_n2n_handshake_versiondata_authority.sh`,
  `ci_check_chainsync_findintersect_compat.sh`, `ci_check_warmstart_eta0_overlay.sh`,
  `ci_check_feed_tag24_unwrap.sh`, `ci_check_feed_leader_threshold_view.sh`,
  `ci_check_forge_successor_evolved_spine.sh`, `ci_check_served_chain_stability.sh`) all present;
  `ls ci/ci_check_*.sh | wc -l` = **134**; `grep -cE '^id = ' docs/ade-invariant-registry.toml` = **328** (the
  nine new rule IDs present). `run_relay_loop` containment + the served-chain handoff fence are byte-unchanged.
- **Cross-reference check (CODEMAP ↔ SEAMS) at `65954fa3`:** the **458 / 134 / 328** counts match the CODEMAP
  header regenerated at the same HEAD exactly. Every module named in the G-K…G-R splices appears in that CODEMAP —
  `ade_network::{codec::primitives (ArrayHead, decode_array_head_two_form), handshake::version_table
  (encode_n2n_version_params), chain_sync::server (Origin reply)}`, `ade_ledger::seed_consensus_inputs
  (epoch_nonce, SEED_CINPUT_SCHEMA_VERSION = 2)`, `ade_runtime::admission::wire_pump (feed tag-24 unwrap)`,
  `ade_node::{node_lifecycle (serve_gate_admits, feed view), node_sync (forge-successor)}` — all inventoried
  there; the CODEMAP header's per-cluster G-K…G-R deltas name the same rules + gates. No stale module references;
  no drift vs CODEMAP unreconciled. (The one carried stale `456` in the §4 Frozen wire-format bullet was corrected
  to `458` this refresh.)
- **`.idd-config.json` is NOT edited by this doc.** Its `_invariant_registry_doc` still reads "321 entries at
  HEAD" and `head_deltas_baseline` is `853344f7` (the G-K S1 baseline) — both **stale vs the true HEAD** (328
  rules at `65954fa3`); a `/head-deltas` / config refresh is the recommended follow-on, but per the task
  constraint this regeneration does **NOT** modify `.idd-config.json` (the authoritative counts are the CODEMAP
  header + `grep -cE '^id = ' docs/ade-invariant-registry.toml`, both = 328).
- NOTE: no `cargo build`/`test`/`check` was run during this regeneration (grep/ls/git only, per the task
  constraint).
- (Prior G-J entry, carried below.)
- Regenerated (scoped DELTA-REFRESH at the PHASE4-N-F-G-J close) at HEAD `550eec3a` (`git rev-parse --short
  HEAD`), downstream of the CODEMAP regenerated at the same HEAD (**126** CI checks, **319** rules, **457**
  canonical types). The prior on-disk SEAMS body was generated at the **PHASE4-N-F-G-D close** (`6bd60c80` / 121
  CI / 315 rules). This refresh splices the **PHASE4-N-F-G-J** closed-surface deltas (five slices — S1 the closed
  GREEN `NodeSchedEvent` emit-only scheduling vocabulary + `ci_check_node_sched_events_emit_only.sh`; S2 the
  closed BLUE sum `PrevHash` + the POSITION-BLIND `$hash32/null` codec + `ci_check_prevhash_single_wire_authority.sh`;
  S3 the NEW BLUE single POSITION-AWARE authority `ade_ledger::block_validity::header_position::check_header_position`
  + a real genesis-successor forge; S4 the node-spine cold-start reachability +
  `ci_check_genesis_successor_reachability.sh`; S5 the C1 genesis rehearsal harness, reuse-only) and updates the
  counts. **G-J adds ONLY closed sums + one NEW BLUE module — verified NO extensible / negotiated surface.** ONE
  NEW BLUE canonical type (`PrevHash`, 456 → 457); ONE NEW BLUE module (`header_position`, a fn + a variant); the
  closed GREEN `live_log::{sched_event, sched_writer}` vocabulary; two additive closed-enum variants
  (`BlockValidityError::HeaderPositionInvalid`, `NodeForgeError::RecoveredTipMissingBlockNo`). Registry → **319**
  (NEW `CN-WIRE-09` + `DC-NODE-08` + `CN-NODE-04`, all `tier = derived`, `enforced`; `CN-REHEARSAL-FIDELITY-01`
  `strengthened_in += "PHASE4-N-F-G-J"`; no rule weakened); CI 121 → **126** (the three NEW gates above; the three
  containment / handoff / memory fences byte-unchanged; `ci_check_rehearsal_manifest_schema.sh` EXTENDED in place
  for the S5 genesis-rehearsal home).
- **G-J delta spot-checked at HEAD `550eec3a` (grep/ls only — no `cargo`):** `crates/ade_types/src/shelley/block.rs`
  `pub enum PrevHash { Genesis, Block(Hash32) }` (NOT `#[non_exhaustive]`); `crates/ade_codec/src/shelley/block.rs`
  `fn decode_prev_hash`; `crates/ade_ledger/src/block_validity/header_position.rs` `pub fn check_header_position`
  + `BlockValidityError::HeaderPositionInvalid` (folding into `BlockRejectClass::HeaderInvalid` in `verdict.rs`,
  no new class); `crates/ade_node/src/live_log/sched_event.rs` `pub enum NodeSchedEvent` (5) + `pub enum FeedReason`
  (`NoBlockAvailable`/`CleanEmpty`/`UnknownDisconnected`) + `pub enum ForgeOutcome` (4), all closed, EMIT-ONLY;
  `crates/ade_node/src/node_sync.rs` `fn forge_header_position` + `NodeForgeError::RecoveredTipMissingBlockNo`;
  `crates/ade_node/src/node_lifecycle.rs` `fn may_cold_start_forge`. The three NEW gates
  (`ci_check_node_sched_events_emit_only.sh`, `ci_check_prevhash_single_wire_authority.sh`,
  `ci_check_genesis_successor_reachability.sh`) present; `ls ci/ci_check_*.sh | wc -l` = **126**;
  `grep -cE '^id = ' docs/ade-invariant-registry.toml` = **319** (`CN-WIRE-09` / `DC-NODE-08` / `CN-NODE-04`
  present). The unaffected G-A..G-E surfaces were NOT re-walked (per the delta-refresh constraint).
- **G-H GAP (load-bearing — honest cross-reference):** this SEAMS body was last fully regenerated at the **G-D**
  close and skipped the intervening **PHASE4-N-F-G-H** cluster. The header counts (457 / 126 / 319) are the true
  HEAD values and include G-H's contributions (the BLUE fn `n2n_supported_for_magic`; the NEW RED
  `ade_runtime::network::serve_dispatch`; `ade_node::node_lifecycle::{run_node_serve_task, bind_serve_listener}` +
  the closed `ServeStartError`; `DC-NODE-07`; the gates `ci_check_single_serve_dispatch_authority.sh` +
  `ci_check_serve_listener_magic_aware.sh`), but the §1 serve-to-peer surface, the §3 `ServeStartError` row, and
  the §5 serve-gate rows are **NOT** spliced into the bodies below. A fuller G-H surface catch-up is the
  recommended follow-on; the CODEMAP carried delta documents G-H fully in the meantime.
- (Prior G-D entry, carried.) Regenerated (scoped INCREMENTAL catch-up through ONE cluster) at HEAD `6bd60c80`
  (`git rev-parse --short HEAD`), downstream of the CODEMAP regenerated at the same HEAD (121 CI checks, 315
  rules, 456 canonical types).
  The prior on-disk SEAMS was generated at the **PHASE4-N-F-G-E close** (`6f848825` / 119 CI checks / 314 rules —
  live-feed bounded memory before authoritative decode/apply on the `--mode node` spine). This refresh catches it
  up through **PHASE4-N-F-G-D** (a private-testnet accepted-block bounty DRY-RUN harness on the `--mode node`
  spine, closing now at `6bd60c80`): four slices — S1 `d4d0f456` (path-fidelity proof + the NEW
  `ci_check_node_path_fidelity.sh` fence), S2 `459cf78d` (the NEW GREEN `ade_node::rehearsal_evidence`
  `PrivateRehearsalManifest` + the NEW RED `ade_node::rehearsal_pass` file I/O + the NEW
  `ci_check_rehearsal_manifest_schema.sh` gate), S3 `076a5af5` (the C1 dry-run runbook + the env-gated operator
  scaffold), S4 `6bd60c80` (a close-surfaced security fix hardening the rehearsal leak gate against an
  archived-home leak); doc commits `a8003dc8` / `93472991`.
- **N-F-G-D deltas are CLOSED / constructor-fenced ATTACH POINTS + two NEW fences, NOT new extension points
  (load-bearing).** The NEW GREEN `PrivateRehearsalManifest` is a constructor-fenced non-promotable envelope (sole
  ctor `from_correlate_outcome`, `None` on `NoEvidence`; `is_rehearsal`/`not_bounty_evidence` literals; closed
  1-variant `RehearsalVenue`) WRAPPING the existing closed `Ba02Manifest`; the NEW RED `rehearsal_pass` is file
  I/O that REUSES `ba02_pass::correlate_peer_log_file` **verbatim** (no alternate correlator) — both classified
  under §3 Closed. **No new module beyond these two `ade_node` modules; no new BLUE authority / canonical type; no
  new `--mode node` argv flag (the `cli.rs` flag set is the pinned 28-flag closed allow-list); no new
  `NodeBlockSource` / `CoordinatorEvent` / `Mode` variant; no from-genesis consensus-inputs constructor; no binary
  wiring.** **No BLUE crate was modified** — the 456 canonical-type total is unchanged (the NEW
  `ade_node::rehearsal_evidence` is GREEN-by-content and the NEW `ade_node::rehearsal_pass` is RED — neither is
  canonical-counted; both live in `ade_node`, not a BLUE `core_paths` entry). `correlate` stays the SOLE
  `Ba02Manifest` constructor. **`run_node_sync` / `run_relay_loop` containment + the served-chain handoff fence +
  the live-feed memory bounds are byte-unchanged.** _(N-F-G-E deltas — the two closed memory caps in front of the
  decode path — and the N-F-G-C live WirePump feed + `ba02_pass` evidence I/O are carried forward unchanged; G-D
  changed none of them.)_
- **N-F-G-D RETIRES NOTHING in §7 and BROADENS NOTHING (load-bearing).** G-D adds a path-faithful, non-promotable
  bounty DRY-RUN **harness** — NOT the bounty deliverable — and does NOT close any §7 candidate. It adds **one new
  operator-pass execution gate** (the **C1 private-testnet dry-run**), operator-gated
  (`blocked_until_operator_c1_net_executed`), exercised only by the env-gated RED test
  `node_c1_dry_run_rehearsal_live` (`ADE_LIVE_C1_DRY_RUN=1`, skipped in CI, NOT a runtime node mode), producing a
  clearly-marked rehearsal manifest, NOT bounty evidence, **flipping no RO-LIVE rule** — DISTINCT from the bounty
  operator pass (the C2 preprod leg). Candidate **#6 stays RETIRED** (closed by G-E). **The OTHER §7 candidates
  are carried UNCHANGED and were NOT broadened:** #0 (live network serve + operator-peer — partly closed at G-C;
  operator-witnessed ACCEPT still the gating follow-on, RO-LIVE-01 partial / RO-LIVE-06 live BA-02 not claimed),
  #1 (live unbounded peer → observable forge ACCEPT), #2 (live BA-02), #3 (RO-MITHRIL-IMPORT-01 remaining), #4
  (N-U forged-block durability), #5 (RO-SYNC-EVIDENCE-01 live leg).
- **Boundary honesty (load-bearing — do NOT soften / do NOT broaden).** N-F-G-D enforces that the private C1
  dry-run is a **path-faithful, non-promotable rehearsal HARNESS** — and nothing more. (1) PATH FIDELITY: the C1
  private dry-run uses the SAME `--mode node` accepted-block path as preview/preprod (`import_live_consensus_inputs`
  → forge → self-accept → sibling-serve → block-fetch → peer log → `correlate`), with NO private-only flag /
  branch / bootstrap / from-genesis constructor; the only differences are operator INPUTS + the evidence LABEL.
  (2) EVIDENCE NON-PROMOTABILITY: a rehearsal manifest is correlate-produced, marked rehearsal, stored ONLY under
  the rehearsal home, sha256-bound, never under / referenced by a bounty home, and flips NO RO-LIVE rule. G-D does
  **NOT** enforce that a C1 run has succeeded — the rehearsal-schema gate is **vacuous until a real
  operator-produced manifest is committed** (only the README is committed under the rehearsal home; no `.toml`).
  **NO RO-LIVE flip** (`RO-LIVE-01` stays `partial`; `RO-LIVE-06` stays schema-only/`enforced`; neither gains a
  G-D strengthening), **NO bounty / preview / preprod completion claim**; private C1 acceptance ≠ bounty completion
  (preview/preprod acceptance = the single bounty deliverable, captured separately); the live C1 execution stays
  `blocked_until_operator_c1_net_executed`. **NO BLUE change**; the relay-loop containment gate, the served-chain
  handoff fence, and the live-feed memory bounds are **byte-unchanged**. BA-02 is satisfied nowhere.
- N-F-G-D delta verified at `6bd60c80` (grep/ls/git only — no `cargo`):
  - `crates/ade_node/src/rehearsal_evidence.rs`: `//! GREEN` (line 8); `pub const REHEARSAL_MANIFEST_SCHEMA_VERSION:
    u32 = 1;` (line 40); closed 1-variant `pub enum RehearsalVenue { PrivateTestnetC1 }` (line 46); `pub struct
    PrivateRehearsalManifest` (line 73); the SOLE ctor `pub fn from_correlate_outcome(...) -> Option<Self>` (line
    87) returning `None` on `NoEvidence`; `pub fn to_canonical_toml(&self) -> String` (line 106) emitting
    `is_rehearsal = true` + `not_bounty_evidence = true` as LITERALS. Pure (no I/O/clock/rand/float/HashMap); lives
    in `ade_node` (NOT a BLUE `core_path`), so NOT canonical-counted.
  - `crates/ade_node/src/rehearsal_pass.rs`: `//! RED` (line 8); `use crate::ba02_pass::correlate_peer_log_file;`
    (line 24 — the REUSED correlator, verbatim); `pub fn correlate_peer_log_file_into_rehearsal(...) ->
    io::Result<Option<PrivateRehearsalManifest>>` (line 31); `pub fn write_private_rehearsal_manifest(manifest:
    &PrivateRehearsalManifest, ...) -> io::Result<()>` (line 48 — the argument type IS the gate). Registered `pub
    mod rehearsal_evidence;` + `pub mod rehearsal_pass;` in `lib.rs` (lines 29–30).
  - Gate `ci/ci_check_node_path_fidelity.sh` present (guard (a): `cli.rs` argv flag set == the pinned closed
    28-flag allow-list; guard (b): no from-genesis consensus-inputs constructor AND `node_lifecycle.rs` sources
    consensus inputs via the shared `import_live_consensus_inputs`). Gate `ci/ci_check_rehearsal_manifest_schema.sh`
    present (vacuous-until-committed; 12-field schema + `is_rehearsal`/`not_bounty_evidence` markers +
    `peer_log_file_sha256` cross-check + the no-bounty-home-leak cross-check over the active + archived G-C bounty
    homes, fail-closed). `ls ci/ci_check_*.sh | wc -l` = **121** at `6bd60c80`.
    `ci_check_node_run_loop_containment.sh`, `ci_check_served_chain_handoff_fence.sh`, and
    `ci_check_live_feed_memory_bounds.sh` are **byte-unchanged** (`git diff --name-only da205bff..6bd60c80` matches
    none of the three).
  - Registry: `grep -cE '^id = ' docs/ade-invariant-registry.toml` = **315**. NEW `CN-REHEARSAL-FIDELITY-01`
    (`tier = release`, `status = enforced`, `introduced_in = "PHASE4-N-F-G-D"`; two coupled clauses) present
    (`grep -n 'CN-REHEARSAL-FIDELITY-01'`). No rule weakened; no RO-LIVE strengthening bump.
  - Carried (N-F-G-E + N-F-G-C, verified present + unchanged at this HEAD): `session/core.rs`
    `MAX_REASSEMBLY_TAIL_BYTES = 16 MiB` + `session/event.rs` `SessionError::ReassemblyBufferOverflow`;
    `node_sync.rs` `MAX_WIRE_PUMP_LOOKAHEAD = 256`; `ba02_pass.rs` (`//! RED`; `correlate_peer_log_file` +
    `write_ba02_manifest` accepting ONLY a `Ba02Manifest`); `node_sync.rs` `from_wire_pump`; `node_lifecycle.rs`
    `spawn_live_wire_pump_source`; gates `ci_check_live_feed_memory_bounds.sh` +
    `ci_check_ba02_evidence_manifest_schema.sh`.
- Counts at `6bd60c80` (N-F-G-D close, this refresh): **456** canonical types (Δ 0 vs the prior SEAMS — no BLUE
  crate modified; the NEW `ade_node::rehearsal_evidence` is GREEN-by-content and `ade_node::rehearsal_pass` is RED,
  so neither is canonical-counted), **121** CI checks (Δ +2 vs the prior SEAMS's 119 — the two G-D gates
  `ci_check_node_path_fidelity.sh` + `ci_check_rehearsal_manifest_schema.sh`), **315** registry rules (Δ +1 vs the
  prior 314 — NEW `CN-REHEARSAL-FIDELITY-01`, `tier = release`, `enforced`; no rule weakened). All counts match
  the CODEMAP header regenerated at the same HEAD.
- All N-F-G-E / N-F-G-C / N-F-G-B / N-F-G-A / N-F-F / N-F-E / N-F-D / N-F-C / N-F-A / N-Z / N-Y closed surfaces
  re-verified present on disk at this HEAD and unchanged by N-F-G-D (no BLUE crate modified; `ade_codec`
  byte-unchanged; `run_relay_loop` / `run_node_sync` containment + the served-chain handoff fence + the live-feed
  memory bounds byte-unchanged; the live-feed wiring + the serve mechanism unchanged); the refresh annotated only
  the seams N-F-G-D added (the GREEN `PrivateRehearsalManifest` non-promotable envelope + the RED `rehearsal_pass`
  file I/O + the path-fidelity fence + the rehearsal-schema gate) and the surfaces they touched (the §1 argv
  path-fidelity note; the §2 rehearsal-evidence data-only/authoritative split; the §3 Closed rows; the §5 CI
  table; the §6 honest-scope bullet; the §7 C1 dry-run operator-pass gate).
- **Cross-reference check (CODEMAP ↔ SEAMS):** every module named in this SEAMS appears in the CODEMAP regenerated
  at the same HEAD — the N-F-G-D surfaces `ade_node::rehearsal_evidence` (`PrivateRehearsalManifest`,
  `RehearsalVenue`, `from_correlate_outcome`, `to_canonical_toml`, `//! GREEN`) + `ade_node::rehearsal_pass`
  (`correlate_peer_log_file_into_rehearsal`, `write_private_rehearsal_manifest`, `//! RED`), AND the carried
  N-F-G-E (`session::core` `MAX_REASSEMBLY_TAIL_BYTES`, `session::event` `SessionError::ReassemblyBufferOverflow`,
  `node_sync` `MAX_WIRE_PUMP_LOOKAHEAD`, `mux_pump::session_err_to_halt`) / N-F-G-C (`ba02_pass`,
  `node_lifecycle::spawn_live_wire_pump_source`, `node_sync::from_wire_pump`, `ba02_evidence`) modules are all
  inventoried there; the CODEMAP header's PHASE4-N-F-G-D delta names the same two `ade_node` modules + the two new
  gates + `CN-REHEARSAL-FIDELITY-01`. The **456 / 121 / 315** counts match the CODEMAP header. No stale module
  references. The two new CI gates (`ci_check_node_path_fidelity.sh` + `ci_check_rehearsal_manifest_schema.sh`) are
  named in both docs.
- **`.idd-config.json` is CURRENT at this close (verified, not edited).** `_invariant_registry_doc` reads "315
  entries at HEAD" (matches), and `head_deltas_baseline` is `6bd60c80` (the N-F-G-D close baseline). No stale count
  fields surfaced this refresh. (This doc does not edit config.)
- The doc is regenerated, not edited. If a value drifts, fix the source, not the doc.
- NOTE: no `cargo build`/`test`/`check` was run during this regeneration (grep/ls/git only, per the task
  constraint).
