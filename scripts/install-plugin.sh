#!/usr/bin/env bash
# Herdr managed-install build step.
#
# The checkout's manifest version selects an exact GitHub release. Supported
# machines receive a SHA-256- and provenance-verified prebuilt binary;
# unavailable assets or verification tools fall back to a locked source build.
# A verification mismatch is a hard failure and never executes downloaded bytes.
set -euo pipefail

name="sheprd"
repo="m-mohamed/sheprd"
root="${SHEPRD_PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
manifest="$root/herdr-plugin.toml"
output="${SHEPRD_PLUGIN_OUTPUT:-$root/target/release/sheprd}"
release_base="https://github.com/$repo/releases/download"

have() {
  command -v "$1" >/dev/null 2>&1
}

build_from_source() {
  if [ -f "${HOME}/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "${HOME}/.cargo/env"
  fi
  if ! have cargo; then
    printf '%s\n' \
      "$name: no verified prebuilt is available and cargo was not found." \
      "Install Rust 1.92+ from https://rustup.rs, then retry:" \
      "  herdr plugin install $repo" >&2
    exit 1
  fi
  printf '%s\n' "$name: building the managed checkout from locked source"
  cargo build --release --locked --manifest-path "$root/Cargo.toml"
  built="$root/target/release/sheprd"
  if [ "$output" != "$built" ]; then
    mkdir -p "$(dirname "$output")"
    install -m 0755 "$built" "$output"
  fi
}

fallback() {
  printf '%s\n' "$name: $1; falling back to a locked source build." >&2
  build_from_source
  exit 0
}

download() {
  if have curl; then
    curl --proto '=https' --tlsv1.2 -fsSL --retry 5 --retry-delay 2 --retry-all-errors "$1" -o "$2"
  else
    return 127
  fi
}

sha256_of() {
  if have sha256sum; then
    sha256sum "$1" | awk '{print $1}'
  elif have shasum; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    return 127
  fi
}

version=$(sed -nE 's/^version = "([^"]+)"/\1/p' "$manifest" | head -n 1)
if [ -z "$version" ]; then
  fallback "could not read a version from $manifest"
fi
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]]; then
  printf '%s\n' "$name: refusing invalid manifest version '$version'" >&2
  exit 1
fi

os="${SHEPRD_UNAME_S:-$(uname -s)}"
arch="${SHEPRD_UNAME_M:-$(uname -m)}"
case "$os-$arch" in
  Darwin-arm64 | Darwin-aarch64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64 | Darwin-amd64) target="x86_64-apple-darwin" ;;
  Linux-aarch64 | Linux-arm64) target="aarch64-unknown-linux-musl" ;;
  Linux-x86_64 | Linux-amd64) target="x86_64-unknown-linux-musl" ;;
  *) fallback "no prebuilt target maps to $os/$arch" ;;
esac

archive="$name-$target.tar.gz"
checksum="$name-$target.sha256"
url="$release_base/v$version"
temporary=$(mktemp -d "${TMPDIR:-/tmp}/sheprd-install.XXXXXX")
trap 'rm -rf -- "$temporary"' EXIT

if ! download "$url/$archive" "$temporary/$archive"; then
  fallback "prebuilt $archive is unavailable for v$version"
fi
if ! download "$url/$checksum" "$temporary/$checksum"; then
  fallback "checksum $checksum is unavailable for v$version"
fi

expected=$(awk 'NF { print $1; exit }' "$temporary/$checksum")
if ! actual=$(sha256_of "$temporary/$archive"); then
  fallback "no SHA-256 tool is available"
fi
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
  printf '%s\n' \
    "$name: checksum mismatch for $archive" \
    "expected: ${expected:-missing}" \
    "actual:   $actual" >&2
  exit 1
fi

if ! have gh; then
  fallback "GitHub CLI is required to verify build provenance"
fi
if ! gh attestation verify "$temporary/$archive" --repo "$repo" >/dev/null; then
  printf '%s\n' "$name: build-provenance verification failed for $archive" >&2
  exit 1
fi

tar -xzf "$temporary/$archive" -C "$temporary"
if [ ! -f "$temporary/$name" ]; then
  printf '%s\n' "$name: verified archive does not contain $name" >&2
  exit 1
fi
mkdir -p "$(dirname "$output")"
install -m 0755 "$temporary/$name" "$output"
printf '%s\n' "$name: installed verified v$version binary for $target"
