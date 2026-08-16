import { defineConfig } from 'vite'
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [
    wasm()
  ],
  // The wasm glue uses top-level await, which needs a modern target. Vite 8
  // supports it natively, so vite-plugin-top-level-await is no longer needed.
  build: {
    target: 'esnext'
  },
  esbuild: {
    target: 'esnext'
  }
});
