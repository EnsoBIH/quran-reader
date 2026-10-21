import type { TextRange } from "@quran-reader/types";

export interface Segment {
  text: string;
  marked: boolean;
}

/**
 * Cuts `text` into plain and marked runs at `ranges` (UTF-16 offsets, sorted and
 * non-overlapping, as search returns them). Joined back, the runs are `text` unchanged.
 * Ranges outside the text are clipped; overlapping ones are merged.
 */
export function segments(text: string, ranges: readonly TextRange[] = []): Segment[] {
  const out: Segment[] = [];
  let at = 0;
  for (const r of ranges) {
    const start = Math.max(at, Math.min(r.start, text.length));
    const end = Math.max(start, Math.min(r.end, text.length));
    if (end === start) continue;
    if (start > at) out.push({ text: text.slice(at, start), marked: false });
    out.push({ text: text.slice(start, end), marked: true });
    at = end;
  }
  if (at < text.length || out.length === 0) out.push({ text: text.slice(at), marked: false });
  return out;
}
