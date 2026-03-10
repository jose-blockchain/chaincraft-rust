#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<EOF
Run multiple Chaincraft CLI nodes on localhost.

Usage:
  $(basename "$0") [NUM_NODES] [BASE_PORT] [--random]

Arguments:
  NUM_NODES  Number of nodes to start (default: 3)
  BASE_PORT  Starting TCP port (default: 21000)

Options:
  --random   Ignore BASE_PORT and choose random ports instead

Examples:
  # Start 3 nodes on ports 21000, 21001, 21002
  $(basename "$0")

  # Start 5 nodes on ports 22000..22004
  $(basename "$0") 5 22000

  # Start 4 nodes on random high ports
  $(basename "$0") 4 --random
EOF
}

NUM_NODES="${1:-3}"
BASE_PORT="${2:-21000}"
RANDOM_PORTS=false

if [[ "${2:-}" == "--random" || "${3:-}" == "--random" ]]; then
  RANDOM_PORTS=true
fi

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if ! [[ "$NUM_NODES" =~ ^[0-9]+$ ]]; then
  echo "ERROR: NUM_NODES must be an integer" >&2
  exit 1
fi

if [[ "$NUM_NODES" -le 0 ]]; then
  echo "ERROR: NUM_NODES must be > 0" >&2
  exit 1
fi

if ! $RANDOM_PORTS; then
  if ! [[ "$BASE_PORT" =~ ^[0-9]+$ ]]; then
    echo "ERROR: BASE_PORT must be an integer" >&2
    exit 1
  fi
fi

PIDS=()
PORTS=()

cleanup() {
  echo
  echo "Stopping nodes..."
  for pid in "${PIDS[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
}

trap cleanup INT TERM

echo "Starting $NUM_NODES Chaincraft CLI nodes..."

for ((i = 0; i < NUM_NODES; i++)); do
  if $RANDOM_PORTS; then
    # Use a random high port range to reduce collision probability
    PORT=$(( (RANDOM % 20000) + 20000 ))
  else
    PORT=$((BASE_PORT + i))
  fi

  PORTS+=("$PORT")

  echo "  Node $((i + 1)) -> port $PORT"

  cargo run --bin chaincraft-cli -- start --port "$PORT" --max-peers "$NUM_NODES" --memory --verbosity 3 &
  PIDS+=("$!")

  # Small delay so logs are readable and ports settle
  sleep 0.3
done

echo
echo "Nodes are running. Press Ctrl+C to stop all nodes."
echo "Ports: ${PORTS[*]}"

wait

