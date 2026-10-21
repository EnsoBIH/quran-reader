import { describe, expect, it } from "vitest";
import { arabicRanges, snippet, textRanges } from "./search.ts";

// Synthetic text only: "w00 w01 … w59", every word three letters and a space.
const text = Array.from({ length: 60 }, (_, i) => `w${String(i).padStart(2, "0")}`).join(" ");
const at = (word: number) => ({ start: word * 4, end: word * 4 + 3 });

describe("snippet", () => {
  it("returns short texts whole", () => {
    expect(snippet("one two", [{ start: 4, end: 7 }])).toEqual({
      text: "one two",
      ranges: [{ start: 4, end: 7 }],
      cutStart: false,
      cutEnd: false,
    });
  });

  it("cuts around the first match at word boundaries", () => {
    const s = snippet(text, [at(30), at(31)], 10, 10);
    expect(s.cutStart).toBe(true);
    expect(s.cutEnd).toBe(true);
    expect(s.text.startsWith("w")).toBe(true);
    expect(s.text.endsWith(" ")).toBe(false);
    expect(text.includes(s.text)).toBe(true);
    for (const r of s.ranges) expect(s.text.slice(r.start, r.end)).toMatch(/^w3[01]$/);
    expect(s.ranges).toHaveLength(2);
  });

  it("keeps the start when the match is near it", () => {
    const s = snippet(text, [at(1)], 10, 10);
    expect(s.cutStart).toBe(false);
    expect(s.text.startsWith("w00")).toBe(true);
    expect(s.text.slice(s.ranges[0]?.start, s.ranges[0]?.end)).toBe("w01");
  });

  it("drops matches outside the snippet", () => {
    const s = snippet(text, [at(5), at(55)], 8, 8);
    expect(s.ranges).toHaveLength(1);
  });
});

describe("arabicRanges", () => {
  it("picks the Arabic text's ranges", () => {
    const hit = {
      ref: { surah: 1, ayah: 1 },
      index: 0,
      matches: [
        { text: "en.test", ranges: [{ start: 0, end: 1 }] },
        { text: "quran", ranges: [{ start: 2, end: 3 }] },
      ],
    };
    expect(arabicRanges(hit)).toEqual([{ start: 2, end: 3 }]);
    expect(arabicRanges({ ...hit, matches: [] })).toEqual([]);
  });
});

describe("textRanges", () => {
  it("picks the ranges of one text of a hit", () => {
    const hit = {
      ref: { surah: 1, ayah: 1 },
      index: 0,
      matches: [
        { text: "quran", ranges: [{ start: 0, end: 3 }] },
        { text: "en.synthetic", ranges: [{ start: 4, end: 9 }] },
      ],
    };
    expect(arabicRanges(hit)).toEqual([{ start: 0, end: 3 }]);
    expect(textRanges(hit, "en.synthetic")).toEqual([{ start: 4, end: 9 }]);
    expect(textRanges(hit, "missing")).toEqual([]);
  });
});
