import type { TranslationInfo } from "@quran-reader/types";
import { useCallback, useEffect, useRef, useState } from "react";
import * as core from "./core.ts";

// Installed translations and the verse texts loaded so far. A text (≈1 MB) is fetched from
// quran-core the first time something shows it: the reader, the compare view (C), search.

export interface Library {
  installed: TranslationInfo[];
  /** Loaded texts by id: 6,236 verses each. */
  texts: ReadonlyMap<string, string[]>;
  /** Loads the texts that are not loaded yet. */
  ensure: (ids: readonly string[]) => void;
  /** Imports a file; resolves to the new translation, rejects with a readable message. */
  importFile: (file: File) => Promise<TranslationInfo>;
  remove: (id: string) => Promise<void>;
  error: string | null;
}

export function useLibrary(
  initial: TranslationInfo[],
  initialTexts: ReadonlyMap<string, string[]>,
): Library {
  const [installed, setInstalled] = useState(initial);
  const [texts, setTexts] = useState(initialTexts);
  const [error, setError] = useState<string | null>(null);
  const requested = useRef(new Set(initialTexts.keys()));
  const live = useRef(true);
  useEffect(() => {
    live.current = true;
    return () => {
      live.current = false;
    };
  }, []);

  const ensure = useCallback((ids: readonly string[]) => {
    for (const id of ids) {
      if (requested.current.has(id)) continue;
      requested.current.add(id);
      core
        .translationText(id)
        .then((verses) => {
          if (live.current) setTexts((t) => new Map(t).set(id, verses));
        })
        .catch((e: unknown) => {
          requested.current.delete(id);
          if (live.current) setError(String(e));
        });
    }
  }, []);

  const importFile = useCallback(async (file: File) => {
    const info = await core.importTranslation(file.name, await file.text());
    // A re-import replaces the text, so drop the old one.
    requested.current.delete(info.id);
    setTexts((t) => {
      const next = new Map(t);
      next.delete(info.id);
      return next;
    });
    setInstalled(await core.translations());
    return info;
  }, []);

  const remove = useCallback(async (id: string) => {
    await core.removeTranslation(id);
    requested.current.delete(id);
    setTexts((t) => {
      const next = new Map(t);
      next.delete(id);
      return next;
    });
    setInstalled(await core.translations());
  }, []);

  return { installed, texts, ensure, importFile, remove, error };
}

const LANGUAGE_NAMES = (() => {
  try {
    return new Intl.DisplayNames(["en"], { type: "language" });
  } catch {
    return null;
  }
})();

/** English name of a language tag, e.g. `de` → "German"; `und` → "Other". */
export function languageName(tag: string): string {
  if (tag === "und") return "Other";
  try {
    return LANGUAGE_NAMES?.of(tag) ?? tag;
  } catch {
    return tag;
  }
}

/** Languages of the installed translations, sorted by name. */
export function languages(installed: readonly TranslationInfo[]): string[] {
  return [...new Set(installed.map((t) => t.language))].sort((a, b) =>
    languageName(a).localeCompare(languageName(b), "en"),
  );
}

/** Joins the hard-wrapped lines of a notice file; keeps blank lines and "- " list items. */
export function reflow(notice: string): string {
  return notice.replace(/(?<!\n)\n(?!\n|- ) */g, " ");
}
