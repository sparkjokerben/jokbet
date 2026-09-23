import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

const host = process.env.TAURI_DEV_HOST;

// Two pages: pet.html stays as small as possible; app.html hosts every
// on-demand window (stats / settings / onboarding) selected by ?view=.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2022",
    rollupOptions: {
      input: {
        pet: resolve(import.meta.dirname, "pet.html"),
        app: resolve(import.meta.dirname, "app.html"),
      },
    },
  },
  test: {
    include: ["src/**/*.test.ts", "site/**/*.test.ts"],
  },
});
