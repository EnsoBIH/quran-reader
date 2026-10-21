//! Name matching for the reference parser: surah names and named verses.
//!
//! Names are compared as *keys*. Latin input is folded to plain ASCII letters with common
//! transliteration variants unified, so `Al-Baqarah`, `baqara` and `bakara` all become
//! `bakara`. Arabic input loses its tashkeel and Quranic marks, alif variants become a bare
//! alif and the article is dropped, so `البقرة` and `بقرة` meet. Keys are internal matching
//! forms only and are never displayed.
//!
//! Keys are byte strings: Latin keys are ASCII, Arabic keys store each letter as its offset
//! from U+0600. The two are never compared with each other.

use crate::model::VerseRef;

/// A verse known by name. Keep this table small and add a test for every entry.
pub(crate) struct Alias {
    /// Shown in the palette.
    pub display: &'static str,
    /// Spellings that match, Latin or Arabic. Typos are left to the fuzzy matcher.
    pub names: &'static [&'static str],
    pub verse_ref: VerseRef,
}

pub(crate) const ALIASES: &[Alias] = &[
    Alias {
        display: "Ayat al-Kursi",
        names: &["Ayat al-Kursi", "Kursi", "Throne Verse", "آية الكرسي"],
        verse_ref: VerseRef::new(2, 255),
    },
    Alias {
        display: "Ayat ad-Dayn",
        names: &[
            "Ayat ad-Dayn",
            "Ayat al-Mudayanah",
            "Debt Verse",
            "آية الدين",
        ],
        verse_ref: VerseRef::new(2, 282),
    },
    Alias {
        display: "Amana ar-Rasul",
        names: &["Amana ar-Rasul", "Khawatim al-Baqarah", "آمن الرسول"],
        verse_ref: VerseRef::new(2, 285),
    },
    Alias {
        display: "Ayat al-Mubahala",
        names: &["Ayat al-Mubahala", "Mubahala", "آية المباهلة"],
        verse_ref: VerseRef::new(3, 61),
    },
    Alias {
        display: "Ayat an-Nur",
        names: &["Ayat an-Nur", "Light Verse", "Verse of Light", "آية النور"],
        verse_ref: VerseRef::new(24, 35),
    },
];

/// Other well-known names of surahs, in addition to the Tanzil metadata names.
pub(crate) const SURAH_NAMES: &[(u16, &str)] = &[
    (1, "Al-Hamd"),
    (1, "Umm al-Kitab"),
    (9, "Bara'ah"),
    (17, "Bani Isra'il"),
    (40, "Al-Mu'min"),
    (41, "Ha-Mim as-Sajdah"),
    (47, "Al-Qital"),
    (76, "Ad-Dahr"),
    (94, "Al-Inshirah"),
    (111, "Al-Lahab"),
    (112, "At-Tawhid"),
];

/// How well a query key matches a name key. Lower is better; compare as a tuple.
pub(crate) type Score = (u8, u8);

/// Whether `s` should be matched as Arabic.
pub(crate) fn is_arabic(s: &str) -> bool {
    s.chars()
        .any(|c| ('\u{0621}'..='\u{064A}').contains(&c) || c == '\u{0671}')
}

/// Builds keys. Holds its buffers, so matching allocates nothing per candidate.
#[derive(Default)]
pub(crate) struct Keyer {
    words: Words,
    raw: Vec<u8>,
}

impl Keyer {
    /// Writes the key of `name` to `out`. With `last_word`, only its last word.
    pub(crate) fn key(&mut self, name: &str, arabic: bool, last_word: bool, out: &mut Vec<u8>) {
        self.words.clear();
        if arabic {
            arabic_words(name, &mut self.words);
        } else {
            latin_words(name, &mut self.words);
        }
        self.words.join(arabic, last_word, &mut self.raw);
        out.clear();
        if arabic {
            out.extend_from_slice(&self.raw);
        } else {
            fold(&self.raw, out);
        }
    }

    /// Like [`Keyer::key`] for user input, which may glue a Latin article to the name
    /// (`albaqara`, `annas`): writes a second key without it to `alt`, or leaves `alt`
    /// empty. Returns whether the input is Arabic.
    pub(crate) fn query_keys(&mut self, name: &str, out: &mut Vec<u8>, alt: &mut Vec<u8>) -> bool {
        let arabic = is_arabic(name);
        self.key(name, arabic, false, out);
        alt.clear();
        let raw = &self.raw;
        let glued = !arabic
            && raw.len() > 4
            && raw[0] == b'a'
            && (raw[1] == b'l' || (raw[1] == raw[2] && raw[1] != b'a'));
        if glued {
            fold(&raw[2..], alt);
        }
        arabic
    }
}

/// Scores `query` against `name`, or `None` if they are too far apart.
///
/// Classes: 0 exact, 1 prefix (`baq` → `baqara`), 2 typo (`bqara`), 3 typo in a prefix.
pub(crate) fn score(query: &[u8], name: &[u8]) -> Option<Score> {
    if query.is_empty() || name.is_empty() {
        return None;
    }
    if query == name {
        return Some((0, 0));
    }
    if query.len() >= 2 && name.starts_with(query) {
        return Some((1, clamp(name.len() - query.len())));
    }
    let max = match query.len() {
        0..=3 => return None,
        4..=6 => 1,
        7..=10 => 2,
        _ => 3,
    };
    if name.len() + max < query.len() {
        return None;
    }
    let (full, prefix) = distances(query, name);
    if full <= max {
        Some((2, clamp(full)))
    } else if prefix <= max {
        Some((3, clamp(prefix)))
    } else {
        None
    }
}

fn clamp(n: usize) -> u8 {
    n.min(usize::from(u8::MAX)) as u8
}

/// Longest key compared; longer ones are cut (no name comes close).
const MAX_KEY: usize = 40;

/// Optimal string alignment distance of `a` to `b`, and of `a` to the closest prefix of `b`.
fn distances(a: &[u8], b: &[u8]) -> (usize, usize) {
    let a = &a[..a.len().min(MAX_KEY)];
    let b = &b[..b.len().min(MAX_KEY)];
    let mut rows = [[0usize; MAX_KEY + 1]; 3];
    let (mut prev2, mut prev, mut cur) = (0, 1, 2);
    for (j, cell) in rows[prev].iter_mut().enumerate().take(b.len() + 1) {
        *cell = j;
    }
    for i in 1..=a.len() {
        rows[cur][0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut d = (rows[prev][j] + 1)
                .min(rows[cur][j - 1] + 1)
                .min(rows[prev][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d = d.min(rows[prev2][j - 2] + 1);
            }
            rows[cur][j] = d;
        }
        (prev2, prev, cur) = (prev, cur, prev2);
    }
    let last = &rows[prev][..=b.len()];
    let prefix = last.iter().copied().min().unwrap_or(a.len());
    (last[b.len()], prefix)
}

/// Words of a name as raw letters, at most a few of them.
#[derive(Default)]
struct Words {
    letters: Vec<u8>,
    ends: Vec<usize>,
}

impl Words {
    fn clear(&mut self) {
        self.letters.clear();
        self.ends.clear();
    }

    fn push(&mut self, letter: u8) {
        self.letters.push(letter);
    }

    fn end_word(&mut self) {
        if self.ends.last().copied().unwrap_or(0) < self.letters.len() {
            self.ends.push(self.letters.len());
        }
    }

    fn word(&self, i: usize) -> &[u8] {
        let start = if i == 0 { 0 } else { self.ends[i - 1] };
        &self.letters[start..self.ends[i]]
    }

    /// Joins the words, dropping leading noise words (`surah`, `the`) and articles. With
    /// `last_word`, only the last word.
    fn join(&self, arabic: bool, last_word: bool, out: &mut Vec<u8>) {
        out.clear();
        let n = self.ends.len();
        let mut first = if last_word { n.saturating_sub(1) } else { 0 };
        while first + 1 < n && is_prefix_word(self.word(first)) {
            first += 1;
        }
        for i in first..n {
            let w = self.word(i);
            let last = i + 1 == n;
            if arabic {
                // `البقرة` → `بقرة`
                out.extend_from_slice(strip_article(w));
            } else if last {
                out.extend_from_slice(w);
            } else if !is_article(w) {
                out.extend_from_slice(strip_ul(w));
            }
        }
    }
}

/// `surah`, `sura`, `sure` (German), `the`, `سورة`.
fn is_prefix_word(w: &[u8]) -> bool {
    matches!(
        w,
        b"the" | b"surah" | b"sura" | b"surat" | b"suratu" | b"sure"
    ) || w == ARABIC_SURAH
}

fn is_article(w: &[u8]) -> bool {
    matches!(
        w,
        b"al"
            | b"el"
            | b"ul"
            | b"an"
            | b"ar"
            | b"as"
            | b"at"
            | b"ad"
            | b"adh"
            | b"az"
            | b"ash"
            | b"ath"
    )
}

/// `ayatul kursi` → `ayat kursi`.
fn strip_ul(w: &[u8]) -> &[u8] {
    match w.strip_suffix(b"ul") {
        Some(rest) if rest.len() >= 3 => rest,
        _ => w,
    }
}

/// Arabic `ال` at the start of a word of four letters or more.
fn strip_article(w: &[u8]) -> &[u8] {
    match w.strip_prefix(&ARABIC_AL[..]) {
        Some(rest) if rest.len() >= 2 => rest,
        _ => w,
    }
}

const fn arabic(c: char) -> u8 {
    (c as u32 - 0x0600) as u8
}

const ARABIC_AL: [u8; 2] = [arabic('ا'), arabic('ل')];
/// Keys of the Arabic keywords سورة, جزء (or جز), حزب and صفحة.
pub(crate) const ARABIC_SURAH: &[u8] = &[arabic('س'), arabic('و'), arabic('ر'), arabic('ه')];
pub(crate) const ARABIC_JUZ: &[u8] = &[arabic('ج'), arabic('ز'), arabic('ء')];
pub(crate) const ARABIC_JUZ_SHORT: &[u8] = &[arabic('ج'), arabic('ز')];
pub(crate) const ARABIC_HIZB: &[u8] = &[arabic('ح'), arabic('ز'), arabic('ب')];
pub(crate) const ARABIC_PAGE: &[u8] = &[arabic('ص'), arabic('ف'), arabic('ح'), arabic('ه')];

fn latin_words(name: &str, words: &mut Words) {
    for c in name.chars().flat_map(char::to_lowercase) {
        match c {
            'a'..='z' => words.push(c as u8),
            'à' | 'á' | 'â' | 'ä' | 'å' | 'ā' => words.push(b'a'),
            'è' | 'é' | 'ê' | 'ë' | 'ē' => words.push(b'e'),
            'ì' | 'í' | 'î' | 'ï' | 'ī' => words.push(b'i'),
            'ò' | 'ó' | 'ô' | 'ö' | 'ō' => words.push(b'o'),
            'ù' | 'ú' | 'û' | 'ü' | 'ū' => words.push(b'u'),
            'ḥ' | 'ḫ' => words.push(b'h'),
            'ṣ' | 'š' => words.push(b's'),
            'ḍ' => words.push(b'd'),
            'ṭ' => words.push(b't'),
            'ẓ' | 'ž' => words.push(b'z'),
            'ġ' => words.push(b'g'),
            // Ayn and hamza marks join the word: `an'aam`, `ra'd`, `mu'minun`.
            '\'' | '’' | '‘' | '`' | 'ʿ' | 'ʾ' | 'ʼ' => {}
            _ => words.end_word(),
        }
    }
    words.end_word();
}

fn arabic_words(name: &str, words: &mut Words) {
    for c in name.chars() {
        match c {
            // Tashkeel, Quranic annotation marks, superscript alif, tatweel.
            '\u{0610}'..='\u{061A}'
            | '\u{064B}'..='\u{065F}'
            | '\u{0670}'
            | '\u{06D6}'..='\u{06ED}'
            | '\u{0640}' => {}
            'أ' | 'إ' | 'آ' | 'ٱ' => words.push(arabic('ا')),
            'ى' | 'ئ' | 'ی' => words.push(arabic('ي')),
            'ة' => words.push(arabic('ه')),
            'ؤ' => words.push(arabic('و')),
            'ک' => words.push(arabic('ك')),
            '\u{0621}'..='\u{064A}' => words.push(arabic(c)),
            _ => words.end_word(),
        }
    }
    words.end_word();
}

/// Unifies transliteration variants: `q`/`k`, `e`/`i`, `o`/`u`, `w`/`u`, `y`/`i`, `dh`/`d`,
/// `th`/`t`, doubled letters, and a final `h` after a vowel (`baqarah`).
fn fold(raw: &[u8], out: &mut Vec<u8>) {
    let mut previous_raw = 0u8;
    for &b in raw {
        let c = match b {
            b'q' => b'k',
            b'e' | b'y' => b'i',
            b'o' | b'w' => b'u',
            b'h' if matches!(previous_raw, b'd' | b't') => {
                previous_raw = b;
                continue;
            }
            _ => b,
        };
        previous_raw = b;
        if out.last() != Some(&c) {
            out.push(c);
        }
    }
    if out.len() >= 3 && out.ends_with(b"h") && matches!(out[out.len() - 2], b'a' | b'i' | b'u') {
        out.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(name: &str) -> String {
        let mut out = Vec::new();
        Keyer::default().key(name, is_arabic(name), false, &mut out);
        if is_arabic(name) {
            out.iter()
                .map(|&b| char::from_u32(0x0600 + u32::from(b)).unwrap())
                .collect()
        } else {
            String::from_utf8(out).unwrap()
        }
    }

    #[test]
    fn latin_variants_share_a_key() {
        for name in [
            "Al-Baqara",
            "Al-Baqarah",
            "baqara",
            "bakara",
            "Surah al-Baqarah",
        ] {
            assert_eq!(k(name), "bakara", "{name}");
        }
        assert_eq!(k("Yaseen"), k("Ya-Sin"));
        assert_eq!(k("Al-An'aam"), k("al-anam"));
        assert_eq!(k("Al-Muminoon"), k("Mu'minun"));
        assert_eq!(k("Al-Lail"), k("al-layl"));
        assert_eq!(k("At-Tawba"), k("Tauba"));
        assert_eq!(k("Ayat al-Kursi"), k("ayatul kursi"));
        assert_eq!(k("Ayat al-Kursi"), k("ayat-ul-kursi"));
        assert_eq!(k("The Cow"), "cu");
    }

    #[test]
    fn arabic_drops_marks_and_article() {
        assert_eq!(k("البقرة"), "بقره");
        assert_eq!(k("بقرة"), "بقره");
        assert_eq!(k("ٱلْبَقَرَةِ"), "بقره");
        assert_eq!(k("سورة البقرة"), "بقره");
        assert_eq!(k("سبإ"), k("سبأ"));
        // آل (family) is a word, not the article.
        assert_eq!(k("آل عمران"), "العمران");
    }

    #[test]
    fn glued_articles_get_a_second_key() {
        let (mut keyer, mut out, mut alt) = (Keyer::default(), Vec::new(), Vec::new());
        keyer.query_keys("albaqara", &mut out, &mut alt);
        assert_eq!(alt, b"bakara");
        keyer.query_keys("annas", &mut out, &mut alt);
        assert_eq!(alt, b"nas");
        keyer.query_keys("baqara", &mut out, &mut alt);
        assert!(alt.is_empty());
    }

    #[test]
    fn scores_rank_exact_prefix_typo() {
        assert_eq!(score(b"bakara", b"bakara"), Some((0, 0)));
        assert_eq!(score(b"bak", b"bakara"), Some((1, 3)));
        assert_eq!(score(b"bkara", b"bakara"), Some((2, 1)));
        assert_eq!(score(b"abkara", b"bakara"), Some((2, 1)));
        assert_eq!(score(b"bkar", b"bakara"), Some((3, 1)));
        assert_eq!(score(b"xyz", b"bakara"), None);
        assert_eq!(score(b"kahf", b"bakara"), None);
    }
}
