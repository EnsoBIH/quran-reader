import type { CoreInfo, Hit, QuranText, SearchResults } from "@quran-reader/types";
import { invoke, isTauri } from "@tauri-apps/api/core";

// Calls into quran-core. Each returns null when running in a plain browser (`pnpm dev:vite`).

export async function coreInfo(): Promise<CoreInfo | null> {
  if (!isTauri()) return null;
  return invoke<CoreInfo>("core_info");
}

/** The whole text with its markers, loaded once at start-up. */
export async function quranText(): Promise<QuranText | null> {
  if (!isTauri()) return null;
  return invoke<QuranText>("quran_text");
}

/** ⌘K palette matches. Rejects with a readable message (e.g. "surah 2 has 286 verses"). */
export async function lookup(query: string, limit: number): Promise<Hit[]> {
  if (!isTauri()) return [];
  return invoke<Hit[]>("lookup", { query, limit });
}

/** ⌘F search. Rejects with a readable message (e.g. "surah 200 does not exist"). */
export async function search(query: string, limit: number): Promise<SearchResults> {
  if (!isTauri()) return { total: 0, hits: [] };
  return invoke<SearchResults>("search", { query, limit });
}

/** Reports that text is on screen; resolves to milliseconds since the app started. */
export async function textVisible(): Promise<number | null> {
  if (!isTauri()) return null;
  return invoke<number>("text_visible");
}
