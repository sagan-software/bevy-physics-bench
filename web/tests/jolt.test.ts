import { expect, it } from 'vitest';
import { createJoltSimulation, type InitialScene } from '../src/jolt';
import { DEFAULT_SETTINGS, type Snapshot } from '../src/types';

const scene: InitialScene = { fingerprint: 'b'.repeat(64), floor_half_extents: [20, .5, 20], bodies: [{
  position: [0, 2, 0], rotation: [0, 0, 0, 1], shape: { kind: 'ball', radius: .5 }, kind: 'dynamic',
  velocity: [0, 0, 0], angular_velocity: [0, 0, 0], joint: null,
}] };

it('runs actual Jolt WebAssembly and releases its world', async () => {
  const simulation = await createJoltSimulation({ ...DEFAULT_SETTINGS, count: 1 }, scene);
  simulation.advance(120);
  const snapshot = JSON.parse(simulation.snapshot()) as Snapshot;
  expect(snapshot.tick).toBe(120);
  expect(snapshot.bodies[0].position[1]).toBeGreaterThan(.45);
  expect(snapshot.bodies[0].position[1]).toBeLessThan(.6);
  simulation.free(); simulation.free();
  expect(() => simulation.advance(1)).toThrow('released');
});

it('rejects a planar configuration before creating a Jolt world', async () => {
  await expect(createJoltSimulation({ ...DEFAULT_SETTINGS, dimension: '2d' }, scene)).rejects.toThrow('3D');
});

it('casts alternating short hits and misses without counting the floor', async () => {
  const bodies = Array.from({ length: 4 }, (_, index) => ({ ...scene.bodies[0], kind: 'static' as const, position: [index * 2, 2, 0] as [number, number, number] }));
  const simulation = await createJoltSimulation({ ...DEFAULT_SETTINGS, scenario: 'queries', count: 4 }, { ...scene, bodies });
  simulation.advance(2);
  expect(JSON.parse(simulation.snapshot()).queries).toEqual({ rays: 4, hits: 2 });
  simulation.free();
});

it('rejects capacity smaller than the initial world', async () => {
  let accepted = false;
  try { const world = await createJoltSimulation({ ...DEFAULT_SETTINGS, count: 1 }, scene, 0); world.free(); accepted = true; } catch { /* Rejection precedes world ownership. */ }
  expect(accepted).toBe(false);
});

it('validates every capacity, kick, and insertion boundary without changing the world', async () => {
  for (const capacity of [NaN, Infinity, -1, .5, 32769]) await expect(createJoltSimulation({ ...DEFAULT_SETTINGS, count: 1 }, scene, capacity)).rejects.toThrow('Capacity');
  const world = await createJoltSimulation({ ...DEFAULT_SETTINGS, scenario: 'balls', count: 1, gravity: 0 }, scene, 4,
    (count, existing) => Array.from({ length: count }, (_, i) => ({ ...scene.bodies[0], position: [4 + i * 2, 6, 0] as [number, number, number] })));
  const before = world.snapshot();
  for (const index of [-1, 1, .5]) expect(() => world.kick!(index, 1, 0, 0)).toThrow('Invalid');
  for (const x of [NaN, Infinity, -100.01, 100.01]) expect(() => world.kick!(0, x, 0, 0)).toThrow('Invalid');
  for (const count of [0, -1, .5, 4]) expect(() => world.append!(count)).toThrow('capacity');
  expect(world.snapshot()).toBe(before);
  world.advance(5); world.append!(2);
  const grown = JSON.parse(world.snapshot()); expect(grown.tick).toBe(5); expect(grown.bodies).toHaveLength(3);
  world.kick!(1, 6, 0, 0); world.advance(10);
  expect(JSON.parse(world.snapshot()).bodies[1].position[0]).toBeGreaterThan(4.9);
  world.free();
  expect(() => world.kick!(0, 0, 0, 0)).toThrow('released'); expect(() => world.append!(1)).toThrow('released');
});

it('rejects non-dynamic kicks and unsupported growth', async () => {
  const world = await createJoltSimulation({ ...DEFAULT_SETTINGS, count: 1 }, { ...scene, bodies: [{ ...scene.bodies[0], kind: 'static' }] }, 2);
  expect(() => world.kick!(0, 1, 0, 0)).toThrow('dynamic');
  expect(() => world.append!(1)).toThrow('Growth'); world.free();
  const withBatch = await createJoltSimulation({ ...DEFAULT_SETTINGS, scenario: 'pyramid', count: 1 }, scene, 2, () => []);
  expect(() => withBatch.append!(1)).toThrow('Growth'); withBatch.free();
});
