#!/usr/bin/env bash
# ci_check_rollback_retention_evidence.sh -- PHASE4-N-AO S13 (DC-NODE-40).
#
# Rolled-back blocks may be retained ONLY as walk-visible EVIDENCE: the LCA walk
# consults the retention on a per-peer-cache MISS to traverse non-durable
# intermediate headers (the blocks Ade itself rolled back), but the durable LCA
# anchor is ChainDb slot+hash ONLY -- a retained block is never the anchor, never
# durable, never a rollback target, never an S2/S4 bypass. The retention is a
# hash-keyed BTreeMap (self-binding, never HashMap-iterated for ordering), populated
# in apply_fork_switch BEFORE the rollback, and k-bounded by block depth.
#
# NOTE: checks use here-strings (grep <<< "$X"), NOT `echo "$X" | grep -q`: with
# `set -o pipefail`, grep -q exits on first match and the echo of a large file gets
# SIGPIPE -> a false failure. Here-strings avoid the pipe.
set -euo pipefail

W="crates/ade_node/src/lca_walk.rs"
M="crates/ade_node/src/node_lifecycle.rs"
fail() { echo "FAIL (ci_check_rollback_retention_evidence): $1" >&2; exit 1; }
[ -f "$W" ] || fail "module $W missing"
[ -f "$M" ] || fail "module $M missing"

WCODE="$(grep -vE '^[[:space:]]*//' "$W")"
WFLAT="$(echo "$WCODE" | tr '\n' ' ')"
MCODE="$(grep -vE '^[[:space:]]*//' "$M")"
MFLAT="$(echo "$MCODE" | tr '\n' ' ')"

# (A) The walk takes a retention EVIDENCE param and consults it ONLY on a per-peer-
# cache MISS (cache.get(..).or_else(|| retention.get(..))) -- never first-class.
grep -Eq 'retention:[[:space:]]*&BTreeMap' <<< "$WCODE" \
  || fail "the walk must take a retention: &BTreeMap evidence param"
grep -Eq '\.or_else\(\|\|[[:space:]]*retention\.get\(' <<< "$WFLAT" \
  || fail "the walk must consult retention only on a cache MISS (.or_else(|| retention.get(..)))"

# (B) The durable LCA anchor is the ChainDb-stored block ONLY -- a retained block is
# NEVER the anchor (anchor still binds get_block_by_hash slot+hash, DC-NODE-29).
grep -Eq 'get_block_by_hash\(&prev\)' <<< "$WCODE" \
  || fail "the LCA anchor must be the durable ChainDb block (get_block_by_hash), never retained"
if grep -Eq 'anchor_(slot|hash):[[:space:]]*retention' <<< "$WCODE"; then
  fail "a retained block must NEVER be the LCA anchor"
fi

# (C) Self-binding applies to EVERY visited entry (cache OR retention).
grep -Eq 'block_hash[[:space:]]*!=[[:space:]]*cur_hash' <<< "$WCODE" \
  || fail "every visited entry (cache or retention) must self-bind or fail closed"
grep -Eq 'CacheSelfBindingViolation' <<< "$WCODE" \
  || fail "self-binding violation must be a closed LcaError variant"

# (D) The retention is a deterministic BTreeMap, never a HashMap.
if grep -Eq 'retention.*HashMap|HashMap.*retention' <<< "$WCODE"; then
  fail "the retention must be a BTreeMap (no HashMap)"
fi

# (E) apply_fork_switch takes the &mut retention and POPULATES it (the rolled-back
# blocks), keyed by the re-derived block_hash (self-binding, never peer-claimed).
grep -Eq 'rollback_retention:[[:space:]]*&mut[[:space:]]*BTreeMap' <<< "$MCODE" \
  || fail "apply_fork_switch must take &mut rollback_retention to populate"
grep -Eq 'rollback_retention\.insert\([[:space:]]*[a-z_]+\.block_hash' <<< "$MFLAT" \
  || fail "the retention key must be the re-derived block_hash (self-binding population)"

# (F) k-BOUND eviction (no unbounded growth) by block depth (security_param).
grep -Eq 'rollback_retention\.retain\(' <<< "$MCODE" \
  || fail "the retention must be k-bounded (retain/evict)"
grep -Eq 'saturating_sub\(security_param\.0\)' <<< "$MFLAT" \
  || fail "the k-bound eviction must use the block-depth security_param.0"

# (G) The dispatch passes the retention to the walk (the consult is wired live).
grep -Eq 'walk_to_durable_lca\([^)]*rollback_retention' <<< "$MFLAT" \
  || fail "the dispatch must pass rollback_retention to walk_to_durable_lca"

echo "OK: rollback-retention is walk-evidence-only (cache-miss consult, ChainDb-only anchor), self-binding + BTreeMap, populated before rollback + k-bounded (DC-NODE-40)"
