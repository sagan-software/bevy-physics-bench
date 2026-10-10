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
