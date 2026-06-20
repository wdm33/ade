// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! EPOCH-CONSENSUS-VIEW S3b-1 (DC-EVIEW-04) — the reduced-UTxO record + the Conway
//! reduction (the pure BLUE core of the durable reduced-UTxO checkpoint).
//!
//! Option B (the disk-backed reduced-UTxO checkpoint) stores, per live output, only
//! `(Coin, ReducedStakeRef)` — datums, scripts, and multi-assets are dropped. Ade is
//! a Conway-era node and only ever snapshots at Conway, where pointer stake is
//! retired and ONLY base-address stake credentials contribute; so the reduced stake
//! reference is the Conway-specialized `Base(StakeCredential) | NonContributing`
//! (option b), derived by reusing Slice-2's classifier at Conway:
//! `classify_output_stake_ref(addr, Conway)` → `Base(cred)` contributes, everything
//! else (pointer-retired / enterprise / Byron / reward / malformed) is
//! `NonContributing`. The era gate is therefore trivially satisfied here; the general
//! era-parameterized pointer machinery (S3a) stays available for tx-validity.
//!
//! Canonical serialization (for the durable checkpoint + its replay-equivalence
//! fingerprint): `ReducedStakeRef` is `0x00` (NonContributing) / `0x01‖hash28`
//! (Base key) / `0x02‖hash28` (Base script); a record is `TxIn(32+2 BE) ‖ coin(8 BE)
//! ‖ ReducedStakeRef`.

use ade_types::shelley::cert::StakeCredential;
use ade_types::tx::{Coin, TxIn};
use ade_types::CardanoEra;
use ade_types::Hash28;

use crate::stake_ref::{classify_output_stake_ref, StakeRefClass};
use crate::utxo::TxOut;

/// The Conway-specialized reduced stake reference (option b): only a base-address
/// stake credential contributes at a Conway snapshot; every other form contributes
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReducedStakeRef {
    Base(StakeCredential),
    NonContributing,
}

const TAG_NON_CONTRIBUTING: u8 = 0x00;
const TAG_BASE_KEY: u8 = 0x01;
const TAG_BASE_SCRIPT: u8 = 0x02;

impl ReducedStakeRef {
    /// Append the canonical encoding.
    pub fn encode(&self, buf: &mut Vec<u8>) {
        match self {
            ReducedStakeRef::NonContributing => buf.push(TAG_NON_CONTRIBUTING),
            ReducedStakeRef::Base(StakeCredential::KeyHash(h)) => {
                buf.push(TAG_BASE_KEY);
                buf.extend_from_slice(&h.0);
            }
            ReducedStakeRef::Base(StakeCredential::ScriptHash(h)) => {
                buf.push(TAG_BASE_SCRIPT);
                buf.extend_from_slice(&h.0);
            }
        }
    }

    /// Decode from the head of `bytes`; returns the value and the number of bytes
    /// consumed. Fail-closed on an unknown tag or a truncated hash.
    pub fn decode(bytes: &[u8]) -> Option<(ReducedStakeRef, usize)> {
        match bytes.first().copied()? {
            TAG_NON_CONTRIBUTING => Some((ReducedStakeRef::NonContributing, 1)),
            tag @ (TAG_BASE_KEY | TAG_BASE_SCRIPT) => {
                let h = bytes.get(1..29)?;
                let mut arr = [0u8; 28];
                arr.copy_from_slice(h);
                let cred = if tag == TAG_BASE_KEY {
                    StakeCredential::KeyHash(Hash28(arr))
                } else {
                    StakeCredential::ScriptHash(Hash28(arr))
                };
                Some((ReducedStakeRef::Base(cred), 29))
            }
            _ => None,
        }
    }
}

/// Reduce a `TxOut` to `(Coin, ReducedStakeRef)` for the Conway stake checkpoint.
/// Reuses the Slice-2 classifier at Conway: a base address's staking credential
/// contributes; everything else is `NonContributing`.
pub fn reduce_txout(out: &TxOut) -> (Coin, ReducedStakeRef) {
    let coin = out.coin();
    let reduced = match classify_output_stake_ref(out.address_bytes(), CardanoEra::Conway) {
        StakeRefClass::Base(cred) => ReducedStakeRef::Base(cred),
        // Conway pointer (retired) / enterprise / Byron / reward / malformed.
        StakeRefClass::Pointer(_) | StakeRefClass::Null | StakeRefClass::Reject(_) => {
            ReducedStakeRef::NonContributing
        }
    };
    (coin, reduced)
}

/// The canonical bytes of one reduced checkpoint record: `TxIn ‖ coin ‖ ref`. The
/// durable store persists this and folds it (in `TxIn` order) into the checkpoint
/// fingerprint, so the fingerprint is a pure function of the reduced UTxO
/// (replay-equivalent).
pub fn encode_reduced_record(txin: &TxIn, coin: Coin, reduced: &ReducedStakeRef) -> Vec<u8> {
    let mut buf = Vec::with_capacity(34 + 8 + 29);
    buf.extend_from_slice(&txin.tx_hash.0);
    buf.extend_from_slice(&txin.index.to_be_bytes());
    buf.extend_from_slice(&coin.0.to_be_bytes());
    reduced.encode(&mut buf);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use ade_types::address::Address;
    use ade_types::Hash32;

    fn hx(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // A real preview base address (type 0): stake key hash = bytes [29..57].
    const BASE_T0: &str = "001b9cdd4e5a6cb08501e46e10551b7a859d4a98251009bb91d69785f65691d68ad87582fc89b9ac43fd0227cfa4108efb791b9987b290a9ba";
    const BASE_STAKE: &str = "5691d68ad87582fc89b9ac43fd0227cfa4108efb791b9987b290a9ba";

    #[test]
    fn base_output_reduces_to_base_credential() {
        let out = TxOut::AlonzoPlus {
            raw: vec![],
            address: hx(BASE_T0),
            coin: Coin(1_000_000),
        };
        let (coin, r) = reduce_txout(&out);
        assert_eq!(coin, Coin(1_000_000));
        assert_eq!(
            r,
            ReducedStakeRef::Base(StakeCredential::KeyHash(
                Hash28(hx(BASE_STAKE).try_into().unwrap())
            ))
        );
    }

    #[test]
    fn enterprise_and_byron_are_non_contributing() {
        // enterprise type 6 (29 bytes).
        let ent = TxOut::ShelleyMary {
            address: {
                let mut v = vec![0x60u8];
                v.extend(std::iter::repeat(0x11).take(28));
                v
            },
            value: Value::from_coin(Coin(5)),
        };
        assert_eq!(reduce_txout(&ent).1, ReducedStakeRef::NonContributing);

        let byron = TxOut::Byron {
            address: Address::Byron(vec![0x99]),
            coin: Coin(7),
        };
        assert_eq!(reduce_txout(&byron).1, ReducedStakeRef::NonContributing);
    }

    #[test]
    fn pointer_output_is_non_contributing_at_conway() {
        // a pointer address (type 4) -> Conway-retired -> NonContributing.
        let mut addr = vec![0x40u8];
        addr.extend(std::iter::repeat(0xaa).take(28));
        addr.extend_from_slice(&[0x01, 0x02, 0x03]);
        let out = TxOut::AlonzoPlus { raw: vec![], address: addr, coin: Coin(9) };
        assert_eq!(reduce_txout(&out).1, ReducedStakeRef::NonContributing);
    }

    #[test]
    fn reduced_stake_ref_round_trips_canonically() {
        for r in [
            ReducedStakeRef::NonContributing,
            ReducedStakeRef::Base(StakeCredential::KeyHash(Hash28([0xab; 28]))),
            ReducedStakeRef::Base(StakeCredential::ScriptHash(Hash28([0xcd; 28]))),
        ] {
            let mut buf = Vec::new();
            r.encode(&mut buf);
            let (back, n) = ReducedStakeRef::decode(&buf).expect("decode");
            assert_eq!(back, r);
            assert_eq!(n, buf.len());
        }
    }

    #[test]
    fn decode_fails_closed_on_bad_tag_or_truncation() {
        assert!(ReducedStakeRef::decode(&[]).is_none());
        assert!(ReducedStakeRef::decode(&[0x09]).is_none()); // unknown tag
        assert!(ReducedStakeRef::decode(&[0x01, 0x00]).is_none()); // truncated hash
    }

    #[test]
    fn record_encoding_is_deterministic_and_canonical() {
        let txin = TxIn { tx_hash: Hash32([0x11; 32]), index: 7 };
        let r = ReducedStakeRef::Base(StakeCredential::KeyHash(Hash28([0x22; 28])));
        let a = encode_reduced_record(&txin, Coin(42), &r);
        let b = encode_reduced_record(&txin, Coin(42), &r);
        assert_eq!(a, b);
        // TxIn(34) + coin(8) + ref(29) = 71 bytes.
        assert_eq!(a.len(), 34 + 8 + 29);
        assert_eq!(&a[0..32], &[0x11; 32]); // tx_hash
        assert_eq!(&a[32..34], &7u16.to_be_bytes()); // index
        assert_eq!(&a[34..42], &42u64.to_be_bytes()); // coin
        assert_eq!(a[42], TAG_BASE_KEY);
    }
}
