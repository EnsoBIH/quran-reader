import type { SearchHit, TextRange } from "@quran-reader/types";

// Helpers for the ⌘F panel. Matching happens in quran-core (`search`); these only pick what
// to show of a verse. They cut at spaces and never change the text.

export const QURAN_TEXT = "quran";

/** Matched ranges of the Arabic text of a hit. */
export function arabicRanges(hit: SearchHit): TextRange[] {
  return hit.matches.find((m) => m.text === QURAN_TEXT)?.ranges ?? [];
}

export interface Snippet {
  text: string;
  ranges: TextRange[];
  /** Whether text was left out before or after. */
  cutStart: boolean;
  cutEnd: boolean;
}

/**
 * The part of `text` around its first match: whole words, about `before` UTF-16 units
 * before the match and `after` after it. Short texts are returned whole.
 */
export function snippet(text: string, ranges: TextRange[], before = 60, after = 140): Snippet {
  const first = ranges[0];
  if (!first || text.length <= before + after) {
    return { text, ranges, cutStart: false, cutEnd: false };
  }
  let start = Math.max(0, first.start - before);
  let end = Math.min(text.length, first.end + after);
  if (start > 0) {
    // Back to the start of the word, so no word is cut in half.
    const space = text.lastIndexOf(" ", start);
    start = space < 0 ? 0 : space + 1;
  }
  if (end < text.length) {
    const space = text.indexOf(" ", end);
    end = space < 0 ? text.length : space;
  }
  const shifted = ranges
    .filter((r) => r.end > start && r.start < end)
    .map((r) => ({ start: Math.max(r.start, start) - start, end: Math.min(r.end, end) - start }));
  return {
    text: text.slice(start, end),
    ranges: shifted,
    cutStart: start > 0,
    cutEnd: end < text.length,
  };
}
