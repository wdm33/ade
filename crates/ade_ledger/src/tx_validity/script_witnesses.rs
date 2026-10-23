// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! The witness-script closure of a Conway transaction (DC-LEDGER-SCRIPT-03).
//!
//! cardano-ledger `babbageUtxowTransition` (`Babbage/Rules/Utxow.hs`), which Conway reuses, runs two
//! script tests:
//!
//! - `validateFailedBabbageScripts`: every provided native script (witness set or reference script)
//!   whose hash is needed and which does not validate → `ScriptWitnessNotValidatingUTXOW`. Scripts
//!   that are provided but not needed are not evaluated.
//! - `babbageMissingScripts`: with `neededNonRefs = needed − reference-script hashes`, every witness
//!   script outside `neededNonRefs` is extraneous and every `neededNonRefs` hash absent from the
//!   witness set is missing.
//!
//! The needed set needs the resolved spend inputs and the reference scripts need the resolved spend
//! and reference inputs, so there are two evidence levels:
//!
//! - **Reduced** (any such input unresolved): a failing witness-set native script is a definite
//!   rejection. If its hash is needed it does not validate, otherwise it is extraneous, so Cardano
//!   rejects the transaction either way. Nothing else can be decided, so a reduced verdict is never
//!   "valid".
//! - **Exact** (every spend and reference input resolved): Cardano's three failure sets.
//!
//! An [`ExactScriptVerdict`] is reachable only through a [`ResolvedScriptContext`], which can be
//! built only when every spend and reference input resolves.

use std::collections::{BTreeMap, BTreeSet};

use ade_codec::cbor::{self, ContainerEncoding};
use ade_types::allegra::script::NativeScript;
use ade_types::conway::governance::GovAction;
use ade_types::conway::tx::ConwayTxBody;
use ade_types::tx::TxIn;
use ade_types::{Hash28, SlotNo};

use crate::error::{LedgerError, NativeScriptFailure, ScriptError};
use crate::scripts::{evaluate_native_script, ScriptVerdict, ValidityInterval};
use crate::witness::{
    prefixed_script_hash, WitnessInfo, NATIVE_SCRIPT_PREFIX, PLUTUS_V1_PREFIX, PLUTUS_V2_PREFIX,
    PLUTUS_V3_PREFIX,
};

use super::required_signers::{
    walk_certificate_credentials, walk_voter_credentials, walk_withdrawal_credentials, Credential,
    RequiredSignerError,
};

/// Why a script hash is needed (cardano-ledger `getConwayScriptsNeeded`, one variant per purpose).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScriptPurpose {
    /// A spend input whose address has a script payment credential.
    Spending,
    /// A withdrawal from a script reward account.
    Rewarding,
    /// A certificate whose witness credential is a script.
    Certifying,
    /// A minting policy.
    Minting,
    /// A committee-hot or DRep voter with a script credential.
    Voting,
    /// A parameter-change or treasury-withdrawal proposal's guardrail script.
    Proposing,
}

/// The needed script hashes, each with the purposes that demanded it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptsNeeded {
    pub hashes: BTreeSet<Hash28>,
    pub provenance: BTreeSet<(ScriptPurpose, Hash28)>,
}

impl ScriptsNeeded {
    fn need(&mut self, purpose: ScriptPurpose, hash: Hash28) {
        self.hashes.insert(hash.clone());
        self.provenance.insert((purpose, hash));
    }
}

/// One resolved spend or reference input: what the closure reads from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScriptOutput {
    /// The output address's wire bytes.
    pub address: Vec<u8>,
    /// The output's `script_ref` item as on the wire (`#6.24(bytes .cbor script)`), if any.
    pub script_ref: Option<Vec<u8>>,
}

impl ResolvedScriptOutput {
    /// From a UTxO entry. Pre-Alonzo outputs carry no `script_ref`.
    pub fn from_txout(out: &crate::utxo::TxOut) -> Result<Self, ScriptWitnessError> {
        match out {
            crate::utxo::TxOut::AlonzoPlus { raw, .. } => Self::from_raw_output(raw),
            other => Ok(ResolvedScriptOutput { address: other.address_bytes().to_vec(), script_ref: None }),
        }
    }

    /// From a stored output's preserved CBOR (`TxOut::AlonzoPlus.raw`, array or map form).
    pub fn from_raw_output(raw: &[u8]) -> Result<Self, ScriptWitnessError> {
        let mut offset = 0;
        let out = ade_codec::babbage::tx::decode_babbage_tx_out(raw, &mut offset)
            .map_err(|_| ScriptWitnessError::MalformedResolvedOutput)?;
        Ok(ResolvedScriptOutput { address: out.address, script_ref: out.script_ref })
    }
}

/// The resolved outputs of every spend input and every reference input of one transaction.
///
/// The only way to an exact verdict. [`ResolvedScriptContext::resolve`] returns `None` unless every
/// such input resolves, so a partial UTxO view cannot produce one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScriptContext {
    outputs: BTreeMap<TxIn, ResolvedScriptOutput>,
}

impl ResolvedScriptContext {
    /// Resolve against a UTxO view. `Ok(None)` when any spend or reference input is absent.
    pub fn resolve_from_utxo(
        body: &ConwayTxBody,
        mut lookup: impl FnMut(&TxIn) -> Option<crate::utxo::TxOut>,
    ) -> Result<Option<Self>, ScriptWitnessError> {
        let mut bad = None;
        let ctx = Self::resolve(body, |t| match lookup(t).map(|o| ResolvedScriptOutput::from_txout(&o)) {
            Some(Ok(o)) => Some(o),
            Some(Err(e)) => {
                bad = Some(e);
                None
            }
            None => None,
        });
        match bad {
            Some(e) => Err(e),
            None => Ok(ctx),
        }
    }

    /// Resolve `body.inputs ∪ body.reference_inputs` through `lookup`. `None` when any is absent.
    pub fn resolve(
        body: &ConwayTxBody,
        mut lookup: impl FnMut(&TxIn) -> Option<ResolvedScriptOutput>,
    ) -> Option<Self> {
        let mut outputs = BTreeMap::new();
        for input in body.inputs.iter().chain(body.reference_inputs.iter().flatten()) {
            if !outputs.contains_key(input) {
                outputs.insert(input.clone(), lookup(input)?);
            }
        }
        Some(ResolvedScriptContext { outputs })
    }
}

/// The three failure sets of Cardano's script closure. All empty means the closure holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptClosureFailures {
    /// Needed native scripts that do not validate (`ScriptWitnessNotValidatingUTXOW`), with the
    /// first failure Ade's evaluator reports for each.
    pub not_validating: BTreeMap<Hash28, NativeScriptFailure>,
    /// Witness scripts that are not needed, or are needed but supplied by reference
    /// (`ExtraneousScriptWitnessesUTXOW`).
    pub extraneous: BTreeSet<Hash28>,
    /// Needed scripts supplied neither by witness nor by reference (`MissingScriptWitnessesUTXOW`).
    pub missing: BTreeSet<Hash28>,
}

impl ScriptClosureFailures {
    pub fn is_empty(&self) -> bool {
        self.not_validating.is_empty() && self.extraneous.is_empty() && self.missing.is_empty()
    }
}

/// What can be decided without the resolved spend and reference inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReducedScriptVerdict {
    /// A witness-set native script fails, so Cardano rejects the transaction: as not-validating if
    /// the script is needed, as extraneous if not. Which of the two is not known here.
    DefiniteReject(ScriptError),
    /// No witness-set native script fails. Missing, extraneous and reference scripts are undecided.
    /// Not a claim that the closure holds.
    Undetermined,
}

/// Cardano's exact script-closure result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactScriptVerdict {
    Valid,
    Invalid(ScriptClosureFailures),
}

/// The witness-script verdict at whichever evidence level the caller's context allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeScriptVerdict {
    Reduced(ReducedScriptVerdict),
    Exact(ExactScriptVerdict),
}

/// The closure itself could not be computed: a field it reads is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptWitnessError {
    /// Certificates, withdrawals or voting procedures failed to parse.
    MalformedField(RequiredSignerError),
    /// The `mint` field failed to parse.
    MalformedMint { offset: usize },
    /// A resolved output failed to parse.
    MalformedResolvedOutput,
    /// A resolved output's `script_ref` failed to parse.
    MalformedScriptRef,
}

/// The transaction's validity interval: `validity_interval_start` (key 8) and `ttl` (key 3).
pub fn validity_interval(body: &ConwayTxBody) -> ValidityInterval {
    ValidityInterval {
        invalid_before: body.validity_interval_start.map(|SlotNo(s)| s),
        invalid_hereafter: body.ttl.map(|SlotNo(s)| s),
    }
}

/// The signer key hashes native scripts are evaluated against (cardano-ledger
/// `keyHashWitnessesTxWits`): vkey witnesses ∪ bootstrap witnesses.
pub fn script_signers(witnesses: &WitnessInfo) -> BTreeSet<Hash28> {
    witnesses
        .available_key_hashes
        .union(&witnesses.bootstrap_key_hashes)
        .cloned()
        .collect()
}

/// The one enforcement seam, shared by the block path and `tx_phase_one`: decide the closure and
/// turn a rejection into its typed error. `Undetermined` is not an error, and not a claim of
/// validity: the reduced level proves only failures.
pub fn check_script_witnesses(
    body: &ConwayTxBody,
    witnesses: &WitnessInfo,
    context: Option<&ResolvedScriptContext>,
) -> Result<NativeScriptVerdict, LedgerError> {
    let verdict =
        native_script_verdict(body, witnesses, context).map_err(LedgerError::ScriptWitnessDerivation)?;
    match &verdict {
        NativeScriptVerdict::Reduced(ReducedScriptVerdict::DefiniteReject(e)) => {
            Err(LedgerError::NativeScriptFailed(e.clone()))
        }
        NativeScriptVerdict::Exact(ExactScriptVerdict::Invalid(f)) => {
            Err(LedgerError::ScriptWitnessClosure(f.clone()))
        }
        NativeScriptVerdict::Reduced(ReducedScriptVerdict::Undetermined)
        | NativeScriptVerdict::Exact(ExactScriptVerdict::Valid) => Ok(verdict),
    }
}

/// Decide the witness-script closure. With `context`, the exact verdict; without, the reduced one.
pub fn native_script_verdict(
    body: &ConwayTxBody,
    witnesses: &WitnessInfo,
    context: Option<&ResolvedScriptContext>,
) -> Result<NativeScriptVerdict, ScriptWitnessError> {
    Ok(match context {
        None => NativeScriptVerdict::Reduced(reduced_verdict(body, witnesses)),
        Some(ctx) => NativeScriptVerdict::Exact(exact_verdict(body, witnesses, ctx)?),
    })
}

/// Witness-set native scripts by hash. A set in Cardano, so a repeated script counts once.
fn witness_native_scripts(witnesses: &WitnessInfo) -> BTreeMap<Hash28, &NativeScript> {
    witnesses.native_scripts.iter().map(|w| (w.hash.clone(), &w.script)).collect()
}

fn failure(script: &NativeScript, signers: &BTreeSet<Hash28>, iv: &ValidityInterval) -> Option<NativeScriptFailure> {
    match evaluate_native_script(script, signers, iv) {
        ScriptVerdict::NativeScriptFailed(reason) => Some(reason),
        _ => None,
    }
}

/// The reduced verdict: the first failing witness-set native script in hash order, if any.
pub fn reduced_verdict(body: &ConwayTxBody, witnesses: &WitnessInfo) -> ReducedScriptVerdict {
    let signers = script_signers(witnesses);
    let iv = validity_interval(body);
    for (hash, script) in witness_native_scripts(witnesses) {
        if let Some(reason) = failure(script, &signers, &iv) {
            return ReducedScriptVerdict::DefiniteReject(ScriptError { script_hash: hash, reason });
        }
    }
    ReducedScriptVerdict::Undetermined
}

fn exact_verdict(
    body: &ConwayTxBody,
    witnesses: &WitnessInfo,
    ctx: &ResolvedScriptContext,
) -> Result<ExactScriptVerdict, ScriptWitnessError> {
    let needed = scripts_needed(body, ctx)?;
    let refs = reference_scripts(body, ctx)?;
    let wit_native = witness_native_scripts(witnesses);
    let signers = script_signers(witnesses);
    let iv = validity_interval(body);

    let mut failures = ScriptClosureFailures::default();

    // validateFailedBabbageScripts: provided = reference scripts ∪ witness scripts, restricted to
    // needed native scripts.
    for hash in &needed.hashes {
        let script = match refs.get(hash) {
            Some(ReferenceScript::Native(s)) => Some(s),
            Some(ReferenceScript::Plutus) => None,
            None => wit_native.get(hash).copied(),
        };
        if let Some(reason) = script.and_then(|s| failure(s, &signers, &iv)) {
            failures.not_validating.insert(hash.clone(), reason);
        }
    }

    // babbageMissingScripts.
    let received: BTreeSet<Hash28> =
        wit_native.keys().cloned().chain(witnesses.plutus_script_hashes.iter().cloned()).collect();
    let needed_non_refs: BTreeSet<Hash28> =
        needed.hashes.iter().filter(|h| !refs.contains_key(*h)).cloned().collect();
    failures.extraneous = received.difference(&needed_non_refs).cloned().collect();
    failures.missing = needed_non_refs.difference(&received).cloned().collect();

    Ok(if failures.is_empty() {
        ExactScriptVerdict::Valid
    } else {
        ExactScriptVerdict::Invalid(failures)
    })
}

/// cardano-ledger `getConwayScriptsNeeded`, all six purposes.
pub fn scripts_needed(
    body: &ConwayTxBody,
    ctx: &ResolvedScriptContext,
) -> Result<ScriptsNeeded, ScriptWitnessError> {
    let mut needed = ScriptsNeeded::default();

    // Spending: getSpendingScriptsNeeded — a resolved input whose address has a script payment
    // credential. The context holds every input, by construction.
    for input in &body.inputs {
        if let Some(out) = ctx.outputs.get(input) {
            if let Some(h) = payment_script_hash(&out.address) {
                needed.need(ScriptPurpose::Spending, h);
            }
        }
    }

    tx_derived_scripts_needed(body, &mut needed)?;
    Ok(needed)
}

/// The five UTxO-free purposes of `getConwayScriptsNeeded`: rewarding, certifying, minting, voting,
/// proposing.
fn tx_derived_scripts_needed(
    body: &ConwayTxBody,
    needed: &mut ScriptsNeeded,
) -> Result<(), ScriptWitnessError> {
    if let Some(w) = &body.withdrawals {
        walk_withdrawal_credentials(w, &mut |c| {
            if let Credential::Script(h) = c {
                needed.need(ScriptPurpose::Rewarding, h);
            }
        })
        .map_err(ScriptWitnessError::MalformedField)?;
    }
    if let Some(c) = &body.certs {
        walk_certificate_credentials(c, &mut |c| {
            if let Credential::Script(h) = c {
                needed.need(ScriptPurpose::Certifying, h);
            }
        })
        .map_err(ScriptWitnessError::MalformedField)?;
    }
    if let Some(m) = &body.mint {
        for policy in mint_policies(m)? {
            needed.need(ScriptPurpose::Minting, policy);
        }
    }
    if let Some(v) = &body.voting_procedures {
        walk_voter_credentials(v, &mut |c| {
            if let Credential::Script(h) = c {
                needed.need(ScriptPurpose::Voting, h);
            }
        })
        .map_err(ScriptWitnessError::MalformedField)?;
    }
    for proposal in body.proposal_procedures.iter().flatten() {
        let guardrail = match &proposal.gov_action {
            GovAction::ParameterChange { policy_hash, .. } => policy_hash,
            GovAction::TreasuryWithdrawals { policy_hash, .. } => policy_hash,
            _ => &None,
        };
        if let Some(h) = guardrail {
            needed.need(ScriptPurpose::Proposing, h.clone());
        }
    }
    Ok(())
}

/// The payment script hash of a Shelley-family address (types 0..=7, header bit 4 set). Byron and
/// other addresses have none (`getScriptHash`).
fn payment_script_hash(address: &[u8]) -> Option<Hash28> {
    let header = *address.first()?;
    if header >> 4 > 0x7 || header & 0x10 == 0 || address.len() < 29 {
        return None;
    }
    let mut h = [0u8; 28];
    h.copy_from_slice(&address[1..29]);
    Some(Hash28(h))
}

/// The policy ids of `mint = { + policy_id => { + asset_name => int } }` (maybe tag-wrapped).
fn mint_policies(data: &[u8]) -> Result<Vec<Hash28>, ScriptWitnessError> {
    let bad = |offset: usize| ScriptWitnessError::MalformedMint { offset };
    let mut offset = 0;
    if cbor::peek_major(data, offset).map_err(|_| bad(offset))? == cbor::MAJOR_TAG {
        cbor::read_tag(data, &mut offset).map_err(|_| bad(offset))?;
    }
    let enc = cbor::read_map_header(data, &mut offset).map_err(|_| bad(offset))?;
    let mut out = Vec::new();
    let mut read_one = |offset: &mut usize| -> Result<(), ScriptWitnessError> {
        let (policy, _) = cbor::read_bytes(data, offset).map_err(|_| bad(*offset))?;
        if policy.len() != 28 {
            return Err(bad(*offset));
        }
        let mut h = [0u8; 28];
        h.copy_from_slice(&policy);
        out.push(Hash28(h));
        cbor::skip_item(data, offset).map_err(|_| bad(*offset))?;
        Ok(())
    };
    match enc {
        ContainerEncoding::Definite(n, _) => {
            for _ in 0..n {
                read_one(&mut offset)?;
            }
        }
        ContainerEncoding::Indefinite => {
            while !cbor::is_break(data, offset).map_err(|_| bad(offset))? {
                read_one(&mut offset)?;
            }
        }
    }
    Ok(out)
}

/// A reference script, as the closure needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceScript {
    Native(NativeScript),
    Plutus,
}

/// cardano-ledger `getReferenceScripts utxo (referenceInputs ∪ inputs)`, keyed by script hash.
pub fn reference_scripts(
    body: &ConwayTxBody,
    ctx: &ResolvedScriptContext,
) -> Result<BTreeMap<Hash28, ReferenceScript>, ScriptWitnessError> {
    let mut out = BTreeMap::new();
    for input in body.inputs.iter().chain(body.reference_inputs.iter().flatten()) {
        if let Some(raw) = ctx.outputs.get(input).and_then(|o| o.script_ref.as_ref()) {
            let (hash, script) = decode_script_ref(raw)?;
            out.insert(hash, script);
        }
    }
    Ok(out)
}

/// `script_ref = #6.24(bytes .cbor script)`, `script = [0, native_script] / [1|2|3, bytes]`. The
/// hash is over the native script's own bytes inside `script`, or over the Plutus bytes' content.
pub fn decode_script_ref(raw: &[u8]) -> Result<(Hash28, ReferenceScript), ScriptWitnessError> {
    let bad = |_| ScriptWitnessError::MalformedScriptRef;
    let mut o = 0;
    let (tag, _) = cbor::read_tag(raw, &mut o).map_err(bad)?;
    if tag != 24 {
        return Err(ScriptWitnessError::MalformedScriptRef);
    }
    let (inner, _) = cbor::read_bytes(raw, &mut o).map_err(bad)?;
    let mut i = 0;
    match cbor::read_array_header(&inner, &mut i).map_err(bad)? {
        ContainerEncoding::Definite(2, _) => {}
        _ => return Err(ScriptWitnessError::MalformedScriptRef),
    }
    let (lang, _) = cbor::read_uint(&inner, &mut i).map_err(bad)?;
    match lang {
        0 => {
            let start = i;
            let script = ade_codec::allegra::script::decode_native_script(&inner, &mut i).map_err(bad)?;
            Ok((prefixed_script_hash(NATIVE_SCRIPT_PREFIX, &inner[start..i]), ReferenceScript::Native(script)))
        }
        1..=3 => {
            let (bytes, _) = cbor::read_bytes(&inner, &mut i).map_err(bad)?;
            let prefix = [PLUTUS_V1_PREFIX, PLUTUS_V2_PREFIX, PLUTUS_V3_PREFIX][(lang - 1) as usize];
            Ok((prefixed_script_hash(prefix, &bytes), ReferenceScript::Plutus))
        }
        _ => Err(ScriptWitnessError::MalformedScriptRef),
    }
}
