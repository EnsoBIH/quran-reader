import type { Highlight, QuranText } from "@quran-reader/types";
import { describe, expect, it } from "vitest";
import { buildMushaf } from "./mushaf.ts";
import {
  copyText,
  EMPTY_STUDY,
  indexStudy,
  localDate,
  localDateTime,
  nextColor,
  portionLabel,
  portionRange,
  refLabel,
  upsert,
} from "./study.ts";

// A made-up two-surah text with placeholder verses (no Quran text in fixtures).
const surah = (number: number, ayahCount: number) => ({
  number,
  nameArabic: `name ${number}`,
  nameLatin: `Surah ${number}`,
  revelation: "meccan" as const,
  ayahCount,
});
const text: QuranText = {
  surahs: [surah(1, 3), surah(2, 4)],
  verses: Array.from({ length: 7 }, (_, i) => `verse ${i}`),
  juzStarts: [0],
  quarterStarts: [0],
  pageStarts: [0],
  sajdas: [],
};
const m = buildMushaf(text);
const r = (surah: number, ayah: number) => ({ surah, ayah });

describe("study helpers", () => {
  it("toggles a colour off and switches to another", () => {
    expect(nextColor(undefined, 3)).toBe(3);
    expect(nextColor(3, 3)).toBeNull();
    expect(nextColor(3, 5)).toBe(5);
  });

  it("replaces, adds and removes entries in verse order", () => {
    const h = (surah: number, ayah: number, color: 1 | 2): Highlight => ({
      ref: r(surah, ayah),
      color,
      updatedAt: "2026-10-22T00:00:00Z",
    });
    let list = upsert([], r(2, 1), h(2, 1, 1));
    list = upsert(list, r(1, 2), h(1, 2, 1));
    list = upsert(list, r(2, 1), h(2, 1, 2));
    expect(list.map((x) => [x.ref.surah, x.ref.ayah, x.color])).toEqual([
      [1, 2, 1],
      [2, 1, 2],
    ]);
    expect(upsert(list, r(1, 2), null)).toHaveLength(1);
  });

  it("indexes by global verse", () => {
    const index = indexStudy(m, {
      ...EMPTY_STUDY,
      highlights: [{ ref: r(2, 2), color: 4, updatedAt: "" }],
      notes: [{ ref: r(1, 1), body: "n", createdAt: "", updatedAt: "" }],
      bookmarks: [{ ref: r(2, 4), createdAt: "" }],
    });
    expect([...index.highlights]).toEqual([[4, 4]]);
    expect([...index.notes]).toEqual([[0, "n"]]);
    expect([...index.bookmarks]).toEqual([[6, ""]]);
  });

  it("copies the verse with its reference and attributions", () => {
    expect(copyText(m, 4)).toBe("verse 4\n\n— Surah 2 2:2\nQuran text: Tanzil.net (CC BY 3.0)");
    expect(copyText(m, 4, { name: "Some Translation", verse: "translated 4" })).toBe(
      "verse 4\n\ntranslated 4\n\n— Surah 2 2:2\nQuran text: Tanzil.net (CC BY 3.0) · Translation: Some Translation",
    );
    // A translation without this verse is left out with its credit.
    expect(copyText(m, 0, { name: "X", verse: undefined })).toBe(
      "verse 0\n\n— Surah 1 1:1\nQuran text: Tanzil.net (CC BY 3.0)",
    );
  });

  it("formats local dates and plan portions", () => {
    const d = new Date(2026, 0, 5, 7, 3);
    expect(localDate(d)).toBe("2026-01-05");
    expect(localDateTime(d)).toBe("2026-01-05 07:03");
    expect(refLabel(m, r(2, 3))).toBe("Surah 2 2:3");
    const p = { day: 2, from: r(1, 3), to: r(2, 2), fromIndex: 2, toIndex: 4 };
    expect(portionLabel(m, 30, p)).toBe("Juz 2");
    expect(portionLabel(m, 60, p)).toBe("Hizb 2");
    expect(portionLabel(m, 7, p)).toBe("Surah 1 1:3 – Surah 2 2:2");
    expect(portionRange(m, { ...p, from: r(2, 1) })).toBe("Surah 2 2:1 – 2:2");
  });
});
