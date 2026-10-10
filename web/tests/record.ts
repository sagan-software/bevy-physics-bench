import { summarize, type RunFile } from '../src/results';
import { DEFAULT_SETTINGS } from '../src/types';

export function record(): RunFile {
  return { schema_version: 1, created_at: '2026-10-10T00:00:00.000Z', source: 'local',
    environment: { runtime: 'wasm', machine: 'Test browser', cpu: null, logical_cores: 8,
      memory_bytes: null, os: 'Linux', browser: 'Test browser', target: 'wasm32-unknown-unknown',
      profile: 'baseline', threads: 1, notes: '', gpu: null },
    build: { compiler: 'rustc 1.98.1', rustflags: '-C target-feature=+simd128', features: [] },
    versions: { bevy: '0.19.1', avian: '0.7.0', bevy_rapier: '0.36.0', rapier: '0.35.0-glamx0.2',
      oxijolt: '1.0.1+jolt-5.6.0', jolt_js: '1.1.0', nexus: '0.6.0', revision: 'a'.repeat(40) },
    measurements: [{ engine: 'avian', settings: { ...DEFAULT_SETTINGS },
      plan: { warmup_steps: 1, samples: 5, batch_steps: 1, repeats: 1 },
      samples_ms: [1, 2, 3, 4, 5], statistics: summarize([1, 2, 3, 4, 5]),
      awake_bodies: 128, finite: true, min_y: .5, queries: { rays: 0, hits: 0 },
      scene_fingerprint: 'b'.repeat(64), scope: 'bevy-step', readback_ms: null }],
  };
}
