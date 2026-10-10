import { expect, it } from 'vitest';
import { runCapacity, validateCapacity, type CapacityPlan } from '../src/capacity';
import { DEFAULT_SETTINGS, type Simulation } from '../src/types';

it('grows the same world and scores the last load that sustained the target', async () => {
  let count = 1, time = 0, freed = 0;
  const world: Simulation = { fingerprint: () => 'scene', advance: () => {}, append: value => { count += value; }, snapshot: () => JSON.stringify({ tick: 1, bodies: Array.from({ length: count }, () => ({ position: [0, 1, 0], rotation: [0, 0, 0, 1], kind: 'dynamic' })), joints: [] }), free: () => { freed++; } };
  const result = await runCapacity(async () => world, 'avian', { ...DEFAULT_SETTINGS, scenario: 'boxes', count: 1 },
    { target_fps: 60, tolerance_percent: 3, increment: 1, maximum: 3, settle_ms: 100, window_ms: 100 },
    { signal: new AbortController().signal, now: () => time, frame: async () => { time += count === 1 ? 16 : 40; }, render: () => {} });
  expect(result.score).toBe(1);
  expect(result.outcome).toBe('threshold');
  expect(result.levels.map(level => level.count)).toEqual([1, 2]);
  expect(result.levels[1].windows).toHaveLength(2);
  expect(count).toBe(2);
  expect(freed).toBe(1);
});

const plan: CapacityPlan = { target_fps: 60, tolerance_percent: 3, increment: 2, maximum: 4, settle_ms: 100, window_ms: 100 };
const settings = { ...DEFAULT_SETTINGS, scenario: 'boxes' as const, count: 1 };
const snapshot = () => JSON.stringify({ tick: 1, bodies: [{ position: [0, 1, 0], rotation: [0, 0, 0, 1], kind: 'dynamic' }], joints: [] });

it('caps the final batch, retries a transient dip, and releases the world', async () => {
  let time = 0, count = 1, freed = 0, calls = 0;
  const additions: number[] = [];
  const world: Simulation = { fingerprint: () => 'scene', advance: () => {}, append: value => { additions.push(value); count += value; }, snapshot: () => JSON.stringify({ ...JSON.parse(snapshot()), bodies: Array.from({ length: count }, () => JSON.parse(snapshot()).bodies[0]) }), free: () => { freed++; } };
  const result = await runCapacity(async () => world, 'rapier', settings, plan, {
    signal: new AbortController().signal, now: () => time, frame: async () => { calls++; time += calls >= 8 && calls <= 10 ? 40 : 16; }, render: () => {},
  });
  expect(result.outcome).toBe('limit'); expect(result.score).toBe(4);
  expect(result.levels[0].windows).toHaveLength(2);
  expect(additions).toEqual([2, 1]); expect(count).toBe(4); expect(freed).toBe(1);
});

it('rejects every invalid plan boundary before allocation', async () => {
  for (const [key, values] of Object.entries({ target_fps: [0, 241, 1.5, NaN], tolerance_percent: [-1, 11], increment: [0, 4097], maximum: [0, 32769], settle_ms: [99, 10001], window_ms: [99, 10001] })) {
    for (const value of values) expect(() => validateCapacity({ ...plan, [key]: value }, settings)).toThrow();
  }
  expect(() => validateCapacity(plan, { ...settings, scenario: 'pyramid' })).toThrow();
  expect(() => validateCapacity(plan, { ...settings, count: 0 })).toThrow();
  expect(() => validateCapacity(plan, { ...settings, count: 5 })).toThrow();
  expect(() => validateCapacity({ target_fps: 1, tolerance_percent: 0, increment: 1, maximum: 1, settle_ms: 100, window_ms: 100 }, settings)).not.toThrow();
  expect(() => validateCapacity({ target_fps: 240, tolerance_percent: 10, increment: 4096, maximum: 32768, settle_ms: 10000, window_ms: 10000 }, { ...settings, scenario: 'balls' })).not.toThrow();
});

it('keeps engine and health errors distinct from FPS threshold outcomes', async () => {
  for (const failure of ['missing-append', 'step', 'nonfinite', 'below-floor', 'count']) {
    let freed = 0, time = 0;
    const world: Simulation = { fingerprint: () => 'scene', advance: () => { if (failure === 'step') throw new Error('engine capacity'); }, snapshot: () => failure === 'count' ? '{"bodies":[]}' : failure === 'nonfinite' ? '{"tick":0,"bodies":[{"position":[1e999,1,0],"rotation":[0,0,0,1],"kind":"dynamic"}],"joints":[]}' : failure === 'below-floor' ? snapshot().replace('[0,1,0]', '[0,-1,0]') : snapshot(), free: () => { freed++; } };
    if (failure !== 'missing-append') world.append = () => {};
    const result = await runCapacity(async () => world, 'avian', settings, plan, { signal: new AbortController().signal, now: () => time, frame: async () => { time += 16; }, render: () => {} });
    expect(result.outcome).toBe('error'); expect(result.score).toBe(0); expect(result.detail).toBeTruthy(); expect(freed).toBe(1);
  }
});

it('does not publish aborted work, including a stalled in-flight step', async () => {
  const controller = new AbortController(); let release!: () => void, freed = 0, rendered = 0;
  let entered!: () => void; const enteredPromise = new Promise<void>(resolve => { entered = resolve; });
  const world: Simulation = { fingerprint: () => 'scene', advance: () => { entered(); return new Promise<void>(resolve => { release = resolve; }); }, append: () => {}, snapshot, free: () => { freed++; } };
  const pending = runCapacity(async () => world, 'avian', settings, plan, { signal: controller.signal, now: () => 0, frame: async () => {}, render: () => { rendered++; } });
  await enteredPromise; controller.abort(); expect(freed).toBe(0); release();
  await expect(pending).rejects.toThrow(); expect(freed).toBe(1); expect(rendered).toBe(0);
  let allocated = false;
  await expect(runCapacity(async () => { allocated = true; return world; }, 'avian', settings, plan, { signal: controller.signal, now: () => 0, frame: async () => {}, render: () => {} })).rejects.toThrow();
  expect(allocated).toBe(false);
});

it('records the GPU actually used and permits static geometry below the floor', async () => {
  let time = 0;
  const device = { name: 'test GPU', backend: 'WebGPU', driver: null };
  const world: Simulation = { fingerprint: () => 'scene', device: () => device, advance: () => {}, append: () => {}, snapshot: () => snapshot().replace('dynamic', 'static').replace('[0,1,0]', '[0,-1,0]'), free: () => {} };
  const result = await runCapacity(async () => world, 'nexus', settings, { ...plan, maximum: 1 }, { signal: new AbortController().signal, now: () => time, frame: async () => { time += 16; }, render: () => {} });
  expect(result.gpu).toEqual(device); expect(result.outcome).toBe('limit');
});

it('rejects a stalled display instead of assigning a misleading FPS score', async () => {
  let time = 0;
  const world: Simulation = { fingerprint: () => 'scene', advance: () => {}, append: () => {}, snapshot, free: () => {} };
  const result = await runCapacity(async () => world, 'avian', settings, plan, { signal: new AbortController().signal, now: () => time, frame: async () => { time += 2001; }, render: () => {} });
  expect(result.outcome).toBe('error'); expect(result.detail).toContain('frame delivery stalled'); expect(result.levels).toEqual([]);
});

it('retains only complete levels when a retry fails', async () => {
  let time = 0, calls = 0, freed = 0;
  const world: Simulation = { fingerprint: () => 'scene', advance: () => {}, append: () => {}, snapshot, free: () => { freed++; } };
  const result = await runCapacity(async () => world, 'avian', settings, plan, {
    signal: new AbortController().signal, now: () => time, frame: async () => { if (++calls === 7) throw new Error('lost display'); time += 40; }, render: () => {},
  });
  expect(result.outcome).toBe('error'); expect(result.levels).toEqual([]); expect(result.score).toBe(0); expect(freed).toBe(1);
});
