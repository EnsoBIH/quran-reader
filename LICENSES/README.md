# Third-party content

The code in this repository is MIT-licensed (see `/LICENSE`). Everything listed here keeps its own license. Every bundled file must have an entry with source, license and checksum.

| Component | Source | License | Bundled | Status |
| --- | --- | --- | --- | --- |
| Arabic text (Uthmani, Tanzil version 1.0.2) | [Tanzil](https://tanzil.net/pub/download/v1.0/), pinned in [`data/sources.toml`](../data/sources.toml) (`tanzil-uthmani`, SHA-256 `faa75f9a…2725`) | Tanzil terms of use (CC BY 3.0, verbatim copies only) — see [`TANZIL.txt`](TANZIL.txt) | Yes, unmodified, in `data/quran.bin` | Active |
| Metadata 1.0 (surahs, juz, hizb quarters, pages, sajdas) | [Tanzil](https://tanzil.net/docs/quran_metadata), pinned in [`data/sources.toml`](../data/sources.toml) (`tanzil-metadata`, SHA-256 `8867c1d8…5c7a`) | CC BY, © 2008–2009 Tanzil.info (`license="cc-by"` in the file) | Yes, in `data/quran.bin` | Active |
| Font: Amiri Quran | [aliftype/amiri](https://github.com/aliftype/amiri) via [Fontsource](https://fontsource.org/fonts/amiri-quran) | SIL OFL 1.1 — see [`OFL-Amiri.txt`](OFL-Amiri.txt) | Yes | Active |
| Font: KFGQPC Uthmanic Hafs | King Fahd Complex | unclear / partly proprietary | **No** | Waiting for written permission |
| English translation: Pickthall, *The Meaning of the Glorious Koran* (1930) | Verses marked `P:` of [gutenberg.org eBook #16955](https://www.gutenberg.org/ebooks/16955), pinned in [`data/sources.toml`](../data/sources.toml) (`gutenberg-16955`, SHA-256 `3b96fa3a…d8bc`) | Public domain (translator † 1936; published 1930) — see [`PICKTHALL.txt`](PICKTHALL.txt) and the review below | Yes, verses unmodified, in `data/translations/en.pickthall.bin` | Active |
| Tanzil translations (Bubenheim, Khoury, Zaidan, Pickthall, …) | [Tanzil](https://tanzil.net/trans/) | "for non-commercial purposes only" (page-wide terms) | **No** — user import ([how](../docs/translation-import.md)) | — |
| Public-domain translations | see the review below | public domain, checked per edition and country | See the review | Reviewed 2026-10-21 |

## Translation review (2026-10-21)

A translation is bundled only if (1) the edition is public domain in Germany/EU and in the US,
(2) the digital file we take it from may be redistributed by anyone, also commercially (the
code is MIT), and (3) the file is aligned to the 6,236 verses of the Kufan count, or can be
aligned without editorial judgement. A public-domain original does not make a digital copy
clean: the copy may be a later, copyrighted revision, or its site may restrict use. Rules used:
Germany/EU 70 years after the author's death (§ 64 UrhG, Directive 2006/116/EC; § 70/§ 71
UrhG rights of 25 years for scholarly or first editions do not apply to any candidate); US
works published before 1931 are public domain (on 2026-01-01 works of 1930 entered it),
later foreign works were restored by the URAA (17 U.S.C. § 104A) and run 95 years from
publication.

The research (2026-10-21) looked at the sources linked below and compared digital texts with
scans of the first editions.

### English

| Edition | Translator | Germany/EU | US | Digital sources, terms, alignment |
| --- | --- | --- | --- | --- |
| **Pickthall, *The Meaning of the Glorious Koran*, London: Knopf / Allen & Unwin, 1930** | Marmaduke Pickthall, 1875–1936 | PD since 2007-01-01 | PD since 2026-01-01 (published 1930) | See below. The 1930 print does not use the Kufan count throughout (al-Fatiha: Basmala unnumbered and verse 7 split; surah 6 ends at 166). |
| Rodwell, *The Koran*, 1861 | J. M. Rodwell, 1808–1900 | PD | PD | Project Gutenberg #2800/#3434: surahs in chronological order, no verse numbers in the text. Not alignable without editorial work. |
| Palmer, *The Qur'ân*, 1880 (Sacred Books of the East 6, 9) | E. H. Palmer, 1840–1882 | PD | PD | No verse-aligned digital text of known origin (fawazahmed0's copy has no source). |
| Sale, *The Koran*, 1734 | George Sale, 1697–1736 | PD | PD | Project Gutenberg #7440: prose with notes, no verse numbers. |
| Muhammad Ali, *The Holy Qur-án*, Woking, 1917 | Muhammad Ali, 1874–1951 | PD since 2022 | PD (1917) | Scanned PDF only; numbering differs from the standard in places; later revisions are copyrighted. |
| Yusuf Ali, Lahore, 1934–38 | A. Yusuf Ali, 1872–1953 | PD since 2024 | **Not PD** (URAA restoration, 95 years: ~2030–2034) | — |
| Shakir (1980s) | disputed | unclear | unclear | Reportedly copied from Muhammad Ali 1917; status unclear. |

Digital sources for Pickthall:

- **Tanzil `en.pickthall`** — terms: "The translations provided at this page are for
  non-commercial purposes only." No provenance given; 35 verses read "no Allah save Him"
  (a blind God → Allah replacement). **Rejected** (terms and text).
- **fawazahmed0/quran-api, risan/quran-json** — copies of the Tanzil file (source given as
  tanzil.net), so the same text and the same terms underneath. **Rejected.**
- **quran.com / Quran Foundation API** — licence to display inside an app only, no
  redistribution. **Rejected.**
- **Project Gutenberg #16955**, "Three translations of the Koran side by side" (Yusuf Ali,
  Pickthall, Shakir; released 2005, updated 2020-12-12). Marked public domain in the US;
  outside the PG trademark, PG's licence lets anyone do anything with a US public-domain text
  once the Project Gutenberg name and licence are removed. Pickthall's verses are the `P:`
  paragraphs, numbered by the Kufan count (6,236; per-surah counts printed in the file).
  Provenance per its own note: "a file whose origins we don't know", re-proofed against
  paper copies. Compared with the 1930 scan, the wording is not verbatim 1930: some "God"
  became "Allah" (2:163 "Your Allah is One Allah; there is no God save Him", 1930: "Your God
  is One God; …"), 1:4 "Master" for 1930 "Owner", punctuation. These are mechanical
  substitutions and the verse division follows the standard count — neither is an original
  creation that could carry a copyright of its own (EU: author's own intellectual creation;
  US: *Feist*), so the text stays public domain. **Clean to bundle** if labelled honestly
  as this digital text, not as the 1930 print. Only the `P:` paragraphs may be taken (Yusuf
  Ali is not public domain in the US; Shakir is unclear).
- **archive.org `in.ernet.dli.2015.216140`** (scan of the 1930 Allen & Unwin / Knopf print,
  undated impression) — public domain scan, but OCR only and the 1930 numbering: a true 1930
  text would have to be proofread and re-cut by us.

### German

| Edition | Translator | Germany/EU | US | Digital sources, terms, alignment |
| --- | --- | --- | --- | --- |
| **Henning, *Der Koran*, Leipzig: Reclam (Universal-Bibliothek), 1901** | Max Henning, 1861–1927 | PD since 1998-01-01 | PD (1901) | Numbered after Flügel, not Kufan, so verses must be re-cut, not renumbered. Revisions are protected: Schimmel 1960 (notes, introduction; † 2003), Kurt Rudolph / Ernst Werner 1968 ff. (wording revised; Rudolph † 2020), Hofmann 1998 (rewritten). |
| Goldschmidt, *Der Koran*, Berlin: Brandus, 1916 | Lazarus Goldschmidt, 1871–1950 | PD since 2021-01-01 | PD (1916) | No digital text found; numbering unknown. |
| Ullmann, *Der Koran*, Crefeld: Funcke, 1840 | Ludwig Ullmann, 1804–1843 | PD | PD | archive.org Fraktur scans, unusable OCR, no verse numbers in the text. |
| Grigull, *Der Koran*, Halle: Hendel, 1901 | Theodor Fr. Grigull, 1850–1915 | PD | PD | archive.org scan with good OCR, but running prose without verse numbers. |
| Rückert (partial, 1888) | Friedrich Rückert, 1788–1866 | PD | PD | Incomplete; modern editions carry editorial rights. |

Digital sources for Henning:

- **projekt-gutenberg.org (Henning, credited "Reclam jun., 1970")** — the Rudolph revision
  (e.g. 4:34 "Frauen", where 1901 has "Weibern"), so copyrighted; terms: "Die Nutzung der
  auf dieser Website bereitgestellten Texte ist ausschließlich zum privaten Gebrauch
  gestattet." **Rejected**, also as a reference.
- **gutenberg.org, de.wikisource, zeno.org, TextGrid, DTA** — no German Quran translation.
- **ULB Sachsen-Anhalt (doi:10.25673/97993) and archive.org `derkoranausdemar00henn`** —
  scans of the 1901 Fraktur print (Halle's record reportedly carries the Public Domain Mark;
  not verified, the repository did not respond). Images or unusable OCR, Flügel numbering.
- **fawazahmed0/quran-api, Tanzil** — only Abu Rida, Khoury, Zaidan, Bubenheim/Elyas: all
  protected, non-commercial at best.

**Result:** no German public-domain translation exists as a clean, verse-aligned digital
text. Bundling one means producing it ourselves (OCR of the 1901 Henning scan, proofreading,
re-cutting Flügel to Kufan verses) — a project of its own. German readers import a
translation they downloaded themselves.

Decision on what is bundled (Pickthall via eBook #16955; no German translation): [DECISIONS.md](../DECISIONS.md) D-017.

The bundled file carries no Project Gutenberg header, licence or name; the source is named
only as provenance in `PICKTHALL.txt` and on the credits page.

## Rules

- Tanzil text is never modified. `cargo xtask build` copies the Tanzil copyright block verbatim from the pinned source into `TANZIL.txt` and embeds it in `data/quran.bin`; `cargo xtask verify` fails if either differs. Don't edit `TANZIL.txt` by hand.
- Each source in `data/sources.toml` is pinned by URL and SHA-256.
- Bundled translations are listed in `BUNDLED` in `crates/xtask/src/main.rs`; each needs a
  review above, a notice file here (embedded verbatim in its data file) and a pinned source.
  `cargo xtask verify` fails on any file in `data/translations/` without one.
- The in-app credits page (footer → Credits) lists every bundled item from this table.
