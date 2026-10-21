import type { VerseRef } from "@quran-reader/types";

// Back/forward through jumps (palette, surah arrows, juz and page), like a browser. Each
// entry is updated to where the reader had scrolled to before leaving it.

export interface History {
  entries: VerseRef[];
  /** The entry currently being read. −1 when empty. */
  index: number;
}

export const EMPTY_HISTORY: History = { entries: [], index: -1 };

const LIMIT = 100;

const same = (a: VerseRef | undefined, b: VerseRef) =>
  a !== undefined && a.surah === b.surah && a.ayah === b.ayah;

/** Records a jump from `from` (where the reader is now) to `to`. Drops forward entries. */
export function visit(h: History, from: VerseRef, to: VerseRef): History {
  const entries = h.entries.slice(0, h.index + 1);
  if (entries.length === 0) entries.push(from);
  else entries[entries.length - 1] = from;
  if (!same(entries[entries.length - 1], to)) entries.push(to);
  const kept = entries.slice(-LIMIT);
  return { entries: kept, index: kept.length - 1 };
}

/** Steps back (`-1`) or forward (`1`) from `current`; `null` at either end. */
export function step(
  h: History,
  current: VerseRef,
  direction: -1 | 1,
): { history: History; to: VerseRef } | null {
  const index = h.index + direction;
  const to = h.entries[index];
  if (h.index < 0 || to === undefined) return null;
  const entries = h.entries.slice();
  entries[h.index] = current;
  return { history: { entries, index }, to };
}

export const canStep = (h: History, direction: -1 | 1) =>
  h.index >= 0 && h.entries[h.index + direction] !== undefined;

const POSITION_KEY = "quran-reader:position";

/** Last reading position, from untrusted JSON. */
export function parsePosition(raw: unknown): VerseRef | null {
  if (typeof raw !== "object" || raw === null) return null;
  const { surah, ayah } = raw as Record<string, unknown>;
  const ok = (n: unknown, max: number) =>
    Number.isInteger(n) && (n as number) >= 1 && (n as number) <= max;
  return ok(surah, 114) && ok(ayah, 286) ? { surah: surah as number, ayah: ayah as number } : null;
}

export function loadPosition(storage: Pick<Storage, "getItem"> | undefined): VerseRef | null {
  try {
    const raw = storage?.getItem(POSITION_KEY);
    return parsePosition(raw ? JSON.parse(raw) : null);
  } catch {
    return null;
  }
}

export function savePosition(storage: Pick<Storage, "setItem"> | undefined, ref: VerseRef) {
  try {
    storage?.setItem(POSITION_KEY, JSON.stringify(ref));
  } catch {
    // Not persisted; the reader starts at the beginning next time.
  }
}
