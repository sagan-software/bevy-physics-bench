# Validation evidence

The public Rust tests cover the 62 CPU engine, scene, and dimension combinations,
shared initial-state identity, engine capability rejection, fixed-step behavior,
queries, character controllers, passive constraints, IK, sleeping, validated
settings, statistics, and native result output. The implementation began with
failing public API tests and added private invariant tests after the external
seams passed.

The native Nexus tests execute all eight supported scenes on an RTX 5070 Ti.
Separate tests cover unsupported profiles, buffer overflow, nonfinite poses,
and withheld adapter metadata. GPU execution tests are ignored in ordinary CI
because they require an actual adapter; collection-host runs include them.
The 10,000-body GPU regression first failed with a 256 MB device-buffer limit,
then passed after the device requested the scene's required limits. Private tests
accept exactly 716,871,680 bytes and reject either relevant limit one byte below it.

The browser exercised all 62 supported CPU combinations with finite poses and
matching body counts. Jolt binding tests use the actual WASM package. Browser
WebGPU execution remains unverified because the preview browser returns no adapter.
The UI reports that capability failure explicitly.

## Required checks

The commands in CONTRIBUTING.md are the reproducible gate set. Local verification
also runs the applicable `liamc-lints` workflow. Stable rustfmt reports ignored
nightly options from this host's global configuration; those are environmental
warnings, not project configuration. Strict Clippy and personal lint diagnostics
from project code must be clean.

Frontend coverage enforces 100% statements, branches, functions, and lines for
benchmark orchestration, record validation, history grouping, machine identity,
capacity orchestration, capacity record validation, and live telemetry.
The DOM controller, Three.js rendering, and external physics bindings are outside
that unit coverage scope. Their browser checks do not establish full line coverage.

Rust branch coverage uses cargo-llvm-cov with the exact commands in CONTRIBUTING.md.
The CPU report covers both dimensions of every adapter and all settings boundaries.
It does not establish 100% whole-project coverage. Remaining gaps include native
window input, display errors, impossible private Bevy resource loss, upstream Jolt
failure callbacks, optimized accessor instantiations, and the WASM-only exports.
The native viewer's exported PPM exercises its renderer without a window.

The current CPU report has 2,284 of 2,416 source lines and 249 of 288 branches hit.
The GPU report has 540 of 739 source lines and 36 of 50 branches hit.
Unlike the earlier report, this GPU report runs the prescribed test command
without additional instrumented CLI collections. The unchanged GPU CLI therefore
has fewer recorded hits. These totals
include inline unit tests and exclude build scripts and WASM-only exports.

[The coverage inventory](coverage.json) lists exact remaining lines, branches,
and reasons. The GPU CLI still lacks focused evidence for invalid machine labels,
settings-file input, and a device change during collection.

Desktop and 390-pixel mobile browser checks exercised paired playback, engine
switching, settings, comparison, and results. A separate finish reviewer confirmed
the mobile control placement, complete engine metadata, and source-link fixes.
The design detector ran once; its flat-heading report was checked against the
rendered hierarchy. Design tokens and the accepted layout are recorded in DESIGN.md.
The 13 published records also loaded in desktop and mobile Results views. Filters
selected the two native pyramid records, and history displayed one series per engine.

Precision tests cover each physical coefficient's `f32` rounding, direct settings
serialization, result-value expansion, round trip, and near-limit validation order.
All 342 published measurements agree on scene fingerprints after that conversion:
29 configurations are represented, including 28 in both native and browser records.

Coverage is execution evidence. Physical assertions, identical generated inputs,
raw timing samples, and result validation establish separate properties. Release
measurements must come from uninstrumented binaries with a clean source revision.

## Interaction and capacity revision

Failing public-seam tests preceded the new kick, growth, and GPU APIs. CPU tests
exercise kicks in both dimensions, malformed values, static and absent targets,
2D Z rejection, IK rejection, sleeping-body activation, capacity bounds,
unsupported scenes, deterministic planar growth, and pose/tick retention.
Nexus hardware tests cover boxes and balls, kicks on appended bodies, static and
absent targets, retained poses, capacity rejection, and unsupported growth profiles.
All eight GPU integration tests pass on the RTX 5070 Ti, including 10000 bodies.

The 63 frontend tests pass. Seven measured modules have 100% coverage:
310 statements, 260 branches, 46 functions, and 215 lines. Cases include transient
FPS dips, two-window failure, capped growth, aborted in-flight work, invalid poses,
wrong body counts, frame stalls, interrupted retries, strict records, and bounded
telemetry. Test clocks and worlds are synthetic; they are not benchmark results.

Every changed native Nexus implementation line is hit. WASM-only exports remain
outside native coverage. Changed CPU production lines
without hits are `src/adapters/interaction.rs:42,66-68` and
`src/adapters/jolt.rs:67`. These dispatch defenses require unsupported private
adapter states or a Jolt character that public target validation already rejects.
Unhit changed branches are the secondary Bevy predicate at interaction.rs:79 and
the Character-handle alternative at jolt.rs:61.

The coverage inventory retains
these gaps rather than claiming whole-project coverage.

The real browser bindings for Avian, Rapier, and Jolt move a body approximately
one metre after a 6 m/s kick and ten 60 Hz steps. Newly appended bodies also
move approximately one metre under the same kick. Adding three bodies preserves
the original pose and tick. A canvas click reports the same body kick in both
engines. Single-engine selection, switching, chart metric selection, invalid
capacity import, navigation cancellation, and the frame-stall error were exercised.
An isolated synthetic chart test rendered filled pixels and changed its numeric
readout to 7.00 ms; that temporary widget was removed.

At a device pixel ratio of 1.45, repeated draws retain the canvas backing store.
A width change resizes it once; subsequent draws retain the new dimensions.

The T3 preview initially stalled while hidden. Those runs produced errors and
were discarded. After the preview became visible, Avian, Rapier, and Jolt
completed threshold runs with continuously updating charts. These functional
checks ran during local builds and are not published comparison scores.
The preview still returns no WebGPU adapter, so Nexus browser execution remains
unverified. The 342 historical step-timing measurements remain unchanged.

Desktop and 390-pixel mobile captures cover the controls, charts, saved error,
and capacity settings. The finish reviewer found overlapping support text and
confirmed the spacing fix resolved. The documenter confirmed that the extension
uses the existing design tokens. The detector ran once; pre-existing palette/type
advisories and its static-heading false positive were recorded without redesign.

A browser regression test first showed one unwanted kick after dragging 30 pixels
away and returning to the starting point. The corrected pointer handling records
whether the gesture ever exceeds five pixels. Browser event checks pass for a
click, a five-pixel gesture, a longer drag, a drag that returns, and cancellation.
These DOM/WebGL checks remain outside the measured unit-coverage modules.

Jolt binding tests run the actual WASM module. They reject invalid capacity,
non-dynamic kicks, invalid velocity components, unsupported growth, and calls after
release. Valid insertion preserves ticks and registers the new body with physics.

A visible growth run exposed bodies leaving the original 20 m floor half-width.
The corrected growth profile reserves 1000 m horizontal half extents before
constructing any engine. Native Avian, Rapier, Jolt, and Nexus regression tests
kick a body beyond x = 20 m and verify that it remains above the floor.
Public geometry tests reject insufficient capacity and preserve fixed-scene
fingerprints. Browser Avian, Rapier, and Jolt report identical growth fingerprints.

All added native floor and fingerprint branches have coverage hits. Browser
exports and the published-capacity DOM section remain outside native and unit
coverage; browser checks cover the actual WASM floor and engine construction.
The frontend suite validates every bundled published capacity record.
