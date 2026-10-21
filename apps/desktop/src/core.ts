import type { CoreInfo } from "@quran-reader/types";
import { invoke, isTauri } from "@tauri-apps/api/core";

/** Calls into quran-core. Returns null when running in a plain browser (`pnpm dev:vite`). */
export async function coreInfo(): Promise<CoreInfo | null> {
  if (!isTauri()) return null;
  return invoke<CoreInfo>("core_info");
}
