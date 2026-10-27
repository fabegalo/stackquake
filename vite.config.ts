import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2022",
    // No sourcemaps in the bundle: smaller attack surface and binary.
    sourcemap: false,
    // No assets inlined as data: URLs; the CSP only allows 'self'.
    assetsInlineLimit: 0,
  },
});
