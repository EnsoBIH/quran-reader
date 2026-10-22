import type { ReaderView, TranslationInfo } from "@quran-reader/types";
import { useEffect, useRef, useState } from "react";
import { clampSize, LIMITS, type Settings, THEMES, VIEWS, withView } from "./settings.ts";
import { type Library, languageName, languages } from "./translations.ts";
import { type Updates, updateStatus } from "./updates.ts";

interface SettingsPanelProps {
  settings: Settings;
  onChange: (s: Settings) => void;
  library: Library;
  /** The translation in view (the chosen one, or the fallback). */
  shown: TranslationInfo | null;
  updates: Updates;
  /** The app version, null in the browser preview. */
  version: string | null;
  onClose: () => void;
}

const VIEW_LABEL: Record<ReaderView, string> = {
  arabic: "Arabic",
  stacked: "Stacked",
  columns: "Columns",
};

export function SettingsPanel({
  settings,
  onChange,
  library,
  shown,
  updates,
  version,
  onClose,
}: SettingsPanelProps) {
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (!panel.current?.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    // Next tick, so the click that opened the panel doesn't close it.
    const id = setTimeout(() => window.addEventListener("mousedown", onDown));
    window.addEventListener("keydown", onKey);
    return () => {
      clearTimeout(id);
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  const size = (name: keyof typeof LIMITS, label: string) => (
    <label className="flex flex-col gap-1">
      <span className="flex justify-between">
        {label}
        <span className="text-muted tabular-nums">{settings[name]} rem</span>
      </span>
      <input
        type="range"
        min={LIMITS[name].min}
        max={LIMITS[name].max}
        step={LIMITS[name].step}
        value={settings[name]}
        onChange={(e) => onChange({ ...settings, [name]: clampSize(name, Number(e.target.value)) })}
        className="accent-accent"
      />
    </label>
  );

  return (
    <div
      ref={panel}
      role="dialog"
      aria-label="Reader settings"
      className="absolute top-full right-3 z-10 mt-2 flex w-72 flex-col gap-4 rounded-lg border border-rule bg-paper p-4 text-sm shadow-xl"
    >
      <Choice
        legend="Theme"
        options={THEMES}
        value={settings.theme}
        label={(t) => t}
        onPick={(theme) => onChange({ ...settings, theme })}
      />
      <Choice
        legend="View"
        options={VIEWS}
        value={settings.view}
        label={(v) => VIEW_LABEL[v]}
        onPick={(view) => onChange(withView(settings, view))}
      />
      <TranslationChoice
        library={library}
        shown={shown}
        onPick={(translation) => onChange({ ...settings, translation })}
      />
      {size("arabicSize", "Arabic size")}
      {size("translationSize", "Translation size")}
      {size("measure", "Line width")}
      <UpdateChoice
        updates={updates}
        version={version}
        auto={settings.checkUpdates}
        onAuto={(checkUpdates) => onChange({ ...settings, checkUpdates })}
      />
    </div>
  );
}

/** Automatic check on or off, a manual check, and its result. */
function UpdateChoice({
  updates,
  version,
  auto,
  onAuto,
}: {
  updates: Updates;
  version: string | null;
  auto: boolean;
  onAuto: (on: boolean) => void;
}) {
  const { state } = updates;
  const busy = state.kind === "checking" || state.kind === "installing";
  const status = updateStatus(state, version);
  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="mb-1">Updates</legend>
      <label className="flex items-center gap-2 text-xs">
        <input
          type="checkbox"
          checked={auto}
          onChange={(e) => onAuto(e.target.checked)}
          className="accent-accent"
        />
        Check for a new version at start-up
      </label>
      <div className="flex items-center gap-2 text-xs">
        <button
          type="button"
          disabled={busy}
          onClick={updates.check}
          className="rounded border border-rule px-2 py-1 text-muted hover:text-ink disabled:opacity-40"
        >
          Check now
        </button>
        {state.kind === "available" && (
          <button
            type="button"
            onClick={() => updates.install(state.update)}
            className="rounded bg-accent px-2 py-1 text-paper"
          >
            Install {state.update.version}
          </button>
        )}
      </div>
      {status && (
        <p role="status" className="text-muted text-xs">
          {status}
        </p>
      )}
      <p className="text-muted text-xs">
        Only asks GitHub Releases for the latest version number. Your notes and reading stay on this
        computer.
      </p>
    </fieldset>
  );
}

function Choice<T extends string>({
  legend,
  options,
  value,
  label,
  onPick,
}: {
  legend: string;
  options: T[];
  value: T;
  label: (o: T) => string;
  onPick: (o: T) => void;
}) {
  return (
    <fieldset className="flex flex-col gap-1">
      <legend className="mb-1">{legend}</legend>
      <div className="flex gap-1 rounded-md bg-rule/50 p-0.5">
        {options.map((o) => (
          <button
            key={o}
            type="button"
            aria-pressed={value === o}
            onClick={() => onPick(o)}
            className="flex-1 rounded px-2 py-1 capitalize text-muted hover:text-ink aria-pressed:bg-paper aria-pressed:text-ink aria-pressed:shadow-sm"
          >
            {label(o)}
          </button>
        ))}
      </div>
    </fieldset>
  );
}

/** Language, translation, import and removal of imports. */
function TranslationChoice({
  library,
  shown,
  onPick,
}: {
  library: Library;
  shown: TranslationInfo | null;
  onPick: (id: string) => void;
}) {
  const { installed } = library;
  const [language, setLanguage] = useState(shown?.language ?? "");
  const [status, setStatus] = useState<{ error: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const file = useRef<HTMLInputElement>(null);
  const langs = languages(installed);
  const lang = langs.includes(language) ? language : (shown?.language ?? langs[0] ?? "");
  const inLanguage = installed.filter((t) => t.language === lang);
  const field = "rounded border border-rule bg-paper px-1.5 py-1";

  const onFile = async (f: File | undefined) => {
    if (!f) return;
    setBusy(true);
    setStatus(null);
    try {
      const info = await library.importFile(f);
      setLanguage(info.language);
      onPick(info.id);
      setStatus({ error: false, text: `Imported “${info.name}” (all 6,236 verses).` });
    } catch (e) {
      setStatus({ error: true, text: `Not imported: ${String(e)}` });
    } finally {
      setBusy(false);
      if (file.current) file.current.value = "";
    }
  };

  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="mb-1">Translation</legend>
      {installed.length === 0 ? (
        <p className="text-muted text-xs">No translation is installed.</p>
      ) : (
        <div className="flex gap-1">
          <select
            aria-label="Translation language"
            value={lang}
            onChange={(e) => {
              setLanguage(e.target.value);
              const first = installed.find((t) => t.language === e.target.value);
              if (first) onPick(first.id);
            }}
            className={`${field} w-24`}
          >
            {langs.map((l) => (
              <option key={l} value={l}>
                {languageName(l)}
              </option>
            ))}
          </select>
          <select
            aria-label="Translation"
            value={shown?.language === lang ? shown.id : ""}
            onChange={(e) => onPick(e.target.value)}
            className={`${field} min-w-0 flex-1`}
          >
            {shown?.language !== lang && <option value="">Choose…</option>}
            {inLanguage.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
                {t.bundled ? "" : " (imported)"}
              </option>
            ))}
          </select>
        </div>
      )}
      <div className="flex items-center gap-2 text-xs">
        <button
          type="button"
          disabled={busy}
          onClick={() => file.current?.click()}
          className="rounded border border-rule px-2 py-1 text-muted hover:text-ink disabled:opacity-40"
        >
          {busy ? "Importing…" : "Import…"}
        </button>
        {shown && !shown.bundled && (
          <button
            type="button"
            disabled={busy}
            onClick={() => {
              library
                .remove(shown.id)
                .then(() => setStatus({ error: false, text: `Removed “${shown.name}”.` }))
                .catch((e: unknown) => setStatus({ error: true, text: String(e) }));
            }}
            className="rounded px-2 py-1 text-muted hover:text-ink disabled:opacity-40"
          >
            Remove “{shown.name}”
          </button>
        )}
        <input
          ref={file}
          type="file"
          accept=".txt,.xml,text/plain,application/xml,text/xml"
          className="hidden"
          aria-label="Translation file"
          onChange={(e) => onFile(e.target.files?.[0])}
        />
      </div>
      {status && (
        <p role="status" className={`text-xs ${status.error ? "text-accent" : "text-muted"}`}>
          {status.text}
        </p>
      )}
      <p className="text-muted text-xs">
        Import a translation you downloaded yourself, e.g. from tanzil.net/trans (plain text or
        XML). It stays on this computer.
      </p>
    </fieldset>
  );
}
