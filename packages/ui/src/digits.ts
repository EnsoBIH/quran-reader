const ARABIC_INDIC_ZERO = 0x0660;

/** Formats a non-negative integer with Arabic-Indic digits (٠١٢٣٤٥٦٧٨٩). */
export function toArabicIndic(n: number): string {
  if (!Number.isInteger(n) || n < 0) {
    throw new RangeError(`expected a non-negative integer, got ${n}`);
  }
  return String(n).replace(/[0-9]/g, (d) => String.fromCharCode(ARABIC_INDIC_ZERO + Number(d)));
}

/** End-of-ayah sign (U+06DD). Amiri Quran draws the following digits inside it. */
export const END_OF_AYAH = "۝";
