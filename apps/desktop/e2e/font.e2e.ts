import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { inflateSync } from "node:zlib";
import { expect, test } from "@playwright/test";
import { arabicCharacters, sections } from "./fixture.ts";

// Every character of the Quran text must be drawn by Amiri Quran itself: its @font-face
// `unicode-range` has to include it and the font file has to have a glyph for it. Otherwise
// the webview silently takes the character from another font. No browser needed.

interface Face {
  /** Absolute path of the WOFF file. */
  file: string;
  ranges: [number, number][];
}

/** The @font-face rules in `css`, with their WOFF files resolved by `resolve`. */
function parseFaces(css: string, resolve: (url: string) => string): Face[] {
  const found: Face[] = [];
  for (const block of css.matchAll(/@font-face\s*{([^}]*)}/g)) {
    const body = block[1] ?? "";
    const url = /url\(["']?([^"')]+\.woff)["']?\)/.exec(body)?.[1];
    const range = /unicode-range:\s*([^;]+);/.exec(body)?.[1];
    if (!url || !range) continue;
    const ranges = range.split(",").map((r): [number, number] => {
      const [from = "", to] = r.trim().replace(/^U\+/i, "").split("-");
      return [Number.parseInt(from, 16), Number.parseInt(to ?? from, 16)];
    });
    found.push({ file: resolve(url), ranges });
  }
  return found;
}

/**
 * The faces of "Amiri Quran" in the order the app declares them: the font package's, then
 * the additions in packages/ui/src/quran-font.css (which the app and the site import).
 */
function faces(): Face[] {
  const require = createRequire(import.meta.url);
  const packageCss = require.resolve("@fontsource/amiri-quran/index.css");
  const packageDir = dirname(packageCss);
  const ownCss = require.resolve("@quran-reader/ui/quran-font.css");
  const own = readFileSync(ownCss, "utf8");
  expect(own).toContain('@import "@fontsource/amiri-quran";');
  return [
    ...parseFaces(readFileSync(packageCss, "utf8"), (url) => join(packageDir, url)),
    ...parseFaces(own, (url) => join(packageDir, url.replace("@fontsource/amiri-quran/", ""))),
  ];
}

/** The code points a WOFF 1.0 font maps to a glyph (its cmap, format 4 or 12). */
function mappedCodePoints(woff: Buffer): Set<number> {
  const tables = woff.readUInt16BE(12);
  let cmap: Buffer | undefined;
  for (let i = 0; i < tables; i++) {
    const entry = 44 + i * 20;
    if (woff.toString("latin1", entry, entry + 4) !== "cmap") continue;
    const offset = woff.readUInt32BE(entry + 4);
    const compressed = woff.readUInt32BE(entry + 8);
    const length = woff.readUInt32BE(entry + 12);
    const data = woff.subarray(offset, offset + compressed);
    cmap = compressed < length ? inflateSync(data) : data;
  }
  if (!cmap) throw new Error("no cmap table");
  const mapped = new Set<number>();
  for (let i = 0; i < cmap.readUInt16BE(2); i++) {
    const at = cmap.readUInt32BE(4 + i * 8 + 4);
    const format = cmap.readUInt16BE(at);
    if (format === 12) {
      for (let g = 0; g < cmap.readUInt32BE(at + 12); g++) {
        const group = at + 16 + g * 12;
        const start = cmap.readUInt32BE(group);
        const end = cmap.readUInt32BE(group + 4);
        const glyph = cmap.readUInt32BE(group + 8);
        for (let c = start; c <= end; c++) if (glyph + c - start !== 0) mapped.add(c);
      }
    } else if (format === 4) {
      const segments = cmap.readUInt16BE(at + 6) / 2;
      const ends = at + 14;
      const starts = ends + segments * 2 + 2;
      const deltas = starts + segments * 2;
      const rangeOffsets = deltas + segments * 2;
      for (let s = 0; s < segments; s++) {
        const start = cmap.readUInt16BE(starts + s * 2);
        const end = cmap.readUInt16BE(ends + s * 2);
        const delta = cmap.readInt16BE(deltas + s * 2);
        const rangeOffset = cmap.readUInt16BE(rangeOffsets + s * 2);
        for (let c = start; c <= end && c !== 0xffff; c++) {
          const glyph =
            rangeOffset === 0
              ? (c + delta) & 0xffff
              : cmap.readUInt16BE(rangeOffsets + s * 2 + rangeOffset + (c - start) * 2);
          if (glyph !== 0) mapped.add(c);
        }
      }
    }
  }
  return mapped;
}

function loadFaces() {
  const list = faces();
  expect(list.length).toBeGreaterThan(0);
  const loaded = list.map((f) => ({ ...f, mapped: mappedCodePoints(readFileSync(f.file)) }));
  // Of the faces whose range includes a character, the browser uses the last declared.
  return (cp: number) =>
    loaded.findLast((f) => f.ranges.some(([from, to]) => cp >= from && cp <= to));
}

const hex = (cp: number) => `U+${cp.toString(16).toUpperCase().padStart(4, "0")}`;

test("Amiri Quran has a glyph for every character of the text", () => {
  const faceFor = loadFaces();
  const chars = arabicCharacters();
  expect(chars.length).toBeGreaterThan(40);
  const missing = chars.filter((c) => {
    const cp = c.codePointAt(0) ?? 0;
    return !faceFor(cp)?.mapped.has(cp);
  });
  expect(missing.map((c) => hex(c.codePointAt(0) ?? 0))).toEqual([]);
});

test("every mark is in the font file of the letter (or space) it sits on", () => {
  // WebKit draws a combining mark with the font of its base character, so a mark whose base
  // comes from another font file shows as a box. In the Tanzil text, waqf marks sit on the
  // space between two words.
  const faceFor = loadFaces();
  const broken = new Set<string>();
  for (const s of sections) {
    for (const row of s.rows) {
      const text = "ref" in row ? row.text : (row.bismillah ?? "");
      let base: number | undefined;
      for (const c of text) {
        const cp = c.codePointAt(0) ?? 0;
        if (!/\p{Mn}/u.test(c)) base = cp;
        else if (base !== undefined && !faceFor(base)?.mapped.has(cp)) {
          broken.add(`${hex(cp)} on ${hex(base)}`);
        }
      }
    }
  }
  expect([...broken]).toEqual([]);
});
