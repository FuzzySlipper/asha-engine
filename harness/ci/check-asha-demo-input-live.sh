#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEMO_ROOT="$REPO_ROOT/../asha-demo"
HOST="127.0.0.1"
PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
LOG_PATH="${TMPDIR:-/tmp}/asha-demo-input-live-${PORT}.log"
ASSERTION="resolved input owns pause, consumption, resume, and semantic replay"

npm --prefix "$DEMO_ROOT" run dev -- --host "$HOST" --port "$PORT" >"$LOG_PATH" 2>&1 &
SERVER_PID=$!

cleanup() {
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 120); do
  if curl --fail --silent "http://$HOST:$PORT/health" >/dev/null; then
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    cat "$LOG_PATH" >&2
    exit 1
  fi
  sleep 0.25
done

curl --fail --silent "http://$HOST:$PORT/health" >/dev/null || {
  cat "$LOG_PATH" >&2
  exit 1
}

BASE_URL="http://$HOST:$PORT" npm --prefix "$DEMO_ROOT" run test:live-ui -- --grep "$ASSERTION"
