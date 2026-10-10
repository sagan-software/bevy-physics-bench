import type { RunFile } from './results';
import type { Settings, Engine, Simulation } from './types';

interface WasmModule {
  default(): Promise<unknown>;
  BrowserSimulation: { new (engine: string, settings: string): Simulation; growing(engine: string, settings: string, capacity: number): Simulation };
  describe_scene(settings: string): string;
  describe_growing(settings: string, capacity: number): string;
  describe_drop(settings: string, count: number, existing: number): string;
  build_metadata(): string;
  BrowserGpu?: { create(settings: string): Promise<GpuWorld>; growing(settings: string, capacity: number): Promise<GpuWorld> };
}
type GpuWorld = Simulation & { metadata(): string; readback_ms(): number };
let loading: Promise<WasmModule> | undefined;

/** Load one shared WebAssembly module; each instance owns a separate Bevy world. */
async function runtime(): Promise<WasmModule> {
  loading ??= (async () => {
    const module: WasmModule = await import('../pkg/bevy_physics_bench.js');
    await module.default();
    return module;
  })().catch(error => { loading = undefined; throw error; });
  return loading;
}

/** Construct one engine world from the shared validated scene description. */
export async function createSimulation(engine: Engine, settings: Settings, capacity?: number): Promise<Simulation> {
  const module = await runtime();
  if (engine === 'nexus') {
    if (!module.BrowserGpu) throw new Error('This build does not include the experimental GPU adapter.');
    const simulation = capacity === undefined ? await module.BrowserGpu.create(JSON.stringify(settings)) : await module.BrowserGpu.growing(JSON.stringify(settings), capacity);
    return { fingerprint: () => simulation.fingerprint(), advance: steps => simulation.advance(steps), kick: (...args) => simulation.kick!(...args), append: count => simulation.append!(count), snapshot: () => simulation.snapshot(), free: () => simulation.free(), readbackMs: () => simulation.readback_ms(), device: () => JSON.parse(simulation.metadata()) };
  }
  if (engine === 'jolt') {
    const { createJoltSimulation } = await import('./jolt');
    const scene = capacity === undefined ? module.describe_scene(JSON.stringify(settings)) : module.describe_growing(JSON.stringify(settings), capacity);
    return createJoltSimulation(settings, JSON.parse(scene), capacity,
      (count, existing) => JSON.parse(module.describe_drop(JSON.stringify(settings), count, existing)));
  }
  return capacity === undefined ? new module.BrowserSimulation(engine, JSON.stringify(settings)) : module.BrowserSimulation.growing(engine, JSON.stringify(settings), capacity);
}

/** Read provenance from the compiled module that constructs the benchmark scenes. */
export async function runtimeMetadata(): Promise<Pick<RunFile, 'build' | 'versions'> & { target: string; profile: RunFile['environment']['profile'] }> {
  return JSON.parse((await runtime()).build_metadata());
}

/** Read the generated floor dimensions once for rendering outside the measured operation. */
export async function floorExtents(settings: Settings, capacity?: number): Promise<[number, number, number]> {
  const module = await runtime();
  const scene = capacity === undefined ? module.describe_scene(JSON.stringify(settings)) : module.describe_growing(JSON.stringify(settings), capacity);
  return JSON.parse(scene).floor_half_extents;
}

/** Probe both the compiled adapter and the browser device before enabling GPU selection. */
export async function gpuAvailability(): Promise<boolean> {
  const gpu = (navigator as Navigator & { gpu?: { requestAdapter(): Promise<unknown> } }).gpu;
  return !!gpu && !!(await runtime()).BrowserGpu && !!await gpu.requestAdapter();
}
