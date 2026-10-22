import { RELEASES_URL } from "./links.ts";
import type { Updates } from "./updates.ts";

/** A quiet card in the corner while an update is available, installing or has failed. */
export function UpdateBanner({
  updates,
  onLink,
}: {
  updates: Updates;
  onLink: (url: string) => void;
}) {
  const { state, install, dismiss } = updates;
  const update =
    (state.kind === "available" && !state.dismissed) || state.kind === "installing"
      ? state.update
      : state.kind === "failed"
        ? state.update
        : undefined;
  if (!update) return null;
  const button = "rounded px-2 py-1 text-sm disabled:opacity-40";

  return (
    <div
      role="status"
      className="fixed right-4 bottom-12 z-20 flex w-80 flex-col gap-2 rounded-lg border border-rule bg-paper p-4 text-sm shadow-xl"
    >
      <p className="text-ink">
        {state.kind === "installing"
          ? `Downloading Quran Reader ${update.version}… The app restarts when it is installed.`
          : state.kind === "failed"
            ? `Quran Reader ${update.version} could not be installed: ${state.message}`
            : `Quran Reader ${update.version} is available (you have ${update.currentVersion}).`}
      </p>
      {update.notes && state.kind === "available" && (
        <p className="max-h-24 overflow-y-auto whitespace-pre-line text-muted text-xs">
          {update.notes}
        </p>
      )}
      <div className="flex justify-end gap-1">
        <button
          type="button"
          onClick={() => onLink(RELEASES_URL)}
          className={`${button} mr-auto text-muted hover:text-ink`}
        >
          Release notes
        </button>
        <button
          type="button"
          disabled={state.kind === "installing"}
          onClick={dismiss}
          className={`${button} text-muted hover:text-ink`}
        >
          Later
        </button>
        <button
          type="button"
          disabled={state.kind === "installing"}
          onClick={() => install(update)}
          className={`${button} bg-accent text-paper`}
        >
          {state.kind === "failed" ? "Try again" : "Install and restart"}
        </button>
      </div>
    </div>
  );
}
