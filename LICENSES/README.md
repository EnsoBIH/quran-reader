# Third-party content

The code in this repository is MIT-licensed (see `/LICENSE`). Everything listed here keeps its own license. Every bundled file must have an entry with source, license and checksum.

| Component | Source | License | Bundled | Status |
| --- | --- | --- | --- | --- |
| Arabic text (Uthmani, Tanzil version 1.0.2) | [Tanzil](https://tanzil.net/pub/download/v1.0/), pinned in [`data/sources.toml`](../data/sources.toml) (`tanzil-uthmani`, SHA-256 `faa75f9a…2725`) | Tanzil terms of use (CC BY 3.0, verbatim copies only) — see [`TANZIL.txt`](TANZIL.txt) | Yes, unmodified, in `data/quran.bin` | Active |
| Metadata 1.0 (surahs, juz, hizb quarters, pages, sajdas) | [Tanzil](https://tanzil.net/docs/quran_metadata), pinned in [`data/sources.toml`](../data/sources.toml) (`tanzil-metadata`, SHA-256 `8867c1d8…5c7a`) | CC BY, © 2008–2009 Tanzil.info (`license="cc-by"` in the file) | Yes, in `data/quran.bin` | Active |
| Font: Amiri Quran | [aliftype/amiri](https://github.com/aliftype/amiri) via [Fontsource](https://fontsource.org/fonts/amiri-quran) | SIL OFL 1.1 — see [`OFL-Amiri.txt`](OFL-Amiri.txt) | Yes | Active |
| Font: KFGQPC Uthmanic Hafs | King Fahd Complex | unclear / partly proprietary | **No** | Waiting for written permission |
| Tanzil translations (Bubenheim, Khoury, Zaidan, …) | Tanzil | non-commercial only | **No** — user import | — |
| Public-domain translations (e.g. Henning 1901, Pickthall 1930) | original editions | public domain, check per edition and country | After review | Open |

## Rules

- Tanzil text is never modified. `cargo xtask build` copies the Tanzil copyright block verbatim from the pinned source into `TANZIL.txt` and embeds it in `data/quran.bin`; `cargo xtask verify` fails if either differs. Don't edit `TANZIL.txt` by hand.
- Each source in `data/sources.toml` is pinned by URL and SHA-256.
- The in-app credits page lists every bundled item from this table.
