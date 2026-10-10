import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['web/tests/**/*.test.ts'],
    coverage: { provider: 'v8', include: ['web/src/benchmark.ts', 'web/src/results.ts', 'web/src/series.ts', 'web/src/machine.ts'], reporter: ['text', 'json-summary'], thresholds: { statements: 100, branches: 100, functions: 100, lines: 100 } },
  },
});
