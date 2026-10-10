import type { RunFile } from './results';
import type { Settings, Engine, Simulation } from './types';

interface WasmModule {
  default(): Promise<unknown>;
  BrowserSimulation: new (engine: string, settings: string) => Simulation;
  describe_scene(settings: string): string;
  build_metadata(): string;
  BrowserGpu?: { create(settings: string): Promise<Simulation & { metadata(): string; readback_ms(): number }> };
}
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
export async function createSimulation(engine: Engine, settings: Settings): Promise<Simulation> {
  const module = await runtime();
  if (engine === 'nexus') {
    if (!module.BrowserGpu) throw new Error('This build does not include the experimental GPU adapter.');
    const simulation = await module.BrowserGpu.create(JSON.stringify(settings));
    return { fingerprint: () => simulation.fingerprint(), advance: steps => simulation.advance(steps), snapshot: () => simulation.snapshot(), free: () => simulation.free(), readbackMs: () => simulation.readback_ms(), device: () => JSON.parse(simulation.metadata()) };
  }
  if (engine === 'jolt') {
    const { createJoltSimulation } = await import('./jolt');
    return createJoltSimulation(settings, JSON.parse(module.describe_scene(JSON.stringify(settings))));
  }
  return new module.BrowserSimulation(engine, JSON.stringify(settings));
}

/** Read provenance from the compiled module that constructs the benchmark scenes. */
export async function runtimeMetadata(): Promise<Pick<RunFile, 'build' | 'versions'> & { target: string; profile: RunFile['environment']['profile'] }> {
  return JSON.parse((await runtime()).build_metadata());
}

/** Read the generated floor dimensions once for rendering outside the measured operation. */
export async function floorExtents(settings: Settings): Promise<[number, number, number]> {
  return JSON.parse((await runtime()).describe_scene(JSON.stringify(settings))).floor_half_extents;
}

/** Probe both the compiled adapter and the browser device before enabling GPU selection. */
export async function gpuAvailability(): Promise<boolean> {
  const gpu = (navigator as Navigator & { gpu?: { requestAdapter(): Promise<unknown> } }).gpu;
  return !!gpu && !!(await runtime()).BrowserGpu && !!await gpu.requestAdapter();
}
