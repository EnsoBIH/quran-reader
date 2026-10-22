import type { TranslationInfo } from "@quran-reader/types";
import { describe, expect, it } from "vitest";
import { languageName, languages, reflow } from "./translations.ts";

const t = (id: string, language: string): TranslationInfo => ({
  id,
  language,
  name: id,
  translator: "",
  source: "",
  license: "",
  notice: "",
  bundled: false,
});

describe("languages", () => {
  it("names languages in English, unknown ones as Other", () => {
    expect(languageName("de")).toBe("German");
    expect(languageName("en")).toBe("English");
    expect(languageName("und")).toBe("Other");
  });

  it("lists each language once, sorted by name", () => {
    expect(languages([t("a", "en"), t("b", "de"), t("c", "en"), t("d", "bs")])).toEqual([
      "bs",
      "en",
      "de",
    ]);
  });
});

describe("reflow", () => {
  it("joins wrapped lines, keeps paragraphs and list items", () => {
    expect(reflow("one\ntwo\n\nthree\n- a\n  b\n- c")).toBe("one two\n\nthree\n- a b\n- c");
  });
});
