#!/usr/bin/env bash
#
# A freshly scaffolded firmware product compiles, for every target.
#
# Why this exists: the RP2040 and STM32 templates re-exported `Frame` and
# `FrameEncoder` from `fiducial-protocol`, which has never exported either.
# `fid add firmware rp2040` wrote a workspace that could not compile, and
# nothing noticed, because the platform's CI checked the repository's own
# firmware workspace and only read the templates as text.
#
# This scaffolds a product, adds each target, points the published
# `fiducial-*` crates at this checkout (so an unreleased version resolves),
# and runs the real compiler for the real target.
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-firmware.sh                       # every target
#   TARGETS="esp32c6" scripts/verify-firmware.sh
#   TARGETS="esp32c6-idf" scripts/verify-firmware.sh   # not in the default set
#
# `esp32c6-idf` is std Rust on ESP-IDF: nightly with build-std, and the first
# build downloads the SDK and its toolchain (gigabytes, minutes), so it is not
# in the default set. It needs `ldproxy` (cargo install ldproxy).
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
targets="${TARGETS:-rp2040 stm32 esp32c6}"

[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

triple() {
  case "$1" in
    rp2040) echo thumbv6m-none-eabi ;;
    stm32) echo thumbv7em-none-eabihf ;;
    esp32c6) echo riscv32imac-unknown-none-elf ;;
    esp32c6-idf) echo riscv32imac-esp-espidf ;;
    *) echo "unknown target $1" >&2; exit 1 ;;
  esac
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

for t in $targets; do
  echo "── firmware $t ($(triple "$t")) ──────────────────────────────────────"
  dir="$work/$t"
  mkdir -p "$dir"
  (
    cd "$dir"
    "$fid" new product >/dev/null
    cd product
    "$fid" add firmware "$t" >/dev/null
    cd firmware
    # The ESP-IDF target is its own workspace, so its manifest is its own.
    manifest=Cargo.toml
    [ "$t" = esp32c6-idf ] && manifest="$t/Cargo.toml"
    # Resolve the platform crates from this checkout, not crates.io.
    cat >> "$manifest" <<PATCH

[patch.crates-io]
fiducial-core     = { path = "$repo/crates/fiducial-core" }
fiducial-protocol = { path = "$repo/crates/fiducial-protocol" }
PATCH
    (cd "$t" && cargo build --release)
    if [ "$t" != esp32c6-idf ]; then
      cargo check --release -p firmware-shared --target "$(triple "$t")"
    fi
  )
done
echo "✓ every firmware target compiles"
