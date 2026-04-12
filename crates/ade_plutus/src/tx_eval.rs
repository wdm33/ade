// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

//! Phase-2 transaction evaluation (Phase 3 Clusters P-C + P-D, slice S-31).
//!
//! Wraps aiken's `tx::eval_phase_two_raw` which internally:
//!   1. Decodes the transaction CBOR.
//!   2. Builds the per-script ScriptContext (V1/V2/V3 per witness-set
//!      key; handles envelope refactor, ref inputs, inline datums,
//!      governance ScriptInfo variants — see S-31 obligation discharge).
//!   3. Evaluates each Plutus script against its ScriptContext with
//!      the provided cost model and budget cap.
//!   4. Returns per-script budget consumption and result.
//!
//! Ade delegates ScriptContext construction to aiken's battle-tested
//! implementation (proven against IOG conformance and aiken's own
//! CI). This crate exposes an Ade-canonical result shape
//! (`TxEvalResult`) so downstream callers never see aiken or pallas
//! types.
//!
//! The slot config is passed explicitly (not defaulted) because Ade's
//! release tier is version-scoped to cardano-node 10.6.2 on mainnet,
//! but the function must also work on other chains (preview, preprod,
//! future mainnet). The caller is responsible for passing the correct
//! `(system_start_ms, zero_slot, slot_length_ms)` for the target
//! network.

use crate::evaluator::PlutusError;

/// Per-script evaluation result.
///
/// Mirrors the relevant subset of aiken's `EvalResult` without
/// leaking aiken types. One entry per executed script (each
/// redeemer in the tx).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerScriptResult {
    /// Raw CBOR of the Redeemer that drove this script execution.
    /// Callers who need the parsed redeemer can decode this via
    /// their preferred path.
    pub redeemer_cbor: Vec<u8>,
    /// Evaluation succeeded (the script returned `()` / `true`).
    pub success: bool,
    /// CPU budget consumed.
    pub cpu: i64,
    /// Memory budget consumed.
    pub mem: i64,
    /// Trace log lines emitted via `trace` / `debug`.
    pub logs: Vec<String>,
}

/// Full-tx phase-2 evaluation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxEvalResult {
    /// Per-script results, in the order aiken produced them
    /// (typically matches the order of redeemers in the witness set).
    pub scripts: Vec<PerScriptResult>,
}

/// Slot configuration: `(system_start_posix_ms, zero_slot, slot_length_ms)`.
///
/// - `system_start_posix_ms`: POSIX time in milliseconds when the
///   chain started. Mainnet: 1 506 203 091 000.
/// - `zero_slot`: the slot index treated as offset 0 for
///   the `zero_time` anchor. For mainnet Shelley, the Shelley-era
///   zero is slot 0 at Shelley hard-fork point.
/// - `slot_length_ms`: slot duration in milliseconds. Mainnet: 1000.
///
/// These values are passed as a tuple to match aiken's
/// `eval_phase_two_raw` signature directly without leaking its
/// `SlotConfig` type across the Ade boundary.
pub type SlotConfig = (u64, u64, u32);

/// Mainnet slot config (cardano-node 10.6.2).
pub const MAINNET_SLOT_CONFIG: SlotConfig = (1506203091000, 0, 1000);

/// Evaluate all Plutus scripts in a transaction against the
/// provided resolved UTxO set.
///
/// # Arguments
///
/// - `tx_cbor`: CBOR bytes of the transaction (full tx, not just
///   the body — includes witness set and metadata).
/// - `resolved_utxos`: pairs of `(input_cbor, output_cbor)` —
///   each input in `tx_cbor`'s `inputs`, `collateral_inputs`, and
///   `reference_inputs` sets must have its resolved output in this
///   list. Input CBOR is the `TransactionInput = [tx_id, ix]`
///   pair; output CBOR is the resolved `TransactionOutput`.
/// - `cost_models_cbor`: optional CBOR of the protocol-parameter
///   `cost_models` map. `None` uses aiken's built-in defaults
///   (fine for conformance testing; mainnet should pass actual
///   pparams).
/// - `initial_budget`: `(cpu, mem)` budget cap for EACH script.
///   Per O-30.3 discharge, the per-tx budget cap is enforced
///   separately at phase-1 via
///   `ade_ledger::late_era_validation::check_tx_ex_units_within_cap`.
/// - `slot_config`: chain-specific time anchors; use
///   `MAINNET_SLOT_CONFIG` for mainnet.
///
/// # Returns
///
/// `Ok(TxEvalResult)` on successful run (per-script results
/// included, individual scripts may have `success: false`).
/// `Err(PlutusError)` on decode or infrastructure failure
/// (malformed tx CBOR, unresolvable input, etc.).
pub fn eval_tx_phase_two(
    tx_cbor: &[u8],
    resolved_utxos: &[(Vec<u8>, Vec<u8>)],
    cost_models_cbor: Option<&[u8]>,
    initial_budget: (u64, u64),
    slot_config: SlotConfig,
) -> Result<TxEvalResult, PlutusError> {
    let raw_results = aiken_uplc::tx::eval_phase_two_raw(
        tx_cbor,
        resolved_utxos,
        cost_models_cbor,
        initial_budget,
        slot_config,
        false, // run_phase_one=false: Ade performs phase-1 via ade_ledger::late_era_validation
        |_| (), // no-op redeemer callback
    )
    .map_err(|e| PlutusError::DecodeFailed(format!("eval_phase_two_raw: {e}")))?;

    let mut scripts = Vec::with_capacity(raw_results.len());
    for (redeemer_cbor, _eval_result) in raw_results {
        // eval_phase_two_raw returns only the redeemer bytes with the
        // final applied ExUnits — aiken's raw wrapper strips the
        // per-script EvalResult down to just the redeemer bytes
        // (the redeemer carries the executed ex_units after rewriting).
        // Extract CPU/mem from the redeemer's ex_units field.
        let (cpu, mem) = extract_redeemer_ex_units(&redeemer_cbor)?;
        scripts.push(PerScriptResult {
            redeemer_cbor,
            success: true, // raw returns Err on script failure; Ok means success
            cpu,
            mem,
            logs: Vec::new(), // raw path doesn't expose logs
        });
    }

    Ok(TxEvalResult { scripts })
}

/// Extract `(cpu, mem)` from a CBOR-encoded Redeemer.
///
/// Redeemer format (Alonzo+ array form):
///   `[tag, index, data, [mem, cpu]]` — position 3 is the ex_units tuple.
///
/// Conway map form stores redeemers differently, but
/// `eval_phase_two_raw` re-serializes to the array form on output.
fn extract_redeemer_ex_units(redeemer_cbor: &[u8]) -> Result<(i64, i64), PlutusError> {
    use ade_codec::cbor::{read_array_header, read_uint, ContainerEncoding};
    let mut offset = 0;
    let hdr = read_array_header(redeemer_cbor, &mut offset)
        .map_err(|e| PlutusError::DecodeFailed(format!("redeemer array header: {e}")))?;
    match hdr {
        ContainerEncoding::Definite(4, _) => {}
        _ => {
            return Err(PlutusError::DecodeFailed(
                "redeemer: expected 4-field array".into(),
            ))
        }
    }
    // Skip tag, index, data.
    for _ in 0..3 {
        ade_codec::cbor::skip_item(redeemer_cbor, &mut offset)
            .map_err(|e| PlutusError::DecodeFailed(format!("redeemer skip: {e}")))?;
    }
    // ex_units = [mem, cpu]
    let ex_hdr = read_array_header(redeemer_cbor, &mut offset)
        .map_err(|e| PlutusError::DecodeFailed(format!("ex_units header: {e}")))?;
    match ex_hdr {
        ContainerEncoding::Definite(2, _) => {}
        _ => {
            return Err(PlutusError::DecodeFailed(
                "ex_units: expected 2-field array".into(),
            ))
        }
    }
    let (mem, _) = read_uint(redeemer_cbor, &mut offset)
        .map_err(|e| PlutusError::DecodeFailed(format!("ex_units mem: {e}")))?;
    let (cpu, _) = read_uint(redeemer_cbor, &mut offset)
        .map_err(|e| PlutusError::DecodeFailed(format!("ex_units cpu: {e}")))?;
    Ok((clamp(cpu), clamp(mem)))
}

fn clamp(v: u64) -> i64 {
    if v > i64::MAX as u64 {
        i64::MAX
    } else {
        v as i64
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Semantic correctness of the underlying evaluator is proven by:
    //   - S-29 Flat decoder probe (6,899 / 6,899 mainnet scripts
    //     decode and round-trip byte-identically through aiken).
    //   - S-30 conformance harness (514 IOG conformance cases pass,
    //     0 budget mismatches across every non-skipped test).
    //
    // The wrapper here is a thin adapter: ~10 lines of aiken call
    // + redeemer ex_units extraction. Real-tx tests with resolved
    // UTxO pairs are a follow-up when Ade has the tx-CBOR →
    // pallas-compatible-UTxO-CBOR conversion infrastructure. That
    // conversion needs full Alonzo+ tx-output serialization which
    // ade_codec doesn't yet do. Tracked as S-31 follow-up.

    #[test]
    fn eval_rejects_malformed_tx_cbor() {
        let result = eval_tx_phase_two(
            &[0xff, 0xff, 0xff],
            &[],
            None,
            (1000, 1000),
            MAINNET_SLOT_CONFIG,
        );
        assert!(
            matches!(result, Err(PlutusError::DecodeFailed(_))),
            "expected DecodeFailed, got {result:?}"
        );
    }

    #[test]
    fn eval_rejects_empty_tx() {
        let result = eval_tx_phase_two(
            &[],
            &[],
            None,
            (1000, 1000),
            MAINNET_SLOT_CONFIG,
        );
        assert!(matches!(result, Err(PlutusError::DecodeFailed(_))));
    }

    #[test]
    fn mainnet_slot_config_constants() {
        // Document the mainnet values so a typo surfaces in review.
        let (system_start_ms, zero_slot, slot_length_ms) = MAINNET_SLOT_CONFIG;
        assert_eq!(system_start_ms, 1_506_203_091_000);
        assert_eq!(zero_slot, 0);
        assert_eq!(slot_length_ms, 1000);
    }

    #[test]
    fn per_script_result_is_ade_canonical() {
        // Regression guard: PerScriptResult must not grow fields that
        // leak aiken or pallas types across the crate boundary.
        let r = PerScriptResult {
            redeemer_cbor: vec![0x84],
            success: true,
            cpu: 100,
            mem: 50,
            logs: vec!["hello".into()],
        };
        // The only types referenced in this struct construction are
        // Vec<u8>, bool, i64, and Vec<String>. No aiken / pallas.
        assert_eq!(r.redeemer_cbor, vec![0x84]);
        assert!(r.success);
    }
}
