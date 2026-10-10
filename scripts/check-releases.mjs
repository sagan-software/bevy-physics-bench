import { writeFile } from 'node:fs/promises';

const pinned = { bevy: '0.19.1', avian2d: '0.7.0', avian3d: '0.7.0', bevy_rapier2d: '0.36.0', bevy_rapier3d: '0.36.0', oxijolt: '1.0.1', nexus_rbd3d: '0.6.0' };
const releases = [];
for (const [name, version] of Object.entries(pinned)) {
  const response = await fetch(`https://crates.io/api/v1/crates/${name}`, { signal: AbortSignal.timeout(20000), headers: { 'User-Agent': 'bevy-physics-bench release check (github.com/sagan-software/bevy-physics-bench)' } });
  if (!response.ok) throw new Error(`${name}: HTTP ${response.status}`);
  const { crate: crateInfo } = await response.json();
  releases.push({ name, pinned: version, latest: crateInfo.max_stable_version, needs_review: version !== crateInfo.max_stable_version });
}
const response = await fetch('https://registry.npmjs.org/jolt-physics/latest', { signal: AbortSignal.timeout(20000) });
if (!response.ok) throw new Error(`jolt-physics: HTTP ${response.status}`);
const jolt = await response.json();
releases.push({ name: 'jolt-physics', pinned: '1.1.0', latest: jolt.version, needs_review: jolt.version !== '1.1.0' });
const result = { checked_at: new Date().toISOString(), releases };
await writeFile('release-report.json', `${JSON.stringify(result, null, 2)}\n`);
for (const release of releases) console.log(`${release.name}: pinned ${release.pinned}; latest ${release.latest}${release.needs_review ? '; compatibility review required' : ''}`);
