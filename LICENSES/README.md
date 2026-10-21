# Third-party content

The code in this repository is MIT-licensed (see `/LICENSE`). Everything listed here keeps its own license. Every bundled file must have an entry with source, license and checksum.

| Component | Source | License | Bundled | Status |
| --- | --- | --- | --- | --- |
| Arabic text (Uthmani) | [Tanzil](https://tanzil.net/pub/download/v1.0/) | Tanzil terms of use (CC BY 3.0, verbatim redistribution only) | Yes, unmodified | Pending — added by `xtask` in Phase 1 |
| Metadata (surahs, juz, hizb, pages, sajda) | [Tanzil](https://tanzil.net/pub/download/v1.0/) | as above | Yes | Pending — Phase 1 |
| Font: Amiri Quran | [aliftype/amiri](https://github.com/aliftype/amiri) via [Fontsource](https://fontsource.org/fonts/amiri-quran) | SIL OFL 1.1 — see [`OFL-Amiri.txt`](OFL-Amiri.txt) | Yes | Active |
| Font: KFGQPC Uthmanic Hafs | King Fahd Complex | unclear / partly proprietary | **No** | Waiting for written permission |
| Tanzil translations (Bubenheim, Khoury, Zaidan, …) | Tanzil | non-commercial only | **No** — user import | — |
| Public-domain translations (e.g. Henning 1901, Pickthall 1930) | original editions | public domain, check per edition and country | After review | Open |

## Rules

- Tanzil text is never modified. The exact Tanzil notice is copied verbatim into `TANZIL.txt` when `xtask` first fetches the source.
- Each source in `data/sources.toml` is pinned by URL and SHA-256.
- The in-app credits page lists every bundled item from this table.
