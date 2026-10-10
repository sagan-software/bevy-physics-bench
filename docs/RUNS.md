# Initial published collection

The 2026-10-10 UTC collection contains 342 measurements in 13 result files.
Every measurement retains 90 raw samples: three fresh worlds, 60 warmup steps
per world, 30 samples per world, and four simulation steps per sample.
The [result index](../web/public/results/index.json) lists every published artifact.

## Included runs

- The Ryzen 9 5950X host completed all 62 CPU combinations in baseline and tuned builds.
- Two GitHub runners each completed the same 62 combinations. Baseline reported
  an AMD EPYC 7763; tuned reported an Intel Xeon Platinum 8573C.
- The T3 Code browser completed all 62 CPU combinations and five CCD-enabled
  comparisons. Its Chromium user agent is recorded; CPU and RAM fields remain null.
- The RTX 5070 Ti completed all eight Nexus scenes twice and a 10,000-body box pile.
- Avian and Rapier each completed the native 10,000-body box pile.
- Five native comparisons enabled CCD in the fast-projectile scene.
- Three repeated native pyramid measurements provide history alongside the first run.

CPU and GitHub artifacts identify source revision
`f283ac2a289ceab679dcfefdf72208bac1c4106e`. Browser artifacts, the second Nexus suite,
and the large Nexus run identify `7ca79bec95c6c497c96a9e2efe16f5943ad9dd9d`.
The first Nexus suite uses the earlier revision. Both revisions are retained.

The local host ran unrelated work during collection. Project builds and benchmark
collectors did not run concurrently with these measurements. CPU affinity, power
settings, and thermals were uncontrolled. The browser document remained visible
but unfocused, with playground playback paused.

The [GitHub collection workflow](https://github.com/sagan-software/bevy-physics-bench/actions/runs/38022858348)
passed both jobs. Its matrix used different CPU models, so baseline versus tuned
results from those jobs do not isolate a build-profile effect.

## Adverse results

Fourteen measurements retain a dynamic-body centre below the floor. These are
Avian fast-projectile results: both dimensions in each native baseline, native
tuned, and GitHub suite; both browser dimensions with CCD disabled and enabled;
and both additional native CCD-enabled runs. Every recorded pose remained finite.
These health observations do not establish physical equivalence or identify a cause.

The 10,000-body Jolt collection failed with
`Jolt exhausted a contact buffer`. It produced no completed result file or timing
claim. Avian and Rapier were then collected separately; their large-scene artifacts
preserve that collection order. Jolt completed every standard 128-body workload.

The initial 10,000-body Nexus attempt failed because a 716,871,680-byte allocation
exceeded the requested 268,435,456-byte device limit. The capacity regression and
subsequent completed benchmark verify the fix. Failed attempts contributed no
samples to the published files.

The browser exposed WebGPU but returned no adapter. Native Nexus execution was
verified; browser Nexus execution remains unverified. No browser GPU timings are
published.

## Interpretation

Use the [measurement method](METHOD.md) and the per-record environment, settings,
fingerprint, build flags, and scope. GPU completion and readback have a different
measurement boundary from a CPU Bevy step. Browser Jolt uses its JavaScript binding
boundary. These scopes do not share speed ratios in the site.

Raw samples were preserved during publication. Publication changed only the
source label and appended collection conditions to native environment notes.
The schema validator recomputed statistics and checked all 13 files before publication.

## Rendered capacity collection

Eleven [capacity records](../web/data/capacity.json) were collected between
2026-10-10T07:35:33.434Z and 2026-10-10T07:45:21.471Z. The production bundle used
source revision `33a7573a3dab4b100939e77508737373401c8ab5`. Each engine ran alone
with a 60 FPS target, 3% tolerance, 128 initial bodies, and increments of 256.
Each level settled for 1000 ms and sampled for 3000 ms. Two missed windows stopped
growth; the recorded score is the last passing count.

- 2D boxes: Avian 3200; Rapier 3968 bodies.
- 2D balls: Avian 3456; Rapier 4480 bodies.
- 3D boxes: Avian 1408; Rapier 2176; Jolt 640 bodies.
- 3D balls: Avian 3456; Rapier 3200; Jolt 896 bodies.

The first ten runs used a 4096-body maximum. Rapier's first 2D ball run reached
that maximum while passing, so its 4096 score remains a lower bound. A separate
run raised the maximum to 8192 and reached the FPS threshold at 4480 bodies.
The records preserve both outcomes and every sampled window.

The T3 preview was visible throughout collection. Each simulation viewport was
947 by 390 CSS pixels with a reported pixel ratio of 1.4500000476837158.
The browser reported Linux x86_64, 32 logical cores, Chromium 152.0.7977.78,
and Electron 44.3.0. WebGL reported
`ANGLE (NVIDIA Corporation, NVIDIA GeForce RTX 5070 Ti/PCIe/SSE2, OpenGL ES 3.2)`.
The browser still provided no WebGPU adapter; Nexus has no browser capacity record.

Project builds were stopped during collection. Other local work, CPU affinity,
power settings, and thermals were not controlled. These are single runs per
configuration, with one higher-cap follow-up. Read the recorded settings and
[measurement method](METHOD.md) when comparing them.
