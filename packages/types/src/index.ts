// Shared domain types. Keep in sync with the serde types in crates/quran-core.

/** 1..=114 */
export type SurahNumber = number;
/** 1-based verse number within a surah. */
export type AyahNumber = number;

export type Revelation = "meccan" | "medinan";

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
  sajda?: boolean;
}

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
