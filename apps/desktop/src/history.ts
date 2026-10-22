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

// The last reading position lives in the study store (DECISIONS.md D-021). Versions before
// it kept it in localStorage; that value is moved into the store once.
const POSITION_KEY = "quran-reader:position";

/** A position from untrusted JSON. */
export function parsePosition(raw: unknown): VerseRef | null {
  if (typeof raw !== "object" || raw === null) return null;
  const { surah, ayah } = raw as Record<string, unknown>;
  const ok = (n: unknown, max: number) =>
    Number.isInteger(n) && (n as number) >= 1 && (n as number) <= max;
  return ok(surah, 114) && ok(ayah, 286) ? { surah: surah as number, ayah: ayah as number } : null;
}

type Legacy = Pick<Storage, "getItem" | "removeItem">;

function loadLegacy(storage: Legacy | undefined): VerseRef | null {
  try {
    const raw = storage?.getItem(POSITION_KEY);
    return parsePosition(raw ? JSON.parse(raw) : null);
  } catch {
    return null;
  }
}

/**
 * The position to start at: the stored one, else the one an older version left in
 * localStorage, which is then saved to the store and removed (kept if saving fails, so the
 * next start tries again).
 */
export async function migratePosition(
  stored: VerseRef | null,
  storage: Legacy | undefined,
  save: (ref: VerseRef) => Promise<void>,
): Promise<VerseRef | null> {
  const legacy = loadLegacy(storage);
  const drop = () => {
    try {
      storage?.removeItem(POSITION_KEY);
    } catch {
      // Left behind; ignored from now on.
    }
  };
  if (stored) {
    if (legacy) drop();
    return stored;
  }
  if (!legacy) return null;
  try {
    await save(legacy);
    drop();
  } catch {
    // Keep it for the next start.
  }
  return legacy;
}
