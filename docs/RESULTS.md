# Result contract

`results.schema.json` defines version 1 using
[JSON Schema 2020-12](https://json-schema.org/draft/2020-12/json-schema-core).
It is a project-specific benchmark format. Records use
[JSON](https://www.rfc-editor.org/rfc/rfc8259.html) and `application/json`.
Every object has a closed member set. Unknown schema versions, members, enums,
and missing required fields are rejected. Nullable fields must still be present.

## Time and lexical values

`created_at` accepts and emits exactly `YYYY-MM-DDTHH:mm:ss.sssZ`.
This local [RFC 3339](https://www.rfc-editor.org/rfc/rfc3339.html) profile requires
uppercase T and Z, UTC, and three fractional digits. Numeric offsets, omitted or
extra fractional digits, leap seconds, impossible dates, and lowercase forms are
rejected. Accepted timestamp strings are preserved without normalization.

Engine and scenario identifiers are lowercase closed vocabularies. Fingerprints
are exactly 64 lowercase hexadecimal digits. Revisions are a 40-digit lowercase
Git SHA, that SHA followed by `-dirty`, `unknown`, or `working-tree` locally.
Publication accepts only a clean full SHA. No URI or media-type fields are accepted.

JSON object order and whitespace have no meaning. Numeric spelling is not retained
when parsing and exporting. The JavaScript importer uses JSON.parse, which retains
the last occurrence of a duplicate object member; the schema validates that result.
Collectors emit each member once. Duplicate measurement identities and duplicate
manifest filenames are rejected. Rust settings parsing rejects duplicate fields.

## Record fields

The root requires `schema_version`, `created_at`, `source`, `environment`, `build`,
`versions`, and `measurements`. `schema_version` is the integer 1. `source` is
`local` or `published`.

`measurements` contains 1–500 records in collection order.
The schema defines every exact type, bound, required member, and nullable value.

`environment` requires runtime, machine label, CPU, logical cores, memory bytes,
OS, browser, target, profile, thread count, notes, and GPU metadata. Unknown CPU,
memory, browser, and GPU values use null. Profiles are `baseline`, `native-tuned`,
and `gpu`; runtimes are `native` and `wasm`. Native tuned results require native
execution. Machine labels reject control characters.

GPU metadata is null or an object with exactly `name`, `backend`, and `driver`.
Name and backend are nonempty strings. Driver is a nonempty string or null.
An adapter that withholds its name receives the explicit label `Undisclosed GPU`.
An empty reported driver becomes null. These placeholders make no hardware claim.

`build` contains compiler text, Rust flags, and a unique ordered feature array.
`versions` requires strings for Bevy, Avian, bevy_rapier, Rapier, Oxijolt, Jolt JS,
Nexus, and source revision. The version set describes the compiled compatibility
profile; each measurement identifies the engine it actually executes.

Each measurement requires engine, settings, plan, samples, statistics, awake-body
count, finite-state flag, minimum body height, query counts, fingerprint, scope,
and readback samples. `queries` has exactly integer `rays` and `hits` fields.
Minimum height is null when there is no finite dynamic-body height. GPU readback
is an array matching the sample count; CPU readback is null.

## Settings and sampling

Settings require scenario, dimension, count, seed, rate in Hz, gravity, friction,
restitution, solver effort, sleeping, and CCD. Body count is 1–32768 and seed is
0–4294967295. Rate is 30, 60, 120, or 240 Hz. Gravity is 0–50 m/s², friction is
0–2, restitution is 0–1, and effort is an integer from 1–32.

Dimensions are `2d` or `3d`. Sleeping is `disabled` or `enabled`. CCD is `discrete`
or `continuous`. The schema enumerates all 13 scenario identifiers. CLI settings
files may omit fields and receive documented Rust defaults; result files require
the complete settings object. Unknown fields and invalid values remain errors.

Rust stores gravity, friction, and restitution as `f32` and validates their bounds
after decimal input rounds to that type. The browser schema validates the entered
JavaScript number before Rust conversion. For example, Rust accepts gravity
`50.0000001` as `50`, while the browser rejects that entered value above `50`.
Focused tests cover this rounding boundary independently for all three fields.

Direct Rust settings serialization emits the shortest decimal for each `f32` value.
Native result objects serialize those values through `serde_json::Value`, which
exposes their `f64` expansion. Browser result objects retain the entered JavaScript
numbers. Thus default gravity appears as `9.810000419616699` in native records and
`9.81` in browser records. Both scene fingerprints use the validated Rust settings.

The plan requires 1–600 warmup steps, 5–1000 samples per repeat, 1–60 steps per
sample, and 1–9 repeats. Samples retain collection order across repeats.
Sample count must equal samples per repeat multiplied by repeats. Durations use
milliseconds per step. Statistics are recomputed within absolute tolerance
`max(1e-9, abs(expected) × 1e-7)` in the statistic's own unit.

## Scene identity and capability checks

Fingerprint profile 1 hashes the UTF-8 serde_json encoding of an object containing
`profile: 1`, complete validated `settings`, and the shared initial `scene`.
The pinned serializer orders object keys lexically. Arrays retain creation order.
The scene contains bodies and floor half extents. Each body has position, rotation,
shape, kind, linear velocity, angular velocity, and a nullable point constraint.

A constraint identifies its earlier parent index and both local anchor vectors.
Vectors use XYZ order and quaternions use XYZW order. The source revision and
lockfile fix generator and floating-number serialization behavior. This is a local
hash profile, not JSON canonicalization or a signature. It authenticates nothing.

Validation requires Bevy-only IK, 3D-only Jolt and Nexus, matching engine/runtime
scope, query counts, awake counts, and recomputed statistics. Nexus additionally
requires its eight supported scenes, disabled sleeping, discrete collision
detection, the GPU profile, adapter metadata, and bounded readback samples.
Readback may exceed total time by at most 0.01 ms for clock conversion precision.

## Publication and evolution

The manifest is an ordered array of at most 5000 unique filenames. Names match
`^[a-z0-9][a-z0-9.-]*\.json$` and cannot be `index.json`. Individual imports are
limited to 20,000,000 bytes. Invalid records display an error and are not retained.
There is no network upload endpoint or authentication protocol.

Browser storage retains up to 30 local runs. Published files are immutable inputs
reviewed through pull requests. Future incompatible formats require a new schema
version and explicit reader migration; version 1 has no open extension members.

## Rendered capacity records

[capacity.schema.json](../capacity.schema.json) defines the separate `capacity-v1`
format using JSON Schema draft 2020-12. The existing `schema_version: 1` step-timing
format is unchanged. The capacity validator also recomputes level order, retry
requirements, FPS, score, and outcome. It rejects additional object members,
missing required members, duplicate engines, and incompatible engine profiles.
Browser JSON parsing keeps the last value of duplicate textual object keys;
validation then checks that resulting object. Array order is preserved.

Every top-level member is required. `kind` is exactly `capacity-v1`. `created_at`
is a valid timestamp spelled `YYYY-MM-DDTHH:mm:ss.sssZ`, with exactly three
fractional digits and uppercase Z; numeric offsets and normalization are rejected.
`machine` is a nonblank string of 1 to 120 characters without control characters.

`browser` and `os` are strings up to 2000 characters. `logical_cores` is an integer
from 1 to 4096. `viewport` is exactly `[width, height]` in CSS pixels, each an
integer from 1 to 16384. `pixel_ratio` is greater than zero and at most two.

The browser collector trims the machine label; imports preserve accepted values.

`metadata` contains exactly `build`, `versions`, `target`, and `profile`.
Build and version members follow the bounded string fields in the schema.
Target is `wasm32-unknown-unknown`; profile is `baseline` or `gpu`.

`measurements` contains one or two engine records, in execution order.
Engine is `avian`, `rapier`, `jolt`, or `nexus`; Jolt and Nexus require 3D.

Each record requires `settings`, `plan`, `score`, `outcome`, `detail`, `levels`,
`scene_fingerprint`, and `gpu`. Settings retain the initial body count and the
existing physical-settings vocabulary. Capacity permits only boxes and balls.

Plan members are integers: `target_fps` is 1 to 240, `tolerance_percent` is 0 to 10,
`increment` is 1 to 4096, and `maximum` is the initial count through 32768.
`settle_ms` and `window_ms` are each 100 to 10000 milliseconds.

Each level requires
its integer body `count` and one or two `windows`. Counts begin at the initial
count and increase by the increment, clamped once at the maximum. A first failed
window requires exactly one retry. No level follows a threshold failure or the
maximum.

Each window requires positive `frames`, `elapsed_ms`, and `fps`.
Frames are integers through 1000000; elapsed time must cover the configured window.
FPS must agree with `frames * 1000 / elapsed_ms` within an absolute tolerance of
`1e-8` FPS. This tolerance permits floating-point serialization round trips.

`score` is an integer from 0 to 32768 and must equal the last passing count.
`outcome` is exactly `threshold`, `limit`, or `error`. Threshold requires two
consecutive missed windows. Limit requires a passing maximum.

Error cannot claim
a completed maximum; its score retains prior passing work for diagnostics, but
the UI shows no valid score. `detail` is null for threshold and limit, or a nonempty
error string up to 10000 characters. An error may have no complete levels.

`scene_fingerprint` is 64 lowercase hexadecimal characters identifying the
initial scene; the plan and recorded levels describe subsequent growth.

CPU records require `gpu: null`. Nexus records require a `gpu` object containing
exactly `name`, `backend`, and `driver`. Name and backend are strings up to 2000
characters; driver is a string or null. Nexus capacity records also require
60 Hz, solver effort 4, disabled sleeping, and discrete collision detection.

The browser retains up to 20 capacity records separately from step timings.
Storage is best effort; JSON export remains available if browser storage is full
or disabled. Import uses the same validator and a 20 MB file limit. Capacity
records appear locally on both the Playground and Results pages. They are not
included in the historical step-timing manifest or its CLI submission validator.
A future incompatible capacity profile requires a new `kind` value.
