import type { QuranText, SajdaKind, SurahEntry, VerseRef } from "@quran-reader/types";

// The reader's view of the whole text: one flat list of rows (a header per surah, then its
// verses) plus per-verse markers, all indexed by global verse index (1:1 is 0).

/** Marker bits per verse. */
export const STARTS_JUZ = 1;
export const STARTS_QUARTER = 2;
export const STARTS_PAGE = 4;

export interface Mushaf {
  surahs: SurahEntry[];
  /** Unmodified Tanzil text by global verse index. */
  verses: string[];
  /** Per row: a global verse index (≥ 0), or −surah for the header of that surah. */
  rows: Int32Array;
  /** Global verse index → row. */
  verseRow: Int32Array;
  /** Surah number → row of its header (index 0 unused). */
  surahRow: Int32Array;
  /** Surah number → global index of its first verse (index 0 unused). */
  firstVerse: Int32Array;
  /** Global verse index → surah number. */
  surahOf: Uint8Array;
  juzOf: Uint8Array;
  /** Hizb quarter 1–240; the hizb is `ceil(quarter / 4)`. */
  quarterOf: Uint8Array;
  pageOf: Uint16Array;
  /** STARTS_* bits. */
  starts: Uint8Array;
  sajdas: Map<number, SajdaKind>;
  juzStarts: number[];
  quarterStarts: number[];
  pageStarts: number[];
}

export function buildMushaf(text: QuranText): Mushaf {
  const count = text.verses.length;
  const rows = new Int32Array(count + text.surahs.length);
  const verseRow = new Int32Array(count);
  const surahRow = new Int32Array(text.surahs.length + 1);
  const firstVerse = new Int32Array(text.surahs.length + 1);
  const surahOf = new Uint8Array(count);
  let row = 0;
  let verse = 0;
  for (const s of text.surahs) {
    surahRow[s.number] = row;
    firstVerse[s.number] = verse;
    rows[row++] = -s.number;
    for (let a = 0; a < s.ayahCount; a++) {
      surahOf[verse] = s.number;
      verseRow[verse] = row;
      rows[row++] = verse++;
    }
  }
  if (verse !== count) throw new Error(`surah verse counts add up to ${verse}, not ${count}`);

  const starts = new Uint8Array(count);
  const ordinals = <T extends Uint8Array | Uint16Array>(out: T, list: number[], bit: number) => {
    list.forEach((start, i) => {
      starts[start] = (starts[start] ?? 0) | bit;
      const end = list[i + 1] ?? count;
      out.fill(i + 1, start, end);
    });
    return out;
  };
  return {
    surahs: text.surahs,
    verses: text.verses,
    rows,
    verseRow,
    surahRow,
    firstVerse,
    surahOf,
    juzOf: ordinals(new Uint8Array(count), text.juzStarts, STARTS_JUZ),
    quarterOf: ordinals(new Uint8Array(count), text.quarterStarts, STARTS_QUARTER),
    pageOf: ordinals(new Uint16Array(count), text.pageStarts, STARTS_PAGE),
    starts,
    sajdas: new Map(text.sajdas.map((s) => [s.index, s.kind])),
    juzStarts: text.juzStarts,
    quarterStarts: text.quarterStarts,
    pageStarts: text.pageStarts,
  };
}

export function refAt(m: Mushaf, index: number): VerseRef {
  const surah = m.surahOf[index] ?? 1;
  return { surah, ayah: index - (m.firstVerse[surah] ?? 0) + 1 };
}

/** Global index of `ref`, clamped to existing verses. */
export function indexOf(m: Mushaf, ref: VerseRef): number {
  const surah = Math.min(Math.max(ref.surah, 1), m.surahs.length);
  const count = m.surahs[surah - 1]?.ayahCount ?? 1;
  const ayah = Math.min(Math.max(ref.ayah, 1), count);
  return (m.firstVerse[surah] ?? 0) + ayah - 1;
}

export function rowOf(m: Mushaf, ref: VerseRef): number {
  // Verse 1 starts at the surah header, so the header and Basmala come into view with it.
  if (ref.ayah <= 1) return m.surahRow[Math.min(Math.max(ref.surah, 1), m.surahs.length)] ?? 0;
  return m.verseRow[indexOf(m, ref)] ?? 0;
}

/** The verse a row stands for: itself, or verse 1 for a surah header. */
export function refOfRow(m: Mushaf, row: number): VerseRef {
  const r = m.rows[row] ?? 0;
  return r < 0 ? { surah: -r, ayah: 1 } : refAt(m, r);
}

/** `Hizb 5`, `¼ Hizb 5`, `½ Hizb 5`, `¾ Hizb 5` for hizb quarter 1–240. */
export function quarterLabel(quarter: number): string {
  const hizb = Math.ceil(quarter / 4);
  const fraction = ["", "¼ ", "½ ", "¾ "][(quarter - 1) % 4];
  return `${fraction}Hizb ${hizb}`;
}
