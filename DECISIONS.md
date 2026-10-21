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

## D-012 · Continuous reader: whole text in one call, own minimal virtualizer (2026-10-21)

**Context.** The reader scrolls through all 6,236 verses (6,350 rows with the surah headers). Only rows near the viewport may be mounted, rows vary from one line to a full screen (2:282), and the text must be on screen right after start-up.

**Decision.**

- *Transport.* One Tauri command, `quran_text`, sends the whole text and its markers once at start-up: verse strings, surah records with their Basmala, start lists for juz, hizb quarters and pages, and the sajdas (≈1.4 MB of JSON). Building and serializing it takes 0.45 ms (Criterion); in the release app, from the `quran_text` call to text on screen takes about 45 ms including IPC, `JSON.parse`, row tables and the first render. Ranges of verses would add round trips for every jump and scroll without a measurable gain. Per-verse juz, quarter and page numbers are computed in JS from the start lists.
- *Virtualizer.* A small one of our own (`apps/desktop/src/heights.ts`, `Reader.tsx`, about 250 lines) instead of a library. Mounted rows sit in normal flow between two spacers whose heights come from a prefix sum over row sizes. Sizes start as estimates from the verse length, font size and line width, and are replaced by ResizeObserver measurements as rows render; when a row *above* the reading position turns out taller or shorter than estimated, the scroll position moves by the difference before the frame is painted, so the text being read never jumps (WebKit has no CSS scroll anchoring). Jumps render the target row first, then scroll to it. A font, measure or view change re-estimates every row and keeps the current row in place.
- Considered: TanStack Virtual (`@tanstack/react-virtual`). It does the same in general form, but needs the same height estimator and scroll correction tuned for WebKit, released ~20 versions in the past month, and would be the app's only runtime dependency besides React.
- *No Suspense on start-up.* `main.tsx` awaits the data and the Quran font before the first render. With `use()` and a Suspense boundary, React 19 held the reveal back by its fallback throttle (~300 ms).

**Consequences.** Measured on an Apple Silicon Mac (release build, median of 15 starts, `QURAN_READER_TIMING=1`): text visible ≈ 420 ms after `run()`, ≈ 450 ms after `exec`. About 260 ms of that pass before the webview starts loading the page (Tauri and WKWebView start-up), so the 300 ms target from the plan is out of reach with this stack as it stands; our own share (bundle, data, first render) is ≈ 150 ms. Scroll performance needs a check on a 120 Hz display. Translations (Phase 4) will add per-verse strings to the same row model.

## D-013 · Reference parser and name matching in quran-core (2026-10-21)

**Context.** ⌘K has to understand `2:255`, `baqara 255`, `Al-Baqarah 255`, `بقرة`, typos (`bakara`), named verses (`Ayat al-Kursi`), and `juz 30`, `hizb 10`, `page 604`, in well under 0.1 ms.

**Decision.** `Quran::lookup` (ranked hits for the palette) and `Quran::parse_ref` (the best hit) live in `crates/quran-core/src/reference.rs`; name matching in `names.rs`. Names are compared as internal keys, never displayed: Latin input is folded to ASCII with transliteration variants unified (`q`/`k`, `e`/`i`, `o`/`u`, `w`/`u`, `y`/`i`, `dh`/`d`, `th`/`t`, doubled letters, a final `h` after a vowel, leading articles and `surah`/`sure`/`the`); Arabic input loses tashkeel and Quranic marks, alif variants become a bare alif, `ى`/`ي`, `ة`/`ه` are unified and the article is dropped. Candidates are each surah's Tanzil Arabic, transliterated and English names, a short table of other names (`Bani Isra'il`, `Al-Inshirah`, …) and a small alias table of named verses (Ayat al-Kursi 2:255, Ayat ad-Dayn 2:282, Amana ar-Rasul 2:285, Ayat al-Mubahala 3:61, Ayat an-Nur 24:35), each with a test. Ranking: exact key, prefix, typo (optimal string alignment distance scaled by length), typo in a prefix. Digits may be Arabic-Indic. Juz, hizb and page numbers resolve through the start lists in the data file (hizb *n* = quarter 4*n* − 3).

**Consequences.** Criterion: numeric references ≈ 0.1 µs, parts ≈ 0.2 µs, names ≈ 15–35 µs (all 114 surahs scored, no allocation per candidate). New named verses need an entry and a test; keep the table small and uncontroversial.

## D-014 · Reader settings and position in localStorage for now (2026-10-21)

**Decision.** Theme (system/light/sepia/dark), view (Arabic, stacked, columns), Arabic and translation size and line width are stored as one JSON object under `quran-reader:settings`; the last reading position under `quran-reader:position`. Every field is validated on load and falls back to its default on its own. Navigation history (⌘[ / ⌘], Alt+← / →, mouse back/forward) is in memory only.

**Consequences.** Phase 5 moves both into the SQLite store with the notes; the parsers in `settings.ts` and `history.ts` stay as the validation layer.
