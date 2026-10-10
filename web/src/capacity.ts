import type { Engine, Settings, Simulation, Snapshot } from './types';

/** One fixed physics step per visible frame; all durations use milliseconds. */
export interface CapacityPlan { target_fps: number; tolerance_percent: number; increment: number; maximum: number; settle_ms: number; window_ms: number }
export interface CapacityWindow { frames: number; elapsed_ms: number; fps: number }
export interface CapacityLevel { count: number; windows: CapacityWindow[] }
export interface CapacityMeasurement { engine: Engine; settings: Settings; plan: CapacityPlan; score: number; outcome: 'threshold' | 'limit' | 'error'; detail: string | null; levels: CapacityLevel[]; scene_fingerprint: string; gpu: { name: string; backend: string; driver: string | null } | null }
export interface CapacityHooks {
  signal: AbortSignal;
  now(): number;
  frame(): Promise<void>;
  render(snapshot: Snapshot, physicsMs: number, count: number, phase: 'settling' | 'sampling'): void;
}

/** Validate the local capacity profile before constructing an engine world. */
export function validateCapacity(plan: CapacityPlan, settings: Settings): void {
  const between = (value: number, low: number, high: number) => Number.isInteger(value) && value >= low && value <= high;
  if (!['boxes', 'balls'].includes(settings.scenario) || !between(settings.count, 1, 32768)
    || !between(plan.target_fps, 1, 240) || !between(plan.tolerance_percent, 0, 10)
    || !between(plan.increment, 1, 4096) || !between(plan.maximum, settings.count, 32768)
    || !between(plan.settle_ms, 100, 10000) || !between(plan.window_ms, 100, 10000)) throw new Error('Invalid capacity settings. Use a box or ball scene and a maximum at least as large as its body count.');
}

/** Grow one owned world. Two consecutive missed windows stop at the last passing load. */
export async function runCapacity(factory: (engine: Engine, settings: Settings, capacity: number) => Promise<Simulation>, engine: Engine, settings: Settings, plan: CapacityPlan, hooks: CapacityHooks): Promise<CapacityMeasurement> {
  validateCapacity(plan, settings);
  hooks.signal.throwIfAborted();
  const world = await factory(engine, settings, plan.maximum);
  const result: CapacityMeasurement = { engine, settings: { ...settings }, plan: { ...plan }, score: 0, outcome: 'limit', detail: null, levels: [], scene_fingerprint: world.fingerprint(), gpu: world.device?.() ?? null };
  let count = settings.count;
  try {
    if (!world.append) throw new Error('This adapter cannot add bodies.');
    const window = async (duration: number, phase: 'settling' | 'sampling'): Promise<CapacityWindow> => {
      const start = hooks.now(); let frames = 0;
      do {
        hooks.signal.throwIfAborted();
        const waiting = hooks.now(); await hooks.frame(); hooks.signal.throwIfAborted();
        if (hooks.now() - waiting > 2000) throw new Error('Browser frame delivery stalled. Keep the preview visible and run again.');
        const begin = hooks.now(); await world.advance(1);
        const physicsMs = hooks.now() - begin;
        hooks.signal.throwIfAborted();
        const snapshot = JSON.parse(world.snapshot()) as Snapshot;
        if (snapshot.bodies.length !== count) throw new Error('The engine returned a different body count.');
        if (snapshot.bodies.some(body => [...body.position, ...body.rotation].some(value => !Number.isFinite(value)) || (body.kind === 'dynamic' && body.position[1] < -.25))) throw new Error('Invalid poses or bodies below the floor.');
        hooks.render(snapshot, physicsMs, count, phase); frames++;
      } while (hooks.now() - start < duration);
      const elapsed_ms = hooks.now() - start;
      return { frames, elapsed_ms, fps: frames * 1000 / elapsed_ms };
    };
    for (;;) {
      await window(plan.settle_ms, 'settling');
      const level: CapacityLevel = { count, windows: [await window(plan.window_ms, 'sampling')] };
      const threshold = plan.target_fps * (1 - plan.tolerance_percent / 100);
      if (level.windows[0].fps < threshold) {
        level.windows.push(await window(plan.window_ms, 'sampling'));
        if (level.windows[1].fps < threshold) { result.levels.push(level); result.outcome = 'threshold'; break; }
      }
      result.levels.push(level);
      result.score = count;
      if (count === plan.maximum) break;
      const amount = Math.min(plan.increment, plan.maximum - count);
      hooks.signal.throwIfAborted(); await world.append(amount); count += amount;
    }
  } catch (cause) {
    hooks.signal.throwIfAborted();
    result.outcome = 'error'; result.detail = String(cause);
  } finally { world.free(); }
  return result;
}
