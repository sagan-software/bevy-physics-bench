# Bevy physics bench

Run configurable physics scenes in the browser, switch engines, and collect timings
from fresh worlds on your own machine.

[Open the playground](https://sagan-software.github.io/bevy-physics-bench/) ·
[Published results](https://sagan-software.github.io/bevy-physics-bench/#results) ·
[Research](docs/RESEARCH.md) · [Measurement method](docs/METHOD.md)

Avian and Rapier support 2D and 3D. Jolt supports 3D through Oxijolt natively and
JoltPhysics.js in the browser. Experimental Nexus supports eight 3D GPU workloads.
Two-bone IK uses a separate Bevy animation series.

The 13 scenes include box and ball piles, pyramids, tall stacks, dominoes, static
fields, moving platforms, fast projectiles, ray queries, character controllers,
passive joint chains, ragdolls, and inverse kinematics. Engine dropdowns reset both
worlds to the same seed. Settings control body count, timestep, materials, gravity,
sleeping, collision detection, solver effort, and sampling.

## Interact and find capacity

Click a dynamic body to kick the same body in both engines. Drag to move the
shared camera. Enter or Space kicks the body at the centre of a focused canvas.
The View selector switches between side-by-side and one-engine playback.
Changing an engine recreates the configured scene; saved results remain available.

Each view has a moving ten-second area chart with FPS and physics-time metrics.
Paired views share an update rate because both engines run in the same loop.
Choose one engine to inspect its update rate in isolation.

Choose Grow to FPS limit for box or ball workloads. Set the target FPS,
tolerance, batch size, maximum body count, settling time, and measurement window.
The runner adds bodies to the existing world and tests each engine separately.
Two consecutive missed windows stop growth. The score is the last passing count;
reaching the maximum reports a lower bound. Errors have no valid score.

Capacity runs include rendering and retain settings, raw windows, browser,
viewport, versions, and machine metadata. Results appear beneath the views and
on the Results page. JSON export and import preserve individual runs.
Keep the page visible and its size fixed during collection. Display refresh rate
limits the measured update rate. The hidden T3 preview could verify behavior but
could not supply a valid published rendered-capacity score.

Nexus growth requires WebGPU, 3D boxes or balls, 60 Hz, solver effort 4,
disabled sleeping, and discrete collision detection. Its current incremental
constructor fixes the timestep and iteration count. Device memory and buffer
limits can require a smaller maximum. Jolt uses its official JavaScript binding.

## Run native benchmarks

Install Rust through rustup. The repository pins Rust 1.98.1 and its components.
Native Jolt also needs a C++ compiler and CMake.

```sh
cargo run --locked --release --bin bench -- bench --machine my-desktop --output run.json
cargo run --locked --release --bin bench -- suite --machine my-desktop --output suite.json
cargo run --locked --release --features native-tuned --bin bench -- suite --machine my-desktop --output tuned.json
```

`bench` compares compatible engines for one scene. `suite` collects 62 supported
engine, scene, and dimension combinations. Use `--engine rapier`, `--count 1024`,
`--scenario balls`, or `--dimension 2d` to select work. `--help` lists every option.
`--settings settings.json` accepts the validated shared settings described in
[the result contract](docs/RESULTS.md).

The file supplies physical settings and body count. The command's `--scenario`
and `--dimension` select the measured row, even when a settings file is supplied.
`suite` always selects its full scene and dimension matrix.

The baseline disables the optional parallel and SIMD features. `native-tuned`
enables Avian parallel/SIMD and Rapier parallel/SIMD8. Jolt uses an inline job
runner in both profiles. These profiles are stored separately in results.

## Render natively

The optional native viewer renders the same CPU adapter snapshots as wireframes.
On Ubuntu, install `libx11-dev` and `libxkbcommon-dev` before building it.

```sh
cargo run --locked --release --features viewer -p bevy-physics-bench-runner -- view
cargo run --features viewer -p bevy-physics-bench-runner -- view --frame scene.ppm
```

Space pauses, N advances one step, R resets, and keys 1–4 select Avian, Rapier,
Jolt, or Bevy IK when the scene supports them. Frame export works without a display.
The browser provides the full settings interface and paired rendering.

## Build the browser site

Install Node.js 24, Rust through rustup, a C++ build toolchain, and CMake.
The shader setup installs cargo-gpu 0.10.0 and its pinned Rust-GPU nightly.
It downloads a separate shader toolchain and needs several gigabytes of build space.

```sh
npm ci
cargo install wasm-bindgen-cli --version 0.2.129 --locked
./scripts/setup-gpu.sh
./scripts/build-web.sh
npm run dev
```

Open `http://localhost:5173/bevy-physics-bench/`. The production site is in `dist/`.
The WASM module contains the CPU integrations and Nexus shaders. Its current raw
size is about 38 MB, so the initial load can take time. Browser Jolt loads its
separate binding on demand. No cross-origin isolation or threaded WASM is required.

Nexus needs an available WebGPU adapter. The dropdown reports an unavailable
adapter instead of substituting CPU results. Its browser path is implemented;
this project's initial test browser could not obtain a WebGPU adapter.

## Run GPU benchmarks

Run the shader setup above before building the separate GPU workspace.

```sh
cargo run --locked --release --manifest-path gpu/Cargo.toml --bin gpu-bench -- suite --machine my-gpu --output gpu.json
cargo run --locked --release --manifest-path gpu/Cargo.toml --bin gpu-bench -- bench --scenario boxes --count 10000 --output gpu-large.json
```

Nexus uses 3D, disabled sleeping, and discrete collision detection. Its supported
scenes are boxes, balls, pyramids, stacks, static fields, dominoes, chains, and
ragdolls. Timings include GPU completion and pose readback. Readback samples are
also stored as a subset of total step time. Native GPU execution was verified on
an NVIDIA GeForce RTX 5070 Ti through Vulkan.

The adapter must support the scene's buffer sizes. For 10,000 bodies, the largest
individual buffer needs 716,871,680 bytes; other buffers add to total memory use.
Unsupported buffer limits return an error before scene allocation.

## Results and maintenance

The Results view filters records by runtime, machine, profile, scene, dimension,
and revision. It retains raw samples and shows history when comparable records
exist. Browser measurements remain in local storage until exported.

[Contributing](CONTRIBUTING.md) explains publication and validation.
[The initial collection](docs/RUNS.md) records 342 measurements and observed failures.
[Validation](docs/VALIDATION.md) records checks and remaining coverage boundaries.
The weekly collection workflow produces baseline and tuned CI artifacts plus a
release report. Maintainers review those artifacts before publishing them.

The project uses Bevy 0.19.1, Avian 0.7.0, bevy_rapier 0.36.0, Oxijolt 1.0.1,
jolt-physics 1.1.0, and Nexus 0.6.0. Bevy 0.20 cannot yet compile with this published
integration set. Dependency upgrades require compatibility and behavior checks.

## License

Project code is available under MIT or Apache-2.0, at your option. Dependencies
retain their own licenses.
