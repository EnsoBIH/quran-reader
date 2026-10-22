import type {
  Changes,
  HighlightColor,
  ImportMode,
  ImportPreview,
  PlanStatus,
  Portion,
  VerseRef,
} from "@quran-reader/types";
import { useEffect, useRef, useState } from "react";
import * as core from "./core.ts";
import { indexOf, type Mushaf } from "./mushaf.ts";
import {
  COLOR_NAMES,
  localDate,
  localDateTime,
  portionLabel,
  portionRange,
  refLabel,
  type Study,
} from "./study.ts";

// S: the study panel beside the reader — bookmarks, notes, highlights with jump-to, the
// khatm reading plan, and export / import.

export type StudyTab = "bookmarks" | "notes" | "highlights" | "plan" | "data";

const TABS: { id: StudyTab; label: string }[] = [
  { id: "bookmarks", label: "Bookmarks" },
  { id: "notes", label: "Notes" },
  { id: "highlights", label: "Highlights" },
  { id: "plan", label: "Plan" },
  { id: "data", label: "Export" },
];

interface StudyPanelProps {
  mushaf: Mushaf;
  study: Study;
  tab: StudyTab;
  onTab: (tab: StudyTab) => void;
  onGo: (ref: VerseRef) => void;
  onEditNote: (index: number) => void;
  notify: (message: string) => void;
  onClose: () => void;
}

const small = "rounded px-2 py-1 text-muted text-xs hover:bg-rule/60 hover:text-ink";
const field =
  "rounded border border-rule bg-paper px-2 py-1 text-ink outline-none focus:border-accent";

export function StudyPanel({
  mushaf,
  study,
  tab,
  onTab,
  onGo,
  onEditNote,
  notify,
  onClose,
}: StudyPanelProps) {
  const { data } = study;
  const counts: Record<StudyTab, number | null> = {
    bookmarks: data.bookmarks.length,
    notes: data.notes.length,
    highlights: data.highlights.length,
    plan: null,
    data: null,
  };
  return (
    <aside
      aria-label="Study"
      className="flex w-[26rem] max-w-[45vw] shrink-0 flex-col border-l border-rule bg-paper"
      onKeyDown={(e) => {
        if (e.key === "Escape" && !(e.target as HTMLElement).closest("input, textarea")) {
          e.preventDefault();
          onClose();
        }
      }}
    >
      <div className="flex items-center gap-1 border-b border-rule px-2 py-1.5">
        <div role="tablist" aria-label="Study" className="flex flex-1 flex-wrap gap-0.5">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={tab === t.id}
              onClick={() => onTab(t.id)}
              className="rounded px-2 py-1 text-muted text-sm hover:text-ink aria-selected:bg-rule/60 aria-selected:text-ink"
            >
              {t.label}
              {counts[t.id] ? (
                <span className="ml-1 text-muted text-xs tabular-nums">{counts[t.id]}</span>
              ) : null}
            </button>
          ))}
        </div>
        <button
          type="button"
          onClick={onClose}
          className="rounded px-1.5 text-muted hover:text-ink"
          aria-label="Close study panel"
          title="Close (S)"
        >
          ×
        </button>
      </div>
      {study.error && (
        <p className="border-b border-rule px-4 py-2 text-red-700 text-sm dark:text-red-400">
          {study.error}
        </p>
      )}
      <div className="min-h-0 flex-1 overflow-y-auto">
        {tab === "bookmarks" && <Bookmarks mushaf={mushaf} study={study} onGo={onGo} />}
        {tab === "notes" && (
          <Notes mushaf={mushaf} study={study} onGo={onGo} onEditNote={onEditNote} />
        )}
        {tab === "highlights" && <Highlights mushaf={mushaf} study={study} onGo={onGo} />}
        {tab === "plan" && <PlanView mushaf={mushaf} onGo={onGo} notify={notify} />}
        {tab === "data" && <ExportImport mushaf={mushaf} study={study} notify={notify} />}
      </div>
    </aside>
  );
}

function Empty({ children }: { children: React.ReactNode }) {
  return <p className="px-4 py-3 text-muted text-sm">{children}</p>;
}

function VerseHead({ mushaf, verse, onGo }: { mushaf: Mushaf; verse: VerseRef; onGo: () => void }) {
  return (
    <button type="button" onClick={onGo} className="text-left text-ink text-sm hover:text-accent">
      {refLabel(mushaf, verse)}
    </button>
  );
}

/** The verse's Arabic on one line, cut by CSS (the text itself stays whole). */
function Snippet({ mushaf, verse }: { mushaf: Mushaf; verse: VerseRef }) {
  return (
    <span dir="rtl" lang="ar" className="block truncate font-quran text-ink/80 text-lg leading-[2]">
      {mushaf.verses[indexOf(mushaf, verse)]}
    </span>
  );
}

function Bookmarks({
  mushaf,
  study,
  onGo,
}: {
  mushaf: Mushaf;
  study: Study;
  onGo: (r: VerseRef) => void;
}) {
  if (study.data.bookmarks.length === 0) {
    return <Empty>No bookmarks yet. Press B on a verse to add one.</Empty>;
  }
  return (
    <ul>
      {study.data.bookmarks.map((b) => (
        <li
          key={`${b.ref.surah}:${b.ref.ayah}`}
          className="flex flex-col gap-1 border-b border-rule px-4 py-2.5"
        >
          <span className="flex items-center gap-2">
            <VerseHead mushaf={mushaf} verse={b.ref} onGo={() => onGo(b.ref)} />
            <button
              type="button"
              className={`${small} ml-auto`}
              aria-label={`Remove the bookmark on ${b.ref.surah}:${b.ref.ayah}`}
              onClick={() => study.setBookmark(b.ref, false).catch(() => {})}
            >
              Remove
            </button>
          </span>
          <Label
            value={b.label ?? ""}
            onSave={(label) => study.setBookmark(b.ref, true, label || null).catch(() => {})}
          />
          <Snippet mushaf={mushaf} verse={b.ref} />
        </li>
      ))}
    </ul>
  );
}

/** A bookmark label, saved on Enter or when the field loses focus. */
function Label({ value, onSave }: { value: string; onSave: (label: string) => void }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const save = () => {
    if (text.trim() !== value) onSave(text.trim());
  };
  return (
    <input
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={save}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          e.currentTarget.blur();
        } else if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          setText(value);
          // Blur after the reset, so the old label is not saved again.
          setTimeout(() => (e.target as HTMLInputElement).blur());
        }
      }}
      placeholder="Add a label…"
      aria-label="Bookmark label"
      maxLength={200}
      className="w-full bg-transparent text-ink/85 text-sm outline-none placeholder:text-muted focus:border-accent"
    />
  );
}

function Notes({
  mushaf,
  study,
  onGo,
  onEditNote,
}: {
  mushaf: Mushaf;
  study: Study;
  onGo: (r: VerseRef) => void;
  onEditNote: (index: number) => void;
}) {
  if (study.data.notes.length === 0) {
    return <Empty>No notes yet. Press N on a verse to write one.</Empty>;
  }
  return (
    <ul>
      {study.data.notes.map((n) => (
        <li
          key={`${n.ref.surah}:${n.ref.ayah}`}
          className="flex flex-col gap-1 border-b border-rule px-4 py-2.5"
        >
          <span className="flex items-center gap-2">
            <VerseHead mushaf={mushaf} verse={n.ref} onGo={() => onGo(n.ref)} />
            <span className="ml-auto text-muted text-xs tabular-nums">
              {n.updatedAt.slice(0, 10)}
            </span>
            <button
              type="button"
              className={small}
              onClick={() => onEditNote(indexOf(mushaf, n.ref))}
            >
              Edit
            </button>
          </span>
          <p className="line-clamp-4 whitespace-pre-wrap text-ink/85 text-sm">{n.body}</p>
        </li>
      ))}
    </ul>
  );
}

function Highlights({
  mushaf,
  study,
  onGo,
}: {
  mushaf: Mushaf;
  study: Study;
  onGo: (r: VerseRef) => void;
}) {
  const [color, setColor] = useState<HighlightColor | null>(null);
  if (study.data.highlights.length === 0) {
    return <Empty>No highlights yet. Press 1–6 on a verse to highlight it.</Empty>;
  }
  const shown = study.data.highlights.filter((h) => color === null || h.color === color);
  return (
    <>
      <div className="flex items-center gap-1.5 border-b border-rule px-4 py-2">
        <button
          type="button"
          aria-pressed={color === null}
          onClick={() => setColor(null)}
          className={`${small} aria-pressed:text-ink`}
        >
          All
        </button>
        {([1, 2, 3, 4, 5, 6] as const).map((c) => (
          <button
            key={c}
            type="button"
            aria-pressed={color === c}
            aria-label={`Only ${COLOR_NAMES[c - 1]}`}
            title={COLOR_NAMES[c - 1]}
            onClick={() => setColor(color === c ? null : c)}
            className="size-4 rounded-full border border-rule aria-pressed:ring-2 aria-pressed:ring-accent"
            style={{ backgroundColor: `var(--color-mark-${c})` }}
          />
        ))}
      </div>
      <ul>
        {shown.map((h) => (
          <li
            key={`${h.ref.surah}:${h.ref.ayah}`}
            className="flex flex-col gap-0.5 border-b border-rule px-4 py-2"
          >
            <span className="flex items-center gap-2">
              <span
                className="size-3 shrink-0 rounded-full"
                style={{ backgroundColor: `var(--color-mark-${h.color})` }}
              />
              <VerseHead mushaf={mushaf} verse={h.ref} onGo={() => onGo(h.ref)} />
              <button
                type="button"
                className={`${small} ml-auto`}
                onClick={() => study.setHighlight(h.ref, null).catch(() => {})}
              >
                Remove
              </button>
            </span>
            <Snippet mushaf={mushaf} verse={h.ref} />
          </li>
        ))}
      </ul>
    </>
  );
}

// ---------------------------------------------------------------------------------------
// Reading plan

const PRESETS = [
  { days: 30, label: "30 days · one juz a day" },
  { days: 60, label: "60 days · one hizb a day" },
  { days: 7, label: "7 days" },
  { days: 365, label: "One year" },
];

function PlanView({
  mushaf,
  onGo,
  notify,
}: {
  mushaf: Mushaf;
  onGo: (r: VerseRef) => void;
  notify: (m: string) => void;
}) {
  const [status, setStatus] = useState<PlanStatus | null | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);
  const today = localDate(new Date());

  useEffect(() => {
    core.planStatus(today).then(setStatus, (e: unknown) => setError(String(e)));
  }, [today]);

  const run = (p: Promise<PlanStatus | null>) =>
    p.then(
      (s) => {
        setStatus(s);
        setError(null);
      },
      (e: unknown) => setError(String(e)),
    );

  if (status === undefined && error === null) return <Empty>Loading…</Empty>;
  return (
    <div className="flex flex-col gap-4 px-4 py-3 text-sm">
      {error && <p className="text-red-700 dark:text-red-400">{error}</p>}
      {status ? (
        <ActivePlan
          mushaf={mushaf}
          status={status}
          onGo={onGo}
          onRead={(day, read) => run(core.setPlanDay(day, read, today))}
          onStop={() =>
            core.stopPlan().then(
              () => {
                setStatus(null);
                notify("Reading plan ended");
              },
              (e: unknown) => setError(String(e)),
            )
          }
        />
      ) : (
        status === null && (
          <NewPlan
            today={today}
            onStart={(days, start) => run(core.startPlan(days, start, today))}
          />
        )
      )}
    </div>
  );
}

function NewPlan({
  today,
  onStart,
}: {
  today: string;
  onStart: (days: number, start: string) => void;
}) {
  const [days, setDays] = useState(30);
  const [start, setStart] = useState(today);
  const valid =
    Number.isInteger(days) && days >= 1 && days <= 604 && /^\d{4}-\d{2}-\d{2}$/.test(start);
  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (valid) onStart(days, start);
      }}
    >
      <p className="text-muted">
        Read the whole Quran in a set number of days. 30 days is one juz a day, 60 days one hizb a
        day; other lengths are cut into equal parts by hizb quarter (or by page beyond 240 days).
      </p>
      <div className="flex flex-wrap gap-1.5">
        {PRESETS.map((p) => (
          <button
            key={p.days}
            type="button"
            aria-pressed={days === p.days}
            onClick={() => setDays(p.days)}
            className="rounded border border-rule px-2 py-1 text-muted hover:text-ink aria-pressed:border-accent aria-pressed:text-ink"
          >
            {p.label}
          </button>
        ))}
      </div>
      <label className="flex items-center justify-between gap-3">
        Days
        <input
          type="number"
          min={1}
          max={604}
          value={Number.isNaN(days) ? "" : days}
          onChange={(e) => setDays(e.target.valueAsNumber)}
          className={`${field} w-24 tabular-nums`}
        />
      </label>
      <label className="flex items-center justify-between gap-3">
        First day
        <input
          type="date"
          value={start}
          onChange={(e) => setStart(e.target.value)}
          className={field}
        />
      </label>
      <button
        type="submit"
        disabled={!valid}
        className="self-start rounded border border-rule px-3 py-1 text-ink hover:border-accent disabled:opacity-40"
      >
        Start the plan
      </button>
    </form>
  );
}

function ActivePlan({
  mushaf,
  status,
  onGo,
  onRead,
  onStop,
}: {
  mushaf: Mushaf;
  status: PlanStatus;
  onGo: (r: VerseRef) => void;
  onRead: (day: number, read: boolean) => void;
  onStop: () => void;
}) {
  const { plan, portions, today } = status;
  const read = new Set(plan.read);
  const [confirm, setConfirm] = useState(false);
  const list = useRef<HTMLUListElement>(null);
  const current = Math.min(Math.max(today, 1), plan.days);
  const firstUnread = portions.find((p) => !read.has(p.day));
  const todays = portions[current - 1];

  useEffect(() => {
    list.current?.querySelector(`[data-day="${current}"]`)?.scrollIntoView({ block: "center" });
  }, [current]);

  const when =
    today < 1
      ? `Starts on ${plan.start}`
      : today > plan.days
        ? `Ended ${today - plan.days} ${today - plan.days === 1 ? "day" : "days"} ago`
        : `Day ${today} of ${plan.days}`;
  const behind = firstUnread && today <= plan.days && firstUnread.day < today ? firstUnread : null;

  return (
    <>
      <div className="flex flex-col gap-1">
        <p className="flex items-baseline justify-between">
          <span className="text-ink">{plan.days}-day khatm</span>
          <span className="text-muted text-xs">from {plan.start}</span>
        </p>
        <div className="h-1.5 overflow-hidden rounded bg-rule">
          <div
            className="h-full bg-accent"
            style={{ width: `${(read.size / plan.days) * 100}%` }}
          />
        </div>
        <p className="text-muted text-xs tabular-nums">
          {read.size} of {plan.days} days read · {when}
          {read.size === plan.days && " · khatm complete"}
        </p>
      </div>
      {todays && (
        <PortionCard
          title={
            today >= 1 && today <= plan.days ? "Today" : today < 1 ? "Day 1" : `Day ${plan.days}`
          }
          mushaf={mushaf}
          days={plan.days}
          portion={todays}
          read={read.has(todays.day)}
          onGo={onGo}
          onRead={onRead}
        />
      )}
      {behind && (
        <PortionCard
          title="Catch up"
          mushaf={mushaf}
          days={plan.days}
          portion={behind}
          read={false}
          onGo={onGo}
          onRead={onRead}
        />
      )}
      <ul ref={list} className="-mx-4 max-h-80 overflow-y-auto border-y border-rule">
        {portions.map((p) => (
          <li
            key={p.day}
            data-day={p.day}
            className={`flex items-center gap-2 px-4 py-1 ${p.day === today ? "bg-rule/50" : ""}`}
          >
            <input
              type="checkbox"
              checked={read.has(p.day)}
              onChange={(e) => onRead(p.day, e.target.checked)}
              aria-label={`Day ${p.day} read`}
              className="accent-accent"
            />
            <span className="w-14 shrink-0 text-muted text-xs tabular-nums">Day {p.day}</span>
            <button
              type="button"
              onClick={() => onGo(p.from)}
              className="min-w-0 truncate text-left text-ink/85 hover:text-accent"
              title={portionRange(mushaf, p)}
            >
              {portionLabel(mushaf, plan.days, p)}
            </button>
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-2">
        {confirm ? (
          <>
            <span className="text-muted">End the plan and forget its progress?</span>
            <button type="button" className={small} onClick={onStop}>
              End it
            </button>
            <button type="button" className={small} onClick={() => setConfirm(false)}>
              Keep it
            </button>
          </>
        ) : (
          <button type="button" className={small} onClick={() => setConfirm(true)}>
            End or restart the plan…
          </button>
        )}
      </div>
    </>
  );
}

function PortionCard({
  title,
  mushaf,
  days,
  portion,
  read,
  onGo,
  onRead,
}: {
  title: string;
  mushaf: Mushaf;
  days: number;
  portion: Portion;
  read: boolean;
  onGo: (r: VerseRef) => void;
  onRead: (day: number, read: boolean) => void;
}) {
  const label = portionLabel(mushaf, days, portion);
  const range = portionRange(mushaf, portion);
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-rule p-3">
      <p className="flex items-baseline justify-between">
        <span className="text-muted text-xs uppercase tracking-wider">{title}</span>
        <span className="text-muted text-xs">Day {portion.day}</span>
      </p>
      <p className="text-ink">{label}</p>
      {label !== range && <p className="text-muted text-xs">{range}</p>}
      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => onGo(portion.from)}
          className="rounded border border-rule px-3 py-1 text-ink hover:border-accent"
        >
          Read
        </button>
        <button
          type="button"
          aria-pressed={read}
          onClick={() => onRead(portion.day, !read)}
          className="rounded border border-rule px-3 py-1 text-muted hover:text-ink aria-pressed:border-accent aria-pressed:text-accent"
        >
          {read ? "✓ Read" : "Mark as read"}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------------------
// Export and import

const KINDS = [
  ["highlights", "Highlights"],
  ["notes", "Notes"],
  ["bookmarks", "Bookmarks"],
] as const;

function ExportImport({
  mushaf,
  study,
  notify,
}: {
  mushaf: Mushaf;
  study: Study;
  notify: (m: string) => void;
}) {
  const [saved, setSaved] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [file, setFile] = useState<{ name: string; text: string } | null>(null);
  const [mode, setMode] = useState<ImportMode>("merge");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!file) return;
    let stale = false;
    setPreview(null);
    core.previewImport(file.text, mode).then(
      (p) => {
        if (!stale) {
          setPreview(p);
          setError(null);
        }
      },
      (e: unknown) => {
        if (!stale) {
          setError(`${file.name}: ${String(e)}`);
          setFile(null);
        }
      },
    );
    return () => {
      stale = true;
    };
  }, [file, mode]);

  const exportAs = (format: "markdown" | "json") =>
    core.exportStudy(format, localDateTime(new Date())).then(
      (path) => {
        setSaved(path);
        setError(null);
      },
      (e: unknown) => setError(String(e)),
    );

  const apply = () => {
    if (!file) return;
    setBusy(true);
    core
      .applyImport(file.text, mode)
      .then(
        (data) => {
          study.replace(data);
          notify(`Imported ${file.name}`);
          setFile(null);
          setPreview(null);
          setError(null);
        },
        (e: unknown) => setError(String(e)),
      )
      .finally(() => setBusy(false));
  };

  const { data } = study;
  const empty =
    data.highlights.length + data.notes.length + data.bookmarks.length === 0 && !data.plan;

  return (
    <div className="flex flex-col gap-5 px-4 py-3 text-sm">
      <section className="flex flex-col gap-2">
        <h3 className="text-ink">Export</h3>
        <p className="text-muted">
          Saves your highlights, notes, bookmarks and reading plan to the Downloads folder. Markdown
          is for reading (with the Arabic of each verse); JSON keeps everything, dates included.
          Both can be imported again.
        </p>
        <div className="flex gap-2">
          <button
            type="button"
            disabled={empty}
            onClick={() => exportAs("markdown")}
            className="rounded border border-rule px-3 py-1 text-ink hover:border-accent disabled:opacity-40"
          >
            Export Markdown
          </button>
          <button
            type="button"
            disabled={empty}
            onClick={() => exportAs("json")}
            className="rounded border border-rule px-3 py-1 text-ink hover:border-accent disabled:opacity-40"
          >
            Export JSON
          </button>
        </div>
        {saved && <p className="break-all text-muted text-xs">Saved to {saved}</p>}
      </section>
      <section className="flex flex-col gap-2">
        <h3 className="text-ink">Import</h3>
        <p className="text-muted">
          Reads a Markdown or JSON export. You see what would change before anything is saved.
        </p>
        <input
          ref={input}
          type="file"
          accept=".json,.md,.markdown,.txt,application/json,text/markdown,text/plain"
          className="hidden"
          onChange={async (e) => {
            const f = e.target.files?.[0];
            e.target.value = "";
            if (!f) return;
            setMode("merge");
            setFile({ name: f.name, text: await f.text() });
          }}
        />
        <button
          type="button"
          onClick={() => input.current?.click()}
          className="self-start rounded border border-rule px-3 py-1 text-ink hover:border-accent"
        >
          Choose a file…
        </button>
        {error && <p className="text-red-700 dark:text-red-400">{error}</p>}
        {file && (
          <div className="flex flex-col gap-3 rounded-lg border border-rule p-3">
            <p className="break-all text-ink">{file.name}</p>
            <fieldset className="flex flex-col gap-1">
              <legend className="sr-only">How to import</legend>
              <label className="flex items-start gap-2">
                <input
                  type="radio"
                  name="mode"
                  checked={mode === "merge"}
                  onChange={() => setMode("merge")}
                  className="mt-1 accent-accent"
                />
                <span>
                  Merge{" "}
                  <span className="text-muted">
                    — keep yours, add the file's; on the same verse the file wins
                  </span>
                </span>
              </label>
              <label className="flex items-start gap-2">
                <input
                  type="radio"
                  name="mode"
                  checked={mode === "replace"}
                  onChange={() => setMode("replace")}
                  className="mt-1 accent-accent"
                />
                <span>
                  Replace{" "}
                  <span className="text-muted">— make everything exactly as in the file</span>
                </span>
              </label>
            </fieldset>
            {preview ? (
              <PreviewTable mushaf={mushaf} preview={preview} />
            ) : (
              <p className="text-muted">Checking…</p>
            )}
            <div className="flex gap-2">
              <button
                type="button"
                disabled={!preview || busy || isEmpty(preview)}
                onClick={apply}
                className="rounded border border-rule px-3 py-1 text-ink hover:border-accent disabled:opacity-40"
              >
                {preview && isEmpty(preview)
                  ? "Nothing to change"
                  : mode === "replace"
                    ? "Replace"
                    : "Import"}
              </button>
              <button
                type="button"
                onClick={() => {
                  setFile(null);
                  setPreview(null);
                }}
                className="rounded px-3 py-1 text-muted hover:text-ink"
              >
                Cancel
              </button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}

const isEmpty = (p: ImportPreview) => KINDS.every(([k]) => p[k].entries.length === 0) && !p.plan;

function PreviewTable({ mushaf, preview }: { mushaf: Mushaf; preview: ImportPreview }) {
  const cell = "px-2 py-0.5 text-right tabular-nums";
  const n = (c: Changes, k: "added" | "changed" | "removed") =>
    c[k] === 0 ? <span className="text-muted">–</span> : c[k];
  const entries = KINDS.flatMap(([k, label]) =>
    preview[k].entries.map((e) => ({ ...e, kind: label })),
  );
  return (
    <div className="flex flex-col gap-2">
      <table className="w-full text-sm">
        <thead className="text-muted text-xs">
          <tr>
            <th className="text-left font-normal" />
            <th className={`${cell} font-normal`}>new</th>
            <th className={`${cell} font-normal`}>changed</th>
            <th className={`${cell} font-normal`}>removed</th>
            <th className={`${cell} font-normal`}>same</th>
          </tr>
        </thead>
        <tbody>
          {KINDS.map(([k, label]) => (
            <tr key={k}>
              <td>{label}</td>
              <td className={cell}>{n(preview[k], "added")}</td>
              <td className={cell}>{n(preview[k], "changed")}</td>
              <td className={cell}>{n(preview[k], "removed")}</td>
              <td className={`${cell} text-muted`}>{preview[k].unchanged}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="text-muted text-xs">
        Reading plan:{" "}
        {preview.plan === "added"
          ? "added"
          : preview.plan === "changed"
            ? "changed"
            : preview.plan === "removed"
              ? "removed"
              : "unchanged"}
      </p>
      {entries.length > 0 && (
        <ul className="max-h-40 overflow-y-auto text-xs">
          {entries.slice(0, 200).map((e) => (
            <li key={`${e.kind}${e.ref.surah}:${e.ref.ayah}`} className="flex gap-2">
              <span className="w-16 text-muted">{e.change}</span>
              <span className="w-20 text-muted">{e.kind}</span>
              <span className="text-ink/85">{refLabel(mushaf, e.ref)}</span>
            </li>
          ))}
          {entries.length > 200 && (
            <li className="text-muted">… and {entries.length - 200} more</li>
          )}
        </ul>
      )}
    </div>
  );
}
