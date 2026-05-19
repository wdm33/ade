// Imperative-Shell module: parses genesis text into the BLUE-consumed
// EraSchedule. Reads files / JSON / parses; never reached by BLUE.

pub mod candidate_fragment;
pub mod chain_selector;
pub mod genesis_parser;

pub use candidate_fragment::build_candidate_fragment;
pub use chain_selector::{
    process_stream_input, OrchestratorError, OrchestratorState, RollbackSnapshot, StreamInput,
    DEFAULT_SNAPSHOT_LIMIT,
};
pub use genesis_parser::{
    compute_anchor_hash, parse_genesis, GenesisBlob, GenesisBundle,
    GenesisParseError, NetworkMagic,
};
