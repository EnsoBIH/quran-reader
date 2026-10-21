import type { Hit, Part } from "@quran-reader/types";
import { useEffect, useRef, useState } from "react";
import * as core from "./core.ts";
import type { Mushaf } from "./mushaf.ts";

// ⌘K: go to a verse, surah, named verse, juz, hizb or page. Matching happens in
// quran-core (`lookup`); this only lists the hits.

interface PaletteProps {
  mushaf: Mushaf;
  onGo: (hit: Hit) => void;
  onClose: () => void;
}

const EXAMPLES = ["2:255", "baqara 255", "بقرة", "Ayat al-Kursi", "juz 30", "hizb 10", "page 604"];
const PART_LABEL: Record<Part, string> = { juz: "Juz", hizb: "Hizb", page: "Page" };

export function Palette({ mushaf, onGo, onClose }: PaletteProps) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<Hit[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => input.current?.focus(), []);

  useEffect(() => {
    let stale = false;
    if (query.trim() === "") {
      setHits([]);
      setError(null);
      return;
    }
    core
      .lookup(query, 8)
      .then((h) => {
        if (stale) return;
        setHits(h);
        setError(null);
        setSelected(0);
      })
      .catch((e: unknown) => {
        if (stale) return;
        setHits([]);
        setError(String(e));
      });
    return () => {
      stale = true;
    };
  }, [query]);

  const go = (hit: Hit | undefined) => {
    if (hit) onGo(hit);
  };

  return (
    <div className="fixed inset-0 z-20 flex justify-center px-4 pt-[12vh]">
      <button
        type="button"
        tabIndex={-1}
        aria-label="Close"
        onMouseDown={onClose}
        className="absolute inset-0 cursor-default bg-black/25"
      />
      <div
        role="dialog"
        aria-label="Go to"
        className="relative h-fit w-full max-w-xl overflow-hidden rounded-lg border border-rule bg-paper shadow-xl"
      >
        <input
          ref={input}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSelected((i) => Math.min(i + 1, hits.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSelected((i) => Math.max(i - 1, 0));
            } else if (e.key === "Enter") {
              e.preventDefault();
              go(hits[selected]);
            } else if (e.key === "Escape") {
              e.preventDefault();
              onClose();
            }
          }}
          placeholder="Go to verse, surah, juz, hizb or page…"
          aria-label="Reference"
          spellCheck={false}
          autoComplete="off"
          className="w-full border-b border-rule bg-transparent px-4 py-3 text-ink outline-none placeholder:text-muted"
        />
        {error !== null ? (
          <p className="px-4 py-3 text-muted text-sm">{error}</p>
        ) : hits.length === 0 ? (
          <p className="px-4 py-3 text-muted text-sm">
            Try{" "}
            {EXAMPLES.map((ex, i) => (
              <span key={ex}>
                {i > 0 && " · "}
                <button
                  type="button"
                  className="text-ink hover:text-accent"
                  onClick={() => setQuery(ex)}
                >
                  {ex}
                </button>
              </span>
            ))}
          </p>
        ) : (
          <ul className="max-h-80 overflow-y-auto py-1">
            {hits.map((hit, i) => {
              const label = hitLabel(mushaf, hit);
              return (
                <li key={`${hit.target.kind}-${hit.ref.surah}-${hit.ref.ayah}-${i}`}>
                  <button
                    type="button"
                    aria-current={i === selected}
                    onMouseEnter={() => setSelected(i)}
                    onClick={() => go(hit)}
                    className="flex w-full items-baseline gap-3 px-4 py-2 text-left aria-current:bg-rule"
                  >
                    <span className="text-ink">{label.title}</span>
                    <span className="truncate text-muted text-sm">{label.detail}</span>
                    {label.arabic && (
                      <span dir="rtl" lang="ar" className="ml-auto font-quran text-lg text-ink">
                        {label.arabic}
                      </span>
                    )}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}

export function hitLabel(m: Mushaf, hit: Hit): { title: string; detail: string; arabic?: string } {
  const s = m.surahs[hit.ref.surah - 1];
  const name = s?.nameLatin ?? `Surah ${hit.ref.surah}`;
  const ref = `${hit.ref.surah}:${hit.ref.ayah}`;
  switch (hit.target.kind) {
    case "verse":
      return { title: `${name} ${ref}`, detail: s?.nameTranslated ?? "", arabic: s?.nameArabic };
    case "surah":
      return {
        title: `${hit.ref.surah}. ${name}`,
        detail: [s?.nameTranslated, s && `${s.ayahCount} verses`].filter(Boolean).join(" · "),
        arabic: s?.nameArabic,
      };
    case "named":
      return { title: hit.target.name, detail: `${name} ${ref}` };
    case "part":
      return {
        title: `${PART_LABEL[hit.target.part]} ${hit.target.number}`,
        detail: `starts at ${name} ${ref}`,
      };
  }
}
