import { describe, expect, it } from 'vitest';
import { runComparison, RunCoordinator } from '../src/benchmark';
import { DEFAULT_SETTINGS, type Engine, type SimulationFactory } from '../src/types';

const plan = { warmup_steps: 2, samples: 5, batch_steps: 4, repeats: 2 };

it('excludes construction, warmup, snapshots, and scheduling from sample durations', async () => {
  let time = 0;
  const calls: string[] = [];
  const factory: SimulationFactory = async engine => {
    time += 1000; calls.push(`create:${engine}`);
    return { fingerprint: () => 'b'.repeat(64), advance: steps => { time += steps * 2; calls.push(`step:${steps}`); },
      snapshot: () => { time += 1000; return '{"tick":22,"bodies":[],"joints":[]}'; },
      free: () => calls.push(`free:${engine}`),
    };
  };
  const results = await runComparison(factory, ['avian', 'rapier'], DEFAULT_SETTINGS, plan,
    { clock: () => time, yieldControl: async () => { time += 1000; } });
  expect(results.map(result => result.samples_ms)).toEqual([Array(10).fill(2), Array(10).fill(2)]);
  expect(calls.filter(call => call.startsWith('create:'))).toEqual(['create:avian', 'create:rapier', 'create:rapier', 'create:avian']);
  expect(calls.filter(call => call.startsWith('free:'))).toHaveLength(4);
});

it('disposes the active engine when a run is canceled', async () => {
  const controller = new AbortController(); let freed = 0;
  const factory: SimulationFactory = async () => ({ fingerprint: () => 'b'.repeat(64), advance: () => {}, snapshot: () => '', free: () => { freed++; } });
  await expect(runComparison(factory, ['avian'], DEFAULT_SETTINGS, plan, {
    signal: controller.signal, clock: () => 1,
    yieldControl: async () => { controller.abort(); },
  })).rejects.toThrow('canceled');
  expect(freed).toBe(1);
});

it('preserves the latest result when an older request completes last', async () => {
  const coordinator = new RunCoordinator<string>(); let visible = '';
  let resolveA!: (value: string) => void; let resolveB!: (value: string) => void;
  const a = coordinator.start(() => new Promise(resolve => { resolveA = resolve; }), value => { visible = value; });
  const b = coordinator.start(() => new Promise(resolve => { resolveB = resolve; }), value => { visible = value; });
  resolveB('B'); await b;
  resolveA('A'); await a;
  expect(visible).toBe('B');
});

it('waits for GPU completion and records readback inside total time', async () => {
  let time = 0;
  const result = await runComparison(async () => ({
    fingerprint: () => 'b'.repeat(64),
    advance: async () => { await Promise.resolve(); time += 6; },
    readbackMs: () => 2,
    device: () => ({ name: 'Test GPU', backend: 'WebGPU', driver: null }),
    snapshot: () => JSON.stringify({ tick: 1, bodies: [], joints: [] }),
    free: () => {},
  }), ['nexus'], { ...DEFAULT_SETTINGS }, { warmup_steps: 1, samples: 5, batch_steps: 1, repeats: 1 }, { clock: () => time, yieldControl: async () => {} });
  expect(result[0].samples_ms).toEqual([6, 6, 6, 6, 6]);
  expect(result[0].readback_ms).toEqual([2, 2, 2, 2, 2]);
  expect(result[0].scope).toBe('gpu-step-readback');
});

const emptySnapshot = JSON.stringify({ tick: 1, bodies: [], joints: [] });
const fake: SimulationFactory = async () => ({ fingerprint: () => 'b'.repeat(64), advance: () => {}, snapshot: () => emptySnapshot, free: () => {} });

it('validates every sampling bound and rejects empty or repeated engines before construction', async () => {
  const bounds = { warmup_steps: [0, 601, 1.5], samples: [4, 1001, 5.5], batch_steps: [0, 61, 1.5], repeats: [0, 10, 1.5] };
  for (const [key, values] of Object.entries(bounds)) for (const value of values) {
    await expect(runComparison(fake, ['avian'], DEFAULT_SETTINGS, { ...plan, [key]: value })).rejects.toThrow(key);
  }
  for (const engines of [[], ['avian', 'avian']] as Engine[][]) await expect(runComparison(fake, engines, DEFAULT_SETTINGS, plan)).rejects.toThrow('distinct');
});

it('releases worlds when fingerprints, timing, or GPU metadata are invalid', async () => {
  for (const kind of ['fingerprint', 'mismatch', 'clock', 'metadata', 'readback', 'negativeReadback', 'nonfiniteReadback']) {
    let freed = 0, tick = 0, time = 0;
    const factory: SimulationFactory = async () => ({
      fingerprint: () => kind === 'fingerprint' ? 'invalid' : (kind === 'mismatch' && ++tick === 2 ? 'c' : 'b').repeat(64),
      advance: () => { time += 1; }, snapshot: () => emptySnapshot, free: () => { freed++; },
      ...(kind === 'metadata' ? {} : { device: () => ({ name: 'Test', backend: 'test', driver: null }),
        readbackMs: () => kind === 'negativeReadback' ? -1 : kind === 'nonfiniteReadback' ? NaN : 999 }),
    });
    const isGpu = ['metadata', 'readback', 'negativeReadback', 'nonfiniteReadback'].includes(kind);
    await expect(runComparison(factory, isGpu ? ['nexus'] : ['avian', 'rapier'], DEFAULT_SETTINGS, plan,
      { clock: () => kind === 'clock' ? NaN : time, yieldControl: async () => {} })).rejects.toThrow();
    expect(freed).toBeGreaterThan(0);
  }
});

it('preserves the lowest finite height across repetitions', async () => {
  let repetition = 0;
  const body = { position: [0, .25, 0], rotation: [0, 0, 0, 1], shape: { kind: 'ball', radius: .5 }, kind: 'dynamic', sleeping: false };
  const factory: SimulationFactory = async () => {
    const current = repetition++;
    return { fingerprint: () => 'b'.repeat(64), advance: () => {}, free: () => {},
      snapshot: () => JSON.stringify({ tick: 1, joints: [], bodies: [{ ...body, position: [0, current === 0 ? .25 : .5, 0] }] }),
    };
  };
  const result = await runComparison(factory, ['avian'], DEFAULT_SETTINGS, plan, { clock: () => 0, yieldControl: async () => {} });
  expect(result[0].min_y).toBe(.25);
  expect(result[0].finite).toBe(true);
});

it('uses default task yielding and reports progress for animation and binding scopes', async () => {
  let progress = 0;
  const result = await runComparison(fake, ['bevy', 'jolt'], DEFAULT_SETTINGS, { ...plan, repeats: 1 }, { progress: completed => { progress = completed; } });
  expect(progress).toBe(10);
  expect(result.map(row => row.scope)).toEqual(['bevy-animation', 'binding-step']);
});

it('cancels before construction and prevents publication after explicit cancellation', async () => {
  const controller = new AbortController(); controller.abort();
  await expect(runComparison(fake, ['avian'], DEFAULT_SETTINGS, plan, { signal: controller.signal })).rejects.toThrow('canceled');
  const coordinator = new RunCoordinator<string>(); let visible = 'unchanged'; let finish!: (value: string) => void;
  const pending = coordinator.start(() => new Promise(resolve => { finish = resolve; }), value => { visible = value; });
  coordinator.cancel(); finish('stale'); await pending;
  expect(visible).toBe('unchanged');
});

it('retains a failed repetition even after a later finite result', async () => {
  let repetition = 0;
  const factory: SimulationFactory = async () => {
    const height = repetition++ === 0 ? null : .5;
    return { fingerprint: () => 'b'.repeat(64), advance: () => {}, free: () => {},
      snapshot: () => JSON.stringify({ tick: 1, joints: [], bodies: [{ position: [0, height, 0], rotation: [0, 0, 0, 1], kind: 'dynamic', sleeping: false }] }) };
  };
  const result = await runComparison(factory, ['avian'], DEFAULT_SETTINGS, plan, { clock: () => 0, yieldControl: async () => {} });
  expect(result[0]).toMatchObject({ finite: false, min_y: .5 });
});
