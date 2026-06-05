#!/usr/bin/env bash
set -uo pipefail

# PHASE4-N-F-G-K S1 (DC-NODE-09): node serve lifetime decoupled from feed end.
# The --mode node --listen serve task's lifetime is owned by the node lifecycle
# owner (the operator shutdown watch), NOT the upstream feed. A clean feed-end
# halt must not tear down serving; the serve task is READ-ONLY (availability, not
# authority).
#
# PHASE4-N-U S3 (DC-NODE-13): the serve source migrated from the in-memory
# ServedChainView accumulator to the durable ChainDb projection
# (ServedChainSource::DurableChainDb). The serve task now takes the durable ChainDb
# as a READ source (Arc<dyn ChainDb>) -- still read-only: it advances no tip and
# admits nothing (no WAL / forge handle, no durable-write call).
#
#   (a) the On-arm spawns run_node_serve_task gated on the operator `shutdown`
#       watch (shutdown.clone()).
#   (b) the feed-end coupling is removed: no dedicated serve stop channel
#       (node_serve_stop) flipped after run_relay_loop returns.
#   (c) run_node_serve_task takes the durable ChainDb as a READ source
#       (Arc<dyn ChainDb>) and NO WAL / forge handle, serves via the read-only
#       durable projection (ServedChainSource::DurableChainDb), and its body
#       performs NO durable-write / admit call.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIFE="$REPO_ROOT/crates/ade_node/src/node_lifecycle.rs"

FAILED=0
print_fail() { echo "FAIL: $1"; FAILED=1; }

[[ -f "$LIFE" ]] || print_fail "missing expected source $LIFE"

# (a) the serve task is spawned gated on the operator shutdown watch (a clone),
#     proving its lifetime is owned by the lifecycle owner.
if ! grep -Eq 'run_node_serve_task\(' "$LIFE"; then
    print_fail "(a) run_node_serve_task spawn not found in the On-arm"
fi
if ! grep -Eq 'shutdown\.clone\(\)' "$LIFE"; then
    print_fail "(a) serve task is not gated on the operator shutdown watch (shutdown.clone())"
fi

# (b) the feed-end stop coupling is gone: the dedicated serve stop channel that the
#     old On-arm flipped right after run_relay_loop returned must not exist.
if grep -Eq 'node_serve_stop' "$LIFE"; then
    print_fail "(b) a dedicated serve stop channel (node_serve_stop) still exists -- feed-end coupling not removed"
fi

# (c) run_node_serve_task is READ-ONLY over the durable chain: it takes the durable
#     ChainDb as a read source (Arc<dyn ChainDb>) and NO WAL / forge handle, serves
#     via the read-only durable projection (ServedChainSource::DurableChainDb), and
#     its body makes no durable-write / admit call.
SIG="$(awk '/pub async fn run_node_serve_task\(/{f=1} f{print} f&&/\)/{exit}' "$LIFE")"
if printf '%s' "$SIG" | grep -Eq 'FileWalStore|WalStore|ForgeActivation|ProducerShell'; then
    print_fail "(c) run_node_serve_task signature gained a WAL/forge handle -- serve must not mutate authoritative state"
fi
if ! printf '%s' "$SIG" | grep -Eq 'Arc<dyn ChainDb>'; then
    print_fail "(c) run_node_serve_task no longer takes the durable ChainDb read source (Arc<dyn ChainDb>)"
fi
if ! grep -Eq 'ServedChainSource::DurableChainDb' "$LIFE"; then
    print_fail "(c) the serve task does not serve via the read-only durable projection (ServedChainSource::DurableChainDb)"
fi
# (c2) the serve task body performs NO durable-write / admit (read-only availability).
BODY="$(awk '/pub async fn run_node_serve_task\(/{f=1} f{print} f&&/^}/{exit}' "$LIFE")"
if printf '%s' "$BODY" | grep -Eq '\.put_block\(|put_snapshot\(|rollback_to_slot\(|AppendWal|pump_block\(|admit_forged_block_durably\('; then
    print_fail "(c2) run_node_serve_task body contains a durable-write/admit call -- serve must be read-only"
fi

if [[ "$FAILED" -ne 0 ]]; then
    echo "ci_check_node_serve_lifetime: FAILED"
    exit 1
fi
echo "ci_check_node_serve_lifetime: OK (DC-NODE-09 -- serve lifetime decoupled from feed end)"
