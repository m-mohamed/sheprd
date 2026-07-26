#!/usr/bin/env bash
set -euo pipefail

command=(
  "${HERDR_BIN_PATH:-herdr}" plugin pane open
  --plugin "${HERDR_PLUGIN_ID:-m-mohamed.sheprd}"
  --entrypoint cleanup-confirm
  --placement overlay
  --focus
)
if [ -n "${HERDR_PLUGIN_CONTEXT_JSON:-}" ]; then
  command+=(--env "HERDR_PLUGIN_CONTEXT_JSON=$HERDR_PLUGIN_CONTEXT_JSON")
fi
exec "${command[@]}"
