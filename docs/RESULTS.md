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
`local` or `published`. `measurements` contains 1–500 records in collection order.
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
