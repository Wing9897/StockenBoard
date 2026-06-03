import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Dedicated Vitest config kept separate from vite.config.ts so the
// Tauri-specific dev-server settings never interfere with the test run.
// This config is only used by the `npm test` scripts and does not affect
// `npm run build`.
export default defineConfig({
  plugins: [react()],
  test: {
    // Provide a browser-like DOM for React component tests.
    environment: "jsdom",
    // Allow describe/it/expect without explicit imports.
    globals: true,
    // Registers jest-dom matchers and auto-cleanup after each test.
    setupFiles: ["./src/test/setup.ts"],
    // Only treat *.test.* / *.spec.* files under src as tests.
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    css: false,
  },
});
