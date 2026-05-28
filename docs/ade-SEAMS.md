# Seams — Where New Work Can Attach (Ade)

> **Status:** Living architectural document. Regenerated; not hand-edited.
> Per-project instance of `~/.claude/methodology/templates/seams.md`.

> 11 crates, **446 canonical types**, **99 CI checks** at HEAD (`273c887`, PHASE4-N-X closing).
> Reads CODEMAP (`docs/ade-CODEMAP.md`, regenerated at the same HEAD) for the module
> list + TCB colors, and the invariant registry (`docs/ade-invariant-registry.toml` —
> **292 entries**) for the rule IDs that gate each closed surface.
>
> **This regeneration is a FULL seams map at HEAD, not a delta refresh.** It supersedes
> the post-PHASE4-N-W SEAMS (HEAD `01e7e08`). The one cluster closed since that anchor is
> folded in: **PHASE4-N-X** — the N2N tag-24 CBOR-in-CBOR wire-envelope authority. N-X
> introduces the NEW BLUE module `ade_codec::cbor::tag24` —
> `wrap_tag24(inner) -> Vec<u8>`, `unwrap_tag24(wire) -> Result<&[u8], TagEnvelopeError>`,
> and the closed `TagEnvelopeError` enum (`NotTag24` / `NotByteString` / `Truncated` /
> `TrailingBytes`) — the single workspace authority for the tag-24 (`0xd8 0x18`) byte
> wrap/unwrap, re-exported at the `ade_codec` crate root. Per-protocol composition lives in
> the `ade_network` BLUE codecs: a served **BlockFetch** `MsgBlock` payload is
> `tag24(bytes([era, block]))` (era **inside** the wrap; EBB-aware era index, Conway = 7)
> via `compose_blockfetch_block` / `decompose_blockfetch_block`; a served **ChainSync**
> `RollForward` header is `[era_tag, tag24(bytes(header_cbor))]` (era_tag **outside** the
> wrap; CONSENSUS era index, Conway = 6 = storage − 1) via `compose_rollforward_header` /
> `decompose_rollforward_header` + `chain_sync_wire_era_index`. The two N2N surfaces use
> **different era-index schemes** — a load-bearing distinction, pinned against real
> cardano-node 11.0.1 captures. The RED hand-rolled tag-24 parses were deleted and migrated
> onto the shared authority (`ade_node::admission::runner` + `ade_core_interop::follow` now
> call `ade_codec::unwrap_tag24`). N-X flips/adds registry rule **CN-WIRE-08** (introduced
> **enforced**), ships one new CI gate (`ci_check_tag24_wire_authority.sh`, 99 total), and
> carries an N-X **strengthening** of **CN-FORGE-03 / DC-CONS-17 / DC-CONS-18**.
>
> **The serve-side tag-24 wire-wrap is no longer a candidate.** It was item 1 in the prior
> SEAMS §7 (deferred from N-V / `open_obligation` on CN-FORGE-03). It **closed** in
> PHASE4-N-X and is now an **enforced** surface, documented in §2 / §3 / §4 below. The
> remaining producer→real-peer follow-ons (the OPERATOR-PASS live leg + N-U durability)
> stay open in §7.
>
> **Cluster-doc location.** Every closed cluster doc is archived under
> `docs/clusters/completed/`, including the entire **N-Q / N-R-A/B/C / N-S-A/B/C** set,
> the **N-M-\*** (admission/seed/WAL/anchor) sub-trees, **N-O**, **N-P**, **N-T**, **N-V**,
> and **N-W**. The **only** cluster directory outside `completed/` is the now-closing
> **PHASE4-N-X** (`cluster.md` + `CLOSURE.md`; archived right after the grounding docs
> regenerate).

---

## 1. Surface Reduction Rules

> External inputs reduce to canonical form before entering authoritative pipelines. Ade
> is a Cardano node, not a request/response service — its "external surfaces" are the
> N2N/N2C wire, operator-supplied key/genesis/opcert files, the cardano-cli UTxO seed
> dump, and argv. Each reduces to a canonical BLUE type before any authoritative
> transition. There is **no HTTP/gRPC/message-bus ingress**; the search for those
> patterns returned nothing (confirmed absent — not a gap).

### Surface: N2N inbound wire (received blocks/headers/txs)

```
Surface: N2N mini-protocol traffic over TCP+mux (RED ade_runtime::network::{n2n_listener, mux_pump, n2n_dialer})
Reduces to: decoded mini-protocol messages → tag-24-stripped inner bytes → PreservedCbor<T> → DecodedBlock (BLUE ade_codec)
Pipeline (fixed; steps may not be reordered or shortcut):
  1. mux::frame::decode_frame                       (BLUE — single frame-decode authority)
  2. session::core::step                            (GREEN — partial-frame buffer + payload reassembly + closed AcceptedMiniProtocol registry)
  3. per-mini-protocol *_transition reducer         (BLUE — chain_sync / block_fetch / etc.)
  3a. tag-24 strip (NEW, N-X)                        (BLUE — decompose_blockfetch_block / decompose_rollforward_header delegate to ade_codec::unwrap_tag24; RED admission::runner / follow call ade_codec::unwrap_tag24 directly — no hand-rolled parse)
  4. ade_codec decode_block_envelope / decode_*     (BLUE — sole PreservedCbor construction site, over the verbatim tag-24-stripped inner bytes)
  5. ade_ledger::receive::reducer / mempool_ingress (BLUE — header→body bridge / wire-ingress chokepoint)
  6. block_validity / tx_validity / admission        (BLUE verdict; GREEN admission compares already-authoritative outputs)
Cross-surface state sharing: the served ServedChainSnapshot (read by both serve and broadcast paths);
  the per-peer outbound map (PerPeerOutbound) is keyed by PeerId — no cross-peer byte leakage.
  The tag-24 unwrap step (N-X) is the SAME shared ade_codec authority used by the serve path's wrap step —
  receive and serve share one CBOR-in-CBOR envelope authority (CN-WIRE-08).
```

### Surface: producer-mode forge → serve → broadcast (the live producer half)

```
Surface: --mode produce slot loop (RED ade_node::produce_mode + GREEN producer::coordinator)
Reduces to: ForgedBlock → AcceptedBlock (BLUE self_accept) → ServedChainSnapshot → tag-24-wrapped wire bytes
Pipeline (fixed; the BLUE-then-RED-then-BLUE composition of run_real_forge):
  1. bootstrap_initial_state                        (RED/GREEN — sole forge-state source; N-T)
  1a. era guard (N-W)                                (RED — non-Praos era fail-closes to ForgeFailureReason::UnsupportedProducerEra before any forge)
  2. RED vrf_prove over expected_vrf_input.alpha_bytes()  (operator VRF key; alpha comes from the BLUE LeaderScheduleAnswer — no RED-side era dispatch; N-W)
  3. BLUE verify_and_evaluate_leader(era, …) → LeaderCheckVerdict  (ade_core::consensus::leader_check; era-correct Praos construction via the single authority; N-R-A + N-W)
  4. RED kes_sign_header(UnsignedHeaderPreImage)    (signs ONLY the branded pre-image; N-S-A)
  5. GREEN assemble_tick
  6. BLUE forge_block → encode_block_envelope       (single canonical block encoder, storage-form [era, block]; N-V)
  7. BLUE self_accept                               (gate — no ForgeSucceeded without Accepted)
  8. ChainEvolution::advance(self)                  (GREEN linear typestate; token only via self_accept; N-T)
  9. ServedChainHandle::push_atomic                 (single served-admit authority; N-R-B/N-T)
 10. BLUE serve composition (NEW, N-X)              (block_fetch::server emits compose_blockfetch_block(storage [era, block]) = tag24(bytes([era, block]));
                                                     chain_sync::server emits compose_rollforward_header(era, header_cbor) = [era_tag, tag24(bytes(header_cbor))] — bytes are tag-24-wrapped before reaching a peer)
 11. OutboundCommand → MuxPump                      (typed relay; no byte tunnel; N-S-B)
Cross-surface state sharing: ChainEvolution threads each forge's post-state into the next
  forge's base (advance consumes self — a stale base is unrepresentable); ServedChainSnapshot
  is shared with the N2N serve path; the per-peer outbound map is shared with the listener.
  The leader VRF alpha flows from LeaderScheduleAnswer.expected_vrf_input (ExpectedVrfInput) —
  the RED prove-step never independently re-derives it (CN-FORGE-04). The serve step's tag-24 wrap
  is the SAME ade_codec authority the receive path uses to unwrap (CN-WIRE-08).
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
```

### Surface: argv (closed mode set)

```
Surface: command line (RED ade_node::cli — Cli / ProduceCli)
Reduces to: a closed mode enum {produce, admission, wire-only, key-gen-KES} (ci_check_node_mode_closure.sh)
Pipeline: argv → Cli → mode driver. --mode produce requires --json-seed + --consensus-inputs.
Cross-surface state sharing: none.
```

**Rule:** New ingress attaches by producing the canonical BLUE type's bytes and entering
the **same** pipeline. A new mini-protocol attaches through `session::core::step` + a BLUE
`*_transition` reducer + a closed `AcceptedMiniProtocol` registry entry — never by a back
door into the core. A new operator file type attaches as a RED parser feeding a BLUE
structural validator. New ingress **may not** introduce a second `PreservedCbor`
construction site, a second block-envelope encoder, a second era→leader-VRF-input
construction (CN-FORGE-04), a second `wrap_tag24` / `unwrap_tag24` definition or a
hand-rolled tag-24 parse in RED (CN-WIRE-08), or a direct-transport write that bypasses
`OutboundCommand`.

---

## 2. Data-Only vs. Authoritative Layers

### Domain: N2N tag-24 wire envelope (NEW, N-X)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sole byte wrap/unwrap authority** | `ade_codec::cbor::tag24::{wrap_tag24, unwrap_tag24}` | BLUE | The **single** workspace authority that wraps inner bytes in a tag-24 (`0xd8 0x18`) CBOR byte-string envelope and strips it. `unwrap_tag24` returns a **zero-copy borrow** of the verbatim inner bytes (no re-encode) and fails closed with a typed `TagEnvelopeError`. Each defined **exactly once** (`ci_check_tag24_wire_authority.sh`). Owns ONLY the byte wrap/unwrap — never per-protocol layout. |
| **BlockFetch composition** | `ade_network::codec::block_fetch::{compose_blockfetch_block, decompose_blockfetch_block}` | BLUE | A served `MsgBlock` payload = `tag24(bytes([era, block]))` — era **inside** the wrap; EBB-aware era index, **Conway = storage index 7**. Delegates the byte wrap/strip to the `ade_codec` authority. |
| **ChainSync composition** | `ade_network::codec::chain_sync::{compose_rollforward_header, decompose_rollforward_header, chain_sync_wire_era_index}` | BLUE | A served `RollForward` header = `[era_tag, tag24(bytes(header_cbor))]` — era_tag **outside** the wrap; **CONSENSUS era index, Conway = 6 = storage − 1** (deliberately different from block-fetch). `HeaderProjection` gained an `era` field; `header_bytes` stays the BARE era-specific header. |
| **Serve emitters** | `ade_network::block_fetch::server` / `chain_sync::server` | BLUE | Emit composed (tag-24-wrapped) bytes — never a bare `[era, block]` over BlockFetch, never a bare header over ChainSync RollForward. |
| **RED consumers (migrated)** | `ade_node::admission::runner` + `ade_core_interop::follow` | RED | Strip a peer's tag-24 envelope via `ade_codec::unwrap_tag24` (the deleted hand-rolled parses); no local tag-24 parse. |

**Rule (CN-WIRE-08):** the workspace has **one** tag-24 byte authority (`wrap_tag24` /
`unwrap_tag24`, each defined exactly once) and per-protocol composition layered over it in
`ade_network`. **The two N2N surfaces use different era-index schemes** — BlockFetch puts
the era **inside** the wrap (storage index, Conway = 7); ChainSync puts the era_tag
**outside** the wrap (consensus index, Conway = 6 = storage − 1). Both compositions are
pinned **byte-identically** against captured cardano-node 11.0.1 wire fixtures (the real
Conway `RollForward` golden under `corpus/network/n2n/chain_sync/preprod_conway_rollforward_*`),
not codec comments. No bare `[era, block]` may be served over BlockFetch; no bare header
over ChainSync RollForward. **No hand-rolled tag-24 parse may exist in RED** — admission and
interop call the shared authority. New protocol support adds a `compose_*` / `decompose_*`
pair delegating to the one `ade_codec` authority; **the wrap/unwrap chokepoint never moves.**

### Domain: block codec (decode + encode)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative ingress** | `ade_codec::cbor::envelope::decode_block_envelope` + the per-era `decode_*_block` | BLUE | Sole `PreservedCbor` construction site; the only place raw bytes become typed semantic values. Operates over the verbatim tag-24-stripped inner bytes on the wire path (N-X). |
| **Authoritative egress (N-V)** | `ade_codec::cbor::envelope::encode_block_envelope` | BLUE | The **single** block-envelope **encoder**, inverse-symmetric to the decoder. Emits the storage-form era-tagged `[era, block]` (Conway = discriminant 7, head `82 07`); the on-wire tag-24 wrap is the N-X composition layer above it. |
| **Producer consumer** | `ade_ledger::producer::forge::forge_block` | BLUE | Wraps forged output via `encode_block_envelope` so `decode_block(forge_block(tick).bytes)` is `Ok`. |

**Rule (CN-FORGE-03, strengthened N-X):** the workspace has **one** block-envelope grammar
in both directions. The producer (forge) and the validator (receive) share it: forge output
round-trips through the same `decode_block` authority that validates received blocks. A
second/parallel block serializer is CI-gated impossible
(`ci_check_no_independent_forge_codepath.sh`, `ci_check_forge_decode_round_trip.sh`). The
N-X strengthening: the on-wire serve form is the tag-24 composition over this storage-form
`[era, block]` — `served bytes == compose_blockfetch_block(forge_block(...).bytes)`, and the
inner decodes byte-identically to the self-accept input. New era support adds a
`decode_*_block` arm + an `encode_block_envelope` discriminant; **the encode/decode
chokepoint pair never moves.**

### Domain: leader-eligibility VRF input (N-W)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Sole era→construction authority** | `ade_core::consensus::vrf_cert::leader_vrf_input(era, slot, eta0)` | BLUE | The **single** place that selects a Praos vs TPraos leader-eligibility VRF construction. Returns the closed `ExpectedVrfInput`. Defined exactly once (`ci_check_producer_praos_vrf.sh` Guard 1). |
| **Era-correct range-extension** | `ade_core::consensus::vrf_cert::leader_value_for(&ExpectedVrfInput, &VrfOutput)` | BLUE | Applies the Praos `praos_leader_value` range-extension vs the TPraos identity, dispatched on the `ExpectedVrfInput` variant. |
| **Leader-schedule producer** | `ade_core::consensus::leader_schedule::query_leader_schedule` | BLUE | Builds `LeaderScheduleAnswer.expected_vrf_input` via `leader_vrf_input`; no bare `vrf_input(` on the producer path (Guard 2). |
| **RED prove-step consumer** | `ade_node::produce_mode::run_real_forge` | RED | Proves over `answer.expected_vrf_input.alpha_bytes()` only — no RED-side era dispatch; a non-Praos era fail-closes to `ForgeFailureReason::UnsupportedProducerEra`. |

**Rule (CN-FORGE-04):** for a given era/protocol version there is **exactly one** VRF
transcript authority. For Praos eras (Babbage/Conway, per `CardanoEra::is_praos()`) the
producer alpha **MUST** equal the validator alpha — `praos_vrf_input(slot, eta0) =
blake2b256(slot‖eta0)` + the `praos_leader_value` range-extension — **NOT** the TPraos
role-tagged alpha (`slot‖eta0‖0x4C`). **No verification/construction fallback may accept
both a TPraos and a Praos VRF input for one era** — the `ExpectedVrfInput` variant *is* the
protocol-family tag, so a Praos and a TPraos alpha can never be confused. The dual
construction lives only inside `leader_vrf_input`; no other file may contain both
`praos_vrf_input(` and a bare `vrf_input(` (`ci_check_producer_praos_vrf.sh` Guard 5). New
era support adds an `ExpectedVrfInput` variant + a `leader_vrf_input` arm; **the era→VRF
construction chokepoint never moves**, and a new variant strengthens CN-FORGE-04.

### Domain: KES signing-key custody

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Data-only loader** | `ade_runtime::producer::keys::load_kes_signing_key_skey` | RED | Reads the 608-byte cardano-cli skey envelope from disk. |
| **Authoritative deserializer** | `ade_crypto::kes_sum::Sum6Kes::raw_deserialize_signing_key_kes` | BLUE | Byte layout is the structural validator; the only path from bytes to a signing key. |
| **Authoritative algorithm** | `ade_crypto::kes_sum` (`KesAlgorithm` trait, `Sum6Kes`) | BLUE | Ade-native Sum6KES, byte-identical to Haskell `cardano-base`. |
| **Signing operation** | `ade_runtime::producer::signing` / `producer_shell::kes_sign_header` | RED | Sole key-custody surface; signs only the branded `UnsignedHeaderPreImage`. |

**Rule:** the RED loader **may not** call `KesSecret::from_bytes_zeroizing` /
`from_seed_at_period` inside `load_kes_signing_key_skey` — only the BLUE deserializer path
(`ci_check_kes_envelope_closed.sh` Guard 2). Signing is RED-confined; BLUE never signs
(`ci_check_no_signing_in_blue.sh`).

### Domain: leader eligibility (RED/BLUE split)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **VRF proof producer** | `ade_node::produce_mode` (`run_real_forge` prove-step) | RED | Produces the VRF proof/output for the slot using the operator's VRF key, over the BLUE answer's `expected_vrf_input.alpha_bytes()`. |
| **Authoritative evaluator** | `ade_core::consensus::leader_check::verify_and_evaluate_leader(era, …)` | BLUE | Verifies the proof + evaluates eligibility from canonical inputs only, with an era-correct threshold (N-W); emits the closed `LeaderCheckVerdict`. |

**Rule (CN-FORGE-02):** BLUE **never** sees the VRF/KES/cold signing keys. The BLUE
evaluator has **no** dependency on `LedgerView`, `EraSchedule`, `ChainDepState`, wall-clock,
storage, or any RED crate. The caller derives `LeaderScheduleAnswer` via the authority path
(`query_leader_schedule`) and passes it in; the evaluator fail-closes on a full-enum
`ExpectedVrfInput` mismatch (wrong era *or* wrong alpha). `ci_check_leader_check_authority.sh`
enforces the import allow-list. New leader-eligibility logic adds to the BLUE evaluator; the
RED/BLUE split never moves.

### Domain: forged-block serving (data-only serve vs. authoritative admit)

| Layer | Module | Color | Role |
|-------|--------|-------|------|
| **Authoritative admit** | `ade_ledger::producer::served_chain::served_chain_admit` | BLUE | The sole entry path into the served index; only self-accepted blocks may be admitted (CN-PROD-04). |
| **Atomic publisher** | `ade_runtime::producer::served_chain_handle::push_atomic` | RED (GREEN-by-content glue) | Wraps `served_chain_admit` inside a `watch::Sender::send_modify` closure — no torn snapshot (CN-SNAPSHOT-01). |
| **Read-side serve** | `ade_network::block_fetch::server::producer_block_fetch_serve` | BLUE | Serves a `RequestRange` only if both endpoints + every block between are present, else `NoBlocks` (CN-SNAPSHOT-02). On the wire it emits the tag-24 composition (N-X). |

**Rule:** a forged block is visible to peers only **after** `push_atomic` succeeds; the
read-side serve is data-only over the BLUE `ServedChainSnapshot`. New serve logic reads the
snapshot; it never admits. The serve emitter wraps via the single tag-24 authority before
bytes reach a peer (CN-WIRE-08).

---

## 3. Closed vs. Extensible Registries

### Closed (frozen — version-gated changes only)

| Registry | Location | Count | Change Rule |
|----------|----------|-------|-------------|
| `TagEnvelopeError` *(NEW, N-X)* | `ade_codec::cbor::tag24` (BLUE) | 4 (`NotTag24 { first_byte }` / `NotByteString { offset }` / `Truncated { offset, needed }` / `TrailingBytes { consumed, total }`) | The closed tag-24 unwrap error. `unwrap_tag24` fails closed on a wrong marker, non-byte-string payload, truncated inner, or trailing bytes (never panics). New variant = strengthening of **CN-WIRE-08** + a registry amendment; variants carry only non-secret offset/length primitives. |
| `ExpectedVrfInput` *(N-W)* | `ade_core::consensus::vrf_cert` (BLUE) | 2 (`Praos([u8;32])` / `Tpraos([u8;41])`) | The 2-variant enum **is** the protocol-family tag for the leader-eligibility VRF input. Built only via `leader_vrf_input(era, slot, eta0)` (the single era→construction authority). A new variant (new protocol family) = a `leader_vrf_input` arm + a **strengthening of CN-FORGE-04**. **No both-alphas fallback** — no caller may accept a Praos and a TPraos alpha for one era. |
| `LeaderCheckVerdict` *(N-R-A)* | `ade_core::consensus::leader_check` (BLUE) | 2 (`Eligible` / `NotEligible`) | New variant = strengthening of **CN-FORGE-02** + a registry amendment. The 2-variant shape is load-bearing: `NotEligible` carries only a bounded `vrf_output_fingerprint`, never forge-capable material — illegal observation is structurally impossible. Do **not** add a third variant without re-proving the no-forge-material property. |
| `ForgeFailureReason` *(extended N-W)* | `ade_runtime::producer::producer_log` (GREEN-by-content) | closed sum incl. `UnsupportedProducerEra` | A non-Praos producer forge attempt fail-closes to `UnsupportedProducerEra` (never silently forges with a TPraos alpha). New variant = strengthening of **CN-FORGE-04 / DC-PROD-01**. No free-form reason strings. |
| `OutboundCommand` *(N-S-B)* | `ade_runtime::network::outbound_command` (RED) | typed `ChainSyncServerMsg` / `BlockFetchServerMsg` variants | New variant = a new typed mini-protocol reply. **No `Vec<u8>` byte tunnel may be added** — the typed-only contract is enforced by `ci_check_no_produce_mode_direct_transport_writes.sh` (CN-OUTBOUND-RELAY-01). |
| `DispatchError` *(N-S-B)* | `ade_node::produce_mode` + `ade_runtime::network::n2n_server` (RED) | closed sum (incl. `UnknownPeer`, `PeerOutboundMissing`) | Lookup failure must stay structured; no `String`-bearing or catch-all variant (CN-PEER-OUTBOUND-MAP-01). |
| `ChainEvolutionError` *(N-T)* | `ade_runtime::producer::chain_evolution` (GREEN-by-content) | closed sum (incl. `AuthorityMismatch`, `SelfAcceptRejected`) | New variant requires a strengthening of **DC-PROD-03**. `AuthorityMismatch` fail-closes when BLUE `block_validity` and BLUE `self_accept` disagree. |
| `BroadcastPushError` *(N-T)* | `ade_node::produce_mode` (RED) | closed sum (incl. `SelfAcceptReplayRejected`) | New variant requires a strengthening of **CN-PROD-04**. |
| `ProducerLogEvent` *(N-Q)* | `ade_runtime::producer::producer_log` (GREEN-by-content) | closed JSONL vocab (`handshake_ok`, `slot_tick`, `leader_elected`, `block_forged`, `block_served`, `peer_chain_tip_observed`, `slot_missed{reason}`, `coordinator_shutdown{reason}`) | New variant = strengthening of **DC-PROD-01**. No free-form reason strings, no key material, no path strings; socket addresses excluded from the replay surface (PeerId is an opaque `u64`). |
| `GenesisParseError` *(N-R-C)* | `ade_runtime::producer::genesis_parser` (RED) | closed sum | New variant = strengthening of **CN-GENESIS-01**. No `String` in load-bearing variants; no implicit defaults / stringly fallback. |
| `OpCertParseError` *(N-R-C)* | `ade_runtime::producer::opcert_envelope` (RED) | closed sum | New variant = strengthening of **CN-OPCERT-01**. No `String` payloads in load-bearing variants. |
| `UnsignedHeaderPreImageError` *(N-S-A)* | `ade_ledger::block_validity::unsigned_header_pre_image` (BLUE) | closed sum | New variant = strengthening of **DC-KES-HEADER-01**. |
| `AcceptedMiniProtocol` *(N-L)* | `ade_network::session` (GREEN) | closed registry | New mini-protocol = registry entry + a `match` arm with **no wildcard accept**. |
| `KesError` / `KesParseError` *(N-P)* | `ade_crypto::kes_sum::errors` (BLUE) | 5 / 6 variants | New variant = strengthening of **DC-CRYPTO-08/09**; carries only non-secret primitives. |
| Operator-evidence manifest TOML schema *(N-S-C)* | `ci_check_operator_evidence_manifest_schema.sh` + `docs/clusters/completed/PHASE4-N-S-C/cluster.md` | closed key set (`schema_version`, `ade_commit`, `cardano_node_version`, `cardano_cli_version`, `network`, `block_hash`, `slot`, `opcert_fingerprint`, `genesis_fingerprint`, `ade_evidence_file`, `peer_log_file`, `peer_log_capture_command`, `peer_log_filter`, `peer_log_file_sha256`, `acceptance_keyword_match`) | Any committed `CE-N-S-LIVE_*.toml` MUST conform; `peer_log_file_sha256` cross-checks the committed peer-log file's actual hash (CN-OPERATOR-EVIDENCE-01). |
| `CardanoEra` + Conway cert / governance / withdrawal enums | `ade_types::{era, conway::*}` + `ade_codec::conway::*` | closed | New era / cert / gov-action = a versioned gate; no unknown-tag swallow, no silent skip, no catch-all (DC-LEDGER-08/09/10/11). `CardanoEra::is_praos()` (N-W) MUST classify exactly {Babbage, Conway}; the per-protocol tag-24 era index schemes (block-fetch storage Conway = 7; chain-sync consensus Conway = 6) are pinned (CN-WIRE-08). |
| Consensus message + verdict enums | `ade_core::consensus`, `ade_ledger::block_validity` / `tx_validity` | closed | `ci_check_consensus_closed_enums.sh` — closed consensus enums; `match` with no wildcard. |
| JSONL event vocabularies (admission / wire-only / live-log) | `ade_node::{admission_log, live_log}`, `ade_runtime::admission` | closed | New event = strengthening of the owning DC rule; allow-list + negative tests; wire success ≠ admission ≠ agreement. |

### Extensible (open within constraints)

| Registry | Location | Extension Rule |
|----------|----------|----------------|
| `PerPeerOutbound` map *(N-S-B)* | `ade_runtime::network::outbound_command` — `Arc<RwLock<BTreeMap<PeerId, mpsc::Sender<OutboundCommand>>>>` | Grows at **runtime**: the listener (`run_per_peer_session`) inserts a sender on `PeerConnected`; MuxPump removes on `emit_peer_disconnected`. **`BTreeMap`, not `HashMap`** — deterministic iteration. `produce_mode` looks up by `PeerId` and cannot fabricate senders; no cross-peer byte leakage (CN-PEER-OUTBOUND-MAP-01, DC-OUTBOUND-FIFO-01). |
| `OpCertCounterMap` | `ade_core::consensus::praos_state` (BLUE) | Grows as op-certs are observed; deterministic ordering (no raw `HashMap`). |
| `ServedChainSnapshot` (served blocks) | `ade_ledger::producer::served_chain` (BLUE) | Grows via `served_chain_admit` only (self-accepted blocks); `push_atomic` is the sole publisher. |
| `MempoolState` (admitted txs) | `ade_ledger::mempool` (BLUE) | Grows via `mempool_ingress` → `admit` only; sorted/deduplicated; the single BLUE wire-ingress chokepoint. |
| Seed entries (imported UTxO) | `ade_runtime::seed_import` (GREEN-by-content) | Grows at import time from a cardano-cli UTxO dump; canonical decoders only, no bypass. |
| Ade-native WAL (append-only) | `ade_runtime::wal` (GREEN-by-content) | Append-only; committed entries are never mutated/rewritten (`ci_check_wal_append_only.sh`). |
| Sum_n KES family | `ade_crypto::kes_sum` (BLUE) | A new `Sum_n` attaches as an internal type-alias step (`Sum1Kes..Sum6Kes`); the `KesAlgorithm` trait surface does not change. |
| Per-protocol tag-24 compositions *(N-X)* | `ade_network::codec::{block_fetch, chain_sync}` | A new mini-protocol's CBOR-in-CBOR composition attaches as a `compose_*` / `decompose_*` pair **delegating** the byte wrap/strip to the single `ade_codec::{wrap_tag24, unwrap_tag24}` authority — never a new wrap/unwrap definition, never a hand-rolled parse (CN-WIRE-08). |

---

## 4. Version-Gated vs. Frozen Contracts

### Frozen (immutable at current version — change = new major version)

- **N2N tag-24 wire envelope (NEW, N-X)** — the CBOR-in-CBOR `0xd8 0x18` byte-string
  envelope, wrapped/stripped through the single `ade_codec::{wrap_tag24, unwrap_tag24}`
  authority. Per-protocol composition is pinned byte-identically against cardano-node 11.0.1
  captures: a served **BlockFetch** `MsgBlock` is `tag24(bytes([era, block]))` (era
  **inside** the wrap, storage index, Conway = 7); a served **ChainSync** `RollForward`
  header is `[era_tag, tag24(bytes(header_cbor))]` (era_tag **outside** the wrap, consensus
  index, Conway = 6 = storage − 1). The two era-index schemes differ — changing either
  breaks block-fetch/chain-sync interop with a real peer. The inner bytes are copied
  verbatim (no re-encode). (CN-WIRE-08.)
- **Leader-eligibility VRF transcript (N-W)** — for Praos eras (Babbage/Conway) the
  leader alpha is `praos_vrf_input(slot, eta0) = blake2b256(slot‖eta0)` + the
  `praos_leader_value` range-extension; for TPraos it is the role-tagged `slot‖eta0‖0x4C`.
  One era→construction authority (`leader_vrf_input`); the `ExpectedVrfInput` variant is the
  protocol-family tag. Changing the transcript breaks every forged-block leader proof AND
  the validator's `verify_praos_vrf`. (CN-FORGE-04.)
- **Block-envelope grammar (N-V)** — storage-form `[era, block]`, Conway = discriminant 7
  (head `82 07`). One encoder (`encode_block_envelope`), one decoder
  (`decode_block_envelope`); inverse-symmetric; `encode` must re-encode a corpus block
  byte-identically. The on-wire serve form is the N-X tag-24 composition over this
  storage-form. Changing the envelope shape breaks every forged block AND every
  received-block decode. (CN-FORGE-03, strengthened N-X.)
- **Unsigned-header KES pre-image recipe (N-S-A)** — the canonical CBOR encoding of
  `ShelleyHeaderBody`. The branded `UnsignedHeaderPreImage(Vec<u8>)`'s **only** constructor
  is `unsigned_header_pre_image(...)`; `kes_sign_header` accepts only this type — arbitrary
  byte signing is mechanically unrepresentable. Output is byte-identical to the validator
  extractor `decode_block(...).header_input.kes.header_body_bytes` for every corpus block.
  (CN-KES-HEADER-01, CN-PREIMAGE-FIXTURE-01.)
- **Sum6KES algorithm + expand_seed prefix (N-P)** — Ade-owned, byte-identical to Haskell
  `cardano-base`; `expand_seed` uses prefix bytes `0x01`/`0x02` (not cardano-crypto Rust's
  `0x00`/`0x01`). 608-byte expanded skey + 448-byte signature layouts are pinned.
- **Wire encoding** — `minicbor` / canonical CBOR; field order = wire order; `PreservedCbor`
  aliases the input bytes (no re-encode for hashing — `ci_check_hash_uses_wire_bytes.sh`).
- **Hash algorithms** — Blake2b-224 / Blake2b-256; the single `block_body_hash` recipe; the
  per-era VRF input transcript. Algorithm immutable per version.
- **Mux frame format** — single `encode_frame` / `decode_frame` pair workspace-wide.
- **All 446 canonical types** — existing wire formats frozen; new types may be added.

### Version-gated (can evolve across major versions)

- New era support: a `decode_*_block` arm + an `encode_block_envelope` discriminant + a
  `CardanoEra` variant + (for the leader path) an `ExpectedVrfInput` variant and a
  `leader_vrf_input` arm + (for the wire path) the per-protocol tag-24 era-index entries
  (versioned gate).
- New mini-protocol: an `AcceptedMiniProtocol` registry entry + a BLUE `*_transition`
  reducer + (for serving) an `OutboundCommand` variant + (for CBOR-in-CBOR payloads) a
  `compose_*` / `decompose_*` pair delegating to the single tag-24 authority.
- New closed-enum variant (`TagEnvelopeError`, `ExpectedVrfInput`, `LeaderCheckVerdict`,
  `OutboundCommand`, `ProducerLogEvent`, `ForgeFailureReason`, the parse-error sums, the
  JSONL vocabularies): a `[[rules]]` registry entry + a strengthening of the owning rule.
- New canonical-type fields (sort/dedup invariants preserved).
- New CI checks (existing checks may be tightened, **never** relaxed — RO-CLOSE-01).

---

## 5. Module Addition Rules

Derived from CODEMAP's Cross-Module Rules + the shared BLUE header.

| Color | Naming convention | Build-config flags | May depend on | MUST NOT depend on |
|-------|-------------------|--------------------|----------------|--------------------|
| **BLUE** | `ade_*` crate, or a BLUE `ade_network` submodule path in `.idd-config.json` `core_paths`; `// Core Contract:` + `//! BLUE …` banner first line | `#![deny(unsafe_code)]`, `deny(unwrap_used / expect_used / panic / float_arithmetic)`; no `#[cfg(feature = …)]` semantic gating | Other BLUE modules only (per the dep DAG: `ade_types` ← `ade_codec`/`ade_crypto` ← `ade_core` ← `ade_ledger`/`ade_plutus`; `ade_network` BLUE submodules ← `ade_codec`+`ade_types`) | `ade_runtime`, `ade_node`, `ade_core_interop`, the RED half of `ade_network`; std runtime / I/O / clock / rand / `HashMap` / float / async |
| **GREEN** | `ade_testkit` crate, `ade_network::session`, or a GREEN-by-content sub-tree inside `ade_runtime` / `ade_node` with a `//! GREEN …` banner | Same deny attributes as BLUE; a purity CI gate per sub-tree | BLUE modules | RED modules in non-test deps; nondeterminism; secret material (e.g. `producer::coordinator` MUST NOT hold `KesSecret`/`VrfSigningKey`/`ColdSigningKey`) |
| **RED** | `ade_runtime`, `ade_node`, `ade_core_interop`, `ade_network::mux::transport`; `//! RED …` banner | tokio/std/I/O allowed | Any module | — (RED is the leaf; `ade_core_interop` + the capture binaries are RED leaves nothing depends on) |

### New module checklist

1. Add to `Cargo.toml` `[workspace] members` (BLUE submodule paths: also add to
   `.idd-config.json` `core_paths`).
2. Apply the `// Core Contract:` + `//! BLUE|GREEN|RED` banner first line
   (`ci_check_module_headers.sh`).
3. BLUE/GREEN: inherit the deny attributes; pass `ci_check_forbidden_patterns.sh`,
   `ci_check_no_async_in_blue.sh`, `ci_check_no_semantic_cfg.sh`.
4. `ci_check_dependency_boundary.sh` rejects forbidden cross-color imports;
   `ci_check_pallas_quarantine.sh` confines `pallas-*` to `ade_plutus`.
5. New canonical types: add round-trip tests (no separate canonical-type registry file —
   `canonical_type_registry: null`; canonical-type rules live inline in registry family T).
6. New closed surface: add a `[[rules]]` entry and a CI gate; reference it by ID in the
   cluster/slice docs.

### CI gates that enforce the boundary (99 total; the producer/network/node set)

| Script | Enforces | Cluster |
|---|---|---|
| `ci_check_tag24_wire_authority.sh` *(NEW)* | CN-WIRE-08 / N-X — single tag-24 wrap/unwrap authority (`wrap_tag24`/`unwrap_tag24` each defined exactly once); no hand-rolled tag-24 parse in RED (`admission::runner` / `follow` call the shared authority); serve paths compose via `compose_blockfetch_block` / `compose_rollforward_header` (no bare `[era, block]` / bare header); both compositions pinned byte-identically against captured cardano-node 11.0.1 fixtures; `read_bytes`/`read_text`/`skip_item` overflow guard. | N-X |
| `ci_check_producer_praos_vrf.sh` | CN-FORGE-04 / N-W — single era→leader-VRF-input authority (`leader_vrf_input` defined once); no bare `vrf_input(` on the producer leader path; no both-alphas fallback outside `vrf_cert.rs`; Praos eligibility threshold via `leader_value_for`. | N-W |
| `ci_check_produce_mode_uses_bootstrap_initial_state.sh` | CN-PROD-03 / N-T — `produce_mode` obtains initial state only via `bootstrap_initial_state`; no `SyntheticForgeInputs` / inline `LedgerState::new` forge base. | N-T |
| `ci_check_forge_decode_round_trip.sh` | CN-FORGE-03 (strengthened N-X) — `decode_block(forge_block(tick).bytes)` is `Ok`; forge output is the enveloped `[era, block]` form. | N-V |
| `ci_check_no_independent_forge_codepath.sh` | CN-FORGE-03 (strengthened N-X) — single forge codepath; no parallel block serializer. | N-V |
| `ci_check_leader_check_authority.sh` | CN-FORGE-02 — BLUE leader-check has no LedgerView/EraSchedule/RED dep; never sees private keys; closed `LeaderCheckVerdict`. | N-R-A |
| `ci_check_unsigned_header_preimage_single_source.sh` | CN-KES-HEADER-01 / DC-KES-HEADER-01 — single canonical pre-image recipe; branded `UnsignedHeaderPreImage`; pure. | N-S-A |
| `ci_check_no_produce_mode_direct_transport_writes.sh` | CN-OUTBOUND-RELAY-01 — bytes only via `OutboundCommand` → `MuxPump`; no direct transport write; no byte tunnel. | N-S-B |
| `ci_check_operator_evidence_manifest_schema.sh` | CN-OPERATOR-EVIDENCE-01 — closed evidence-manifest TOML schema + peer-log SHA256 cross-check. | N-S-C |
| `ci_check_producer_coordinator_no_secrets.sh` | CN-PROD-02 — GREEN coordinator never owns/stores private signing material. | N-Q |
| `ci_check_node_mode_closure.sh` | closed `ade_node` mode set. | N-Q |
| `ci_check_served_chain_closure.sh`, `ci_check_snapshot_encoder_closure.sh` | served-chain + snapshot-encoder closure (carry-forward, exercised by per-peer dispatch). | N-R-B |

> Earlier-cluster gates (N-A..N-P, the N-M-* admission/seed/WAL/anchor set, the N-L
> wire-session set) are present in the 99 total; per-script detail is in the registry's
> `ci_script` fields. The full list is `ls ci/ci_check_*.sh`.

---

## 6. Forbidden Patterns (per color)

- **BLUE:** no clock, rand, raw `HashMap`/`HashSet`/`IndexMap`, float, env access,
  network/filesystem, async runtime, locale-dependent ops, OS-dependent ordering. No
  signing operations (`ci_check_no_signing_in_blue.sh`). No `#[cfg(feature = …)]` semantic
  gating. No `PreservedCbor` construction outside `ade_codec`. No re-encode of wire bytes
  when hashing. No second era→leader-VRF-input construction (CN-FORGE-04) —
  `leader_vrf_input` is the single authority; no both-alphas fallback. **No second
  `wrap_tag24` / `unwrap_tag24` definition (CN-WIRE-08)** — the single tag-24 byte
  authority lives in `ade_codec::cbor::tag24`; per-protocol composers delegate to it and
  never re-encode the inner bytes; serve emitters never produce a bare `[era, block]` /
  bare header.
- **GREEN:** no nondeterminism; no participation in authoritative outputs. The
  `producer::coordinator` MUST NOT own/store private signing material. `ChainEvolution`
  (N-T) MUST NEVER mint `AcceptedBlock` — it obtains the token solely from BLUE
  `self_accept`; `advance` consumes `self` (linear typestate — a stale base is
  unrepresentable). `ProducerLogEvent` / `ForgeFailureReason` are closed vocabularies — no
  open/wildcard variant, no free-form reason strings. Evidence/admission reducers compare
  already-authoritative outputs; `lagging` ≠ success; wire success ≠ admission ≠ agreement.
- **RED:** no direct mutation of BLUE state; no construction of semantic types from raw
  bytes (must go through the canonical decoders / BLUE structural validators); no bypassing
  canonical validation. `produce_mode` MUST emit outbound bytes only via `OutboundCommand`
  (no direct `MuxTransportHandle::outbound` write, no `Vec<u8>` byte tunnel). The per-peer
  outbound map is `BTreeMap` (deterministic), keyed by `PeerId` (no cross-peer leakage).
  Key custody is confined to `producer::signing` / `producer_shell`. `run_real_forge`
  (N-W) MUST NOT perform RED-side era dispatch for the leader-VRF alpha — it proves over
  the BLUE answer's `expected_vrf_input.alpha_bytes()`, and a non-Praos era fail-closes to
  `ForgeFailureReason::UnsupportedProducerEra` (never silently forges with a TPraos alpha).
  **No hand-rolled tag-24 parse (CN-WIRE-08, N-X)** — `ade_node::admission::runner` and
  `ade_core_interop::follow` strip a peer's CBOR-in-CBOR envelope via the single
  `ade_codec::unwrap_tag24` authority; no local tag-24 parse.

### Project-specific additions (Ade)

- **No synthetic forge state (N-T hard prohibition):** `produce_mode` MUST NOT construct
  `SyntheticForgeInputs`, a zero-stake `LeaderScheduleAnswer`, or an inline
  `LedgerState::new(...)` forge base. `bootstrap_initial_state` is the sole forge-state
  source.
- **No durability in the produce_mode path (N-U scope):** no WAL/snapshot writes inside the
  `produce_mode` loop — forged-block durability is deferred to N-U (see §7).
- **`cardano_crypto::kes` is a `#[cfg(test)]` oracle only** under `crates/ade_crypto/src/**`
  (`ci_check_kes_sum_compatibility.sh` Guard 3). `pallas-*` confined to `ade_plutus`.
- **Commit-attribution override (CLAUDE.md):** this repo carries a model-attribution
  trailer on commit messages only (bounty requirement). Source comments, PRs, releases,
  issue comments still follow the global no-AI-attribution rule.

---

## 7. Candidate & Not-Yet-Wired Seams (declared follow-ons — NOT closed)

> Surfaced honestly per IDD: these are **declared** future attach points, not closed
> surfaces. Each is named in a registry rule or a cluster CLOSURE record.
>
> **N-X is no longer here.** The serve-side tag-24 wire-wrap (formerly item 1, deferred
> from N-V / `open_obligation` on CN-FORGE-03) **closed** in PHASE4-N-X and is now
> **enforced** (`CN-WIRE-08`, `status = "enforced"`, `introduced_in = "PHASE4-N-X"`,
> `ci_script = ci/ci_check_tag24_wire_authority.sh`); it is documented as a closed surface
> in §2 / §3 / §4 above. The remaining producer→real-peer follow-ons stay open below.

1. **N-U — forged-block durability.** WAL / ChainDB / snapshot / warm-start for forged blocks
   (crash → bootstrap warm-start). Explicitly out of N-T scope (`open_obligation` on
   CN-PROD-03 / DC-PROD-03); N-T exercises only the cold-start branch.

### Operator-pass execution gates (schema enforced, execution blocked)

- **CN-OPERATOR-EVIDENCE-01 / CN-CONS-06 / RO-LIVE-01** — the manifest schema is enforced,
  but C1 (private testnet) / C2 (preprod) operator-pass execution is
  `blocked_until_operator_pass_executed`. Until an operator commits a `CE-N-S-LIVE_*.toml`,
  the CI gate is vacuously satisfied. This is an execution gate, not a code seam. With both
  CN-FORGE-04 (N-W, leader-VRF authority) and CN-WIRE-08 (N-X, on-wire tag-24 serve/receive
  authority) now enforced, the producer forge composition is mechanically complete through
  the serve step — the served block reaches a peer in the correct tag-24-wrapped form. The
  remaining blocker to a real-peer block-acceptance pass is the OPERATOR-PASS live leg
  itself (executing C1/C2 against a real cardano-node peer).

---

## Generation notes

- Regenerated full at HEAD `273c887` (`git rev-parse --short HEAD`), downstream of the
  CODEMAP at the same HEAD. PHASE4-N-X is the active cluster, closing now.
- Every closed surface was verified against the on-disk code: `wrap_tag24` / `unwrap_tag24`
  / `TagEnvelopeError` (`crates/ade_codec/src/cbor/tag24.rs:39,61,79`), the per-protocol
  composers `compose_blockfetch_block` / `decompose_blockfetch_block`
  (`crates/ade_network/src/codec/block_fetch.rs:203,210`), `chain_sync_wire_era_index` /
  `compose_rollforward_header` / `decompose_rollforward_header`
  (`crates/ade_network/src/codec/chain_sync.rs:268,279,292`), and the RED unwrap migration
  (`crates/ade_node/src/admission/runner.rs:489`, `crates/ade_core_interop/src/follow.rs:199`),
  plus the carry-forward `ExpectedVrfInput` / `leader_vrf_input` / encoder / `ChainEvolution`
  / `UnsignedHeaderPreImage` / `OutboundCommand` / parse-error sums. The new CI gate
  `ci_check_tag24_wire_authority.sh` was confirmed present (99 total via
  `ls ci/ci_check_*.sh | wc -l`). CN-WIRE-08 confirmed `status = "enforced"` /
  `introduced_in = "PHASE4-N-X"` in the registry (`docs/ade-invariant-registry.toml`, 292
  rules); CN-FORGE-03 / DC-CONS-17 / DC-CONS-18 carry the N-X strengthening.
- Cluster-doc location verified on disk: only `docs/clusters/PHASE4-N-X/` lives outside
  `docs/clusters/completed/` (archived right after the grounding docs regenerate).
- Counts: 446 canonical types (+1 from the closed `TagEnvelopeError` enum in `ade_codec`),
  99 CI checks (+`ci_check_tag24_wire_authority.sh`), 292 registry rules (+1: CN-WIRE-08).
- The doc is regenerated, not edited. If a value drifts, fix the source, not the doc.
