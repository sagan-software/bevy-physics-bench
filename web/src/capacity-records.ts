import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../../capacity.schema.json' with { type: 'json' };
import { validateCapacity, type CapacityMeasurement } from './capacity';
import type { RunFile } from './results';

/** Shared result controls use exact labels, with `all` disabling one filter. */
export type CapacityFilters = Record<'runtime' | 'dimension' | 'scenario' | 'machine' | 'profile' | 'version', string>;

/** Select rows without changing the original record used by JSON export. */
export function capacityEntries(records: CapacityRecord[], filters?: CapacityFilters): { record: CapacityRecord; measurement: CapacityMeasurement }[] {
  const entries: { record: CapacityRecord; measurement: CapacityMeasurement }[] = [];
  const matches = (selected: string, actual: string) => selected === 'all' || selected === actual;
  for (const record of records) for (const measurement of record.measurements) {
    const profile = measurement.engine === 'nexus' ? 'gpu' : record.metadata.profile;
    if (!filters || (matches(filters.runtime, 'wasm') && matches(filters.dimension, measurement.settings.dimension)
      && matches(filters.scenario, measurement.settings.scenario) && matches(filters.machine, record.machine)
      && matches(filters.profile, profile) && matches(filters.version, record.metadata.versions.revision))) entries.push({ record, measurement });
  }
  return entries;
}

/** Separate wire format for rendered growth scores; fixed-scene v1 records remain unchanged. */
export interface CapacityRecord {
  kind: 'capacity-v1'; created_at: string; machine: string; browser: string; os: string; logical_cores: number;
  viewport: [number, number]; pixel_ratio: number;
  metadata: Pick<RunFile, 'build' | 'versions'> & { target: string; profile: RunFile['environment']['profile'] };
  measurements: CapacityMeasurement[];
}

const validate = new Ajv2020({ strict: true }).compile<CapacityRecord>(schema);

/** Recompute scores and reject incompatible or contradictory records before displaying them. */
export function validateCapacityRecord(value: unknown): CapacityRecord {
  if (!validate(value)) throw new Error('Invalid capacity record.');
  const date = new Date(value.created_at);
  if (!Number.isFinite(date.getTime()) || date.toISOString() !== value.created_at) throw new Error('Capacity date must use UTC milliseconds and Z.');
  if (/[\u0000-\u001f\u007f]/u.test(value.machine) || !value.machine.trim()) throw new Error('Invalid machine label.');
  if (value.metadata.target !== 'wasm32-unknown-unknown' || value.metadata.profile === 'native-tuned') throw new Error('Capacity records require a browser build.');
  const engines = new Set<string>();
  for (const result of value.measurements) {
    validateCapacity(result.plan, result.settings);
    if (engines.has(result.engine)) throw new Error('Duplicate capacity engine.'); engines.add(result.engine);
    if ((result.engine === 'jolt' || result.engine === 'nexus') && result.settings.dimension !== '3d') throw new Error('This engine requires 3D.');
    if (result.engine === 'nexus') {
      if (!result.gpu || result.settings.hz !== 60 || result.settings.solver_effort !== 4 || result.settings.sleeping !== 'disabled' || result.settings.ccd !== 'discrete') throw new Error('Unsupported Nexus growth profile.');
    } else if (result.gpu !== null) throw new Error('CPU capacity records cannot claim a compute device.');
    let count = result.settings.count, score = 0, failed = false;
    const threshold = result.plan.target_fps * (1 - result.plan.tolerance_percent / 100);
    for (const level of result.levels) {
      if (failed || count > result.plan.maximum || level.count !== count) throw new Error('Invalid capacity level order.');
      for (const window of level.windows) {
        if (window.elapsed_ms < result.plan.window_ms || Math.abs(window.fps - window.frames * 1000 / window.elapsed_ms) > 1e-8) throw new Error('Capacity FPS disagrees with its measured window.');
      }
      const firstFailed = level.windows[0].fps < threshold;
      if (firstFailed !== (level.windows.length === 2)) throw new Error('A missed window requires exactly one retry.');
      failed = firstFailed && level.windows[1].fps < threshold;
      if (!failed) score = count;
      count = count === result.plan.maximum ? count + 1 : Math.min(count + result.plan.increment, result.plan.maximum);
    }
    if (score !== result.score || (result.outcome === 'threshold') !== failed
      || (result.outcome === 'limit' && score !== result.plan.maximum)
      || (result.outcome === 'error' && score === result.plan.maximum)
      || (result.outcome === 'error' ? !result.detail : result.detail !== null)) throw new Error('Capacity outcome disagrees with measured levels.');
  }
  return value;
}
