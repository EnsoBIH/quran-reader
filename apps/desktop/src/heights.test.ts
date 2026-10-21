import { describe, expect, it } from "vitest";
import { Heights } from "./heights.ts";

describe("Heights", () => {
  it("sums estimates into offsets", () => {
    const h = new Heights(4, (i) => (i + 1) * 10);
    expect([0, 1, 2, 3, 4].map((i) => h.offset(i))).toEqual([0, 10, 30, 60, 100]);
    expect(h.total()).toBe(100);
  });

  it("updates later offsets after a measurement", () => {
    const h = new Heights(4, () => 10);
    h.total();
    expect(h.set(1, 25)).toBe(15);
    expect(h.set(1, 25)).toBe(0);
    expect(h.offset(1)).toBe(10);
    expect(h.offset(2)).toBe(35);
    expect(h.total()).toBe(55);
    h.set(0, 5);
    expect(h.offset(3)).toBe(40);
  });

  it("finds the row at a height", () => {
    const h = new Heights(3, () => 100);
    expect(h.rowAt(-5)).toBe(0);
    expect(h.rowAt(0)).toBe(0);
    expect(h.rowAt(99.5)).toBe(0);
    expect(h.rowAt(100)).toBe(1);
    expect(h.rowAt(250)).toBe(2);
    expect(h.rowAt(1e9)).toBe(2);
  });

  it("resets to new estimates", () => {
    const h = new Heights(2, () => 10);
    h.set(0, 40);
    h.reset(() => 20);
    expect(h.total()).toBe(40);
  });

  it("stays fast for the whole Quran", () => {
    const h = new Heights(6350, () => 120);
    const start = performance.now();
    for (let i = 0; i < 1000; i++) {
      h.set(i * 6, 130);
      h.rowAt(i * 700);
    }
    expect(performance.now() - start).toBeLessThan(200);
  });
});
