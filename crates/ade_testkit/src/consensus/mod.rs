// GREEN — corpus harness for consensus tests. Non-authoritative;
// affects only test infrastructure. Lives outside BLUE.
//
// S-B1 ships a minimal harness skeleton — the `hfc_schedule` corpus
// is consumed directly by integration tests under `ade_core/tests/`
// and `ade_runtime/tests/`. Later slices (S-B6, S-B8, S-B9, S-B10)
// extend this module with `nonce_evolution`, `leader_schedule`,
// `fork_choice`, `rollback`, and `consensus_stream_replay` drivers.

pub mod corpus;
