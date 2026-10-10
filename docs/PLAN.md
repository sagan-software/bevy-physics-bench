# Implementation and acceptance

## Delivery

1. Research engine versions, browser support, benchmark methodology, and existing suites.
2. Implement a typed scene model and native/WebAssembly Bevy adapters.
3. Add configurable scenes, live playback, engine switching, and isolated comparisons.
4. Collect native and browser results with raw samples and machine provenance.
5. Add result validation, release checks, CI, and GitHub Pages deployment.
6. Verify the published site at desktop and mobile widths.

## Invariants

- Engine, dimension, scenario, and measurement profile use closed vocabularies.
- Counts, timestep, gravity, material settings, and solver effort validate at ingress.
- A scene's seed and settings derive its initial bodies and joint definitions.
- Each simulation owns one engine world. Switching reconstructs the same initial scene.
- A paused simulation cannot advance except through an explicit step.
- A timed run excludes scene construction, rendering, and snapshot serialization.
- Comparable runs share scene, dimension, settings, seed, runtime, and machine.
- Samples contain finite nonnegative durations in milliseconds per simulation step.
- Errors distinguish invalid settings, unsupported capabilities, and numerical failure.
- Scene storage and snapshots use O(body count) space. Stepping retains engine state.

## Validation gates

- Begin with a failing public integration test and runnable example.
- Establish module files, then public API skeletons, before implementation.
- Test every scene and engine/dimension combination, invalid settings, and simulation effects.
- Add private invariant tests after the public test passes.
- Run `cargo fmt --all -- --check`.
- Run `cargo test`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run the applicable personal lint workflow without changing its policy.
- Run coverage and document any uncovered production branch.
- Build release WebAssembly and the native runner.
- Test sample statistics, result schema validation, comparison isolation, cancellation, and engine switching.
- Inspect desktop and mobile views; run the design detector and finish review.
- Run examples and documentation tests after the final prose-only documentation edit.
- Verify CI and the deployed GitHub Pages site, including actual WebAssembly execution.

## Instruction audit

The initial tracked repository contains no local AGENTS.md or AI reviewer instructions.
The user's supplied AGENTS.md applies to every changed file.
The configured Git identity is Bill Curry, bill@sagan.software.
Preserve the starter icon and unrelated workspace files.
