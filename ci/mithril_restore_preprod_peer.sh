#!/usr/bin/env bash
set -euo pipefail

# PHASE4-N-M-A1.1 (operator infra path) — Mithril restore for
# the docker `cardano-node-preprod` peer.
#
# DOCTRINE BOUNDARY (READ THIS FIRST):
#
#   Mithril is used here as an OPERATOR/INFRA accelerator for
#   the HASKELL cardano-node peer ONLY. It MUST NOT be:
#     - a BLUE trust root for Ade
#     - a substitute for `--json-seed`
#     - a substitute for `--consensus-inputs-json`
#     - a substitute for `admit_via_block_validity`
#     - claimed as evidence that "Ade supports Mithril import"
#
#   Ade-side Mithril import / verification / anchor-binding is
#   the RO-MITHRIL-IMPORT-01 open obligation, NOT closed by
#   running this script. See:
#     - docs/evidence/phase4-n-m-c-operator-pass-README.md §7
#     - memory [[feedback-mithril-is-peer-infra-not-ade-authority]]
#
# What this script does:
#   1. Stops the docker `cardano-node-preprod` container.
#   2. Downloads + restores the latest preprod Mithril snapshot
#      into the bind-mounted `.cardano-node-preprod/db/` dir
#      via the `mithril-client` CLI.
#   3. Restarts the docker peer.
#   4. Reports the peer's tip + sync progress on next stable
#      query.
#
# What this script does NOT do:
#   - Touch any file under `crates/`, `docs/evidence/`, or any
#     Ade-side seed / bundle / WAL / snapshot file.
#   - Run any Ade binary.
#   - Make any registry edits.
#
# Requirements:
#   - `docker` + `docker start/stop` access for
#     `cardano-node-preprod`.
#   - `mithril-client` CLI on PATH. Install:
#       https://mithril.network/doc/manual/getting-started/bootstrap-cardano-node
#   - Network access to the Mithril preprod aggregator.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PEER_DIR="$REPO_ROOT/.cardano-node-preprod"
DB_DIR="$PEER_DIR/db"
CONTAINER="${ADE_LIVE_PEER_CONTAINER:-cardano-node-preprod}"

PREPROD_AGGREGATOR_ENDPOINT="${MITHRIL_AGGREGATOR_ENDPOINT:-https://aggregator.release-preprod.api.mithril.network/aggregator}"
PREPROD_GENESIS_VERIFICATION_KEY="${MITHRIL_GENESIS_VERIFICATION_KEY:-5b3132372c37332c3132342c3136312c362c3133372c3133312c3231332c3230372c3131372c3139382c38352c3137362c3139392c3136322c3234312c36382c3132332c3131392c3134352c31332c3233322c3234332c34392c3232392c322c3234392c3230352c3230362c39372c39312c3131385d}"

if ! command -v mithril-client >/dev/null 2>&1; then
    cat >&2 <<'EOF'
FATAL: `mithril-client` CLI not found on PATH.

Install per https://mithril.network/doc/manual/getting-started/bootstrap-cardano-node
or set ADE_LIVE_MITHRIL_CLIENT to the binary path.
EOF
    if [[ -n "${ADE_LIVE_MITHRIL_CLIENT:-}" ]]; then
        MITHRIL="$ADE_LIVE_MITHRIL_CLIENT"
    else
        exit 2
    fi
else
    MITHRIL="$(command -v mithril-client)"
fi

if [[ ! -d "$DB_DIR" ]]; then
    echo "FATAL: expected DB dir missing: $DB_DIR" >&2
    exit 2
fi

echo "==> Stopping docker peer container: $CONTAINER"
docker stop "$CONTAINER" || true

# Clear the existing DB. Mithril restores a coherent snapshot;
# mixing it with a partial existing DB is unsupported.
echo "==> Clearing existing peer DB at $DB_DIR"
read -p "Type YES to wipe $DB_DIR and restore from Mithril (or Ctrl-C to abort): " CONFIRM
if [[ "$CONFIRM" != "YES" ]]; then
    echo "Aborted (no changes made)."
    exit 1
fi
rm -rf "$DB_DIR"
mkdir -p "$DB_DIR"

echo "==> Selecting latest preprod Mithril snapshot"
SNAPSHOT_DIGEST=$(
    AGGREGATOR_ENDPOINT="$PREPROD_AGGREGATOR_ENDPOINT" \
    GENESIS_VERIFICATION_KEY="$PREPROD_GENESIS_VERIFICATION_KEY" \
    "$MITHRIL" cardano-db snapshot list --json \
        | python3 -c "
import json, sys
xs = json.load(sys.stdin)
if not xs:
    print('FATAL: aggregator returned empty snapshot list', file=sys.stderr)
    sys.exit(2)
print(xs[0]['digest'])
"
)
if [[ -z "$SNAPSHOT_DIGEST" ]]; then
    echo "FATAL: no Mithril snapshot digest returned" >&2
    exit 2
fi
echo "    selected snapshot digest: $SNAPSHOT_DIGEST"

echo "==> Downloading + restoring snapshot to $DB_DIR"
AGGREGATOR_ENDPOINT="$PREPROD_AGGREGATOR_ENDPOINT" \
GENESIS_VERIFICATION_KEY="$PREPROD_GENESIS_VERIFICATION_KEY" \
"$MITHRIL" cardano-db download "$SNAPSHOT_DIGEST" \
    --download-dir "$DB_DIR"

# The Mithril client may write into a sub-directory named after
# the digest. If so, lift the contents to $DB_DIR/.
if [[ -d "$DB_DIR/$SNAPSHOT_DIGEST" ]]; then
    echo "==> Flattening restored snapshot dir"
    shopt -s dotglob
    mv "$DB_DIR/$SNAPSHOT_DIGEST"/* "$DB_DIR/"
    rmdir "$DB_DIR/$SNAPSHOT_DIGEST"
    shopt -u dotglob
fi

echo "==> Restarting docker peer container"
docker start "$CONTAINER"

echo "==> Waiting 30s for peer to start replaying"
sleep 30

echo "==> Querying peer tip (may take a while to settle)"
docker exec "$CONTAINER" sh -c \
    "cardano-cli query tip --testnet-magic 1 --socket-path /opt/cardano/ipc/node.socket" \
    || echo "    (peer not yet ready — re-query after a few minutes)"

cat <<'EOF'

==> Done.

Reminder of the doctrine boundary (also in
docs/evidence/phase4-n-m-c-operator-pass-README.md §7):

  Mithril here accelerated the docker HASKELL peer ONLY.
  It did NOT close RO-MITHRIL-IMPORT-01.
  It did NOT substitute for Ade's --json-seed or
  --consensus-inputs-json.
  All admission still flows through Ade's BLUE
  admit_via_block_validity authority.

Next: wait for peer syncProgress ≥ 99%, then run the C5
operator pass per §4 of the operator-pass-README.
EOF
