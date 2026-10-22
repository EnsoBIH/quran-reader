import type {
  CoreInfo,
  Hit,
  SearchHit,
  StudyData,
  TranslationInfo,
  VerseRef,
} from "@quran-reader/types";
import {
  type ReactNode,
  useCallback,
  useEffect,
  useEffectEvent,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import { Compare } from "./Compare.tsx";
import { Credits } from "./Credits.tsx";
import * as core from "./core.ts";
import { canStep, EMPTY_HISTORY, type History, migratePosition, step, visit } from "./history.ts";
import { buildMushaf, indexOf, type Mushaf, refAt, refOfRow, rowOf } from "./mushaf.ts";
import { NoteEditor } from "./NoteEditor.tsx";
import { Palette } from "./Palette.tsx";
import { Reader, type ReaderHandle, type VerseAction, type VerseMarks } from "./Reader.tsx";
import { SearchPanel } from "./SearchPanel.tsx";
import { SettingsPanel } from "./SettingsPanel.tsx";
import { StudyPanel, type StudyTab } from "./StudyPanel.tsx";
import { arabicRanges, textRanges } from "./search.ts";
import {
  applySettings,
  loadSettings,
  pickTranslation,
  saveSettings,
  toggleTranslation,
} from "./settings.ts";
import {
  COLOR_NAMES,
  copyText,
  EMPTY_STUDY,
  nextColor,
  refLabel,
  useStudy,
  writeClipboard,
} from "./study.ts";
import { useLibrary } from "./translations.ts";

export interface Loaded {
  mushaf: Mushaf | null;
  info: CoreInfo | null;
  translations: TranslationInfo[];
  /** The text of the translation shown at start-up, if the view shows one. */
  texts: Map<string, string[]>;
  study: StudyData;
  /** Why the study store could not be read, if it could not. */
  studyError: string | null;
  /** Last reading position. */
  position: VerseRef | null;
  error: string | null;
}

const storage = () => (typeof localStorage === "undefined" ? undefined : localStorage);
const userLanguages = () => (typeof navigator === "undefined" ? [] : navigator.languages);

/** Starts loading text, info, translations and the Quran font at once; never rejects. */
export async function load(): Promise<Loaded> {
  try {
    const font = document.fonts.load('1em "Amiri Quran"', "ب").catch(() => []);
    const settings = loadSettings(storage());
    // The translation in view is needed for the first frame; the others load when used.
    const translations = core.translations().then(async (installed) => {
      const shown = pickTranslation(installed, settings.translation, userLanguages());
      const texts = new Map<string, string[]>();
      if (shown && settings.view !== "arabic") {
        texts.set(shown.id, await core.translationText(shown.id));
      }
      return { installed, texts };
    });
    const study = core.studyData().then(
      (data) => ({ data: data ?? EMPTY_STUDY, error: null }),
      (e: unknown) => ({ data: EMPTY_STUDY, error: String(e) }),
    );
    const position = core
      .position()
      .catch(() => null)
      .then((stored) => migratePosition(stored, storage(), core.setPosition));
    const [text, info, t, s, pos] = await Promise.all([
      core.quranText(),
      core.coreInfo(),
      translations,
      study,
      position,
      font,
    ]);
    return {
      mushaf: text ? buildMushaf(text) : null,
      info,
      translations: t.installed,
      texts: t.texts,
      study: s.data,
      studyError: s.error,
      position: pos,
      error: null,
    };
  } catch (e) {
    return {
      mushaf: null,
      info: null,
      translations: [],
      texts: new Map(),
      study: EMPTY_STUDY,
      studyError: null,
      position: null,
      error: String(e),
    };
  }
}

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
  return <ReaderApp mushaf={mushaf} info={info} loaded={loaded} />;
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

function ReaderApp({
  mushaf,
  info,
  loaded,
}: {
  mushaf: Mushaf;
  info: CoreInfo | null;
  loaded: Loaded;
}) {
  const [settings, setSettings] = useState(() => loadSettings(storage()));
  const library = useLibrary(loaded.translations, loaded.texts);
  const shown = pickTranslation(library.installed, settings.translation, userLanguages());
  const shownText = shown ? library.texts.get(shown.id) : undefined;
  const { ensure } = library;
  const [initialRow] = useState(() => rowOf(mushaf, loaded.position ?? { surah: 1, ayah: 1 }));
  const study = useStudy(mushaf, loaded.study, loaded.studyError);
  const [selected, setSelected] = useState<number | null>(null);
  const [studyPanel, setStudyPanel] = useState(false);
  const [studyTab, setStudyTab] = useState<StudyTab>("bookmarks");
  const [noteFor, setNoteFor] = useState<number | null>(null);
  const [toast, setToast] = useState<{ text: string; id: number } | null>(null);
  const [anchor] = useState(() => createStore(initialRow));
  const [history, setHistory] = useState<History>(EMPTY_HISTORY);
  const historyRef = useRef(history);
  const [palette, setPalette] = useState(false);
  const [panel, setPanel] = useState(false);
  const [search, setSearch] = useState<{ query: string; focus: number } | null>(null);
  const [lastQuery, setLastQuery] = useState("");
  const [found, setFound] = useState<{ hit: SearchHit } | null>(null);
  const [compare, setCompare] = useState(false);
  const [credits, setCredits] = useState(false);
  const reader = useRef<ReaderHandle>(null);

  // Texts load when something shows them: the reader, compare (C) and search (⌘F).
  const ids = library.installed.map((t) => t.id).join("\n");
  const all = compare || search !== null;
  useEffect(() => {
    if (all) ensure(ids === "" ? [] : ids.split("\n"));
    else if (shown && settings.view !== "arabic") ensure([shown.id]);
  }, [all, ids, shown, settings.view, ensure]);
  // Stable identity: the reader re-estimates every row height when it changes.
  const shownId = shown?.id;
  const shownLanguage = shown?.language;
  const translation = useMemo(
    () =>
      shownId && shownLanguage && shownText
        ? { id: shownId, language: shownLanguage, verses: shownText }
        : null,
    [shownId, shownLanguage, shownText],
  );
  const marks: VerseMarks | null = found && {
    index: found.hit.index,
    arabic: arabicRanges(found.hit),
    translation: translation ? textRanges(found.hit, translation.id) : [],
  };

  useEffect(() => {
    applySettings(settings, document.documentElement);
    saveSettings(storage(), settings);
  }, [settings]);

  // Remember the reading position across restarts (in the study store).
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const save = () => {
      core.setPosition(refOfRow(mushaf, anchor.get())).catch(() => {});
    };
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
  const openSearch = () => {
    setStudyPanel(false);
    setSearch((s) => ({ query: s?.query ?? lastQuery, focus: (s?.focus ?? 0) + 1 }));
  };
  const toggleStudy = () => {
    if (search) closeSearch();
    setStudyPanel((p) => !p);
  };

  const notify = useCallback((text: string) => setToast({ text, id: Date.now() }), []);
  useEffect(() => {
    if (!toast) return;
    const id = setTimeout(() => setToast(null), 2200);
    return () => clearTimeout(id);
  }, [toast]);

  /** The verse 1–6, N, B and ⌘C act on: the selected one while it is in view, else the one being read. */
  const target = () => {
    const row = selected === null ? -1 : (mushaf.verseRow[selected] ?? -1);
    if (selected !== null && reader.current?.isVisible(row)) return selected;
    return indexOf(mushaf, current());
  };
  const act = (index: number, action: VerseAction) => {
    const ref = refAt(mushaf, index);
    const label = refLabel(mushaf, ref);
    const failed = (e: unknown) => notify(String(e));
    setSelected(index);
    if (action.kind === "highlight") {
      const color = nextColor(study.index.highlights.get(index), action.color);
      study
        .setHighlight(ref, color)
        .then(() =>
          notify(
            color
              ? `${label} highlighted ${COLOR_NAMES[color - 1]}`
              : `Highlight removed from ${label}`,
          ),
        )
        .catch(failed);
    } else if (action.kind === "note") {
      setNoteFor(index);
    } else if (action.kind === "bookmark") {
      const on = !study.index.bookmarks.has(index);
      study
        .setBookmark(ref, on)
        .then(() => notify(on ? `Bookmarked ${label}` : `Bookmark removed from ${label}`))
        .catch(failed);
    } else {
      const shownVerse =
        settings.view !== "arabic" && translation ? translation.verses[index] : undefined;
      writeClipboard(
        copyText(
          mushaf,
          index,
          shown && shownVerse ? { name: shown.name, verse: shownVerse } : null,
        ),
      )
        .then(() => notify(`Copied ${label}`))
        .catch(failed);
    }
  };
  // Stable for the memoized rows; always runs the latest `act`.
  const actRef = useRef(act);
  actRef.current = act;
  const onAction = useCallback((index: number, a: VerseAction) => actRef.current(index, a), []);
  const goStudy = (to: VerseRef) => {
    go(to);
    setSelected(indexOf(mushaf, to));
  };
  const closeSearch = () => {
    if (search) setLastQuery(search.query);
    setSearch(null);
    setFound(null);
    reader.current?.focus();
  };
  const onSearchHit = (hit: SearchHit) => {
    setFound({ hit });
    // Land on the verse itself, not on the surah header above verse 1.
    commit(visit(historyRef.current, current(), hit.ref));
    reader.current?.scrollToRow(mushaf.verseRow[hit.index] ?? 0);
  };

  const onKey = useEffectEvent((e: KeyboardEvent) => {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === "f") {
      e.preventDefault();
      setPalette(false);
      setPanel(false);
      openSearch();
      return;
    }
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
    if (palette || credits || noteFor !== null) return;
    const t = e.target as HTMLElement | null;
    const typing = t?.closest("input, select, textarea") != null;
    const plain = !typing && !mod && !e.altKey;
    // ⌘C copies the verse when no text is selected.
    if (mod && !e.altKey && !typing && e.key.toLowerCase() === "c" && !compare) {
      if (window.getSelection()?.isCollapsed !== false) {
        e.preventDefault();
        act(target(), { kind: "copy" });
      }
      return;
    }
    if (!typing && !mod && !e.altKey && e.key.toLowerCase() === "t") {
      e.preventDefault();
      setSettings(toggleTranslation);
      return;
    }
    if (!typing && !mod && !e.altKey && e.key.toLowerCase() === "c") {
      e.preventDefault();
      setPanel(false);
      setCompare((c) => !c);
      return;
    }
    if (compare) return;
    if (plain && !e.shiftKey && /^[1-6]$/.test(e.key)) {
      e.preventDefault();
      act(target(), { kind: "highlight", color: Number(e.key) as 1 | 2 | 3 | 4 | 5 | 6 });
      return;
    }
    if (plain && e.key.toLowerCase() === "n") {
      e.preventDefault();
      act(target(), { kind: "note" });
      return;
    }
    if (plain && e.key.toLowerCase() === "b") {
      e.preventDefault();
      act(target(), { kind: "bookmark" });
      return;
    }
    if (plain && e.key.toLowerCase() === "s") {
      e.preventDefault();
      toggleStudy();
      return;
    }
    if (plain && e.key === "Escape" && selected !== null) {
      e.preventDefault();
      setSelected(null);
      return;
    }
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
        onSearch={openSearch}
        studyShown={studyPanel}
        onStudy={toggleStudy}
        onCompare={() => setCompare(true)}
        translationShown={settings.view !== "arabic"}
        onTranslation={() => setSettings(toggleTranslation)}
        onSettings={() => setPanel((p) => !p)}
        settingsPanel={
          panel && (
            <SettingsPanel
              settings={settings}
              onChange={setSettings}
              library={library}
              shown={shown}
              onClose={() => setPanel(false)}
            />
          )
        }
      />
      <div className="flex min-h-0 flex-1">
        <Reader
          ref={reader}
          mushaf={mushaf}
          settings={settings}
          initialRow={initialRow}
          translation={settings.view === "arabic" ? null : translation}
          marks={marks}
          study={study.index}
          selected={selected}
          onSelect={setSelected}
          onAction={onAction}
          onAnchor={onAnchor}
          onFirstPaint={onFirstPaint}
        />
        {studyPanel && (
          <StudyPanel
            mushaf={mushaf}
            study={study}
            tab={studyTab}
            onTab={setStudyTab}
            onGo={goStudy}
            onEditNote={setNoteFor}
            notify={notify}
            onClose={() => {
              setStudyPanel(false);
              reader.current?.focus();
            }}
          />
        )}
        {search && (
          <SearchPanel
            mushaf={mushaf}
            translations={library.installed}
            texts={library.texts}
            query={search.query}
            onQuery={(query) => setSearch((s) => s && { ...s, query })}
            focusKey={search.focus}
            active={found?.hit.index ?? null}
            onGo={onSearchHit}
            onClose={closeSearch}
          />
        )}
      </div>
      <Footer info={info} error={null} onCredits={() => setCredits(true)} />
      {palette && <Palette mushaf={mushaf} onGo={onHit} onClose={() => setPalette(false)} />}
      {noteFor !== null && (
        <NoteEditor
          verse={refAt(mushaf, noteFor)}
          label={refLabel(mushaf, refAt(mushaf, noteFor))}
          arabic={mushaf.verses[noteFor] ?? ""}
          translation={
            translation && settings.view !== "arabic" && translation.verses[noteFor] !== undefined
              ? { text: translation.verses[noteFor] ?? "", language: translation.language }
              : undefined
          }
          initial={study.index.notes.get(noteFor) ?? ""}
          onSave={(body) => {
            const ref = refAt(mushaf, noteFor);
            const label = refLabel(mushaf, ref);
            const had = study.index.notes.has(noteFor);
            setNoteFor(null);
            reader.current?.focus();
            study
              .setNote(ref, body)
              .then(() =>
                notify(
                  body.trim() ? `Note saved on ${label}` : had ? `Note deleted from ${label}` : "",
                ),
              )
              .catch((e: unknown) => notify(String(e)));
          }}
          onClose={() => {
            setNoteFor(null);
            reader.current?.focus();
          }}
        />
      )}
      {toast?.text && (
        <div
          role="status"
          className="pointer-events-none fixed bottom-12 left-1/2 z-30 -translate-x-1/2 rounded-full bg-ink px-4 py-1.5 text-paper text-sm shadow-lg"
        >
          {toast.text}
        </div>
      )}
      {compare && (
        <Compare
          mushaf={mushaf}
          start={current()}
          translations={library.installed}
          texts={library.texts}
          onGo={(to) => {
            setCompare(false);
            go(to);
          }}
          onClose={() => {
            setCompare(false);
            reader.current?.focus();
          }}
        />
      )}
      {credits && (
        <Credits
          info={info}
          translations={library.installed}
          onClose={() => {
            setCredits(false);
            reader.current?.focus();
          }}
        />
      )}
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
  onSearch: () => void;
  studyShown: boolean;
  onStudy: () => void;
  onCompare: () => void;
  translationShown: boolean;
  onTranslation: () => void;
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
  onSearch,
  studyShown,
  onStudy,
  onCompare,
  translationShown,
  onTranslation,
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
        <button type="button" onClick={onSearch} className={`${select} text-muted hover:text-ink`}>
          Search <kbd className="font-sans">⌘F</kbd>
        </button>
        <button
          type="button"
          onClick={onStudy}
          aria-pressed={studyShown}
          className={`${select} text-muted hover:text-ink aria-pressed:text-ink`}
          title="Bookmarks, notes, highlights, reading plan, export (S)"
        >
          Study <kbd className="font-sans">S</kbd>
        </button>
        <button
          type="button"
          onClick={onTranslation}
          aria-pressed={translationShown}
          className={`${select} text-muted hover:text-ink aria-pressed:text-ink`}
          title={translationShown ? "Hide the translation (T)" : "Show the translation (T)"}
        >
          Translation <kbd className="font-sans">T</kbd>
        </button>
        <button
          type="button"
          onClick={onCompare}
          className={`${select} hidden text-muted hover:text-ink lg:block`}
          title="Compare translations of the current verse (C)"
        >
          Compare <kbd className="font-sans">C</kbd>
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

function Footer({
  info,
  error,
  onCredits,
}: {
  info: CoreInfo | null;
  error: string | null;
  onCredits?: () => void;
}) {
  return (
    <footer className="flex shrink-0 justify-between gap-4 border-t border-rule px-5 py-2 text-muted text-xs tabular-nums">
      <span>
        {error
          ? `core error: ${error}`
          : info
            ? `quran-core ${info.version} · ${info.surahCount} surahs · ${info.ayahCount} verses`
            : "browser preview — quran-core not attached"}
      </span>
      <span>
        Quran text: Tanzil.net (CC BY 3.0) · tanzil.net
        {onCredits && (
          <>
            {" · "}
            <button type="button" onClick={onCredits} className="hover:text-ink">
              Credits
            </button>
          </>
        )}
      </span>
    </footer>
  );
}
