import { describe, expect, it } from 'vitest';
import { Telemetry } from '../src/telemetry';

describe('live telemetry', () => {
  it('retains dips in a bounded time series and measures completed frames', () => {
    const history = new Telemetry();
    history.frame(0, 2);
    history.frame(20, 3);
    history.frame(60, 12);
    expect(history.points).toEqual([{ time: 20, fps: 50, physics: 3 }, { time: 60, fps: 25, physics: 12 }]);
    history.pause(); history.frame(10000, 0);
    expect(history.points).toHaveLength(2);
    history.frame(10020, 4);
    expect(history.points.at(-1)?.fps).toBe(50);
  });
  it('bounds history, ignores duplicate timestamps, and clears on a new scene', () => {
    const history = new Telemetry();
    history.frame(0, 1); history.frame(0, 1);
    for (let index = 1; index <= 2500; index++) history.frame(index * 10, 2);
    expect(history.points).toHaveLength(2400);
    expect(history.points[0].time).toBe(1010);
    expect(history.points.at(-1)?.time).toBe(25000);
    history.clear(); expect(history.points).toEqual([]);
    history.frame(30000, 1); expect(history.points).toEqual([]);
  });
});
