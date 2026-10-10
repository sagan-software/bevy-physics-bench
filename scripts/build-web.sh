#!/bin/sh
# Bundle the CPU and GPU adapters into one module and build the static site.
set -eu
export RUSTFLAGS="${RUSTFLAGS:--C target-feature=+simd128}"
cargo build --locked --manifest-path gpu/Cargo.toml --release \
    --target wasm32-unknown-unknown --lib
wasm-bindgen --target web --out-dir web/pkg \
    --out-name bevy_physics_bench \
    "${CARGO_TARGET_DIR:-gpu/target}/wasm32-unknown-unknown/release/bevy_physics_bench_gpu.wasm"
cp results.schema.json web/public/results.schema.json
npm run build
