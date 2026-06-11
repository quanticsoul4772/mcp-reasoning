import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Relative base so the built assets resolve when the SPA is served from the
// embedded sidecar root. Output to dist/, which is committed and embedded via
// rust-embed (the Rust build needs no Node toolchain).
export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    // Single chunk keeps the embedded asset set small and simple.
    chunkSizeWarningLimit: 1500,
  },
});
