// Core Contract:
// - Deterministic: same inputs + same seed => byte-identical outputs
// - No wall-clock time, true randomness, HashMap/HashSet, or floats
//
// PHASE4-N-E S2 (DC-MEM-04 + DC-MEM-01 strengthening): GREEN harness
// for ingress-replay over the B-track adversarial corpus.

pub mod ingress_replay;

pub use ingress_replay::{
    b_track_corpus_as_ingress, replay_ingress_trace, wrap_as_ingress, BTrackCase,
    ExpectedOutcome,
};
