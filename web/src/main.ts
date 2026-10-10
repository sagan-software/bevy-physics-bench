import './style.css';
import { DEFAULT_SETTINGS, ENGINES, SCENARIOS, type Engine, type Settings, type Simulation, type Snapshot } from './types';
import { createSimulation, runtimeMetadata, floorExtents, gpuAvailability } from './wasm';
import { machineName } from './machine';
import { groupMeasurements, historySeries } from './series';
import { PhysicsView } from './view';
import { DEFAULT_PLAN, RunCoordinator, runComparison } from './benchmark';
import { comparable, validateRun, type Measurement, type Plan, type RunFile } from './results';

const element = <T extends HTMLElement>(id: string): T => {
  const found = document.getElementById(id); if (!found) throw new Error(`Missing control: ${id}`); return found as T;
};
const field = (name: string) => element<HTMLFormElement>('settings').elements.namedItem(name) as HTMLInputElement | HTMLSelectElement;
const status = (text: string) => { element('run-status').textContent = text; };
const error = (text: string) => { element('error').textContent = text; element('error').hidden = !text; };
let settings = { ...DEFAULT_SETTINGS };
let simulations: Simulation[] = [];
let views: PhysicsView[] = [];
let snapshots: Snapshot[] = [];
let isPlaying = true;
let isMeasuring = false;
let generation = 0;
let runGeneration = 0;
let isLoading = false;
let isAdvancing = false;
let stepping: Promise<void> = Promise.resolve();
let hasGpu = false;
let latest: RunFile | undefined;
let published: RunFile[] = [];
const runs = new RunCoordinator<Measurement[]>();
const selectedEngines = (): Engine[] => [element<HTMLSelectElement>('engine-left').value as Engine, element<HTMLSelectElement>('engine-right').value as Engine];
let versions: RunFile['versions'] = { bevy: '0.19.1', avian: '0.7.0', bevy_rapier: '0.36.0', rapier: '0.35.0-glamx0.2', oxijolt: '1.0.1+jolt-5.6.0', jolt_js: '1.1.0', nexus: '0.6.0', revision: 'working-tree' };

function storedRuns(): RunFile[] {
  try {
    const records: unknown = JSON.parse(localStorage.getItem('bevy-physics-runs-v1') ?? '[]');
    if (!Array.isArray(records)) return [];
    return records.slice(0, 30).flatMap(record => { try { return [validateRun(record)]; } catch { return []; } });
  } catch { return []; }
}
function saveRun(run: RunFile): void {
  try { localStorage.setItem('bevy-physics-runs-v1', JSON.stringify([run, ...storedRuns()].slice(0, 30))); } catch { /* Export remains available when browser storage is full or disabled. */ }
}
function setPlaying(value: boolean): void {
  isPlaying = value; element('play').textContent = value ? 'Pause' : 'Play';
  element('play').setAttribute('aria-pressed', String(value));
}
function fillSettings(): void {
  for (const [name, value] of Object.entries(settings)) {
    const control = element<HTMLFormElement>('settings').elements.namedItem(name) as HTMLInputElement | null;
    if (control) control.value = String(value);
  }
  element<HTMLSelectElement>('dimension').value = settings.dimension;
  element<HTMLSelectElement>('mobile-scene').value = settings.scenario;
  const scenario = SCENARIOS.find(item => item.id === settings.scenario)!;
  element('scene-title').textContent = scenario.name; element('scene-description').textContent = scenario.description;
  document.querySelectorAll<HTMLButtonElement>('[data-scene]').forEach(button => button.setAttribute('aria-current', String(button.dataset.scene === settings.scenario)));
  document.querySelector<HTMLElement>('.camera-help')!.textContent = settings.dimension === '2d' ? 'Drag to pan · Scroll to zoom' : 'Drag to orbit · Scroll to zoom';
}
function readSettings(): Settings {
  return { ...settings, dimension: element<HTMLSelectElement>('dimension').value as Settings['dimension'],
    count: Number(field('count').value), seed: Number(field('seed').value), hz: Number(field('hz').value),
    gravity: Number(field('gravity').value), friction: Number(field('friction').value), restitution: Number(field('restitution').value),
    solver_effort: Number(field('solver_effort').value), sleeping: field('sleeping').value as Settings['sleeping'], ccd: field('ccd').value as Settings['ccd'] };
}
function clearMeasurement(): void {
  latest = undefined; element<HTMLButtonElement>('export').disabled = true;
  element('comparison').replaceChildren(Object.assign(document.createElement('p'), { className: 'empty', textContent: 'No measured run for these settings.' }));
}
function stopRun(): void {
  runGeneration++; runs.cancel(); isMeasuring = false; element('cancel').hidden = true; element('progress').hidden = true;
  element('benchmark').hidden = false;
}
async function reset(): Promise<void> {
  stopRun(); const request = ++generation; isLoading = true; error(''); clearMeasurement();
  const configuredSettings = { ...settings };
  const isIk = settings.scenario === 'inverse-kinematics';
  const canGpu = hasGpu && settings.dimension === '3d' && !['kinematics', 'ccd', 'queries', 'characters', 'inverse-kinematics'].includes(settings.scenario) && settings.sleeping === 'disabled' && settings.ccd === 'discrete';
  document.querySelector<HTMLElement>('.views')!.dataset.single = String(isIk);
  for (const side of ['left', 'right']) {
    const select = element<HTMLSelectElement>(`engine-${side}`);
    for (const option of select.options) option.disabled = isIk ? option.value !== 'bevy'
      : option.value === 'bevy' || (option.value === 'jolt' && settings.dimension === '2d') || (option.value === 'nexus' && !canGpu);
    if (isIk) select.value = 'bevy';
    else if (select.selectedOptions[0]?.disabled) select.value = side === 'left' ? 'avian' : 'rapier';
  }
  element<HTMLButtonElement>('benchmark').disabled = true;
  const engines = selectedEngines();
  status('Loading the configured scene…');
  let pending: Simulation[] = [];
  try {
    // Construct replacements before releasing the visible worlds. A stale load cannot replace them.
    await stepping;
    if (request !== generation) return;
    const floor = await floorExtents(configuredSettings);
    for (const engine of engines) pending.push(await createSimulation(engine, configuredSettings));
    if (request !== generation) { pending.forEach(simulation => simulation.free()); return; }
    simulations.forEach(simulation => simulation.free()); simulations = pending; pending = [];
    snapshots = simulations.map(simulation => JSON.parse(simulation.snapshot()) as Snapshot);
    const sides = ['left', 'right'];
    if (!views.length) {
      views = sides.map((side, index) => {
        const stage = element(`stage-${side}`); stage.replaceChildren();
        return new PhysicsView(stage, ENGINES[engines[index]].color, `${ENGINES[engines[index]].name} physics simulation`);
      });
      views[0].onCameraChange = () => views[1].syncCamera(views[0]);
      views[1].onCameraChange = () => views[0].syncCamera(views[1]);
    }
    views.forEach((view, index) => {
      view.reset(snapshots[index], configuredSettings.dimension, ENGINES[engines[index]].color, floor);
      view.canvas.setAttribute('aria-label', `${ENGINES[engines[index]].name} ${settings.dimension} ${settings.scenario} simulation`);
      element(`version-${sides[index]}`).textContent = ENGINES[engines[index]].version;
      document.querySelector(`#engine-${sides[index]}`)!.closest('label')!.querySelector('.engine-dot')!.className = `engine-dot ${engines[index]}`;
      element(`live-${sides[index]}`).textContent = 'Ready';
    });
    element('tick').textContent = 'Tick 0';
    element<HTMLButtonElement>('benchmark').disabled = !isIk && engines[0] === engines[1];
    status(isIk ? 'Measure Bevy animation work with the current joint count.' : engines[0] === engines[1] ? 'Choose different engines to run a comparison.' : 'Run both engines from the same initial scene. Playback pauses during measurement.');
    element('timing-note').textContent = isIk ? 'Bevy animation time · Rendering excluded' : engines.includes('nexus') ? 'GPU completion and readback included · Rendering excluded' : engines.includes('jolt') ? 'Separate Bevy and Jolt binding timings · Rendering excluded' : 'Bevy integration time · Rendering excluded';
    fillSettings(); writeLink();
  } catch (cause) {
    pending.forEach(simulation => simulation.free());
    if (request !== generation) return;
    setPlaying(false); error(`Could not load the scene. ${String(cause)} Use Reset to retry.`); status('Scene loading failed.');
  } finally { if (request === generation) isLoading = false; }
}
function writeLink(): void {
  const parameters = new URLSearchParams();
  for (const [key, value] of Object.entries(settings)) parameters.set(key, String(value));
  const engines = selectedEngines(); parameters.set('a', engines[0]); parameters.set('b', engines[1]);
  const route = location.hash.split('?')[0] || '#playground';
  history.replaceState(null, '', `${location.pathname}${route}?${parameters}`);
}
function readLink(): void {
  const parameters = new URLSearchParams(location.hash.split('?')[1]);
  for (const key of Object.keys(settings) as (keyof Settings)[]) {
    const value = parameters.get(key);
    if (value !== null) (settings as unknown as Record<string, unknown>)[key] = typeof DEFAULT_SETTINGS[key] === 'number' ? Number(value) : value;
  }
  if (!SCENARIOS.some(item => item.id === settings.scenario)) settings.scenario = DEFAULT_SETTINGS.scenario;
  if (!['2d', '3d'].includes(settings.dimension)) settings.dimension = DEFAULT_SETTINGS.dimension;
  for (const [key, side] of [['a', 'left'], ['b', 'right']]) {
    const value = parameters.get(key); if (value && Object.hasOwn(ENGINES, value)) element<HTMLSelectElement>(`engine-${side}`).value = value;
  }
}

/** Build result cells with text nodes so submitted machine names cannot become markup. */
function measurementTable(measurements: Measurement[]): HTMLTableElement {
  const table = document.createElement('table'); table.className = 'results-table';
  const header = table.createTHead().insertRow();
  for (const label of ['Engine', 'Median', 'P95', 'Variation', 'Awake', 'State']) { const th = document.createElement('th'); th.scope = 'col'; th.textContent = label; header.append(th); }
  const maximum = Math.max(...measurements.map(measurement => measurement.statistics.median_ms), .001);
  for (const measurement of measurements) {
    const row = table.createTBody().insertRow(); const summary = measurement.statistics;
    const values = [ENGINES[measurement.engine].name, `${summary.median_ms.toFixed(3)} ms`, `${summary.p95_ms.toFixed(3)} ms`, `${summary.cv_percent.toFixed(1)}%`, String(measurement.awake_bodies), !measurement.finite ? 'Invalid poses' : measurement.min_y !== null && measurement.min_y < -.25 ? 'Below floor' : 'Finite'];
    values.forEach((value, index) => {
      const cell = row.insertCell(); cell.textContent = value;
      if (index === 1) { const bar = document.createElement('div'); bar.className = `time-bar ${measurement.engine}`; bar.style.width = `${summary.median_ms / maximum * 100}%`; cell.append(bar); }
    });
  }
  return table;
}
function showComparison(measurements: Measurement[]): void {
  const container = element('comparison'); container.replaceChildren(measurementTable(measurements));
  const note = document.createElement('p'); note.className = 'comparison-note';
  const [first, second] = measurements;
  if (measurements.some(measurement => !measurement.finite || (measurement.min_y !== null && measurement.min_y < -.25))) {
    note.textContent = 'A run produced invalid poses or bodies below the floor. Inspect the simulation before comparing speed.';
  } else if (first && second && comparable(first, second) && Math.min(first.statistics.median_ms, second.statistics.median_ms) > 0) {
    const faster = first.statistics.median_ms < second.statistics.median_ms ? first : second;
    const slower = faster === first ? second : first;
    const ratio = slower.statistics.median_ms / faster.statistics.median_ms;
    note.textContent = `${ENGINES[faster.engine].name} used ${(100 * (1 - 1 / ratio)).toFixed(1)}% less median step time in this run. ${first.samples_ms.length} samples per engine.`;
  } else if (second && !comparable(first, second)) note.textContent = 'These adapters measure different integration paths. Timings are shown separately.';
  else if (!second) note.textContent = `${first.samples_ms.length} samples. Bevy-only animation work; no physics-engine ranking.`;
  else note.textContent = 'Timer resolution is insufficient for a ratio. Increase steps per sample.';
  container.append(note);
}
async function measure(): Promise<void> {
  if (isLoading || !element<HTMLFormElement>('settings').reportValidity()) return;
  const request = ++runGeneration;
  isMeasuring = true; setPlaying(false); error('');
  element('benchmark').hidden = true; element('cancel').hidden = false; element('progress').hidden = false;
  const plan = Object.fromEntries(Object.keys(DEFAULT_PLAN).map(key => [key, Number(field(key).value)])) as unknown as Plan;
  const capturedSettings = { ...settings }; const engines = [...new Set(selectedEngines())];
  try {
    await stepping;
    const metadata = await runtimeMetadata();
    let gpu: RunFile['environment']['gpu'] = null;
    if (request !== runGeneration) return;
    versions = metadata.versions;
    await runs.start(signal => runComparison(createSimulation, engines, capturedSettings, plan, {
      signal, device: metadata => {
        if (gpu && JSON.stringify(gpu) !== JSON.stringify(metadata)) throw new Error('The GPU adapter changed between repetitions.');
        gpu = metadata;
      }, progress: (completed, total, engine) => {
        if (request !== runGeneration) return;
        element<HTMLProgressElement>('progress').value = completed / total;
        status(`${ENGINES[engine].name}: sample ${completed} of ${total}. Playback is paused.`);
      },
    }), measurements => {
      latest = { schema_version: 1, created_at: new Date().toISOString(), source: 'local', versions, build: { ...metadata.build, features: [...metadata.build.features, ...(engines.includes('nexus') ? ['nexus-webgpu'] : [])] },
        environment: { runtime: 'wasm', machine: field('machine').value.trim(), cpu: null, logical_cores: navigator.hardwareConcurrency || 1,
          memory_bytes: null, os: navigator.platform, browser: navigator.userAgent, target: metadata.target,
          profile: engines.includes('nexus') ? 'gpu' : metadata.profile, gpu, threads: 1, notes: 'Interactive browser; CPU model and physical RAM are not exposed.' }, measurements };
      validateRun(latest); saveRun(latest); showComparison(measurements); element<HTMLButtonElement>('export').disabled = false;
      status(`Complete. ${plan.repeats} fresh runs per engine; ${plan.warmup_steps} warmup steps per run; ${plan.batch_steps} steps per sample.`);
    });
  } catch (cause) {
    if (request === runGeneration && isMeasuring) { error(`Comparison failed. ${String(cause)}`); status('No result was published.'); }
  } finally { if (request === runGeneration && isMeasuring) stopRun(); }
}
function download(run: RunFile): void {
  const blob = new Blob([`${JSON.stringify(run, null, 2)}\n`], { type: 'application/json' });
  const url = URL.createObjectURL(blob); const anchor = document.createElement('a'); anchor.href = url;
  anchor.download = `bevy-physics-${run.environment.runtime}-${run.created_at.replace(/[:.]/g, '-')}.json`;
  anchor.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
}
async function loadResults(): Promise<void> {
  const label = element('results-status'); label.textContent = 'Loading published results…';
  try {
    const response = await fetch(`${import.meta.env.BASE_URL}results/index.json`, { cache: 'no-cache' });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const files: unknown = await response.json();
    if (!Array.isArray(files) || files.length > 5000 || new Set(files).size !== files.length || files.some(file => typeof file !== 'string')) throw new Error('Invalid result index.');
    published = await Promise.all(files.map(async file => {
      if (!/^[a-z0-9][a-z0-9.-]*\.json$/.test(file)) throw new Error('Invalid result filename.');
      const response = await fetch(`${import.meta.env.BASE_URL}results/${file}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const run = validateRun(await response.json());
      return { ...run, source: 'published' } as RunFile;
    }));
    label.textContent = `${published.length} published runs. ${storedRuns().length} runs stored on this device.`;
  } catch (cause) { label.textContent = `Published results could not be loaded: ${String(cause)}. Use Refresh to retry.`; }
  updateFilters(); renderResults();
}
function updateFilters(): void {
  const all = [...storedRuns(), ...published];
  for (const [id, values] of [
    ['filter-machine', all.map(run => run.environment.machine)],
    ['filter-version', all.map(run => run.versions.revision)],
  ] as [string, string[]][]) {
    const select = element<HTMLSelectElement>(id); const selected = select.value;
    while (select.options.length > 1) select.remove(1);
    for (const value of [...new Set(values)].sort()) select.add(new Option(id === 'filter-version' ? value.slice(0, 12) : value, value));
    select.value = [...select.options].some(option => option.value === selected) ? selected : 'all';
  }
}
function filteredRuns(): RunFile[] {
  const filters = Object.fromEntries(['runtime', 'dimension', 'scenario', 'machine', 'profile', 'version'].map(key => [key, element<HTMLSelectElement>(`filter-${key}`).value]));
  return [...storedRuns(), ...published].filter(run =>
    (filters.runtime === 'all' || run.environment.runtime === filters.runtime) &&
    (filters.machine === 'all' || run.environment.machine === filters.machine) &&
    (filters.profile === 'all' || run.environment.profile === filters.profile) &&
    (filters.version === 'all' || run.versions.revision === filters.version)
  ).map(run => ({ ...run, measurements: run.measurements.filter(measurement =>
    (filters.dimension === 'all' || measurement.settings.dimension === filters.dimension) &&
    (filters.scenario === 'all' || measurement.settings.scenario === filters.scenario)) }))
    .filter(run => run.measurements.length).sort((a, b) => b.created_at.localeCompare(a.created_at));
}
function renderResults(): void {
  const root = element('results-list'); root.replaceChildren();
  const all = filteredRuns();
  const history = element('history-list'); history.replaceChildren();
  for (const series of historySeries(all).filter(series => series.points.length > 1)) {
    const first = series.points[0]; const last = series.points.at(-1)!;
    const details = document.createElement('details');
    const summary = document.createElement('summary');
    summary.textContent = `${ENGINES[first.measurement.engine].name} · ${first.measurement.settings.scenario} · ${first.measurement.settings.dimension} · ${first.run.environment.machine} · ${series.points.length} runs`;
    const chart = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    chart.setAttribute('viewBox', '0 0 600 150'); chart.setAttribute('role', 'img'); chart.classList.add('history-chart');
    const chartTitle = document.createElementNS(chart.namespaceURI, 'title'); chartTitle.textContent = 'Median step time by collection date; lower is faster'; chart.append(chartTitle);
    const maximum = Math.max(...series.points.map(point => point.measurement.statistics.median_ms), .001);
    const points = series.points.map((point, index) => [20 + index / (series.points.length - 1) * 560, 130 - point.measurement.statistics.median_ms / maximum * 110]);
    const path = document.createElementNS(chart.namespaceURI, 'polyline'); path.setAttribute('points', points.map(point => point.join(',')).join(' ')); path.setAttribute('fill', 'none'); path.setAttribute('stroke', ENGINES[first.measurement.engine].color); path.setAttribute('stroke-width', '2'); chart.append(path);
    const list = document.createElement('ol'); list.className = 'history-points';
    series.points.forEach((point, index) => {
      const circle = document.createElementNS(chart.namespaceURI, 'circle'); circle.setAttribute('cx', String(points[index][0])); circle.setAttribute('cy', String(points[index][1])); circle.setAttribute('r', '4'); circle.setAttribute('fill', ENGINES[first.measurement.engine].color); chart.append(circle);
      const item = document.createElement('li'); item.textContent = `${point.run.created_at.slice(0, 10)} · ${point.measurement.statistics.median_ms.toFixed(3)} ms · ${point.run.versions.revision.slice(0, 12)}`; list.append(item);
    });
    const caption = document.createElement('p'); caption.className = 'run-meta'; caption.textContent = `${first.run.created_at.slice(0, 10)} to ${last.run.created_at.slice(0, 10)} · ${first.measurement.settings.count} bodies · ${first.run.environment.profile} · ${first.measurement.scope}`;
    details.append(summary, caption, chart, list); history.append(details);
  }
  element('history').hidden = !history.childElementCount;
  for (const run of all) {
    for (const group of groupMeasurements(run.measurements)) {
      const measurement = group[0]; const scene = SCENARIOS.find(scene => scene.id === measurement.settings.scenario)!;
      const article = document.createElement('article'); article.className = 'result-run';
      const heading = document.createElement('div'); heading.className = 'run-heading';
      const title = document.createElement('h2'); title.textContent = `${scene.name} · ${measurement.settings.dimension.toUpperCase()} · ${run.environment.machine}`;
      const exportButton = document.createElement('button'); exportButton.textContent = 'JSON'; exportButton.setAttribute('aria-label', `Export ${title.textContent}`); exportButton.addEventListener('click', () => download(run)); heading.append(title, exportButton);
      const meta = document.createElement('p'); meta.className = 'run-meta';
      meta.textContent = `${new Date(run.created_at).toLocaleString()} · ${run.environment.runtime} · ${run.environment.profile} · ${measurement.settings.count} bodies · ${measurement.scope} · ${run.source}`;
      const scroll = document.createElement('div'); scroll.className = 'table-scroll'; scroll.append(measurementTable(group));
      const details = document.createElement('details'); const summary = document.createElement('summary'); summary.textContent = 'Hardware, versions, and settings';
      const machine = document.createElement('p'); machine.textContent = `${run.environment.cpu ?? 'CPU model unavailable'} · ${run.environment.logical_cores} logical cores · ${run.environment.memory_bytes === null ? 'RAM unavailable' : (run.environment.memory_bytes / 1073741824).toFixed(1) + ' GiB RAM'} · ${run.environment.os}`;
      const data = document.createElement('pre'); data.textContent = JSON.stringify({ environment: run.environment, build: run.build, versions: run.versions, settings: measurement.settings, plan: measurement.plan, scene_fingerprint: measurement.scene_fingerprint, queries: measurement.queries }, null, 2);
      details.append(summary, machine, data); article.append(heading, meta, scroll, details); root.append(article);
    }
  }
  if (!root.childElementCount) root.append(Object.assign(document.createElement('p'), { textContent: 'No runs match these filters. Run a comparison or choose different filters.' }));
}
function showTab(): void {
  const route = location.hash.split('?')[0].slice(1); const tab = ['results', 'method'].includes(route) ? route : 'playground';
  for (const id of ['playground', 'results', 'method']) element(id).hidden = id !== tab;
  document.querySelectorAll<HTMLAnchorElement>('[data-tab]').forEach(link => {
    if (link.dataset.tab === tab) link.setAttribute('aria-current', 'page'); else link.removeAttribute('aria-current');
  });
  if (tab !== 'playground') setPlaying(false);
  if (tab === 'results') void loadResults();
}

/** Complete visible steps before replacing or releasing either owned world. */
async function advanceVisible(steps: number, request: number): Promise<void> {
  const current = simulations;
  try {
    for (const [index, simulation] of current.entries()) {
      const start = performance.now();
      const pending = simulation.advance(steps); if (pending) await pending;
      if (request !== generation) return;
      const milliseconds = (performance.now() - start) / steps;
      snapshots[index] = JSON.parse(simulation.snapshot()); views[index].update(snapshots[index]);
      const side = index === 0 ? 'left' : 'right';
      element(`live-${side}`).textContent = `Live step ${milliseconds.toFixed(2)} ms`;
      element(`awake-${side}`).textContent = settings.scenario === 'queries' ? `${snapshots[index].queries?.hits ?? 0} / ${snapshots[index].queries?.rays ?? 0} hits` : `${snapshots[index].bodies.filter(body => body.kind === 'dynamic' && !body.sleeping).length} awake`;
    }
    if (request === generation) element('tick').textContent = `Tick ${snapshots[0]?.tick ?? 0}`;
  } catch (cause) {
    if (request === generation) { setPlaying(false); error(`Simulation stopped. ${String(cause)} Use Reset to retry.`); }
  }
}
let lastFrame = performance.now(), accumulated = 0;
function frame(now: number): void {
  const elapsed = Math.min(.1, (now - lastFrame) / 1000); lastFrame = now;
  if (isPlaying && !isLoading && !isAdvancing && !isMeasuring && simulations.length === 2 && !document.hidden && !element('playground').hidden) {
    accumulated += elapsed; const steps = Math.min(4, Math.floor(accumulated * settings.hz));
    if (steps) {
      accumulated = Math.max(0, accumulated - steps / settings.hz);
      isAdvancing = true;
      stepping = advanceVisible(steps, generation).finally(() => { isAdvancing = false; });
    }
  } else accumulated = 0;
  requestAnimationFrame(frame);
}

let currentGroup = '';
for (const scene of SCENARIOS) {
  if (scene.group !== currentGroup) { const heading = document.createElement('h3'); heading.className = 'scene-group'; heading.textContent = scene.group; element('scenes').append(heading); currentGroup = scene.group; }
  const button = document.createElement('button'); button.textContent = scene.name; button.dataset.scene = scene.id;
  button.addEventListener('click', () => { settings.scenario = scene.id; fillSettings(); void reset(); }); element('scenes').append(button);
  for (const id of ['mobile-scene', 'filter-scenario']) { const option = document.createElement('option'); option.value = scene.id; option.textContent = scene.name; element<HTMLSelectElement>(id).append(option); }
}
element('settings').addEventListener('submit', event => { event.preventDefault(); settings = readSettings(); void reset(); });
element('defaults').addEventListener('click', () => { settings = { ...DEFAULT_SETTINGS, scenario: settings.scenario, dimension: settings.dimension }; fillSettings(); void reset(); });
element('dimension').addEventListener('change', () => { settings = readSettings(); void reset(); });
element('mobile-scene').addEventListener('change', () => { settings.scenario = element<HTMLSelectElement>('mobile-scene').value as Settings['scenario']; fillSettings(); void reset(); });
for (const side of ['left', 'right']) element(`engine-${side}`).addEventListener('change', () => { void reset(); });
element('play').addEventListener('click', () => { if (!isMeasuring) setPlaying(!isPlaying); });
element('reset').addEventListener('click', () => { void reset(); });
element('step').addEventListener('click', () => {
  if (isLoading || isMeasuring || isAdvancing) return;
  setPlaying(false); isAdvancing = true;
  stepping = advanceVisible(1, generation).finally(() => { isAdvancing = false; });
});
element('benchmark').addEventListener('click', () => { void measure(); });
element('cancel').addEventListener('click', () => { stopRun(); status('Comparison canceled. No partial result was saved.'); });
element('export').addEventListener('click', () => { if (latest) download(latest); });
element('refresh-results').addEventListener('click', () => { void loadResults(); });
for (const id of ['filter-runtime', 'filter-dimension', 'filter-scenario', 'filter-machine', 'filter-profile', 'filter-version']) element(id).addEventListener('change', renderResults);
window.addEventListener('hashchange', showTab);
document.addEventListener('visibilitychange', () => { if (document.hidden && isMeasuring) { stopRun(); status('Comparison canceled because the tab became hidden. Keep it visible and run again.'); } });
window.addEventListener('pagehide', () => { stopRun(); setPlaying(false); void stepping.then(() => { simulations.forEach(simulation => simulation.free()); views.forEach(view => view.dispose()); }); });
element('import-results').addEventListener('change', async event => {
  const input = event.target as HTMLInputElement;
  try {
    const file = input.files?.[0]; if (!file) return;
    if (file.size > 20_000_000) throw new Error('Result files must be smaller than 20 MB.');
    const run = validateRun(JSON.parse(await file.text()));
    saveRun({ ...run, source: 'local' }); updateFilters(); renderResults();
    element('results-status').textContent = 'Imported on this device. No file was uploaded.';
  } catch (cause) { element('results-status').textContent = `Import failed: ${String(cause)}`; }
  finally { input.value = ''; }
});
element('settings').addEventListener('input', event => {
  const input = event.target as HTMLInputElement;
  if (Object.hasOwn(settings, input.name)) {
    element<HTMLButtonElement>('benchmark').disabled = true;
    status('Apply the changed scene settings before measuring.');
  }
});
field('machine').value = machineName(() => localStorage, () => crypto.randomUUID());
field('machine').addEventListener('change', () => { try { localStorage.setItem('bevy-physics-machine', field('machine').value.trim()); } catch { /* Export works without storage. */ } });
readLink(); fillSettings(); showTab(); void reset(); requestAnimationFrame(frame);
void gpuAvailability().then(available => {
  hasGpu = available;
  for (const side of ['left', 'right']) {
    const option = element<HTMLSelectElement>(`engine-${side}`).querySelector<HTMLOptionElement>('option[value="nexus"]')!;
    option.textContent = available ? 'Nexus · GPU' : 'Nexus · no WebGPU adapter';
    option.disabled = !available || settings.dimension !== '3d' || ['kinematics', 'ccd', 'queries', 'characters', 'inverse-kinematics'].includes(settings.scenario) || settings.sleeping !== 'disabled' || settings.ccd !== 'discrete';
  }
}).catch(() => {});
