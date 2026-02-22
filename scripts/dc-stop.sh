#!/usr/bin/env bash
# Stop and remove a spec's devcontainer.
# Usage: dc-stop.sh <spec-id>
set -euo pipefail

SPEC_ID="${1:?Usage: dc-stop.sh <spec-id>}"
CONTAINER_NAME="dobonomodo-${SPEC_ID}"

if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "Stopping container: $CONTAINER_NAME"
    docker rm -f "$CONTAINER_NAME"
    echo "Container removed: $CONTAINER_NAME"
else
    echo "Container not found: $CONTAINER_NAME"
fi
