# Define — codec-wire-crypto

Drafts for Doctrine §32. Lines carry numbers, not IDs; IDs are assigned when a line is approved. This document is not edited after its final consensus round; the batch and the human's reply go in the consensus record (idse.md §1.3).

This is batch B02 of the Ade IDSE migration (`docs/planning/idse-migration-plan.md`). It covers decoding and ingress, mini-protocol wire formats, cryptographic verification, KES and VRF keys, and header signing. Each line names the legacy IDD entries it carries (`legacy:`). Every one of the batch's 37 legacy IDs (36 assigned plus T-ENC-02, deferred here by the foundations batch) appears on a line or in a settlement. Seven of them restate invariants already defined by the foundations batch, and settle as merges into those IDs.

Sources: docs/legacy/idd-invariant-registry.toml (entries listed under each line), docs/legacy/clusters/completed/PHASE4-N-P/period-from-zeroed-sum6-tree-shape-proof.md, docs/ade-invariant-registry.toml (DET-002, DET-003, DET-004, SEC-001, SEC-002)

## Drafts

```text
#1  VAL  ClosedGrammarDecoding  [critical]
    statement:      Each authoritative wire or ledger type has exactly one decoder per protocol version, and it accepts exactly the closed grammar the reference node accepts for that type and version, including valid non-canonical CBOR; each of Ade's own storage record types (write-ahead log, chain database, snapshots) has exactly one decoder per format version, accepting exactly the closed grammar Ade defines for that version. Bytes outside the applicable grammar are rejected with a deterministic structured error before any authoritative state transition, and no fallback, legacy or lenient parser accepts bytes the decoder rejects, on any code path. Where a wire or ledger type's grammar and meaning are unchanged between two supported protocol versions, both versions' decoders give the same result for the same bytes, and every difference between protocol versions is one the reference node also makes; differences between Ade storage format versions are exactly those Ade's format definitions declare.
    counterexample: A delegation certificate with an extra trailing field that the decoder rejects but a fallback parser on the recovery path accepts; or a transaction body map with a duplicated key, which the reference node rejects, that Ade decodes and validates.
    secondary:      [SEC]
    scope:          codec: every authoritative wire and ledger type, and every Ade storage record type
    assumptions:    [A2]
    depends_on:     [DET-003, SEC-002]
    locality:       local
    legacy:         [T-ENC-02, CN-WIRE-03, CN-WIRE-04, CN-WIRE-05]

#2  STR  NamedIngressChokepoints  [critical]
    statement:      Every external byte string that reaches an authoritative transition, from the network, the node-to-client socket, or storage rehydration (write-ahead log, chain database, snapshots), enters the core through the named decode chokepoint for its type and era; no path hands raw or partially decoded bytes to a core transition around it.
    counterexample: Recovery replays stored blocks through a separate decode path that skips the era-tag check the network path performs.
    secondary:      [VAL]
    scope:          ingress: network, N2C, storage rehydration
    assumptions:    []
    depends_on:     [#1, STR-002]
    locality:       local
    legacy:         [T-INGRESS-01, DC-INGRESS-01, DC-INGRESS-02, CN-WIRE-06]

#3  EVO  ClosedVersionedMessages  [standard]
    statement:      Every mini-protocol message decodes into exactly one variant of a closed message type for the negotiated protocol version; bytes that match no variant are rejected and end the session, and no variant is chosen by guessing or by trying alternatives.
    counterexample: A chain-sync message tag that is valid only in a later protocol version is accepted on a session that negotiated an earlier one, or an unknown tag is skipped instead of rejected.
    secondary:      [VAL]
    scope:          mini-protocol codecs (N2N and N2C)
    assumptions:    [A2]
    depends_on:     [#1]
    locality:       local
    legacy:         [CN-WIRE-07]

#4  VAL  Tag24EnvelopeAuthority  [standard]
    statement:      CBOR-in-CBOR (tag 24) envelopes in block-fetch and chain-sync messages are built and removed only by one shared BLUE authority and match the reference node's wire shape: a served block-fetch block is tag 24 over [era, block], a served chain-sync header is [era, tag 24 over header]; a received block-fetch block is unwrapped exactly once, at the receive boundary, before block decode; a malformed or missing envelope, a wrong inner length or trailing bytes are rejected with a structured error, and the inner bytes are kept verbatim.
    counterexample: Ade serves a bare [era, block] over block-fetch, or a received block-fetch payload without its tag-24 wrapper is handed to the block decoder.
    secondary:      [STR]
    scope:          block-fetch and chain-sync codecs, serve and feed paths
    assumptions:    [A2]
    depends_on:     [#1, DET-002]
    locality:       local
    legacy:         [CN-WIRE-08, CN-WIRE-12]

#5  VAL  PrevHashClosedSum  [standard]
    statement:      A Conway header's previous-hash field is the closed sum Genesis or Block(32-byte hash), encoded as CBOR null or as the 32-byte hash through one shared codec; no placeholder hash stands for the genesis predecessor, a block numbered 0 carries Genesis, and every other block carries Block.
    counterexample: A forged genesis-successor block that carries an all-zero 32-byte hash as its previous hash, or a block numbered 5 that carries null.
    secondary:      [STR]
    scope:          header codec, forging and header validation
    assumptions:    [A2]
    depends_on:     [#1, DET-003]
    locality:       local
    legacy:         [CN-WIRE-09]

#6  VAL  N2NHandshakeWireCompatibility  [standard]
    statement:      The node-to-node handshake encodes version data, MsgAcceptVersion and query replies in the reference node's closed grammar for the negotiated version, and the initiator and responder use one shared per-version encoder: versions 11 to 15 carry the four-field version data, and versions 16 and later add the Peras-support field.
    counterexample: The responder answers a version-15 proposal with version data encoded as a bare integer, which the reference node rejects with a handshake decode error.
    secondary:      []
    scope:          N2N handshake codec
    assumptions:    [A2]
    depends_on:     [#1]
    locality:       local
    legacy:         [CN-WIRE-10]

#7  VAL  FindIntersectCompatibility  [standard]
    statement:      The chain-sync server decodes a MsgFindIntersect point list in both the definite-length and the indefinite-length array forms the reference node sends, while every other array in chain-sync messages stays definite-length only; it answers IntersectFound with the first listed point that lies on its served chain, Origin always counting as on it, and IntersectNotFound when none does.
    counterexample: The server rejects a reference node's MsgFindIntersect because its point list is an indefinite-length array, or answers IntersectNotFound to a request whose list ends with Origin.
    secondary:      []
    scope:          chain-sync server codec and intersect reply
    assumptions:    [A2]
    depends_on:     [#1]
    locality:       local
    legacy:         [CN-WIRE-11]

#8  SEC  VerificationMatchesReference  [critical]
    statement:      Ed25519 signature, Praos VRF, Sum6 KES and operational-certificate verification accept exactly the signatures, proofs and inputs the reference implementation accepts, on every published and captured test vector; each verification is a pure function of its inputs, and a failed verification is final, never retried with another parser or serialization of the same input.
    counterexample: A VRF proof the reference rejects (a non-canonical point encoding) that Ade's verifier accepts, or a signature check that fails, re-encodes the message and passes on the second try.
    secondary:      [VAL]
    scope:          ade_crypto verification and the consensus checks that call it
    assumptions:    [A2]
    depends_on:     [SEC-001]
    locality:       local
    legacy:         [DC-CRYPTO-01, CN-CRYPTO-04]

#9  SEC  HashDomainSeparation  [critical]
    statement:      Every consensus-relevant hash Ade computes uses the domain prefix and input layout the protocol defines for its purpose, so no value derived for one purpose can stand in for a value derived for another.
    counterexample: The leader value and the nonce contribution derived from one VRF output without their distinct protocol prefixes, so the two values are equal.
    secondary:      [DET]
    scope:          consensus hashing (VRF range extension, nonces, header and body hashes)
    assumptions:    [A2]
    depends_on:     []
    locality:       local
    legacy:         [CN-CRYPTO-02]

#10 SEC  KesForwardSecurity  [critical]
    statement:      The producer signs with the KES key evolved to exactly the period the header claims, computed relative to the operational certificate's start period; evolution only moves forward, a key evolved past a period never signs for it, a request before the key's current period, beyond its lifetime or outside the certificate's window fails closed, and the seed material of evolved-away periods is erased.
    counterexample: The producer signs a slot in KES period 5 with a key still at period 3, signs period 3 with a key already evolved to period 4, or derives the period from the absolute slot instead of relative to the certificate's start.
    secondary:      [SAF]
    scope:          RED producer signing shell and the BLUE Sum6 KES update
    assumptions:    [A2]
    depends_on:     [SEC-001]
    locality:       local
    legacy:         [DC-CRYPTO-05, DC-CRYPTO-10]

#11 VAL  SigningMatchesReference  [critical]
    statement:      Ade's Sum6 KES and Praos VRF signing produce output byte-identical to the Haskell cardano-base reference for every input: verification-key derivation, key generation from a seed, evolution across all 64 periods, KES signatures and VRF proofs; every KES signature and VRF proof Ade emits verifies under Ade's own verifier (#8) with the matching verification key; the 608-byte expanded KES signing key serializes and deserializes byte-identically to the reference for every period from 0 to 63, with the current period inferred uniquely from the key's tree shape. The algorithm is Ade's own code: no production path builds a key of an upstream library through unsafe layout assumptions, transmute, vendored private access or fork-only constructors.
    counterexample: A seed expansion that uses different prefix bytes from cardano-base, so the derived verification key differs from the one cardano-cli derives from the same key file.
    secondary:      [DET]
    scope:          ade_crypto Sum6 KES and VRF, RED producer signing
    assumptions:    [A2, N1]
    depends_on:     [#8]
    locality:       local
    legacy:         [DC-CRYPTO-03, DC-CRYPTO-04, DC-CRYPTO-08, DC-CRYPTO-09]

#12 SEC  ClosedKeyEnvelopes  [critical]
    statement:      Signing keys and operational certificates load only through closed parsers, each a deterministic function of the file's bytes, accepting exactly three grammars: the Ade KES envelope ade.kes.seed.v1 (fields format, role, crypto, seed_32, period_idx and format_version, each validated, plus the ignored metadata fields genesis_hash, network_magic, created_at_slot and created_by, and no other field); the cardano-cli text envelope of type KesSigningKey_ed25519_kes_2^6 whose cborHex is a CBOR byte string of exactly 608 bytes, a Sum6 KES signing key whose every subtree's verification-key hashes are consistent and whose leaves are not all zero; and the cardano-cli text envelope of type NodeOperationalCertificate whose cborHex is the CBOR array [[32-byte hot key, unsigned-integer sequence number, unsigned-integer KES period, 64-byte signature], 32-byte cold key]. Any other shape, role, crypto tag, version, size, arity or field type is rejected with a structured error and no fallback, and no private-key byte appears in any error, log or telemetry output.
    counterexample: A KES envelope with an unknown field loads by ignoring the field, a 612-byte key file loads by truncation, a key whose subtree verification keys disagree loads, or a parse error message includes the seed bytes.
    secondary:      [VAL]
    scope:          RED key and certificate loading
    assumptions:    [A2, N1]
    depends_on:     [SEC-001]
    locality:       local
    legacy:         [DC-CRYPTO-06, DC-CRYPTO-07, CN-OPCERT-01, DC-OPCERT-01]

#13 SEC  HeaderPreImageSingleSource  [critical]
    statement:      A forged header's KES signature is over exactly the header-body bytes a validator extracts from the resulting block: one BLUE recipe builds the pre-image, it gives byte-identical output to the validator's extraction for every corpus block, and the signer accepts only a pre-image built by that recipe.
    counterexample: The producer signs a re-encoding of the header body whose bytes differ from what the validator extracts, so every block it forges fails KES verification at its peers.
    secondary:      [VAL, STR]
    scope:          forging (pre-image recipe, signer) and header validation (extraction)
    assumptions:    [A2]
    depends_on:     [DET-002, #10]
    locality:       local
    legacy:         [CN-KES-HEADER-01, DC-KES-HEADER-01, CN-PREIMAGE-FIXTURE-01]

#14 DET  SingleProjectEncoding  [critical]
    statement:      Every byte surface Ade defines that replay compares or that is persisted (state fingerprints, persisted records, verdict and event streams) has exactly one canonical encoding, produced by one encoder; no surface is written by two encoders or in two encodings of the same value.
    counterexample: The verdict stream is serialized by one encoder during live following and by a different one during replay, so the two streams differ in bytes for the same verdicts.
    secondary:      [REC]
    scope:          Ade-defined replay and persisted surfaces
    assumptions:    [A1]
    depends_on:     [DET-001, DET-002]
    locality:       local
    legacy:         [CN-WIRE-02]

#15 PRC  WireCompatibilityPinnedToCapturedBytes  [standard]
    statement:      Every check that Ade's wire encoding or decoding is compatible with the reference node compares against byte strings captured from a real reference node, never against Ade's own encoder output alone.
    counterexample: A test that "proves" handshake compatibility by encoding version data with Ade's encoder and decoding it with Ade's decoder, which passes even when the reference node rejects the encoding.
    secondary:      [OBS]
    scope:          wire-compatibility evidence
    assumptions:    [A2]
    depends_on:     [PRC-002]
    locality:       local
    legacy:         []
```

## New assumptions

```text
N1  nonzero-derived-kes-seeds: a KES sub-seed derived by the Sum6 seed expansion is never all zero (probability 2^-256), so a zeroed sub-seed in a stored key marks a period that has been evolved away.
```

## Redefinitions

```text
R1  SEC-001  SigningInShellVerificationInCore  weaken
    invariant:
      current:  Private key material (KES, VRF and cold keys) is loaded, held and passed only by RED shell code, and every signing or proving call is made from RED code; no BLUE transition takes a private key as input or produces a signature or VRF proof. Every signature, VRF and KES verification that decides whether a block, header or transaction is accepted is computed in BLUE code, and no RED code passes a verification verdict into a BLUE transition.
      proposed: Private key material (KES, VRF and cold keys) is loaded, stored and owned only by RED shell code, and every signing, proving or key-evolution call is made from RED code; no BLUE transition takes a private key as input or produces a signature or VRF proof. Pure signing, proving and key-evolution algorithms may live in BLUE crates as library functions, provided only RED code calls them. Every signature, VRF and KES verification that decides whether a block, header or transaction is accepted is computed in BLUE code, and no RED code passes a verification verdict into a BLUE transition.
    why:        The Sum6 KES key-evolution and signing algorithm is Ade's own pure code in ade_crypto::kes_sum, a BLUE path, so it can be checked against the reference (#11) and verified symmetrically (#8). Every caller is RED (ade_runtime::producer, ade_node). Read literally, "held and passed only by RED shell code" forbids a BLUE library function receiving a key even when only RED calls it. The change permits that, and keeps every prohibition on BLUE transitions.
    cost:       claim becomes unresolved until the migration's re-evidence pass (M2) records evidence for the new meaning; it is unresolved already.
```

## Transitions in scope

- `decode_<type>(Bytes, ProtocolVersion) -> Result<Value, DecodeError>` for each authoritative wire, ledger and storage type
- `decode_message(Bytes, NegotiatedVersion) -> Result<Message, CodecError>` for each mini-protocol
- `verify_{ed25519,vrf,kes,opcert}(Inputs) -> Result<(), VerifyError>`
- RED: `kes_advance_to(KesKey, Period) -> Result<KesKey, EvolutionError>` and `sign_header(KesKey, UnsignedHeaderPreImage) -> Signature`

## TCB color hypothesis

- BLUE: `ade_codec` (including the tag-24 authority), `ade_types`, `ade_crypto` (verification, Sum6 KES algorithm, VRF), `ade_ledger::block_validity` (pre-image recipe and extraction), and the `ade_network` codec and handshake modules in `core_paths`
- GREEN: none
- RED: `ade_runtime::producer` (key loading, envelopes, KES evolution and signing, VRF proving), `ade_runtime::admission::wire_pump` (receive-boundary unwrap call), `ade_network` session drivers

## Open questions

- S1. DC-CBOR-01 (round-trip identity for all era types) — merged into DET-003 RoundTripIdentity, which states it for every successfully decoded byte string.
- S2. DC-CBOR-02 and CN-WIRE-01 (preserve original bytes on hash and signature paths) — merged into DET-002 DesignatedByteAuthority, whose protocol-hash clause states it.
- S3. CN-WIRE-02 (one canonical project encoding for internal replay and state surfaces) — drafted as #14. DET-002 covers only commitments and persisted records, not every surface replay compares.
- S4. DC-CRYPTO-02 and CN-CRYPTO-01 (signing outside the core, verification inside) — merged into SEC-001 SigningInShellVerificationInCore, as redefined by R1.
- S5. CN-CRYPTO-03 (collections with consensus meaning ordered deterministically before hashing or comparison) — merged into DET-004 HostIndependentResults, with a stated departure. DET-004 carries the entry's purpose, that no result depends on non-canonical iteration order, and DET-002 fixes the byte order of hashed protocol objects. It does not carry the procedure "sort before every comparison": an order-independent comparison of an unordered collection stays allowed, because it has no nondeterminism to prevent.
- S6. Live acceptance of served blocks by a real peer (CN-WIRE-08's open obligation) — deferred to B10 pipeline-production and B12 resources-release, where block production and the live acceptance criteria are defined.
- S8. T-ENC-02's meaning changes from "non-canonical bytes are rejected" to "bytes outside the reference grammar are rejected; valid non-canonical CBOR is accepted and preserved", as the foundations batch's settlement S1 deferred it. Drafted as #1.
- S9. DC-CRYPTO-06's "the Ade KES envelope is the sole accepted format" was superseded within the legacy record by DC-CRYPTO-07 (PHASE4-N-P), which added the cardano-cli 608-byte key. #12 accepts both and states each grammar.
- S10. The legacy wire entries' rule that compatibility is pinned to captured reference-node bytes, never an Ade-to-Ade round trip (CN-WIRE-08, CN-WIRE-10, CN-WIRE-11) — drafted as #15.
- S7. CN-WIRE-03's version-equivalence clause (deserialization equivalent across supported versions and code paths) — drafted as #1, whose cross-version and "on any code path" clauses carry it.
```
