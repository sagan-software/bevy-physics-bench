# Contributing

Use a clean checkout and retain the exact benchmark settings and raw samples.
Read [the measurement method](docs/METHOD.md) before interpreting a comparison.

## Submit a result

1. Run a native command from the README or export a browser comparison.
2. Record a stable machine label and relevant collection conditions.
3. Run `node scripts/validate-results.mjs path/to/run.json`.
4. Copy the record into `web/public/results/` with a unique descriptive `.json` filename.
5. Add that filename to `web/public/results/index.json`.
6. Run `npm run validate:results`.
7. Open a pull request describing the hardware and collection conditions.

Published records require a clean 40-character source revision. Native records
require detected CPU metadata. Do not edit samples or statistics to
improve a score. Preserve old files; a new run gets a new filename.

Browser exports are local until submitted. Review machine labels, OS details,
and browser metadata before publication. Unknown browser CPU and memory fields
must remain null. A record's hash checks shared scene identity; it is not a
signature and does not authenticate a contributor or their measurements.

## Change an adapter or workload

Start with a failing public-seam test. Preserve shared geometry and documented
measurement boundaries. Test engine capability rejection, physical behavior, and
result validation independently. Add each supported workload to the appropriate
suite, browser catalog, and capability tests.

Run these checks from the repository root:

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo test --features viewer -p bevy-physics-bench-runner
npm ci
npm run coverage
npm run validate:results
npm run build
```

Run these checks after GPU changes:

```sh
cargo fmt --all --manifest-path gpu/Cargo.toml -- --check
cargo test --manifest-path gpu/Cargo.toml
cargo clippy --manifest-path gpu/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path gpu/Cargo.toml --test simulation -- --ignored --test-threads=1
```

The ignored tests require a real GPU. A CI job without an adapter cannot establish
GPU execution. Build WebAssembly and inspect desktop and mobile rendering after
browser changes. Test engine switching, cancellation, invalid settings, imports,
and an unavailable GPU adapter.

## Coverage and release checks

Install `cargo-llvm-cov` and the pinned toolchain's `llvm-tools` component.
The branch instrumentation currently needs `RUSTC_BOOTSTRAP=1` on this toolchain.
Use instrumentation only for validation; never publish instrumented timings.

```sh
RUSTC_BOOTSTRAP=1 cargo llvm-cov --workspace --all-features --branch --lcov --output-path rust-coverage.lcov
RUSTC_BOOTSTRAP=1 cargo llvm-cov --manifest-path gpu/Cargo.toml --workspace --all-features --branch --lcov --output-path gpu-coverage.lcov -- --include-ignored --test-threads=1
node scripts/check-releases.mjs
cargo run --example compare
cargo test --doc
cargo run --manifest-path gpu/Cargo.toml --example step
cargo test --manifest-path gpu/Cargo.toml --doc
```

The GPU example requires a working adapter. The personal lint workflow is also
required when available: `liamc-lints` runs the repository's Rust, Markdown, shell,
and schema checks without changing lint configuration.

The Monday collection workflow uploads CI runs and release reports for 90 days.
It does not publish them automatically. Inspect compatibility and adverse results
before updating lockfiles or accepting new records. GitHub Pages deploys from
`main` after verification and the WASM build pass.
