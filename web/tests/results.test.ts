import { describe, expect, it } from 'vitest';
import { validateRun, validateSettings, comparable, health, summarize, type RunFile } from '../src/results';
import { DEFAULT_SETTINGS } from '../src/types';

import { record } from './record';

it('accepts a complete record without coercing or stripping values', () => {
  const run = record(); expect(validateRun(run)).toBe(run);
});

describe('rejects invalid records before rendering', () => {
  for (const [name, mutate] of Object.entries({
    version: (run: any) => { run.schema_version = 2; },
    extra: (run: any) => { run.extra = 'unknown'; },
    timestamp: (run: any) => { run.created_at = '2026-10-10T00:00:00+00:00'; },
    calendar: (run: any) => { run.created_at = '2026-02-30T00:00:00.000Z'; },
    engine: (run: any) => { run.measurements[0].engine = '__proto__'; },
    count: (run: any) => { run.measurements[0].settings.count = 0; },
    samples: (run: any) => { run.measurements[0].samples_ms.pop(); },
    negative: (run: any) => { run.measurements[0].samples_ms[0] = -1; },
    nonfinite: (run: any) => { run.measurements[0].samples_ms[0] = NaN; },
    mean: (run: any) => { run.measurements[0].statistics.mean_ms = 99; },
    percentile: (run: any) => { run.measurements[0].statistics.p95_ms = 4; },
    awake: (run: any) => { run.measurements[0].awake_bodies = 129; },
    queries: (run: any) => { run.measurements[0].queries.hits = 1; },
    planarJolt: (run: any) => { run.measurements[0].engine = 'jolt'; run.measurements[0].settings.dimension = '2d'; },
    ikRanking: (run: any) => { run.measurements[0].settings.scenario = 'inverse-kinematics'; },
    fingerprint: (run: any) => { run.measurements[0].scene_fingerprint = 'unknown'; },
    profile: (run: any) => { run.environment.profile = 'fastest'; },
  })) it(name, () => { const run = record(); (mutate as unknown as (run: RunFile) => void)(run); expect(() => validateRun(run)).toThrow(); });
});

it('checks settings independently and accepts exact documented boundaries', () => {
  for (const count of [1, 32768]) expect(validateSettings({ ...DEFAULT_SETTINGS, count }).count).toBe(count);
  for (const changes of [{ gravity: Infinity }, { gravity: -1 }, { friction: 2.1 }, { restitution: 1.1 },
    { solver_effort: 0 }, { solver_effort: 33 }, { hz: 59 }, { seed: -1 }, { seed: 4294967296 }, { extra: 1 }]) {
    expect(() => validateSettings({ ...DEFAULT_SETTINGS, ...changes })).toThrow();
  }
});

it('compares settings by value while preserving timing scopes and fingerprints', () => {
  const first = record().measurements[0], second = structuredClone(first);
  second.settings = Object.fromEntries(Object.entries(second.settings).reverse()) as typeof second.settings;
  expect(comparable(first, second)).toBe(true);
  second.scope = 'binding-step'; expect(comparable(first, second)).toBe(false);
  second.scope = first.scope; second.scene_fingerprint = 'c'.repeat(64); expect(comparable(first, second)).toBe(false);
});

it('reports invalid poses without using them in minimum-height statistics', () => {
  expect(health({ tick: 0, bodies: [], joints: [] })).toMatchObject({ finite: true, awake_bodies: 0, min_y: null });
  const body = { position: [0, NaN, 0], rotation: [0, 0, 0, 1], shape: { kind: 'ball', radius: .5 }, kind: 'dynamic', sleeping: false } as const;
  expect(health({ tick: 1, bodies: [body as any], joints: [] })).toMatchObject({ finite: false, awake_bodies: 1, min_y: null });
});

it('uses even and odd medians, nearest-rank p95, and population dispersion', () => {
  expect(summarize([4, 1, 3, 2])).toMatchObject({ mean_ms: 2.5, median_ms: 2.5, p95_ms: 4, sample_count: 4 });
  expect(summarize([0])).toMatchObject({ cv_percent: 0, stddev_ms: 0 });
  expect(summarize([3, 1, 2]).median_ms).toBe(2);
  for (const samples of [[], [-1], [Infinity], [NaN]]) expect(() => summarize(samples)).toThrow();
});

it('rejects unsupported GPU profiles and readback durations outside the total sample', () => {
  const run = record(); const measurement = run.measurements[0];
  measurement.engine = 'nexus'; measurement.scope = 'gpu-step-readback'; measurement.readback_ms = [0, 1, 2, 3, 4];
  run.environment.profile = 'gpu'; run.environment.gpu = { name: 'Test GPU', backend: 'Vulkan', driver: null };
  expect(validateRun(run)).toBe(run);
  for (const settings of [{ scenario: 'ccd' }, { sleeping: 'enabled' }, { ccd: 'continuous' }]) {
    const invalid = structuredClone(run); Object.assign(invalid.measurements[0].settings, settings);
    expect(() => validateRun(invalid)).toThrow();
  }
  measurement.readback_ms[0] = 2;
  expect(() => validateRun(run)).toThrow('readback');
});

it('rejects a native execution profile attached to a browser result', () => {
  const run = record(); run.environment.profile = 'native-tuned';
  expect(() => validateRun(run)).toThrow('native');
});

it('rejects every missing required record and settings field', () => {
  for (const key of Object.keys(record())) { const run: any = record(); delete run[key]; expect(() => validateRun(run)).toThrow(); }
  for (const key of Object.keys(DEFAULT_SETTINGS)) { const settings: any = { ...DEFAULT_SETTINGS }; delete settings[key]; expect(() => validateSettings(settings)).toThrow(); }
});

it('validates measurement relationships independently of schema shape', () => {
  for (const mutate of [
    (r: RunFile) => { r.environment.machine = 'host\nname'; },
    (r: RunFile) => { r.created_at = '2026-99-10T00:00:00.000Z'; },
    (r: RunFile) => { r.measurements[0].scope = 'binding-step'; },
    (r: RunFile) => { r.measurements[0].readback_ms = [1]; },
    (r: RunFile) => { r.measurements[0].settings.scenario = 'queries'; },
    (r: RunFile) => { r.measurements[0].queries = { rays: 1, hits: 0 }; },
    (r: RunFile) => { r.measurements.push(structuredClone(r.measurements[0])); },
    (r: RunFile) => { r.measurements[0].engine = 'nexus'; r.measurements[0].scope = 'gpu-step-readback'; },
  ]) { const run = record(); mutate(run); expect(() => validateRun(run)).toThrow(); }
  for (const [engine, scope, runtime] of [['jolt','binding-step','wasm'], ['jolt','bevy-step','native'], ['bevy','bevy-animation','wasm']] as const) {
    const run = record(); Object.assign(run.measurements[0], { engine, scope }); run.environment.runtime = runtime;
    if (engine === 'bevy') run.measurements[0].settings.scenario = 'inverse-kinematics';
    expect(validateRun(run)).toBe(run);
  }
});

it('keeps finite pose health and settings, plan, and failure comparison boundaries', () => {
  const body = { position: [0, .25, 0], rotation: [0, 0, 0, 1], shape: { kind: 'ball', radius: .5 }, kind: 'dynamic', sleeping: false };
  expect(health({ tick: 1, bodies: [body as any, { ...body, sleeping: true } as any, { ...body, kind: 'static' } as any], joints: [], queries: { rays: 3, hits: 2 } })).toEqual({ finite: true, min_y: .25, awake_bodies: 1, queries: { rays: 3, hits: 2 } });
  const first = record().measurements[0];
  for (const mutate of [(m: typeof first) => { m.settings.count = 1; }, (m: typeof first) => { m.plan.repeats = 2; }, (m: typeof first) => { m.finite = false; }]) {
    const second = structuredClone(first); mutate(second); expect(comparable(first, second)).toBe(false);
  }
  expect(comparable({ ...first, finite: false }, first)).toBe(false);
});
