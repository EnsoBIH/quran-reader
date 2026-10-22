import type { CoreInfo, TranslationInfo } from "@quran-reader/types";
import { type ReactNode, useEffect, useRef } from "react";
import { REPO_URL, SOURCE_URLS } from "./links.ts";
import { languageName, reflow } from "./translations.ts";

// Credits: every bundled text and font with its source and license (LICENSES/README.md),
// the user's own imports, which stay on this computer, and where to report a problem.

interface CreditsProps {
  info: CoreInfo | null;
  translations: TranslationInfo[];
  /** Opens a link in the browser. */
  onLink: (url: string) => void;
  /** Opens the "Report a problem" form for the verse being read. */
  onReport: () => void;
  onClose: () => void;
}

/** Bundled translations whose source page the app may open. */
const TRANSLATION_URLS: Record<string, string> = { "en.pickthall": SOURCE_URLS.gutenberg };

export function Credits({ info, translations, onLink, onReport, onClose }: CreditsProps) {
  const link = (url: string, text: string) => (
    <button
      type="button"
      onClick={() => onLink(url)}
      className="text-ink underline decoration-rule underline-offset-2 hover:decoration-ink"
    >
      {text}
    </button>
  );
  const dialog = useRef<HTMLDivElement>(null);
  useEffect(() => dialog.current?.focus(), []);
  const bundled = translations.filter((t) => t.bundled);
  const imported = translations.filter((t) => !t.bundled);

  return (
    <div className="fixed inset-0 z-20 flex justify-center px-4 py-[8vh]">
      <button
        type="button"
        tabIndex={-1}
        aria-label="Close"
        onMouseDown={onClose}
        className="absolute inset-0 cursor-default bg-black/25"
      />
      <div
        ref={dialog}
        role="dialog"
        aria-label="Credits"
        tabIndex={-1}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onClose();
          }
        }}
        className="relative flex max-h-full w-full max-w-2xl flex-col overflow-hidden rounded-lg border border-rule bg-paper shadow-xl outline-none"
      >
        <header className="flex items-center border-b border-rule px-5 py-3">
          <h2 className="text-ink">Credits</h2>
          <button
            type="button"
            onClick={onClose}
            className="ml-auto rounded px-2 py-1 text-muted text-sm hover:text-ink"
          >
            Close
          </button>
        </header>
        <div className="flex flex-col gap-5 overflow-y-auto px-5 py-4 text-sm">
          <p className="text-muted">
            Quran Reader {info ? info.version : ""} is open source under the MIT license (
            {link(`${REPO_URL}/`, "github.com/EnsoBIH/quran-reader")}). The texts and fonts it ships
            keep their own licenses, listed in <code>LICENSES/</code> of the repository.
          </p>
          <section className="flex flex-col gap-2 rounded-md border border-rule px-4 py-3">
            <h3 className="text-ink">Report a problem</h3>
            <p className="text-muted">
              A verse that looks wrong, a mark in the wrong place, a translation next to the wrong
              verse, or anything else: the form on GitHub opens with the verse you are reading, the
              version and your system filled in. Nothing is sent until you submit it there.
            </p>
            <div>
              <button
                type="button"
                onClick={onReport}
                className="rounded bg-accent px-3 py-1 text-paper"
              >
                Report a problem…
              </button>
            </div>
          </section>
          <Item
            title="Arabic text and metadata"
            source={<>Tanzil.net · {link(SOURCE_URLS.tanzil, "tanzil.net")}</>}
          >
            Tanzil Quran Text (Uthmani, version 1.0.2) and Quran metadata 1.0, © Tanzil.net, under
            the Creative Commons Attribution 3.0 license. Shown exactly as published, without any
            change.
          </Item>
          <Item
            title="Font: Amiri Quran"
            source={link(SOURCE_URLS.amiri, "github.com/aliftype/amiri")}
          >
            By Khaled Hosny, under the SIL Open Font License 1.1.
          </Item>
          {bundled.map((t) => {
            const url = TRANSLATION_URLS[t.id];
            return (
              <Item
                key={t.id}
                title={`Translation: ${t.name}`}
                source={
                  <>
                    {languageName(t.language)} · {t.license}
                    {url && <> · {link(url, t.source)}</>}
                  </>
                }
              >
                <span className="whitespace-pre-line">{reflow(t.notice)}</span>
              </Item>
            );
          })}
          {imported.length > 0 && (
            <section className="flex flex-col gap-1">
              <h3 className="text-ink">Your imported translations</h3>
              <p className="text-muted">
                Stored only on this computer, never uploaded or shared. Their own terms apply.
              </p>
              <ul className="list-disc pl-5 text-muted">
                {imported.map((t) => (
                  <li key={t.id}>
                    {t.name}
                    {t.translator && ` · ${t.translator}`} ({t.source})
                  </li>
                ))}
              </ul>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}

function Item({
  title,
  source,
  children,
}: {
  title: string;
  source: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-1">
      <h3 className="text-ink">{title}</h3>
      <p className="text-muted text-xs">{source}</p>
      <p className="text-ink/85 leading-relaxed">{children}</p>
    </section>
  );
}
