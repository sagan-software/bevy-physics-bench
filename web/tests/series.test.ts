import { describe, expect, it } from 'vitest';
import { groupMeasurements, historySeries } from '../src/series';
import { record } from './record';

describe('result series boundaries', () => {
  it('separates settings and timing scopes while keeping comparable engines together', () => {
    const a = record().measurements[0];
    const b = { ...a, engine: 'rapier' as const };
    const c = { ...a, settings: { ...a.settings, count: 2 } };
    const d = { ...a, scope: 'binding-step' as const };
    expect(groupMeasurements([a, b, c, d]).map(group => group.length)).toEqual([2, 1, 1]);
  });
  it('orders history by time and separates machines, browsers, profiles and engine identities', () => {
    const first = record(); first.created_at = '2026-10-08T00:00:00.000Z';
    const next = structuredClone(first); next.created_at = '2026-10-09T00:00:00.000Z'; next.versions.avian = '0.8.0';
    const machine = structuredClone(first); machine.environment.machine = 'Second machine';
    const browser = structuredClone(first); browser.environment.browser = 'New browser';
    const profile = structuredClone(first); profile.environment.profile = 'native-tuned';
    const engine = structuredClone(first); engine.measurements[0].engine = 'rapier';
    const compiler = structuredClone(first); compiler.build.compiler = 'different compiler';
    const series = historySeries([next, machine, browser, profile, engine, compiler, first]);
    expect(series).toHaveLength(6);
    const avian = series.find(series => series.points.length === 2)!;
    expect(avian.points.map(point => point.run.created_at)).toEqual([first.created_at, next.created_at]);
    expect(avian.points[1].run.versions.avian).toBe('0.8.0');
  });
});
