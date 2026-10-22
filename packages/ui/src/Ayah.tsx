import type { HighlightColor, ReaderView, TextRange, VerseRef } from "@quran-reader/types";
import type { ReactNode } from "react";
import { END_OF_AYAH, toArabicIndic } from "./digits.ts";
import { Marked } from "./Marked.tsx";

export interface AyahProps {
  verseRef: VerseRef;
  /** Unmodified Tanzil text. */
  text: string;
  /** Arabic only (default), translation below, or translation beside the Arabic. */
  view?: ReaderView;
  /** Shown in the translation slot; the slot stays empty (reference only) without one. */
  translation?: string;
  translationLang?: string;
  /** A small line under the verse, e.g. page and sajda markers. */
  meta?: ReactNode;
  /** Search matches to highlight in `text` (UTF-16 offsets). */
  marks?: readonly TextRange[];
  /** Search matches to highlight in `translation`. */
  translationMarks?: readonly TextRange[];
  /** The verse's highlight colour (keys 1–6), shown as a background. */
  highlight?: HighlightColor;
  /** Shown under the verse, e.g. its note. */
  children?: ReactNode;
}

export function Ayah({
  verseRef,
  text,
  view = "arabic",
  translation,
  translationLang = "en",
  meta,
  marks,
  translationMarks,
  highlight,
  children,
}: AyahProps) {
  const id = `${verseRef.surah}:${verseRef.ayah}`;
  const arabic = (
    <p
      dir="rtl"
      lang="ar"
      className="col-start-2 row-start-1 font-quran text-ink leading-[2.2]"
      style={{ fontSize: "var(--reader-arabic-size)" }}
    >
      <Marked text={text} ranges={marks} />{" "}
      <span aria-hidden="true">
        {END_OF_AYAH}
        {toArabicIndic(verseRef.ayah)}
      </span>
      <span className="sr-only">({id})</span>
    </p>
  );
  const slot = view !== "arabic" && (
    <p
      lang={translationLang}
      dir="auto"
      className="col-start-1 row-start-1 font-sans text-ink/85 leading-relaxed"
      style={{ fontSize: "var(--reader-translation-size)" }}
    >
      <span className="mr-2 text-muted tabular-nums text-sm">{id}</span>
      {translation !== undefined && <Marked text={translation} ranges={translationMarks} />}
    </p>
  );
  return (
    <article
      id={`v${verseRef.surah}-${verseRef.ayah}`}
      data-ref={id}
      data-highlight={highlight}
      className={`${
        view === "columns" ? "grid grid-cols-2 gap-x-8 gap-y-2 py-5" : "flex flex-col gap-3 py-5"
      } ${highlight ? "verse-highlight -mx-3 my-1 rounded-md px-3" : ""}`}
    >
      {arabic}
      {slot}
      {meta !== undefined && <div className="col-span-2 text-muted text-xs">{meta}</div>}
      {children !== undefined && <div className="col-span-2">{children}</div>}
    </article>
  );
}
