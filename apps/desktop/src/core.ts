import type { CoreInfo, Surah, SurahText } from "@quran-reader/types";
import { invoke, isTauri } from "@tauri-apps/api/core";

// Calls into quran-core. Each returns null when running in a plain browser (`pnpm dev:vite`).

export async function coreInfo(): Promise<CoreInfo | null> {
  if (!isTauri()) return null;
  return invoke<CoreInfo>("core_info");
}

export async function surahs(): Promise<Surah[] | null> {
  if (!isTauri()) return null;
  return invoke<Surah[]>("surahs");
}

export async function surah(number: number): Promise<SurahText | null> {
  if (!isTauri()) return null;
  return invoke<SurahText>("surah", { number });
}
