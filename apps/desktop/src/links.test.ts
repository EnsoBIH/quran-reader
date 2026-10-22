import { describe, expect, it } from "vitest";
import { osName, problemUrl, REPO_URL } from "./links.ts";

const MAC =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)";
const WINDOWS =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0";
const LINUX = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)";

describe("osName", () => {
  it("names the three webviews' systems", () => {
    expect(osName(MAC)).toBe("macOS");
    expect(osName(WINDOWS)).toBe("Windows");
    expect(osName(LINUX)).toBe("Linux");
    expect(osName("")).toBe("");
  });
});

describe("problemUrl", () => {
  it("fills in the issue form fields", () => {
    const url = new URL(
      problemUrl({
        version: "1.0.0",
        verse: "2:255",
        translation: "Pickthall (1930)",
        view: "stacked",
        theme: "sepia",
        userAgent: MAC,
      }),
    );
    expect(`${url.origin}${url.pathname}`).toBe(`${REPO_URL}/issues/new`);
    expect(Object.fromEntries(url.searchParams)).toEqual({
      template: "problem.yml",
      verse: "2:255",
      version: "1.0.0",
      os: "macOS",
      setup: "view: stacked, translation: Pickthall (1930), theme: sepia",
    });
  });

  it("leaves out what it does not know", () => {
    const url = new URL(
      problemUrl({
        version: null,
        verse: null,
        translation: "Pickthall (1930)",
        view: "arabic",
        theme: "system",
        userAgent: "",
      }),
    );
    expect(Object.fromEntries(url.searchParams)).toEqual({
      template: "problem.yml",
      setup: "view: arabic, theme: system",
    });
  });
});
