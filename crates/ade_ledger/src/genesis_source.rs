// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! BLUE Conway-genesis → canonical-initial-state transform
//! (PHASE4-N-Y S4).
//!
//! `genesis_initial_state` is the pure transform that turns a
//! controlled Conway genesis (initial funds + initial nonce + era
//! marker) into the `(LedgerState, PraosChainDepState)` pair the
//! single closed bootstrap authority
//! ([`ade_runtime::bootstrap::bootstrap_initial_state`]) feeds into
//! its `genesis_initial` cold-start branch (CN-NODE-01). It is NOT a
//! second bootstrap authority and introduces no `*Anchor` trait /
//! plugin seam (cluster §7).
//!
//! DC-GENESIS-SRC-01: a controlled genesis enters initial state only
//! through `bootstrap_initial_state`; the genesis→initial-state
//! transform is pure / deterministic; a **non-Conway** genesis fails
//! closed (`GenesisSourceError::NonConwayEra`) — no Byron→Conway
//! historical replay path is invoked in this cluster
//! ([[RO-GENESIS-REPLAY-01]], deferred).
//!
//! Boundary: the genesis *file read / parse* is RED
//! (`ade_runtime::producer::genesis_parser`, CN-GENESIS-01); this
//! transform consumes the already-typed config and never touches the
//! filesystem, a clock, or a `String`. The initial nonce is supplied
//! by the caller because deriving it is genesis-parser business, not
//! BLUE business (mirrors [`PraosChainDepState::genesis`]).

use ade_core::consensus::praos_state::{Nonce, PraosChainDepState};
use ade_types::tx::TxIn;
use ade_types::CardanoEra;

use crate::state::LedgerState;
use crate::utxo::{TxOut, UTxOState};

/// Closed, typed initial fund: a single genesis UTxO entry. The RED
/// genesis parser derives the pseudo-`TxIn` from the genesis address;
/// the BLUE transform only inserts already-formed `(TxIn, TxOut)`
/// pairs, keeping it free of any address-hashing / `String` work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenesisInitialFund {
    pub tx_in: TxIn,
    pub tx_out: TxOut,
}

/// Closed typed representation of a controlled Conway genesis, as the
/// BLUE transform consumes it. Constructed by the RED genesis-bootstrap
/// entry from the parsed genesis file; all fields required (no
/// `Default`, no `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConwayGenesisConfig {
    /// The declared era of the controlled genesis. Anything other than
    /// `CardanoEra::Conway` fails closed (this cluster is Conway-only).
    pub era: CardanoEra,
    /// The genesis-derived initial nonce for the controlled net.
    pub initial_nonce: Nonce,
    /// The genesis initial funds, in deterministic insertion order.
    pub initial_funds: Vec<GenesisInitialFund>,
}

/// Closed error sum for the genesis→initial-state transform. Carries
/// only typed primitives; no `String` / `anyhow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisSourceError {
    /// The controlled genesis declared a non-Conway era. Fail-closed:
    /// this cluster does not invoke any Byron→Conway historical replay
    /// path ([[RO-GENESIS-REPLAY-01]]).
    NonConwayEra { found: CardanoEra },
}

/// Pure BLUE transform: a controlled Conway genesis → the
/// `(LedgerState, PraosChainDepState)` cold-start seed.
///
/// - UTxO is built from the genesis initial funds (deterministic
///   `BTreeMap` insertion).
/// - `PraosChainDepState::genesis(initial_nonce)` seeds every nonce
///   from the genesis-derived value.
/// - Era-guarded to `CardanoEra::Conway`; any other era →
///   `GenesisSourceError::NonConwayEra`, no state produced.
///
/// The result feeds `BootstrapInputs.genesis_initial` — the same
/// closed `bootstrap_initial_state` call, never a parallel path.
pub fn genesis_initial_state(
    conway_genesis: &ConwayGenesisConfig,
) -> Result<(LedgerState, PraosChainDepState), GenesisSourceError> {
    if conway_genesis.era != CardanoEra::Conway {
        return Err(GenesisSourceError::NonConwayEra {
            found: conway_genesis.era,
        });
    }

    let mut utxo = UTxOState::new();
    for fund in &conway_genesis.initial_funds {
        utxo.utxos.insert(fund.tx_in.clone(), fund.tx_out.clone());
    }

    let mut ledger = LedgerState::new(CardanoEra::Conway);
    ledger.utxo_state = utxo;

    let chain_dep = PraosChainDepState::genesis(conway_genesis.initial_nonce.clone());

    Ok((ledger, chain_dep))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    use ade_types::tx::Coin;
    use ade_types::Hash32;

    use crate::fingerprint::fingerprint;
    use crate::snapshot::{decode_snapshot, encode_snapshot};

    fn sample_config(era: CardanoEra) -> ConwayGenesisConfig {
        ConwayGenesisConfig {
            era,
            initial_nonce: Nonce(Hash32([0xCD; 32])),
            initial_funds: vec![
                GenesisInitialFund {
                    tx_in: TxIn {
                        tx_hash: Hash32([0x01; 32]),
                        index: 0,
                    },
                    tx_out: TxOut::ShelleyMary {
                        address: vec![0xAA; 29],
                        value: crate::value::Value::from_coin(Coin(1_000_000)),
                    },
                },
                GenesisInitialFund {
                    tx_in: TxIn {
                        tx_hash: Hash32([0x02; 32]),
                        index: 0,
                    },
                    tx_out: TxOut::ShelleyMary {
                        address: vec![0xBB; 29],
                        value: crate::value::Value::from_coin(Coin(2_000_000)),
                    },
                },
            ],
        }
    }

    #[test]
    fn genesis_to_initial_state_deterministic() {
        let cfg = sample_config(CardanoEra::Conway);
        let (l1, c1) = genesis_initial_state(&cfg).expect("conway genesis");
        let (l2, c2) = genesis_initial_state(&cfg).expect("conway genesis");
        // Two runs byte-identical: same ledger fingerprint, same
        // chain-dep, same canonical snapshot bytes.
        assert_eq!(fingerprint(&l1).combined, fingerprint(&l2).combined);
        assert_eq!(c1, c2);
        let s1 = encode_snapshot(&l1, &c1).expect("encode");
        let s2 = encode_snapshot(&l2, &c2).expect("encode");
        assert_eq!(s1, s2);
    }

    #[test]
    fn genesis_non_conway_fail_closed() {
        for era in [
            CardanoEra::ByronEbb,
            CardanoEra::ByronRegular,
            CardanoEra::Shelley,
            CardanoEra::Allegra,
            CardanoEra::Mary,
            CardanoEra::Alonzo,
            CardanoEra::Babbage,
        ] {
            let cfg = sample_config(era);
            match genesis_initial_state(&cfg) {
                Err(GenesisSourceError::NonConwayEra { found }) => {
                    assert_eq!(found, era);
                }
                other => panic!("expected NonConwayEra for {era:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn genesis_path_fp_equals_snapshot_path_fp() {
        // Genesis-path: build the initial state directly.
        let cfg = sample_config(CardanoEra::Conway);
        let (genesis_ledger, genesis_chain_dep) =
            genesis_initial_state(&cfg).expect("conway genesis");
        let genesis_fp = fingerprint(&genesis_ledger).combined;

        // Snapshot-path: encode → decode round-trip, then confirm the
        // re-materialized state's fingerprint equals the genesis-path
        // fingerprint (internal cross-path determinism, CE-Y-11).
        let bytes = encode_snapshot(&genesis_ledger, &genesis_chain_dep).expect("encode");
        let (decoded_ledger, decoded_chain_dep) = decode_snapshot(&bytes).expect("decode");
        let snapshot_fp = fingerprint(&decoded_ledger).combined;

        assert_eq!(genesis_fp, snapshot_fp);
        assert_eq!(genesis_chain_dep, decoded_chain_dep);
    }
}
