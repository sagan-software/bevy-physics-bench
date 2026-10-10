import type { Measurement, RunFile } from './results';

/** Canonicalize field order without changing values or the order of array elements. */
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    return `{${Object.entries(value).sort(([a], [b]) => a.localeCompare(b)).map(([key, child]) => `${JSON.stringify(key)}:${canonical(child)}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

/** Keep different configurations and integration paths out of the same comparison table. */
function measurementKey(measurement: Measurement): string {
  return canonical([measurement.settings, measurement.plan, measurement.scope, measurement.scene_fingerprint]);
}

export function groupMeasurements(measurements: Measurement[]): Measurement[][] {
  const groups = new Map<string, Measurement[]>();
  for (const measurement of measurements) {
    const key = measurementKey(measurement);
    const group = groups.get(key) ?? []; group.push(measurement); groups.set(key, group);
  }
  return [...groups.values()];
}

export interface HistoryPoint { run: RunFile; measurement: Measurement }
export interface HistorySeries { points: HistoryPoint[] }

/** Version history holds hardware, runtime, build profile, engine, and measurement inputs fixed. */
export function historySeries(runs: RunFile[]): HistorySeries[] {
  const groups = new Map<string, HistorySeries>();
  for (const run of runs) {
    const { notes: _notes, ...environment } = run.environment;
    for (const measurement of run.measurements) {
      const key = canonical([environment, run.build.compiler, run.build.rustflags, run.build.features, measurement.engine, measurementKey(measurement)]);
      const group = groups.get(key) ?? { points: [] };
      group.points.push({ run, measurement }); groups.set(key, group);
    }
  }
  for (const series of groups.values()) series.points.sort((a, b) => a.run.created_at.localeCompare(b.run.created_at));
  return [...groups.values()];
}
