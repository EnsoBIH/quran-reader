import { describe, expect, it } from "vitest";
import { toArabicIndic } from "./digits.ts";

describe("toArabicIndic", () => {
  it("maps every digit", () => {
    expect(toArabicIndic(1234567890)).toBe("١٢٣٤٥٦٧٨٩٠");
  });

  it("formats verse numbers", () => {
    expect(toArabicIndic(255)).toBe("٢٥٥");
    expect(toArabicIndic(0)).toBe("٠");
  });

  it("rejects invalid input", () => {
    expect(() => toArabicIndic(-1)).toThrow(RangeError);
    expect(() => toArabicIndic(1.5)).toThrow(RangeError);
  });
});
