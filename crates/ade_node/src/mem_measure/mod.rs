//! MEM-MEASURE-A1 — bounded inbound admission + memory-measurement substrate.
//!
//! Cluster MEM-MEASURE, slice A1 (hermetic — no live peer). Three TCB colors,
//! file-separated:
//!   - GREEN [`bounded_admission`] — the deterministic bounded inbound-admission
//!     fold that fronts the BLUE `mempool_ingress` (CN-MEM-01). Pure.
//!   - RED [`rss_sampler`] — the single `/proc/self/status` reader; observes
//!     process memory and influences no authoritative output.
//!   - GREEN [`evidence`] — the evidence-record schema, its validator, and the
//!     replay-fingerprint pairing (the load-bearing measurement discipline).
//!   - GREEN/RED [`runner`] — the measurement seam: drives the GREEN workload
//!     under RED sampling and emits a paired evidence record.
//!
//! No BLUE type is introduced; `mempool_ingress` is reused unchanged. RSS
//! magnitude never enters a fingerprint, verdict, or validator pass/fail.

pub mod bounded_admission;
pub mod evidence;
pub mod rss_sampler;
pub mod runner;

pub use bounded_admission::{
    replay_bounded_ingress_trace, BoundedOutcome, ShedReason, MAX_INBOUND_ADMISSION_BYTES,
    MAX_INBOUND_ADMISSION_COUNT,
};
pub use evidence::{
    fingerprint_hex, pair_replay, validate_evidence, EvidenceDefect, MemEvidenceRecord,
    ReplayVerdict,
};
pub use rss_sampler::{
    sample_private_dirty_kib, sample_rss_anon_kib, sample_vm_hwm_kib, sample_vm_rss_kib,
    RssSampleKib, RssWindow,
};
pub use runner::run_hermetic_bounded_ingress_measurement;
