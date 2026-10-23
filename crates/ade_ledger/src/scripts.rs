// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
// - Encode invariants in types
// - Explicit state transitions only
// - Canonical serialization for all persisted/hashed data

use std::collections::BTreeSet;
use ade_types::allegra::script::{NativeScript, NativeScriptNode};
use ade_types::Hash28;

use crate::error::NativeScriptFailure;

/// Result of evaluating a script against its environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptVerdict {
    /// All conditions of the native script were satisfied.
    NativeScriptPassed,
    /// At least one condition of the native script was not satisfied.
    NativeScriptFailed(NativeScriptFailure),
    /// Plutus script executed successfully within the declared
    /// ex_units budget. The verdict agrees with `isValid = True`.
    PlutusPassed {
        /// CPU steps consumed.
        cpu: i64,
        /// Memory units consumed.
        mem: i64,
    },
    /// Plutus script failed during evaluation. Triggers phase-2
    /// (collateral consumed) if `isValid = True` was declared,
    /// or phase-1 if `isValid = False` was declared and this
    /// script should have failed (tx dropped).
    PlutusFailed {
        /// CPU steps attempted before the failure.
        cpu_attempted: i64,
        /// Memory units attempted before the failure.
        mem_attempted: i64,
        /// Classification of the failure cause.
        reason: PlutusFailureReason,
    },
}

/// Classification of Plutus script evaluation failure modes.
///
/// Mirrors the Haskell `FailureDescription` from `cardano-ledger`'s
/// `AlonzoUtxosPredFailure::ValidationTagMismatch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlutusFailureReason {
    /// CEK machine returned an error term or the evaluator
    /// exhausted its budget (over-ran declared ex_units).
    ExecutionFailed,
    /// Script context / redeemer / cost-model couldn't be
    /// constructed. Mirrors `CollectErrors`.
    ContextBuildFailed,
    /// Budget consumed exceeded the declared ex_units ceiling.
    BudgetExhausted,
}

/// Deterministic classification of a transaction's script posture.
///
/// This is a classification surface, not a verdict. It describes what
/// kind of scripts a transaction contains, determined purely from the
/// parsed transaction body structure — no execution, no state lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptPosture {
    /// Transaction contains no script-related indicators.
    NoScripts,
    /// Transaction may contain native scripts but no Plutus indicators.
    /// (No `script_data_hash` in the body.)
    NonPlutusScriptsOnly,
    /// Transaction contains Plutus script indicators (`script_data_hash`
    /// present). Execution is deferred to Phase 3.
    PlutusPresentDeferred,
}

/// A transaction's validity interval: body key 8 (`validity_interval_start`) and key 3 (`ttl`).
/// A missing start is −∞ and a missing expiry is +∞ (cardano-ledger `ValidityInterval`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidityInterval {
    pub invalid_before: Option<u64>,
    pub invalid_hereafter: Option<u64>,
}

impl ValidityInterval {
    /// No bound on either side.
    pub const UNBOUNDED: ValidityInterval =
        ValidityInterval { invalid_before: None, invalid_hereafter: None };
}

/// Evaluate a native script against the transaction's signer key hashes and validity interval.
///
/// Deterministic: same inputs always produce the same verdict. cardano-ledger `evalTimelock`:
/// - Sig: the key hash is in `signers`
/// - All: all sub-scripts must pass; the first failure in order is reported
/// - Any: at least one sub-script must pass
/// - MOfN: at least M sub-scripts must pass (M <= 0 always passes)
/// - InvalidBefore(lock): `lock <= tx start`; never holds when the tx has no start
/// - InvalidHereafter(lock): `tx expiry <= lock`; never holds when the tx has no expiry
///
/// The containing block's slot plays no part: timelocks constrain the transaction's own interval,
/// which the ledger separately requires to contain the slot.
///
/// One forward pass over the post-order nodes: each node's sub-script verdicts are the last `arity`
/// entries of a heap stack, so stack use does not grow with nesting depth (DC-LEDGER-SCRIPT-02).
/// Every sub-script is evaluated; evaluation has no side effects, so the verdict is the one a
/// short-circuiting evaluator reaches.
pub fn evaluate_native_script(
    script: &NativeScript,
    signers: &BTreeSet<Hash28>,
    interval: &ValidityInterval,
) -> ScriptVerdict {
    let mut verdicts: Vec<ScriptVerdict> = Vec::new();
    for node in script.nodes() {
        // from_post_order guarantees the stack holds at least `arity` verdicts here.
        let arity = usize::try_from(node.arity()).unwrap_or(usize::MAX).min(verdicts.len());
        let subs = verdicts.split_off(verdicts.len() - arity);
        verdicts.push(evaluate_node(node, &subs, signers, interval));
    }
    verdicts.pop().unwrap_or(ScriptVerdict::NativeScriptPassed)
}

fn evaluate_node(
    node: &NativeScriptNode,
    subs: &[ScriptVerdict],
    signers: &BTreeSet<Hash28>,
    interval: &ValidityInterval,
) -> ScriptVerdict {
    match node {
        NativeScriptNode::Sig(key_hash) => {
            if signers.contains(key_hash) {
                ScriptVerdict::NativeScriptPassed
            } else {
                ScriptVerdict::NativeScriptFailed(
                    NativeScriptFailure::MissingRequiredSignature {
                        key_hash: key_hash.clone(),
                    },
                )
            }
        }

        NativeScriptNode::All { .. } => {
            for verdict in subs {
                if let ScriptVerdict::NativeScriptFailed(reason) = verdict {
                    return ScriptVerdict::NativeScriptFailed(reason.clone());
                }
            }
            ScriptVerdict::NativeScriptPassed
        }

        NativeScriptNode::Any { .. } => {
            // Empty Any: no sub-script can pass → threshold not met
            if subs.contains(&ScriptVerdict::NativeScriptPassed) {
                ScriptVerdict::NativeScriptPassed
            } else {
                ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                    required: 1,
                    provided: 0,
                })
            }
        }

        NativeScriptNode::MOfN { threshold, .. } => {
            // cardano-ledger isValidMOf: a threshold of zero or below is satisfied outright.
            let Ok(needed) = u64::try_from(*threshold) else {
                return ScriptVerdict::NativeScriptPassed;
            };
            if needed == 0 {
                return ScriptVerdict::NativeScriptPassed;
            }
            let passed = subs
                .iter()
                .filter(|v| **v == ScriptVerdict::NativeScriptPassed)
                .count() as u64;
            if passed >= needed {
                ScriptVerdict::NativeScriptPassed
            } else {
                ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                    required: *threshold,
                    provided: passed,
                })
            }
        }

        // lteNegInfty lock txStart: false when the start is −∞.
        NativeScriptNode::InvalidBefore(lock) => match interval.invalid_before {
            Some(start) if *lock <= start => ScriptVerdict::NativeScriptPassed,
            tx_start => ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockStartNotMet {
                lock_start: ade_types::SlotNo(*lock),
                tx_start: tx_start.map(ade_types::SlotNo),
            }),
        },

        // ltePosInfty txExp lock: false when the expiry is +∞.
        NativeScriptNode::InvalidHereafter(lock) => match interval.invalid_hereafter {
            Some(expiry) if expiry <= *lock => ScriptVerdict::NativeScriptPassed,
            tx_expiry => ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockExpiryNotMet {
                lock_expiry: ade_types::SlotNo(*lock),
                tx_expiry: tx_expiry.map(ade_types::SlotNo),
            }),
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sigs(hashes: &[[u8; 28]]) -> BTreeSet<Hash28> {
        hashes.iter().map(|h| Hash28(*h)).collect()
    }

    // -----------------------------------------------------------------------
    // Sig tests
    // -----------------------------------------------------------------------

    #[test]
    fn sig_passes_when_key_present() {
        let key = Hash28([0x01; 28]);
        let available = sigs(&[[0x01; 28]]);
        let verdict = evaluate_native_script(&NativeScript::sig(key), &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn sig_fails_when_key_absent() {
        let key = Hash28([0x01; 28]);
        let available = sigs(&[[0x02; 28]]);
        let verdict = evaluate_native_script(&NativeScript::sig(key.clone()), &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(
            verdict,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::MissingRequiredSignature {
                key_hash: key,
            })
        );
    }

    // -----------------------------------------------------------------------
    // All tests
    // -----------------------------------------------------------------------

    #[test]
    fn all_passes_when_all_subs_pass() {
        let script = NativeScript::all(vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
        ]);
        let available = sigs(&[[0x01; 28], [0x02; 28]]);
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn all_fails_when_one_sub_fails() {
        let script = NativeScript::all(vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x03; 28])),
        ]);
        let available = sigs(&[[0x01; 28]]);
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        match verdict {
            ScriptVerdict::NativeScriptFailed(_) => {}
            _ => unreachable!(),
        }
    }

    #[test]
    fn all_empty_passes() {
        let script = NativeScript::all(vec![]);
        let available = BTreeSet::new();
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    // -----------------------------------------------------------------------
    // Any tests
    // -----------------------------------------------------------------------

    #[test]
    fn any_passes_when_one_sub_passes() {
        let script = NativeScript::any(vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
        ]);
        let available = sigs(&[[0x02; 28]]);
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn any_fails_when_none_pass() {
        let script = NativeScript::any(vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
        ]);
        let available = sigs(&[[0x03; 28]]);
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(
            verdict,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                required: 1,
                provided: 0,
            })
        );
    }

    #[test]
    fn any_empty_fails() {
        let script = NativeScript::any(vec![]);
        let available = BTreeSet::new();
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(
            verdict,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                required: 1,
                provided: 0,
            })
        );
    }

    // -----------------------------------------------------------------------
    // MOfN tests
    // -----------------------------------------------------------------------

    #[test]
    fn m_of_n_passes_when_threshold_met() {
        let script = NativeScript::m_of_n(2, vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
            NativeScript::sig(Hash28([0x03; 28])),
        ]);
        let available = sigs(&[[0x01; 28], [0x03; 28]]);
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    #[test]
    fn m_of_n_fails_when_threshold_not_met() {
        let script = NativeScript::m_of_n(2, vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::sig(Hash28([0x02; 28])),
            NativeScript::sig(Hash28([0x03; 28])),
        ]);
        let available = sigs(&[[0x01; 28]]);
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(
            verdict,
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::ThresholdNotMet {
                required: 2,
                provided: 1,
            })
        );
    }

    #[test]
    fn m_of_n_zero_threshold_always_passes() {
        let script = NativeScript::m_of_n(0, vec![
            NativeScript::sig(Hash28([0x01; 28])),
        ]);
        let available = BTreeSet::new();
        let verdict = evaluate_native_script(&script, &available, &ValidityInterval::UNBOUNDED);
        assert_eq!(verdict, ScriptVerdict::NativeScriptPassed);
    }

    // -----------------------------------------------------------------------
    // Timelock tests: against the transaction's validity interval (cardano-ledger evalTimelock)
    // -----------------------------------------------------------------------

    fn iv(start: Option<u64>, expiry: Option<u64>) -> ValidityInterval {
        ValidityInterval { invalid_before: start, invalid_hereafter: expiry }
    }

    fn eval_iv(script: &NativeScript, start: Option<u64>, expiry: Option<u64>) -> ScriptVerdict {
        evaluate_native_script(script, &BTreeSet::new(), &iv(start, expiry))
    }

    #[test]
    fn invalid_before_holds_when_the_tx_starts_at_or_after_the_lock() {
        let script = NativeScript::invalid_before(100);
        assert_eq!(eval_iv(&script, Some(100), None), ScriptVerdict::NativeScriptPassed);
        assert_eq!(eval_iv(&script, Some(101), None), ScriptVerdict::NativeScriptPassed);
        assert_eq!(
            eval_iv(&script, Some(99), None),
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockStartNotMet {
                lock_start: ade_types::SlotNo(100),
                tx_start: Some(ade_types::SlotNo(99)),
            })
        );
    }

    #[test]
    fn invalid_before_never_holds_without_a_tx_start() {
        let script = NativeScript::invalid_before(0);
        assert_eq!(
            eval_iv(&script, None, Some(10)),
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockStartNotMet {
                lock_start: ade_types::SlotNo(0),
                tx_start: None,
            })
        );
    }

    #[test]
    fn invalid_hereafter_holds_when_the_tx_expires_at_or_before_the_lock() {
        let script = NativeScript::invalid_hereafter(200);
        assert_eq!(eval_iv(&script, None, Some(200)), ScriptVerdict::NativeScriptPassed);
        assert_eq!(eval_iv(&script, None, Some(199)), ScriptVerdict::NativeScriptPassed);
        assert_eq!(
            eval_iv(&script, None, Some(201)),
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockExpiryNotMet {
                lock_expiry: ade_types::SlotNo(200),
                tx_expiry: Some(ade_types::SlotNo(201)),
            })
        );
    }

    #[test]
    fn invalid_hereafter_never_holds_without_a_tx_expiry() {
        let script = NativeScript::invalid_hereafter(u64::MAX);
        assert_eq!(
            eval_iv(&script, Some(0), None),
            ScriptVerdict::NativeScriptFailed(NativeScriptFailure::TimelockExpiryNotMet {
                lock_expiry: ade_types::SlotNo(u64::MAX),
                tx_expiry: None,
            })
        );
    }

    // -----------------------------------------------------------------------
    // Composed / nested tests
    // -----------------------------------------------------------------------

    #[test]
    fn time_window_script() {
        // Satisfied only by a tx interval inside [100, 200].
        let script = NativeScript::all(vec![
            NativeScript::invalid_before(100),
            NativeScript::invalid_hereafter(200),
        ]);
        assert!(matches!(eval_iv(&script, Some(50), Some(150)), ScriptVerdict::NativeScriptFailed(_)));
        assert_eq!(eval_iv(&script, Some(120), Some(180)), ScriptVerdict::NativeScriptPassed);
        assert!(matches!(eval_iv(&script, Some(150), Some(250)), ScriptVerdict::NativeScriptFailed(_)));
        assert!(matches!(eval_iv(&script, Some(150), None), ScriptVerdict::NativeScriptFailed(_)));
    }

    #[test]
    fn sig_plus_timelock() {
        let script = NativeScript::all(vec![
            NativeScript::sig(Hash28([0xaa; 28])),
            NativeScript::invalid_before(50),
        ]);
        let available = sigs(&[[0xaa; 28]]);
        let v = evaluate_native_script(&script, &available, &iv(Some(10), None));
        assert!(matches!(v, ScriptVerdict::NativeScriptFailed(_)));
        let v = evaluate_native_script(&script, &available, &iv(Some(50), None));
        assert_eq!(v, ScriptVerdict::NativeScriptPassed);
        let v = evaluate_native_script(&script, &BTreeSet::new(), &iv(Some(50), None));
        assert!(matches!(v, ScriptVerdict::NativeScriptFailed(_)));
    }

    #[test]
    fn determinism_same_inputs_same_output() {
        let script = NativeScript::m_of_n(1, vec![
            NativeScript::sig(Hash28([0x01; 28])),
            NativeScript::invalid_before(10),
        ]);
        let available = sigs(&[[0x01; 28]]);

        let v1 = evaluate_native_script(&script, &available, &iv(Some(5), None));
        let v2 = evaluate_native_script(&script, &available, &iv(Some(5), None));
        assert_eq!(v1, v2);
    }

}
