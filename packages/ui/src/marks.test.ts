import { describe, expect, it } from "vitest";
import { segments } from "./marks.ts";

const join = (s: { text: string }[]) => s.map((x) => x.text).join("");

describe("segments", () => {
  it("keeps the text unchanged", () => {
    const text = "alpha beta gamma";
    const s = segments(text, [
      { start: 0, end: 5 },
      { start: 11, end: 16 },
    ]);
    expect(join(s)).toBe(text);
    expect(s).toEqual([
      { text: "alpha", marked: true },
      { text: " beta ", marked: false },
      { text: "gamma", marked: true },
    ]);
  });

  it("returns one plain run without ranges", () => {
    expect(segments("abc")).toEqual([{ text: "abc", marked: false }]);
    expect(segments("")).toEqual([{ text: "", marked: false }]);
  });

  it("clips and merges bad ranges", () => {
    const s = segments("abcdef", [
      { start: 1, end: 3 },
      { start: 2, end: 4 },
      { start: 5, end: 99 },
    ]);
    expect(join(s)).toBe("abcdef");
    expect(s.filter((x) => x.marked).map((x) => x.text)).toEqual(["bc", "d", "f"]);
  });

  it("counts UTF-16 units like the core does", () => {
    const s = segments("a😀bc", [{ start: 3, end: 5 }]);
    expect(s).toEqual([
      { text: "a😀", marked: false },
      { text: "bc", marked: true },
    ]);
  });
});
