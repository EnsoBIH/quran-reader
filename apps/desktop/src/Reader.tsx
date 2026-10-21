import type { ReaderView, TextRange } from "@quran-reader/types";
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

// Continuous, virtualized reader (DECISIONS.md D-012). Only rows near the viewport are
// mounted, in normal flow between two spacers whose heights come from `Heights`. Rows are
// measured with a ResizeObserver; when a row above the reading position turns out taller
// or shorter than estimated, the scroll position moves by the difference, so the text being
// read never jumps.

export interface ReaderHandle {
  /** Puts `row` at the top of the viewport. */
  scrollToRow(row: number): void;
  /** Moves keyboard focus to the reader (e.g. after closing search). */
  focus(): void;
}

interface ReaderProps {
  mushaf: Mushaf;
  settings: Settings;
  /** Row shown first. */
  initialRow: number;
  /** Search matches to highlight in one verse (global verse index). */
  marks?: { index: number; ranges: TextRange[] } | null;
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
const COLUMN_GAP = 32;

export function Reader({
  mushaf,
  settings,
  initialRow,
  marks,
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
      focus: () => scroller.current?.focus({ preventScroll: true }),
    }),
    [jump],
  );

  // Measured sizes. Runs after layout and before paint, so corrections are never visible.
  const [observer] = useState(
    () =>
      new ResizeObserver((entries) => {
        const el = scroller.current;
        const h = heights.current;
        if (!el || !h) return;
        const reading = h.rowAt(el.scrollTop);
        let shift = 0;
        for (const entry of entries) {
          const row = mounted.current.get(entry.target);
          const size = entry.borderBoxSize[0]?.blockSize;
          if (row === undefined || size === undefined || size === 0) continue;
          const delta = h.set(row, size);
          if (row < reading) shift += delta;
        }
        if (shift !== 0) el.scrollTop += shift;
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
    const estimate = estimator(mushaf, view, arabicSize * px, translationSize * px, width);
    const h = heights.current;
    if (!h) {
      heights.current = new Heights(mushaf.rows.length, estimate);
      jump(initialRow);
      return;
    }
    const row = h.rowAt(el.scrollTop);
    const within = (el.scrollTop - h.offset(row)) / Math.max(h.size(row), 1);
    h.reset(estimate);
    for (const [node, r] of mounted.current) h.set(r, node.getBoundingClientRect().height);
    el.scrollTop = h.offset(row) + within * h.size(row);
    update();
  }, [mushaf, view, arabicSize, translationSize, width, initialRow, jump, update]);

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
        {rows.map((row) => (
          <Row
            key={row}
            mushaf={mushaf}
            row={row}
            view={view}
            observe={observe}
            marks={marks && mushaf.rows[row] === marks.index ? marks.ranges : undefined}
          />
        ))}
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
  marks?: TextRange[];
}

const Row = memo(function Row({ mushaf, row, view, observe, marks }: RowProps) {
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
  return (
    <div ref={observe} data-row={row} className="border-b border-rule">
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
        meta={meta.length > 0 ? meta.join(" · ") : undefined}
        marks={marks}
      />
    </div>
  );
});

/** Height estimates in pixels; measuring replaces them as rows are rendered. */
function estimator(
  m: Mushaf,
  view: ReaderView,
  arabicPx: number,
  translationPx: number,
  width: number,
): (row: number) => number {
  const textWidth = Math.max(width - 48, 100);
  const arabicWidth = view === "columns" ? (textWidth - COLUMN_GAP) / 2 : textWidth;
  const line = arabicPx * 2.2;
  const translation = translationPx * 1.625;
  return (row) => {
    const r = m.rows[row] ?? 0;
    if (r < 0) {
      const basmala = m.surahs[-r - 1]?.bismillah !== undefined ? 48 + line : 0;
      return 40 + 194 + basmala;
    }
    const length = (m.verses[r]?.length ?? 0) + 3;
    const lines = Math.max(1, Math.ceil((length * ARABIC_ADVANCE * arabicPx) / arabicWidth));
    let size = 41 + lines * line;
    if (view === "stacked") size += 12 + translation;
    const starts = m.starts[r] ?? 0;
    if (starts & (STARTS_JUZ | STARTS_QUARTER)) size += 36;
    if (starts & STARTS_PAGE || m.sajdas.has(r)) size += 28;
    return size;
  };
}
