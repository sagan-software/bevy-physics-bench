/** One completed visible update. FPS includes scheduling, physics, and rendering. */
export interface LivePoint { time: number; fps: number; physics: number }

/** Bounded history; pausing breaks the interval instead of recording a false dip. */
export class Telemetry {
  private buffer: LivePoint[] = [];
  private cursor = 0;
  private previous?: number;
  get points(): LivePoint[] { return [...this.buffer.slice(this.cursor), ...this.buffer.slice(0, this.cursor)]; }
  frame(time: number, physics: number): void {
    if (this.previous !== undefined && time > this.previous) {
      this.buffer[this.cursor] = { time, fps: 1000 / (time - this.previous), physics };
      this.cursor = (this.cursor + 1) % 2400;
    }
    this.previous = time;
  }
  pause(): void { this.previous = undefined; }
  clear(): void { this.buffer = []; this.cursor = 0; this.pause(); }
}
