import type { CoreInfo, Theme } from "@quran-reader/types";
import { SurahHeader } from "@quran-reader/ui";
import { useEffect, useState } from "react";
import { coreInfo } from "./core.ts";

const THEMES: Theme[] = ["light", "sepia", "dark"];

export function App() {
  const [theme, setTheme] = useState<Theme | null>(null);
  const [info, setInfo] = useState<CoreInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (theme) document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    coreInfo()
      .then(setInfo)
      .catch((e: unknown) => setError(String(e)));
  }, []);

  return (
    <div className="flex h-full flex-col">
      <nav className="flex items-center justify-between border-b border-rule px-5 py-3 text-sm">
        <span className="font-medium">Quran Reader</span>
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

      <main
        className="mx-auto w-full flex-1 overflow-y-auto px-6 py-10"
        style={{ maxWidth: "var(--reader-measure)" }}
      >
        <SurahHeader
          surah={{
            number: 1,
            nameArabic: "الفاتحة",
            nameLatin: "Al-Fatihah",
            nameTranslated: "The Opening",
            revelation: "meccan",
            ayahCount: 7,
          }}
        />
        <p className="mt-8 text-center text-muted text-sm">
          The text arrives with the data build in Phase 1.
        </p>
      </main>

      <footer className="border-t border-rule px-5 py-2 text-muted text-xs tabular-nums">
        {error
          ? `core error: ${error}`
          : info
            ? `quran-core ${info.version} · ${info.surahCount} surahs · ${info.ayahCount} verses · data ${info.dataLoaded ? "loaded" : "not built"}`
            : "browser preview — quran-core not attached"}
      </footer>
    </div>
  );
}
