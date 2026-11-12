#!/bin/sh
# Install a prebuilt `fid` — no Rust toolchain, a few seconds.
#
#   scripts/install-fid.sh 0.11.2            # into ~/.local/bin
#   FID_INSTALL_DIR=/usr/local/bin scripts/install-fid.sh 0.11.2
#
# From anywhere, pinned to the tag it installs:
#
#   curl -fsSL https://raw.githubusercontent.com/AleksaZCodes/fiducial/v0.11.2/scripts/install-fid.sh | sh -s -- 0.11.2
#
# The version is an argument and never "latest": which `fid` runs a product's
# gate is that product's decision, and a moving default makes yesterday's green
# build unreproducible.
#
# Prebuilt binaries exist for Linux x86-64 (glibc 2.39+, i.e. Ubuntu 24.04 and
# GitHub's `ubuntu-latest`). Anywhere else, or when the release carries no
# binary, this falls back to `cargo install` and says so.
set -eu

version="${1:-}"
if [ -z "$version" ]; then
  echo "usage: install-fid.sh <version>   e.g. install-fid.sh 0.11.2" >&2
  exit 2
fi
version="${version#v}"
dir="${FID_INSTALL_DIR:-$HOME/.local/bin}"

if command -v fid >/dev/null 2>&1 && [ "$(fid --version 2>/dev/null | awk '{print $NF}')" = "$version" ]; then
  echo "fid $version is already installed: $(command -v fid)"
  exit 0
fi

from_source() {
  echo "install-fid: $1 — building from source with cargo (this takes minutes)" >&2
  if ! command -v cargo >/dev/null 2>&1; then
    echo "install-fid: cargo is not installed either. Install Rust (https://rustup.rs) or use Linux x86-64." >&2
    exit 1
  fi
  cargo install fiducial-cli --version "$version" --locked
  exit 0
}

[ "$(uname -s)" = "Linux" ] || from_source "no prebuilt binary for $(uname -s)"
[ "$(uname -m)" = "x86_64" ] || from_source "no prebuilt binary for $(uname -m)"

asset="fid-$version-x86_64-unknown-linux-gnu.tar.gz"
url="https://github.com/AleksaZCodes/fiducial/releases/download/v$version/$asset"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$url" -o "$tmp/$asset" || from_source "release v$version has no $asset"
curl -fsSL "$url.sha256" -o "$tmp/$asset.sha256" || from_source "release v$version has no checksum for $asset"

# The checksum file is `<hash>  <name>`; compare the hash with what arrived.
want=$(cut -d' ' -f1 "$tmp/$asset.sha256")
got=$(sha256sum "$tmp/$asset" | cut -d' ' -f1)
if [ "$want" != "$got" ]; then
  echo "install-fid: checksum mismatch for $asset (expected $want, got $got)" >&2
  exit 1
fi

mkdir -p "$dir"
tar -xzf "$tmp/$asset" -C "$dir" fid
chmod +x "$dir/fid"

# A binary that cannot start here (an older glibc) is worse than none.
if ! "$dir/fid" --version >/dev/null 2>&1; then
  rm -f "$dir/fid"
  from_source "the prebuilt binary does not run on this system"
fi

echo "installed fid $version → $dir/fid"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "note: $dir is not on PATH — add it, or run $dir/fid directly" ;;
esac
