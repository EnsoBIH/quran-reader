# Content review

Before 1.0 (and again whenever the text, the metadata, a bundled translation or the
renderer changes), people with knowledge of Arabic and of the Quran check what the app
shows. Code review and tests can show that the text is *byte for byte* Tanzil's; only
people can say that it *reads* correctly in every webview and that the app speaks about it
with care.

How to take part: pick an area below, open a
[Content review issue](https://github.com/EnsoBIH/quran-reader/issues/new?template=content-review.yml)
with what you compared against and what you found (each finding with its verse reference),
and the area is entered in the sign-off table at the end. One reviewer per area is the
minimum; areas A and B should have two, independently.

What to review with:

- The app itself (Releases, or `pnpm dev:desktop` from source). Settings (Aa) change the
  theme, view and sizes; ⌘K jumps to any reference.
- The screenshot sheets of the three webviews: `apps/desktop/e2e/screenshots/wkwebview/`
  (macOS), `webview2/` (Windows) and `webkitgtk/` (Linux). They are rendered from the app's
  own data and components (`apps/desktop/e2e/verses.json`, generated, never typed by hand).
  Every character that occurs anywhere in the text is on these sheets at least once.
- A printed Madina Mushaf (King Fahd Complex, Hafs ʿan ʿĀṣim) or another mushaf you trust,
  and [tanzil.net](https://tanzil.net) for the source text.

## A. Arabic text and its rendering

The text is Tanzil's Uthmani text 1.0.2, unmodified (DECISIONS.md D-004, D-009), in the font
Amiri Quran (D-005). On each of the three screenshot sheets, and in the app at two sizes:

- [ ] Letters join correctly; no isolated forms inside a word, no gaps, no boxes or `◌`.
- [ ] Every haraka, shadda, sukun and tanwin sits on its own letter, not on its neighbour,
      and none collides with the line above or below.
- [ ] Superscript alif (ٰ), small waw and ya (ۥ ۦ), small high letters (ۜ ۨ), the iqlab
      mim (ۢ ۭ), madd (ٓ) and hamza forms are drawn and placed as in the mushaf.
- [ ] Waqf marks (ۖ ۗ ۚ ۛ ۘ ۙ, including the paired ۛ … ۛ of 2:2) are present and placed
      above the right word.
- [ ] The rare marks of Hafs: 11:41 (imala, ۪), 12:11 (ishmam, ۫), 41:44 (tashil, ۬),
      21:88 (ۨ), and the sakt places (18:1, 36:52, 75:27, 83:14).
- [ ] The sajda sign ۩ and the end-of-ayah sign with its number (۝ with the number inside,
      in Arabic-Indic digits) look right; verse numbers match the mushaf.
- [ ] Disjoined letters (الٓمٓ، كٓهيعٓصٓ، حمٓ عٓسٓقٓ، نٓ …) are drawn with their madd.
- [ ] Long verses (2:282, 2:255, 24:35) break between words only, never inside a word, and
      never separate a mark from its letter. Try a narrow window too.
- [ ] Highlight colours (keys 1–6) and search matches (⌘F) do not change the shape or
      colour of the letters.
- [ ] Copy (⌘C on a verse) gives the same text as the mushaf when pasted elsewhere.

## B. Structure

- [ ] Al-Fatihah: the Basmala is verse 1. At-Tawbah (9): no Basmala. Every other surah: the
      Basmala on its own line above verse 1, not numbered. 95 and 97 spell it بِّسْمِ, as in
      the Madina Mushaf; 27:30 has it inside the verse.
- [ ] Verse counts follow the Kufan count (6,236): spot-check the surahs you know.
- [ ] Juz (30), hizb and quarter (240) and page (604, Madina Mushaf) starts: spot-check at
      least ten, e.g. juz 2 (2:142), juz 30 (78:1), page 604, the quarter marks in juz 1.
- [ ] Sajdas: the 15 places, and how the app labels them ("Sajda (recommended)" /
      "Sajda (obligatory)", from Tanzil's metadata). Schools differ on which are obligatory
      and on 22:77 and 38:24; say whether the labels should stay, change or go.
- [ ] Meccan / Medinan as shown in the surah headers.

## C. Surah names and named verses

- [ ] Arabic names, transliterations and English meanings in the surah headers and in ⌘K
      (Tanzil metadata).
- [ ] The other names ⌘K accepts (`crates/quran-core/src/names.rs`, e.g. Bani Isra'il,
      Al-Inshirah) are well known and correct.
- [ ] Named verses (D-013): Ayat al-Kursi 2:255, Ayat ad-Dayn 2:282, Amana ar-Rasul 2:285,
      Ayat al-Mubahala 3:61, Ayat an-Nur 24:35. Correct, and uncontroversial?

## D. Search

- [ ] ⌘F ignores tashkeel and letter variants (D-015). Try words you would search for
      (with and without hamza, ة/ه, ى/ي, with the article) and check nothing obvious is
      missed. Known gaps are listed in D-015 (superscript alif spellings such as
      ٱلْكِتَٰبِ, and يَٰٓأَيُّهَا as one word): are they acceptable for 1.0?

## E. Translation and attributions

- [ ] Pickthall (D-017) sits next to the right verse: check al-Fatihah, 2:255, 24:35, the
      last verses of 6, 9 and 114, and a few at random.
- [ ] The way the app describes it ("this digital text, not the 1930 print", the
      "God" → "Allah" substitutions) is accurate and fair.
- [ ] Credits (footer → Credits) name Tanzil, Amiri and Pickthall correctly, with the terms
      Tanzil asks for.

## F. Wording

- [ ] The app and the site (`apps/web`) speak about the Quran respectfully and accurately:
      terms (surah, ayah/verse, juz, hizb, sajda, khatm), the plan wording, the copy format
      (`— Surah name 2:255`), and nothing that implies the app is a mushaf authority or a
      tafsir.

## Sign-off

| Area | Reviewer | Issue | Date | Result |
| --- | --- | --- | --- | --- |
| A | | | | open |
| A | | | | open |
| B | | | | open |
| B | | | | open |
| C | | | | open |
| D | | | | open |
| E | | | | open |
| F | | | | open |

Findings that need a change in the *text* go to Tanzil first: the app shows Tanzil's text
and never edits it (CONTRIBUTING.md). Rendering, metadata labels, names and wording are
fixed here.
