import { defineConfig } from "@playwright/test";

// Screenshot tests of the Arabic text (DECISIONS.md D-028). Each project stands for one of
// the app's webviews and runs on that webview's OS, with the browser engine behind it:
// WebKit on macOS (WKWebView), Edge on Windows (WebView2 is Edge's Chromium), WebKit on
// Linux (Playwright's Linux WebKit is the GTK port, as is WebKitGTK).
const ENGINES = [
  { name: "wkwebview", platform: "darwin", use: { browserName: "webkit" } },
  { name: "webview2", platform: "win32", use: { browserName: "chromium", channel: "msedge" } },
  { name: "webkitgtk", platform: "linux", use: { browserName: "webkit" } },
] as const;

const PORT = 1430;
// `missing` or `all` writes baselines; unset or empty: see `updateSnapshots` below.
const updateSnapshots = (process.env.E2E_UPDATE_SNAPSHOTS || undefined) as
  | "all"
  | "missing"
  | "none"
  | undefined;

export default defineConfig({
  testDir: ".",
  testMatch: "*.e2e.ts",
  // Baselines per engine, committed: e2e/screenshots/<engine>/<section>.png.
  snapshotPathTemplate: "{testDir}/screenshots/{projectName}/{arg}{ext}",
  outputDir: "results",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI
    ? [["list"], ["html", { open: "never", outputFolder: "report" }]]
    : "list",
  // A missing baseline fails in CI; locally it is written (review it before committing).
  updateSnapshots: updateSnapshots ?? (process.env.CI ? "none" : "missing"),
  expect: {
    toHaveScreenshot: { maxDiffPixelRatio: 0.002, animations: "disabled", caret: "hide" },
  },
  use: {
    baseURL: `http://localhost:${PORT}`,
    viewport: { width: 900, height: 1000 },
    deviceScaleFactor: 1,
    colorScheme: "light",
    locale: "en-US",
  },
  projects: ENGINES.filter((e) =>
    process.env.E2E_ENGINE ? e.name === process.env.E2E_ENGINE : e.platform === process.platform,
  ).map(({ name, use }) => ({ name, use })),
  webServer: {
    command: `pnpm exec vite --port ${PORT} --strictPort`,
    cwd: "..",
    url: `http://localhost:${PORT}/e2e/index.html`,
    reuseExistingServer: !process.env.CI,
  },
});
