#!/bin/sh
# Install the pinned shader compiler required by the Nexus build script.
set -eu
cargo install cargo-gpu --version 0.10.0 --locked
cargo fetch --locked --manifest-path gpu/Cargo.toml
shader_manifest=$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" \
    -path '*/nexus_rbd_shaders3d-0.6.0/Cargo.toml' -print -quit)
test -n "$shader_manifest"
cargo gpu install --shader-crate "$(dirname "$shader_manifest")" \
    --auto-install-rust-toolchain
