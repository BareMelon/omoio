import { defineConfig } from "vite";

// https://vite.dev/config/
export default defineConfig({
  // index.html lives in src/, beside the rest of the frontend.
  root: "src",
  build: {
    outDir: "../dist",
    emptyOutDir: true,
  },
  // Vite options tailored for Tauri development.
  // Prevent Vite from obscuring Rust errors.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port, fail if it's not available.
    port: 1420,
    strictPort: true,
    watch: {
      // Tell Vite to ignore watching `src-tauri`.
      ignored: ["**/src-tauri/**"],
    },
  },
});
