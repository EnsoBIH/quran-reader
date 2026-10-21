import type { ReaderView, Theme, TranslationInfo } from "@quran-reader/types";

// Reader settings, kept in localStorage until the SQLite store arrives (Phase 5).

export interface Settings {
  /** `system` follows the OS light/dark setting. */
  theme: Theme | "system";
  view: ReaderView;
  /** Font sizes and line measure in rem. */
  arabicSize: number;
  translationSize: number;
  measure: number;
  /** Id of the translation shown beside the Arabic; `null` until the user picks one. */
  translation: string | null;
}

export const THEMES: Settings["theme"][] = ["system", "light", "sepia", "dark"];
export const VIEWS: ReaderView[] = ["arabic", "stacked", "columns"];

export const LIMITS = {
  arabicSize: { min: 1.25, max: 3.5, step: 0.125 },
  translationSize: { min: 0.875, max: 1.5, step: 0.0625 },
  measure: { min: 30, max: 64, step: 2 },
} as const;

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  view: "arabic",
  arabicSize: 1.875,
  translationSize: 1.0625,
  measure: 42,
  translation: null,
};

const KEY = "quran-reader:settings";

/** Reads settings from untrusted JSON, falling back to the default for each bad field. */
export function parseSettings(raw: unknown): Settings {
  const o = typeof raw === "object" && raw !== null ? (raw as Record<string, unknown>) : {};
  const pick = <T>(value: unknown, allowed: readonly T[], fallback: T): T =>
    allowed.includes(value as T) ? (value as T) : fallback;
  const size = (name: keyof typeof LIMITS) => {
    const value = o[name];
    return typeof value === "number" && Number.isFinite(value)
      ? clampSize(name, value)
      : DEFAULT_SETTINGS[name];
  };
  return {
    theme: pick(o.theme, THEMES, DEFAULT_SETTINGS.theme),
    view: pick(o.view, VIEWS, DEFAULT_SETTINGS.view),
    arabicSize: size("arabicSize"),
    translationSize: size("translationSize"),
    measure: size("measure"),
    translation: typeof o.translation === "string" && o.translation !== "" ? o.translation : null,
  };
}

/** Clamps to the allowed range and rounds to its step. */
export function clampSize(name: keyof typeof LIMITS, value: number): number {
  const { min, max, step } = LIMITS[name];
  return Math.min(max, Math.max(min, Math.round(value / step) * step));
}

export function loadSettings(storage: Pick<Storage, "getItem"> | undefined): Settings {
  try {
    const raw = storage?.getItem(KEY);
    return parseSettings(raw ? JSON.parse(raw) : null);
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(storage: Pick<Storage, "setItem"> | undefined, s: Settings) {
  try {
    storage?.setItem(KEY, JSON.stringify(s));
  } catch {
    // Storage full or unavailable: settings just don't persist.
  }
}

/** Applies theme and sizes to the document (see `--reader-*` in packages/ui theme.css). */
export function applySettings(s: Settings, root: HTMLElement) {
  if (s.theme === "system") delete root.dataset.theme;
  else root.dataset.theme = s.theme;
  root.style.setProperty("--reader-arabic-size", `${s.arabicSize}rem`);
  root.style.setProperty("--reader-translation-size", `${s.translationSize}rem`);
  root.style.setProperty("--reader-measure", `${s.measure}rem`);
}

/**
 * The translation to show: the chosen one if it is installed, otherwise the first in the
 * user's language (`languages`, e.g. `navigator.languages`), otherwise the first installed.
 */
export function pickTranslation(
  installed: readonly TranslationInfo[],
  chosen: string | null,
  languages: readonly string[] = [],
): TranslationInfo | null {
  const found = installed.find((t) => t.id === chosen);
  if (found) return found;
  for (const tag of languages) {
    const lang = tag.toLowerCase().split("-")[0];
    const match = installed.find((t) => t.language === lang);
    if (match) return match;
  }
  return installed[0] ?? null;
}
