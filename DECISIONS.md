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

## D-009 · Tanzil sources: Uthmani XML + metadata 1.0 (2026-10-21)

**Context.** Tanzil offers the Uthmani text as plain text, text with verse numbers, XML and SQL, generated on request by `download.php` with a few options. The plain-text formats prefix the Basmala to verse 1 of every surah except 1 and 9, so showing it on its own line would mean cutting the displayed string. Metadata (surahs, juz, hizb quarters, manzils, rukus, pages, sajdas) is a static file, `quran-data.xml` 1.0.

**Decision.** Pin two files in `data/sources.toml`, each by exact URL and SHA-256:

- `download.php?quranType=uthmani&outType=xml&marks=true&sajdah=true` — Uthmani text with Tanzil's default options (pause marks and sajdah signs on, rub-el-hizb signs off). The XML has the Basmala in its own `bismillah` attribute, so nothing is split or joined; every verse and Basmala is taken byte for byte. The response is generated but deterministic (same SHA-256 on repeated downloads).
- `res/text/metadata/quran-data.xml`.

The copyright block from the top of the text file is written verbatim to `LICENSES/TANZIL.txt` and embedded in `data/quran.bin` (the terms ask for it in every derived file). Only line endings (CRLF → LF) and the blank lines around the block are normalized.

**Consequences.** The Basmala is not uniform: surahs 95 and 97 spell it with a shadda on the bā' (بِّسْمِ), as in the Madina Mushaf. Checks therefore test its presence, never equality with one fixed string. If Tanzil changes a file, `cargo xtask fetch` fails on the SHA-256; a new pin needs a review of the text diff, not just a new hash.

## D-010 · Binary data format read in place (2026-10-21)

**Context.** Text must be on screen in the first frame, and `unsafe` is forbidden in the workspace, so no transmuting or unchecked UTF-8.

**Decision.** `data/quran.bin` (≈1.3 MB, committed) is a small section table followed by flat sections: verse texts concatenated as UTF-8 with a u32 offset table, surah records, a deduplicated string table, and start lists for juz, hizb quarters and pages. The layout is documented in `crates/quran-core/src/format.rs`. `Quran::from_bytes` validates once (UTF-8, offsets, char boundaries, counts) and then hands out `&str` slices into the original bytes; juz/quarter/page are binary searches over the start lists. `quran-core` embeds the file with `include_bytes!` (via its build script, so the crate still compiles without it and reports `data_loaded: false`). The encoder lives next to the reader so both always agree; `cargo xtask build` only parses the sources and calls it.

`quran_core::TEXT_SHA256` is the SHA-256 of a canonical listing (`surah|ayah|text`, Basmala as ayah 0). `cargo xtask verify` checks it against the Tanzil XML, `cargo test` against the embedded file, and `verify` also rebuilds the file and requires it to match the committed one byte for byte.

**Consequences.** Measured (Criterion, Apple Silicon, release): load and validate 0.8 ms (almost all UTF-8 validation), one surah as owned JSON-ready structs 17 µs for al-Baqarah, `parse_ref` 19 ns. No compression, so the file stays sliceable in place. A format change bumps `format::VERSION`.

## D-011 · Minimal tooling dependencies (2026-10-21)

**Decision.** Runtime `quran-core` keeps only `serde`. `xtask` adds `roxmltree` (XML, no dependencies of its own), `sha2` and `toml` (both already in the tree through Tauri), and downloads with the system `curl` (preinstalled on macOS, Windows 10+ and CI runners) instead of an HTTP client with its own TLS stack. Criterion is a dev-dependency with default features off (no plotting, no rayon).

**Consequences.** `cargo xtask fetch` needs `curl` on `PATH`. CI caches `.cache/` keyed on `data/sources.toml`, so tanzil.net is only hit when a pin changes.
