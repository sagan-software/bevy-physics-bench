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
