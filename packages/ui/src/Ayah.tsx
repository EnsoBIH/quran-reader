import type { VerseRef } from "@quran-reader/types";
import { END_OF_AYAH, toArabicIndic } from "./digits.ts";

export interface AyahProps {
  verseRef: VerseRef;
  /** Unmodified Tanzil text. */
  text: string;
  translation?: string;
  translationLang?: string;
  layout?: "stacked" | "columns";
}

export function Ayah({
  verseRef,
  text,
  translation,
  translationLang = "en",
  layout = "stacked",
}: AyahProps) {
  const id = `${verseRef.surah}:${verseRef.ayah}`;
  return (
    <article
      id={`v${verseRef.surah}-${verseRef.ayah}`}
      data-ref={id}
      className={
        layout === "columns"
          ? "grid grid-cols-2 gap-8 border-b border-rule py-5"
          : "flex flex-col gap-3 border-b border-rule py-5"
      }
    >
      <p
        dir="rtl"
        lang="ar"
        className="font-quran text-ink leading-[2.2]"
        style={{ fontSize: "var(--reader-arabic-size)" }}
      >
        {text}{" "}
        <span aria-hidden="true">
          {END_OF_AYAH}
          {toArabicIndic(verseRef.ayah)}
        </span>
        <span className="sr-only">({id})</span>
      </p>
      {translation !== undefined && (
        <p
          lang={translationLang}
          className="font-sans text-ink/85 leading-relaxed"
          style={{ fontSize: "var(--reader-translation-size)" }}
        >
          <span className="mr-2 text-muted tabular-nums text-sm">{id}</span>
          {translation}
        </p>
      )}
    </article>
  );
}
