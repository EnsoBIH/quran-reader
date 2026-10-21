# quran-reader

A fast, beautifully typeset Quran reader. Open source (MIT), no accounts, no tracking — all notes stay on your machine.

> Status: **Phase 0 — scaffolding.** The repo builds and runs, but no Quran text is bundled yet. The data pipeline (`xtask`) lands in Phase 1.

## Goals

- **Instant** — text on the first frame, full-text search in milliseconds.
- **Keyboard first** — ⌘K for references (`2:255`, `baqara 255`, `بقرة`), ⌘F to search, ← → for surahs.
- **Typography** — calm layout, light / sepia / dark, adjustable size and measure; proper Arabic shaping and RTL.
- **Local study** — highlights in six colors, notes, bookmarks in a local SQLite file, export to Markdown/JSON.
- **Clean licensing** — only freely redistributable content in the repo, pinned sources with SHA-256, reproducible data build.

Inspired by [bible-rs](https://github.com/michaelshimeles/bible-rs).

## Layout

```
apps/
  desktop/          Tauri 2 app: React + Vite + Tailwind
    src-tauri/      Tauri shell, commands call into quran-core
  web/              Landing page (Astro): /, /download, /privacy
packages/
  ui/               Shared React components + theme (Ayah, SurahHeader)
  types/            Shared TS types (Surah, Ayah, VerseRef, Highlight)
  config/           Shared tsconfig presets
crates/
  quran-core/       Model, binary format, reference parser, search, store + `quran` CLI
  xtask/            Reproducible data build from pinned sources
data/               Generated data files (freely licensed only)
LICENSES/           License and provenance per data source and font
```

Two workspaces in one repo: pnpm + Turborepo for JS/TS, Cargo for Rust.

## Development

Requirements: Node ≥ 24, pnpm 11, Rust (pinned in `rust-toolchain.toml`), and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
pnpm install
pnpm dev:desktop     # Tauri app with hot reload
pnpm dev:web         # landing page on http://localhost:4321
```

Checks:

```bash
pnpm lint typecheck test build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Rust CLI and data build:

```bash
cargo quran read 2:255
cargo xtask --help
```

Release bundle of the desktop app: `pnpm --filter @quran-reader/desktop tauri:build`.

## Licensing

Code: [MIT](LICENSE). Data and fonts keep their own licenses — see [LICENSES/](LICENSES/README.md). The Arabic text is shown exactly as published by [Tanzil.net](https://tanzil.net) and is never modified.

Design decisions are recorded in [DECISIONS.md](DECISIONS.md).
