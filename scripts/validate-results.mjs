import { readFile, stat } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { validateRun } from '../web/src/results.ts';

const directory = new URL('../web/public/results/', import.meta.url);
const manifest = JSON.parse(await readFile(new URL('index.json', directory), 'utf8'));
if (!Array.isArray(manifest) || manifest.length > 5000 || new Set(manifest).size !== manifest.length) throw new Error('Invalid or duplicate result manifest entries.');
const files = process.argv.slice(2);
const paths = files.length ? files : manifest.map(name => {
  if (typeof name !== 'string' || !/^[a-z0-9][a-z0-9.-]*\.json$/.test(name) || name === 'index.json') throw new Error('Invalid result filename.');
  return fileURLToPath(new URL(name, directory));
});
for (const path of paths) {
  if ((await stat(path)).size > 20_000_000) throw new Error(`${path}: result exceeds 20 MB.`);
  const result = validateRun(JSON.parse(await readFile(path, 'utf8')));
  if (!/^[0-9a-f]{40}$/.test(result.versions.revision)) throw new Error(`${path}: published results require a clean, full Git revision.`);
  if (!result.environment.cpu && result.environment.runtime === 'native') throw new Error(`${path}: native collection requires detected CPU metadata.`);
  console.log(`${path}: ${result.measurements.length} measurements validated`);
}
console.log(`${paths.length} result files validated`);
