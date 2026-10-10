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
benchmark orchestration, record validation, history grouping, and machine identity.
The DOM controller, Three.js rendering, and external physics bindings are outside
that unit coverage scope. Their browser checks do not establish full line coverage.

Rust branch coverage uses cargo-llvm-cov with the exact commands in CONTRIBUTING.md.
The CPU report covers both dimensions of every adapter and all settings boundaries.
It does not establish 100% whole-project coverage. Remaining gaps include native
window input, display errors, impossible private Bevy resource loss, upstream Jolt
failure callbacks, optimized accessor instantiations, and the WASM-only exports.
The native viewer's exported PPM exercises its renderer without a window.

The recorded CPU report has 2,001 of 2,128 source lines and 221 of 258 branches hit.
The GPU report has 558 of 567 source lines and 29 of 34 branches hit. These totals
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

Coverage is execution evidence. Physical assertions, identical generated inputs,
raw timing samples, and result validation establish separate properties. Release
measurements must come from uninstrumented binaries with a clean source revision.
