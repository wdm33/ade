//! `ade_mem_diag` — RED DIAGNOSTIC crate (allocator diagnostic probe).
//!
//! ===========================================================================
//! CONTRACT (load-bearing — MEM-OPT-UTXO-DISK):
//! - RED diagnostic crate ONLY.
//! - No authoritative semantics.
//! - No BLUE dependencies, and NO BLUE crate may depend on this crate.
//! - No returned value (this crate returns nothing) may influence the ledger,
//!   consensus, storage, chain selection, replay fingerprints, or protocol
//!   behavior.
//!
//! ===========================================================================
//!
//! Sole purpose: quarantine the one `unsafe` FFI call the workspace needs for a
//! process-memory diagnostic — mimalloc `mi_collect(force=true)` — so that
//! `ade_node` (the node authority/binary crate) keeps `#![deny(unsafe_code)]`
//! with ZERO local exceptions.
//!
//! Used by `ade_node` ONLY behind the S0 diagnostic env toggle
//! (`ADE_MEM_PHASE_DIAGNOSTIC`), off the authoritative admission loop. The
//! forced reclaim is admissible only as a MEASUREMENT INTERVENTION; it is never
//! a production memory-management requirement and never a replay-agreement
//! dependency. The S0 transcript labels the post-collect sample explicitly
//! (`t3_after_forced_allocator_collect_diagnostic_only`).
//!
//! This crate deliberately does NOT carry `#![deny(unsafe_code)]`: it is the
//! quarantine. The single `unsafe` block below is the entire unsafe surface.

/// Force the process allocator (mimalloc) to return freed-but-retained pages to
/// the OS, so a subsequent owned-RSS sample reflects the reclaimed state. This
/// is the S0 diagnostic's decisive control: mimalloc's default `MADV_FREE`
/// purging leaves freed pages resident until memory pressure, so without an
/// explicit `mi_collect(force)` a retained-freed footprint and a live working
/// set are indistinguishable ("high and flat").
///
/// DIAGNOSTIC ONLY (see the crate contract). It changes no authoritative output
/// — it returns only already-freed memory and cannot alter the ledger, any
/// fingerprint, or a replay verdict. On a non-mimalloc build (e.g. a unit test
/// on the default allocator) it collects an empty mimalloc heap: a harmless
/// no-op.
pub fn force_allocator_collect_for_diagnostic_only() {
    // SAFETY: `mi_collect` is a libmimalloc entrypoint with no preconditions; it
    // reclaims only freed memory and cannot affect any live allocation. mimalloc
    // is the `ade_node` process global allocator, so the symbol is linked there;
    // elsewhere it operates on an empty mimalloc heap (a no-op).
    unsafe { libmimalloc_sys::mi_collect(true) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The probe is callable and a safe no-op on the default test allocator
    /// (it never panics; it returns nothing observable).
    #[test]
    fn collect_is_a_safe_noop_to_call() {
        force_allocator_collect_for_diagnostic_only();
        force_allocator_collect_for_diagnostic_only();
    }
}
