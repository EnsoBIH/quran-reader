import type { Surah, VerseRef } from "@quran-reader/types";
import { Ayah, Basmala, SurahHeader } from "@quran-reader/ui";
import { createRoot } from "react-dom/client";
import "./page.css";
import fixture from "./verses.json";

// The screenshot test page: the verses of verses.json (generated from the app's data),
// rendered with the app's components, theme and font. `pnpm test:e2e` shoots each section.

export type Row =
  | { ref: VerseRef; text: string; translation: string }
  | { header?: Surah; bismillah: string | null };

export interface Section {
  id: string;
  title: string;
  rows: Row[];
}

/** Sections shown with the translation below the Arabic, as on first start (D-020). */
const STACKED = new Set(["fatiha", "kursi"]);

function Page({ sections }: { sections: Section[] }) {
  return (
    <main
      className="mx-auto flex flex-col gap-6 px-6 py-6"
      style={{ maxWidth: "var(--reader-measure)" }}
    >
      {sections.map((s) => (
        <section key={s.id} data-section={s.id} className="bg-paper px-2">
          <h1 className="pt-2 text-muted text-xs">{s.title}</h1>
          {s.rows.map((row, i) =>
            "ref" in row ? (
              <Ayah
                key={`${row.ref.surah}:${row.ref.ayah}`}
                verseRef={row.ref}
                text={row.text}
                view={STACKED.has(s.id) ? "stacked" : "arabic"}
                translation={row.translation}
              />
            ) : (
              <div key={i}>
                {row.header && <SurahHeader surah={row.header} />}
                {row.bismillah && <Basmala text={row.bismillah} />}
              </div>
            ),
          )}
        </section>
      ))}
    </main>
  );
}

const root = document.getElementById("root");
if (!root) throw new Error("#root missing");
document.fonts.load('1em "Amiri Quran"', "ب").then(() => {
  createRoot(root).render(<Page sections={fixture.sections as Section[]} />);
  // Ready once React has painted and every font face in use has loaded.
  requestAnimationFrame(() =>
    document.fonts.ready.then(() => {
      document.body.dataset.ready = "true";
    }),
  );
});
