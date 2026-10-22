import { readFileSync } from "node:fs";
import type { Section } from "./page.tsx";

/** verses.json, generated from the app's data by `cargo test -p quran-core --test fixture`. */
export const sections: Section[] = JSON.parse(
  readFileSync(new URL("./verses.json", import.meta.url), "utf8"),
).sections;

/** Every distinct character of the Arabic on the test page (which covers the whole text). */
export function arabicCharacters(): string[] {
  const chars = new Set<string>();
  for (const s of sections) {
    for (const row of s.rows) {
      const text = "ref" in row ? row.text : (row.bismillah ?? "");
      for (const c of text) chars.add(c);
    }
  }
  return [...chars].sort();
}
