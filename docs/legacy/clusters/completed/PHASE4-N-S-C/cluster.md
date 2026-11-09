# PHASE4-N-S-C — Paired acceptance evidence (cluster doc)

> **Status:** Operator-dependent. The engineering surfaces
> N-S-C needs (real KES-signs-real-unsigned-header bridge +
> MuxPump outbound-relay) are CLOSED in N-S-A (HEAD `2c9d791`)
> and N-S-B (HEAD `1718976`). What remains is operator
> action: bring up a private testnet (C1) or preprod (C2),
> run `ade_node --mode produce`, capture paired evidence,
> commit the manifest.
>
> **Predecessor:** PHASE4-N-S-A + PHASE4-N-S-B (both at
> HEAD `1718976`).
>
> **Closure type:** OPERATOR-WITNESSED — cannot close in CI.

## §1 Primary invariant

> The bounty-facing paired evidence — `Ade evidence.jsonl`
> `BlockForged H` AND cardano-node peer log
> `BlockAccepted H` / `AddedToCurrentChain H` — is captured
> and committed under
> `docs/clusters/PHASE4-N-S-C/CE-N-S-LIVE_YYYYMMDD-<short_commit>.{jsonl,log,toml}`
> per the closed manifest schema (with
> `peer_log_capture_command`, `peer_log_filter`,
> `peer_log_file_sha256`, `acceptance_keyword_match`). The
> peer log is the raw `docker logs` output; the manifest's
> `peer_log_file_sha256` cross-checks the committed file's
> hash.

## §2 Slice index

| Slice | Purpose | Closure |
|---|---|---|
| **C1** | Hermetic private-testnet pass. Single-pool topology; operator controls all stake. ade_node --mode produce against private cardano-node; capture paired evidence; C1's success flips bridge open_obligations (`CN-FORGE-01`, `CN-PROD-01` final remainder, `CN-SNAPSHOT-01`). | operator-witnessed |
| **C2** | Preprod operator pass (bounty-facing strengthening). Once preprod stake provisioned for the cold key (delegation registered, snapshot active), run same pass against docker `cardano-node-preprod`. Capture paired evidence. C2's success conditionally strengthens / narrows `CN-CONS-06` and `RO-LIVE-01`. | operator-witnessed |
| **C-close** | Cluster-level close. Document remaining bounty surfaces (TxSubmission2, mempool, N2C, multi-Haskell-node) as out-of-scope-for-N-S. Refresh grounding docs. | mechanical |

## §3 Honest-scope framing

Per the invariants sketch §8 and the cluster plan §2:

- C1 (private testnet) is the **engineering bridge proof** —
  proves end-to-end that Ade can forge a block + serve it
  via N2N + cardano-node accepts. It flips the deferred-
  bridge open_obligations from N-R and the in-process
  wiring from N-S-A/B.
- C2 (preprod) is the **bounty-facing strengthening** —
  proves the same end-to-end against the real preprod
  chain. It conditionally narrows `CN-CONS-06.open_obligation`
  and `RO-LIVE-01.open_obligation`.
- **C1 is NOT a substitute for C2.** The bounty test
  surface requires preprod evidence; C1 is faster and
  hermetic but doesn't satisfy the public surface.

## §4 What N-S-C does NOT close

| Surface | Status |
|---|---|
| TxSubmission2 → mempool → block inclusion | OUT-OF-SCOPE (separate cluster) |
| N2C local-chain-sync / local-tx-submission | OUT-OF-SCOPE (separate cluster) |
| Private-testnet two-Haskell-node topology | OUT-OF-SCOPE (separate cluster) |
| Hot-key KES rotation across periods | OP-OPS-04 follow-on |
| Multi-peer concurrent forge load | future |

Empty-block forging remains the explicit scope per the
invariants sketch §9. The runbook will state this so
empty-block evidence is not misread as closing the broader
TxSubmission obligation.

## §5 Operator-action gate

**N-S-C cannot close in this commit.** The engineering
surface is complete; the operator must:

1. (C1) Bring up a private Cardano testnet with a single
   stake pool whose cold/VRF/KES keys are operator-owned.
2. Run `ade_node --mode produce --listen 127.0.0.1:3001 ...`
   with `peer_outbound: Some(new_per_peer_outbound())`
   wiring (the integration step N-S-B B4 documented).
3. Wait for a leader slot (eligibility under operator's
   stake).
4. Capture the Ade `evidence.jsonl` showing
   `BlockForged H` + the peer log showing
   `BlockAccepted H` for the same H.
5. Commit the paired manifest per the closed schema.

The runbook lives at
`docs/active/cn-cons-06-operator-runbook.md` (carried from
N-Q + extended) plus a new
`docs/clusters/PHASE4-N-S-C/CE-N-S-OPERATOR_PROCEDURE.md`
companion (operator procedure, step-list pinned to the
cluster).

Until C1 or C2 executes, the bridge open_obligation entries
(`CN-FORGE-01`, `CN-PROD-01` final remainder,
`CN-SNAPSHOT-01`) carry `blocked_until_operator_pass_executed`.

## §6 References

- N-S planning: [`../../planning/phase4-n-s-{invariants,cluster-slice-plan}.md`](../../planning/).
- N-S-A close: [`../PHASE4-N-S-A/S4.md`](../PHASE4-N-S-A/S4.md).
- N-S-B close: [`../PHASE4-N-S-B/S4.md`](../PHASE4-N-S-B/S4.md).
- N-Q operator runbook (predecessor):
  [`../../active/cn-cons-06-operator-runbook.md`](../../active/cn-cons-06-operator-runbook.md).
- Doctrine: [[feedback-shell-must-not-overstate-semantic-truth]]
  (C-close documents what's complete + what's gated;
  doesn't blindly flip CN-CONS-06 / RO-LIVE-01),
  [[reference-local-preprod-docker-cardano-node]] (C2 target),
  [[project-bounty-requirements]].
