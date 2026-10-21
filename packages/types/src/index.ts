// Shared domain types. Keep in sync with the serde types in crates/quran-core.

/** 1..=114 */
export type SurahNumber = number;
/** 1-based verse number within a surah. */
export type AyahNumber = number;

export type Revelation = "meccan" | "medinan";

export type SajdaKind = "recommended" | "obligatory";

export interface VerseRef {
  surah: SurahNumber;
  ayah: AyahNumber;
}

export interface Surah {
  number: SurahNumber;
  nameArabic: string;
  nameLatin: string;
  nameTranslated?: string;
  revelation: Revelation;
  ayahCount: number;
}

export interface Ayah {
  ref: VerseRef;
  /** Unmodified Tanzil text. Never normalize this for display. */
  text: string;
  juz: number;
  hizbQuarter: number;
  page: number;
  sajda?: SajdaKind;
}

/** Returned by the `surah` Tauri command. */
export interface SurahText {
  surah: Surah;
  /** Basmala shown above verse 1. Absent for surah 1 (it is verse 1 there) and surah 9. */
  bismillah?: string;
  ayahs: Ayah[];
}

/** A division of the Madina Mushaf: 30 juz, 60 hizbs (of four quarters each), 604 pages. */
export type Part = "juz" | "hizb" | "page";

/** What a ⌘K palette entry points at. */
export type Target =
  | { kind: "verse" }
  | { kind: "surah" }
  | { kind: "named"; name: string }
  | { kind: "part"; part: Part; number: number };

/** One ⌘K palette entry, returned by the `lookup` Tauri command. */
export interface Hit {
  target: Target;
  /** Where reading starts. */
  ref: VerseRef;
}

/** A prostration verse, by global verse index (1:1 is 0). */
export interface Sajda {
  index: number;
  kind: SajdaKind;
}

export interface SurahEntry extends Surah {
  /** Basmala shown above verse 1. Absent for surah 1 (it is verse 1 there) and surah 9. */
  bismillah?: string;
}

/**
 * The whole Quran for continuous reading, returned once by the `quran_text` Tauri command.
 * Verses are addressed by global index (1:1 is 0, 114:6 is 6235).
 */
export interface QuranText {
  surahs: SurahEntry[];
  /** Unmodified Tanzil text of every verse, in Quran order. Never normalize for display. */
  verses: string[];
  /** Global index of the first verse of each juz. */
  juzStarts: number[];
  /** Global index of the first verse of each hizb quarter (four per hizb). */
  quarterStarts: number[];
  /** Global index of the first verse of each page. */
  pageStarts: number[];
  sajdas: Sajda[];
}

/**
 * A range of an unmodified text in UTF-16 code units, i.e. JavaScript string indices
 * (`text.slice(start, end)`). Never splits a letter from its tashkeel.
 */
export interface TextRange {
  start: number;
  end: number;
}

/** Matched ranges in one searchable text of a verse. */
export interface TextMatches {
  /** `"quran"` for the Arabic; translation ids later. */
  text: string;
  /** Sorted, non-overlapping. */
  ranges: TextRange[];
}

/** A verse that matches a search. */
export interface SearchHit {
  ref: VerseRef;
  /** Global verse index (1:1 is 0). */
  index: number;
  matches: TextMatches[];
}

/** Returned by the `search` Tauri command. */
export interface SearchResults {
  /** All matching verses, also those beyond the limit. */
  total: number;
  /** Matching verses in Quran order, at most the limit. */
  hits: SearchHit[];
}

/** Arabic only, Arabic with the translation below, or both side by side. */
export type ReaderView = "arabic" | "stacked" | "columns";

export type HighlightColor = 1 | 2 | 3 | 4 | 5 | 6;

export interface Highlight {
  id: string;
  ref: VerseRef;
  color: HighlightColor;
  createdAt: string;
}

export interface Note {
  id: string;
  ref: VerseRef;
  body: string;
  createdAt: string;
  updatedAt: string;
}

export interface Bookmark {
  id: string;
  ref: VerseRef;
  label?: string;
  createdAt: string;
}

export type Theme = "light" | "sepia" | "dark";

/** Returned by the `core_info` Tauri command. */
export interface CoreInfo {
  version: string;
  surahCount: number;
  ayahCount: number;
  dataLoaded: boolean;
}
