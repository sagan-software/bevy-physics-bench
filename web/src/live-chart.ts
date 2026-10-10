import { Telemetry } from './telemetry';

/** Render ten seconds of measured updates with a continuously moving time axis. */
export class LiveChart {
  readonly history = new Telemetry();
  private canvas = document.createElement('canvas');
  private value = document.createElement('output');
  private select = document.createElement('select');
  private context: CanvasRenderingContext2D;
  private color = '#7cdbbc';
  constructor(mount: HTMLElement, side: string) {
    this.select.setAttribute('aria-label', `Chart metric ${side}`);
    for (const [value, text] of [['fps', 'FPS'], ['physics', 'Physics ms']]) { const option = document.createElement('option'); option.value = value; option.textContent = text; this.select.append(option); }
    const header = document.createElement('div'); header.className = 'chart-heading'; header.append(this.select, this.value);
    this.canvas.setAttribute('role', 'img'); this.canvas.setAttribute('aria-label', `${side} performance over the last ten seconds`);
    mount.append(header, this.canvas);
    this.context = this.canvas.getContext('2d')!;
    this.select.addEventListener('change', () => this.draw(this.history.points.at(-1)?.time ?? performance.now()));
  }
  reset(color: string): void { this.color = color; this.history.clear(); this.draw(performance.now()); }
  draw(now: number): void {
    const width = this.canvas.clientWidth, height = 84, ratio = Math.min(devicePixelRatio, 2);
    const pixelWidth = Math.round(width * ratio), pixelHeight = Math.round(height * ratio);
    if (this.canvas.width !== pixelWidth || this.canvas.height !== pixelHeight) { this.canvas.width = pixelWidth; this.canvas.height = pixelHeight; }
    const context = this.context; context.setTransform(ratio, 0, 0, ratio, 0, 0); context.clearRect(0, 0, width, height);
    const points = this.history.points.filter(point => point.time >= now - 10000);
    const metric = this.select.value as 'fps' | 'physics';
    const ceiling = metric === 'fps' ? Math.max(60, ...points.map(point => point.fps)) : Math.max(16.67, ...points.map(point => point.physics));
    const left = 32, right = Math.max(left, width - 8), bottom = 65, top = 7;
    const y = (value: number) => bottom - value / ceiling * (bottom - top);
    context.strokeStyle = '#3d4f5a'; context.lineWidth = 1;
    context.beginPath(); context.moveTo(left, bottom); context.lineTo(right, bottom); context.moveTo(left, y(metric === 'fps' ? 60 : 16.67)); context.lineTo(right, y(metric === 'fps' ? 60 : 16.67)); context.stroke();
    context.font = '10px system-ui'; context.fillStyle = '#c5d3db'; context.fillText(metric === 'fps' ? '60' : '16.7', 2, y(metric === 'fps' ? 60 : 16.67) + 4); context.fillText('−10 s', left, 80); context.fillText('now', right - 23, 80);
    if (!points.length) { this.value.textContent = '—'; return; }
    const x = (time: number) => left + (time - now + 10000) / 10000 * (right - left);
    context.beginPath(); context.moveTo(x(points[0].time), bottom);
    for (const point of points) context.lineTo(x(point.time), y(point[metric]));
    context.lineTo(x(points.at(-1)!.time), bottom); context.closePath(); context.fillStyle = `${this.color}40`; context.fill();
    context.beginPath(); points.forEach((point, index) => { if (index) context.lineTo(x(point.time), y(point[metric])); else context.moveTo(x(point.time), y(point[metric])); });
    context.strokeStyle = this.color; context.stroke();
    const recent = points.filter(point => point.time >= now - 500);
    const value = recent.length ? recent.reduce((sum, point) => sum + point[metric], 0) / recent.length : points.at(-1)![metric];
    this.value.textContent = `${value.toFixed(metric === 'fps' ? 0 : 2)} ${metric === 'fps' ? 'FPS' : 'ms'}`;
  }
}
