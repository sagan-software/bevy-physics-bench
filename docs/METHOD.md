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
