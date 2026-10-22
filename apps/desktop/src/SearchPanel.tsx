import type { SearchHit, TranslationInfo } from "@quran-reader/types";
import { Marked } from "@quran-reader/ui";
import { useEffect, useRef, useState } from "react";
import * as core from "./core.ts";
import type { Mushaf } from "./mushaf.ts";
import { QURAN_TEXT, snippet } from "./search.ts";

// ⌘F: full-text search beside the reader, over the Arabic and every installed translation.
// Matching and highlight ranges come from quran-core (`search`); this lists the hits and
// jumps to them.

interface SearchPanelProps {
  mushaf: Mushaf;
  translations: TranslationInfo[];
  /** Loaded translation texts, for the snippets of translation matches. */
  texts: ReadonlyMap<string, string[]>;
  query: string;
  onQuery: (query: string) => void;
  /** Changes whenever ⌘F is pressed, to focus the input again. */
  focusKey: number;
  /** The hit being read, highlighted in the list. */
  active: number | null;
  onGo: (hit: SearchHit) => void;
  onClose: () => void;
}

const PAGE = 100;
const EXAMPLES = ["رحمن", '"الحمد لله"', "الله -رحيم", "نور surah:24", "رب juz:30"];

export function SearchPanel({
  mushaf,
  translations,
  texts,
  query,
  onQuery,
  focusKey,
  active,
  onGo,
  onClose,
}: SearchPanelProps) {
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [total, setTotal] = useState(0);
  const [limit, setLimit] = useState(PAGE);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLUListElement>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: focusKey is the trigger
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, [focusKey]);

  useEffect(() => setLimit(PAGE), [query]);

  useEffect(() => {
    let stale = false;
    if (query.trim() === "") {
      setHits([]);
      setTotal(0);
      setError(null);
      return;
    }
    core
      .search(query, limit)
      .then((r) => {
        if (stale) return;
        setHits(r.hits);
        setTotal(r.total);
        setError(null);
        if (limit === PAGE) setSelected(0);
      })
      .catch((e: unknown) => {
        if (stale) return;
        setHits([]);
        setTotal(0);
        setError(String(e));
      });
    return () => {
      stale = true;
    };
  }, [query, limit]);

  useEffect(() => {
    list.current?.querySelector(`[data-hit="${selected}"]`)?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const go = (i: number) => {
    const hit = hits[i];
    if (!hit) return;
    setSelected(i);
    onGo(hit);
  };

  return (
    <aside
      aria-label="Search"
      className="flex w-[26rem] max-w-[45vw] shrink-0 flex-col border-l border-rule bg-paper"
    >
      <div className="flex items-center gap-2 border-b border-rule px-3 py-2">
        <input
          ref={input}
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSelected((i) => Math.min(i + 1, hits.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSelected((i) => Math.max(i - 1, 0));
            } else if (e.key === "Enter") {
              e.preventDefault();
              // Shift+Enter goes back up the list, Enter down it once a hit is shown.
              if (e.shiftKey) go(Math.max(selected - 1, 0));
              else go(active === hits[selected]?.index ? selected + 1 : selected);
            } else if (e.key === "Escape") {
              e.preventDefault();
              onClose();
            }
          }}
          placeholder="Search the Quran…"
          aria-label="Search"
          spellCheck={false}
          autoComplete="off"
          className="min-w-0 flex-1 bg-transparent py-1 text-ink outline-none placeholder:text-muted"
        />
        <button
          type="button"
          onClick={onClose}
          className="rounded px-1.5 text-muted hover:text-ink"
          aria-label="Close search"
          title="Close (Esc)"
        >
          ×
        </button>
      </div>
      {error !== null ? (
        <p className="px-4 py-3 text-muted text-sm">{error}</p>
      ) : query.trim() === "" ? (
        <Help onExample={onQuery} />
      ) : (
        <>
          <p className="border-b border-rule px-4 py-1.5 text-muted text-xs tabular-nums">
            {total === 0
              ? "No verses found"
              : `${total.toLocaleString("en")} ${total === 1 ? "verse" : "verses"}`}
          </p>
          <ul ref={list} className="min-h-0 flex-1 overflow-y-auto">
            {hits.map((hit, i) => (
              <li key={hit.index} data-hit={i}>
                <Result
                  mushaf={mushaf}
                  translations={translations}
                  texts={texts}
                  hit={hit}
                  selected={i === selected}
                  active={hit.index === active}
                  onClick={() => go(i)}
                />
              </li>
            ))}
            {hits.length < total && (
              <li className="p-3 text-center">
                <button
                  type="button"
                  onClick={() => setLimit((l) => l + PAGE)}
                  className="rounded border border-rule px-3 py-1 text-muted text-sm hover:text-ink"
                >
                  Show more ({hits.length} of {total.toLocaleString("en")})
                </button>
              </li>
            )}
          </ul>
        </>
      )}
    </aside>
  );
}

function Result({
  mushaf,
  translations,
  texts,
  hit,
  selected,
  active,
  onClick,
}: {
  mushaf: Mushaf;
  translations: TranslationInfo[];
  texts: ReadonlyMap<string, string[]>;
  hit: SearchHit;
  selected: boolean;
  active: boolean;
  onClick: () => void;
}) {
  const s = mushaf.surahs[hit.ref.surah - 1];
  // Every text that matched: the Arabic first, then the translations in settings order.
  const parts = hit.matches.map((m) => {
    if (m.text === QURAN_TEXT) {
      return { id: m.text, part: snippet(mushaf.verses[hit.index] ?? "", m.ranges) };
    }
    const t = translations.find((x) => x.id === m.text);
    const verse = texts.get(m.text)?.[hit.index];
    return { id: m.text, info: t, part: verse === undefined ? null : snippet(verse, m.ranges) };
  });
  return (
    <button
      type="button"
      aria-current={selected}
      onClick={onClick}
      className="flex w-full flex-col gap-1 border-b border-rule px-4 py-2.5 text-left aria-current:bg-rule/60"
    >
      <span className="flex items-baseline gap-2 text-xs">
        <span className={active ? "text-accent" : "text-muted"}>
          {s?.nameLatin ?? `Surah ${hit.ref.surah}`} {hit.ref.surah}:{hit.ref.ayah}
        </span>
      </span>
      {parts.map(({ id, info, part }) =>
        id === QURAN_TEXT && part ? (
          <span key={id} dir="rtl" lang="ar" className="font-quran text-ink text-xl leading-[2.1]">
            {part.cutStart && <span className="text-muted">… </span>}
            <Marked text={part.text} ranges={part.ranges} />
            {part.cutEnd && <span className="text-muted"> …</span>}
          </span>
        ) : (
          <span key={id} lang={info?.language} dir="auto" className="text-ink/85 text-sm">
            <span className="mr-1.5 text-muted text-xs">{info?.name ?? id}</span>
            {part ? (
              <>
                {part.cutStart && <span className="text-muted">… </span>}
                <Marked text={part.text} ranges={part.ranges} />
                {part.cutEnd && <span className="text-muted"> …</span>}
              </>
            ) : (
              <span className="text-muted">…</span>
            )}
          </span>
        ),
      )}
    </button>
  );
}

function Help({ onExample }: { onExample: (q: string) => void }) {
  const row = "flex justify-between gap-4";
  return (
    <div className="flex flex-col gap-3 px-4 py-3 text-muted text-sm">
      <p>
        Searches the Arabic text, ignoring tashkeel and letter variants (أ إ آ ٱ, ى/ي, ة/ه), and
        every installed translation (case and accents ignored, words match at their start).
      </p>
      <dl className="flex flex-col gap-1">
        <div className={row}>
          <dt>All words</dt>
          <dd className="text-ink">رحمن رحيم</dd>
        </div>
        <div className={row}>
          <dt>Phrase</dt>
          <dd className="text-ink">"الحمد لله"</dd>
        </div>
        <div className={row}>
          <dt>Leave out</dt>
          <dd className="text-ink">الله -رحيم</dd>
        </div>
        <div className={row}>
          <dt>Only in</dt>
          <dd className="text-ink">surah:2 · surah:2-4 · surah:yasin · juz:30</dd>
        </div>
      </dl>
      <p>
        Try{" "}
        {EXAMPLES.map((ex, i) => (
          <span key={ex}>
            {i > 0 && " · "}
            <button
              type="button"
              className="text-ink hover:text-accent"
              onClick={() => onExample(ex)}
            >
              {ex}
            </button>
          </span>
        ))}
      </p>
      <p className="text-xs">↑ ↓ choose · Enter go to the verse · Esc close</p>
    </div>
  );
}
