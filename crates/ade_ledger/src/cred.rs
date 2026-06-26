// Core Contract:
// - Deterministic: same inputs => same output
// - No wall-clock, randomness, HashMap/HashSet, or floats
// - Encode invariants in types

//! THE single source of truth for the stake-credential CBOR discriminant.
//!
//! cardano-ledger encodes a `Credential` as `array(2)[tag, hash28]` with **tag 0 = KeyHash,
//! tag 1 = ScriptHash**. Every native-state CBOR decoder — the LedgerDB `state` decode
//! (`ledgerdb_state`), the snapshot cert-state decode (`snapshot::cert_state`), and the governance
//! decode (`snapshot::gov_state`) — MUST map `(tag, hash28)` through this ONE function, so the
//! convention cannot drift between decoders.
//!
//! Why this matters: a flipped discriminant in one decoder silently mislabels ~every delegator
//! (key<->script). The delegation map then no longer joins the UTxO (which is classified from
//! addresses), so the per-pool stake collapses (~10% of real) and the cross-epoch leader schedule
//! rejects valid blocks. This exact flip lived in `ledgerdb_state::read_credential` and broke
//! continuous operation across an epoch boundary; consolidating the convention here makes a
//! recurrence a one-line change in one place, pinned by the test below.

use ade_types::shelley::cert::StakeCredential;
use ade_types::Hash28;

/// Map a cardano-ledger `Credential` CBOR tag + its 28-byte hash to a [`StakeCredential`].
/// `tag 0 => KeyHash`, `tag 1 => ScriptHash`. Returns `None` for any other tag so the caller fails
/// closed rather than guessing.
#[inline]
pub fn stake_credential_from_ledger_tag(tag: u64, hash: Hash28) -> Option<StakeCredential> {
    match tag {
        0 => Some(StakeCredential::KeyHash(hash)),
        1 => Some(StakeCredential::ScriptHash(hash)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_ledger_credential_tags() {
        // 0 = KeyHash, 1 = ScriptHash. If this ever flips, every native-bootstrap delegator is
        // mislabeled and the seed+2 leader schedule collapses. This is the codebase-wide pin.
        assert_eq!(
            stake_credential_from_ledger_tag(0, Hash28([0xaa; 28])),
            Some(StakeCredential::KeyHash(Hash28([0xaa; 28]))),
        );
        assert_eq!(
            stake_credential_from_ledger_tag(1, Hash28([0xbb; 28])),
            Some(StakeCredential::ScriptHash(Hash28([0xbb; 28]))),
        );
        assert_eq!(stake_credential_from_ledger_tag(2, Hash28([0; 28])), None);
    }
}
