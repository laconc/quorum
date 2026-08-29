import { defineConfig, devices } from "@playwright/test";

/**
 * The instant every run is frozen at.
 *
 * Determinism in the screenshot pipeline rests on four things, and this is the
 * first: with a moving clock, anything relative — "due in 14 days", "reported
 * this morning" — drifts between runs and churns every image. The other three
 * are fixed identifiers from the seed generator, a font stack that resolves
 * identically (which is why images are produced in a container), and animation
 * disabled.
 */
const APP_CLOCK = "2026-03-01T12:00:00Z";
const PORT = 8137;

/**
 * Where the built binaries are.
 *
 * Defaults to the host build. `make screenshots` overrides it to point at the
 * Linux build, because images are produced inside a container so that a
 * developer's machine and CI rasterise text identically — see the Makefile.
 */
const BIN = process.env.APP_BIN_DIR ?? "../app/target/release";

export default defineConfig({
  testDir: "./tests",
  // A screenshot that needed a retry to match is a screenshot nobody can trust.
  retries: 0,
  // Deterministic output ordering, and no chance of two runs sharing the
  // single-instance database.
  workers: 1,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? [["github"], ["list"]] : [["list"]],

  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    // Motion is off everywhere: it is a screenshot-determinism control and an
    // accessibility one at the same time.
    reducedMotion: "reduce",
    trace: "retain-on-failure",
  },

  expect: {
    toHaveScreenshot: {
      animations: "disabled",
      caret: "hide",
      // Zero tolerance. A pixel budget hides exactly the small regressions
      // this suite exists to catch, and invites tuning the number instead of
      // fixing the cause.
      maxDiffPixels: 0,
    },
  },

  projects: [
    {
      name: "chromium-desktop",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 800 } },
    },
    {
      // 375px is the width the design is decided at, and iOS Safari is the
      // device this audience actually holds.
      name: "webkit-mobile",
      use: { ...devices["iPhone SE"] },
    },
    {
      name: "chromium-mobile",
      use: { ...devices["Desktop Chrome"], viewport: { width: 375, height: 812 } },
    },
  ],

  webServer: {
    // The release binary, seeded fresh. Building here rather than in the test
    // keeps the failure legible when the build is what broke.
    // The data directory is removed first: every run starts from the seed and
    // nothing else, so two runs cannot disagree because of what a previous one
    // left behind.
    command: `rm -rf ../.e2e-data && ${BIN}/seed ../.e2e-data && APP_CLOCK=${APP_CLOCK} APP_PORT=${PORT} ${BIN}/app-web`,
    url: `http://127.0.0.1:${PORT}/healthz`,
    reuseExistingServer: false,
    stdout: "pipe",
    stderr: "pipe",
    timeout: 60_000,
  },
});
