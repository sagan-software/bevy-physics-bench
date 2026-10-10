import { defineConfig } from 'vite';

export default defineConfig({
  root: 'web',
  base: '/bevy-physics-bench/',
  build: { outDir: '../dist', emptyOutDir: true },
});
