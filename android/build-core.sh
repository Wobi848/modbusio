#!/bin/sh
# Builds modbusio for Android (aarch64) and copies the binary to the path given as argument. The
# Gradle task cargoBuild sets the NDK's linker and calls this script.
set -eu

destination=$1
repo=$(cd "$(dirname "$0")/.." && pwd)

cargo build --release --target aarch64-linux-android --manifest-path "$repo/Cargo.toml" >&2
mkdir -p "$(dirname "$destination")"
cp "$repo/target/aarch64-linux-android/release/modbusio" "$destination"
