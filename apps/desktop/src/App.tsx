import type { CoreInfo, Surah, SurahText, Theme } from "@quran-reader/types";
import { Ayah, Basmala, SurahHeader } from "@quran-reader/ui";
import { useEffect, useRef, useState } from "react";
import * as core from "./core.ts";

const THEMES: Theme[] = ["light", "sepia", "dark"];
const SURAH_COUNT = 114;

export function App() {
  const [theme, setTheme] = useState<Theme | null>(null);
  const [info, setInfo] = useState<CoreInfo | null>(null);
  const [surahs, setSurahs] = useState<Surah[] | null>(null);
  const [current, setCurrent] = useState(1);
  const [text, setText] = useState<SurahText | null>(null);
  const [error, setError] = useState<string | null>(null);
  const main = useRef<HTMLElement>(null);

  useEffect(() => {
    if (theme) document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    Promise.all([core.coreInfo(), core.surahs()])
      .then(([i, s]) => {
        setInfo(i);
        setSurahs(s);
      })
      .catch((e: unknown) => setError(String(e)));
  }, []);

  useEffect(() => {
    let stale = false;
    core
      .surah(current)
      .then((t) => {
        if (stale) return;
        setText(t);
        main.current?.scrollTo(0, 0);
      })
      .catch((e: unknown) => setError(String(e)));
    return () => {
      stale = true;
    };
  }, [current]);

  const go = (n: number) => setCurrent(Math.min(SURAH_COUNT, Math.max(1, n)));

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "ArrowLeft") setCurrent((n) => Math.max(1, n - 1));
      if (e.key === "ArrowRight") setCurrent((n) => Math.min(SURAH_COUNT, n + 1));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="flex h-full flex-col">
      <nav className="flex items-center justify-between gap-4 border-b border-rule px-5 py-3 text-sm">
        <span className="font-medium">Quran Reader</span>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => go(current - 1)}
            disabled={current <= 1}
            aria-label="Previous surah"
            className="rounded px-2 py-1 text-muted hover:text-ink disabled:opacity-40"
          >
            ←
          </button>
          <select
            aria-label="Surah"
            value={current}
            onChange={(e) => go(Number(e.target.value))}
            className="rounded border border-rule bg-paper px-2 py-1 tabular-nums"
          >
            {(surahs ?? []).map((s) => (
              <option key={s.number} value={s.number}>
                {s.number}. {s.nameLatin}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={() => go(current + 1)}
            disabled={current >= SURAH_COUNT}
            aria-label="Next surah"
            className="rounded px-2 py-1 text-muted hover:text-ink disabled:opacity-40"
          >
            →
          </button>
        </div>
        <fieldset className="flex gap-1">
          <legend className="sr-only">Theme</legend>
          {THEMES.map((t) => (
            <button
              key={t}
              type="button"
              aria-pressed={theme === t}
              onClick={() => setTheme(t)}
              className="rounded px-2 py-1 capitalize text-muted hover:text-ink aria-pressed:bg-rule aria-pressed:text-ink"
            >
              {t}
            </button>
          ))}
        </fieldset>
      </nav>

      <main ref={main} className="flex-1 overflow-y-auto">
        <div className="mx-auto w-full px-6 py-10" style={{ maxWidth: "var(--reader-measure)" }}>
          {text ? (
            <>
              <SurahHeader surah={text.surah} />
              {text.bismillah !== undefined && <Basmala text={text.bismillah} />}
              {text.ayahs.map((a) => (
                <Ayah key={a.ref.ayah} verseRef={a.ref} text={a.text} />
              ))}
            </>
          ) : (
            <p className="mt-8 text-center text-muted text-sm">
              {error ?? (info ? "Loading…" : "Browser preview — the text comes from quran-core.")}
            </p>
          )}
        </div>
      </main>

      <footer className="flex justify-between gap-4 border-t border-rule px-5 py-2 text-muted text-xs tabular-nums">
        <span>
          {error
            ? `core error: ${error}`
            : info
              ? `quran-core ${info.version} · ${info.surahCount} surahs · ${info.ayahCount} verses · data ${info.dataLoaded ? "loaded" : "not built"}`
              : "browser preview — quran-core not attached"}
        </span>
        <span>Quran text: Tanzil.net (CC BY 3.0) · tanzil.net</span>
      </footer>
    </div>
  );
}
