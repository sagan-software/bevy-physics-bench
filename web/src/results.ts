import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../../results.schema.json' with { type: 'json' };
import type { Engine, Settings, Snapshot } from './types';

export interface Statistics {
  sample_count: number; mean_ms: number; median_ms: number; p95_ms: number;
  min_ms: number; max_ms: number; stddev_ms: number; cv_percent: number;
}
export interface Plan { warmup_steps: number; samples: number; batch_steps: number; repeats: number }
export interface Measurement {
  engine: Engine; settings: Settings; plan: Plan; samples_ms: number[];
  statistics: Statistics; awake_bodies: number; finite: boolean; min_y: number | null;
  queries: { rays: number; hits: number }; scene_fingerprint: string;
  scope: 'bevy-step' | 'binding-step' | 'gpu-step-readback' | 'bevy-animation';
  readback_ms: number[] | null;
}
export interface RunFile {
  schema_version: 1; created_at: string; source: 'local' | 'published';
  environment: { runtime: 'wasm' | 'native'; machine: string; cpu: string | null;
    logical_cores: number; memory_bytes: number | null; os: string; browser: string | null;
    target: string; profile: 'baseline' | 'native-tuned' | 'gpu'; threads: number; notes: string;
    gpu: { name: string; backend: string; driver: string | null } | null };
  build: { compiler: string; rustflags: string; features: string[] };
  versions: { bevy: string; avian: string; bevy_rapier: string; rapier: string; oxijolt: string; jolt_js: string; nexus: string; revision: string };
  measurements: Measurement[];
}

/** Summarize milliseconds per step using the same definitions as the Rust runner. */
export function summarize(samples: readonly number[]): Statistics {
  if (samples.length === 0 || samples.some(sample => !Number.isFinite(sample) || sample < 0)) {
    throw new Error('Timing samples must be finite, nonnegative, and nonempty.');
  }
  const ordered = [...samples].sort((a, b) => a - b);
  const count = ordered.length;
  const mean = ordered.reduce((sum, value) => sum + value, 0) / count;
  const midpoint = Math.floor(count / 2);
  const median = count % 2 ? ordered[midpoint] : (ordered[midpoint - 1] + ordered[midpoint]) / 2;
  const deviation = Math.sqrt(ordered.reduce((sum, value) => sum + (value - mean) ** 2, 0) / count);
  return { sample_count: count, mean_ms: mean, median_ms: median,
    p95_ms: ordered[Math.ceil(count * .95) - 1], min_ms: ordered[0], max_ms: ordered[count - 1],
    stddev_ms: deviation, cv_percent: mean === 0 ? 0 : deviation / mean * 100 };
}

/** Inspect actual poses after timing has stopped. Ground penetration remains visible in the result. */
export function health(snapshot: Snapshot): Pick<Measurement, 'awake_bodies' | 'finite' | 'min_y' | 'queries'> {
  const dynamic = snapshot.bodies.filter(body => body.kind === 'dynamic');
  const finite = snapshot.bodies.every(body => [...body.position, ...body.rotation].every(Number.isFinite));
  return { queries: snapshot.queries ?? { rays: 0, hits: 0 }, awake_bodies: dynamic.filter(body => !body.sleeping).length, finite,
    min_y: finite && dynamic.length ? Math.min(...dynamic.map(body => body.position[1])) : null };
}

/** Compare known fields, independent of JSON object insertion order. */
function sameFields(first: object, second: object): boolean {
  return Object.entries(first).every(([key, value]) => value === (second as Record<string, unknown>)[key]);
}

/** Ratios require equal physical inputs, sampling plans, and measurement boundaries. */
export function comparable(first: Measurement, second: Measurement): boolean {
  return sameFields(first.settings, second.settings) && sameFields(first.plan, second.plan)
    && first.scene_fingerprint === second.scene_fingerprint && first.scope === second.scope
    && first.finite && second.finite;
}

const ajv = new Ajv2020({ allErrors: false, strict: true });
const validateRecord = ajv.compile<RunFile>(schema);
const checkSettings = ajv.getSchema<Settings>(`${schema.$id}#/$defs/settings`)!;

/** Validate JSON settings without changing their spelling, values, or field set. */
export function validateSettings(value: unknown): Settings {
  if (!checkSettings(value)) throw new Error(`Invalid settings: ${checkSettings.errors?.[0]?.instancePath} ${checkSettings.errors?.[0]?.message}`);
  return value as Settings;
}

/** Validate both the wire schema and relationships that JSON Schema cannot express. */
export function validateRun(value: unknown): RunFile {
  if (!validateRecord(value)) throw new Error(`Invalid result: ${validateRecord.errors?.[0]?.instancePath} ${validateRecord.errors?.[0]?.message}`);
  const date = new Date(value.created_at);
  if (!Number.isFinite(date.getTime()) || date.toISOString() !== value.created_at) throw new Error('created_at must be a valid UTC date with millisecond precision and Z.');
  if (/[\u0000-\u001f\u007f]/u.test(value.environment.machine)) throw new Error('Machine names cannot contain control characters.');
  if (value.environment.runtime === 'wasm' && value.environment.profile === 'native-tuned') throw new Error('The native-tuned profile requires a native runtime.');
  const identities = new Set<string>();
  for (const measurement of value.measurements) {
    const { settings, engine, scope, plan, samples_ms: samples, statistics } = measurement;
    const isIk = settings.scenario === 'inverse-kinematics';
    if (isIk !== (engine === 'bevy')) throw new Error('Inverse kinematics belongs only to the Bevy animation series.');
    if ((engine === 'jolt' || engine === 'nexus') && settings.dimension !== '3d') throw new Error('This adapter supports only 3D.');
    const expectedScope = engine === 'bevy' ? 'bevy-animation' : engine === 'nexus' ? 'gpu-step-readback'
      : engine === 'jolt' && value.environment.runtime === 'wasm' ? 'binding-step' : 'bevy-step';
    if (scope !== expectedScope) throw new Error('The measurement scope does not match this engine and runtime.');
    if (samples.length !== plan.samples * plan.repeats || statistics.sample_count !== samples.length) throw new Error('Sample counts do not match the sampling plan.');
    if (measurement.awake_bodies > settings.count) throw new Error('Awake bodies exceed the scene body count.');
    if (measurement.queries.hits > measurement.queries.rays) throw new Error('Query hits exceed the ray count.');
    if (settings.scenario === 'queries' ? measurement.queries.rays !== settings.count : measurement.queries.rays !== 0) throw new Error('The query count does not match the workload.');
    if (engine === 'nexus') {
      if (value.environment.profile !== 'gpu' || !value.environment.gpu || !measurement.readback_ms || measurement.readback_ms.length !== samples.length) throw new Error('GPU results require device metadata and one readback duration per sample.');
      if (settings.sleeping !== 'disabled' || settings.ccd !== 'discrete' || !['boxes', 'balls', 'pyramid', 'stacks', 'static-field', 'dominoes', 'joint-chain', 'ragdolls'].includes(settings.scenario)) throw new Error('The settings exceed the supported Nexus profile.');
      if (measurement.readback_ms.some((duration, index) => duration > samples[index] + .01)) throw new Error('GPU readback duration exceeds the total sample.');
    } else if (measurement.readback_ms !== null) throw new Error('CPU results cannot contain GPU readback durations.');
    const recomputed = summarize(samples);
    for (const key of Object.keys(recomputed) as (keyof Statistics)[]) {
      const tolerance = Math.max(1e-9, Math.abs(recomputed[key]) * 1e-7);
      if (Math.abs(statistics[key] - recomputed[key]) > tolerance) throw new Error(`Reported ${key} does not match the raw samples.`);
    }
    const identity = JSON.stringify([engine, Object.entries(settings).sort(), Object.entries(plan).sort(), scope, measurement.scene_fingerprint]);
    if (identities.has(identity)) throw new Error('A result contains a duplicate measurement.');
    identities.add(identity);
  }
  return value;
}
