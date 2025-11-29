/// <reference types="vitest" />
/// <reference types="vitest/globals" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const serverUrl = "http://localhost:8000";

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react({
      babel: {
        plugins: ["babel-plugin-react-compiler"],
      },
    }),
  ],
  server: {
    proxy: {
      "/events": {
        target: serverUrl,
        changeOrigin: true,
      },
      "/schemas": {
        target: serverUrl,
        changeOrigin: true,
      },
      "/api/push": {
        target: serverUrl,
        changeOrigin: true,
      },
      "/query": {
        target: serverUrl,
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
  },
  assetsInclude: ["**/*.json"],
  // @ts-ignore - Vitest options
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/setupTests.ts"],
    testTimeout: 10000,
    hookTimeout: 10000,
    teardownTimeout: 10000,
    pool: "forks",
    isolate: true,
  },
});
