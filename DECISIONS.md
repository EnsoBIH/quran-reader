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

## D-015 · Search index built at run time, not stored in the data file (2026-10-21)

**Context.** Search must take under 5 ms over the whole Quran, ignoring tashkeel and letter variants, and the normalized forms must never be shown or shipped as text (D-004). The index could be built when it is first needed, or written into `data/quran.bin` by `cargo xtask build`.

**Decision.** `quran_core::search_index()` builds it on first use from the embedded text and keeps it for the process; the Tauri app builds it on a background thread at start-up. The index (`crates/quran-core/src/search.rs`) is one normalized string per searchable text — all verses, each followed by `\n` — plus a verse offset table. A query scans that string for its longest word and checks the other words and the exclusions per candidate verse. For the hits it returns, it normalizes the verse again while recording, for every normalized byte, the bytes of the original it came from, and maps the matches back onto the unmodified text as UTF-16 ranges (JavaScript string indices). Marks are counted with the letter before them, so a range never splits a letter from its tashkeel.

Normalization (`normalize.rs`, shared with the name matching of D-013): tashkeel, Quranic annotation marks (pause and sajda signs, small letters, U+06D6–06ED), the superscript alif and tatweel are dropped; أ إ آ ٱ → ا, ى ئ → ي, ة → ه, ؤ → و; for search only, a standalone hamza ء is dropped too, so Uthmani spellings such as `ءَامَنُوا` meet common ones (`آمنوا`). Latin text is lowercased with diacritics folded (`é` → `e`, `ß` → `ss`).

Measured (Criterion, Apple Silicon, release):

| | |
| --- | --- |
| build the index | 3.2 ms (once, off the start-up path) |
| `search("زيتون")` (rare word, one scan) | 0.20 ms |
| `search("الله")` (1,500+ verses, 100 with ranges) | 0.44 ms |
| `search("م")` (a single letter, almost every verse) | 0.30 ms |
| `search("الله -رحمن")` (slowest of the set) | 0.67 ms |
| `search("الله juz:30")` | 0.02 ms |

Stored in the file, the normalized text would add ≈ 750 KB (+55 %) to `data/quran.bin` and ≈ 0.6 ms of UTF-8 validation to every start, on the path to the first frame — to save 3.2 ms once, later and on another thread. It would also put a modified copy of the text into a shipped file, and `xtask verify` would need to check it.

**Consequences.** No format change (`format::VERSION` stays 1). Known gaps of plain normalization on the Uthmani spelling: words written with a superscript alif or small letters don't meet the common spelling (`ٱلْكِتَٰبِ` → `الكتب`, so `كتاب` does not find it; `ٱلصَّلَوٰةَ` → `الصلوه`), and `يَٰٓأَيُّهَا` is one word in the text. Fixing that needs spelling-variant data (or the Tanzil simple text, aligned word by word) and is left for later. Translations (Phase 4) are added with `SearchIndex::add_text`; a query runs on each text on its own, and a verse is a hit if any text matches.

## D-016 · Search query language (2026-10-21)

**Decision.**

- Words: all must occur in the verse (`رحمن رحيم`). Arabic words match anywhere inside a word, because particles, prepositions and the article are written together with the next word and pronouns with the one before (`رحمن` finds `ٱلرَّحْمَٰنِ`). Latin words match at the start of a word (`merc` finds `mercy`, `in` does not find `within`).
- `"…"` (also `“…”`, `„…“`, `«…»`): a phrase, its words adjacent and in order. An unclosed quote runs to the end of the query.
- `-word`, `-"…"`: the verse must not contain it. A query of exclusions only is rejected.
- `surah:2`, `surah:2-4`, `surah:baqara` (any name the ⌘K palette knows), `juz:30`, `juz:28-30`; short forms `s:` and `j:`; Arabic-Indic digits work. Several `surah:` scopes add up, as do several `juz:` scopes; `surah:` and `juz:` together intersect (`surah:2 juz:1` is 2:1–2:141).
- Results are in Quran order with the total count; the app asks for 100 at a time.

**Consequences.** No ranking yet: in Quran order, results are predictable and the first hit of a common word is the first place it occurs. The ⌘F panel highlights matches with a background colour only, so the text keeps its colour and shaping.

## D-017 · Bundled translations: Pickthall (1930) only, German by import (2026-10-21)

**Context.** Phase 4 bundles translations only if they are clean (D-004): public domain in Germany/EU and in the US, taken from a digital source whose own terms allow redistribution by anyone (the code is MIT, so also commercial use), and aligned to the 6,236 verses. Tanzil's translations are "for non-commercial purposes only". The review per edition, country and source is in `LICENSES/README.md` ("Translation review").

**Decision.**

- **English: Pickthall, *The Meaning of the Glorious Koran* (1930)**, public domain in the EU since 2007 (Pickthall † 1936) and in the US since 2026-01-01 (published 1930). Text: the `P:` verses of gutenberg.org eBook #16955, which follows the Kufan count. PG's licence binds only its trademark; the bundled file contains the verses alone. The text is *not* the verbatim 1930 print (some "God" → "Allah", 1:4 "Master" for "Owner", punctuation, standard verse division) and its origin is unknown; these mechanical changes carry no copyright of their own. It is labelled as this digital text, never as the 1930 print. Rejected: Tanzil `en.pickthall` and its copies (non-commercial terms, 35 corrupted "no Allah save Him" verses), quran.com (no redistribution).
- **German: none.** Henning (1901, † 1927), Goldschmidt (1916, † 1950), Ullmann (1840) and Grigull (1901) are public domain, but no clean, verse-aligned digital text exists: projekt-gutenberg.org has the copyrighted Rudolph revision of Henning under private-use-only terms, the rest are Fraktur scans without (or with Flügel) verse numbers. German readers import a translation they downloaded (D-019).
- The user approved this choice on 2026-10-21 before anything was bundled.

**Consequences.** A German public-domain text would have to be produced: OCR of the 1901 Henning print, proofreading, and re-cutting Flügel's verses at Kufan boundaries — a project of its own, which would publish the result with its own provenance. If gutenberg.org changes the file, `cargo xtask fetch` fails on the SHA-256 and the new text needs a review like the first.

## D-018 · Translations in their own files, not in `data/quran.bin` (2026-10-21)

**Context.** Translations could be further sections of `data/quran.bin` or separate files.

**Decision.** One file per translation, `data/translations/<id>.bin`, in the container of `quran.bin` (header, section table, 4-byte aligned sections; `format::write_sections`) with its own magic `QURANTRN`: the description (id, language, name, translator, source, licence, notice), the SHA-256 of its source, verse offsets and the verses (`crates/quran-core/src/translation.rs`). `quran-core`'s build script embeds every file in `data/translations/`; `Translation::from_bytes` validates once and reads in place like `Quran`. `cargo xtask build` writes them, `verify` rebuilds each from its pin and notice and requires the same bytes, and refuses files that are not in its list.

Why separate:

- `quran.bin`, its text digest and the Tanzil notice stay exactly as they are; adding or dropping a translation never touches the Arabic file.
- Each translation carries its own licence notice and source pin, so its terms travel with it.
- User imports (D-019) are written in the same format, so the app reads bundled and imported translations with one code path.
- Start-up stays lean: the app sends a translation to the webview only when the view shows one (≈ 0.9 MB JSON for Pickthall), not with the Arabic.

**Consequences.** Embedded size +0.85 MB for Pickthall. The app's search index holds the Arabic and every installed translation (`SearchIndex::add_text`, `add_owned_text` for imports); building it takes ≈ 6.8 ms with Pickthall, on a background thread at start-up. Measured (Criterion, Apple Silicon): `mercy` 0.40 ms, `"lord of the worlds"` 0.27 ms, `the` 0.45 ms, `allah -merciful` 0.58 ms, `الله` 0.53 ms — well under 5 ms.

## D-019 · User import of other translations (2026-10-21)

**Context.** Most translations (Tanzil's included) may not be redistributed, but users may download them for their own use.

**Decision.** Settings → Translation → Import… reads a file the user picked (plain text, `surah|ayah|text` or XML, Tanzil's formats; `docs/translation-import.md`). `quran_core::import::parse` requires every one of the 6,236 verses exactly once and refuses the file otherwise with the offending line, so a translation is never shown next to the wrong verse. Verse texts are kept as in the file; the comments (Tanzil's notice) are kept as the notice, `Name:`, `Translator:`, `ID:` and `Language:` lines fill in the description. The app stores the import as a translation file (D-018) under `<app data>/translations/user.<id>.bin`, never uploads or bundles it, and lists it as "imported" in settings, compare and credits. The file is read in the webview (`<input type="file">`) and sent to Rust as text, so no dialog plugin is needed. Settings keep the chosen translation's id (D-014); without a choice the app picks one in the system language, else the first installed.

**Consequences.** Imports use the same search and display paths as bundled translations. An import that no longer loads (damaged, or a future incompatible format) is skipped at start-up with a message on stderr.

## D-020 · Translation shown from the first start; T toggles it (2026-10-22)

**Context.** With the Arabic-only view as the default, the bundled translation was visible only in the compare view (C) until the user found the view setting.

**Decision.** New installs start in the stacked view (translation below each verse). T, or the Translation button in the top bar, hides the translation and shows it again in the view it was last shown in (`translatedView`, stored with the settings, D-014). Settings already saved keep their view.

**Consequences.** Start-up now also loads the shown translation before the first frame (≈ 0.9 MB of JSON for Pickthall); it is fetched in parallel with the Arabic text. Readers who want the Arabic alone press T once.

