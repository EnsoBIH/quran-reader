export interface BasmalaProps {
  /** Unmodified Tanzil text (its spelling differs in some surahs, e.g. 95 and 97). */
  text: string;
}

/** The Basmala as its own line above verse 1 (all surahs except 1 and 9). */
export function Basmala({ text }: BasmalaProps) {
  return (
    <p
      dir="rtl"
      lang="ar"
      className="py-6 text-center font-quran text-ink leading-[2.2]"
      style={{ fontSize: "var(--reader-arabic-size)" }}
    >
      {text}
    </p>
  );
}
