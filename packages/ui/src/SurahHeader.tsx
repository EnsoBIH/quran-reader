import type { Surah } from "@quran-reader/types";
import { toArabicIndic } from "./digits.ts";

const REVELATION_LABEL = { meccan: "Meccan", medinan: "Medinan" } as const;

export interface SurahHeaderProps {
  surah: Surah;
}

export function SurahHeader({ surah }: SurahHeaderProps) {
  return (
    <header className="flex flex-col items-center gap-2 border-y border-rule py-6 text-center">
      <p className="text-muted text-xs uppercase tracking-[0.2em]">
        Surah {surah.number} · {toArabicIndic(surah.number)}
      </p>
      <h2 dir="rtl" lang="ar" className="font-quran text-4xl text-ink leading-normal">
        {surah.nameArabic}
      </h2>
      <p className="font-sans text-ink">
        {surah.nameLatin}
        {surah.nameTranslated !== undefined && (
          <span className="text-muted"> — {surah.nameTranslated}</span>
        )}
      </p>
      <p className="font-sans text-muted text-sm">
        {REVELATION_LABEL[surah.revelation]} · {surah.ayahCount} verses
      </p>
    </header>
  );
}
