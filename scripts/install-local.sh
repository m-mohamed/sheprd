#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_DIR="${SHEPRD_INSTALL_DIR:-$HOME/.local/bin}"

cd "$ROOT"

cargo build --release --locked

mkdir -p "$INSTALL_DIR"
install -m 755 target/release/sheprd "$INSTALL_DIR/sheprd"

echo "installed sheprd to $INSTALL_DIR/sheprd"
