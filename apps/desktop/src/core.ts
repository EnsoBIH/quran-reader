import type {
  Bookmark,
  CoreInfo,
  Highlight,
  HighlightColor,
  Hit,
  ImportMode,
  ImportPreview,
  Note,
  PlanStatus,
  QuranText,
  SearchResults,
  StudyData,
  TranslationInfo,
  VerseRef,
} from "@quran-reader/types";
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

// Study store (one SQLite file in the app data folder; DECISIONS.md D-021). Every call
// rejects with a readable message, e.g. when the store could not be opened.

/** Highlights, notes, bookmarks and the plan. */
export async function studyData(): Promise<StudyData | null> {
  if (!isTauri()) return null;
  return invoke<StudyData>("study_data");
}

/** Sets a verse's highlight, or removes it with `null`. */
export async function setHighlight(verse: VerseRef, color: HighlightColor | null) {
  return invoke<Highlight | null>("set_highlight", { verse, color });
}

/** Sets a verse's note; an empty body removes it. */
export async function setNote(verse: VerseRef, body: string) {
  return invoke<Note | null>("set_note", { verse, body });
}

/** Adds or relabels a bookmark (`label` null: none), or removes it. */
export async function setBookmark(verse: VerseRef, bookmarked: boolean, label: string | null) {
  return invoke<Bookmark | null>("set_bookmark", { verse, bookmarked, label });
}

/** The reading plan as of `today` (local `YYYY-MM-DD`), or null without one. */
export async function planStatus(today: string) {
  return invoke<PlanStatus | null>("plan_status", { today });
}

export async function startPlan(days: number, start: string, today: string) {
  return invoke<PlanStatus | null>("start_plan", { days, start, today });
}

export async function stopPlan(): Promise<void> {
  return invoke("stop_plan");
}

export async function setPlanDay(day: number, read: boolean, today: string) {
  return invoke<PlanStatus | null>("set_plan_day", { day, read, today });
}

/** The last reading position, from the store. */
export async function position(): Promise<VerseRef | null> {
  if (!isTauri()) return null;
  return invoke<VerseRef | null>("position");
}

export async function setPosition(verse: VerseRef): Promise<void> {
  if (!isTauri()) return;
  return invoke("set_position", { verse });
}

/** Writes an export to the Downloads folder; resolves to its path. */
export async function exportStudy(format: "markdown" | "json", exported: string) {
  return invoke<string>("export_study", { format, exported });
}

/** What importing an export (JSON or Markdown) would change. Changes nothing. */
export async function previewImport(contents: string, mode: ImportMode) {
  return invoke<ImportPreview>("preview_import", { contents, mode });
}

/** Imports an export; resolves to the study data afterwards. */
export async function applyImport(contents: string, mode: ImportMode) {
  return invoke<StudyData>("apply_import", { contents, mode });
}

/** A newer version on GitHub Releases. */
export interface UpdateInfo {
  version: string;
  currentVersion: string;
  /** Release notes, if the release has any. */
  notes: string | null;
}

/**
 * Asks GitHub Releases for a newer version; null when this one is current. Rejects in
 * development builds and when offline.
 */
export async function checkUpdate(): Promise<UpdateInfo | null> {
  if (!isTauri()) return null;
  return invoke<UpdateInfo | null>("check_update");
}

/** Downloads, verifies and installs the update found by {@link checkUpdate}, then restarts. */
export async function installUpdate(): Promise<void> {
  return invoke("install_update");
}

/** Opens a project or credits link in the browser (the app refuses any other URL). */
export async function openLink(url: string): Promise<void> {
  if (!isTauri()) {
    window.open(url, "_blank", "noopener");
    return;
  }
  return invoke("open_link", { url });
}

/** Reports that text is on screen; resolves to milliseconds since the app started. */
export async function textVisible(): Promise<number | null> {
  if (!isTauri()) return null;
  return invoke<number>("text_visible");
}
