import type { TranslationInfo, VerseRef } from "@quran-reader/types";
import { Ayah } from "@quran-reader/ui";
import { useEffect, useRef, useState } from "react";
import { indexOf, type Mushaf, refAt } from "./mushaf.ts";
import { languageName } from "./translations.ts";

// C: one verse in the Arabic and every installed translation, one under the other.
// ↑ ↓ (or ← →) move to the previous or next verse, Enter reads on from there.

interface CompareProps {
  mushaf: Mushaf;
  /** The verse shown first, usually the one being read. */
  start: VerseRef;
  translations: TranslationInfo[];
  /** Loaded texts; the others show a placeholder until they arrive. */
  texts: ReadonlyMap<string, string[]>;
  onGo: (to: VerseRef) => void;
  onClose: () => void;
}

export function Compare({ mushaf, start, translations, texts, onGo, onClose }: CompareProps) {
  const [index, setIndex] = useState(() => indexOf(mushaf, start));
  const dialog = useRef<HTMLDivElement>(null);
  const ref = refAt(mushaf, index);
  const surah = mushaf.surahs[ref.surah - 1];
  const last = mushaf.verses.length - 1;

  useEffect(() => dialog.current?.focus(), []);

  return (
    <div className="fixed inset-0 z-20 flex justify-center px-4 py-[8vh]">
      <button
        type="button"
        tabIndex={-1}
        aria-label="Close"
        onMouseDown={onClose}
        className="absolute inset-0 cursor-default bg-black/25"
      />
      <div
        ref={dialog}
        role="dialog"
        aria-label={`Compare translations of ${ref.surah}:${ref.ayah}`}
        tabIndex={-1}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown" || e.key === "ArrowRight") {
            e.preventDefault();
            setIndex((i) => Math.min(i + 1, last));
          } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
            e.preventDefault();
            setIndex((i) => Math.max(i - 1, 0));
          } else if (e.key === "Enter") {
            e.preventDefault();
            onGo(ref);
          } else if (e.key === "Escape") {
            e.preventDefault();
            onClose();
          }
        }}
        className="relative flex max-h-full w-full max-w-3xl flex-col overflow-hidden rounded-lg border border-rule bg-paper shadow-xl outline-none"
      >
        <header className="flex items-center gap-2 border-b border-rule px-4 py-2 text-sm">
          <button
            type="button"
            className="rounded px-2 py-1 text-muted hover:text-ink disabled:opacity-40"
            onClick={() => setIndex((i) => Math.max(i - 1, 0))}
            disabled={index === 0}
            aria-label="Previous verse"
          >
            ↑
          </button>
          <button
            type="button"
            className="rounded px-2 py-1 text-muted hover:text-ink disabled:opacity-40"
            onClick={() => setIndex((i) => Math.min(i + 1, last))}
            disabled={index === last}
            aria-label="Next verse"
          >
            ↓
          </button>
          <span className="text-ink">
            {surah?.nameLatin ?? `Surah ${ref.surah}`} {ref.surah}:{ref.ayah}
          </span>
          <button
            type="button"
            className="ml-auto rounded px-2 py-1 text-muted hover:text-ink"
            onClick={() => onGo(ref)}
          >
            Read from here ↵
          </button>
        </header>
        <div className="overflow-y-auto px-6 pb-4">
          <Ayah verseRef={ref} text={mushaf.verses[index] ?? ""} />
          {translations.length === 0 ? (
            <p className="border-t border-rule py-4 text-muted text-sm">
              No translation is installed. Import one in the reader settings (⌘,).
            </p>
          ) : (
            translations.map((t) => {
              const verse = texts.get(t.id)?.[index];
              return (
                <section key={t.id} className="border-t border-rule py-4">
                  <h3 className="mb-1 text-muted text-xs">
                    {t.name}
                    {t.translator && t.translator !== t.name && ` · ${t.translator}`} ·{" "}
                    {languageName(t.language)}
                    {!t.bundled && " · imported"}
                  </h3>
                  <p
                    lang={t.language}
                    dir="auto"
                    className="text-ink/85 leading-relaxed"
                    style={{ fontSize: "var(--reader-translation-size)" }}
                  >
                    {verse ?? <span className="text-muted">Loading…</span>}
                  </p>
                </section>
              );
            })
          )}
        </div>
        <p className="border-t border-rule px-4 py-2 text-muted text-xs">
          ↑ ↓ previous / next verse · Enter read from here · Esc close
        </p>
      </div>
    </div>
  );
}
