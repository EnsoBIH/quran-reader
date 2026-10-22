import type { HighlightColor, ReaderView, TextRange } from "@quran-reader/types";
import { Ayah, Basmala, SurahHeader } from "@quran-reader/ui";
import {
  type CSSProperties,
  memo,
  type Ref,
  type RefCallback,
  useCallback,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { flushSync } from "react-dom";
import { Heights } from "./heights.ts";
import {
  type Mushaf,
  quarterLabel,
  refAt,
  STARTS_JUZ,
  STARTS_PAGE,
  STARTS_QUARTER,
} from "./mushaf.ts";
import type { Settings } from "./settings.ts";
import { COLOR_NAMES, type StudyIndex } from "./study.ts";

// Continuous, virtualized reader (DECISIONS.md D-012). Only rows near the viewport are
// mounted, in normal flow between two spacers whose heights come from `Heights`. Rows are
// measured with a ResizeObserver; when a row above the reading position turns out taller
// or shorter than estimated, the scroll position moves by the difference, so the text being
// read never jumps.

export interface ShownTranslation {
  id: string;
  /** BCP 47 tag, for `lang` (fonts, hyphenation, screen readers). */
  language: string;
  /** All 6,236 verses, unmodified. */
  verses: string[];
}

/** Search matches in one verse: in the Arabic, and in the translation shown. */
export interface VerseMarks {
  index: number;
  arabic: TextRange[];
  translation: TextRange[];
}

/** What the toolbar under the selected verse does. */
export type VerseAction =
  | { kind: "highlight"; color: HighlightColor }
  | { kind: "note" }
  | { kind: "bookmark" }
  | { kind: "copy" };

export interface ReaderHandle {
  /** Puts `row` at the top of the viewport. */
  scrollToRow(row: number): void;
  /** Whether any part of `row` is in the viewport. */
  isVisible(row: number): boolean;
  /** Moves keyboard focus to the reader (e.g. after closing search). */
  focus(): void;
}

interface ReaderProps {
  mushaf: Mushaf;
  settings: Settings;
  /** Row shown first. */
  initialRow: number;
  /** Translation shown in the stacked and two-column views, if one is installed. */
  translation?: ShownTranslation | null;
  /** Search matches to highlight in one verse (global verse index). */
  marks?: VerseMarks | null;
  /** Highlights, notes and bookmarks by global verse index. */
  study?: StudyIndex | null;
  /** The selected verse (global index): outlined, with a toolbar. */
  selected?: number | null;
  /** A click on a verse selects it (`null`: the selected one again, which unselects). */
  onSelect?: (index: number | null) => void;
  onAction?: (index: number, action: VerseAction) => void;
  /** Called with the row at the top of the viewport whenever it changes. */
  onAnchor: (row: number) => void;
  /** Called once, after the first frame with text has been painted. */
  onFirstPaint?: () => void;
  ref?: Ref<ReaderHandle>;
}

/** Rows mounted beyond the viewport, in pixels on each side. */
const OVERSCAN = 800;
/** Average advance of one UTF-16 unit of Uthmani text (marks included) per font pixel. */
const ARABIC_ADVANCE = 0.27;
/** Average advance of one character of Latin-script translation text per font pixel. */
const LATIN_ADVANCE = 0.5;
const COLUMN_GAP = 32;

export function Reader({
  mushaf,
  settings,
  initialRow,
  translation,
  marks,
  study,
  selected,
  onSelect,
  onAction,
  onAnchor,
  onFirstPaint,
  ref,
}: ReaderProps) {
  const scroller = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLDivElement>(null);
  const heights = useRef<Heights | null>(null);
  const mounted = useRef(new Map<Element, number>());
  const anchor = useRef(-1);
  const [range, setRange] = useState<[number, number]>([0, 0]);
  const [width, setWidth] = useState(0);
  const { view, arabicSize, translationSize } = settings;

  const update = useCallback(() => {
    const el = scroller.current;
    const h = heights.current;
    if (!el || !h) return;
    const top = el.scrollTop;
    const start = h.rowAt(Math.max(0, top - OVERSCAN));
    const end = Math.min(h.count, h.rowAt(top + el.clientHeight + OVERSCAN) + 1);
    setRange((r) => (r[0] === start && r[1] === end ? r : [start, end]));
    const row = h.rowAt(top + 1);
    if (row !== anchor.current) {
      anchor.current = row;
      onAnchor(row);
    }
  }, [onAnchor]);

  // Jumps render the target first (so the spacer above it makes the page tall enough),
  // then scroll in the layout effect below. The handle flushes, so a jump from a key press
  // is on screen in the next frame.
  const pending = useRef<number | null>(null);
  const jump = useCallback((row: number) => {
    const h = heights.current;
    if (!h) return;
    pending.current = row;
    setRange([row, Math.min(row + 1, h.count)]);
  }, []);
  useLayoutEffect(() => {
    const el = scroller.current;
    const h = heights.current;
    if (pending.current === null || !el || !h) return;
    el.scrollTop = h.offset(pending.current);
    pending.current = null;
    update();
  });
  useImperativeHandle(
    ref,
    () => ({
      scrollToRow: (row) => flushSync(() => jump(row)),
      isVisible: (row) => {
        const el = scroller.current;
        const h = heights.current;
        if (!el || !h) return false;
        return h.offset(row + 1) > el.scrollTop && h.offset(row) < el.scrollTop + el.clientHeight;
      },
      focus: () => scroller.current?.focus({ preventScroll: true }),
    }),
    [jump],
  );

  // Measured sizes. Runs after layout and before paint, so corrections are never visible.
  // The row at the top keeps its distance from the viewport edge. It is found the same way
  // as in `update` (one pixel in): offsets are fractional and the browser rounds scrollTop,
  // so a jump to row r can leave scrollTop a fraction above r's offset, where a plain
  // rowAt(scrollTop) says r − 1 — and r − 1's correction would then push r off its place.
  const [observer] = useState(
    () =>
      new ResizeObserver((entries) => {
        const el = scroller.current;
        const h = heights.current;
        if (!el || !h) return;
        const top = el.scrollTop;
        const reading = h.rowAt(top + 1);
        const within = top - h.offset(reading);
        let changed = false;
        for (const entry of entries) {
          const row = mounted.current.get(entry.target);
          const size = entry.borderBoxSize[0]?.blockSize;
          if (row === undefined || size === undefined || size === 0) continue;
          if (h.set(row, size) !== 0 && row < reading) changed = true;
        }
        if (changed) el.scrollTop = h.offset(reading) + within;
      }),
  );
  useEffect(() => () => observer.disconnect(), [observer]);
  const observe = useCallback<RefCallback<HTMLDivElement>>(
    (el) => {
      if (!el) return;
      mounted.current.set(el, Number(el.dataset.row));
      observer.observe(el);
      return () => {
        observer.unobserve(el);
        mounted.current.delete(el);
      };
    },
    [observer],
  );

  // Content width, for estimates.
  useLayoutEffect(() => {
    const el = content.current;
    if (!el) return;
    const measure = () => setWidth(Math.round(el.clientWidth));
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // (Re-)estimate all rows when anything that changes their height changes, keeping the
  // row being read where it is.
  const first = useRef(true);
  useLayoutEffect(() => {
    const el = scroller.current;
    if (!el || width === 0) return;
    const px = Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
    const estimate = estimator(
      mushaf,
      view,
      arabicSize * px,
      translationSize * px,
      width,
      translation?.verses,
    );
    const h = heights.current;
    if (!h) {
      heights.current = new Heights(mushaf.rows.length, estimate);
      jump(initialRow);
      return;
    }
    const row = h.rowAt(el.scrollTop + 1);
    const within = (el.scrollTop - h.offset(row)) / Math.max(h.size(row), 1);
    h.reset(estimate);
    for (const [node, r] of mounted.current) h.set(r, node.getBoundingClientRect().height);
    el.scrollTop = h.offset(row) + within * h.size(row);
    update();
  }, [mushaf, view, arabicSize, translationSize, width, translation, initialRow, jump, update]);

  useLayoutEffect(() => {
    if (!first.current || range[1] === 0) return;
    first.current = false;
    scroller.current?.focus({ preventScroll: true });
    // A task queued from the next animation frame runs right after that frame is painted.
    requestAnimationFrame(() => setTimeout(() => onFirstPaint?.()));
  }, [range, onFirstPaint]);

  const h = heights.current;
  const [start, end] = range;
  const rows: number[] = [];
  for (let row = start; row < end; row++) rows.push(row);
  const style = {
    "--reader-arabic-size": `${arabicSize}rem`,
    "--reader-translation-size": `${translationSize}rem`,
    maxWidth: view === "columns" ? "calc(var(--reader-measure) * 1.75)" : "var(--reader-measure)",
  } as CSSProperties;

  return (
    <div
      ref={scroller}
      tabIndex={-1}
      onScroll={() => flushSync(update)}
      className="min-h-0 min-w-0 flex-1 overflow-y-auto overscroll-contain outline-none"
    >
      <div ref={content} className="mx-auto w-full px-6 pb-[40vh]" style={style}>
        <div style={{ height: h ? h.offset(start) : 0 }} />
        {rows.map((row) => {
          const r = mushaf.rows[row] ?? 0;
          return (
            <Row
              key={row}
              mushaf={mushaf}
              row={row}
              view={view}
              observe={observe}
              translation={translation}
              marks={marks && r === marks.index ? marks : undefined}
              highlight={study?.highlights.get(r)}
              note={study?.notes.get(r)}
              bookmark={study?.bookmarks.get(r)}
              selected={selected === r}
              onSelect={onSelect}
              onAction={onAction}
            />
          );
        })}
        <div style={{ height: h ? h.total() - h.offset(end) : 0 }} />
      </div>
    </div>
  );
}

interface RowProps {
  mushaf: Mushaf;
  row: number;
  view: ReaderView;
  observe: RefCallback<HTMLDivElement>;
  translation?: ShownTranslation | null;
  marks?: VerseMarks;
  highlight?: HighlightColor;
  note?: string;
  /** Label, or "" without one; undefined when not bookmarked. */
  bookmark?: string;
  selected: boolean;
  onSelect?: (index: number | null) => void;
  onAction?: (index: number, action: VerseAction) => void;
}

const Row = memo(function Row({
  mushaf,
  row,
  view,
  observe,
  translation,
  marks,
  highlight,
  note,
  bookmark,
  selected,
  onSelect,
  onAction,
}: RowProps) {
  const r = mushaf.rows[row] ?? 0;
  if (r < 0) {
    const surah = mushaf.surahs[-r - 1];
    if (!surah) return null;
    return (
      <div ref={observe} data-row={row} className="pt-10">
        <SurahHeader surah={surah} />
        {surah.bismillah !== undefined && <Basmala text={surah.bismillah} />}
      </div>
    );
  }
  const starts = mushaf.starts[r] ?? 0;
  const quarter = mushaf.quarterOf[r] ?? 1;
  const sajda = mushaf.sajdas.get(r);
  const band =
    starts & STARTS_JUZ
      ? `Juz ${mushaf.juzOf[r]} · ${quarterLabel(quarter)}`
      : starts & STARTS_QUARTER
        ? quarterLabel(quarter)
        : null;
  const meta = [
    starts & STARTS_PAGE ? `Page ${mushaf.pageOf[r]}` : null,
    sajda ? `۩ Sajda (${sajda})` : null,
  ].filter((m) => m !== null);
  const bookmarkLine = bookmark !== undefined && (
    <span className="text-accent">Bookmark{bookmark !== "" && `: ${bookmark}`}</span>
  );
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents lint/a11y/noStaticElementInteractions: the keyboard acts on the reading verse (1–6, N, B, ⌘C); a click only picks another one
    <div
      ref={observe}
      data-row={row}
      data-selected={selected || undefined}
      className="border-b border-rule"
      onClick={(e) => {
        // Selecting text, or clicking a control inside the row, doesn't change the selection.
        if (window.getSelection()?.isCollapsed === false) return;
        if ((e.target as HTMLElement).closest("button, a, input, textarea")) return;
        onSelect?.(selected ? null : r);
      }}
    >
      {band && (
        <p className="flex items-center gap-3 pt-5 text-accent text-xs uppercase tracking-[0.15em]">
          <span aria-hidden="true" className="font-quran text-base normal-case">
            ۞
          </span>
          {band}
          <span className="h-px flex-1 bg-rule" />
        </p>
      )}
      <Ayah
        verseRef={refAt(mushaf, r)}
        text={mushaf.verses[r] ?? ""}
        view={view}
        translation={translation?.verses[r]}
        translationLang={translation?.language}
        meta={
          meta.length > 0 || bookmarkLine ? (
            <>
              {meta.join(" · ")}
              {meta.length > 0 && bookmarkLine && " · "}
              {bookmarkLine}
            </>
          ) : undefined
        }
        marks={marks?.arabic}
        translationMarks={marks?.translation}
        highlight={highlight}
      >
        {(note !== undefined || selected) && (
          <>
            {note !== undefined && (
              <button
                type="button"
                onClick={() => onAction?.(r, { kind: "note" })}
                className="block w-full whitespace-pre-wrap border-accent/60 border-l-2 py-0.5 pl-3 text-left text-ink/80 text-sm hover:text-ink"
                title="Edit the note (N)"
              >
                {note}
              </button>
            )}
            {selected && (
              <Toolbar
                index={r}
                highlight={highlight}
                bookmarked={bookmark !== undefined}
                hasNote={note !== undefined}
                onAction={onAction}
              />
            )}
          </>
        )}
      </Ayah>
    </div>
  );
});

const tool = "rounded px-2 py-1 text-muted text-xs hover:bg-rule/60 hover:text-ink";

/** Actions for the selected verse, with their keys. */
function Toolbar({
  index,
  highlight,
  bookmarked,
  hasNote,
  onAction,
}: {
  index: number;
  highlight?: HighlightColor;
  bookmarked: boolean;
  hasNote: boolean;
  onAction?: (index: number, action: VerseAction) => void;
}) {
  return (
    <div role="toolbar" aria-label="Verse" className="mt-2 flex flex-wrap items-center gap-1">
      {([1, 2, 3, 4, 5, 6] as const).map((c) => (
        <button
          key={c}
          type="button"
          aria-pressed={highlight === c}
          aria-label={`Highlight ${COLOR_NAMES[c - 1]} (${c})`}
          title={`${highlight === c ? "Remove the highlight" : `Highlight ${COLOR_NAMES[c - 1]}`} (${c})`}
          onClick={() => onAction?.(index, { kind: "highlight", color: c })}
          className="size-5 rounded-full border border-rule aria-pressed:ring-2 aria-pressed:ring-accent"
          style={{ backgroundColor: `var(--color-mark-${c})` }}
        />
      ))}
      <span className="mx-1 h-4 w-px bg-rule" />
      <button type="button" className={tool} onClick={() => onAction?.(index, { kind: "note" })}>
        {hasNote ? "Edit note" : "Note"} <kbd className="font-sans">N</kbd>
      </button>
      <button
        type="button"
        className={tool}
        onClick={() => onAction?.(index, { kind: "bookmark" })}
      >
        {bookmarked ? "Remove bookmark" : "Bookmark"} <kbd className="font-sans">B</kbd>
      </button>
      <button type="button" className={tool} onClick={() => onAction?.(index, { kind: "copy" })}>
        Copy <kbd className="font-sans">⌘C</kbd>
      </button>
    </div>
  );
}

/** Height estimates in pixels; measuring replaces them as rows are rendered. */
function estimator(
  m: Mushaf,
  view: ReaderView,
  arabicPx: number,
  translationPx: number,
  width: number,
  translations?: string[],
): (row: number) => number {
  const textWidth = Math.max(width - 48, 100);
  const arabicWidth = view === "columns" ? (textWidth - COLUMN_GAP) / 2 : textWidth;
  const line = arabicPx * 2.2;
  const translationLine = translationPx * 1.625;
  // The reference ("2:255 ") comes first in the slot.
  const translationLines = (r: number) =>
    Math.max(
      1,
      Math.ceil(
        (((translations?.[r]?.length ?? 0) + 8) * LATIN_ADVANCE * translationPx) / arabicWidth,
      ),
    );
  return (row) => {
    const r = m.rows[row] ?? 0;
    if (r < 0) {
      const basmala = m.surahs[-r - 1]?.bismillah !== undefined ? 48 + line : 0;
      return 40 + 194 + basmala;
    }
    const length = (m.verses[r]?.length ?? 0) + 3;
    const lines = Math.max(1, Math.ceil((length * ARABIC_ADVANCE * arabicPx) / arabicWidth));
    let size = 41 + lines * line;
    if (view === "stacked") size += 12 + translationLines(r) * translationLine;
    if (view === "columns") size = Math.max(size, 41 + translationLines(r) * translationLine);
    const starts = m.starts[r] ?? 0;
    if (starts & (STARTS_JUZ | STARTS_QUARTER)) size += 36;
    if (starts & STARTS_PAGE || m.sajdas.has(r)) size += 28;
    return size;
  };
}
