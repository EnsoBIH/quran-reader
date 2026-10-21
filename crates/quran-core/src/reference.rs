//! Reference parser for the ⌘K palette.
//!
//! Understands
//! - numeric references: `2:255`, `2 255`, `2.255`, `٢:٢٥٥`, and a bare surah `36`;
//! - surah names with an optional verse: `baqara 255`, `Al-Baqarah 255`, `بقرة`, typos such
//!   as `bakara`, English names (`the cow`) and a few other names (`Bani Isra'il`);
//! - named verses from a small alias table: `Ayat al-Kursi`;
//! - parts of the Madina Mushaf: `juz 30`, `hizb 10`, `page 604`.
//!
//! [`Quran::lookup`] returns ranked hits for the palette, [`Quran::parse_ref`] the best one.
//! Name matching (see `names.rs`) uses internal normalized keys that are never displayed.

use std::fmt;

use crate::model::{Hit, Part, Target};
use crate::names::{self, ALIASES, Keyer, SURAH_NAMES, Score};
use crate::{Quran, SURAH_COUNT, VerseRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    Invalid(String),
    /// No surah or named verse matches the name.
    Unknown(String),
    SurahOutOfRange(u16),
    AyahZero,
    AyahOutOfRange {
        surah: u16,
        ayah: u16,
        count: u16,
    },
    PartOutOfRange {
        part: Part,
        number: u16,
        count: u16,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty reference"),
            Self::Invalid(s) => write!(f, "not a reference: {s:?}"),
            Self::Unknown(s) => write!(f, "no surah or named verse matches {s:?}"),
            Self::SurahOutOfRange(n) => write!(f, "surah {n} does not exist (1–{SURAH_COUNT})"),
            Self::AyahZero => write!(f, "verse numbers start at 1"),
            Self::AyahOutOfRange { surah, ayah, count } => {
                write!(f, "surah {surah} has {count} verses, not {ayah}")
            }
            Self::PartOutOfRange {
                part,
                number,
                count,
            } => write!(f, "{part} {number} does not exist (1–{count})"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses a reference and checks it against the embedded data. A bare surah number (`36`)
/// means its first verse. Without embedded data only numeric references work, and only the
/// surah range is checked.
pub fn parse_ref(input: &str) -> Result<VerseRef, ParseError> {
    match crate::quran() {
        Ok(quran) => quran.parse_ref(input),
        Err(_) => match Query::parse(input)? {
            Query::Numeric { surah, ayah } => {
                check_surah(surah)?;
                match ayah {
                    Some(0) => Err(ParseError::AyahZero),
                    a => Ok(VerseRef::new(surah, a.unwrap_or(1))),
                }
            }
            _ => Err(ParseError::Invalid(input.trim().to_owned())),
        },
    }
}

impl Quran<'_> {
    /// The best match for `input` (see [`Quran::lookup`]).
    pub fn parse_ref(&self, input: &str) -> Result<VerseRef, ParseError> {
        let hits = self.lookup(input, 1)?;
        Ok(hits[0].verse_ref)
    }

    /// Up to `limit` matches for `input`, best first. Never empty when `Ok`. Numeric
    /// references and parts give one hit; names give a ranked list.
    pub fn lookup(&self, input: &str, limit: usize) -> Result<Vec<Hit>, ParseError> {
        let limit = limit.max(1);
        match Query::parse(input)? {
            Query::Numeric { surah, ayah } => {
                check_surah(surah)?;
                let target = if ayah.is_some() {
                    Target::Verse
                } else {
                    Target::Surah
                };
                let verse_ref = self.check(VerseRef::new(surah, ayah.unwrap_or(1)))?;
                Ok(vec![Hit { target, verse_ref }])
            }
            Query::Part { part, number } => {
                let verse_ref =
                    self.part_start(part, number)
                        .ok_or(ParseError::PartOutOfRange {
                            part,
                            number,
                            count: self.part_count(part),
                        })?;
                Ok(vec![Hit {
                    target: Target::Part { part, number },
                    verse_ref,
                }])
            }
            Query::Name { name, ayah } => self.lookup_name(&name, ayah, limit),
        }
    }

    fn lookup_name(
        &self,
        name: &str,
        ayah: Option<u16>,
        limit: usize,
    ) -> Result<Vec<Hit>, ParseError> {
        let mut keyer = Keyer::default();
        let (mut query, mut glued, mut buf) = (Vec::new(), Vec::new(), Vec::new());
        let arabic = keyer.query_keys(name, &mut query, &mut glued);
        let mut best = |candidate: &str, last_word: bool| -> Option<Score> {
            keyer.key(candidate, arabic, last_word, &mut buf);
            let a = names::score(&query, &buf);
            let b = names::score(&glued, &buf);
            a.into_iter().chain(b).min()
        };

        // (score, kind, surah number or alias index)
        let mut ranked: Vec<(Score, u8, usize)> = Vec::new();
        for s in self.surahs() {
            let score = if arabic {
                best(s.name_arabic, false)
                    .into_iter()
                    .chain(best(s.name_arabic, true))
                    .min()
            } else {
                let extra = SURAH_NAMES
                    .iter()
                    .filter(|(n, _)| *n == s.number)
                    .map(|(_, name)| (*name, false));
                [
                    (s.name_latin, false),
                    (s.name_latin, true),
                    (s.name_translated, false),
                ]
                .into_iter()
                .chain(extra)
                .filter_map(|(n, last_word)| best(n, last_word))
                .min()
            };
            if let Some(score) = score {
                ranked.push((score, 0, usize::from(s.number)));
            }
        }
        if ayah.is_none() {
            for (i, alias) in ALIASES.iter().enumerate() {
                if let Some(score) = alias
                    .names
                    .iter()
                    .filter(|n| names::is_arabic(n) == arabic)
                    .filter_map(|n| best(n, false))
                    .min()
                {
                    ranked.push((score, 1, i));
                }
            }
        }
        ranked.sort_unstable();

        let mut hits = Vec::with_capacity(limit.min(ranked.len()));
        let mut first_error = None;
        for &(_, kind, i) in &ranked {
            let hit = if kind == 1 {
                let alias = &ALIASES[i];
                Hit {
                    target: Target::Named {
                        name: alias.display,
                    },
                    verse_ref: alias.verse_ref,
                }
            } else {
                let surah = i as u16;
                match ayah {
                    None => Hit {
                        target: Target::Surah,
                        verse_ref: VerseRef::new(surah, 1),
                    },
                    Some(a) => match self.check(VerseRef::new(surah, a)) {
                        Ok(verse_ref) => Hit {
                            target: Target::Verse,
                            verse_ref,
                        },
                        Err(e) => {
                            first_error.get_or_insert(e);
                            continue;
                        }
                    },
                }
            };
            hits.push(hit);
            if hits.len() == limit {
                break;
            }
        }
        if hits.is_empty() {
            return Err(first_error.unwrap_or_else(|| ParseError::Unknown(name.to_owned())));
        }
        Ok(hits)
    }
}

fn check_surah(surah: u16) -> Result<(), ParseError> {
    if (1..=SURAH_COUNT).contains(&surah) {
        Ok(())
    } else {
        Err(ParseError::SurahOutOfRange(surah))
    }
}

/// The shape of the input, before it is checked against the data.
#[derive(Debug, PartialEq, Eq)]
enum Query {
    Numeric { surah: u16, ayah: Option<u16> },
    Part { part: Part, number: u16 },
    Name { name: String, ayah: Option<u16> },
}

impl Query {
    fn parse(input: &str) -> Result<Self, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::Empty);
        }
        let invalid = || ParseError::Invalid(input.to_owned());
        let normalized: String = input.chars().map(ascii_digit).collect();
        let tokens = tokens(&normalized);
        let number = |t: &str| t.parse::<u16>().map_err(|_| invalid());
        let is_number = |t: &str| t.bytes().all(|b| b.is_ascii_digit());

        let mut rest = &tokens[..];
        if let [keyword, n] = rest
            && !is_number(keyword)
            && is_number(n)
            && let Some(part) = part_keyword(keyword)
        {
            return Ok(Self::Part {
                part,
                number: number(n)?,
            });
        }
        if rest.len() > 1 && is_surah_keyword(rest[0]) {
            rest = &rest[1..];
        }
        if rest.iter().all(|t| is_number(t)) {
            return match rest {
                [s] => Ok(Self::Numeric {
                    surah: number(s)?,
                    ayah: None,
                }),
                [s, a] => Ok(Self::Numeric {
                    surah: number(s)?,
                    ayah: Some(number(a)?),
                }),
                _ => Err(invalid()),
            };
        }
        let (name, ayah) = match rest {
            [name @ .., last] if is_number(last) => (name, Some(number(last)?)),
            _ => (rest, None),
        };
        if name.iter().any(|t| is_number(t)) {
            return Err(invalid());
        }
        if ayah == Some(0) {
            return Err(ParseError::AyahZero);
        }
        Ok(Self::Name {
            name: name.join(" "),
            ayah,
        })
    }
}

/// Arabic-Indic and Persian digits as ASCII.
fn ascii_digit(c: char) -> char {
    match c {
        '٠'..='٩' => char::from(b'0' + (c as u32 - '٠' as u32) as u8),
        '۰'..='۹' => char::from(b'0' + (c as u32 - '۰' as u32) as u8),
        _ => c,
    }
}

/// Splits at whitespace and `: . , / ،`, and between digits and letters (`juz30`).
fn tokens(input: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for word in input.split(|c: char| c.is_whitespace() || matches!(c, ':' | '.' | ',' | '/' | '،'))
    {
        let mut start = 0;
        let mut digits = None;
        for (i, c) in word.char_indices() {
            let d = c.is_ascii_digit();
            if digits.is_some_and(|prev| prev != d) {
                out.push(&word[start..i]);
                start = i;
            }
            digits = Some(d);
        }
        if start < word.len() {
            out.push(&word[start..]);
        }
    }
    out
}

fn part_keyword(token: &str) -> Option<Part> {
    let arabic = names::is_arabic(token);
    let mut key = Vec::new();
    Keyer::default().key(token, arabic, false, &mut key);
    if arabic {
        return match key.as_slice() {
            names::ARABIC_JUZ | names::ARABIC_JUZ_SHORT => Some(Part::Juz),
            names::ARABIC_HIZB => Some(Part::Hizb),
            names::ARABIC_PAGE => Some(Part::Page),
            _ => None,
        };
    }
    // Folded forms (see `names.rs`): page → pagi, Seite → siti, parah → para.
    match key.as_slice() {
        b"juz" | b"para" => Some(Part::Juz),
        b"hizb" => Some(Part::Hizb),
        b"pagi" | b"pg" | b"p" | b"siti" => Some(Part::Page),
        _ => None,
    }
}

fn is_surah_keyword(token: &str) -> bool {
    let arabic = names::is_arabic(token);
    let mut key = Vec::new();
    Keyer::default().key(token, arabic, false, &mut key);
    // Folded forms of surah, sura, surat, suratu and sure (German).
    key == names::ARABIC_SURAH
        || (!arabic && matches!(key.as_slice(), b"sura" | b"surat" | b"suratu" | b"suri"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quran() -> &'static Quran<'static> {
        crate::quran().expect("data/quran.bin missing — run `cargo xtask build`")
    }

    fn parse(input: &str) -> Result<VerseRef, ParseError> {
        quran().parse_ref(input)
    }

    fn top(input: &str) -> Hit {
        quran().lookup(input, 5).unwrap()[0]
    }

    #[test]
    fn parses_numeric_forms() {
        let expected = Ok(VerseRef::new(2, 255));
        assert_eq!(parse_ref("2:255"), expected);
        assert_eq!(parse_ref(" 2 255 "), expected);
        assert_eq!(parse_ref("2.255"), expected);
        assert_eq!(parse_ref("٢:٢٥٥"), expected);
        assert_eq!(parse_ref("surah 2:255"), expected);
    }

    #[test]
    fn bare_surah_means_first_verse() {
        assert_eq!(parse_ref("36"), Ok(VerseRef::new(36, 1)));
        assert_eq!(top("36").target, Target::Surah);
        assert_eq!(top("2:255").target, Target::Verse);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(parse_ref(""), Err(ParseError::Empty));
        assert_eq!(parse_ref("0:1"), Err(ParseError::SurahOutOfRange(0)));
        assert_eq!(parse_ref("115"), Err(ParseError::SurahOutOfRange(115)));
        assert_eq!(parse_ref("2:0"), Err(ParseError::AyahZero));
        assert!(matches!(parse_ref("2:255:1"), Err(ParseError::Invalid(_))));
        assert!(matches!(parse_ref("2 baqara"), Err(ParseError::Invalid(_))));
        assert!(matches!(parse_ref("xyzzy"), Err(ParseError::Unknown(_))));
        assert_eq!(parse_ref("baqara 0"), Err(ParseError::AyahZero));
    }

    #[test]
    fn checks_verse_bounds() {
        assert_eq!(parse("2:286"), Ok(VerseRef::new(2, 286)));
        assert_eq!(parse("114:6"), Ok(VerseRef::new(114, 6)));
        assert_eq!(
            parse_ref("2:287"),
            Err(ParseError::AyahOutOfRange {
                surah: 2,
                ayah: 287,
                count: 286
            })
        );
        assert_eq!(
            parse_ref("1 8"),
            Err(ParseError::AyahOutOfRange {
                surah: 1,
                ayah: 8,
                count: 7
            })
        );
        assert_eq!(
            parse("fatiha 8"),
            Err(ParseError::AyahOutOfRange {
                surah: 1,
                ayah: 8,
                count: 7
            })
        );
    }

    #[test]
    fn parses_surah_names() {
        let kursi = Ok(VerseRef::new(2, 255));
        assert_eq!(parse("baqara 255"), kursi);
        assert_eq!(parse("Al-Baqarah 255"), kursi);
        assert_eq!(parse("al-baqarah:255"), kursi);
        assert_eq!(parse("albaqara 255"), kursi);
        assert_eq!(parse("baqara255"), kursi);
        assert_eq!(parse("Sure Baqara 255"), kursi);
        assert_eq!(parse("the cow 255"), kursi);
        assert_eq!(parse("بقرة"), Ok(VerseRef::new(2, 1)));
        assert_eq!(parse("البقرة ٢٥٥"), kursi);
        assert_eq!(parse("سورة البقرة"), Ok(VerseRef::new(2, 1)));
        assert_eq!(parse("yasin"), Ok(VerseRef::new(36, 1)));
        assert_eq!(parse("Ya-Sin 12"), Ok(VerseRef::new(36, 12)));
        assert_eq!(parse("al imran 7"), Ok(VerseRef::new(3, 7)));
        assert_eq!(parse("ikhlas"), Ok(VerseRef::new(112, 1)));
        assert_eq!(parse("an-nas"), Ok(VerseRef::new(114, 1)));
        assert_eq!(parse("annas"), Ok(VerseRef::new(114, 1)));
        assert_eq!(parse("rahman"), Ok(VerseRef::new(55, 1)));
        assert_eq!(parse("ar-rahman 13"), Ok(VerseRef::new(55, 13)));
        assert_eq!(parse("Bani Israil"), Ok(VerseRef::new(17, 1)));
        assert_eq!(parse("kahf"), Ok(VerseRef::new(18, 1)));
        assert_eq!(top("baqara").target, Target::Surah);
    }

    #[test]
    fn tolerates_typos() {
        assert_eq!(parse("bakara"), Ok(VerseRef::new(2, 1)));
        assert_eq!(parse("bqara 255"), Ok(VerseRef::new(2, 255)));
        assert_eq!(parse("bakarah 255"), Ok(VerseRef::new(2, 255)));
        assert_eq!(parse("fatiah"), Ok(VerseRef::new(1, 1)));
        assert_eq!(parse("ikhlaas"), Ok(VerseRef::new(112, 1)));
        assert_eq!(parse("maryem"), Ok(VerseRef::new(19, 1)));
        assert_eq!(parse("بقره"), Ok(VerseRef::new(2, 1)));
    }

    #[test]
    fn prefixes_rank_shorter_names_first() {
        let hits = quran().lookup("baq", 5).unwrap();
        assert_eq!(hits[0].verse_ref, VerseRef::new(2, 1));
        let hits = quran().lookup("an", 8).unwrap();
        assert!(hits.len() > 3, "{hits:?}");
    }

    #[test]
    fn parses_parts() {
        let part = |part, number| Target::Part { part, number };
        assert_eq!(top("juz 30").target, part(Part::Juz, 30));
        assert_eq!(parse("juz 30"), Ok(VerseRef::new(78, 1)));
        assert_eq!(parse("Juz 1"), Ok(VerseRef::new(1, 1)));
        assert_eq!(parse("juz30"), Ok(VerseRef::new(78, 1)));
        assert_eq!(parse("para 2"), Ok(VerseRef::new(2, 142)));
        assert_eq!(parse("جزء ٣٠"), Ok(VerseRef::new(78, 1)));
        assert_eq!(top("page 604").target, part(Part::Page, 604));
        assert_eq!(parse("page 604"), Ok(VerseRef::new(112, 1)));
        assert_eq!(parse("p. 1"), Ok(VerseRef::new(1, 1)));
        assert_eq!(parse("Seite 2"), Ok(VerseRef::new(2, 1)));
        assert_eq!(top("hizb 10").target, part(Part::Hizb, 10));
        assert_eq!(parse("hizb 1"), Ok(VerseRef::new(1, 1)));
        // Hizb 10 is the second half of juz 5, which starts at 4:24.
        assert_eq!(parse("hizb 10"), Ok(VerseRef::new(4, 88)));
        assert_eq!(parse("hizb 60"), Ok(VerseRef::new(87, 1)));
        assert_eq!(
            parse("juz 31"),
            Err(ParseError::PartOutOfRange {
                part: Part::Juz,
                number: 31,
                count: 30
            })
        );
        assert_eq!(
            parse("page 605"),
            Err(ParseError::PartOutOfRange {
                part: Part::Page,
                number: 605,
                count: 604
            })
        );
        assert!(matches!(
            parse("hizb 0"),
            Err(ParseError::PartOutOfRange { .. })
        ));
    }

    fn assert_alias(input: &str, display: &'static str, verse_ref: VerseRef) {
        let hit = top(input);
        assert_eq!(
            hit,
            Hit {
                target: Target::Named { name: display },
                verse_ref
            },
            "{input}"
        );
    }

    #[test]
    fn alias_ayat_al_kursi() {
        let r = VerseRef::new(2, 255);
        for input in [
            "Ayat al-Kursi",
            "ayatul kursi",
            "ayat ul-kursi",
            "kursi",
            "throne verse",
            "آية الكرسي",
            "ayat al kursy",
        ] {
            assert_alias(input, "Ayat al-Kursi", r);
        }
    }

    #[test]
    fn alias_ayat_ad_dayn() {
        let r = VerseRef::new(2, 282);
        for input in [
            "Ayat ad-Dayn",
            "ayat al-mudayana",
            "debt verse",
            "آية الدين",
        ] {
            assert_alias(input, "Ayat ad-Dayn", r);
        }
    }

    #[test]
    fn alias_amana_ar_rasul() {
        let r = VerseRef::new(2, 285);
        for input in [
            "Amana ar-Rasul",
            "amanar rasul",
            "khawatim al-baqara",
            "آمن الرسول",
        ] {
            assert_alias(input, "Amana ar-Rasul", r);
        }
    }

    #[test]
    fn alias_ayat_al_mubahala() {
        let r = VerseRef::new(3, 61);
        for input in ["Ayat al-Mubahala", "mubahala", "آية المباهلة"] {
            assert_alias(input, "Ayat al-Mubahala", r);
        }
    }

    #[test]
    fn alias_ayat_an_nur() {
        let r = VerseRef::new(24, 35);
        for input in ["Ayat an-Nur", "light verse", "verse of light", "آية النور"] {
            assert_alias(input, "Ayat an-Nur", r);
        }
        // The surah of the same name still comes first for its own name.
        assert_eq!(top("an-nur").target, Target::Surah);
    }

    #[test]
    fn every_alias_points_at_an_existing_verse() {
        for alias in ALIASES {
            assert_eq!(quran().check(alias.verse_ref), Ok(alias.verse_ref));
        }
    }

    #[test]
    fn every_surah_finds_itself_by_name() {
        for s in quran().surahs() {
            for name in [s.name_latin, s.name_arabic] {
                assert_eq!(
                    parse(name),
                    Ok(VerseRef::new(s.number, 1)),
                    "surah {} by {name:?}",
                    s.number
                );
            }
        }
    }

    #[test]
    fn displays_as_colon_form() {
        assert_eq!(VerseRef::new(2, 255).to_string(), "2:255");
    }
}
