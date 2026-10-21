import { describe, expect, it } from "vitest";
import { canStep, EMPTY_HISTORY, parsePosition, step, visit } from "./history.ts";

const r = (surah: number, ayah: number) => ({ surah, ayah });

function must<T>(value: T | null): T {
  if (value === null) throw new Error("expected a step");
  return value;
}

describe("history", () => {
  it("records where the reader was and where it went", () => {
    const h = visit(EMPTY_HISTORY, r(1, 1), r(2, 255));
    expect(h.entries).toEqual([r(1, 1), r(2, 255)]);
    expect(h.index).toBe(1);
    expect(canStep(h, -1)).toBe(true);
    expect(canStep(h, 1)).toBe(false);
  });

  it("goes back and forward, remembering the scrolled position", () => {
    let h = visit(EMPTY_HISTORY, r(1, 1), r(2, 255));
    // Reader scrolled on to 2:260, then goes back.
    const back = step(h, r(2, 260), -1);
    expect(back?.to).toEqual(r(1, 1));
    h = must(back).history;
    const forward = step(h, r(1, 3), 1);
    expect(forward?.to).toEqual(r(2, 260));
    expect(forward?.history.entries).toEqual([r(1, 3), r(2, 260)]);
    expect(step(must(forward).history, r(2, 261), 1)).toBeNull();
  });

  it("drops forward entries on a new jump", () => {
    let h = visit(EMPTY_HISTORY, r(1, 1), r(2, 1));
    h = visit(h, r(2, 5), r(3, 1));
    h = must(step(h, r(3, 2), -1)).history;
    h = visit(h, r(2, 5), r(18, 1));
    expect(h.entries).toEqual([r(1, 1), r(2, 5), r(18, 1)]);
    expect(canStep(h, 1)).toBe(false);
  });

  it("does not record a jump to where the reader already is", () => {
    const h = visit(EMPTY_HISTORY, r(2, 255), r(2, 255));
    expect(h.entries).toEqual([r(2, 255)]);
    expect(canStep(h, -1)).toBe(false);
  });

  it("keeps at most 100 entries", () => {
    let h = EMPTY_HISTORY;
    for (let i = 1; i <= 150; i++) h = visit(h, r(1, 1), r(2, i));
    expect(h.entries.length).toBe(100);
    expect(h.entries.at(-1)).toEqual(r(2, 150));
  });

  it("is empty at first", () => {
    expect(step(EMPTY_HISTORY, r(1, 1), -1)).toBeNull();
    expect(canStep(EMPTY_HISTORY, -1)).toBe(false);
  });
});

describe("parsePosition", () => {
  it("accepts a verse reference", () => {
    expect(parsePosition({ surah: 2, ayah: 255 })).toEqual(r(2, 255));
  });

  it("rejects anything else", () => {
    for (const bad of [
      null,
      "2:255",
      { surah: 0, ayah: 1 },
      { surah: 115, ayah: 1 },
      { surah: 2 },
      { surah: 1.5, ayah: 1 },
    ]) {
      expect(parsePosition(bad)).toBeNull();
    }
  });
});
