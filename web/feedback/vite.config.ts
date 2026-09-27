import { defineConfig } from "vitest/config";

// No plugin: Vite's own transform (oxc) compiles TSX with React's automatic
// runtime. The build writes hashed files under `assets/`, which the service
// serves as immutable, and an `index.html` without inline script, so the
// service's CSP (`script-src 'self'`) holds.
export default defineConfig({
  oxc: {
    jsx: { runtime: "automatic" },
  },
  build: {
    outDir: "dist",
    assetsDir: "assets",
    sourcemap: false,
    // The modulepreload polyfill is injected inline in some setups; this UI
    // targets current browsers only, and the CSP forbids inline script.
    modulePreload: { polyfill: false },
    target: "es2023",
  },
  server: {
    // `npm run dev` proxies the JSON routes to a local service.
    proxy: {
      "/ui/api": "http://127.0.0.1:28780",
    },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["src/test-setup.ts"],
    restoreMocks: true,
    unstubGlobals: true,
  },
});
