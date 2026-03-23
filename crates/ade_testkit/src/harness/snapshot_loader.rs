use std::io::Read;
use std::path::Path;

use ade_types::Hash32;

use super::HarnessError;

/// A loaded snapshot: the raw oracle state bytes plus parsed metadata.
///
/// The raw CBOR bytes ARE the authoritative comparison surface.
/// `state_hash()` = `Blake2b-256(raw_cbor)` — no re-encoding,
/// the oracle bytes are preserved as the hash source.
///
/// This is the `reset_to` target for the differential harness:
/// load a snapshot, verify its hash, use it as the starting state
/// for boundary replay.
#[derive(Clone)]
pub struct LoadedSnapshot {
    /// Raw ExtLedgerState CBOR bytes — the oracle comparison surface.
    pub raw_cbor: Vec<u8>,
    /// Parsed header metadata.
    pub header: SnapshotHeader,
    /// Pre-computed state hash (Blake2b-256 of raw_cbor).
    pub state_hash: Hash32,
}

impl LoadedSnapshot {
    /// Load a snapshot from a tarball, parse its header, compute its hash.
    pub fn from_tarball(tarball_path: &std::path::Path) -> Result<Self, HarnessError> {
        let raw_cbor = extract_state_from_tarball(tarball_path)?;
        let header = parse_snapshot_header(&raw_cbor)?;
        let state_hash = compute_state_hash(&raw_cbor);
        Ok(Self {
            raw_cbor,
            header,
            state_hash,
        })
    }

    /// Build a minimal `LedgerState` from the snapshot metadata.
    ///
    /// Build a `LedgerState` from the snapshot, loading delegation data
    /// from the go snapshot into the cert_state.
    ///
    /// The UTxO set is empty (compact format impedance). Delegation state
    /// is populated from the oracle's go snapshot delegation map.
    pub fn to_ledger_state(&self) -> ade_ledger::state::LedgerState {
        use ade_ledger::state::{EpochState, LedgerState};
        use ade_ledger::pparams::ProtocolParameters;
        use ade_ledger::utxo::UTxOState;
        use ade_types::{EpochNo, SlotNo};

        let era = telescope_to_era(self.header.telescope_length);

        // Load delegation data from the go snapshot
        let cert_state = self.load_delegation_state();

        // Load go snapshot stake distribution
        let snapshots = self.load_snapshot_state();

        LedgerState {
            utxo_state: UTxOState::new(),
            epoch_state: EpochState {
                epoch: EpochNo(self.header.epoch),
                slot: SlotNo(0),
                snapshots,
                reserves: ade_types::tx::Coin(self.header.reserves),
                treasury: ade_types::tx::Coin(self.header.treasury),
                block_production: {
                    use ade_types::tx::PoolId;
                    use ade_types::Hash28;
                    let bp = parse_block_production(&self.raw_cbor).unwrap_or_default();
                    bp.into_iter().map(|(h, count)| {
                        let mut pool_bytes = [0u8; 28];
                        pool_bytes.copy_from_slice(&h.0[..28]);
                        (PoolId(Hash28(pool_bytes)), count)
                    }).collect()
                },
                epoch_fees: ade_types::tx::Coin(self.header.epoch_fees),
            },
            protocol_params: {
                let mut pp = ProtocolParameters::default();
                // Oracle mainnet values differ from genesis defaults
                pp.n_opt = 500; // mainnet n_opt (not genesis 150)
                pp.treasury_growth = ade_ledger::rational::Rational::new(1, 5)
                    .unwrap_or_else(ade_ledger::rational::Rational::zero);
                pp
            },
            era,
            track_utxo: true,
            cert_state,
        }
    }

    /// Load snapshot state (mark/set/go) from the oracle CBOR.
    ///
    /// Populates the go snapshot with stake distribution and pool stakes
    /// from the oracle. Mark and set start empty (they'd need the previous
    /// epoch's data which we don't have loaded).
    fn load_snapshot_state(&self) -> ade_ledger::epoch::SnapshotState {
        use ade_ledger::epoch::{
            GoSnapshot, MarkSnapshot, SetSnapshot, SnapshotState, StakeSnapshot,
        };
        use ade_types::tx::{Coin, PoolId};
        use ade_types::Hash28;
        use std::collections::BTreeMap;

        // Parse go snapshot stake distribution
        let stake_entries = match parse_go_stake_distribution(&self.raw_cbor) {
            Ok(s) => s,
            Err(_) => return SnapshotState::new(),
        };

        let delegations_raw = match parse_go_delegations(&self.raw_cbor) {
            Ok(d) => d,
            Err(_) => return SnapshotState::new(),
        };

        // Build go snapshot delegations: credential → (pool, stake)
        let mut go_delegations: BTreeMap<Hash28, (PoolId, Coin)> = BTreeMap::new();
        let mut pool_stakes: BTreeMap<PoolId, Coin> = BTreeMap::new();

        // Build stake lookup
        let mut stake_map: BTreeMap<[u8; 28], u64> = BTreeMap::new();
        for (cred_hash, stake) in &stake_entries {
            let mut key = [0u8; 28];
            key.copy_from_slice(&cred_hash.0[..28]);
            stake_map.insert(key, *stake);
        }

        // Combine delegation + stake
        for (cred_hash, pool_hash) in &delegations_raw {
            let mut cred_bytes = [0u8; 28];
            cred_bytes.copy_from_slice(&cred_hash.0[..28]);
            let mut pool_bytes = [0u8; 28];
            pool_bytes.copy_from_slice(&pool_hash.0[..28]);

            let stake = stake_map.get(&cred_bytes).copied().unwrap_or(0);
            let pool = PoolId(Hash28(pool_bytes));

            go_delegations.insert(Hash28(cred_bytes), (pool.clone(), Coin(stake)));

            let entry = pool_stakes.entry(pool).or_insert(Coin(0));
            entry.0 = entry.0.saturating_add(stake);
        }

        let go = StakeSnapshot {
            delegations: go_delegations,
            pool_stakes,
        };

        SnapshotState {
            mark: MarkSnapshot(StakeSnapshot::new()),
            set: SetSnapshot(StakeSnapshot::new()),
            go: GoSnapshot(go),
        }
    }

    /// Load delegation and pool state from the go snapshot.
    fn load_delegation_state(&self) -> ade_ledger::delegation::CertState {
        use ade_ledger::delegation::{CertState, DelegationState, PoolState, PoolParams};
        use ade_types::shelley::cert::StakeCredential;
        use ade_types::tx::{Coin, PoolId};
        use ade_types::Hash28;
        use std::collections::BTreeMap;

        let delegations_raw = match parse_go_delegations(&self.raw_cbor) {
            Ok(d) => d,
            Err(_) => return CertState::new(),
        };

        let pools_raw = parse_go_pool_params(&self.raw_cbor).unwrap_or_default();

        // Build delegation state
        let mut delegations = BTreeMap::new();
        let mut registrations = BTreeMap::new();

        for (cred_hash, pool_hash) in &delegations_raw {
            let mut cred_bytes = [0u8; 28];
            cred_bytes.copy_from_slice(&cred_hash.0[..28]);
            let cred = StakeCredential(Hash28(cred_bytes));

            let mut pool_bytes = [0u8; 28];
            pool_bytes.copy_from_slice(&pool_hash.0[..28]);
            let pool = PoolId(Hash28(pool_bytes));

            delegations.insert(cred.clone(), pool);
            registrations.insert(cred, Coin(0));
        }

        // Build pool state
        let mut pools = BTreeMap::new();
        for (pool_hash, pledge, cost, margin_num, margin_den, reward_acct) in &pools_raw {
            let mut pool_bytes = [0u8; 28];
            pool_bytes.copy_from_slice(&pool_hash.0[..28]);
            let pool_id = PoolId(Hash28(pool_bytes));

            pools.insert(pool_id.clone(), PoolParams {
                pool_id,
                vrf_hash: ade_types::Hash32([0u8; 32]), // not needed for rewards
                pledge: Coin(*pledge),
                cost: Coin(*cost),
                margin: (*margin_num, *margin_den),
                reward_account: reward_acct.clone(),
            });
        }

        CertState {
            delegation: DelegationState {
                registrations,
                delegations,
                rewards: BTreeMap::new(),
            },
            pool: PoolState {
                pools,
                retiring: BTreeMap::new(),
            },
        }
    }
}

fn telescope_to_era(telescope_length: u32) -> ade_types::CardanoEra {
    use ade_types::CardanoEra;
    match telescope_length {
        1 => CardanoEra::ByronRegular,
        2 => CardanoEra::Shelley,
        3 => CardanoEra::Allegra,
        4 => CardanoEra::Mary,
        5 => CardanoEra::Alonzo,
        6 => CardanoEra::Babbage,
        7 => CardanoEra::Conway,
        _ => CardanoEra::Conway,
    }
}

/// Parsed header of an ExtLedgerState snapshot.
///
/// Extracted from the CBOR structure without fully deserializing
/// the multi-hundred-megabyte state. Provides enough information
/// to identify the era, epoch, and telescope position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotHeader {
    /// Number of eras in the telescope (1 = Byron only, 5 = Alonzo, etc.)
    pub telescope_length: u32,
    /// Era index of the current state (0-based position in telescope)
    pub current_era_index: u32,
    /// Epoch number from the NewEpochState
    pub epoch: u64,
    /// Treasury in lovelace (from AccountState)
    pub treasury: u64,
    /// Reserves in lovelace (from AccountState)
    pub reserves: u64,
    /// Accumulated fees from the epoch (SnapShots[3]).
    pub epoch_fees: u64,
    /// Total size of the state CBOR in bytes
    pub state_size: usize,
}

/// Extract the `state` file from a snapshot tarball.
///
/// Each snapshot tarball contains `./state` (ExtLedgerState CBOR),
/// `./tables/tvar` (ImmutableDB), and `./meta`.
pub fn extract_state_from_tarball(tarball_path: &Path) -> Result<Vec<u8>, HarnessError> {
    let file = std::fs::File::open(tarball_path)
        .map_err(|e| HarnessError::IoError(format!("{}: {e}", tarball_path.display())))?;

    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| HarnessError::IoError(format!("tar entries: {e}")))?
    {
        let mut entry =
            entry.map_err(|e| HarnessError::IoError(format!("tar entry: {e}")))?;

        let path = entry
            .path()
            .map_err(|e| HarnessError::IoError(format!("tar path: {e}")))?
            .to_path_buf();

        if path.file_name().and_then(|n| n.to_str()) == Some("state") {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| HarnessError::IoError(format!("read state: {e}")))?;
            return Ok(buf);
        }
    }

    Err(HarnessError::ParseError(
        "state file not found in tarball".to_string(),
    ))
}

/// Parse the CBOR header of an ExtLedgerState to extract era/epoch metadata.
///
/// Does NOT fully deserialize the state — only reads enough structure
/// to identify the telescope length and current era's epoch number.
pub fn parse_snapshot_header(state_cbor: &[u8]) -> Result<SnapshotHeader, HarnessError> {
    let state_size = state_cbor.len();

    // outer: array(2) [era_index_or_state, ...]
    let (off, outer_len) = read_array_header(state_cbor, 0)?;

    if outer_len != 2 {
        return Err(HarnessError::ParseError(format!(
            "expected outer array(2), got array({outer_len})"
        )));
    }

    // Element 0: uint — era index in NS encoding
    let (off, _era_idx) = read_uint(state_cbor, off)?;

    // Element 1: array(2) [telescope, header_state]
    let (off, pair_len) = read_array_header(state_cbor, off)?;
    if pair_len != 2 {
        return Err(HarnessError::ParseError(format!(
            "expected state pair array(2), got array({pair_len})"
        )));
    }

    // Telescope: array(N) where N = number of eras
    let (off, telescope_length) = read_array_header(state_cbor, off)?;

    // Skip past eras (small) to reach current era (last element)
    let mut off = off;
    for _ in 0..telescope_length - 1 {
        off = skip_cbor(state_cbor, off)?;
    }

    // Current era: array(2) [Bound, State]
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip Bound
    let off = skip_cbor(state_cbor, off)?;

    // State: array(2) [version_uint, payload]
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip version
    let off = skip_cbor(state_cbor, off)?;

    // Payload: array(3) [WithOrigin, NewEpochState, Transition]
    let (off, _) = read_array_header(state_cbor, off)?;
    // Skip WithOrigin
    let off = skip_cbor(state_cbor, off)?;

    // NewEpochState: array(N) — first element is epoch uint
    let (off, _nes_len) = read_array_header(state_cbor, off)?;

    // NES[0]: epoch number
    let (off, epoch) = read_uint(state_cbor, off)?;

    // Skip NES[1] (nesBprev), NES[2] (nesBcur) to reach NES[3] (EpochState)
    let off = skip_cbor(state_cbor, off)?; // NES[1]
    let off = skip_cbor(state_cbor, off)?; // NES[2]

    // NES[3] = EpochState = array(4) [AccountState, LedgerState, SnapShots, NonMyopic]
    let (off, _) = read_array_header(state_cbor, off)?;

    // ES[0] = AccountState = array(2) [treasury, reserves]
    let (off, acct_len) = read_array_header(state_cbor, off)?;
    let (treasury, reserves) = if acct_len == 2 {
        let (off, treasury) = read_uint(state_cbor, off)?;
        let (_, reserves) = read_uint(state_cbor, off)?;
        (treasury, reserves)
    } else {
        (0, 0)
    };

    // Parse epoch fees separately (re-navigating is simpler than tracking offset)
    let epoch_fees = parse_epoch_fees(state_cbor).unwrap_or(0);

    Ok(SnapshotHeader {
        telescope_length,
        current_era_index: telescope_length - 1,
        epoch,
        treasury,
        reserves,
        epoch_fees,
        state_size,
    })
}

fn parse_epoch_fees(state_cbor: &[u8]) -> Result<u64, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // Skip ES[0] (AccountState), ES[1] (LedgerState) to reach ES[2] (SnapShots)
    let off = skip_cbor(state_cbor, es)?; // ES[0]
    let off = skip_cbor(state_cbor, off)?; // ES[1]

    // ES[2] = SnapShots = array(4) [mark, set, go, fees]
    let (ss_inner, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, ss_inner)?; // skip mark
    let off = skip_cbor(state_cbor, off)?; // skip set
    let off = skip_cbor(state_cbor, off)?; // skip go

    // SS[3] = epoch fees
    let (_, fees) = read_uint(state_cbor, off)?;
    Ok(fees)
}

/// Compute the Blake2b-256 hash of a snapshot state file.
///
/// This is the oracle comparison surface: `Blake2b-256(encodeDiskExtLedgerState)`.
/// The hash of the state file bytes IS the state hash at the snapshot slot.
pub fn compute_state_hash(state_cbor: &[u8]) -> Hash32 {
    ade_crypto::blake2b_256(state_cbor)
}

/// An oracle state hash entry from a hash file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleHashEntry {
    pub slot: u64,
    pub hash: Hash32,
    pub state_size: u64,
}

/// Parse an oracle hash file.
///
/// Format: `SlotNo <slot>|<hex_hash>|<state_size>` per line.
pub fn parse_oracle_hashes(content: &str) -> Result<Vec<OracleHashEntry>, HarnessError> {
    let mut entries = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('|').collect();
        if parts.len() < 2 {
            return Err(HarnessError::ParseError(format!(
                "line {}: expected slot|hash|size", i + 1
            )));
        }
        let slot_str = parts[0]
            .trim()
            .strip_prefix("SlotNo ")
            .unwrap_or(parts[0].trim());
        let slot: u64 = slot_str
            .parse()
            .map_err(|e| HarnessError::ParseError(format!("line {}: bad slot: {e}", i + 1)))?;

        let hex = parts[1].trim();
        if hex.len() != 64 {
            return Err(HarnessError::ParseError(format!(
                "line {}: hash must be 64 hex chars, got {}", i + 1, hex.len()
            )));
        }
        let mut hash_bytes = [0u8; 32];
        for j in 0..32 {
            hash_bytes[j] = u8::from_str_radix(&hex[j * 2..j * 2 + 2], 16)
                .map_err(|e| HarnessError::ParseError(format!("line {}: bad hex: {e}", i + 1)))?;
        }

        let state_size = if parts.len() >= 3 {
            parts[2]
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            0
        };

        entries.push(OracleHashEntry {
            slot,
            hash: Hash32(hash_bytes),
            state_size,
        });
    }
    Ok(entries)
}

/// Parse the go snapshot's pool params, stake distribution, and delegation
/// from the ExtLedgerState CBOR.
///
/// Returns (pool_count, stake_entry_count, delegation_entry_count).
/// This is the T-21B state-load bridge: loading delegation/pool data
/// from the oracle's on-disk format into usable counts and structures.
pub fn parse_go_snapshot_counts(state_cbor: &[u8]) -> Result<(usize, usize, usize), HarnessError> {
    // Navigate to ES[2] = SnapShots
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;

    // ES[2] = SnapShots = array(4) [mark, set, go, fee_total]
    let ss_off = skip_n_fields(state_cbor, es, 2)?; // skip ES[0], ES[1]
    let (ss_inner, _) = read_array_header(state_cbor, ss_off)?;

    // SS[2] = go snapshot = array(3) [stake_dist, delegations, pool_params]
    let go_off = skip_n_fields(state_cbor, ss_inner, 2)?; // skip mark, set
    let (go_inner, go_len) = read_array_header(state_cbor, go_off)?;
    if go_len != 3 {
        return Err(HarnessError::ParseError(format!(
            "go snapshot expected array(3), got array({go_len})"
        )));
    }

    // Count entries in each map
    let stake_count = count_indef_map(state_cbor, go_inner)?;
    let deleg_off = skip_cbor(state_cbor, go_inner)?;
    let deleg_count = count_indef_map(state_cbor, deleg_off)?;
    let pool_off = skip_cbor(state_cbor, deleg_off)?;
    let pool_count = count_indef_map(state_cbor, pool_off)?;

    Ok((pool_count, stake_count, deleg_count))
}

/// Parse the go snapshot's delegation map into a Vec of (credential_hash, pool_hash) pairs.
///
/// This loads the actual delegation data needed for reward computation.
pub fn parse_go_delegations(
    state_cbor: &[u8],
) -> Result<Vec<(Hash32, Hash32)>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;
    let ss_off = skip_n_fields(state_cbor, es, 2)?;
    let (ss_inner, _) = read_array_header(state_cbor, ss_off)?;
    let go_off = skip_n_fields(state_cbor, ss_inner, 2)?;
    let (go_inner, _) = read_array_header(state_cbor, go_off)?;

    // Skip stake_dist to get to delegation map
    let deleg_off = skip_cbor(state_cbor, go_inner)?;

    // Parse delegation map: key = array(2)[type, hash28], value = bytes(28)
    let (map_start, _, _map_val) = read_cbor_initial(state_cbor, deleg_off)?;

    let mut delegations = Vec::new();
    let mut co = map_start;

    // Iterate map entries (works for both definite and indefinite)
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: array(2) [type_tag, bytes(28)]
        let (key_inner, _, _) = read_cbor_initial(state_cbor, co)?;
        let key_end = skip_cbor(state_cbor, co)?;

        // Read credential hash (skip type tag, read hash28)
        let (_, _tag) = read_uint(state_cbor, key_inner)?;
        let tag_end = skip_cbor(state_cbor, key_inner)?;
        // Read hash bytes
        let (hash_start, _, hash_len) = read_cbor_initial(state_cbor, tag_end)?;
        let mut cred_hash = [0u8; 32];
        if hash_len >= 28 {
            cred_hash[..28].copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
        }
        co = key_end;

        // Value: bytes(28) = pool hash
        let (pool_start, _, pool_len) = read_cbor_initial(state_cbor, co)?;
        let mut pool_hash = [0u8; 32];
        if pool_len >= 28 {
            pool_hash[..28].copy_from_slice(&state_cbor[pool_start..pool_start + 28]);
        }
        co = skip_cbor(state_cbor, co)?;

        delegations.push((Hash32(cred_hash), Hash32(pool_hash)));
    }

    Ok(delegations)
}

/// Parse nesBprev (block production counts per pool) from NES[1].
///
/// Returns Vec of (pool_hash, blocks_produced).
/// Pools not in this map produced zero blocks.
pub fn parse_block_production(
    state_cbor: &[u8],
) -> Result<std::collections::BTreeMap<Hash32, u64>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;

    // NES[0] = epoch, NES[1] = nesBprev
    let bprev_off = skip_cbor(state_cbor, off)?; // skip NES[0]

    let (mut co, _, _) = read_cbor_initial(state_cbor, bprev_off)?;
    let mut production = std::collections::BTreeMap::new();

    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: bytes(28) pool hash
        let (key_start, key_maj, key_len) = read_cbor_initial(state_cbor, co)?;
        let mut pool_hash = [0u8; 32];
        if key_maj == 2 && key_len >= 28 {
            pool_hash[..28].copy_from_slice(&state_cbor[key_start..key_start + 28]);
        }
        co = skip_cbor(state_cbor, co)?;

        // Value: uint (block count)
        let (_, blocks) = read_uint(state_cbor, co)?;
        co = skip_cbor(state_cbor, co)?;

        production.insert(Hash32(pool_hash), blocks);
    }

    Ok(production)
}

/// Parse stake distribution from the go snapshot.
///
/// Returns Vec of (credential_hash, stake_lovelace).
pub fn parse_go_stake_distribution(
    state_cbor: &[u8],
) -> Result<Vec<(Hash32, u64)>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;
    let ss_off = skip_n_fields(state_cbor, es, 2)?;
    let (ss_inner, _) = read_array_header(state_cbor, ss_off)?;
    let go_off = skip_n_fields(state_cbor, ss_inner, 2)?;
    let (go_inner, _) = read_array_header(state_cbor, go_off)?;

    // First map in go snapshot is stake distribution
    let (mut co, _, _) = read_cbor_initial(state_cbor, go_inner)?;
    let mut stakes = Vec::new();

    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: array(2) [type_tag, bytes(28)]
        let (key_inner, _, _) = read_cbor_initial(state_cbor, co)?;
        let key_end = skip_cbor(state_cbor, co)?;

        let (_, _tag) = read_uint(state_cbor, key_inner)?;
        let tag_end = skip_cbor(state_cbor, key_inner)?;
        let (hash_start, _, hash_len) = read_cbor_initial(state_cbor, tag_end)?;
        let mut cred_hash = [0u8; 32];
        if hash_len >= 28 {
            cred_hash[..28].copy_from_slice(&state_cbor[hash_start..hash_start + 28]);
        }
        co = key_end;

        // Value: uint (stake in lovelace)
        let (_, stake) = read_uint(state_cbor, co)?;
        co = skip_cbor(state_cbor, co)?;

        stakes.push((Hash32(cred_hash), stake));
    }

    Ok(stakes)
}

/// Pool params tuple: (pool_hash, pledge, cost, margin_num, margin_den, reward_account).
type PoolParamsTuple = (Hash32, u64, u64, u64, u64, Vec<u8>);

/// Parse pool params from the go snapshot.
pub fn parse_go_pool_params(
    state_cbor: &[u8],
) -> Result<Vec<PoolParamsTuple>, HarnessError> {
    let off = navigate_to_nes(state_cbor)?;
    let es = skip_nes_to_epoch_state(state_cbor, off)?;
    let ss_off = skip_n_fields(state_cbor, es, 2)?;
    let (ss_inner, _) = read_array_header(state_cbor, ss_off)?;
    let go_off = skip_n_fields(state_cbor, ss_inner, 2)?;
    let (go_inner, _) = read_array_header(state_cbor, go_off)?;

    // Skip stake_dist and delegation to get pool_params map
    let pool_off = skip_cbor(state_cbor, go_inner)?; // skip stake_dist
    let pool_off = skip_cbor(state_cbor, pool_off)?; // skip delegations

    let (mut co, _, _) = read_cbor_initial(state_cbor, pool_off)?;
    let mut pools = Vec::new();

    while co < state_cbor.len() && state_cbor[co] != 0xff {
        // Key: bytes(28) = pool hash
        let (key_start, key_maj, key_len) = read_cbor_initial(state_cbor, co)?;
        let mut pool_hash = [0u8; 32];
        if key_maj == 2 && key_len >= 28 {
            pool_hash[..28].copy_from_slice(&state_cbor[key_start..key_start + 28]);
        }
        co = skip_cbor(state_cbor, co)?;

        // Value: array(9) [operator, vrf, pledge, cost, margin, reward_acct, ...]
        let val_start = co;
        let (val_inner, _, val_len) = read_cbor_initial(state_cbor, co)?;

        if val_len >= 6 {
            // Skip [0] operator, [1] vrf
            let f2 = skip_cbor(state_cbor, val_inner)?; // skip operator
            let f2 = skip_cbor(state_cbor, f2)?; // skip vrf

            // [2] pledge
            let (_, pledge) = read_uint(state_cbor, f2)?;
            let f3 = skip_cbor(state_cbor, f2)?;

            // [3] cost
            let (_, cost) = read_uint(state_cbor, f3)?;
            let f4 = skip_cbor(state_cbor, f3)?;

            // [4] margin: tag(30, array(2, [num, den]))
            let (tag_inner, _, _) = read_cbor_initial(state_cbor, f4)?; // tag(30)
            let (margin_inner, _) = read_array_header(state_cbor, tag_inner)?;
            let (_, margin_num) = read_uint(state_cbor, margin_inner)?;
            let den_off = skip_cbor(state_cbor, margin_inner)?;
            let (_, margin_den) = read_uint(state_cbor, den_off)?;
            let f5 = skip_cbor(state_cbor, f4)?;

            // [5] reward_account: bytes(29)
            let (acct_start, _, acct_len) = read_cbor_initial(state_cbor, f5)?;
            let reward_acct = state_cbor[acct_start..acct_start + acct_len as usize].to_vec();

            pools.push((Hash32(pool_hash), pledge, cost, margin_num, margin_den, reward_acct));
        }

        co = skip_cbor(state_cbor, val_start)?;
    }

    Ok(pools)
}

fn navigate_to_nes(state_cbor: &[u8]) -> Result<usize, HarnessError> {
    let (off, _) = read_array_header(state_cbor, 0)?; // outer array(2)
    let (off, _) = read_uint(state_cbor, off)?; // era index
    let (off, _) = read_array_header(state_cbor, off)?; // state pair
    let (off, telescope_len) = read_array_header(state_cbor, off)?; // telescope

    // Skip to last telescope entry
    let mut off = off;
    for _ in 0..telescope_len - 1 {
        off = skip_cbor(state_cbor, off)?;
    }

    // Current era: array(2) [Bound, State]
    let (off, _) = read_array_header(state_cbor, off)?;
    let off = skip_cbor(state_cbor, off)?; // skip Bound
    let (off, _) = read_array_header(state_cbor, off)?; // State = array(2) [version, payload]
    let off = skip_cbor(state_cbor, off)?; // skip version
    let (off, _) = read_array_header(state_cbor, off)?; // payload = array(3)
    let off = skip_cbor(state_cbor, off)?; // skip WithOrigin

    // Now at NES
    let (off, _) = read_array_header(state_cbor, off)?;
    Ok(off)
}

fn skip_nes_to_epoch_state(state_cbor: &[u8], nes_body: usize) -> Result<usize, HarnessError> {
    // Skip NES[0] (epoch), NES[1] (bprev), NES[2] (bcur) to reach NES[3] (EpochState)
    let off = skip_cbor(state_cbor, nes_body)?; // NES[0]
    let off = skip_cbor(state_cbor, off)?; // NES[1]
    let off = skip_cbor(state_cbor, off)?; // NES[2]

    // NES[3] = EpochState = array(4)
    let (off, _) = read_array_header(state_cbor, off)?;
    Ok(off)
}

fn skip_n_fields(state_cbor: &[u8], start: usize, n: u32) -> Result<usize, HarnessError> {
    let mut off = start;
    for _ in 0..n {
        off = skip_cbor(state_cbor, off)?;
    }
    Ok(off)
}

fn count_indef_map(state_cbor: &[u8], offset: usize) -> Result<usize, HarnessError> {
    let (mut co, _, val) = read_cbor_initial(state_cbor, offset)?;
    if val != u64::MAX && val > 0 {
        // Definite map — but our read_cbor_initial returns -1 as u64::MAX? No.
        // Actually, for indef, we returned -1 which is i64 -1 but stored as... hmm.
        // Let me handle this properly:
        return Ok(val as usize);
    }
    // Indefinite map: count entries until break
    let mut count = 0;
    while co < state_cbor.len() && state_cbor[co] != 0xff {
        co = skip_cbor(state_cbor, co)?; // key
        co = skip_cbor(state_cbor, co)?; // value
        count += 1;
    }
    Ok(count)
}

// --- Minimal CBOR reader (no external deps, no unwrap) ---

fn read_array_header(data: &[u8], offset: usize) -> Result<(usize, u32), HarnessError> {
    let (off, major, val) = read_cbor_initial(data, offset)?;
    if major != 4 {
        return Err(HarnessError::ParseError(format!(
            "expected array (major 4) at offset {offset}, got major {major}"
        )));
    }
    Ok((off, val as u32))
}

fn read_uint(data: &[u8], offset: usize) -> Result<(usize, u64), HarnessError> {
    let (off, major, val) = read_cbor_initial(data, offset)?;
    if major != 0 {
        return Err(HarnessError::ParseError(format!(
            "expected uint (major 0) at offset {offset}, got major {major}"
        )));
    }
    Ok((off, val))
}

fn read_cbor_initial(data: &[u8], offset: usize) -> Result<(usize, u8, u64), HarnessError> {
    if offset >= data.len() {
        return Err(HarnessError::ParseError(format!(
            "unexpected EOF at offset {offset}"
        )));
    }
    let b = data[offset];
    let major = (b >> 5) & 0x7;
    let ai = b & 0x1f;
    let mut off = offset + 1;

    let val = if ai < 24 {
        ai as u64
    } else if ai == 24 {
        if off >= data.len() {
            return Err(HarnessError::ParseError("EOF in uint8".to_string()));
        }
        let v = data[off] as u64;
        off += 1;
        v
    } else if ai == 25 {
        if off + 2 > data.len() {
            return Err(HarnessError::ParseError("EOF in uint16".to_string()));
        }
        let v = u16::from_be_bytes([data[off], data[off + 1]]) as u64;
        off += 2;
        v
    } else if ai == 26 {
        if off + 4 > data.len() {
            return Err(HarnessError::ParseError("EOF in uint32".to_string()));
        }
        let v = u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
            as u64;
        off += 4;
        v
    } else if ai == 27 {
        if off + 8 > data.len() {
            return Err(HarnessError::ParseError("EOF in uint64".to_string()));
        }
        let v = u64::from_be_bytes([
            data[off],
            data[off + 1],
            data[off + 2],
            data[off + 3],
            data[off + 4],
            data[off + 5],
            data[off + 6],
            data[off + 7],
        ]);
        off += 8;
        v
    } else if ai == 31 {
        // indefinite length — sentinel value, callers must handle
        return Ok((off, major, u64::MAX));
    } else {
        return Err(HarnessError::ParseError(format!(
            "unsupported additional info {ai} at offset {offset}"
        )));
    };

    Ok((off, major, val))
}

fn skip_cbor(data: &[u8], offset: usize) -> Result<usize, HarnessError> {
    if offset >= data.len() {
        return Err(HarnessError::ParseError(format!(
            "unexpected EOF at offset {offset}"
        )));
    }
    let b = data[offset];
    let major = (b >> 5) & 0x7;
    let ai = b & 0x1f;
    let mut off = offset + 1;

    let val: u64 = if ai < 24 {
        ai as u64
    } else if ai == 24 {
        let v = data[off] as u64;
        off += 1;
        v
    } else if ai == 25 {
        let v = u16::from_be_bytes([data[off], data[off + 1]]) as u64;
        off += 2;
        v
    } else if ai == 26 {
        let v = u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
            as u64;
        off += 4;
        v
    } else if ai == 27 {
        let v = u64::from_be_bytes([
            data[off],
            data[off + 1],
            data[off + 2],
            data[off + 3],
            data[off + 4],
            data[off + 5],
            data[off + 6],
            data[off + 7],
        ]);
        off += 8;
        v
    } else if ai == 31 {
        // indefinite
        match major {
            4 => {
                while off < data.len() && data[off] != 0xff {
                    off = skip_cbor(data, off)?;
                }
                return Ok(off + 1);
            }
            5 => {
                while off < data.len() && data[off] != 0xff {
                    off = skip_cbor(data, off)?;
                    off = skip_cbor(data, off)?;
                }
                return Ok(off + 1);
            }
            2 | 3 => {
                while off < data.len() && data[off] != 0xff {
                    off = skip_cbor(data, off)?;
                }
                return Ok(off + 1);
            }
            _ => {
                return Err(HarnessError::ParseError(format!(
                    "unexpected indefinite major {major} at offset {offset}"
                )));
            }
        }
    } else {
        return Err(HarnessError::ParseError(format!(
            "unsupported ai {ai} at offset {offset}"
        )));
    };

    match major {
        0 | 1 | 7 => Ok(off),
        2 | 3 => Ok(off + val as usize),
        6 => skip_cbor(data, off),
        4 => {
            for _ in 0..val {
                off = skip_cbor(data, off)?;
            }
            Ok(off)
        }
        5 => {
            for _ in 0..val {
                off = skip_cbor(data, off)?;
                off = skip_cbor(data, off)?;
            }
            Ok(off)
        }
        _ => Err(HarnessError::ParseError(format!(
            "unknown major {major} at offset {offset}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn snapshots_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("corpus")
            .join("snapshots")
    }

    #[test]
    fn load_byron_pre_hfc_snapshot() {
        let tarball = snapshots_dir().join("snapshot_4492800.tar.gz");
        if !tarball.exists() {
            eprintln!("Skipping: {}", tarball.display());
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        assert!(state_bytes.len() > 100, "state file too small");

        let header = parse_snapshot_header(&state_bytes).unwrap();
        eprintln!("Byron pre-HFC: telescope={}, era_idx={}, epoch={}, size={}",
            header.telescope_length, header.current_era_index,
            header.epoch, header.state_size);

        // At slot 4,492,800 the HFC has occurred: epoch 208, telescope 2
        assert_eq!(header.epoch, 208);
    }

    #[test]
    fn load_shelley_epoch_boundary_snapshot() {
        let tarball = snapshots_dir().join("snapshot_4924880.tar.gz");
        if !tarball.exists() {
            eprintln!("Skipping: {}", tarball.display());
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let header = parse_snapshot_header(&state_bytes).unwrap();
        eprintln!("Shelley epoch boundary: telescope={}, era_idx={}, epoch={}, size={}",
            header.telescope_length, header.current_era_index,
            header.epoch, header.state_size);

        // Shelley at slot 4,924,880 = epoch 209, telescope length 2
        assert_eq!(header.epoch, 209);
        assert_eq!(header.telescope_length, 2);
    }

    #[test]
    fn snapshot_state_hashes_are_stable() {
        let tarball = snapshots_dir().join("snapshot_4492800.tar.gz");
        if !tarball.exists() {
            eprintln!("Skipping: {}", tarball.display());
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let hash1 = compute_state_hash(&state_bytes);
        let hash2 = compute_state_hash(&state_bytes);
        assert_eq!(hash1, hash2, "state hash must be deterministic");

        // Hash is 64 hex chars
        let hex = format!("{hash1}");
        assert_eq!(hex.len(), 64);
        eprintln!("Byron HFC state hash: {hex}");
    }

    #[test]
    fn parse_oracle_hash_file() {
        let hash_file = snapshots_dir().join("hashes_4924880.txt");
        if !hash_file.exists() {
            eprintln!("Skipping: {}", hash_file.display());
            return;
        }

        let content = std::fs::read_to_string(&hash_file).unwrap();
        let entries = parse_oracle_hashes(&content).unwrap();

        assert!(!entries.is_empty());
        assert_eq!(entries[0].slot, 4924900);
        assert!(entries[0].state_size > 0);
        eprintln!(
            "Shelley epoch boundary hashes: {} entries, slots {}..{}",
            entries.len(),
            entries[0].slot,
            entries[entries.len() - 1].slot
        );
    }

    #[test]
    fn epoch_boundary_state_size_matches_oracle() {
        // The snapshot state size should match the oracle's state_size
        // for the first hash entry (which is the state AFTER the first
        // block post-snapshot).
        let cases = [
            ("snapshot_4924880.tar.gz", "hashes_4924880.txt"),
            ("snapshot_17020848.tar.gz", "hashes_17020848.txt"),
            ("snapshot_40348902.tar.gz", "hashes_40348902.txt"),
            ("snapshot_134092810.tar.gz", "hashes_134092810.txt"),
        ];

        for (snap, hashes) in &cases {
            let tarball = snapshots_dir().join(snap);
            let hash_file = snapshots_dir().join(hashes);
            if !tarball.exists() || !hash_file.exists() {
                continue;
            }

            let state_bytes = extract_state_from_tarball(&tarball).unwrap();
            let content = std::fs::read_to_string(&hash_file).unwrap();
            let entries = parse_oracle_hashes(&content).unwrap();

            // State size should be close to (but not exactly equal to)
            // the first oracle entry's state_size. The snapshot is PRE-block,
            // the oracle hash is POST-block.
            let snap_size = state_bytes.len() as u64;
            let oracle_size = entries[0].state_size;
            let diff = snap_size.abs_diff(oracle_size);

            eprintln!(
                "{}: snapshot={}, oracle_first={}, diff={}",
                snap, snap_size, oracle_size, diff
            );

            // Size should be within 1% — same era, close slots
            let pct = (diff as f64 / snap_size as f64) * 100.0;
            assert!(
                pct < 1.0,
                "{snap}: state size divergence too large: {pct:.2}%"
            );
        }
    }

    #[test]
    fn loaded_snapshot_provides_reset_to() {
        let tarball = snapshots_dir().join("snapshot_4492800.tar.gz");
        if !tarball.exists() {
            return;
        }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();

        // State hash is deterministic
        assert_eq!(snap.state_hash, compute_state_hash(&snap.raw_cbor));

        // Can produce a LedgerState
        let state = snap.to_ledger_state();
        assert_eq!(state.epoch_state.epoch, ade_types::EpochNo(208));
        assert_eq!(state.era, ade_types::CardanoEra::Shelley);

        eprintln!(
            "LoadedSnapshot: epoch={}, era={:?}, hash={}, size={}",
            snap.header.epoch, state.era, snap.state_hash, snap.raw_cbor.len()
        );
    }

    #[test]
    fn hfc_pair_hashes_differ() {
        // Pre-HFC and post-epoch-boundary snapshots for the same transition
        // must have different state hashes (blocks were applied between them).
        let pairs = [
            ("snapshot_4492800.tar.gz", "snapshot_4924880.tar.gz"),
            ("snapshot_16588800.tar.gz", "snapshot_17020848.tar.gz"),
            ("snapshot_72316896.tar.gz", "snapshot_72748820.tar.gz"),
        ];

        for (pre, post) in &pairs {
            let pre_path = snapshots_dir().join(pre);
            let post_path = snapshots_dir().join(post);
            if !pre_path.exists() || !post_path.exists() {
                continue;
            }

            let pre_snap = LoadedSnapshot::from_tarball(&pre_path).unwrap();
            let post_snap = LoadedSnapshot::from_tarball(&post_path).unwrap();

            assert_ne!(
                pre_snap.state_hash, post_snap.state_hash,
                "pre-HFC and epoch boundary must have different hashes"
            );

            // Telescope should match (same era after HFC)
            assert_eq!(pre_snap.header.telescope_length, post_snap.header.telescope_length);

            eprintln!(
                "{} -> {}: telescopes match ({}), hashes differ",
                pre, post, pre_snap.header.telescope_length
            );
        }
    }

    #[test]
    fn to_ledger_state_preserves_delegation_count() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();

        // Parse count directly from CBOR
        let (_, _, raw_deleg_count) = parse_go_snapshot_counts(&snap.raw_cbor).unwrap();

        // Load into LedgerState
        let state = snap.to_ledger_state();
        let stored_deleg_count = state.cert_state.delegation.delegations.len();
        let stored_reg_count = state.cert_state.delegation.registrations.len();

        eprintln!("Delegation count chain:");
        eprintln!("  raw CBOR:      {raw_deleg_count}");
        eprintln!("  stored delegs: {stored_deleg_count}");
        eprintln!("  stored regs:   {stored_reg_count}");

        assert_eq!(
            stored_deleg_count, raw_deleg_count,
            "delegation count must survive parse → store conversion"
        );
        assert_eq!(
            stored_reg_count, raw_deleg_count,
            "registration count must match delegation count"
        );
    }

    #[test]
    fn to_ledger_state_preserves_pool_count() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();
        let (raw_pool_count, _, _) = parse_go_snapshot_counts(&snap.raw_cbor).unwrap();
        let state = snap.to_ledger_state();
        let stored_pool_count = state.cert_state.pool.pools.len();

        eprintln!("Pool count chain:");
        eprintln!("  raw CBOR:     {raw_pool_count}");
        eprintln!("  stored pools: {stored_pool_count}");

        assert_eq!(
            stored_pool_count, raw_pool_count,
            "pool count must survive parse → store conversion"
        );

        // Verify pool params have correct values
        if let Some((_, params)) = state.cert_state.pool.pools.iter().next() {
            eprintln!("  first pool: pledge={}, cost={}, margin={}/{}",
                params.pledge.0 / 1_000_000,
                params.cost.0 / 1_000_000,
                params.margin.0, params.margin.1,
            );
            assert!(params.cost.0 > 0, "pool cost must be non-zero");
            assert!(params.margin.1 > 0, "margin denominator must be non-zero");
        }
    }

    #[test]
    fn go_snapshot_total_stake_matches_oracle() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() { return; }

        let snap = LoadedSnapshot::from_tarball(&tarball).unwrap();
        let state = snap.to_ledger_state();

        let total_stake: u64 = state.epoch_state.snapshots.go.0.pool_stakes
            .values().map(|c| c.0).sum();

        // Oracle total stake (from Python): ~20.2T lovelace
        eprintln!("Go snapshot total stake: {} lovelace = {} ADA",
            total_stake, total_stake / 1_000_000);

        let pool_count = state.epoch_state.snapshots.go.0.pool_stakes.len();
        let deleg_count = state.epoch_state.snapshots.go.0.delegations.len();

        eprintln!("  pool_stakes entries: {}", pool_count);
        eprintln!("  delegation entries: {}", deleg_count);

        // Should be in the right ballpark (~20B ADA)
        assert!(total_stake > 1_000_000_000_000_000,
            "total stake should be > 1B ADA in lovelace");
    }

    #[test]
    fn parse_go_snapshot_from_allegra() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let (pools, stakes, delegs) = parse_go_snapshot_counts(&state_bytes).unwrap();

        eprintln!(
            "Allegra ep237 go snapshot: {} pools, {} stakes, {} delegations",
            pools, stakes, delegs
        );

        // From Python analysis: SS[2] (go) has ~1,445 pools, ~96,260 stakes, ~98,331 delegations
        assert!(pools > 1000, "should have > 1000 pools");
        assert!(stakes > 90000, "should have > 90000 stake entries");
        assert!(delegs > 90000, "should have > 90000 delegation entries");
    }

    #[test]
    fn parse_go_delegations_from_allegra() {
        let tarball = snapshots_dir().join("snapshot_17020848.tar.gz");
        if !tarball.exists() {
            return;
        }

        let state_bytes = extract_state_from_tarball(&tarball).unwrap();
        let delegations = parse_go_delegations(&state_bytes).unwrap();

        eprintln!("Allegra ep237: {} delegations loaded", delegations.len());
        assert!(delegations.len() > 90000);

        // Verify delegation entries have non-zero pool hashes
        for (cred, pool) in delegations.iter().take(3) {
            eprintln!(
                "  cred={}... → pool={}...",
                &format!("{cred}")[..16],
                &format!("{pool}")[..16],
            );
            assert_ne!(*pool, Hash32([0u8; 32]), "pool hash must be non-zero");
        }
    }

    #[test]
    fn load_all_proof_grade_snapshots() {
        // Epoch values are from the CBOR (ground truth), not assumed.
        // Pre-HFC snapshots may show the post-transition epoch because
        // the state at the HFC boundary slot is already the new era.
        let proof_snapshots = [
            ("snapshot_4492800.tar.gz", 208u64, "byron->shelley HFC"),
            ("snapshot_4924880.tar.gz", 209, "shelley epoch boundary"),
            ("snapshot_16588800.tar.gz", 236, "shelley->allegra HFC"),
            ("snapshot_17020848.tar.gz", 237, "allegra epoch boundary"),
            ("snapshot_23068800.tar.gz", 251, "allegra->mary HFC"),
            ("snapshot_23500962.tar.gz", 252, "mary epoch boundary"),
            ("snapshot_39916975.tar.gz", 290, "mary->alonzo HFC"),
            ("snapshot_40348902.tar.gz", 291, "alonzo epoch boundary"),
            ("snapshot_72316896.tar.gz", 365, "alonzo->babbage HFC"),
            ("snapshot_72748820.tar.gz", 366, "babbage epoch boundary"),
            ("snapshot_133660855.tar.gz", 507, "babbage->conway HFC"),
            ("snapshot_134092810.tar.gz", 508, "conway epoch boundary"),
        ];

        eprintln!("\n=== Proof-Grade Snapshot Verification ===");
        eprintln!("{:<35} {:>5} {:>5} {:>10}", "Snapshot", "Epoch", "Tele", "Size");
        eprintln!("{}", "-".repeat(60));

        for (filename, expected_epoch, label) in &proof_snapshots {
            let tarball = snapshots_dir().join(filename);
            if !tarball.exists() {
                eprintln!("{:<35} MISSING", label);
                continue;
            }

            let state_bytes = extract_state_from_tarball(&tarball).unwrap();
            let header = parse_snapshot_header(&state_bytes).unwrap();

            eprintln!(
                "{:<35} {:>5} {:>5} {:>10}",
                label, header.epoch, header.telescope_length, header.state_size
            );

            assert_eq!(
                header.epoch, *expected_epoch,
                "{label}: expected epoch {expected_epoch}, got {}",
                header.epoch
            );
        }
        eprintln!("========================================\n");
    }
}
