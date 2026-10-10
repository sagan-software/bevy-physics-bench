import { expect, it } from 'vitest';
import { validateCapacityRecord, type CapacityRecord } from '../src/capacity-records';
import { DEFAULT_SETTINGS } from '../src/types';
import run from '../public/results/2026-10-10-5950x-ccd-3d.json' with { type: 'json' };
import publishedCapacity from '../data/capacity.json';

function record(): CapacityRecord {
  return { kind: 'capacity-v1', created_at: '2026-10-10T00:00:00.000Z', machine: 'test', browser: 'test', os: 'test', logical_cores: 1, viewport: [800, 400], pixel_ratio: 1,
    metadata: { build: run.build, versions: run.versions, target: 'wasm32-unknown-unknown', profile: 'baseline' }, measurements: [{ engine: 'avian', settings: { ...DEFAULT_SETTINGS, scenario: 'boxes', count: 1 }, plan: { target_fps: 60, tolerance_percent: 3, increment: 1, maximum: 2, settle_ms: 100, window_ms: 100 }, score: 1, outcome: 'threshold', detail: null, scene_fingerprint: '0'.repeat(64), gpu: null,
      levels: [{ count: 1, windows: [{ frames: 6, elapsed_ms: 100, fps: 60 }] }, { count: 2, windows: [{ frames: 3, elapsed_ms: 100, fps: 30 }, { frames: 3, elapsed_ms: 100, fps: 30 }] }] }] };
}
function reject(change: (value: CapacityRecord) => void): void { const value = record(); change(value); expect(() => validateCapacityRecord(value)).toThrow(); }
const pass = { frames: 6, elapsed_ms: 100, fps: 60 };
const gpu = { name: 'test GPU', backend: 'WebGPU', driver: null };

it('validates scores, exact threshold boundaries, transient retries, and terminal outcomes', () => {
  expect(validateCapacityRecord(record()).measurements[0].score).toBe(1);
  const limit = record(), measurement = limit.measurements[0];
  measurement.levels[1].windows[1] = pass; measurement.score = 2; measurement.outcome = 'limit';
  expect(validateCapacityRecord(limit)).toBe(limit);
  measurement.levels[1].windows = [{ frames: 6, elapsed_ms: 100, fps: 60 }]; measurement.plan.tolerance_percent = 0;
  expect(validateCapacityRecord(limit)).toBe(limit);
  const error = record(); error.measurements[0] = { ...error.measurements[0], score: 0, levels: [], outcome: 'error', detail: 'Allocation failed' };
  expect(validateCapacityRecord(error)).toBe(error);
  const firstFailed = record(); firstFailed.measurements[0].levels.shift(); firstFailed.measurements[0].levels[0].count = 1; firstFailed.measurements[0].score = 0;
  expect(validateCapacityRecord(firstFailed)).toBe(firstFailed);
});

it('rejects contradictory levels, retry windows, scores, and outcomes', () => {
  reject(value => { value.measurements[0].score = 2; });
  reject(value => { value.measurements[0].levels[0].count = 2; });
  reject(value => { value.measurements[0].levels.push(value.measurements[0].levels[1]); });
  reject(value => { value.measurements[0].levels[0].windows[0].elapsed_ms = 99; });
  reject(value => { value.measurements[0].levels[0].windows[0].fps = 59; });
  reject(value => { value.measurements[0].levels[1].windows.pop(); });
  reject(value => { value.measurements[0].levels[0].windows.push(pass); });
  reject(value => { value.measurements[0].outcome = 'limit'; });
  reject(value => { value.measurements[0].levels.pop(); value.measurements[0].outcome = 'limit'; });
  reject(value => { value.measurements[0].detail = 'unexpected error'; });
  reject(value => { value.measurements[0].levels = []; value.measurements[0].score = 0; value.measurements[0].outcome = 'error'; });
  reject(value => { const m = value.measurements[0]; m.levels[1].windows = [pass]; m.score = 2; m.outcome = 'error'; m.detail = 'impossible after completion'; });
  reject(value => { const m = value.measurements[0]; m.levels[1].windows = [pass]; m.levels.push(m.levels[1]); m.score = 2; m.outcome = 'limit'; });
});

it('checks engine dimensions, GPU metadata, and the incremental Nexus profile', () => {
  for (const engine of ['avian', 'rapier', 'jolt', 'nexus'] as const) {
    const value = record(); value.measurements[0].engine = engine;
    if (engine === 'nexus') value.measurements[0].gpu = gpu;
    expect(validateCapacityRecord(value)).toBe(value);
  }
  reject(value => { value.measurements.push(value.measurements[0]); });
  for (const engine of ['jolt', 'nexus'] as const) reject(value => { value.measurements[0].engine = engine; value.measurements[0].settings.dimension = '2d'; });
  reject(value => { value.measurements[0].gpu = gpu; });
  reject(value => { value.measurements[0].engine = 'nexus'; });
  for (const change of [{ hz: 30 }, { solver_effort: 3 }, { sleeping: 'enabled' }, { ccd: 'continuous' }]) reject(value => {
    value.measurements[0].engine = 'nexus'; value.measurements[0].gpu = gpu; Object.assign(value.measurements[0].settings, change);
  });
});

it('requires an exact UTC timestamp, machine label, browser build, and valid growth plan', () => {
  for (const created_at of ['2026-99-10T00:00:00.000Z', '2026-02-30T00:00:00.000Z', '2026-10-10T00:00:00Z', '2026-10-10T00:00:00.000+00:00']) reject(value => { value.created_at = created_at; });
  for (const machine of [' ', 'test\n', 'test\u007f']) reject(value => { value.machine = machine; });
  reject(value => { value.metadata.target = 'x86_64-unknown-linux-gnu'; });
  reject(value => { value.metadata.profile = 'native-tuned'; });
  reject(value => { value.measurements[0].settings.scenario = 'pyramid'; });
  reject(value => { value.measurements[0].plan.maximum = 1; value.measurements[0].settings.count = 2; });
});

it('rejects missing members and additional members at each object boundary', () => {
  const paths = [[], ['metadata'], ['metadata', 'build'], ['metadata', 'versions'], ['measurements', 0], ['measurements', 0, 'settings'], ['measurements', 0, 'plan'], ['measurements', 0, 'levels', 0], ['measurements', 0, 'levels', 0, 'windows', 0]];
  for (const path of paths) {
    const object = path.reduce<any>((value, key) => value[key], record());
    for (const key of Object.keys(object)) {
      const value = record(), target = path.reduce<any>((item, key) => item[key], value); delete target[key];
      expect(() => validateCapacityRecord(value), `${path.join('.')}.${key}`).toThrow();
    }
    const value = record(); path.reduce<any>((item, key) => item[key], value).extra = true;
    expect(() => validateCapacityRecord(value)).toThrow();
  }
  for (const key of Object.keys(gpu)) reject(value => { value.measurements[0].engine = 'nexus'; value.measurements[0].gpu = { ...gpu }; delete (value.measurements[0].gpu as any)[key]; });
  reject(value => { (value.measurements[0].gpu as any) = { ...gpu, extra: true }; });
});
it('validates every published capacity record before the site uses it', () => {
  for (const record of publishedCapacity) expect(() => validateCapacityRecord(record)).not.toThrow();
});
