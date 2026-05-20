// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Closed, era-versioned required-signer enumeration (DC-TXV-05).
//!
//! `required_signers` returns the complete set of `Hash28` key hashes a
//! transaction must have a vkey witness for, partitioned by a CLOSED
//! [`SignerSource`] enum. A signer source not in the enum is impossible
//! to silently omit; adding a source is an explicit, versioned change.
//!
//! ## Spec grounding (Conway)
//!
//! The enumeration mirrors `getConwayWitsVKeyNeeded`
//! (`Cardano.Ledger.Conway.UTxO`):
//!
//! ```text
//! getConwayWitsVKeyNeeded utxo txBody =
//!     getShelleyWitsVKeyNeededNoGov utxo txBody     -- inputs, collateral,
//!                                                    -- withdrawals, certs
//!   `Set.union` Set.map asWitness (reqSignerHashes) -- field k14
//!   `Set.union` voterWitnesses txBody               -- field k19
//! ```
//!
//! ### 1. Resolved input / collateral payment credentials
//!
//! `getShelleyWitsVKeyNeededNoGov` (`Cardano.Ledger.Shelley.UTxO`) folds
//! spendable inputs: `Addr _ (KeyHashObj pay) _ -> insert pay`. Only a
//! key-hash payment credential contributes; a script-hash payment
//! credential is a phase-2 / native-script obligation (NOT a vkey
//! signer). Collateral inputs are spendable inputs too, hence the same
//! rule. Byron bootstrap inputs use the BootstrapWitness path, which is
//! out of this slice's scope; they contribute no Ed25519 vkey signer
//! here (documented scope boundary, exercised on real UTxO in B2-S3).
//!
//! ### 2. Explicit required_signers (k14)
//!
//! Each entry is a raw `addr_keyhash` and is always required
//! (`Set.map asWitness reqSignerHashes`).
//!
//! ### 3. Withdrawal reward-account credentials (k5)
//!
//! `wdrlAuthors`: each reward account's credential contributes via
//! `credKeyHashWitness` — key-hash → required, script-hash → not.
//!
//! ### 4. Certificate key hashes (per cert kind)
//!
//! `getVKeyWitnessConwayTxCert` (`Cardano.Ledger.Conway.TxCert`). Per the
//! Conway CDDL cert discriminants:
//!
//! | tag | cert | required key hash |
//! |-----|------|-------------------|
//! | 0  | account_registration (RegCert SNothing) | NONE (transitional: no witness) |
//! | 1  | account_unregistration | stake credential (key-hash) |
//! | 2  | delegation_to_stake_pool | stake credential (key-hash) |
//! | 3  | pool_registration | operator pool key + all pool_owners |
//! | 4  | pool_retirement | pool key hash |
//! | 7  | account_registration_deposit | stake credential (key-hash) |
//! | 8  | account_unregistration_deposit | stake credential (key-hash) |
//! | 9  | delegation_to_drep | stake credential (key-hash) |
//! | 10 | delegation_to_stake_pool_and_drep | stake credential (key-hash) |
//! | 11 | reg_delegation_to_stake_pool | stake credential (key-hash) |
//! | 12 | reg_delegation_to_drep | stake credential (key-hash) |
//! | 13 | reg_delegation_to_stake_pool_and_drep | stake credential (key-hash) |
//! | 14 | committee_authorization | committee COLD credential (key-hash) |
//! | 15 | committee_resignation | committee COLD credential (key-hash) |
//! | 16 | drep_registration | DRep credential (key-hash) |
//! | 17 | drep_unregistration | DRep credential (key-hash) |
//! | 18 | drep_update | DRep credential (key-hash) |
//!
//! Script-hash credentials in tags 1/2/7..18 contribute NO vkey signer
//! (`credKeyHashWitness` returns `Nothing`). Shelley tags 5/6 (genesis
//! delegation, MIR) are removed in Conway and rejected structurally
//! upstream; they are not part of this enumeration.
//!
//! ### 5. Governance voters (k19)
//!
//! `voterWitnesses`: each voter contributes its credential's key hash.
//! Per the CDDL `voter` array, the leading tag selects the credential
//! kind: 0/2/4 = `addr_keyhash` (required), 1/3 = `script_hash` (NOT a
//! vkey signer). Tags map to committee-hot (0/1), DRep (2/3), and
//! stake-pool (4) voters; the stake-pool voter is always a key hash.

use std::collections::{BTreeMap, BTreeSet};

use ade_codec::cbor::{self, ContainerEncoding};
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::TxIn;
use ade_types::{CardanoEra, Hash28};

/// A spent (or collateral) output's resolved payment information.
///
/// `address` is the raw output address bytes exactly as stored in the
/// UTxO. The required-signer derivation classifies its payment
/// credential (key-hash vs script-hash) from the header byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOutput {
    pub address: Vec<u8>,
}

/// Resolved inputs supplied to [`required_signers`].
///
/// Maps each spent input AND collateral input to its resolved output so
/// the payment credential can be derived. An input that the caller could
/// not resolve is simply absent from the map — [`required_signers`]
/// turns that into a structured [`RequiredSignerError::UnresolvableInput`]
/// fail-fast, never a silent skip.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedInputs {
    pub resolved: BTreeMap<TxIn, ResolvedOutput>,
}

impl ResolvedInputs {
    pub fn new() -> Self {
        ResolvedInputs {
            resolved: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, input: TxIn, output: ResolvedOutput) {
        self.resolved.insert(input, output);
    }
}

/// The CLOSED, era-versioned enumeration of required-signer sources.
///
/// Every required `Hash28` is tagged with the source that demanded it,
/// so a missing-signer rejection can report WHICH obligation failed.
/// Adding a source is an explicit, versioned addition — an omitted
/// source is impossible because this enum is exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignerSource {
    /// Resolved spend-input payment key-hash credential.
    InputPaymentKey,
    /// Explicit required_signers field (tx body k14).
    ExplicitRequiredSigner,
    /// Withdrawal reward-account key-hash credential (k5).
    WithdrawalKey,
    /// Certificate key hash, per cert kind (certs in k4).
    CertificateKey,
    /// Governance voter key-hash credential (voting_procedures, k19).
    GovernanceVoter,
    /// Collateral-input payment key-hash credential (k11), script txs.
    CollateralPaymentKey,
}

/// The closed set of required key hashes plus per-key provenance.
///
/// `keys` is the deduplicated coverage requirement; `provenance` records
/// every (source, key) demand so a rejection names the failing source.
/// A single key required by two sources appears once in `keys` and twice
/// in `provenance`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequiredSigners {
    pub keys: BTreeSet<Hash28>,
    pub provenance: BTreeSet<(SignerSource, Hash28)>,
}

impl RequiredSigners {
    fn require(&mut self, source: SignerSource, key: Hash28) {
        self.keys.insert(key.clone());
        self.provenance.insert((source, key));
    }

    /// All sources that demanded `key` (for diagnostics / per-source gates).
    pub fn sources_for(&self, key: &Hash28) -> BTreeSet<SignerSource> {
        self.provenance
            .iter()
            .filter(|(_, k)| k == key)
            .map(|(s, _)| *s)
            .collect()
    }
}

/// Closed failure taxonomy for required-signer derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequiredSignerError {
    /// A spend or collateral input could not be resolved against the
    /// supplied UTxO. Fail-fast — never a silent "assume covered".
    UnresolvableInput { input: TxIn },
    /// A field that must parse (certs / withdrawals / voters CBOR) was
    /// malformed. Fail-closed — never skip the obligation.
    MalformedField {
        field: RequiredSignerField,
        offset: usize,
    },
    /// The era is not supported by this closed enumeration.
    UnsupportedEra { era: CardanoEra },
}

/// Which tx field failed to parse during derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredSignerField {
    Certificates,
    Withdrawals,
    VotingProcedures,
    OutputAddress,
}

/// Derive the closed required-signer set for a transaction body.
///
/// `resolved_inputs` supplies the spent and collateral outputs so input
/// payment credentials can be classified. An input present in the body
/// but absent from `resolved_inputs` is an
/// [`RequiredSignerError::UnresolvableInput`] — the caller MUST supply
/// every input it intends to be coverage-checked. (The body-path wiring
/// only populates input/collateral sources when `track_utxo=true`; see
/// the slice doc track_utxo note.)
pub fn required_signers(
    tx_body: &ConwayTxBody,
    resolved_inputs: &ResolvedInputs,
    era: CardanoEra,
) -> Result<RequiredSigners, RequiredSignerError> {
    if era != CardanoEra::Conway {
        return Err(RequiredSignerError::UnsupportedEra { era });
    }

    let mut req = RequiredSigners::default();

    // Source 1: resolved spend-input payment key-hash credentials.
    for input in &tx_body.inputs {
        let out = resolved_inputs
            .resolved
            .get(input)
            .ok_or_else(|| RequiredSignerError::UnresolvableInput {
                input: input.clone(),
            })?;
        if let Some(kh) = payment_key_hash(&out.address)? {
            req.require(SignerSource::InputPaymentKey, kh);
        }
    }

    // Source 6: collateral-input payment key-hash credentials.
    if let Some(collateral) = &tx_body.collateral_inputs {
        for input in collateral {
            let out =
                resolved_inputs
                    .resolved
                    .get(input)
                    .ok_or_else(|| RequiredSignerError::UnresolvableInput {
                        input: input.clone(),
                    })?;
            if let Some(kh) = payment_key_hash(&out.address)? {
                req.require(SignerSource::CollateralPaymentKey, kh);
            }
        }
    }

    // Source 2: explicit required_signers (k14).
    if let Some(signers) = &tx_body.required_signers {
        for kh in signers {
            req.require(SignerSource::ExplicitRequiredSigner, kh.clone());
        }
    }

    // Source 3: withdrawal reward-account key-hash credentials (k5).
    if let Some(withdrawals_cbor) = &tx_body.withdrawals {
        collect_withdrawal_keys(withdrawals_cbor, &mut req)?;
    }

    // Source 4: certificate key hashes (k4), per cert kind.
    if let Some(certs_cbor) = &tx_body.certs {
        collect_certificate_keys(certs_cbor, &mut req)?;
    }

    // Source 5: governance voter key-hash credentials (k19).
    if let Some(voters_cbor) = &tx_body.voting_procedures {
        collect_voter_keys(voters_cbor, &mut req)?;
    }

    Ok(req)
}

/// Derive ONLY the tx-derived required signers — explicit (k14),
/// withdrawal (k5), certificate (k4), governance voter (k19). These need
/// no resolved UTxO, so this is the surface the body path checks
/// unconditionally (regardless of `track_utxo`). Input and collateral
/// payment-key coverage is added by [`required_signers`] when the UTxO is
/// available (the slice doc track_utxo note).
///
/// This is NOT a weakening of the closed enumeration: it is the strict
/// subset of [`SignerSource`] whose derivation is UTxO-free. Sources
/// `InputPaymentKey` / `CollateralPaymentKey` are intentionally absent
/// here and supplied by the full function.
pub fn tx_derived_required_signers(
    tx_body: &ConwayTxBody,
    era: CardanoEra,
) -> Result<RequiredSigners, RequiredSignerError> {
    if era != CardanoEra::Conway {
        return Err(RequiredSignerError::UnsupportedEra { era });
    }
    let mut req = RequiredSigners::default();

    if let Some(signers) = &tx_body.required_signers {
        for kh in signers {
            req.require(SignerSource::ExplicitRequiredSigner, kh.clone());
        }
    }
    if let Some(withdrawals_cbor) = &tx_body.withdrawals {
        collect_withdrawal_keys(withdrawals_cbor, &mut req)?;
    }
    if let Some(certs_cbor) = &tx_body.certs {
        collect_certificate_keys(certs_cbor, &mut req)?;
    }
    if let Some(voters_cbor) = &tx_body.voting_procedures {
        collect_voter_keys(voters_cbor, &mut req)?;
    }
    Ok(req)
}

// ---------------------------------------------------------------------------
// Address payment-credential classification
// ---------------------------------------------------------------------------

/// Classify an output address's payment credential, returning
/// `Some(key_hash)` only when it is a vkey (key-hash) credential.
///
/// Address header byte (Conway CDDL "address format"):
///   bits 7-4 = address type, bits 3-0 = network id.
///   For Shelley-family addresses (types 0x0..=0x7), bit 4 of the header
///   selects payment cred kind: 0 = key-hash, 1 = script-hash. The
///   payment credential is the 28 bytes immediately after the header.
///   - 0x0..=0x3 base, 0x4..=0x5 pointer, 0x6..=0x7 enterprise.
///   - 0x8 Byron bootstrap (bootstrap-witness path, not a vkey signer here).
///   - 0xE..=0xF reward address (no payment part; never an input address).
///
/// A script-hash payment credential returns `None` (phase-2 / native
/// script obligation, NOT a vkey signer — over-requiring would be a
/// false reject). A malformed Shelley address (too short to hold a
/// 28-byte payment credential) is a fail-closed
/// [`RequiredSignerError::MalformedField`].
fn payment_key_hash(address: &[u8]) -> Result<Option<Hash28>, RequiredSignerError> {
    if address.is_empty() {
        return Err(RequiredSignerError::MalformedField {
            field: RequiredSignerField::OutputAddress,
            offset: 0,
        });
    }
    let header = address[0];
    let addr_type = header >> 4;
    match addr_type {
        // Shelley base / pointer / enterprise: payment cred is bytes [1..29].
        0x0..=0x7 => {
            let payment_is_script = (header & 0x10) != 0;
            if payment_is_script {
                // Script-hash payment credential — not a vkey signer.
                return Ok(None);
            }
            if address.len() < 1 + 28 {
                return Err(RequiredSignerError::MalformedField {
                    field: RequiredSignerField::OutputAddress,
                    offset: 1,
                });
            }
            let mut h = [0u8; 28];
            h.copy_from_slice(&address[1..29]);
            Ok(Some(Hash28(h)))
        }
        // Byron bootstrap: bootstrap-witness path; no Ed25519 vkey signer here.
        0x8 => Ok(None),
        // Reward / future / other: not a valid spend-input payment address.
        _ => Ok(None),
    }
}

/// Classify a reward account's stake credential. The reward-account
/// header (CDDL "reward addresses") is `bits 7-5 = 111`, bit 4 selects
/// key-hash (0) vs script-hash (1), then 28 bytes of credential.
fn reward_account_key_hash(
    reward_account: &[u8],
) -> Result<Option<Hash28>, RequiredSignerError> {
    if reward_account.is_empty() {
        return Err(RequiredSignerError::MalformedField {
            field: RequiredSignerField::Withdrawals,
            offset: 0,
        });
    }
    let header = reward_account[0];
    let is_script = (header & 0x10) != 0;
    if is_script {
        return Ok(None);
    }
    if reward_account.len() < 1 + 28 {
        return Err(RequiredSignerError::MalformedField {
            field: RequiredSignerField::Withdrawals,
            offset: 1,
        });
    }
    let mut h = [0u8; 28];
    h.copy_from_slice(&reward_account[1..29]);
    Ok(Some(Hash28(h)))
}

// ---------------------------------------------------------------------------
// Withdrawals (k5): {+ reward_account => coin}
// ---------------------------------------------------------------------------

fn collect_withdrawal_keys(
    data: &[u8],
    req: &mut RequiredSigners,
) -> Result<(), RequiredSignerError> {
    let mut offset = 0;
    skip_optional_tag(data, &mut offset, RequiredSignerField::Withdrawals)?;
    let enc = read_map(data, &mut offset, RequiredSignerField::Withdrawals)?;
    let mut process = |data: &[u8], offset: &mut usize| -> Result<(), RequiredSignerError> {
        let (account, _) = read_bytes(data, offset, RequiredSignerField::Withdrawals)?;
        let _ = skip(data, offset, RequiredSignerField::Withdrawals)?; // coin
        if let Some(kh) = reward_account_key_hash(&account)? {
            req.require(SignerSource::WithdrawalKey, kh);
        }
        Ok(())
    };
    iterate(data, &mut offset, enc, RequiredSignerField::Withdrawals, &mut process)
}

// ---------------------------------------------------------------------------
// Certificates (k4): [+ certificate], certificate = [tag, ...]
// ---------------------------------------------------------------------------

fn collect_certificate_keys(
    data: &[u8],
    req: &mut RequiredSigners,
) -> Result<(), RequiredSignerError> {
    let mut offset = 0;
    skip_optional_tag(data, &mut offset, RequiredSignerField::Certificates)?;
    let enc = read_array(data, &mut offset, RequiredSignerField::Certificates)?;
    let mut process = |data: &[u8], offset: &mut usize| -> Result<(), RequiredSignerError> {
        collect_one_certificate(data, offset, req)
    };
    iterate(data, &mut offset, enc, RequiredSignerField::Certificates, &mut process)
}

fn collect_one_certificate(
    data: &[u8],
    offset: &mut usize,
    req: &mut RequiredSigners,
) -> Result<(), RequiredSignerError> {
    // Each certificate is an array whose first element is an integer tag.
    // We read the tag + the grammar-relevant element(s), then drain the
    // rest of THIS array's elements so the cursor lands on the next cert.
    let enc = read_array(data, offset, RequiredSignerField::Certificates)?;
    let (tag, _) = read_uint(data, offset, RequiredSignerField::Certificates)?;
    // Elements consumed so far past the array open: the tag (1).
    let mut consumed: u64 = 1;

    match tag {
        // tag 0: account_registration (RegCert SNothing) — NO witness.
        // tag 5/6: removed in Conway (genesis deleg / MIR); structurally
        //   rejected upstream. No key hash here.
        0 | 5 | 6 => {}
        // tag 1/2/7/8/9/10/11/12/13: stake credential is element 1.
        1 | 2 | 7 | 8 | 9 | 10 | 11 | 12 | 13 => {
            if let Some(kh) = read_credential_key_hash(data, offset)? {
                req.require(SignerSource::CertificateKey, kh);
            }
            consumed += 1;
        }
        // tag 3: pool_registration — operator key + all pool_owners.
        3 => {
            collect_pool_registration_keys(data, offset, req)?;
            // pool_params is a flattened single logical element group; we
            // consumed operator..pool_owners explicitly. Drain the rest of
            // the cert array (relays, pool_metadata) by element count below
            // using the indefinite-or-definite drain.
            consumed += 7; // operator,vrf,pledge,cost,margin,reward_account,owners
        }
        // tag 4: pool_retirement — element 1 is the pool key hash (raw).
        4 => {
            let kh = read_raw_hash28(data, offset, RequiredSignerField::Certificates)?;
            req.require(SignerSource::CertificateKey, kh);
            consumed += 1;
        }
        // tag 14/15/16/17/18: credential is element 1 (committee cold / DRep).
        14..=18 => {
            if let Some(kh) = read_credential_key_hash(data, offset)? {
                req.require(SignerSource::CertificateKey, kh);
            }
            consumed += 1;
        }
        _ => {
            // Unknown discriminant — fail-closed rather than silently skip,
            // because an unrecognized cert may carry an unmet obligation.
            return Err(RequiredSignerError::MalformedField {
                field: RequiredSignerField::Certificates,
                offset: *offset,
            });
        }
    }

    drain_array_tail(data, offset, enc, consumed, RequiredSignerField::Certificates)
}

/// pool_registration_cert = (3, pool_params); pool_params begins
/// `(operator: pool_keyhash, vrf_keyhash, pledge, cost, margin,
///   reward_account, pool_owners: set<addr_keyhash>, relays, pool_metadata)`.
/// The cert array is flattened. Consumes operator..pool_owners (7 elements);
/// relays + pool_metadata are drained by the caller.
fn collect_pool_registration_keys(
    data: &[u8],
    offset: &mut usize,
    req: &mut RequiredSigners,
) -> Result<(), RequiredSignerError> {
    // operator (pool_keyhash, raw 28-byte bstr)
    let operator = read_raw_hash28(data, offset, RequiredSignerField::Certificates)?;
    req.require(SignerSource::CertificateKey, operator);
    skip(data, offset, RequiredSignerField::Certificates)?; // vrf_keyhash
    skip(data, offset, RequiredSignerField::Certificates)?; // pledge
    skip(data, offset, RequiredSignerField::Certificates)?; // cost
    skip(data, offset, RequiredSignerField::Certificates)?; // margin
    skip(data, offset, RequiredSignerField::Certificates)?; // reward_account
    // pool_owners : set<addr_keyhash>
    skip_optional_tag(data, offset, RequiredSignerField::Certificates)?;
    let enc = read_array(data, offset, RequiredSignerField::Certificates)?;
    let mut process = |data: &[u8], offset: &mut usize| -> Result<(), RequiredSignerError> {
        let kh = read_raw_hash28(data, offset, RequiredSignerField::Certificates)?;
        req.require(SignerSource::CertificateKey, kh);
        Ok(())
    };
    iterate(data, offset, enc, RequiredSignerField::Certificates, &mut process)
}

// ---------------------------------------------------------------------------
// Voting procedures (k19): {+ voter => {+ gov_action_id => voting_procedure}}
// ---------------------------------------------------------------------------

fn collect_voter_keys(
    data: &[u8],
    req: &mut RequiredSigners,
) -> Result<(), RequiredSignerError> {
    let mut offset = 0;
    skip_optional_tag(data, &mut offset, RequiredSignerField::VotingProcedures)?;
    let enc = read_map(data, &mut offset, RequiredSignerField::VotingProcedures)?;
    let mut process = |data: &[u8], offset: &mut usize| -> Result<(), RequiredSignerError> {
        // voter = [tag, hash28]; tags 0/2/4 = addr_keyhash, 1/3 = script_hash.
        let v_enc = read_array(data, offset, RequiredSignerField::VotingProcedures)?;
        let (tag, _) = read_uint(data, offset, RequiredSignerField::VotingProcedures)?;
        match tag {
            0 | 2 | 4 => {
                let kh = read_raw_hash28(data, offset, RequiredSignerField::VotingProcedures)?;
                req.require(SignerSource::GovernanceVoter, kh);
            }
            1 | 3 => {
                // script-hash voter — not a vkey signer.
                skip(data, offset, RequiredSignerField::VotingProcedures)?;
            }
            _ => {
                return Err(RequiredSignerError::MalformedField {
                    field: RequiredSignerField::VotingProcedures,
                    offset: *offset,
                });
            }
        }
        // tag (1) + the hash element (1) consumed.
        drain_array_tail(data, offset, v_enc, 2, RequiredSignerField::VotingProcedures)?;
        // Skip the value map {+ gov_action_id => voting_procedure}.
        skip(data, offset, RequiredSignerField::VotingProcedures)?;
        Ok(())
    };
    iterate(
        data,
        &mut offset,
        enc,
        RequiredSignerField::VotingProcedures,
        &mut process,
    )
}

// ---------------------------------------------------------------------------
// Credential helpers
// ---------------------------------------------------------------------------

/// credential = [0, addr_keyhash // 1, script_hash]. Returns the key hash
/// only when the leading tag is 0 (key-hash); tag 1 (script-hash) returns
/// `None`. Also handles `drep = [0, addr_keyhash // 1, script_hash // 2 // 3]`
/// (tags 2/3 are AlwaysAbstain/NoConfidence — no credential, never reached
/// as a cert credential element). committee_cold/drep credentials reuse
/// the same `[tag, hash]` shape.
fn read_credential_key_hash(
    data: &[u8],
    offset: &mut usize,
) -> Result<Option<Hash28>, RequiredSignerError> {
    let enc = read_array(data, offset, RequiredSignerField::Certificates)?;
    let (tag, _) = read_uint(data, offset, RequiredSignerField::Certificates)?;
    let result = match tag {
        0 => Some(read_raw_hash28(data, offset, RequiredSignerField::Certificates)?),
        1 => {
            skip(data, offset, RequiredSignerField::Certificates)?; // script_hash
            None
        }
        _ => {
            return Err(RequiredSignerError::MalformedField {
                field: RequiredSignerField::Certificates,
                offset: *offset,
            });
        }
    };
    // tag (1) + the hash/script element (1) consumed.
    drain_array_tail(data, offset, enc, 2, RequiredSignerField::Certificates)?;
    Ok(result)
}

/// Read a 28-byte hash stored as a CBOR byte string (validates length).
fn read_raw_hash28(
    data: &[u8],
    offset: &mut usize,
    field: RequiredSignerField,
) -> Result<Hash28, RequiredSignerError> {
    let (bytes, _) = read_bytes(data, offset, field)?;
    if bytes.len() != 28 {
        return Err(RequiredSignerError::MalformedField {
            field,
            offset: *offset,
        });
    }
    let mut h = [0u8; 28];
    h.copy_from_slice(&bytes);
    Ok(Hash28(h))
}

// ---------------------------------------------------------------------------
// CBOR adapters — map ade_codec CodecError into the closed taxonomy.
// ---------------------------------------------------------------------------

fn skip_optional_tag(
    data: &[u8],
    offset: &mut usize,
    field: RequiredSignerField,
) -> Result<(), RequiredSignerError> {
    if *offset < data.len() {
        let major = cbor::peek_major(data, *offset).map_err(|_| RequiredSignerError::MalformedField {
            field,
            offset: *offset,
        })?;
        if major == cbor::MAJOR_TAG {
            cbor::read_tag(data, offset).map_err(|_| RequiredSignerError::MalformedField {
                field,
                offset: *offset,
            })?;
        }
    }
    Ok(())
}

fn read_array(
    data: &[u8],
    offset: &mut usize,
    field: RequiredSignerField,
) -> Result<ContainerEncoding, RequiredSignerError> {
    cbor::read_array_header(data, offset).map_err(|_| RequiredSignerError::MalformedField {
        field,
        offset: *offset,
    })
}

fn read_map(
    data: &[u8],
    offset: &mut usize,
    field: RequiredSignerField,
) -> Result<ContainerEncoding, RequiredSignerError> {
    cbor::read_map_header(data, offset).map_err(|_| RequiredSignerError::MalformedField {
        field,
        offset: *offset,
    })
}

fn read_uint(
    data: &[u8],
    offset: &mut usize,
    field: RequiredSignerField,
) -> Result<(u64, cbor::IntWidth), RequiredSignerError> {
    cbor::read_uint(data, offset).map_err(|_| RequiredSignerError::MalformedField {
        field,
        offset: *offset,
    })
}

fn read_bytes(
    data: &[u8],
    offset: &mut usize,
    field: RequiredSignerField,
) -> Result<(Vec<u8>, cbor::IntWidth), RequiredSignerError> {
    cbor::read_bytes(data, offset).map_err(|_| RequiredSignerError::MalformedField {
        field,
        offset: *offset,
    })
}

fn skip(
    data: &[u8],
    offset: &mut usize,
    field: RequiredSignerField,
) -> Result<(usize, usize), RequiredSignerError> {
    cbor::skip_item(data, offset).map_err(|_| RequiredSignerError::MalformedField {
        field,
        offset: *offset,
    })
}

/// Iterate a CBOR container (array or map) of definite or indefinite
/// length, applying `process` once per top-level element (a map entry's
/// key+value must both be consumed inside `process`).
fn iterate<F>(
    data: &[u8],
    offset: &mut usize,
    enc: ContainerEncoding,
    field: RequiredSignerField,
    process: &mut F,
) -> Result<(), RequiredSignerError>
where
    F: FnMut(&[u8], &mut usize) -> Result<(), RequiredSignerError>,
{
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                process(data, offset)?;
            }
        }
        ContainerEncoding::Indefinite => {
            loop {
                let is_break =
                    cbor::is_break(data, *offset).map_err(|_| RequiredSignerError::MalformedField {
                        field,
                        offset: *offset,
                    })?;
                if is_break {
                    *offset += 1;
                    break;
                }
                process(data, offset)?;
            }
        }
    }
    Ok(())
}

/// Drain the trailing elements of an array whose header was `enc` and of
/// which `consumed` top-level elements have already been read. For a
/// definite array of arity `n`, skips exactly `n - consumed` items
/// (fail-closed if the caller over-consumed). For an indefinite array,
/// skips through the break byte.
fn drain_array_tail(
    data: &[u8],
    offset: &mut usize,
    enc: ContainerEncoding,
    consumed: u64,
    field: RequiredSignerField,
) -> Result<(), RequiredSignerError> {
    match enc {
        ContainerEncoding::Definite(n, _) => {
            if consumed > n {
                return Err(RequiredSignerError::MalformedField {
                    field,
                    offset: *offset,
                });
            }
            for _ in 0..(n - consumed) {
                skip(data, offset, field)?;
            }
            Ok(())
        }
        ContainerEncoding::Indefinite => loop {
            let is_break =
                cbor::is_break(data, *offset).map_err(|_| RequiredSignerError::MalformedField {
                    field,
                    offset: *offset,
                })?;
            if is_break {
                *offset += 1;
                return Ok(());
            }
            skip(data, offset, field)?;
        },
    }
}
