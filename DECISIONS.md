# Decisions

Short records of decisions and why they were made. Newest at the bottom. Change a decision by adding a new entry that supersedes the old one — don't rewrite history.

Format: **context → decision → consequences**.

---

## D-001 · Tauri 2 + React instead of egui (2026-10-21)

**Context.** bible-rs uses egui. Arabic needs real OpenType shaping, RTL/bidi and mark positioning for tashkeel. egui is built around Latin script and has no complex shaping.

**Decision.** Desktop app on Tauri 2 with a React + Vite frontend. Hot logic (decoding, reference parsing, search) lives in the Rust crate `quran-core` and is exposed as Tauri commands.

**Consequences.** Rendering is done by the system webview (WebKit / WebView2 / WebKitGTK) with HarfBuzz-grade shaping. Webviews differ per OS, so Arabic needs screenshot tests on all three. Mobile (iOS/Android) stays possible via Tauri.

## D-002 · Astro for the landing page (2026-10-21)

**Context.** The landing page is almost entirely static (/, /download, /privacy). It should share components and theme with the app.

**Decision.** Astro with the React integration. Components come from `packages/ui`, theme tokens from `packages/ui/src/theme.css`.

**Consequences.** Zero JS by default, React islands only where needed. Screenshots and live demo look the same as the app.

## D-003 · One repo, two workspaces (2026-10-21)

**Decision.** pnpm + Turborepo for JS/TS, a Cargo workspace for Rust (`crates/*` and `apps/desktop/src-tauri`). Shared JS versions live in the pnpm catalog (`pnpm-workspace.yaml`). Internal TS packages export their sources directly — no build step.

**Consequences.** `turbo run build` builds the frontends only. The full Tauri bundle is the separate task `tauri:build`, whose Turborepo inputs include `crates/**/*.rs`, `Cargo.toml`, `Cargo.lock` and `data/**` so caching stays correct when Rust code or data changes. Rust is checked in CI with `cargo test` / `cargo clippy`, not through Turborepo.

## D-004 · Only unmodified, freely licensed content in the repo (2026-10-21)

**Decision.** The Arabic text is the Tanzil Uthmani text, redistributed verbatim with attribution (CC BY 3.0 terms). `xtask` fetches only pinned URLs, checks SHA-256, and generates the binary format. A normalized search index is internal and never shipped as display text. Translations that aren't clearly free are user-imported, not bundled.

**Consequences.** A test asserts 114 surahs, 6,236 verses and source checksums. CI fails on any drift.

## D-005 · Amiri Quran as default font (2026-10-21)

**Decision.** Ship Amiri Quran (SIL OFL 1.1) via `@fontsource/amiri-quran`. Don't bundle KFGQPC Uthmanic Hafs until written permission exists.

## D-006 · Working title `quran-reader` (2026-10-21)

**Decision.** Repo name `quran-reader`, package scope `@quran-reader/*`, bundle identifier `io.github.quranreader.desktop`.

**Consequences.** Renaming later is cheap except for the bundle identifier: it determines where the app stores its data (SQLite with notes), so it must be final before the first public release.

## D-007 · English for repo-facing text (2026-10-21)

**Decision.** README, docs, code comments and commit messages in English to reach more contributors. App UI gets i18n (German and English first).

## D-008 · Platforms (open)

macOS only like bible-rs, or Windows/Linux from day one? Tauri allows both; the cost is testing effort. CI currently builds and tests on Linux and macOS.
