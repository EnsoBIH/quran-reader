import type { QuranText } from "@quran-reader/types";
import { describe, expect, it } from "vitest";
import {
  buildMushaf,
  indexOf,
  quarterLabel,
  refAt,
  refOfRow,
  rowOf,
  STARTS_JUZ,
  STARTS_PAGE,
  STARTS_QUARTER,
} from "./mushaf.ts";

// A made-up three-surah text with placeholder verses (no Quran text in fixtures).
const surah = (number: number, ayahCount: number, bismillah?: string) => ({
  number,
  nameArabic: `name ${number}`,
  nameLatin: `Surah ${number}`,
  revelation: "meccan" as const,
  ayahCount,
  bismillah,
});
const text: QuranText = {
  surahs: [surah(1, 3), surah(2, 4, "basmala"), surah(3, 2, "basmala")],
  verses: Array.from({ length: 9 }, (_, i) => `verse ${i}`),
  juzStarts: [0, 5],
  quarterStarts: [0, 2, 5, 7],
  pageStarts: [0, 3, 4, 8],
  sajdas: [{ index: 6, kind: "obligatory" }],
};

describe("buildMushaf", () => {
  const m = buildMushaf(text);

  it("puts a header row before each surah", () => {
    expect([...m.rows]).toEqual([-1, 0, 1, 2, -2, 3, 4, 5, 6, -3, 7, 8]);
    expect(m.surahRow[2]).toBe(4);
    expect(m.verseRow[3]).toBe(5);
  });

  it("maps global indices and references", () => {
    expect(refAt(m, 0)).toEqual({ surah: 1, ayah: 1 });
    expect(refAt(m, 6)).toEqual({ surah: 2, ayah: 4 });
    expect(refAt(m, 8)).toEqual({ surah: 3, ayah: 2 });
    expect(indexOf(m, { surah: 2, ayah: 2 })).toBe(4);
    // Out-of-range references are clamped.
    expect(indexOf(m, { surah: 2, ayah: 99 })).toBe(6);
    expect(indexOf(m, { surah: 0, ayah: 0 })).toBe(0);
  });

  it("starts verse 1 at its surah header", () => {
    expect(rowOf(m, { surah: 2, ayah: 1 })).toBe(4);
    expect(rowOf(m, { surah: 2, ayah: 3 })).toBe(7);
    expect(refOfRow(m, 4)).toEqual({ surah: 2, ayah: 1 });
    expect(refOfRow(m, 7)).toEqual({ surah: 2, ayah: 3 });
  });

  it("assigns juz, quarter and page to every verse", () => {
    expect([...m.juzOf]).toEqual([1, 1, 1, 1, 1, 2, 2, 2, 2]);
    expect([...m.quarterOf]).toEqual([1, 1, 2, 2, 2, 3, 3, 4, 4]);
    expect([...m.pageOf]).toEqual([1, 1, 1, 2, 3, 3, 3, 3, 4]);
    expect(m.starts[5]).toBe(STARTS_JUZ | STARTS_QUARTER);
    expect(m.starts[3]).toBe(STARTS_PAGE);
    expect(m.starts[1]).toBe(0);
    expect(m.sajdas.get(6)).toBe("obligatory");
  });

  it("rejects inconsistent counts", () => {
    expect(() => buildMushaf({ ...text, verses: text.verses.slice(1) })).toThrow();
  });
});

describe("quarterLabel", () => {
  it("names hizb quarters", () => {
    expect(quarterLabel(1)).toBe("Hizb 1");
    expect(quarterLabel(2)).toBe("¼ Hizb 1");
    expect(quarterLabel(3)).toBe("½ Hizb 1");
    expect(quarterLabel(4)).toBe("¾ Hizb 1");
    expect(quarterLabel(240)).toBe("¾ Hizb 60");
  });
});
