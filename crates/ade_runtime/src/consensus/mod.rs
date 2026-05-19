// Imperative-Shell module: parses genesis text into the BLUE-consumed
// EraSchedule. Reads files / JSON / parses; never reached by BLUE.

pub mod genesis_parser;

pub use genesis_parser::{
    compute_anchor_hash, parse_genesis, GenesisBlob, GenesisBundle,
    GenesisParseError, NetworkMagic,
};
