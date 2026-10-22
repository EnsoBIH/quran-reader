#!/usr/bin/env node
// Lists the bundles `tauri build` wrote under <target dir>/**/release/bundle/ and fails if
// an installer is 15 MB or larger (DECISIONS.md D-026). Writes a table to the job summary
// on GitHub Actions.
//
//   node apps/desktop/scripts/bundle-size.mjs [target dir, default: target]

import { appendFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const LIMIT = 15_000_000;

/** What each bundle is, and whether the limit applies to it. */
const KINDS = [
  { test: /\.dmg$/, kind: "macOS disk image", installer: true },
  { test: /-setup\.exe$/, kind: "Windows installer (NSIS)", installer: true },
  { test: /\.msi$/, kind: "Windows installer (MSI)", installer: true },
  { test: /\.deb$/, kind: "Debian / Ubuntu package", installer: true },
  { test: /\.rpm$/, kind: "Fedora / openSUSE package", installer: true },
  // Carries its own copy of WebKitGTK and GTK, so it cannot be this small; it is the only
  // Linux build the updater can replace in place.
  { test: /\.AppImage$/, kind: "AppImage (bundles WebKitGTK)", installer: false },
  { test: /\.app\.tar\.gz$/, kind: "macOS update archive", installer: false },
];

function* files(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory() && !entry.name.endsWith(".app")) yield* files(path);
    else if (entry.isFile()) yield path;
  }
}

function* bundleDirs(target) {
  for (const entry of readdirSync(target, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    // target/release/bundle and target/<triple>/release/bundle
    for (const dir of [
      join(target, entry.name, "bundle"),
      join(target, entry.name, "release", "bundle"),
    ]) {
      try {
        if (statSync(dir).isDirectory()) yield dir;
      } catch {}
    }
  }
}

const target = process.argv[2] ?? "target";
const rows = [];
for (const dir of bundleDirs(target)) {
  for (const path of files(dir)) {
    const kind = KINDS.find((k) => k.test.test(path));
    if (kind) rows.push({ path: relative(target, path), size: statSync(path).size, ...kind });
  }
}

const mb = (bytes) => `${(bytes / 1e6).toFixed(1)} MB`;
const over = rows.filter((r) => r.installer && r.size >= LIMIT);
const lines = [
  "| Bundle | Kind | Size | Limit |",
  "| --- | --- | ---: | --- |",
  ...rows.map(
    (r) =>
      `| \`${r.path}\` | ${r.kind} | ${mb(r.size)} | ${
        r.installer ? (r.size < LIMIT ? `< ${mb(LIMIT)} ✓` : `**≥ ${mb(LIMIT)} ✗**`) : "—"
      } |`,
  ),
];
console.log(lines.join("\n"));
if (process.env.GITHUB_STEP_SUMMARY) {
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### Bundle sizes\n\n${lines.join("\n")}\n`);
}

if (!rows.some((r) => r.installer)) {
  console.error(`no installers found under ${target}/**/release/bundle`);
  process.exit(1);
}
if (over.length > 0) {
  console.error(`${over.length} installer(s) at or over ${mb(LIMIT)}`);
  process.exit(1);
}
