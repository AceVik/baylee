import { defineConfig, devices } from "@playwright/test";

import { BASE_URL } from "./e2e/harness.ts";

// Against a real baylee-feedback (e2e/harness.ts): `cargo build -p
// baylee-feedback`, `npm run build`, DATABASE_URL set, then `npm run e2e`.
// `localhost` rather than 127.0.0.1: a browser keeps a `Secure` cookie over
// plain HTTP only there.
export default defineConfig({
  testDir: "e2e",
  globalSetup: "./e2e/global-setup.ts",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  use: {
    baseURL: BASE_URL,
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
});
