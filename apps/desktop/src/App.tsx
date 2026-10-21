import type { CoreInfo, Hit, VerseRef } from "@quran-reader/types";
import {
  type ReactNode,
  useCallback,
  useEffect,
  useEffectEvent,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import * as core from "./core.ts";
import {
  canStep,
  EMPTY_HISTORY,
  type History,
  loadPosition,
  savePosition,
  step,
  visit,
} from "./history.ts";
import { buildMushaf, indexOf, type Mushaf, refAt, refOfRow, rowOf } from "./mushaf.ts";
import { Palette } from "./Palette.tsx";
import { Reader, type ReaderHandle } from "./Reader.tsx";
import { SettingsPanel } from "./SettingsPanel.tsx";
import { applySettings, loadSettings, saveSettings } from "./settings.ts";

export interface Loaded {
  mushaf: Mushaf | null;
  info: CoreInfo | null;
  error: string | null;
}

/** Starts loading text, info and the Quran font at once; never rejects. */
export async function load(): Promise<Loaded> {
  try {
    const font = document.fonts.load('1em "Amiri Quran"', "ب").catch(() => []);
    const [text, info] = await Promise.all([core.quranText(), core.coreInfo(), font]);
    return { mushaf: text ? buildMushaf(text) : null, info, error: null };
  } catch (e) {
    return { mushaf: null, info: null, error: String(e) };
  }
}

const storage = () => (typeof localStorage === "undefined" ? undefined : localStorage);

export function App({ loaded }: { loaded: Loaded }) {
  const { mushaf, info, error } = loaded;
  if (!mushaf) {
    return (
      <div className="fixed inset-0 flex flex-col">
        <p className="m-auto text-muted text-sm">
          {error ?? "Browser preview — the text comes from quran-core."}
        </p>
        <Footer info={info} error={error} />
      </div>
    );
  }
  return <ReaderApp mushaf={mushaf} info={info} />;
}

/** The row at the top of the reader, shared without re-rendering the reader itself. */
function createStore(initial: number) {
  let value = initial;
  const listeners = new Set<() => void>();
  return {
    get: () => value,
    set(next: number) {
      if (next === value) return;
      value = next;
      for (const l of listeners) l();
    },
    subscribe(l: () => void) {
      listeners.add(l);
      return () => listeners.delete(l);
    },
  };
}
type Store = ReturnType<typeof createStore>;

function ReaderApp({ mushaf, info }: { mushaf: Mushaf; info: CoreInfo | null }) {
  const [settings, setSettings] = useState(() => loadSettings(storage()));
  const [initialRow] = useState(() =>
    rowOf(mushaf, loadPosition(storage()) ?? { surah: 1, ayah: 1 }),
  );
  const [anchor] = useState(() => createStore(initialRow));
  const [history, setHistory] = useState<History>(EMPTY_HISTORY);
  const historyRef = useRef(history);
  const [palette, setPalette] = useState(false);
  const [panel, setPanel] = useState(false);
  const reader = useRef<ReaderHandle>(null);

  useEffect(() => {
    applySettings(settings, document.documentElement);
    saveSettings(storage(), settings);
  }, [settings]);

  // Remember the reading position across restarts.
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const save = () => savePosition(storage(), refOfRow(mushaf, anchor.get()));
    const unsubscribe = anchor.subscribe(() => {
      clearTimeout(timer);
      timer = setTimeout(save, 300);
    });
    window.addEventListener("pagehide", save);
    return () => {
      clearTimeout(timer);
      unsubscribe();
      window.removeEventListener("pagehide", save);
    };
  }, [anchor, mushaf]);

  const current = useCallback(() => refOfRow(mushaf, anchor.get()), [mushaf, anchor]);
  const commit = (h: History) => {
    historyRef.current = h;
    setHistory(h);
  };
  const show = (to: VerseRef) => reader.current?.scrollToRow(rowOf(mushaf, to));
  const go = (to: VerseRef) => {
    commit(visit(historyRef.current, current(), to));
    show(to);
  };
  const travel = (direction: -1 | 1) => {
    const r = step(historyRef.current, current(), direction);
    if (!r) return;
    commit(r.history);
    show(r.to);
  };
  const surahStep = (direction: -1 | 1) => {
    const surah = current().surah + direction;
    if (surah >= 1 && surah <= mushaf.surahs.length) go({ surah, ayah: 1 });
  };
  const onHit = (hit: Hit) => {
    setPalette(false);
    go(hit.ref);
  };

  const onKey = useEffectEvent((e: KeyboardEvent) => {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      setPanel(false);
      setPalette((p) => !p);
      return;
    }
    if (mod && e.key === ",") {
      e.preventDefault();
      setPanel((p) => !p);
      return;
    }
    if (palette) return;
    const t = e.target as HTMLElement | null;
    const typing = t?.closest("input, select, textarea") != null;
    if ((mod && e.key === "[") || (e.altKey && e.key === "ArrowLeft")) {
      e.preventDefault();
      travel(-1);
    } else if ((mod && e.key === "]") || (e.altKey && e.key === "ArrowRight")) {
      e.preventDefault();
      travel(1);
    } else if (!typing && !mod && !e.altKey && !e.shiftKey) {
      if (e.key === "ArrowLeft") surahStep(-1);
      if (e.key === "ArrowRight") surahStep(1);
    }
  });
  const onMouseButton = useEffectEvent((e: MouseEvent) => {
    if (e.button === 3) travel(-1);
    if (e.button === 4) travel(1);
  });
  useEffect(() => {
    const key = (e: KeyboardEvent) => onKey(e);
    const mouse = (e: MouseEvent) => onMouseButton(e);
    window.addEventListener("keydown", key);
    window.addEventListener("mouseup", mouse);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("mouseup", mouse);
    };
  }, []);

  const onAnchor = useCallback((row: number) => anchor.set(row), [anchor]);
  const onFirstPaint = useCallback(() => {
    const inPage = Math.round(performance.now());
    core.textVisible().then((ms) => {
      console.info(`text visible after ${ms ?? "?"} ms (${inPage} ms in the webview)`);
    });
  }, []);

  return (
    <div className="fixed inset-0 flex flex-col">
      <TopBar
        mushaf={mushaf}
        anchor={anchor}
        history={history}
        onGo={go}
        onTravel={travel}
        onSurah={surahStep}
        onPalette={() => setPalette(true)}
        onSettings={() => setPanel((p) => !p)}
        settingsPanel={
          panel && (
            <SettingsPanel
              settings={settings}
              onChange={setSettings}
              onClose={() => setPanel(false)}
            />
          )
        }
      />
      <Reader
        ref={reader}
        mushaf={mushaf}
        settings={settings}
        initialRow={initialRow}
        onAnchor={onAnchor}
        onFirstPaint={onFirstPaint}
      />
      <Footer info={info} error={null} />
      {palette && <Palette mushaf={mushaf} onGo={onHit} onClose={() => setPalette(false)} />}
    </div>
  );
}

interface TopBarProps {
  mushaf: Mushaf;
  anchor: Store;
  history: History;
  onGo: (to: VerseRef) => void;
  onTravel: (direction: -1 | 1) => void;
  onSurah: (direction: -1 | 1) => void;
  onPalette: () => void;
  onSettings: () => void;
  settingsPanel: ReactNode;
}

const button =
  "rounded px-2 py-1 text-muted hover:text-ink disabled:opacity-40 disabled:hover:text-muted";
const select = "rounded border border-rule bg-paper px-1.5 py-1 tabular-nums";

function TopBar({
  mushaf,
  anchor,
  history,
  onGo,
  onTravel,
  onSurah,
  onPalette,
  onSettings,
  settingsPanel,
}: TopBarProps) {
  const row = useSyncExternalStore(anchor.subscribe, anchor.get);
  const ref = refOfRow(mushaf, row);
  const index = indexOf(mushaf, ref);
  const juz = mushaf.juzOf[index] ?? 1;
  const hizb = Math.ceil((mushaf.quarterOf[index] ?? 1) / 4);
  const page = mushaf.pageOf[index] ?? 1;
  const start = (list: number[], i: number) => refAt(mushaf, list[i] ?? 0);
  const numbers = (n: number) => Array.from({ length: n }, (_, i) => i + 1);

  return (
    <nav className="relative flex shrink-0 items-center gap-2 border-b border-rule px-3 py-2 text-sm">
      <button
        type="button"
        className={button}
        onClick={() => onTravel(-1)}
        disabled={!canStep(history, -1)}
        aria-label="Back"
        title="Back (⌘[)"
      >
        ‹
      </button>
      <button
        type="button"
        className={button}
        onClick={() => onTravel(1)}
        disabled={!canStep(history, 1)}
        aria-label="Forward"
        title="Forward (⌘])"
      >
        ›
      </button>
      <span className="mx-1 h-4 w-px bg-rule" />
      <button
        type="button"
        className={button}
        onClick={() => onSurah(-1)}
        disabled={ref.surah <= 1}
        aria-label="Previous surah"
        title="Previous surah (←)"
      >
        ←
      </button>
      <select
        aria-label="Surah"
        value={ref.surah}
        onChange={(e) => onGo({ surah: Number(e.target.value), ayah: 1 })}
        className={`${select} max-w-44`}
      >
        {mushaf.surahs.map((s) => (
          <option key={s.number} value={s.number}>
            {s.number}. {s.nameLatin}
          </option>
        ))}
      </select>
      <button
        type="button"
        className={button}
        onClick={() => onSurah(1)}
        disabled={ref.surah >= mushaf.surahs.length}
        aria-label="Next surah"
        title="Next surah (→)"
      >
        →
      </button>
      <span className="hidden text-muted tabular-nums lg:inline">
        {ref.surah}:{ref.ayah}
      </span>

      <div className="ml-auto flex items-center gap-2">
        <select
          aria-label="Juz"
          value={juz}
          onChange={(e) => onGo(start(mushaf.juzStarts, Number(e.target.value) - 1))}
          className={select}
        >
          {numbers(mushaf.juzStarts.length).map((n) => (
            <option key={n} value={n}>
              Juz {n}
            </option>
          ))}
        </select>
        <select
          aria-label="Hizb"
          value={hizb}
          onChange={(e) => onGo(start(mushaf.quarterStarts, (Number(e.target.value) - 1) * 4))}
          className={`${select} hidden md:block`}
        >
          {numbers(mushaf.quarterStarts.length / 4).map((n) => (
            <option key={n} value={n}>
              Hizb {n}
            </option>
          ))}
        </select>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            const field = e.currentTarget.elements.namedItem("page") as HTMLInputElement;
            const n = Number(field.value);
            if (Number.isInteger(n) && n >= 1 && n <= mushaf.pageStarts.length) {
              onGo(start(mushaf.pageStarts, n - 1));
              field.value = "";
              field.blur();
            }
          }}
        >
          <input
            name="page"
            type="number"
            min={1}
            max={mushaf.pageStarts.length}
            placeholder={`p. ${page}`}
            aria-label={`Page (now ${page})`}
            className={`${select} w-20`}
          />
        </form>
        <button type="button" onClick={onPalette} className={`${select} text-muted hover:text-ink`}>
          Go to… <kbd className="font-sans">⌘K</kbd>
        </button>
        <button
          type="button"
          onClick={onSettings}
          className={button}
          aria-label="Reader settings"
          title="Reader settings (⌘,)"
        >
          Aa
        </button>
      </div>
      {settingsPanel}
    </nav>
  );
}

function Footer({ info, error }: { info: CoreInfo | null; error: string | null }) {
  return (
    <footer className="flex shrink-0 justify-between gap-4 border-t border-rule px-5 py-2 text-muted text-xs tabular-nums">
      <span>
        {error
          ? `core error: ${error}`
          : info
            ? `quran-core ${info.version} · ${info.surahCount} surahs · ${info.ayahCount} verses`
            : "browser preview — quran-core not attached"}
      </span>
      <span>Quran text: Tanzil.net (CC BY 3.0) · tanzil.net</span>
    </footer>
  );
}
