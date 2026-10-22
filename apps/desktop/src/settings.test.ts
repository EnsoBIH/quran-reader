import type { TranslationInfo } from "@quran-reader/types";
import { describe, expect, it } from "vitest";
import {
  applySettings,
  clampSize,
  DEFAULT_SETTINGS,
  LIMITS,
  loadSettings,
  parseSettings,
  pickTranslation,
  saveSettings,
  toggleTranslation,
  withView,
} from "./settings.ts";

function translation(id: string, language: string, bundled = true): TranslationInfo {
  return { id, language, name: id, translator: "", source: "", license: "", notice: "", bundled };
}

function memoryStorage() {
  const data = new Map<string, string>();
  return {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
  };
}

describe("settings", () => {
  it("falls back to defaults for missing or broken input", () => {
    expect(parseSettings(null)).toEqual(DEFAULT_SETTINGS);
    expect(parseSettings("dark")).toEqual(DEFAULT_SETTINGS);
    expect(
      parseSettings({ theme: "neon", view: 3, arabicSize: "big", measure: Number.NaN }),
    ).toEqual(DEFAULT_SETTINGS);
  });

  it("keeps valid fields and fixes the others one by one", () => {
    const s = parseSettings({
      theme: "sepia",
      view: "columns",
      arabicSize: 99,
      translationSize: 1,
    });
    expect(s.theme).toBe("sepia");
    expect(s.view).toBe("columns");
    expect(s.arabicSize).toBe(LIMITS.arabicSize.max);
    expect(s.translationSize).toBe(1);
    expect(s.measure).toBe(DEFAULT_SETTINGS.measure);
  });

  it("clamps and snaps sizes to their step", () => {
    expect(clampSize("measure", 10)).toBe(30);
    expect(clampSize("measure", 41)).toBe(42);
    expect(clampSize("arabicSize", 2.06)).toBe(2.0);
    expect(clampSize("translationSize", 0.1)).toBe(0.875);
  });

  it("round-trips through storage", () => {
    const storage = memoryStorage();
    expect(loadSettings(storage)).toEqual(DEFAULT_SETTINGS);
    const s = { ...DEFAULT_SETTINGS, theme: "dark" as const, arabicSize: 2.5, measure: 50 };
    saveSettings(storage, s);
    expect(loadSettings(storage)).toEqual(s);
  });

  it("survives unreadable storage", () => {
    expect(loadSettings({ getItem: () => "{not json" })).toEqual(DEFAULT_SETTINGS);
    expect(
      loadSettings({
        getItem: () => {
          throw new Error("denied");
        },
      }),
    ).toEqual(DEFAULT_SETTINGS);
    expect(() =>
      saveSettings(
        {
          setItem: () => {
            throw new Error("full");
          },
        },
        DEFAULT_SETTINGS,
      ),
    ).not.toThrow();
  });

  it("applies theme and sizes to the root element", () => {
    const vars = new Map<string, string>();
    const root = {
      dataset: {} as Record<string, string>,
      style: { setProperty: (k: string, v: string) => void vars.set(k, v) },
    };
    applySettings({ ...DEFAULT_SETTINGS, theme: "sepia", measure: 36 }, root as never);
    expect(root.dataset.theme).toBe("sepia");
    expect(vars.get("--reader-measure")).toBe("36rem");
    applySettings(DEFAULT_SETTINGS, root as never);
    expect(root.dataset.theme).toBeUndefined();
  });

  it("keeps the chosen translation id", () => {
    expect(parseSettings({ translation: "en.pickthall" }).translation).toBe("en.pickthall");
    expect(parseSettings({ translation: "" }).translation).toBeNull();
    expect(parseSettings({ translation: 3 }).translation).toBeNull();
  });

  it("checks for updates unless turned off", () => {
    expect(DEFAULT_SETTINGS.checkUpdates).toBe(true);
    expect(parseSettings({ checkUpdates: false }).checkUpdates).toBe(false);
    expect(parseSettings({ checkUpdates: "no" }).checkUpdates).toBe(true);
  });

  it("picks the chosen translation, else one in the user's language, else the first", () => {
    const installed = [translation("en.a", "en"), translation("user.de.b", "de", false)];
    expect(pickTranslation(installed, "user.de.b")?.id).toBe("user.de.b");
    expect(pickTranslation(installed, "gone", ["de-AT", "en"])?.id).toBe("user.de.b");
    expect(pickTranslation(installed, null, ["fr"])?.id).toBe("en.a");
    expect(pickTranslation([], "en.a")).toBeNull();
  });

  it("shows the translation by default", () => {
    expect(DEFAULT_SETTINGS.view).toBe("stacked");
    expect(parseSettings({ view: "arabic" }).translatedView).toBe("stacked");
    expect(parseSettings({ translatedView: "arabic" }).translatedView).toBe("stacked");
  });

  it("toggles the translation back to the view it was last shown in", () => {
    const columns = withView(DEFAULT_SETTINGS, "columns");
    expect(columns.translatedView).toBe("columns");
    const hidden = toggleTranslation(columns);
    expect(hidden.view).toBe("arabic");
    expect(hidden.translatedView).toBe("columns");
    expect(toggleTranslation(hidden).view).toBe("columns");
    expect(withView(columns, "arabic").translatedView).toBe("columns");
  });
});
