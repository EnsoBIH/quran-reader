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


## D-021 · Study store: SQLite via `rusqlite` (bundled) in `quran-core` (2026-10-22)

**Context.** Highlights, notes, bookmarks, the reading plan and the last position must stay on the user's machine in one file, survive app updates (schema changes), and be testable without the UI. D-014 kept settings and position in localStorage until this phase.

**Decision.**

- `rusqlite` 0.40 with the `bundled` feature (default features off: no statement cache, no wasm backend), behind a `store` feature of `quran-core` that only the desktop app enables, so the CLI and `xtask` keep their lean dependency tree (D-011). `bundled` compiles SQLite into the binary, so every OS gets the same SQLite and no system library is needed. New crates in the tree: `rusqlite`, `libsqlite3-sys` (with `cc` at build time), `bitflags`, `smallvec`, `fallible-iterator`, `fallible-streaming-iterator`. Size cost, measured with the release profile (LTO, stripped, Apple Silicon) on a probe binary that opens a database and runs a query: +1.7 MB. Considered: a JSON file (no queries, no transactions, every change rewrites everything), `sqlx`/`diesel` (async runtime or ORM for six tables), the system SQLite (not on every Linux/Windows install, versions differ).
- The file is `<app data>/study.sqlite3` (on macOS `~/Library/Application Support/io.github.quranreader.desktop/`), default rollback journal so it stays one file. Nothing in it ever leaves the machine except through the user's own export (D-022).
- Every row is keyed by verse (`surah`, `ayah`), never by a translation, so it stays valid when the translation changes. One highlight, one note and one bookmark per verse.
- Schema versions are SQLite's `user_version`. `store::MIGRATIONS` is an append-only list; opening a file runs each missing step in its own transaction, and a file with a higher version than the build knows (written by a newer app) is refused instead of changed. Tests cover a fresh file, reopening, an upgrade that keeps rows, a failing step that rolls back, and a newer file.
- The last reading position moves from localStorage into the store (`state` table); an existing localStorage value is copied once at start-up and then removed (kept if the copy fails). Navigation history stays in memory: like a browser's back/forward it belongs to the session, and replaying it after a restart would send ⌘[ to places read days ago. Reader settings stay in localStorage: they are per-device display preferences, and the theme has to be known before the first frame.

**Consequences.** `Store` is synchronous behind a mutex in the Tauri app; every call is a few small statements (well under a millisecond). Changing the schema means adding a migration and a test, never editing a shipped one. The bundle identifier (D-006) decides the folder, so it must stay fixed once released.

## D-022 · Study features: keys, export formats and import merge (2026-10-22)

**Decision.**

- *Which verse.* Clicking a verse selects it (outline and a small toolbar: six colours, note, bookmark, copy). Keys act on the selected verse while it is in view, otherwise on the verse at the top of the reader (the one ⌘K/C also use): 1–6 highlight (the same key again removes it), N opens the note editor (Esc or ⌘↵ saves, an emptied note is deleted), B toggles a bookmark (labels are edited in the study panel), ⌘C copies the verse when no text is selected. S opens the study panel (bookmarks, notes, highlights with jump-to; plan; export/import); it shares the right side with ⌘F search.
- *Highlights* are a tint of `--color-mark-1..6` behind the whole verse, not a text colour, so the Arabic keeps its colour and shaping.
- *Copy* puts the unmodified Arabic, the shown translation (if any), `— Surah name s:a` and the attributions (Tanzil, and the translation's name) on the clipboard.
- *Export* writes to the Downloads folder (`quran-reader-study-YYYY-MM-DD.md` / `.json`, never overwriting), so no dialog plugin is needed. JSON is lossless and versioned (`"format": "quran-reader-study", "version": 1`); a newer version is refused with a message. Markdown has a section per verse with its Arabic text, `- Highlight: yellow (1)`, `- Bookmark: label` and the note as a quote, plus the plan; it can be imported again and loses only the timestamps.
- *Import* reads either format (JSON if it starts with `{`), validates everything first (existing verses, colours 1–6, one entry per verse, plan days), and shows a preview: per kind how many entries are new, changed, removed and unchanged, and which verses. *Merge* keeps the user's entries and adds the file's; on the same verse with different content the file wins. *Replace* makes the data exactly the file's. Entries with the same content keep their stored dates. A plan with the same length and start merges its read days. The whole import is one transaction.

## D-023 · Khatm plan portions (2026-10-22)

**Decision.** A plan has 1–604 days and a start date. Up to 240 days, the 240 hizb quarters are spread evenly (day *i* of *n* reads quarters ⌊(i−1)·240/n⌋ to ⌊i·240/n⌋ − 1), so 30 days is exactly one juz a day and 60 days one hizb a day; longer plans are cut by Madina Mushaf pages the same way. Portions come from `quran_core::plan::portions`; "today" is the UI's local date sent as `YYYY-MM-DD`, and the plan day is the calendar difference to the start (own date arithmetic, no date crate). The panel shows today's portion with Read / Mark as read, a catch-up card for the first unread earlier day, progress, and every day with a checkbox.

## D-024 · Reader keeps the jumped-to verse in place while rows are measured (2026-10-22)

**Context.** Phase 2 measured restored and jumped-to verses landing 60–150 px too high in the stacked view (in a browser pane, unconfirmed in the app). The ResizeObserver correction (D-012) found the reading row with `rowAt(scrollTop)`. Row offsets are fractional estimates and the browser rounds `scrollTop`, so after a jump to row *r* it could sit a fraction of a pixel above *r*'s offset; `rowAt` then said *r* − 1, and the measured size change of *r* − 1 — the row right above the target, with the largest estimate error in the stacked view — was not compensated, pushing the target off by that error.

**Decision.** The correction now takes the reading row the same way as `update` does (one pixel in), remembers how far the viewport top is into it, and after applying the new sizes puts it back at exactly that distance. The re-estimate after a font or view change uses the same rule.

**Consequences.** Measured in the real window (WKWebView, dev build), offset of the jumped-to row from the viewport top for 2:255, 18:10, 2:282, 24:35, 3:100, 36:1 and 9:60: stacked view before −55 −53 11 10 −54 1 −82 px, after 1 1 1 0 1 1 0; columns before 0 0 0 −57 0 −62 0, after 0 0 0 1 0 1 0; Arabic only 0 throughout. A plan portion (4:24) and a restore after restart (3:1, stacked) also land at 0. The bug was real, not only a test-environment artifact.

## D-025 · Platforms and release pipeline: macOS, Windows and Linux from 1.0 (2026-10-22)

**Context.** D-008 left open whether to ship macOS only. Phase 6 asks for release builds of all three platforms with `tauri-action`.

**Decision.** Supersedes D-008. 1.0 ships for macOS (one universal build for Apple silicon and Intel, macOS 11+), Windows 10/11 x64 and Linux x86-64. `.github/workflows/release.yml` builds on `macos-15`, `windows-2025` and `ubuntu-22.04` (the oldest supported LTS runner, so the packages work on older glibc) when a tag `v<version>` is pushed, and uploads the bundles to a *draft* release that the maintainer publishes by hand after checking it (docs/release.md). A first job checks that the tag matches the version, that the updater key is in `tauri.conf.json` and that every signing secret is set, so a release is never built half-signed. Pull requests that touch the app shell or the workflow build the same bundles unsigned, as a check that bundling still works and stays small.

- macOS: Developer ID signature and notarization with the Apple secrets (docs/release.md); the bundler imports the certificate into a temporary keychain.
- Windows: NSIS (`-setup.exe`, per-user install, no admin rights; the updater prefers it) and MSI. No Authenticode certificate yet, so SmartScreen warns on first start; the download page says so. Azure Trusted Signing or an OV certificate can be added later without changing the app.
- Linux: AppImage, `.deb`, `.rpm`.
- One version for everything: the root `Cargo.toml` (`workspace.package.version`, now 1.0.0). `tauri.conf.json` no longer has its own `version` and takes the Cargo one, so the app, the `quran-core` version shown in the footer and the tag cannot disagree. The JS packages are private and keep 0.0.0.

**Consequences.** The bundle identifier (D-006) is now final: changing it after a release would move users' study data. Building costs about 15 runner minutes per platform per release.

## D-026 · Installer budget: under 15 MB (2026-10-22)

**Decision.** Every installer — `.dmg`, NSIS `-setup.exe`, `.msi`, `.deb`, `.rpm` — must be smaller than 15 MB (15,000,000 bytes). `apps/desktop/scripts/bundle-size.mjs` runs after each release build, lists every bundle in the job summary and fails the job when an installer reaches the limit. The AppImage is listed but exempt: it carries its own WebKitGTK and GTK (typically 70–100 MB), which is the point of the format, and it is the Linux build the updater can replace in place. The `.app.tar.gz` is the macOS update payload, not an installer.

What goes in: the Rust binary with the embedded data (`quran.bin` 1.3 MB, Pickthall 0.85 MB), SQLite (+1.7 MB, D-021), the updater (HTTP client with rustls and the `ring` crypto, minisign verification) and the frontend (0.46 MB: JS, CSS and the Amiri Quran subsets as woff2 and woff; Tauri compresses embedded assets with Brotli). The universal macOS binary holds two architectures, so it is the largest; the `.dmg` is compressed. The release profile already uses LTO, one codegen unit and `strip` (root `Cargo.toml`).

**Consequences.** The first CI release build gives the real numbers; a regression shows up as a red pull request check before it reaches a release.

## D-027 · Auto-updater via GitHub Releases (2026-10-22)

**Decision.** `tauri-plugin-updater` with one endpoint, `https://github.com/EnsoBIH/quran-reader/releases/latest/download/latest.json`, which `tauri-action` writes into every release (`updaterJsonPreferNsis: true`). `releases/latest` resolves only to published, non-pre-release releases, so drafts and pre-releases never reach users.

- The check and the install run in Rust (`check_update`, `install_update` in `src-tauri/src/lib.rs`); the webview gets no plugin permissions. The update found by the check is kept in Rust state, so the webview cannot hand in a different one.
- The app checks once, five seconds after start-up (off the start-up path), if *Check for a new version at start-up* is on in the settings (default on, stored with the reader settings, D-014). A found update shows a card with the version, the release notes, *Install and restart* and *Later*; nothing is downloaded before the click. Settings → Updates has *Check now*. Development builds never check.
- Every update is verified against the minisign public key in `tauri.conf.json` before it is installed. Windows installs in `passive` mode (progress bar, no questions) and restarts the app; macOS and the AppImage are replaced in place, then the app restarts.
- The request carries no user data; GitHub sees the IP address and the plugin's user agent like any download. The privacy page says so.
- The public key is empty in the repository until the maintainer generates the key pair (docs/release.md); the private key never enters the repository. An agent or contributor cannot create it on the maintainer's behalf, because whoever holds it can ship code to every installed copy.

**Consequences.** New crates in the app: `tauri-plugin-updater` (with `reqwest`, `rustls` + `ring`, `minisign-verify`, `zip`/`tar`/`flate2`) and `tauri-plugin-opener` (D-030). Losing the private key means existing installs can no longer be updated. `.deb` and `.rpm` installs get the update offered but are best updated by downloading the new package.

## D-028 · Screenshot tests of the Arabic per webview engine (2026-10-22)

**Context.** D-001: the app is drawn by three different webviews (WKWebView, WebView2, WebKitGTK), so Arabic shaping needs a check on each. Driving the real webviews is not possible on all three: `tauri-driver` (WebDriver) has no macOS support, and none of the webviews exposes a screenshot API through Tauri.

**Decision.** Playwright renders a test page (`apps/desktop/e2e/index.html`) with the app's own components (`Ayah`, `SurahHeader`, `Basmala`), theme and font, on the OS of each webview with the engine behind it: WebKit on macOS (the engine of WKWebView, with CoreText), Microsoft Edge on Windows (WebView2 *is* Edge's Chromium, with DirectWrite), WebKit on Linux (Playwright's Linux WebKit is the GTK port, as WebKitGTK; HarfBuzz and FreeType). CI runs them as the job "Arabic rendering" on pinned runner images.

- *Verses.* `apps/desktop/e2e/verses.json` is generated from the embedded data by `UPDATE_FIXTURE=1 cargo test -p quran-core --test fixture` and checked by `cargo test`, so the page shows exactly the app's text. Sections: al-Fatihah with Pickthall below, 2:1–5 with header and Basmala, 2:255, the Basmala variants (95, 97, 27:30), disjoined letters, sajda verses, surah 9, the rare Hafs marks (11:41, 12:11, 21:88, 41:44), short surahs, and — chosen by the generator — the shortest verse for each character not covered yet, so every character that occurs in the text is on the page.
- *Screenshots.* One per section, compared with the committed baseline of that engine (`e2e/screenshots/<engine>/`, `maxDiffPixelRatio` 0.002). A missing baseline fails in CI; new ones are generated by the runners (docs/release.md) and reviewed as images.
- *Measurements*, which hold in every engine without a baseline: letters join (a word is at least 10 % narrower than the same letters forced apart with ZWNJ; HarfBuzz with Amiri Quran: 43 vs 95 px for بسم at 40 px), tashkeel adds no advance width (marks drawn as separate glyphs or with a dotted circle would; ±1 px), the end-of-ayah sign encloses its number (۝٢٨٦ is as wide as ۝), and the verse runs right to left.
- *Font coverage*, without a browser: every character of the text lies in the `unicode-range` of an Amiri Quran `@font-face` and has a glyph in that face's file (cmap), so no character silently falls back to another font.

**Consequences.** The tests stand for the webviews' engines, not the exact system builds: WKWebView uses the macOS system WebKit, WebView2 the Evergreen runtime, WebKitGTK the distribution's version. Real-window checks per platform stay part of the release checklist and the content review (docs/content-review.md). New runner images or Playwright versions mean new baselines.

## D-029 · Landing page on GitHub Pages with per-PR previews (2026-10-22)

**Decision.** `.github/workflows/site.yml` builds `apps/web` and pushes it to the `gh-pages` branch: main to the root (`JamesIves/github-pages-deploy-action`, rebasing rather than force-pushing and keeping `pr-preview/`), each pull request that changes the site to `pr-preview/pr-<n>/` with a sticky comment linking it, removed again when the pull request closes (`rossjrw/pr-preview-action`). Both third-party actions are pinned to a commit. Astro takes `site` and `base` from `SITE_URL` / `SITE_BASE`, all internal links go through `href()` (`apps/web/src/lib/links.ts`), and assets go to `assets/` instead of `_astro/`, which Jekyll on GitHub Pages would drop.

Considered: Cloudflare Pages, Netlify or Vercel (previews built in, but an outside account and its token in the repository secrets), and the "GitHub Actions" Pages source (no subfolders, so no previews next to the main site).

**Consequences.** No account outside GitHub. Pull requests from forks get no preview (their token is read-only). Pages has to be switched on once (docs/release.md). A custom domain later only needs a `CNAME` file and `SITE_BASE=/`.

## D-030 · "Report a problem" and external links (2026-10-22)

**Decision.** The footer and the credits have *Report a problem*. It opens the GitHub issue form `.github/ISSUE_TEMPLATE/problem.yml` in the browser with the verse being read (or the selected one), the app version, the OS (from the user agent) and view, translation and theme filled in through the form's query parameters (`apps/desktop/src/links.ts`). Nothing is sent from the app; the user sees and edits everything before submitting, and needs a GitHub account to do so. The credits link to their sources.

Links open through one Rust command, `open_link`, which only accepts URLs below a short list of prefixes (the repository, tanzil.net, the Amiri repository, the Gutenberg eBook) and uses `tauri-plugin-opener` from Rust; the webview gets no opener permission. So a bug in the frontend cannot open arbitrary URLs or local files.

## D-031 · Content review before 1.0 (2026-10-22)

**Decision.** `docs/content-review.md` is the checklist for people with Arabic and Quran knowledge: rendering on each webview (with the screenshot sheets of D-028), structure (Basmala, counts, juz/hizb/page, sajdas and their labels), names and named verses, search, the translation and attributions, wording. Each reviewer files a *Content review* issue (`.github/ISSUE_TEMPLATE/content-review.yml`) with their background, what they compared against and their findings; the sign-off table in the document records it. Areas A and B need two independent reviewers. Text findings go to Tanzil, since the app never edits the text (D-004).

**Consequences.** The 1.0 tag waits for the sign-off (docs/release.md, step 1). The review itself cannot be automated and is not part of the pull request that sets it up.

## D-032 · Spaces from the Arabic font file, so waqf marks are drawn (2026-10-22)

**Context.** The first run of the screenshot tests (D-028) in WebKit on Linux showed every waqf mark (ۖ ۗ ۚ ۛ ۘ ۙ, e.g. the paired ۛ of 2:2) as an empty box, and lines with them ran past the left edge. In the Tanzil text a waqf mark is a combining mark on the *space* between two words. `@fontsource/amiri-quran` splits Amiri Quran into an Arabic and a Latin file and assigns U+0020 to the Latin one; WebKit draws a combining mark with the font of its base character, and the Latin file has no waqf marks.

**Decision.** `packages/ui/src/quran-font.css` imports the Fontsource faces and adds one more face of "Amiri Quran" with the Arabic file for U+0020 and U+00A0 (the Arabic file has both). Faces declared later win for overlapping ranges, so spaces in Arabic text now come from the Arabic file, together with the marks on them. The desktop app, the landing page and the test page import this file instead of the package. No new file is shipped: the face points at the same hashed asset. A font test checks that every mark in the text is in the file of the character it sits on; without this face it lists the six waqf marks on U+0020.

**Consequences.** Spaces in Latin text set in Amiri Quran come from the Arabic file too (same design, same width). Whether WKWebView and WebView2 showed the boxes as well is visible in the CI screenshots of this change.
