// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PROPOSAL-PROCEDURES-DECODE PP-S2: GREEN harness for canonical
// proposal_procedures corpus replay (CE-PP-6). Synthetic canonical
// fixtures per OQ-5 — the existing in-tree Conway corpus contains no
// real-chain txs carrying proposal_procedures at this HEAD.

pub mod proposal_procedures_replay;

pub use proposal_procedures_replay::{
    canonical_corpus, replay_canonical_corpus, CorpusEntry, ReplayOutcome,
};
