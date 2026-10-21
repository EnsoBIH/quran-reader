//! Reference parser for the ⌘K palette.
//!
//! Understands numeric references (`2:255`, `2 255`, `2.255`) and checks them against the
//! verse count of each surah. Surah names, Arabic input, typos and named verses
//! (`Ayat al-Kursi`) follow with the ⌘K palette.

use std::fmt;

use crate::{Quran, SURAH_COUNT, VerseRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    Invalid(String),
    SurahOutOfRange(u16),
    AyahZero,
    AyahOutOfRange { surah: u16, ayah: u16, count: u16 },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty reference"),
            Self::Invalid(s) => write!(f, "not a reference: {s:?}"),
            Self::SurahOutOfRange(n) => write!(f, "surah {n} does not exist (1–{SURAH_COUNT})"),
            Self::AyahZero => write!(f, "verse numbers start at 1"),
            Self::AyahOutOfRange { surah, ayah, count } => {
                write!(f, "surah {surah} has {count} verses, not {ayah}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses `surah:ayah` and checks it against the embedded data. A bare surah number (`36`)
/// means its first verse. Without embedded data only the surah range is checked.
pub fn parse_ref(input: &str) -> Result<VerseRef, ParseError> {
    let r = parse_numeric(input)?;
    match crate::quran() {
        Ok(quran) => quran.check(r),
        Err(_) => Ok(r),
    }
}

impl Quran<'_> {
    /// Like [`parse_ref`], checked against this data.
    pub fn parse_ref(&self, input: &str) -> Result<VerseRef, ParseError> {
        self.check(parse_numeric(input)?)
    }
}

fn parse_numeric(input: &str) -> Result<VerseRef, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::Empty);
    }

    let mut parts = input
        .split(|c: char| c == ':' || c == '.' || c.is_whitespace())
        .filter(|p| !p.is_empty());
    let invalid = || ParseError::Invalid(input.to_owned());

    let surah: u16 = parts
        .next()
        .ok_or_else(invalid)?
        .parse()
        .map_err(|_| invalid())?;
    let ayah: u16 = match parts.next() {
        Some(p) => p.parse().map_err(|_| invalid())?,
        None => 1,
    };
    if parts.next().is_some() {
        return Err(invalid());
    }

    if !(1..=SURAH_COUNT).contains(&surah) {
        return Err(ParseError::SurahOutOfRange(surah));
    }
    if ayah == 0 {
        return Err(ParseError::AyahZero);
    }
    Ok(VerseRef::new(surah, ayah))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numeric_forms() {
        let expected = Ok(VerseRef::new(2, 255));
        assert_eq!(parse_ref("2:255"), expected);
        assert_eq!(parse_ref(" 2 255 "), expected);
        assert_eq!(parse_ref("2.255"), expected);
    }

    #[test]
    fn bare_surah_means_first_verse() {
        assert_eq!(parse_ref("36"), Ok(VerseRef::new(36, 1)));
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(parse_ref(""), Err(ParseError::Empty));
        assert_eq!(parse_ref("0:1"), Err(ParseError::SurahOutOfRange(0)));
        assert_eq!(parse_ref("115"), Err(ParseError::SurahOutOfRange(115)));
        assert_eq!(parse_ref("2:0"), Err(ParseError::AyahZero));
        assert!(matches!(parse_ref("2:255:1"), Err(ParseError::Invalid(_))));
        assert!(matches!(parse_ref("baqara"), Err(ParseError::Invalid(_))));
    }

    #[test]
    fn checks_verse_bounds() {
        let quran = crate::quran().expect("data/quran.bin missing — run `cargo xtask build`");
        assert_eq!(quran.parse_ref("2:286"), Ok(VerseRef::new(2, 286)));
        assert_eq!(quran.parse_ref("114:6"), Ok(VerseRef::new(114, 6)));
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
    }

    #[test]
    fn displays_as_colon_form() {
        assert_eq!(VerseRef::new(2, 255).to_string(), "2:255");
    }
}
