import type { Engine, Settings, SimulationFactory, Snapshot } from './types';
import { health, summarize, validateSettings, type Measurement, type Plan } from './results';

interface Options {
  device?: (metadata: { name: string; backend: string; driver: string | null }) => void;
  signal?: AbortSignal;
  clock?: () => number;
  yieldControl?: () => Promise<void>;
  progress?: (completed: number, total: number, engine: Engine) => void;
}
export const DEFAULT_PLAN: Plan = { warmup_steps: 60, samples: 30, batch_steps: 4, repeats: 3 };

/** Yield through the task queue without the nested-timer minimum delay. */
export function yieldTask(): Promise<void> {
  return new Promise(resolve => {
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      channel.port1.close(); channel.port2.close(); resolve();
    };
    channel.port2.postMessage(null);
  });
}

/** Run isolated engine worlds, rotating engine order between fresh repetitions. */
export async function runComparison(factory: SimulationFactory, engines: Engine[], settings: Settings,
  plan: Plan, options: Options = {}): Promise<Measurement[]> {
  validateSettings(settings);
  const { signal, clock = () => performance.now(),
    yieldControl = yieldTask, progress } = options;
  for (const [key, minimum, maximum] of [['warmup_steps', 1, 600], ['samples', 5, 1000],
    ['batch_steps', 1, 60], ['repeats', 1, 9]] as const) {
    if (!Number.isInteger(plan[key]) || plan[key] < minimum || plan[key] > maximum) {
      throw new Error(`${key} must be an integer between ${minimum} and ${maximum}.`);
    }
  }
  if (!engines.length || new Set(engines).size !== engines.length) throw new Error('Choose distinct engines.');
  const checkCanceled = () => { if (signal?.aborted) throw new Error('Comparison canceled.'); };
  const samples = new Map(engines.map(engine => [engine, [] as number[]]));
  const fingerprints = new Map<Engine, string>();
  const readbacks = new Map(engines.map(engine => [engine, [] as number[]]));
  let fingerprint: string | undefined;
  const states = new Map<Engine, ReturnType<typeof health>>();
  const total = engines.length * plan.repeats * plan.samples;
  let completed = 0;
  for (let repeat = 0; repeat < plan.repeats; repeat++) {
    const order = repeat % 2 ? [...engines].reverse() : engines;
    for (const engine of order) {
      checkCanceled();
      const simulation = await factory(engine, { ...settings });
      try {
        checkCanceled();
        const currentFingerprint = simulation.fingerprint();
        if (!/^[0-9a-f]{64}$/.test(currentFingerprint) || (fingerprint && fingerprint !== currentFingerprint)) throw new Error('The engines generated different initial scenes.');
        fingerprint = currentFingerprint; fingerprints.set(engine, currentFingerprint);
        if (engine === 'nexus') {
          if (!simulation.device || !simulation.readbackMs) throw new Error('The GPU adapter did not provide measurement metadata.');
          options.device?.(simulation.device());
        }
        // Warmup is bounded too, so a heavy scene can still be canceled.
        for (let remaining = plan.warmup_steps; remaining > 0; remaining -= 4) {
          const pending = simulation.advance(Math.min(4, remaining));
          if (pending) await pending;
          await yieldControl(); checkCanceled();
        }
        for (let sample = 0; sample < plan.samples; sample++) {
          checkCanceled();
          const start = clock();
          const pending = simulation.advance(plan.batch_steps);
          if (pending) await pending;
          const duration = (clock() - start) / plan.batch_steps;
          if (!Number.isFinite(duration) || duration < 0) throw new Error('The browser returned an invalid timing sample.');
          if (engine === 'nexus') {
            const readback = simulation.readbackMs!() / plan.batch_steps;
            if (!Number.isFinite(readback) || readback < 0 || readback > duration + .01) throw new Error('The GPU adapter returned an invalid readback duration.');
            readbacks.get(engine)!.push(readback);
          }
          samples.get(engine)!.push(duration);
          progress?.(++completed, total, engine);
          await yieldControl();
        }
        checkCanceled();
        const state = health(JSON.parse(simulation.snapshot()) as Snapshot);
        const previous = states.get(engine);
        // Preserve a failed earlier repetition rather than reporting only the last one.
        states.set(engine, { ...state, finite: state.finite && (previous?.finite ?? true),
          min_y: state.min_y === null ? previous?.min_y ?? null : Math.min(state.min_y, previous?.min_y ?? Infinity) });
      } finally { simulation.free(); }
    }
  }
  return engines.map(engine => ({ engine, settings: { ...settings }, plan: { ...plan },
    scene_fingerprint: fingerprints.get(engine)!, scope: engine === 'bevy' ? 'bevy-animation' : engine === 'jolt' ? 'binding-step' : engine === 'nexus' ? 'gpu-step-readback' : 'bevy-step', readback_ms: engine === 'nexus' ? readbacks.get(engine)! : null,
    samples_ms: samples.get(engine)!, statistics: summarize(samples.get(engine)!), ...states.get(engine)! }));
}

/** Replacing a run revokes its ability to publish a late result. */
export class RunCoordinator<T> {
  private current?: AbortController;
  cancel(): void { this.current?.abort(); this.current = undefined; }
  async start(run: (signal: AbortSignal) => Promise<T>, publish: (result: T) => void): Promise<void> {
    this.cancel();
    const controller = new AbortController(); this.current = controller;
    const result = await run(controller.signal);
    if (this.current === controller && !controller.signal.aborted) publish(result);
  }
}
