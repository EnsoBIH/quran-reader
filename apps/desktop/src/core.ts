import type { CoreInfo, Hit, QuranText, SearchResults, TranslationInfo } from "@quran-reader/types";
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

/** Installed translations: bundled ones first, then the user's imports. */
export async function translations(): Promise<TranslationInfo[]> {
  if (!isTauri()) return [];
  return invoke<TranslationInfo[]>("translations");
}

/** All 6,236 verses of a translation, in Quran order. */
export async function translationText(id: string): Promise<string[]> {
  return invoke<string[]>("translation_text", { id });
}

/**
 * Imports a translation file (Tanzil plain text, `surah|ayah|text` or XML). Rejects with
 * a readable message when the file does not have every verse exactly once.
 */
export async function importTranslation(fileName: string, contents: string) {
  return invoke<TranslationInfo>("import_translation", { fileName, contents });
}

/** Deletes an imported translation. */
export async function removeTranslation(id: string): Promise<void> {
  return invoke("remove_translation", { id });
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
