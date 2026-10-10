# Measurement method

Each engine receives the same generated bodies, geometry, velocities, materials,
gravity, seed, and fixed timestep. The scene fingerprint identifies those inputs.
The floor is an additional static collider and is excluded from the body count.
Lengths use metres, velocities use metres per second, and gravity uses metres per
second squared. Quaternions use XYZW order; 3D coordinates are right-handed with Y up.

## Workloads and fairness

Pyramids are triangular walls of boxes in both dimensions. Partial chains,
ragdolls, and IK limbs retain the requested exact body count. Static-field counts
include static obstacles. Character workloads mix static ramps and kinematic
balls driven through each engine's character controller.

A seeded linear congruential generator adds bounded positional jitter. Shared
primitive density is one; linear and angular damping are zero. Point constraints
disable contacts only between directly connected bodies. Ragdolls are passive
point-constrained assemblies, not anatomical models or equivalent motor systems.

Avian solver effort selects substeps. Rapier and Nexus use solver iterations.
Jolt uses full substeps with ten velocity passes and two position passes each.
Equal effort values do different work. Enabled sleeping uses engine thresholds;
CCD also uses engine-specific algorithms. Compare physical behavior with timing.

Ray-query scenes cast one downward ray per body after each step. Alternate rays
start beyond the scene to exercise misses. Character controllers and analytic
two-bone IK are separate workloads. IK has no physics-engine ranking.

## Timing boundary

Scene construction and warmup occur before timed samples. Each repeat constructs
a new world. Engine order reverses on alternate repeats. A sample contains a fixed
batch of steps; elapsed milliseconds divided by batch steps gives milliseconds
per step. Rendering, JSON serialization, health inspection, and yielding between
samples occur outside timing.

`bevy-step` includes the Bevy schedule, physics, controllers or queries, and pose
synchronization for Avian, Rapier, and native Jolt. `binding-step` measures browser
Jolt's JavaScript binding and synchronization. `bevy-animation` measures analytic
IK within Bevy. Ratios require matching scopes, settings, plans, and fingerprints.

`gpu-step-readback` includes Nexus submission, completion, pose readback, and a
collision-pair counter readback. The separately reported readback interval includes
the wait for GPU completion. It is a subset of total time, not a second charge.
It cannot identify GPU kernel time independently. GPU and CPU scopes remain separate.

Nexus uses one batch, one floor plus the requested bodies, a fixed collision-pair
capacity of 32 times that total, 256 multibody contact constraints, and 64 solver
colors. Pair-buffer overflow and nonfinite poses fail the step. Other device or
shader failures propagate as errors. These limits are an experimental local profile.

The device requests enough buffer and storage-binding capacity for the largest
Nexus 0.6.0 primitive-scene allocation. Its size is `(bodies + 1) × 32 × 2 × 112`
bytes: one floor, 32 pairs per body, two contacts per pair, and 112 bytes per record.
For 10,000 bodies, this is 716,871,680 bytes for one buffer; total allocation is larger.
The adapter must support both requested limits before scene allocation begins.
This check does not guarantee enough free device memory for every buffer.
The controlling sources are [Nexus allocations](https://docs.rs/nexus_rbd3d/0.6.0/src/nexus_rbd3d/pipeline/rbd_state_from_rapier.rs.html)
and [wgpu 30.0.1 limits](https://docs.rs/wgpu/30.0.1/wgpu/struct.Limits.html).

## Statistics and validity

The median sorts samples and averages the middle pair for an even count. P95 is
the nearest-rank sample at `ceil(0.95 × count)`. Standard deviation uses population
variance. The coefficient of variation is `100 × deviation / mean`, or zero when
all samples are zero. All duration operands use milliseconds per step.

Raw samples are authoritative. Validation recomputes every statistic and checks
sample counts against the plan. Zero samples can occur with coarse browser timers;
they are retained rather than replaced with invented precision.

Every repeat checks final poses for finite components. Records retain the lowest
finite dynamic-body centre across repeats, the last awake-body count, and query
counts. These are limited health checks, not a proof of physical equivalence.
The UI flags nonfinite poses and centres below the floor and suppresses misleading
speed ratios for invalid results.

## Collection conditions

Browser results depend on browser version, timer precision, background activity,
and thermal state. Keep the tab visible. The browser does not disclose a reliable
CPU model, so unknown hardware fields remain null. Give each machine a stable label.
The default browser label uses a locally stored random identifier.

Initial native records identify the AMD Ryzen 9 5950X collection host. GPU records
also identify its NVIDIA GeForce RTX 5070 Ti and driver. The host runs other work;
these observations are not controlled laboratory rankings. Collection excludes
this project's concurrent builds, but unrelated host activity remains uncontrolled.

The baseline, tuned native build, and GPU profile form separate series. Browser
and native records remain separate. History fixes machine facts, compiler,
build flags, enabled features, settings, plan, fingerprint, scope, and engine while
allowing dependency versions and source revisions to change. Different environments
never form a single regression line.

## Interactive playback and capacity

A click adds a world-space velocity change to one dynamic body. It preserves the
body's prior velocity and wakes sleeping bodies. Paired views use the same stable
body index and velocity change even after their poses diverge.

In 3D, the kick adds
12 m/s along the camera's horizontal viewing direction and 6 m/s upward. In 2D,
it adds either -12 or 12 m/s horizontally and 6 m/s upward, with zero Z velocity.
When a 3D camera ray is vertical, the horizontal kick is zero.

A gesture that ever moves more than five CSS pixels from its starting point does
not kick, even if it returns. Physics interactions are disabled
during measured runs. A kick from a paused view advances one shared step.

The ten-second area chart records completed visible updates. FPS is
`1000 / interval_ms`, where the interval is the elapsed time between completed
updates and 1000 converts milliseconds to seconds. Physics time divides elapsed
step time in milliseconds by the number of fixed steps. Paired views share FPS;
each retains its own physics time. Manual steps and pauses do not contribute FPS.

The chart retains at most 2400 points and redraws on animation frames. Its numeric
readout averages points from the latest 500 milliseconds.

Capacity runs own one simulation at a time and free it before measuring the next
engine. Each frame performs one fixed physics step, snapshot transfer, and render
submission. The measured update rate includes browser scheduling and those
operations. It approximates rendered throughput; it does not measure physical
display presentation or isolated GPU kernel execution. The browser schedules
animation frames near display refresh and can suspend hidden pages.
Sources: [requestAnimationFrame](https://developer.mozilla.org/en-US/docs/Web/API/Window/requestAnimationFrame)
and [performance.now](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now).

Each load has a settling interval followed by a measurement window. A window's
FPS is `frames * 1000 / elapsed_ms`: `frames` counts completed updates,
`elapsed_ms` measures the whole window in milliseconds, and 1000 converts units.
Allocation, insertion, and settling are excluded from that window.

A window
passes when FPS is at least `target_fps * (1 - tolerance_percent / 100)`.
The default 60 FPS target and 3% tolerance require 58.2 FPS.

A missed window gets
one retry at the same load. Two misses stop the run. The score is the last passing
body count, or zero if the initial load fails. A maximum-count outcome is a lower
bound, not a measured failure threshold. Higher targets can exceed display refresh.

Growth retains existing poses, velocities, and ticks. Added boxes or balls use
unit-density colliders and a deterministic eight-column lattice, independent of
current engine poses. For stable body index `i`, depth `d` is 1 in 2D and 8 in 3D.
Coordinates in metres are `x = (i % 8) * 1.1 - 3.85`,
`y = 10 + floor(i / (8 * d)) * 1.1`, and
`z = (floor(i / 8) % d) * 1.1 - (d - 1) * 0.55`.

The scene's original floor remains in place. The total cannot exceed 32768 bodies.
Different frame rates also advance different amounts of simulation time per
wall-clock second because capacity uses one step per update.

Nexus 0.6.0's incremental buffers use `RbdState::empty` and `append_bodies`.
The empty constructor fixes 60 Hz and four solver iterations; this growth profile
rejects other settings. The fixed-scene adapter keeps its configurable solver
profile. GPU kicks read velocities, preserve angular velocity, and write the
selected linear velocity between steps. These transfers occur outside measurement.
Sources: [Nexus state](https://docs.rs/nexus_rbd3d/0.6.0/nexus_rbd3d/pipeline/struct.RbdState.html)
and [incremental insertion](https://docs.rs/nexus_rbd3d/0.6.0/src/nexus_rbd3d/pipeline/insertion_removal.rs.html).

Nonfinite poses, incorrect body counts, dynamic centres below -0.25 m, backend
failures, or frame waits longer than two seconds invalidate capacity results.
An interrupted retry does not retain its incomplete level. Cancellation, leaving
the playground, hiding the page, or resizing it prevents saving a partial run.
The hidden-preview test produced an explicit frame-stall error. No rendered FPS
capacity score from that preview is published.
