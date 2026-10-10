# Engine and benchmark research

Checked on 2026-10-09. The starting scope came from the user's Claude conversation
`782eb8c9-6cf2-4753-81ff-dbf651f7d7d2` and its saved benchmark research.
The separate learned humanoid controller idea is outside this project's scope.

## Compatible releases

Bevy 0.20.0 is current. Avian 0.7.0 and bevy_rapier 0.36.0 require Bevy 0.19.
The requested combination of every latest release cannot compile together.
This project uses the newest mutually compatible published set, Bevy 0.19.1,
Avian 0.7.0, and bevy_rapier 0.36.0. Cargo.lock fixes transitive dependencies.

The Rapier plugin uses rapier 0.35.0-glamx0.2, not standalone Rapier 0.36.
The registry dependency records control this decision:
[Bevy](https://index.crates.io/be/vy/bevy),
[Avian](https://index.crates.io/av/ia/avian3d), and
[bevy_rapier](https://index.crates.io/be/vy/bevy_rapier3d).

Avian and Rapier provide CPU simulation in both dimensions and compile to
`wasm32-unknown-unknown`. Avian runs directly in Bevy's ECS. The Rapier plugin
synchronizes Bevy components with its backend, so that synchronization belongs
in a Bevy integration measurement.
Sources: [Avian](https://github.com/avianphysics/avian) and
[Rapier's Bevy integration](https://github.com/dimforge/rapier/tree/master/bindings/bevy_rapier).

## Jolt and GPU physics

Rolt 0.3.1 binds Jolt 5.0.0. Oxijolt 1.0.1 binds Jolt 5.6.0 and explicitly
has no WebAssembly target. Jolt's official browser binding is JoltPhysics.js.
Its Emscripten build is a separate adapter, so its integration cost must be
identified separately from a Rust Bevy plugin.
Sources: [Rolt](https://github.com/SecondHalfGames/jolt-rust),
[Oxijolt](https://github.com/pockerhead/oxijolt), and
[JoltPhysics.js](https://github.com/jrouwe/JoltPhysics.js).

Nexus is an experimental GPU engine. The current registry release is 0.6.0,
which supersedes the saved report's 0.5.0. Its build compiles Rust-GPU shaders;
the documented prerequisite is cargo-gpu 0.10.0 and its shader toolchain.
Browser execution requires WebGPU. Nexus documents Safari as unsupported.
GPU execution and readback need separate measurements because gameplay may
need CPU-visible poses each frame.

Sources: [Nexus](https://github.com/dimforge/nexus),
[registry](https://index.crates.io/ne/xu/nexus_rbd3d), and
[shader build](https://github.com/dimforge/nexus/blob/master/crates/nexus_rbd3d/build.rs).

## Scenarios and measurement

Box2D's benchmark suite establishes useful workloads: pyramids, piles, static
fields, kinematic obstacles, joint grids, queries, and sleeping bodies.
Jolt adds passive ragdolls and mesh collisions. This project implements seeded
scenes in metres with the same body descriptions for each compatible adapter.
The scenes are custom workloads, not claims to reproduce upstream scores.
Sources: [Box2D benchmarks](https://github.com/erincatto/box2d/tree/main/benchmark),
[Box2D scenes](https://github.com/erincatto/box2d/blob/main/shared/benchmarks.c),
and [Jolt performance tests](https://github.com/jrouwe/JoltPhysics/blob/master/Docs/PerformanceTest.md).

Creation, warmup, pose serialization, and rendering are outside timed samples.
Each sample times a fixed batch of steps. Repeated runs reconstruct the scene.
Publish raw samples, median, p95, minimum, maximum, mean, and dispersion.
Report physical validity and awake bodies alongside timing.

A faster unstable
simulation is not a better result.
Sources: [Box2D methodology](https://github.com/erincatto/box2d/blob/main/benchmark/performance_v320.md)
and [High Resolution Time](https://www.w3.org/TR/hr-time-3/).

The shared profile fixes timestep, geometry, initial velocity, friction,
restitution, gravity, sleeping, and CCD. Solver effort remains engine-specific:
Avian substeps and Rapier solver iterations do not represent equal work.
The UI and result files must retain that distinction.
Sources: [Avian scheduling](https://docs.rs/avian3d/0.7.0/avian3d/schedule/index.html)
and [Rapier integration parameters](https://docs.rs/rapier3d/0.35.0-glamx0.2/rapier3d/dynamics/struct.IntegrationParameters.html).

Character controllers have different algorithms and are a separate workload.
IK is an animation operation, not a physics solver comparison; an IK result
belongs to a Bevy-only series. Passive ragdolls avoid assuming motor equivalence.

## Hosting and results

GitHub Pages hosts static files and WebAssembly. The baseline uses one browser
thread and avoids requiring cross-origin isolation or a service worker.
Users can run locally, export results, and submit result files through PRs.
Machine and runtime identifiers partition result series. Hosted-runner timing
is labeled as CI data because shared runner hardware does not provide a stable
reference machine. Historical records remain immutable.

Sources: [GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/about-github-pages),
[Bevy Twitcher](https://github.com/bevyengine/twitcher), and
[rustc-perf collection](https://github.com/rust-lang/rustc-perf/tree/master/collector).

Release monitoring checks published dependency versions. A version change
requests validation before it can replace the known compatible set.
Public PR validation must not execute submission-supplied code on a dedicated
machine. Sources: [crates.io data access](https://crates.io/data-access) and
[GitHub Actions security](https://docs.github.com/en/actions/reference/security/secure-use).

## Implemented profiles

The project uses the official `jolt-physics` 1.1.0 browser package. Its underlying
Jolt core version is not inferred from the npm version. Oxijolt 1.0.1 lacks public
solver-iteration setters, so effort selects full substeps while retaining Jolt's
ten velocity passes and two position passes. Native Jolt uses an inline job runner.
Sources: [published Oxijolt API](https://docs.rs/oxijolt/1.0.1/oxijolt/) and
[browser binding](https://github.com/jrouwe/JoltPhysics.js).

Nexus uses Khal 0.4.0 and Vortx 0.5.0. cargo-gpu selects the 2026-07-03 shader
nightly for the published shader crate. The native runner completed all eight
supported scenes on an RTX 5070 Ti. Browser WASM includes the GPU adapter, but the
test browser returned no WebGPU adapter. Browser GPU execution remains unverified.
Sources: [Khal 0.4.0](https://docs.rs/khal/0.4.0/khal/) and
[Nexus 0.6.0](https://docs.rs/nexus_rbd3d/0.6.0/nexus_rbd3d/).

The native viewer uses minifb 0.28.0 to present CPU-rendered primitive wireframes.
The browser uses Three.js for rendering and receives poses from the same seeded
scene model. Rendering stays outside measured work.
Source: [minifb 0.28.0](https://docs.rs/minifb/0.28.0/minifb/).
