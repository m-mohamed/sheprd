#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 VERSION OUTPUT" >&2
  exit 64
fi

VERSION="$1"
OUTPUT="$2"

awk -v version="$VERSION" '
  BEGIN {
    capture = 0
    found = 0
  }
  $0 ~ "^##[[:space:]]+v?" version "([[:space:]]|$)" {
    capture = 1
    found = 1
    next
  }
  capture && $0 ~ "^##[[:space:]]+" {
    capture = 0
  }
  capture {
    print
  }
  END {
    if (!found) {
      exit 2
    }
  }
' CHANGELOG.md > "$OUTPUT" || {
  rm -f "$OUTPUT"
  echo "error: CHANGELOG.md has no section for version $VERSION" >&2
  exit 2
}

if [ ! -s "$OUTPUT" ]; then
  rm -f "$OUTPUT"
  echo "error: CHANGELOG.md section for version $VERSION is empty" >&2
  exit 2
fi
