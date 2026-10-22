import type {
  Bookmark,
  Highlight,
  HighlightColor,
  Note,
  Portion,
  StudyData,
  VerseRef,
} from "@quran-reader/types";
import { useCallback, useMemo, useState } from "react";
import * as core from "./core.ts";
import { indexOf, type Mushaf, refAt } from "./mushaf.ts";

// Highlights, notes and bookmarks in the reader. The store in quran-core is the source of
// truth; this keeps a copy indexed by global verse index for the rows, and replaces entries
// with what the store returns after each change.

export const COLOR_NAMES = ["yellow", "green", "blue", "pink", "purple", "orange"] as const;

export const EMPTY_STUDY: StudyData = { highlights: [], notes: [], bookmarks: [], plan: null };

/** Study data by global verse index, for the reader rows. */
export interface StudyIndex {
  highlights: Map<number, HighlightColor>;
  notes: Map<number, string>;
  /** Label, or "" for a bookmark without one. */
  bookmarks: Map<number, string>;
}

export function indexStudy(m: Mushaf, data: StudyData): StudyIndex {
  return {
    highlights: new Map(data.highlights.map((h) => [indexOf(m, h.ref), h.color])),
    notes: new Map(data.notes.map((n) => [indexOf(m, n.ref), n.body])),
    bookmarks: new Map(data.bookmarks.map((b) => [indexOf(m, b.ref), b.label ?? ""])),
  };
}

const same = (a: VerseRef, b: VerseRef) => a.surah === b.surah && a.ayah === b.ayah;
const byRef = (a: { ref: VerseRef }, b: { ref: VerseRef }) =>
  a.ref.surah - b.ref.surah || a.ref.ayah - b.ref.ayah;

/** Puts `entry` in place of the list's entry for `ref` (or removes it), keeping the order. */
export function upsert<T extends { ref: VerseRef }>(list: T[], ref: VerseRef, entry: T | null) {
  const rest = list.filter((x) => !same(x.ref, ref));
  if (entry) rest.push(entry);
  return rest.sort(byRef);
}

/** Keys 1–6: the same colour again removes the highlight, another one replaces it. */
export function nextColor(
  current: HighlightColor | undefined,
  pressed: HighlightColor,
): HighlightColor | null {
  return current === pressed ? null : pressed;
}

export interface Study {
  data: StudyData;
  index: StudyIndex;
  error: string | null;
  setHighlight: (ref: VerseRef, color: HighlightColor | null) => Promise<void>;
  setNote: (ref: VerseRef, body: string) => Promise<void>;
  setBookmark: (ref: VerseRef, bookmarked: boolean, label?: string | null) => Promise<void>;
  /** After an import. */
  replace: (data: StudyData) => void;
}

export function useStudy(mushaf: Mushaf, initial: StudyData, initialError: string | null): Study {
  const [data, setData] = useState(initial);
  const [error, setError] = useState(initialError);
  const index = useMemo(() => indexStudy(mushaf, data), [mushaf, data]);

  const run = useCallback(async (change: () => Promise<(d: StudyData) => StudyData>) => {
    try {
      const apply = await change();
      setData(apply);
      setError(null);
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }, []);

  const setHighlight = useCallback(
    (ref: VerseRef, color: HighlightColor | null) =>
      run(async () => {
        const h = await core.setHighlight(ref, color);
        return (d) => ({ ...d, highlights: upsert<Highlight>(d.highlights, ref, h) });
      }),
    [run],
  );
  const setNote = useCallback(
    (ref: VerseRef, body: string) =>
      run(async () => {
        const n = await core.setNote(ref, body);
        return (d) => ({ ...d, notes: upsert<Note>(d.notes, ref, n) });
      }),
    [run],
  );
  const setBookmark = useCallback(
    (ref: VerseRef, bookmarked: boolean, label: string | null = null) =>
      run(async () => {
        const b = await core.setBookmark(ref, bookmarked, label);
        return (d) => ({ ...d, bookmarks: upsert<Bookmark>(d.bookmarks, ref, b) });
      }),
    [run],
  );
  const replace = useCallback((next: StudyData) => setData(next), []);

  return { data, index, error, setHighlight, setNote, setBookmark, replace };
}

/** `Al-Baqara 2:255`. */
export function refLabel(m: Mushaf, ref: VerseRef): string {
  const name = m.surahs[ref.surah - 1]?.nameLatin ?? `Surah ${ref.surah}`;
  return `${name} ${ref.surah}:${ref.ayah}`;
}

/**
 * What "copy verse" puts on the clipboard: the unmodified Arabic, the translation if one
 * is shown, the reference, and the attributions.
 */
export function copyText(
  m: Mushaf,
  index: number,
  translation?: { name: string; verse: string | undefined } | null,
): string {
  const ref = refAt(m, index);
  const lines = [m.verses[index] ?? ""];
  if (translation?.verse) lines.push("", translation.verse);
  lines.push("", `— ${refLabel(m, ref)}`);
  const credits = ["Quran text: Tanzil.net (CC BY 3.0)"];
  if (translation?.verse) credits.push(`Translation: ${translation.name}`);
  lines.push(credits.join(" · "));
  return lines.join("\n");
}

/** Puts text on the clipboard, falling back to a hidden textarea where the API is missing. */
export async function writeClipboard(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    return;
  } catch {
    // Not allowed here (or no API): fall through.
  }
  const area = document.createElement("textarea");
  area.value = text;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.opacity = "0";
  document.body.append(area);
  area.select();
  const ok = document.execCommand("copy");
  area.remove();
  if (!ok) throw new Error("the clipboard is not available");
}

/** Local calendar date as `YYYY-MM-DD` (plans count days in the user's time zone). */
export function localDate(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Local date and time for the Markdown export header, e.g. `2026-10-22 14:05`. */
export function localDateTime(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${localDate(d)} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** `Juz 5` for 30-day plans, `Hizb 9` for 60-day plans, otherwise the verse range. */
export function portionLabel(m: Mushaf, days: number, p: Portion): string {
  if (days === 30) return `Juz ${p.day}`;
  if (days === 60) return `Hizb ${p.day}`;
  return portionRange(m, p);
}

/** `Al-Baqara 2:142 – 2:252`, or with both names when the portion crosses surahs. */
export function portionRange(m: Mushaf, p: Portion): string {
  const to = p.to.surah === p.from.surah ? `${p.to.surah}:${p.to.ayah}` : refLabel(m, p.to);
  return `${refLabel(m, p.from)} – ${to}`;
}
