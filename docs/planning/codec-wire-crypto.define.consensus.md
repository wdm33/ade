# Consensus record — docs/planning/codec-wire-crypto.define.md

Append one section per round (Doctrine §21). Never rewrite a round. `received` must equal `blob` (idse.md §1.2 step 1). `blob` is the document's reviewed-text hash when the round ran: `python3 -I ~/.claude/methodology/tools/idse-doc-hash.py <document>` (idse.md §1.2 step 5). Quote Codex verbatim. Round numbers continue across the record. A round right after an `Outcome: consensus` line starts a new run, and the round limit counts the rounds of the current run (idse.md §1.2 step 4).

## Round 1 — 2026-11-08 — docs/planning/codec-wire-crypto.define.md @ blob 431b16a5d002d215c318ff23536e01c3c2f9c0d3 · received 431b16a5d002d215c318ff23536e01c3c2f9c0d3

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 1 | #1 scope | "the reference node defines no wire grammar for Ade-specific WAL or snapshot records; the statement also omits storage types from its opening quantifier." | revised: #1 separates reference-defined wire and ledger grammars from Ade-defined, closed, versioned storage grammars, and quantifies over both | open |
| 2 | S3 | "DET-002 requires canonical encoding for Ade commitments and persisted records, but does not require one encoding for every internal replay/state surface" | revised: CN-WIRE-02 drafted as new line #14 SingleProjectEncoding; S3 updated | open |
| 3 | S5 | "An order-independent comparison of an unordered collection satisfies DET-004 but violates the legacy requirement." | rebuttal: the legacy entry's stated purpose (its attack_rationale: "Prevents set and map iteration nondeterminism") is carried in full by DET-004, which forbids any BLUE result depending on non-canonical iteration order. An order-independent comparison has no nondeterminism to prevent, so requiring an ordering step before it adds a procedure, not a property. Hashing order for protocol objects is fixed by DET-002 (wire bytes). The merge stands. | open |
| 4 | S7 | "One decoder per version does not establish deserialization equivalence across supported versions: two individually unique decoders can disagree." | revised: #1 adds that where grammar and meaning are unchanged between two supported versions, both decoders agree, and every difference between versions is one the reference node also makes | open |
| 5 | #11 and #8 | "neither guarantees signing/verification symmetry for every valid input." | revised: #11 requires every emitted KES signature and VRF proof to verify under Ade's own verifier with the matching key | open |
| 6 | #11 assumptions | "The cited tree-shape proof expressly assumes that present derived seeds are nonzero." | revised: new assumption N1 nonzero-derived-kes-seeds, cited by #11 and #12; #12 rejects keys whose leaves are all zero | open |
| 7 | #12 assumptions | "Compatibility with cardano-cli KES and operational-certificate envelopes relies on the reference semantics covered by A2, but the line cites no assumptions." | revised: #12 cites A2 and N1 | open |
| 8 | #12 statement | "“Closed parsers” does not enumerate the accepted grammars or structural checks." | revised: #12 enumerates the three accepted grammars with their fields, sizes, arities and structural checks; the counterexample adds inconsistent subtree keys | open |
| 9 | Open questions: T-ENC-02 and DC-CRYPTO-06 | "changes deterministic rejection of noncanonical bytes … and replaces the native-envelope-only policy with multiple accepted key formats, without recording these departures" | revised: S8 records the T-ENC-02 change (as the foundations batch's S1 set it up); S9 records that DC-CRYPTO-07 superseded DC-CRYPTO-06's "sole format" within the legacy record | open |
| 10 | #11 and #13 family | "Signing correctness and confinement of the header-signing pre-image defend cryptographic authority against adversarial misuse, but both use VAL as primary" | revised for #13: SEC primary (the signer accepting only the recipe's pre-image restricts what the key will sign), VAL and STR secondary. rebuttal for #11: byte-identical signing must hold with no adversary present; a wrong output makes peers reject Ade's blocks, a validity failure of produced objects, so it keeps VAL under Doctrine §6 ("Invariants that must hold with no adversary present … keep their own family") | open |
| 11 | #11: DC-CRYPTO-08 disposition | "does not preserve the legacy prohibition on compatibility shims using unsafe layouts, transmute, vendored private access or fork-only constructors." | revised: #11 adds that the algorithm is Ade's own code and no production path builds an upstream key that way | open |
| 12 | #4, #6 and #7 | "The legacy wire entries explicitly require byte-pinning against captured reference-node fixtures and reject Ade-to-Ade round trips as sufficient evidence." | revised: new line #15 WireCompatibilityPinnedToCapturedBytes (PRC); S10 records it | open |

Outcome: continue

## Round 2 — 2026-11-08 — docs/planning/codec-wire-crypto.define.md @ blob 415de629943d2b78d23fcdfe85f97610f577bedf · received 415de629943d2b78d23fcdfe85f97610f577bedf

Round 1 items 1, 2, 4–7 and 9–12: Codex marked each `accepted`.

| # | target | critique (Codex, verbatim excerpt) | resolution (Claude) | status |
|---|---|---|---|---|
| 3 | S5 | "DET-004 permits order-independent comparison without ordering; the legacy statement requires deterministic ordering before comparison. … explicitly settle its removal as a departure" | revised: S5 now records the merge with an explicit departure: the purpose is carried by DET-004 and DET-002, and the "sort before every comparison" procedure is not, with the reason | open |
| 8 | #12 statement | "#12 specifies the expanded KES payload but omits the cardano-cli envelope type KesSigningKey_ed25519_kes_2^6 and its CBOR-byte-string framing; opcert sequence number and KES period also lack explicit unsigned-integer constraints." | revised: #12 names both cardano-cli text-envelope types, the 608-byte CBOR byte-string framing, and unsigned-integer sequence number and KES period | open |
| 13 | #10–#11, TCB, S4 | "BLUE-owned KES evolution and signing take private keys and produce signatures, contradicting SEC-001’s approved prohibition … draft an explicit redefinition with the exact current and proposed fields and record the boundary decision for human approval." | revised: R1 redefines SEC-001 (nature: weaken), with the exact current text from the registry and the proposed text permitting pure signing, proving and evolution algorithms in BLUE crates when only RED calls them; S4 now points at SEC-001 as redefined by R1. All non-test callers checked: ade_runtime::producer and ade_node (RED) | open |
| 14 | #1 cross-version clause | "requiring every version difference to be one the reference node makes, although it defines no Ade storage formats." | revised: the reference rule applies to wire and ledger types across protocol versions; storage format version differences are those Ade's format definitions declare | open |

Outcome: continue

## Round 3 — 2026-11-08 — docs/planning/codec-wire-crypto.define.md @ blob d108e065ccf45efd24732b2e87155099f42a466f · received d108e065ccf45efd24732b2e87155099f42a466f

Round 2 items 3, 8, 13 and 14: Codex marked each `accepted`. Codex: "no new items".

Outcome: consensus

## Batch 1 — 2026-11-08 — after round 3 @ blob d108e065ccf45efd24732b2e87155099f42a466f

```text
Define batch — codec-wire-crypto (B02) — consensus: round 3 (docs/planning/codec-wire-crypto.define.consensus.md)

 1. VAL  ClosedGrammarDecoding                   [critical]  One decoder per type and version accepts exactly the reference grammar (or Ade's own storage grammar); everything else is rejected before any transition, with no fallback parser.
 2. STR  NamedIngressChokepoints                 [critical]  Network, N2C and storage-rehydration bytes reach the core only through the named decode chokepoint for their type and era.
 3. EVO  ClosedVersionedMessages                 [standard]  Each mini-protocol message decodes into exactly one variant of a closed type for the negotiated version; anything else ends the session.
 4. VAL  Tag24EnvelopeAuthority                  [standard]  One BLUE authority builds and strips block-fetch/chain-sync tag-24 envelopes in the reference shape, unwrapping received blocks exactly once.
 5. VAL  PrevHashClosedSum                       [standard]  The previous-hash field is Genesis (null) or Block(hash); block 0 carries Genesis, every other block a hash, never a placeholder.
 6. VAL  N2NHandshakeWireCompatibility           [standard]  N2N handshake data uses the reference grammar per version (4 fields for 11–15, plus Peras from 16), one shared encoder both ways.
 7. VAL  FindIntersectCompatibility              [standard]  MsgFindIntersect point lists decode in definite and indefinite form; the reply is the first listed point on the served chain (Origin always counts).
 8. SEC  VerificationMatchesReference            [critical]  Ed25519, VRF, KES and opcert verification accept exactly what the reference accepts; a failed check is final, never retried another way.
 9. SEC  HashDomainSeparation                    [critical]  Each consensus hash uses its protocol-defined prefix and layout, so a value for one purpose can't stand in for another.
10. SEC  KesForwardSecurity                      [critical]  The producer signs with the key evolved to exactly the header's period (relative to the opcert start); evolution only moves forward, and old seed material is erased.
11. VAL  SigningMatchesReference                 [critical]  KES and VRF signing are byte-identical to cardano-base, emitted proofs verify under Ade's own verifier, and the 608-byte key round-trips for every period.
12. SEC  ClosedKeyEnvelopes                      [critical]  Keys and opcerts load only through three enumerated closed grammars; anything else is rejected, and private bytes never reach errors or logs.
13. SEC  HeaderPreImageSingleSource              [critical]  The KES signature covers exactly the header bytes validators extract; the signer accepts only a pre-image from the one recipe.
14. DET  SingleProjectEncoding                   [critical]  Every Ade-defined surface that replay compares or that is persisted has exactly one canonical encoding and one encoder.
15. PRC  WireCompatibilityPinnedToCapturedBytes  [standard]  Wire-compatibility checks compare against bytes captured from a real reference node, never Ade-to-Ade round trips alone.

New assumptions (approved with the lines that cite them):
 N1  nonzero-derived-kes-seeds: a KES sub-seed from the Sum6 expansion is never all zero (probability 2^-256), so a zeroed sub-seed marks an evolved-away period — cited by 11, 12

Redefinitions (the full current and proposed text is in the draft):
 R1. SEC-001  SigningInShellVerificationInCore  weaken  Pure signing, proving and key-evolution algorithms may live in BLUE crates as library functions, provided only RED code calls them; every prohibition on BLUE transitions stays.
      SEC-001's claim stays unresolved until the migration's re-evidence pass (M2) records evidence for the new meaning.

Settled in consensus:
 S1. DC-CBOR-01 — merged into DET-003 RoundTripIdentity.
 S2. DC-CBOR-02, CN-WIRE-01 — merged into DET-002 DesignatedByteAuthority (wire bytes on hash and signature paths).
 S3. CN-WIRE-02 — drafted as 14 (DET-002 covers only commitments and persisted records).
 S4. DC-CRYPTO-02, CN-CRYPTO-01 — merged into SEC-001 as redefined by R1.
 S5. CN-CRYPTO-03 — merged into DET-004, with a stated departure: its purpose (no dependence on iteration order) is carried; the "sort before every comparison" procedure is not.
 S6. Live acceptance of served blocks by a real peer — deferred to B10 pipeline-production and B12 resources-release.
 S7. CN-WIRE-03 version equivalence — drafted into 1.
 S8. T-ENC-02's meaning changes to "bytes outside the reference grammar are rejected; valid non-canonical CBOR is accepted and preserved" — drafted as 1.
 S9. DC-CRYPTO-06's "sole envelope format" — already superseded in the legacy record by DC-CRYPTO-07; 12 accepts both and states each grammar.
 S10. Pin wire compatibility to captured reference bytes — drafted as 15.

Legacy dispositions (37): 1←T-ENC-02, CN-WIRE-03, CN-WIRE-04, CN-WIRE-05 · 2←T-INGRESS-01, DC-INGRESS-01, DC-INGRESS-02, CN-WIRE-06 · 3←CN-WIRE-07 · 4←CN-WIRE-08, CN-WIRE-12 · 5←CN-WIRE-09 · 6←CN-WIRE-10 · 7←CN-WIRE-11 · 8←DC-CRYPTO-01, CN-CRYPTO-04 · 9←CN-CRYPTO-02 · 10←DC-CRYPTO-05, DC-CRYPTO-10 · 11←DC-CRYPTO-03, DC-CRYPTO-04, DC-CRYPTO-08, DC-CRYPTO-09 · 12←DC-CRYPTO-06, DC-CRYPTO-07, CN-OPCERT-01, DC-OPCERT-01 · 13←CN-KES-HEADER-01, DC-KES-HEADER-01, CN-PREIMAGE-FIXTURE-01 · 14←CN-WIRE-02 · merged: DC-CBOR-01→DET-003; DC-CBOR-02, CN-WIRE-01→DET-002; DC-CRYPTO-02, CN-CRYPTO-01→SEC-001; CN-CRYPTO-03→DET-004

Reply "approve all", or per line: "1 ok", "2 reject", "3 amend: <text>", ranges like "1-2 ok".
Answer the new assumption and the redefinition the same way: "N1 ok", "R1 ok", "R1 reject", "R1 amend: <text>".
A settlement stands unless you write "reopen S1: <direction>".
```

Human reply: "approve all"

The registry lint rejected line 1 as approved: its secondary families listed SEC, which Doctrine §6 forbids (a security-related invariant takes SEC as its primary family). The line was put back to the human before any commit.

Human reply: "1 amend: drop SEC from secondary"
