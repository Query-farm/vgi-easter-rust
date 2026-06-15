#!/usr/bin/env bash
# Copyright 2026 Query Farm LLC - https://query.farm
#
# Launch the vgi-easter worker in HTTP mode and bridge external :8080 to the
# ephemeral 127.0.0.1 port it announces (`PORT:<n>` on stdout). The vgi Rust SDK
# does not (yet) expose a bind address/port for `--http`, so we forward instead.
set -euo pipefail

PUBLIC_PORT="${PORT:-8080}"

# Start the worker, capture its announced port from the `PORT:<n>` line.
coproc WORKER { exec vgi-easter --http; }

INNER_PORT=""
while IFS= read -r line <&"${WORKER[0]}"; do
  echo "$line"
  if [[ "$line" == PORT:* ]]; then
    INNER_PORT="${line#PORT:}"
    break
  fi
done

if [[ -z "$INNER_PORT" ]]; then
  echo "worker did not announce a port" >&2
  exit 1
fi

echo "Bridging 0.0.0.0:${PUBLIC_PORT} -> 127.0.0.1:${INNER_PORT}" >&2
# Keep echoing the worker's remaining stdout in the background.
cat <&"${WORKER[0]}" &

exec socat "TCP-LISTEN:${PUBLIC_PORT},fork,reuseaddr" "TCP:127.0.0.1:${INNER_PORT}"
