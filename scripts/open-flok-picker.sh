#!/usr/bin/env bash
set -euo pipefail

exec "${HERDR_BIN_PATH:-herdr}" plugin pane open \
  --plugin "${HERDR_PLUGIN_ID:-m-mohamed.sheprd}" \
  --entrypoint project-picker \
  --placement popup \
  --width 80% \
  --height 80% \
  --focus
