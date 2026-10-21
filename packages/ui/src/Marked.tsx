import type { TextRange } from "@quran-reader/types";
import { segments } from "./marks.ts";

export interface MarkedProps {
  /** Unmodified text. */
  text: string;
  /** Search matches to highlight (UTF-16 offsets into `text`). */
  ranges?: readonly TextRange[];
}

/** `text` with `ranges` wrapped in `<mark>`. The text itself is never changed. */
export function Marked({ text, ranges }: MarkedProps) {
  if (!ranges || ranges.length === 0) return text;
  return segments(text, ranges).map((s, i) =>
    s.marked ? (
      // biome-ignore lint/suspicious/noArrayIndexKey: runs are positional and never reordered
      <mark key={i} className="search-mark">
        {s.text}
      </mark>
    ) : (
      s.text
    ),
  );
}
