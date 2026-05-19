// GREEN — deterministic but non-authoritative. Builds the canonical
// `CandidateFragment` that BLUE fork-choice consumes. Materialization
// lives here so the orchestrator (S-B10) and tests share a single
// construction path. No authoritative decision is made in this file;
// the only contract is the resulting fragment shape.

use ade_core::consensus::candidate::{CandidateFragment, TiebreakerView};
use ade_core::consensus::events::{BlockDistance, Point};
use ade_core::consensus::header_summary::ValidatedHeaderSummary;
use ade_types::BlockNo;

/// Construct a `CandidateFragment` from its component parts. Trivial
/// — exists so callers (S-B10 orchestrator, tests) have a uniform
/// construction path and the materialization surface is grep-able.
pub fn build_candidate_fragment(
    anchor: Point,
    anchor_block_no: BlockNo,
    headers: Vec<ValidatedHeaderSummary>,
    select_view: TiebreakerView,
    rollback_depth: BlockDistance,
) -> CandidateFragment {
    CandidateFragment {
        anchor,
        anchor_block_no,
        headers,
        select_view,
        rollback_depth,
    }
}
