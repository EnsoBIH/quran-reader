//! Character folding shared by name matching (`names.rs`) and the search index
//! (`search.rs`).
//!
//! Folded forms are internal matching keys. They are never displayed and never stored in
//! the data file; the text on screen is always the unmodified Tanzil text.

/// What a character becomes in a folded form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fold {
    /// Left out without ending the word: tashkeel, Quranic marks, apostrophes.
    Drop,
    /// Ends the word: spaces, punctuation, symbols.
    Break,
    Letter(char),
    /// `ß` → `ss`.
    Letters(char, char),
}

/// Whether `c` is in one of the Arabic blocks (letters, marks and Arabic punctuation).
pub(crate) fn is_arabic_block(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}')
}

/// Arabic: tashkeel, Quranic annotation marks, superscript alif and tatweel are dropped;
/// alif variants (أ إ آ ٱ) become a bare alif, ى ئ ی become ي, ة becomes ه, ؤ becomes و and
/// ک becomes ك. Other letters stay; everything else ends the word.
pub(crate) fn arabic(c: char) -> Fold {
    match c {
        // Tashkeel, Quranic annotation marks (pause and sajda signs, small letters),
        // superscript alif, tatweel.
        '\u{0610}'..='\u{061A}'
        | '\u{064B}'..='\u{065F}'
        | '\u{0670}'
        | '\u{06D6}'..='\u{06ED}'
        | '\u{08D3}'..='\u{08FF}'
        | '\u{0640}' => Fold::Drop,
        'أ' | 'إ' | 'آ' | 'ٱ' => Fold::Letter('ا'),
        'ى' | 'ئ' | 'ی' => Fold::Letter('ي'),
        'ة' => Fold::Letter('ه'),
        'ؤ' => Fold::Letter('و'),
        'ک' => Fold::Letter('ك'),
        '\u{0621}'..='\u{064A}' => Fold::Letter(c),
        _ => Fold::Break,
    }
}

/// Latin and other scripts: lowercase, Latin diacritics folded (`é` → `e`, `ḥ` → `h`), ayn
/// and hamza marks and apostrophes dropped so `an'aam` and `God's` stay one word. Other
/// letters and digits stay; everything else ends the word.
pub(crate) fn latin(c: char) -> Fold {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'a'..='z' | '0'..='9' => Fold::Letter(c),
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => Fold::Letter('a'),
        'ç' => Fold::Letter('c'),
        'è' | 'é' | 'ê' | 'ë' | 'ē' => Fold::Letter('e'),
        'ì' | 'í' | 'î' | 'ï' | 'ī' => Fold::Letter('i'),
        'ñ' => Fold::Letter('n'),
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' => Fold::Letter('o'),
        'ù' | 'ú' | 'û' | 'ü' | 'ū' => Fold::Letter('u'),
        'ḥ' | 'ḫ' => Fold::Letter('h'),
        'ṣ' | 'š' => Fold::Letter('s'),
        'ḍ' => Fold::Letter('d'),
        'ṭ' => Fold::Letter('t'),
        'ẓ' | 'ž' => Fold::Letter('z'),
        'ġ' => Fold::Letter('g'),
        'ß' => Fold::Letters('s', 's'),
        '\'' | '’' | '‘' | '`' | 'ʿ' | 'ʾ' | 'ʼ' => Fold::Drop,
        // Combining marks of decomposed input (`e` + U+0301).
        '\u{0300}'..='\u{036F}' => Fold::Drop,
        _ if c.is_alphanumeric() => Fold::Letter(c),
        _ => Fold::Break,
    }
}

/// The fold of the search index: [`arabic`] for Arabic characters, [`latin`] for the rest,
/// and additionally the standalone hamza ء is dropped. The Uthmani text writes a hamza on
/// the line where common spelling has a madda or a seat (`ءَامَنُوا` for `آمنوا`, `شَىْءٍ` for
/// `شيء`), so without it both spellings meet.
pub(crate) fn search(c: char) -> Fold {
    if c == 'ء' {
        Fold::Drop
    } else if is_arabic_block(c) {
        arabic(c)
    } else {
        latin(c)
    }
}

/// Appends the search form of `text` to `out`: folded letters, words separated by one
/// space, no space at either end.
///
/// With `origin`, also appends for every byte written to `out` the byte range of `text` it
/// came from. Dropped marks are added to the range of the letter right before them, so a
/// mapped range never splits a letter from its tashkeel.
pub(crate) fn normalize(text: &str, out: &mut String, mut origin: Option<&mut Vec<(u32, u32)>>) {
    let start = out.len();
    // Bytes of `origin` that belong to the last letter written, while marks may follow it.
    let mut last: Option<(usize, usize)> = None;
    let mut space = false;
    for (i, c) in text.char_indices() {
        let end = i + c.len_utf8();
        let (a, b) = match search(c) {
            Fold::Drop => {
                if let (Some(o), Some((from, to))) = (origin.as_deref_mut(), last) {
                    for range in &mut o[from..to] {
                        range.1 = end as u32;
                    }
                }
                continue;
            }
            Fold::Break => {
                space = out.len() > start;
                last = None;
                continue;
            }
            Fold::Letter(a) => (a, None),
            Fold::Letters(a, b) => (a, Some(b)),
        };
        if space {
            out.push(' ');
            if let Some(o) = origin.as_deref_mut() {
                o.push((i as u32, i as u32));
            }
            space = false;
        }
        let before = out.len();
        out.push(a);
        if let Some(b) = b {
            out.push(b);
        }
        if let Some(o) = origin.as_deref_mut() {
            let from = o.len();
            o.extend(std::iter::repeat_n(
                (i as u32, end as u32),
                out.len() - before,
            ));
            last = Some((from, o.len()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(text: &str) -> String {
        let mut out = String::new();
        normalize(text, &mut out, None);
        out
    }

    #[test]
    fn strips_tashkeel_and_quranic_marks() {
        // Fatha, kasra, damma, tanwin, shadda, sukun, superscript alif, tatweel.
        assert_eq!(n("كَتَبَ"), "كتب");
        assert_eq!(n("كِتَابٌ"), "كتاب");
        assert_eq!(n("مُدَرِّسٍ"), "مدرس");
        assert_eq!(n("رَحْمَـٰن"), "رحمن");
        // Small high letters, rounded zero, small waw and yeh.
        assert_eq!(n("قَالُوا۟"), "قالوا");
        assert_eq!(n("بِهِۦ"), "به");
        assert_eq!(n("لَهُۥ"), "له");
    }

    #[test]
    fn unifies_letter_variants() {
        for alif in ["أمر", "إمر", "آمر", "ٱمر", "امر"] {
            assert_eq!(n(alif), "امر", "{alif}");
        }
        assert_eq!(n("هدى"), "هدي");
        assert_eq!(n("مدرسة"), "مدرسه");
        assert_eq!(n("سئل"), "سيل");
        assert_eq!(n("مؤمن"), "مومن");
        // A hamza on the line is dropped, so Uthmani and common spellings meet.
        assert_eq!(n("ءَامَنَ"), n("آمن"));
        assert_eq!(n("شَىْءٍ"), n("شيء"));
    }

    #[test]
    fn pause_marks_and_sajda_signs_do_not_leave_words() {
        assert_eq!(n("قَالَ ۚ إِنَّ"), "قال ان");
        assert_eq!(n("كَتَبَ ۩"), "كتب");
        assert_eq!(n("  كتب،  قال  "), "كتب قال");
    }

    #[test]
    fn folds_latin() {
        assert_eq!(n("Café Naïve"), "cafe naive");
        assert_eq!(n("Straße"), "strasse");
        assert_eq!(n("God's word, (twice)!"), "gods word twice");
        assert_eq!(n("Ḥadīth"), "hadith");
        assert_eq!(n("cafe\u{0301}"), "cafe");
        assert_eq!(n("Год 2"), "год 2");
    }

    #[test]
    fn origin_maps_every_byte_and_keeps_marks_with_their_letter() {
        let text = "كَتَبَ ۚ قَالَ";
        let (mut out, mut origin) = (String::new(), Vec::new());
        normalize(text, &mut out, Some(&mut origin));
        assert_eq!(out, "كتب قال");
        assert_eq!(origin.len(), out.len());
        // The first letter maps to itself and its fatha.
        let (s, e) = origin[0];
        assert_eq!(&text[s as usize..e as usize], "كَ");
        // The last letter of the first word does not take the pause mark after the space.
        let (s, e) = origin["كتب".len() - 1];
        assert_eq!(&text[s as usize..e as usize], "بَ");
    }
}
