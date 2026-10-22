import { useCallback, useEffect, useState } from "react";
import * as core from "./core.ts";

// Updates from GitHub Releases (DECISIONS.md D-027). The check runs in Rust; the update is
// signed, and installing it needs the user's click.

export type UpdateState =
  | { kind: "idle" }
  | { kind: "checking"; manual: boolean }
  /** Checked on request: nothing newer. */
  | { kind: "current" }
  | { kind: "available"; update: core.UpdateInfo; dismissed: boolean }
  | { kind: "installing"; update: core.UpdateInfo }
  | { kind: "failed"; message: string; manual: boolean; update?: core.UpdateInfo };

/** How long after start-up the automatic check runs, so it never competes with the text. */
const AUTO_DELAY_MS = 5000;

export function useUpdates(auto: boolean) {
  const [state, setState] = useState<UpdateState>({ kind: "idle" });

  const check = useCallback(async (manual: boolean) => {
    setState({ kind: "checking", manual });
    try {
      const update = await core.checkUpdate();
      setState(update ? { kind: "available", update, dismissed: false } : { kind: "current" });
    } catch (e) {
      setState({ kind: "failed", message: String(e), manual });
    }
  }, []);

  // Once per start, if allowed. Errors (offline, development build) stay quiet.
  useEffect(() => {
    if (!auto) return;
    const timer = setTimeout(() => check(false), AUTO_DELAY_MS);
    return () => clearTimeout(timer);
  }, [auto, check]);

  const install = useCallback(async (update: core.UpdateInfo) => {
    setState({ kind: "installing", update });
    try {
      // Restarts the app when done; on Windows the installer does.
      await core.installUpdate();
    } catch (e) {
      setState({ kind: "failed", message: String(e), manual: true, update });
    }
  }, []);

  const dismiss = useCallback(
    () => setState((s) => (s.kind === "available" ? { ...s, dismissed: true } : { kind: "idle" })),
    [],
  );

  return { state, check: () => check(true), install, dismiss };
}

export type Updates = ReturnType<typeof useUpdates>;

/** One line for the settings panel. */
export function updateStatus(state: UpdateState, version: string | null): string | null {
  switch (state.kind) {
    case "idle":
      return null;
    case "checking":
      return "Checking for updates…";
    case "current":
      return version ? `Quran Reader ${version} is the latest version.` : "No update available.";
    case "available":
      return `Version ${state.update.version} is available.`;
    case "installing":
      return `Installing ${state.update.version}…`;
    case "failed":
      return state.manual || state.update ? `Update failed: ${state.message}` : null;
  }
}
