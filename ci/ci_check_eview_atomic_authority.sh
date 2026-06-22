#!/usr/bin/env bash
set -uo pipefail

# EPOCH-CONTINUITY-ACTIVATION ECA-2/3/4 (DC-EPOCH-14): the atomic epoch-authority transition +
# recovery. ONE owned ActiveEpochAuthority is the SOLE leadership + header-validation view source;
# the boundary swap is atomic (durable-before-visible); the forge enforces a slot-aware, MODE-aware
# epoch guard (the canonical EpochAuthorityMode -- recovered from durable state, NEVER an ambient
# flag); warm-start recovers the SAME promoted authority by RE-DERIVING the candidate and rejecting a
# non-recomputable record. This gate asserts the mechanism is present + the criteria proofs exist.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$REPO_ROOT"
FAILED=0; fail() { echo "FAIL: $1"; FAILED=1; }
EA=crates/ade_node/src/epoch_activation.rs
EZ=crates/ade_node/src/epoch_activate.rs
EW=crates/ade_node/src/epoch_wire.rs
NS=crates/ade_node/src/node_sync.rs
NL=crates/ade_node/src/node_lifecycle.rs

# (1) the CANONICAL authority mode is a TYPE (recovered from durable state), NOT an ambient bool.
grep -qF 'pub enum EpochAuthorityMode' "$EA" || fail "EpochAuthorityMode (the canonical authority mode) is missing"
grep -qF 'SeedOnly {' "$EA" || fail "EpochAuthorityMode::SeedOnly is missing"
grep -qF 'ContinuityRequired {' "$EA" || fail "EpochAuthorityMode::ContinuityRequired is missing"
# a runtime flag must NOT decide the authority mode / consensus behaviour (user 2026-06-23).
if grep -rnE 'eview_configured[[:space:]]*:[[:space:]]*bool' crates/ >/dev/null 2>&1; then
    fail "an ambient 'eview_configured: bool' flag must not decide the authority mode (DC-EPOCH-14)"
fi

# (2) the 3-way, slot-aware, mode-aware epoch guard verdict.
grep -qF 'pub enum AuthorityEpochVerdict' "$EA" || fail "AuthorityEpochVerdict (the 3-way guard verdict) is missing"
grep -qF 'SeedOnlyPastSupport' "$EA" || fail "the SeedOnly graceful no-forge verdict is missing"
grep -qF 'fn guard_epoch' "$EA" || fail "guard_epoch is missing"

# (3) the forge reads the ONE authority's view + enforces the guard BEFORE leadership (terminal mismatch).
grep -qF 'authority.pool_distr_view()' "$NS" || fail "the forge does not read the one authority's view"
grep -qF 'authority.guard_epoch(protocol_epoch)' "$NS" || fail "the forge does not enforce the epoch guard before leadership"
grep -qF 'AuthorityEpochMismatch' "$NS" || fail "the forge's terminal epoch-mismatch is missing"

# (4) RECOVERY EXACTNESS: re-derive + recover_active_view (reject-non-recomputable), NO new WAL write.
grep -qF 'pub fn recover_at_boundary' "$EZ" || fail "recover_at_boundary (the warm-start re-derive + recover) is missing"
grep -qF 'recover_active_view(Some(record), Some(&candidate))' "$EZ" || fail "recover_at_boundary does not recover against the durable record (reject-non-recomputable)"
grep -qF 'pub fn maybe_recover_promoted_authority' "$EW" || fail "maybe_recover_promoted_authority (the warm-start wiring) is missing"

# (5) the mode is built from the EVIEW package + the recovery is wired at relay setup BEFORE the loop.
grep -qF 'ActiveEpochAuthority::continuity(' "$NL" || fail "the relay setup does not build the ContinuityRequired mode from the EVIEW package"
grep -qF 'maybe_recover_promoted_authority(' "$NL" || fail "the warm-start recovery is not wired into the relay setup"
grep -qF 'resolve_activation_record(&entries)' "$NL" || fail "the relay setup does not resolve the durable activation record for recovery"

# (6) the criteria proofs exist.
for t in \
    authority_epoch_guard_is_mode_aware_and_identity_is_exact \
    cross_consumer_identity_validation_and_forge_resolve_one_authority_view \
    seed_only_sole_view_cannot_validate_n1_header_rejects_before_acceptance \
    forge_continuity_required_missing_promotion_at_n1_is_terminal \
    recover_at_boundary_round_trips_the_durable_record_and_rejects_a_tamper ; do
    grep -qrF "fn $t" crates/ade_node/src/ || fail "the DC-EPOCH-14 proof '$t' is missing"
done

if (( FAILED == 0 )); then
    echo "OK: atomic epoch-authority transition + recovery (DC-EPOCH-14; one holder + mode-aware guard + reject-non-recomputable warm-start recovery; criteria proofs present)"
fi
exit $FAILED
