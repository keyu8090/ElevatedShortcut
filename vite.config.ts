import { defineConfig } from "vite";
import fs from "node:fs";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// Fix building from a Windows junction/symlinked workspace path: Rollup doesn't allow emitting
// assets with "../" in their names, which can happen when `process.cwd()` is a junction that
// resolves to a different real path. Using the real path as Vite root avoids that.
const root = fs.realpathSync(process.cwd());

// https://vite.dev/config/
export default defineConfig(async () => ({
  root,
  resolve: {
    preserveSymlinks: true,
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
