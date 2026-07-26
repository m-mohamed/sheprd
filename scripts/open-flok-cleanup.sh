#!/usr/bin/env bash
set -euo pipefail

exec "${HERDR_BIN_PATH:-herdr}" plugin pane open \
  --plugin "${HERDR_PLUGIN_ID:-m-mohamed.sheprd}" \
  --entrypoint cleanup-confirm \
  --placement popup \
  --width 90% \
  --height 80% \
  --focus
