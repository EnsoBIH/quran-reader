import type { ReaderView } from "@quran-reader/types";
import { useEffect, useRef } from "react";
import { clampSize, LIMITS, type Settings, THEMES, VIEWS } from "./settings.ts";

interface SettingsPanelProps {
  settings: Settings;
  onChange: (s: Settings) => void;
  onClose: () => void;
}

const VIEW_LABEL: Record<ReaderView, string> = {
  arabic: "Arabic",
  stacked: "Stacked",
  columns: "Columns",
};

export function SettingsPanel({ settings, onChange, onClose }: SettingsPanelProps) {
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
        onPick={(view) => onChange({ ...settings, view })}
      />
      {settings.view !== "arabic" && (
        <p className="-mt-2 text-muted text-xs">
          No translation is installed yet; importing one comes in a later version.
        </p>
      )}
      {size("arabicSize", "Arabic size")}
      {size("translationSize", "Translation size")}
      {size("measure", "Line width")}
    </div>
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
