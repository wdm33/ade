# PHASE4-N-F-G-L — Serve-side N2N handshake cardano-node compatibility (CN-WIRE-10)

> **Grounded in a live failure + captured real-node fixtures.** After PHASE4-N-F-G-K fixed the
> serve-lifetime coupling, the live C1 rerun showed the follower now REACHES Ade's serve port `:3002`,
> but the N2N handshake fails: `HandshakeDecodeError NodeToNodeV_15 "unknown encoding: TInt 1"`. Ade's
> serve-side (responder) handshake encoding is rejected by a real cardano-node. **ASYMMETRIC:** Ade's
> INITIATOR handshake works (the `--peer` feed connected + ChainSync ran). S1 captured the canonical
> real-node fixtures: `corpus/network/n2n/handshake/preprod_v11_v16_propose_recv.cbor` (public, magic 1)
> + `c1privnet_v11_v16_propose_recv.cbor` (the EXACT failing peer, magic 42).
>
> Grounding: `[[project_phase4_c1_genesis_rehearsal_live_state]]`; the live failure transcript is the
> follower docker log.

## §0 Slices with sharply different IDD status
- **Mechanical, fixture-pinned (S1 + S2):** capture the canonical real-cardano-node V15 handshake
  (S1 — done, fixtures committed) + correct Ade's serve responder encoding to match it (S2). Closes on
  the captured-fixture byte pin + a re-capture diff, NOT an Ade↔Ade round-trip.
- **Operator-gated (S3):** the live C1 handshake confirmation (a real cardano-node completes the
  handshake with Ade's serve, then ChainSync/BlockFetch) stays operator-gated; it resumes the G-K
  rehearsal. No RO-LIVE flip.

## §1 Primary invariant (CN-WIRE-10)
Ade's N2N handshake RESPONDER must encode `versionData` / `MsgAcceptVersion` / `MsgQueryReply` in the
closed Cardano NodeToNode wire grammar a real cardano-node decodes — through the SAME closed handshake
encoding authority as the INITIATOR path. One handshake grammar, both directions; the versionData arity
must match the negotiated version (V11–15 = 4-element `[networkMagic, diffusionMode, peerSharing,
query]`; V16+ adds its 5th field). No second/parallel handshake encoder; no runtime negotiation of
meaning beyond version selection.

## §2 The defect + the captured canonical fixture (proven, not assumed)
Observed (live C1, real cardano-node 11.0.1 follower dialing Ade `:3002`):
`HandshakeDecodeError NodeToNodeV_15 "unknown encoding: TInt 1"`. Asymmetric — Ade's initiator handshake
succeeds.

Captured canonical RESPONDER encoding (S1 fixtures), real cardano-node 11.0.1, **negotiated V15** (NOT
V16 — 11.0.1's max-common N2N is V15):
```
MsgAcceptVersion(15) payload = 83 01 0f 84 <magic> f5 00 f4
  = [1, 15, [networkMagic, true, 0, false]]   -- versionData is a FOUR-element array
preprod (magic 1):  83 01 0f 84 01    f5 00 f4
c1      (magic 42): 83 01 0f 84 18 2a f5 00 f4
```

## §3 The root-cause lead (S2's first task)
Ade has two version-table builders. The serve responder uses `n2n_supported_for_magic -> n2n(magic)`
(`ade_network/src/handshake/version_table.rs`) for ALL versions ("the VersionData shape is identical");
the initiator uses `build_n2n_version_table` / the capture bin's `version_params_for_n2n`, which sets
versionData arity PER VERSION (4 fields for V11–15, 5 for V16+). So the serve responder very likely
emits one fixed (non-per-version) versionData; a V15 peer decoding it hits an extra/mistyped field as
`TInt 1`. Ade's own client tolerates it (the Ade↔Ade loopback passes); a real cardano-node does not.
Root-causing the exact `TInt 1` is S2's first step — against the S1 fixture.

## §6 TCB color
The fix is in the **closed N2N handshake WIRE GRAMMAR** (`ade_network` codec + version table) — a
deterministic codec authority, a closed semantic surface. **NOT** the ledger/consensus BLUE core, but
codec-correctness, not RED glue — so it carries the codec discipline (fixture-pinned, one authority
both directions, structured errors, NO decoder loosening). No new canonical type; no replay weight
(the handshake is pre-chain).

## §7 Slices
| Slice | Scope | CE | Registry | Status |
|---|---|---|---|---|
| **S1** | Capture + commit the canonical real-cardano-node V15 handshake fixture (the failing peer + a public reference); pin the target | CE-G-L-1 (capture half) | CN-WIRE-10 (declared) | Merged (`e42cb249`) |
| **S2** | Correct Ade's serve responder versionData/accept encoding to match the fixture per negotiated version; one shared closed handshake authority for initiator + responder | CE-G-L-1 (encode half) | CN-WIRE-10 enforced | Merged (`853344f7`) |
| **S3** | Live C1 handshake confirmation: a real cardano-node completes the handshake with Ade's serve. The ChainSync `MsgFindIntersect` step that follows is a NEW layer = **PHASE4-N-F-G-M** (not unfinished G-L). | CE-G-L-2 | CN-WIRE-10 | **LIVE-CONFIRMED** (follower `ColdToWarm → WarmToHot → PromoteWarmDone` on :3002, 2026-06-04 07:27:04Z; no HandshakeDecodeError / TInt 1) |

> **CLOSE (narrow claim).** `CN-WIRE-10` enforced: Ade's serve-side N2N handshake RESPONDER is
> cardano-node compatible for V15 (byte-pinned to the captured real-node fixtures + live-confirmed: the
> real follower reaches HOT with Ade's serve). This cluster does **NOT** claim ChainSync works,
> BlockFetch works, the follower accepted block 0, or any RO-LIVE completion. The next blocker — the
> serve-side ChainSync `MsgFindIntersect` response (the follower's hot session times out at
> `ExceededTimeLimit (ChainSync … ServerHasAgency (SingIntersect))` ~10 s in) — is **PHASE4-N-F-G-M**.

## §8 Cluster Exit Criteria
- **CE-G-L-1 (mechanical):** Ade's serve responder encodes the V15 (and each supported version's)
  handshake reply byte-identically to the captured real-cardano-node fixture
  (`corpus/network/n2n/handshake/*_v11_v16_propose_recv.cbor`, payload-level, modulo the mux-header
  timestamp); the initiator + responder share ONE closed handshake encoding authority. Pinned against
  the captured fixture, NOT an Ade↔Ade round-trip.
- **CE-G-L-2 (operator-gated):** a real cardano-node follower completes the N2N handshake with Ade's
  serve (no `HandshakeDecodeError`) and proceeds to ChainSync — which resumes G-K's CE-G-K-3 (the
  follower BlockFetches the served block 0, validates the null-prev block → `correlate` →
  `PrivateRehearsalManifest`). `blocked_until_operator_c1_genesis_successor_rehearsal`; no RO-LIVE flip;
  acceptance only via the follower log through `correlate`.

## §9 Replay obligations
None new — the handshake is pre-chain, no authoritative state, no new canonical type. The fix is a
wire-grammar encoder correction, fixture-pinned.

## §10 Invariants
- **Preserves:** `DC-NODE-07` (single shared serve), `DC-NODE-09` (serve lifetime, G-K), `CN-WIRE-08`
  (tag-24), the INITIATOR handshake path (works — touched only to share the one authority),
  `RO-LIVE-01/06` (no flip).
- **Adds:** `CN-WIRE-10` (serve-side N2N handshake cardano-node compatibility), declared → enforced at
  S2 close.

## §11 Forbidden during this cluster
**Do NOT loosen / broaden Ade's handshake DECODER to accept `TInt 1` or unknown shapes** — the failure
is the real peer rejecting Ade's ENCODING; the fix makes Ade EMIT the closed grammar, never adds
fallback interpretation (`[[feedback_codec_closed_grammar]]`). No forge/PrevHash/consensus change; no
ChainSync/BlockFetch change unless the handshake proves a downstream issue; no new mini-protocol; no
second/parallel handshake encoder; no runtime negotiation looseness (version SELECTION only); no
private-only flag; no RO-LIVE flip; no acceptance claim without the follower log through `correlate`;
the validation fixture MUST be a real cardano-node capture, never an Ade↔Ade round-trip.

## §12 Open questions
- **OQ-L1:** is the divergence purely versionData arity (4 vs 5), or also field types / the query-reply
  branch (`codec/handshake.rs` `MsgQueryReply`)? → S2 roots it out against the fixture.
- **OQ-L2:** does the responder need per-version versionData (like the initiator path), or is one
  per-negotiated-version encoding sufficient? → S2.

## §13 Non-goals
The ChainSync/BlockFetch serve logic (proven, downstream of the handshake); TLS; peer-sharing beyond
what V15 requires; the initiator path (works); N2C handshake; cross-epoch / durable progression.
