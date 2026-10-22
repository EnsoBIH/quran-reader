import { expect, type Page, test } from "@playwright/test";
import { sections } from "./fixture.ts";

// Arabic rendering in each webview engine: one screenshot per section of the test page,
// compared with the committed baseline of that engine, plus measurements that hold in every
// engine and catch broken shaping without a baseline.

test.beforeEach(async ({ page }) => {
  await page.goto("/e2e/index.html");
  await page.locator("body[data-ready]").waitFor();
});

for (const { id } of sections) {
  test(`screenshot: ${id}`, async ({ page }) => {
    await expect(page.locator(`[data-section="${id}"]`)).toHaveScreenshot(`${id}.png`);
  });
}

/** Advance widths in px of `texts`, set in Amiri Quran at 40px. */
function widths(page: Page, texts: string[]): Promise<number[]> {
  return page.evaluate((texts) => {
    return texts.map((text) => {
      const span = document.createElement("span");
      span.dir = "rtl";
      span.lang = "ar";
      span.style.font = '40px "Amiri Quran"';
      span.style.whiteSpace = "pre";
      span.textContent = text;
      document.body.append(span);
      const width = span.getBoundingClientRect().width;
      span.remove();
      return width;
    });
  }, texts);
}

const ZWNJ = "‌";

test("the Quran font is loaded", async ({ page }) => {
  const loaded = await page.evaluate(() =>
    [...document.fonts].some(
      (f) => f.family.replace(/["']/g, "") === "Amiri Quran" && f.status === "loaded",
    ),
  );
  expect(loaded).toBe(true);
});

test("letters join (contextual forms)", async ({ page }) => {
  // Joined forms are much narrower than the isolated forms that a zero-width non-joiner
  // between the letters forces; without shaping both would be the same width.
  const words = ["بسم", "محمد", "سلام", "نعم", "كتب"];
  const joined = await widths(page, words);
  const isolated = await widths(
    page,
    words.map((w) => [...w].join(ZWNJ)),
  );
  words.forEach((word, i) => {
    expect(joined[i], word).toBeLessThan((isolated[i] ?? 0) * 0.9);
  });
});

test("tashkeel attaches to its letter without taking space", async ({ page }) => {
  // Marks positioned on their base letter add no width; marks drawn as separate glyphs
  // (or with a dotted circle for a broken cluster) would.
  const pairs = [
    ["بِسْمِ", "بسم"],
    ["مُحَمَّدٌ", "محمد"],
    ["كَتَبَ", "كتب"],
    ["ٱلرَّحْمَٰنِ", "ٱلرحمن"],
  ] as const;
  const marked = await widths(
    page,
    pairs.map(([m]) => m),
  );
  const bare = await widths(
    page,
    pairs.map(([, b]) => b),
  );
  pairs.forEach(([word], i) => {
    expect(Math.abs((marked[i] ?? 0) - (bare[i] ?? 0)), word).toBeLessThanOrEqual(1);
  });
});

test("the end-of-ayah sign encloses its number", async ({ page }) => {
  // Amiri Quran draws the digits after U+06DD inside the sign, so the pair is as wide as
  // the sign alone (this is how the reader numbers verses).
  const [sign = 0, digits = 0, numbered = 0] = await widths(page, ["۝", "٢٨٦", "۝٢٨٦"]);
  expect(digits).toBeGreaterThan(0);
  expect(numbered).toBeLessThanOrEqual(sign + 1);
});

test("verses run right to left", async ({ page }) => {
  const order = await page.evaluate(() => {
    const p = document.querySelector('[data-section="fatiha"] p[lang="ar"]');
    const text = p?.firstChild;
    if (!text || text.nodeType !== Node.TEXT_NODE) return null;
    const rect = (start: number) => {
      const range = document.createRange();
      range.setStart(text, start);
      range.setEnd(text, start + 1);
      return range.getBoundingClientRect();
    };
    // The first letter of the first word and of the last word (1:1 fits on one line).
    const last = (text.textContent ?? "").lastIndexOf(" ") + 1;
    return { first: rect(0).right, last: rect(last).right };
  });
  expect(order).not.toBeNull();
  expect(order?.first).toBeGreaterThan(order?.last ?? Number.POSITIVE_INFINITY);
});
