import type { VerseRef } from "@quran-reader/types";
import { useEffect, useRef, useState } from "react";

// N: the note of one verse. Esc or ⌘↵ saves and closes (an emptied note is deleted), so
// nothing typed is lost by closing; "Discard" leaves the note as it was.

interface NoteEditorProps {
  verse: VerseRef;
  /** `Al-Baqara 2:255`. */
  label: string;
  /** Unmodified Arabic text of the verse. */
  arabic: string;
  translation?: { text: string; language: string };
  initial: string;
  onSave: (body: string) => void;
  onClose: () => void;
}

export function NoteEditor({
  verse,
  label,
  arabic,
  translation,
  initial,
  onSave,
  onClose,
}: NoteEditorProps) {
  const [body, setBody] = useState(initial);
  const area = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const el = area.current;
    if (!el) return;
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
  }, []);

  const save = () => (body === initial ? onClose() : onSave(body));

  return (
    <div className="fixed inset-0 z-20 flex items-start justify-center px-4 pt-[10vh]">
      <button
        type="button"
        tabIndex={-1}
        aria-label="Save and close"
        onMouseDown={save}
        className="absolute inset-0 cursor-default bg-black/25"
      />
      <div
        role="dialog"
        aria-label={`Note on ${verse.surah}:${verse.ayah}`}
        className="relative flex max-h-[80vh] w-full max-w-2xl flex-col overflow-hidden rounded-lg border border-rule bg-paper shadow-xl"
        onKeyDown={(e) => {
          if (e.key === "Escape" || (e.key === "Enter" && (e.metaKey || e.ctrlKey))) {
            e.preventDefault();
            e.stopPropagation();
            save();
          }
        }}
      >
        <header className="flex items-baseline gap-2 border-b border-rule px-4 py-2 text-sm">
          <span className="text-ink">Note · {label}</span>
        </header>
        <div className="overflow-y-auto px-4 pt-3">
          <p dir="rtl" lang="ar" className="font-quran text-ink text-xl leading-[2.1]">
            {arabic}
          </p>
          {translation && (
            <p lang={translation.language} dir="auto" className="mt-1 text-ink/75 text-sm">
              {translation.text}
            </p>
          )}
        </div>
        <textarea
          ref={area}
          value={body}
          onChange={(e) => setBody(e.target.value)}
          rows={8}
          placeholder="Your note…"
          aria-label="Note"
          className="m-4 mb-2 resize-y rounded border border-rule bg-paper p-3 text-ink leading-relaxed outline-none focus:border-accent"
        />
        <footer className="flex items-center gap-2 px-4 pb-3 text-sm">
          <span className="text-muted text-xs">Esc or ⌘↵ saves · an empty note is deleted</span>
          <button
            type="button"
            onClick={onClose}
            className="ml-auto rounded px-2 py-1 text-muted hover:text-ink"
          >
            Discard changes
          </button>
          {initial !== "" && (
            <button
              type="button"
              onClick={() => onSave("")}
              className="rounded px-2 py-1 text-muted hover:text-ink"
            >
              Delete
            </button>
          )}
          <button
            type="button"
            onClick={save}
            className="rounded border border-rule px-3 py-1 text-ink hover:border-accent"
          >
            Save
          </button>
        </footer>
      </div>
    </div>
  );
}
