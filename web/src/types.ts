export type Engine = 'avian' | 'rapier' | 'jolt' | 'bevy' | 'nexus';
export type Dimension = '2d' | '3d';
export type Scenario = 'boxes' | 'balls' | 'pyramid' | 'stacks' | 'static-field' | 'kinematics' | 'dominoes' | 'joint-chain' | 'ragdolls' | 'ccd' | 'queries' | 'characters' | 'inverse-kinematics';
export interface Settings {
  scenario: Scenario;
  dimension: Dimension;
  count: number;
  seed: number;
  hz: number;
  gravity: number;
  friction: number;
  restitution: number;
  solver_effort: number;
  sleeping: 'enabled' | 'disabled';
  ccd: 'continuous' | 'discrete';
}
export interface Body {
  position: [number, number, number];
  rotation: [number, number, number, number];
  shape: { kind: 'box'; half_extents: [number, number, number] } | { kind: 'ball'; radius: number };
  kind: 'static' | 'dynamic' | 'kinematic';
  sleeping: boolean;
}
export interface Snapshot { tick: number; bodies: Body[]; joints: [number, number][]; queries?: { rays: number; hits: number } }
export interface Simulation { fingerprint(): string; advance(steps: number): void | Promise<void>; kick?(index: number, x: number, y: number, z: number): void | Promise<void>; append?(count: number): void | Promise<void>; readbackMs?(): number; device?(): { name: string; backend: string; driver: string | null }; snapshot(): string; free(): void }
export type SimulationFactory = (engine: Engine, settings: Settings) => Promise<Simulation>;
export const DEFAULT_SETTINGS: Settings = {
  scenario: 'pyramid', dimension: '3d', count: 128, seed: 42, hz: 60,
  gravity: 9.81, friction: 0.5, restitution: 0, solver_effort: 4,
  sleeping: 'disabled', ccd: 'discrete',
};
export const ENGINES: Record<Engine, { name: string; version: string; color: string; effort: string }> = {
  avian: { name: 'Avian', version: '0.7.0', color: '#7cdbbc', effort: 'substeps' },
  rapier: { name: 'Rapier', version: '0.36.0', color: '#e8b776', effort: 'solver iterations' },
  nexus: { name: 'Nexus GPU', version: '0.6.0', color: '#bfa0d8', effort: 'GPU solver iterations' },
  bevy: { name: 'Bevy IK', version: '0.19.1', color: '#c1c9cc', effort: 'not used by the analytic IK solver' },
  jolt: { name: 'Jolt', version: 'JS 1.1.0', color: '#aabcf0', effort: 'full substeps; 10 velocity / 2 position passes each' },
};
export const SCENARIOS: { id: Scenario; name: string; description: string; group: string }[] = [
  { id: 'pyramid', name: 'Pyramid', description: 'A triangular stack of boxes under gravity.', group: 'Rigid bodies' },
  { id: 'boxes', name: 'Box pile', description: 'Separated boxes fall and form a contact pile.', group: 'Rigid bodies' },
  { id: 'balls', name: 'Ball pit', description: 'Circles or spheres fall onto a flat floor.', group: 'Rigid bodies' },
  { id: 'stacks', name: 'Tall stacks', description: 'Twelve-box columns test resting contact stability.', group: 'Rigid bodies' },
  { id: 'dominoes', name: 'Dominoes', description: 'The first domino starts a chain of collisions.', group: 'Rigid bodies' },
  { id: 'static-field', name: 'Static field', description: 'One dynamic body for every three static colliders.', group: 'Collision and motion' },
  { id: 'kinematics', name: 'Moving platforms', description: 'Prescribed platforms push dynamic bodies.', group: 'Collision and motion' },
  { id: 'ccd', name: 'Fast projectiles', description: 'Small bodies travel at 120 m/s toward a thin floor. Enable CCD to compare tunneling.', group: 'Collision and motion' },
  { id: 'queries', name: 'Ray queries', description: 'Short downward rays alternate collider hits and empty-space misses each step.', group: 'Collision and motion' },
  { id: 'characters', name: 'Character controllers', description: 'Shape-cast controllers move across fixed ramps. Controller algorithms differ between engines.', group: 'Collision and motion' },
  { id: 'joint-chain', name: 'Joint chains', description: 'Twelve-link chains hang from fixed anchors.', group: 'Constraints' },
  { id: 'ragdolls', name: 'Passive ragdolls', description: 'Six-part humanoids connected by passive point joints.', group: 'Constraints' },
  { id: 'inverse-kinematics', name: 'Two-bone IK', description: 'Analytic one-metre limbs follow moving targets. This Bevy animation series does not rank physics engines.', group: 'Animation' },
];
