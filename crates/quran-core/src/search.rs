//! Full-text search for the ⌘F panel and `quran search`.
//!
//! Query language:
//! - words: every word must occur in the verse — `رحمن`, `mercy`;
//! - `"…"` (also `“…”`, `„…“`, `«…»`): a phrase, its words adjacent and in this order;
//! - `-word`, `-"…"`: the verse must not contain it;
//! - `surah:2`, `surah:2-4`, `surah:baqara`, `juz:30`, `juz:28-30` (also `sura:`, `s:`, `j:`):
//!   search only there. Scopes of one kind add up; `surah:` and `juz:` together intersect.
//!
//! Matching runs on normalized forms (`normalize.rs`): Arabic without tashkeel and Quranic
//! marks, alif variants, ى/ي, ة/ه and the hamza seats unified; Latin lowercased without
//! diacritics. An Arabic word or phrase matches anywhere inside a word, since particles,
//! prepositions and the article are written together with the next word and pronouns with
//! the one before — `رحمن` finds `ٱلرَّحْمَٰنِ`. A Latin one matches at the start of a word,
//! so `merc` finds `mercy` but `in` does not find `within`.
//!
//! The index holds one or more searchable texts, each a string per verse aligned with the
//! Quran by global verse index: the Tanzil Arabic ([`QURAN_TEXT`]) and, from Phase 4, the
//! translations ([`SearchIndex::add_text`]). A query runs on each text on its own; a verse
//! is a hit if any text matches. Hits carry the matched ranges mapped back onto the
//! unmodified text, so the UI highlights without touching the text. The normalized forms
//! are built at run time (DECISIONS.md D-015) and never leave this module.

use std::fmt;
use std::ops::Range;

use crate::model::{Part, SearchHit, SearchResults, Target, TextMatches, TextRange};
use crate::normalize;
use crate::reference::ascii_digit;
use crate::{AYAH_COUNT, Quran, SURAH_COUNT, VerseRef};

/// Id of the Tanzil Arabic text in an index.
pub const QURAN_TEXT: &str = "quran";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    Empty,
    /// The query has only exclusions or scopes.
    NothingToFind,
    InvalidScope(String),
    UnknownSurah(String),
    SurahOutOfRange(u16),
    JuzOutOfRange {
        number: u16,
        count: u16,
    },
    /// A text added to the index does not have one string per verse.
    VerseCount {
        text: String,
        count: usize,
    },
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty search"),
            Self::NothingToFind => write!(f, "nothing to search for — add a word or a phrase"),
            Self::InvalidScope(s) => write!(
                f,
                "not a scope: {s:?} (try surah:2, surah:2-4, surah:baqara or juz:30)"
            ),
            Self::UnknownSurah(s) => write!(f, "no surah matches {s:?}"),
            Self::SurahOutOfRange(n) => write!(f, "surah {n} does not exist (1–{SURAH_COUNT})"),
            Self::JuzOutOfRange { number, count } => {
                write!(f, "juz {number} does not exist (1–{count})")
            }
            Self::VerseCount { text, count } => {
                write!(f, "text {text:?} has {count} verses, expected {AYAH_COUNT}")
            }
        }
    }
}

impl std::error::Error for SearchError {}

/// Searchable texts of the Quran. Build once and share; see [`crate::search_index`].
pub struct SearchIndex<'a> {
    quran: Quran<'a>,
    texts: Vec<SearchText<'a>>,
}

impl<'a> SearchIndex<'a> {
    /// An index of the Tanzil Arabic text.
    pub fn new(quran: Quran<'a>) -> Self {
        Self {
            quran,
            texts: vec![SearchText::new(QURAN_TEXT, quran.verses().collect())],
        }
    }

    /// Adds a further searchable text, e.g. a translation: one string per verse in Quran
    /// order. Its hits carry `id`.
    pub fn add_text(&mut self, id: &str, verses: Vec<&'a str>) -> Result<(), SearchError> {
        if verses.len() != usize::from(AYAH_COUNT) {
            return Err(SearchError::VerseCount {
                text: id.to_owned(),
                count: verses.len(),
            });
        }
        self.texts.push(SearchText::new(id, verses));
        Ok(())
    }

    /// Verses matching `query`, in Quran order: the count of all of them and the first
    /// `limit` with their matched ranges.
    pub fn search(&self, query: &str, limit: usize) -> Result<SearchResults, SearchError> {
        let query = Query::parse(query)?;
        let scope = self.resolve(&query.scopes)?;
        // (verse, text), sorted by verse.
        let mut found: Vec<(u16, usize)> = Vec::new();
        for (t, text) in self.texts.iter().enumerate() {
            found.extend(text.matching(&query, &scope).into_iter().map(|v| (v, t)));
        }
        if self.texts.len() > 1 {
            found.sort_unstable();
        }

        let (mut total, mut hits) = (0, Vec::new());
        for group in found.chunk_by(|a, b| a.0 == b.0) {
            total += 1;
            if hits.len() >= limit {
                continue;
            }
            let index = group[0].0;
            let Some(verse_ref) = self.quran.verse_ref_at(index) else {
                continue;
            };
            hits.push(SearchHit {
                verse_ref,
                index,
                matches: group
                    .iter()
                    .map(|&(_, t)| {
                        let text = &self.texts[t];
                        TextMatches {
                            text: text.id.clone(),
                            ranges: text.ranges(usize::from(index), &query.include),
                        }
                    })
                    .collect(),
            });
        }
        Ok(SearchResults { total, hits })
    }

    /// Global verse index ranges to search, sorted and disjoint.
    fn resolve(&self, scopes: &[Scope]) -> Result<Vec<Range<usize>>, SearchError> {
        let all = 0..usize::from(AYAH_COUNT);
        let (mut surahs, mut juz) = (Vec::new(), Vec::new());
        for scope in scopes {
            match scope {
                Scope::Surahs(from, to) => surahs.push(self.surah_range(*from, *to)?),
                Scope::SurahName(name) => {
                    let n = self.surah_named(name)?;
                    surahs.push(self.surah_range(n, n)?);
                }
                Scope::Juz(from, to) => juz.push(self.juz_range(*from, *to)?),
            }
        }
        let surahs = if surahs.is_empty() {
            vec![all.clone()]
        } else {
            merge(surahs)
        };
        let juz = if juz.is_empty() {
            vec![all]
        } else {
            merge(juz)
        };
        Ok(intersect(&surahs, &juz))
    }

    fn surah_range(&self, from: u16, to: u16) -> Result<Range<usize>, SearchError> {
        let first = |n: u16| self.quran.global_index(VerseRef::new(n, 1));
        let start = first(from).ok_or(SearchError::SurahOutOfRange(from))?;
        let last = self
            .quran
            .surah(to)
            .and_then(|s| self.quran.global_index(VerseRef::new(to, s.ayah_count)))
            .ok_or(SearchError::SurahOutOfRange(to))?;
        Ok(usize::from(start)..usize::from(last) + 1)
    }

    fn surah_named(&self, name: &str) -> Result<u16, SearchError> {
        let unknown = || SearchError::UnknownSurah(name.to_owned());
        let hits = self.quran.lookup(name, 8).map_err(|_| unknown())?;
        hits.iter()
            .find(|h| h.target == Target::Surah)
            .map(|h| h.verse_ref.surah)
            .ok_or_else(unknown)
    }

    fn juz_range(&self, from: u16, to: u16) -> Result<Range<usize>, SearchError> {
        let count = self.quran.part_count(Part::Juz);
        let start = |n: u16| {
            self.quran
                .part_start(Part::Juz, n)
                .and_then(|r| self.quran.global_index(r))
                .map(usize::from)
                .ok_or(SearchError::JuzOutOfRange { number: n, count })
        };
        let end = if to == count {
            usize::from(AYAH_COUNT)
        } else {
            start(to.saturating_add(1))
                .map_err(|_| SearchError::JuzOutOfRange { number: to, count })?
        };
        Ok(start(from)?..end)
    }
}

/// One searchable text, normalized: all verses in one string, each followed by `\n`, so a
/// search scans one contiguous slice and a phrase never runs into the next verse.
struct SearchText<'a> {
    id: String,
    /// Unmodified verse strings, for mapping matches back.
    verses: Vec<&'a str>,
    normalized: String,
    /// Byte offset of each verse in `normalized`, then its length.
    starts: Vec<u32>,
}

impl<'a> SearchText<'a> {
    fn new(id: &str, verses: Vec<&'a str>) -> Self {
        let size: usize = verses.iter().map(|v| v.len()).sum();
        let mut normalized = String::with_capacity(size);
        let mut starts = Vec::with_capacity(verses.len() + 1);
        for v in &verses {
            starts.push(normalized.len() as u32);
            normalize::normalize(v, &mut normalized, None);
            normalized.push('\n');
        }
        starts.push(normalized.len() as u32);
        Self {
            id: id.to_owned(),
            verses,
            normalized,
            starts,
        }
    }

    fn start(&self, verse: usize) -> usize {
        self.starts[verse] as usize
    }

    /// Normalized form of verse `i`, without its `\n`.
    fn verse(&self, i: usize) -> &str {
        &self.normalized[self.start(i)..self.start(i + 1) - 1]
    }

    /// Global indices of the matching verses in `scope`, in order.
    fn matching(&self, query: &Query, scope: &[Range<usize>]) -> Vec<u16> {
        // Scan for the longest word (usually the rarest), then check the others per verse.
        let Some(lead) = query.include.iter().max_by_key(|t| t.text.len()) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for range in scope {
            let (mut at, end) = (self.start(range.start), self.start(range.end));
            let mut v = range.start;
            while let Some(p) = lead.find(&self.normalized, at, end) {
                while self.start(v + 1) <= p {
                    v += 1;
                }
                let verse = self.verse(v);
                let matches = query
                    .include
                    .iter()
                    .all(|t| std::ptr::eq(t, lead) || t.occurs_in(verse))
                    && !query.exclude.iter().any(|t| t.occurs_in(verse));
                if matches {
                    out.push(v as u16);
                }
                at = self.start(v + 1);
            }
        }
        out
    }

    /// Where `terms` occur in verse `v`, as ranges of its unmodified text.
    fn ranges(&self, v: usize, terms: &[Term]) -> Vec<TextRange> {
        let original = self.verses[v];
        let (mut normalized, mut origin) = (String::new(), Vec::new());
        normalize::normalize(original, &mut normalized, Some(&mut origin));
        debug_assert_eq!(normalized, self.verse(v));

        let mut bytes: Vec<(usize, usize)> = Vec::new();
        for term in terms {
            let mut at = 0;
            while let Some(p) = term.find(&normalized, at, normalized.len()) {
                let end = p + term.text.len();
                bytes.push((origin[p].0 as usize, origin[end - 1].1 as usize));
                at = end;
            }
        }
        bytes.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::with_capacity(bytes.len());
        for (s, e) in bytes {
            match merged.last_mut() {
                Some(last) if s <= last.1 => last.1 = last.1.max(e),
                _ => merged.push((s, e)),
            }
        }
        utf16_ranges(original, &merged)
    }
}

/// Byte ranges (sorted, disjoint, on char boundaries) as UTF-16 ranges.
fn utf16_ranges(text: &str, bytes: &[(usize, usize)]) -> Vec<TextRange> {
    let (mut pos, mut units) = (0, 0u32);
    let mut to_units = |b: usize| {
        units += text[pos..b].encode_utf16().count() as u32;
        pos = b;
        units
    };
    bytes
        .iter()
        .map(|&(s, e)| TextRange {
            start: to_units(s),
            end: to_units(e),
        })
        .collect()
}

/// Sorts and merges overlapping or touching ranges.
fn merge(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.sort_unstable_by_key(|r| (r.start, r.end));
    let mut out: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for r in ranges {
        match out.last_mut() {
            Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
            _ => out.push(r),
        }
    }
    out
}

/// Intersection of two sorted, disjoint range lists.
fn intersect(a: &[Range<usize>], b: &[Range<usize>]) -> Vec<Range<usize>> {
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < a.len() && j < b.len() {
        let start = a[i].start.max(b[j].start);
        let end = a[i].end.min(b[j].end);
        if start < end {
            out.push(start..end);
        }
        if a[i].end < b[j].end {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// A word or phrase, normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Term {
    text: String,
    /// Latin terms match at the start of a word only, Arabic ones anywhere.
    word_start: bool,
}

impl Term {
    /// `None` if nothing is left after normalizing (e.g. only tashkeel or punctuation).
    fn new(raw: &str) -> Option<Self> {
        let mut text = String::new();
        normalize::normalize(raw, &mut text, None);
        let first = text.chars().next()?;
        Some(Self {
            word_start: !normalize::is_arabic_block(first),
            text,
        })
    }

    /// First match in `hay[from..to]`, as an offset into `hay`.
    fn find(&self, hay: &str, mut from: usize, to: usize) -> Option<usize> {
        while from < to {
            let p = from + hay[from..to].find(self.text.as_str())?;
            if !self.word_start || p == 0 || matches!(hay.as_bytes()[p - 1], b' ' | b'\n') {
                return Some(p);
            }
            from = p + self.text.chars().next().map_or(1, char::len_utf8);
        }
        None
    }

    fn occurs_in(&self, verse: &str) -> bool {
        self.find(verse, 0, verse.len()).is_some()
    }
}

/// A search query, parsed but not yet checked against the data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Query {
    include: Vec<Term>,
    exclude: Vec<Term>,
    scopes: Vec<Scope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Scope {
    /// Surahs `from..=to`.
    Surahs(u16, u16),
    SurahName(String),
    /// Juz `from..=to`.
    Juz(u16, u16),
}

const OPEN_QUOTES: &[char] = &['"', '“', '„', '«'];
const CLOSE_QUOTES: &[char] = &['"', '”', '“', '»'];

impl Query {
    fn parse(input: &str) -> Result<Self, SearchError> {
        let mut query = Self::default();
        let mut rest = input.trim_start();
        while !rest.is_empty() {
            let (negated, body) = match rest.strip_prefix('-') {
                Some(body) => (true, body),
                None => (false, rest),
            };
            let (token, quoted, after) = match body.strip_prefix(OPEN_QUOTES) {
                Some(inner) => match inner.find(CLOSE_QUOTES) {
                    Some(end) => {
                        let close = inner[end..].chars().next().map_or(1, char::len_utf8);
                        (&inner[..end], true, &inner[end + close..])
                    }
                    // An unclosed quote takes the rest of the input.
                    None => (inner, true, ""),
                },
                None => {
                    let end = body.find(char::is_whitespace).unwrap_or(body.len());
                    (&body[..end], false, &body[end..])
                }
            };
            rest = after.trim_start();
            if !quoted && let Some(scope) = Scope::parse(token)? {
                if negated {
                    return Err(SearchError::InvalidScope(format!("-{token}")));
                }
                query.scopes.push(scope);
                continue;
            }
            if let Some(term) = Term::new(token) {
                if negated {
                    query.exclude.push(term);
                } else {
                    query.include.push(term);
                }
            }
        }
        if query.include.is_empty() {
            return Err(if query.exclude.is_empty() && query.scopes.is_empty() {
                SearchError::Empty
            } else {
                SearchError::NothingToFind
            });
        }
        Ok(query)
    }
}

impl Scope {
    /// `Some` if `token` is `keyword:value` with a scope keyword, `None` if it is a word.
    fn parse(token: &str) -> Result<Option<Self>, SearchError> {
        let Some((keyword, value)) = token.split_once(':') else {
            return Ok(None);
        };
        let surah = match keyword.to_ascii_lowercase().as_str() {
            "surah" | "sura" | "s" => true,
            "juz" | "j" => false,
            _ => return Ok(None),
        };
        let invalid = || SearchError::InvalidScope(token.to_owned());
        let value: String = value.chars().map(ascii_digit).collect();
        let number = |s: &str| s.trim().parse::<u16>().map_err(|_| invalid());
        let numeric = !value.is_empty()
            && value.chars().any(|c| c.is_ascii_digit())
            && value
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '-' | '–'));
        if numeric {
            let (from, to) = match value.split_once(['-', '–']) {
                Some((a, b)) => (number(a)?, number(b)?),
                None => (number(&value)?, number(&value)?),
            };
            let (from, to) = (from.min(to), from.max(to));
            return Ok(Some(if surah {
                Scope::Surahs(from, to)
            } else {
                Scope::Juz(from, to)
            }));
        }
        if surah && !value.trim().is_empty() {
            return Ok(Some(Scope::SurahName(value)));
        }
        Err(invalid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(s: &str) -> Term {
        Term::new(s).unwrap()
    }

    #[test]
    fn parses_words_phrases_and_exclusions() {
        let q = Query::parse(r#"mercy "the  Day" -fire -"hell fire""#).unwrap();
        assert_eq!(q.include, vec![term("mercy"), term("the day")]);
        assert_eq!(q.exclude, vec![term("fire"), term("hell fire")]);
        assert!(q.scopes.is_empty());
        // Other quote styles, and an unclosed quote running to the end.
        let q = Query::parse("“a b” «c d» „e f“ \"g h").unwrap();
        let phrases: Vec<_> = q.include.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(phrases, ["a b", "c d", "e f", "g h"]);
        // A hyphen inside a word is not an exclusion; the word becomes a phrase.
        assert_eq!(
            Query::parse("al-kitab").unwrap().include,
            vec![term("al kitab")]
        );
    }

    #[test]
    fn parses_scopes() {
        let q = Query::parse("x surah:2 S:3-5 juz:٣٠ j:28–29 sura:al-baqarah").unwrap();
        assert_eq!(
            q.scopes,
            vec![
                Scope::Surahs(2, 2),
                Scope::Surahs(3, 5),
                Scope::Juz(30, 30),
                Scope::Juz(28, 29),
                Scope::SurahName("al-baqarah".into()),
            ]
        );
        assert_eq!(
            Query::parse("x surah:5-2").unwrap().scopes,
            vec![Scope::Surahs(2, 5)]
        );
        // Not a scope keyword: just words.
        assert_eq!(
            Query::parse("time:now").unwrap().include,
            vec![term("time now")]
        );
    }

    #[test]
    fn rejects_queries_without_words() {
        assert_eq!(Query::parse("  "), Err(SearchError::Empty));
        assert_eq!(Query::parse("\u{064E}\u{0651} ,"), Err(SearchError::Empty));
        assert_eq!(Query::parse("-fire"), Err(SearchError::NothingToFind));
        assert_eq!(Query::parse("juz:30"), Err(SearchError::NothingToFind));
        for bad in [
            "x juz:",
            "x juz:abc",
            "x surah:",
            "x juz:1-",
            "x -juz:3",
            "x surah:99999",
        ] {
            assert!(
                matches!(Query::parse(bad), Err(SearchError::InvalidScope(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn latin_terms_match_at_word_starts_only() {
        let t = term("in");
        assert_eq!(t.find("within in", 0, 9), Some(7));
        assert!(!t.occurs_in("within"));
        assert!(t.occurs_in("inner"));
        // Arabic terms match inside words.
        assert!(term("رحمن").occurs_in("الرحمن"));
    }

    #[test]
    fn utf16_offsets_count_surrogate_pairs() {
        let text = "a😀bc";
        assert_eq!(
            utf16_ranges(text, &[(0, 1), (5, 7)]),
            [
                TextRange { start: 0, end: 1 },
                TextRange { start: 3, end: 5 }
            ]
        );
        assert_eq!(TextRange { start: 3, end: 5 }.to_bytes(text), 5..7);
        assert_eq!(TextRange { start: 1, end: 3 }.to_bytes(text), 1..5);
    }

    #[test]
    fn merges_and_intersects_ranges() {
        assert_eq!(merge(vec![5..9, 0..3, 2..4, 9..10]), vec![0..4, 5..10]);
        let (a, b) = ([0..10, 20..30], [5..25, 40..50]);
        assert_eq!(intersect(&a, &b), vec![5..10, 20..25]);
        assert_eq!(intersect(&a[..1], &a[1..]), vec![]);
    }

    // --- against the embedded text -----------------------------------------------------

    fn index() -> &'static SearchIndex<'static> {
        crate::search_index().expect("data/quran.bin missing — run `cargo xtask build`")
    }

    fn refs(query: &str) -> Vec<VerseRef> {
        let results = index().search(query, usize::MAX).unwrap();
        assert_eq!(results.total, results.hits.len());
        results.hits.iter().map(|h| h.verse_ref).collect()
    }

    fn normalized(s: &str) -> String {
        let mut out = String::new();
        normalize::normalize(s, &mut out, None);
        out
    }

    /// Words of a verse as written in the data, so no Quran text is typed into the tests.
    fn words(r: VerseRef) -> Vec<&'static str> {
        crate::quran()
            .unwrap()
            .text(r)
            .unwrap()
            .split(' ')
            .collect()
    }

    #[test]
    fn finds_words_regardless_of_tashkeel_and_letter_variants() {
        let plain = refs("الله");
        assert!(plain.len() > 1500, "{}", plain.len());
        assert_eq!(plain[0], VerseRef::new(1, 1));
        // The same word as written in 1:1, with all its marks.
        assert_eq!(refs(words(VerseRef::new(1, 1))[1]), plain);
        // Hamza seats and alif variants meet.
        assert_eq!(refs("آمنوا"), refs("ءامنوا"));
        assert_eq!(refs("أنزل"), refs("انزل"));
    }

    #[test]
    fn phrases_need_adjacent_words_in_order() {
        let w = words(VerseRef::new(1, 2));
        let phrase = refs(&format!("\"{} {}\"", w[0], w[1]));
        assert!(phrase.contains(&VerseRef::new(1, 2)));
        let both = refs(&format!("{} {}", w[0], w[1]));
        let reversed = refs(&format!("\"{} {}\"", w[1], w[0]));
        assert!(phrase.len() <= both.len());
        assert!(phrase.iter().all(|r| both.contains(r)));
        assert!(!reversed.contains(&VerseRef::new(1, 2)));
    }

    #[test]
    fn exclusions_remove_verses() {
        let all = refs("الله");
        let without = refs("الله -رحيم");
        let with = refs("الله رحيم");
        assert!(!with.is_empty());
        assert_eq!(without.len() + with.len(), all.len());
        let quran = crate::quran().unwrap();
        for r in &without {
            assert!(!normalized(quran.text(*r).unwrap()).contains("رحيم"), "{r}");
        }
    }

    #[test]
    fn scopes_limit_the_search() {
        let quran = crate::quran().unwrap();
        let all = refs("الله");
        let baqara = refs("الله surah:2");
        assert!(!baqara.is_empty());
        assert_eq!(
            baqara,
            all.iter()
                .copied()
                .filter(|r| r.surah == 2)
                .collect::<Vec<_>>()
        );
        assert_eq!(refs("الله surah:baqara"), baqara);
        assert_eq!(refs("الله surah:2-3 surah:3"), refs("الله s:2-3"));
        let amma = refs("الله juz:30");
        assert!(!amma.is_empty());
        assert!(amma.iter().all(|r| quran.ayah(*r).unwrap().juz == 30));
        assert_eq!(
            amma.len(),
            all.iter()
                .filter(|r| quran.ayah(**r).unwrap().juz == 30)
                .count()
        );
        // Both kinds: surah 2 within juz 1 ends at 2:141.
        let both = refs("الله surah:2 juz:1");
        assert!(both.iter().all(|r| r.surah == 2 && r.ayah <= 141));
        assert_eq!(refs("الله juz:28-30 juz:29"), refs("الله juz:28-30"));
        let search = |q| index().search(q, 1).map(|_| ());
        assert_eq!(
            search("الله surah:115"),
            Err(SearchError::SurahOutOfRange(115))
        );
        assert_eq!(
            search("الله juz:31"),
            Err(SearchError::JuzOutOfRange {
                number: 31,
                count: 30
            })
        );
        assert!(matches!(
            search("الله surah:xyzzy"),
            Err(SearchError::UnknownSurah(_))
        ));
    }

    #[test]
    fn ranges_map_back_onto_the_unmodified_text() {
        let quran = crate::quran().unwrap();
        let w = words(VerseRef::new(2, 255));
        for query in [
            "الله",
            "رحمن",
            "م",
            "قال",
            &format!("\"{} {}\"", w[2], w[3]),
        ] {
            let expected: Vec<String> = Query::parse(query)
                .unwrap()
                .include
                .into_iter()
                .map(|t| t.text)
                .collect();
            let results = index().search(query, usize::MAX).unwrap();
            assert!(results.total > 0, "{query}");
            for hit in &results.hits {
                let text = quran.text(hit.verse_ref).unwrap();
                let [matches] = hit.matches.as_slice() else {
                    panic!("one text")
                };
                assert_eq!(matches.text, QURAN_TEXT);
                assert!(!matches.ranges.is_empty(), "{query} {}", hit.verse_ref);
                let mut previous_end = 0;
                for range in &matches.ranges {
                    assert!(range.start >= previous_end && range.start < range.end);
                    previous_end = range.end;
                    let bytes = range.to_bytes(text);
                    let slice = &text[bytes.clone()];
                    // The slice is the match: normalized, it holds the term, and nothing
                    // more than a single match of an Arabic term.
                    let n = normalized(slice);
                    assert!(
                        expected.iter().any(|t| n.contains(t.as_str())),
                        "{query} {}: {slice:?}",
                        hit.verse_ref
                    );
                    // It never cuts a letter from its marks.
                    let next = text[bytes.end..].chars().next();
                    assert!(
                        next.is_none_or(|c| normalize::search(c) != normalize::Fold::Drop
                            || c == 'ء'
                            || text[..bytes.end].ends_with(' ')),
                        "{query} {}: mark after {slice:?}",
                        hit.verse_ref
                    );
                }
            }
        }
    }

    #[test]
    fn limit_keeps_the_total() {
        let results = index().search("الله", 5).unwrap();
        assert_eq!(results.hits.len(), 5);
        assert!(results.total > 1500);
        assert_eq!(index().search("الله", 0).unwrap().hits.len(), 0);
    }

    // --- a further text (translations from Phase 4), with synthetic strings -------------

    fn synthetic() -> Vec<String> {
        let mut verses: Vec<String> = (0..AYAH_COUNT).map(|i| format!("line {i}")).collect();
        verses[0] = "In the Name of the Café — Straße".into();
        verses[10] = "😀 Mercy, and more MERCIFUL mercy".into();
        verses[20] = "within the garden of rivers".into();
        verses[6235] = "the garden; no rivers".into();
        verses
    }

    fn with_synthetic(verses: &[String]) -> SearchIndex<'_> {
        let mut index = SearchIndex::new(*crate::quran().unwrap());
        index
            .add_text("test", verses.iter().map(String::as_str).collect())
            .unwrap();
        index
    }

    fn slices(index: &SearchIndex<'_>, verses: &[String], query: &str) -> Vec<(u16, Vec<String>)> {
        let results = index.search(query, usize::MAX).unwrap();
        results
            .hits
            .iter()
            .flat_map(|h| h.matches.iter().map(move |m| (h, m)))
            .filter(|(_, m)| m.text == "test")
            .map(|(h, m)| {
                let text = &verses[usize::from(h.index)];
                let found = m.ranges.iter().map(|r| text[r.to_bytes(text)].to_owned());
                (h.index, found.collect())
            })
            .collect()
    }

    #[test]
    fn latin_text_folds_case_and_diacritics() {
        let verses = synthetic();
        let index = with_synthetic(&verses);
        assert_eq!(
            slices(&index, &verses, "merc"),
            [(10, vec!["Merc".into(), "MERC".into(), "merc".into()])]
        );
        assert_eq!(slices(&index, &verses, "cafe"), [(0, vec!["Café".into()])]);
        assert_eq!(
            slices(&index, &verses, "STRASSE"),
            [(0, vec!["Straße".into()])]
        );
        // UTF-16 offsets past an emoji.
        let hit = index.search("merciful", 1).unwrap().hits.remove(0);
        assert_eq!(hit.matches[0].ranges, [TextRange { start: 19, end: 27 }]);
    }

    #[test]
    fn latin_text_phrases_exclusions_and_word_starts() {
        let verses = synthetic();
        let index = with_synthetic(&verses);
        assert_eq!(
            slices(&index, &verses, "in"),
            [(0, vec!["In".into()])],
            "not inside `within`"
        );
        let garden: Vec<u16> = slices(&index, &verses, "\"the garden\"")
            .into_iter()
            .map(|(i, _)| i)
            .collect();
        assert_eq!(garden, [20, 6235]);
        let only = slices(&index, &verses, "garden -within");
        assert_eq!(only, [(6235, vec!["garden".into()])]);
        // A phrase does not run across punctuation into the next sentence part only by
        // accident: punctuation is a word break, so `garden no` still matches.
        assert_eq!(slices(&index, &verses, "\"garden no\"").len(), 1);
        // Scopes apply to every text.
        assert_eq!(slices(&index, &verses, "rivers surah:114").len(), 1);
        // Arabic queries do not match the Latin text, Latin ones not the Arabic.
        assert!(slices(&index, &verses, "الله").is_empty());
        let results = index.search("garden", usize::MAX).unwrap();
        assert!(
            results
                .hits
                .iter()
                .all(|h| h.matches.iter().all(|m| m.text == "test"))
        );
    }

    #[test]
    fn added_texts_need_one_string_per_verse() {
        let mut index = SearchIndex::new(*crate::quran().unwrap());
        assert_eq!(
            index.add_text("short", vec!["a"; 3]),
            Err(SearchError::VerseCount {
                text: "short".into(),
                count: 3
            })
        );
    }
}
