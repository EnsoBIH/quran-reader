//! Binary data format of `data/quran.bin`, written by `cargo xtask build`.
//!
//! The file is read in place: [`Quran::from_bytes`] validates the structure once (sizes,
//! offsets, UTF-8) and afterwards every accessor is a slice or a binary search into the
//! original bytes. Nothing is copied or decompressed.
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! header   magic "QURANBIN" · version u32 · section count u32
//! table    per section: tag [u8; 4] · offset u32 · length u32   (offsets from file start)
//! NOTE     Tanzil copyright block, verbatim (UTF-8)
//! SRCS     SHA-256 of the text source, then of the metadata source (2 × 32 bytes)
//! STRO     string offsets into STRS, u32 × (n + 1)
//! STRS     surah names and Basmala strings (UTF-8, deduplicated)
//! SURA     per surah, 16 bytes: first ayah u16 · ayah count u16 · revelation order u16 ·
//!          revelation u8 (0 Meccan, 1 Medinan) · rukus u8 · name, latin name,
//!          translated name, Basmala: string index u16 each (0xFFFF = none)
//! AYAO     verse offsets into TEXT, u32 × (6,236 + 1)
//! TEXT     verse texts in Quran order, concatenated (UTF-8, unmodified Tanzil text)
//! JUZ_     first verse of each juz, global verse index u16
//! QRTR     first verse of each hizb quarter, global verse index u16
//! PAGE     first verse of each Madina Mushaf page, global verse index u16
//! SJDA     per sajda, 4 bytes: global verse index u16 · kind u8 (0 recommended,
//!          1 obligatory) · reserved u8
//! ```
//!
//! A "global verse index" is the 0-based position of a verse in the whole Quran (1:1 is 0,
//! 114:6 is 6,235).

use std::collections::HashMap;
use std::fmt;

use crate::model::{
    Ayah, Part, QuranText, Revelation, Sajda, SajdaKind, Surah, SurahEntry, SurahText, VerseRef,
};
use crate::reference::ParseError;
use crate::{AYAH_COUNT, SURAH_COUNT};

pub const MAGIC: [u8; 8] = *b"QURANBIN";
pub const VERSION: u32 = 1;

const NOTICE: [u8; 4] = *b"NOTE";
const SOURCES: [u8; 4] = *b"SRCS";
const STRING_OFFSETS: [u8; 4] = *b"STRO";
const STRINGS: [u8; 4] = *b"STRS";
const SURAHS: [u8; 4] = *b"SURA";
const AYAH_OFFSETS: [u8; 4] = *b"AYAO";
const TEXT: [u8; 4] = *b"TEXT";
const JUZ: [u8; 4] = *b"JUZ_";
const QUARTERS: [u8; 4] = *b"QRTR";
const PAGES: [u8; 4] = *b"PAGE";
const SAJDAS: [u8; 4] = *b"SJDA";

const HEADER_LEN: usize = 16;
const TABLE_ENTRY_LEN: usize = 12;
const SURAH_RECORD_LEN: usize = 16;
const SAJDA_RECORD_LEN: usize = 4;
const NO_STRING: u16 = u16::MAX;

/// Why a byte buffer is not a valid data file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    BadMagic,
    UnsupportedVersion(u32),
    MissingSection(&'static str),
    Malformed(&'static str),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a quran-reader data file"),
            Self::UnsupportedVersion(v) => {
                write!(
                    f,
                    "data format version {v} is not supported (expected {VERSION})"
                )
            }
            Self::MissingSection(tag) => write!(f, "data file has no {tag} section"),
            Self::Malformed(what) => write!(f, "malformed data file: {what}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// The whole Quran, read in place from a data file.
#[derive(Debug, Clone, Copy)]
pub struct Quran<'a> {
    notice: &'a str,
    sources: &'a [u8],
    string_offsets: &'a [u8],
    strings: &'a str,
    surahs: &'a [u8],
    ayah_offsets: &'a [u8],
    text: &'a str,
    juz: &'a [u8],
    quarters: &'a [u8],
    pages: &'a [u8],
    sajdas: &'a [u8],
}

/// Surah metadata borrowed from the data file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurahMeta<'a> {
    pub number: u16,
    pub name_arabic: &'a str,
    pub name_latin: &'a str,
    pub name_translated: &'a str,
    pub revelation: Revelation,
    pub revelation_order: u16,
    pub rukus: u8,
    pub ayah_count: u16,
    /// Basmala shown above verse 1. `None` for surah 1 (where it *is* verse 1) and surah 9.
    pub bismillah: Option<&'a str>,
    first_ayah: u16,
}

impl SurahMeta<'_> {
    pub fn to_surah(&self) -> Surah {
        Surah {
            number: self.number,
            name_arabic: self.name_arabic.to_owned(),
            name_latin: self.name_latin.to_owned(),
            name_translated: Some(self.name_translated.to_owned()),
            revelation: self.revelation,
            ayah_count: self.ayah_count,
        }
    }
}

/// A verse with its position markers, borrowed from the data file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AyahMeta<'a> {
    pub verse_ref: VerseRef,
    /// Unmodified Tanzil text.
    pub text: &'a str,
    pub juz: u8,
    pub hizb_quarter: u8,
    pub page: u16,
    pub sajda: Option<SajdaKind>,
}

impl AyahMeta<'_> {
    pub fn to_ayah(&self) -> Ayah {
        Ayah {
            verse_ref: self.verse_ref,
            text: self.text.to_owned(),
            juz: self.juz,
            hizb_quarter: self.hizb_quarter,
            page: self.page,
            sajda: self.sajda,
        }
    }
}

impl<'a> Quran<'a> {
    /// Validates `bytes` and returns a view into them. Runs in O(file size) once (UTF-8 and
    /// offset checks); every later lookup is O(1) or O(log n) and cannot panic.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, FormatError> {
        let sections = Sections::parse(bytes)?;
        let notice = utf8(sections.get(NOTICE, "NOTE")?, "notice is not UTF-8")?;
        let sources = sections.get(SOURCES, "SRCS")?;
        let string_offsets = sections.get(STRING_OFFSETS, "STRO")?;
        let strings = utf8(sections.get(STRINGS, "STRS")?, "strings are not UTF-8")?;
        let surahs = sections.get(SURAHS, "SURA")?;
        let ayah_offsets = sections.get(AYAH_OFFSETS, "AYAO")?;
        let text = utf8(sections.get(TEXT, "TEXT")?, "text is not UTF-8")?;
        let quran = Self {
            notice,
            sources,
            string_offsets,
            strings,
            surahs,
            ayah_offsets,
            text,
            juz: sections.get(JUZ, "JUZ_")?,
            quarters: sections.get(QUARTERS, "QRTR")?,
            pages: sections.get(PAGES, "PAGE")?,
            sajdas: sections.get(SAJDAS, "SJDA")?,
        };
        quran.validate()?;
        Ok(quran)
    }

    fn validate(&self) -> Result<(), FormatError> {
        let malformed = FormatError::Malformed;
        if self.sources.len() != 64 {
            return Err(malformed("SRCS must hold two SHA-256 digests"));
        }
        validate_offsets(self.string_offsets, self.strings, "string offsets")?;
        let string_count = self.string_offsets.len() / 4 - 1;

        if self.surahs.len() != usize::from(SURAH_COUNT) * SURAH_RECORD_LEN {
            return Err(malformed("expected 114 surah records"));
        }
        if self.ayah_offsets.len() != (usize::from(AYAH_COUNT) + 1) * 4 {
            return Err(malformed("expected 6,236 verse offsets"));
        }
        validate_offsets(self.ayah_offsets, self.text, "verse offsets")?;

        let mut next_ayah = 0u16;
        for i in 0..usize::from(SURAH_COUNT) {
            let r = &self.surahs[i * SURAH_RECORD_LEN..(i + 1) * SURAH_RECORD_LEN];
            if read_u16(r, 0) != next_ayah {
                return Err(malformed("surahs are not contiguous"));
            }
            let count = read_u16(r, 1);
            if count == 0 {
                return Err(malformed("surah without verses"));
            }
            next_ayah = next_ayah
                .checked_add(count)
                .ok_or(malformed("verse count overflow"))?;
            if r[6] > 1 {
                return Err(malformed("unknown revelation place"));
            }
            for field in 4..8 {
                let s = read_u16(r, field);
                let optional = field == 7;
                if usize::from(s) >= string_count && !(optional && s == NO_STRING) {
                    return Err(malformed("string index out of range"));
                }
            }
        }
        if next_ayah != AYAH_COUNT {
            return Err(malformed("surah verse counts do not add up to 6,236"));
        }

        // Juz and hizb quarter ordinals are handed out as u8, page numbers as u16.
        validate_starts(self.juz, u8::MAX.into(), "juz starts")?;
        validate_starts(self.quarters, u8::MAX.into(), "hizb quarter starts")?;
        validate_starts(self.pages, AYAH_COUNT.into(), "page starts")?;

        if !self.sajdas.len().is_multiple_of(SAJDA_RECORD_LEN) {
            return Err(malformed("sajda records"));
        }
        let mut previous = None;
        for r in self.sajdas.chunks_exact(SAJDA_RECORD_LEN) {
            let ayah = read_u16(r, 0);
            if ayah >= AYAH_COUNT || previous.is_some_and(|p| p >= ayah) || r[2] > 1 {
                return Err(malformed("sajda records"));
            }
            previous = Some(ayah);
        }
        Ok(())
    }

    /// The Tanzil copyright block, verbatim. Must be shown with the text (credits page).
    pub fn notice(&self) -> &'a str {
        self.notice
    }

    /// SHA-256 of the pinned Tanzil text file the data was built from.
    pub fn text_source_sha256(&self) -> &'a [u8] {
        &self.sources[..32]
    }

    /// SHA-256 of the pinned Tanzil metadata file the data was built from.
    pub fn metadata_source_sha256(&self) -> &'a [u8] {
        &self.sources[32..]
    }

    /// Metadata of surah `number` (1–114).
    pub fn surah(&self, number: u16) -> Option<SurahMeta<'a>> {
        let i = usize::from(number.checked_sub(1)?);
        let r = self
            .surahs
            .get(i * SURAH_RECORD_LEN..(i + 1) * SURAH_RECORD_LEN)?;
        let bismillah = read_u16(r, 7);
        Some(SurahMeta {
            number,
            first_ayah: read_u16(r, 0),
            ayah_count: read_u16(r, 1),
            revelation_order: read_u16(r, 2),
            revelation: if r[6] == 0 {
                Revelation::Meccan
            } else {
                Revelation::Medinan
            },
            rukus: r[7],
            name_arabic: self.string(read_u16(r, 4)),
            name_latin: self.string(read_u16(r, 5)),
            name_translated: self.string(read_u16(r, 6)),
            bismillah: (bismillah != NO_STRING).then(|| self.string(bismillah)),
        })
    }

    /// All 114 surahs in order.
    pub fn surahs(&self) -> impl Iterator<Item = SurahMeta<'a>> + '_ {
        (1..=SURAH_COUNT).filter_map(|n| self.surah(n))
    }

    /// Checks that `r` names an existing verse.
    pub fn check(&self, r: VerseRef) -> Result<VerseRef, ParseError> {
        let surah = self
            .surah(r.surah)
            .ok_or(ParseError::SurahOutOfRange(r.surah))?;
        match r.ayah {
            0 => Err(ParseError::AyahZero),
            a if a > surah.ayah_count => Err(ParseError::AyahOutOfRange {
                surah: r.surah,
                ayah: a,
                count: surah.ayah_count,
            }),
            _ => Ok(r),
        }
    }

    /// 0-based position of `r` in the whole Quran, if the verse exists.
    pub fn global_index(&self, r: VerseRef) -> Option<u16> {
        let surah = self.surah(r.surah)?;
        (1..=surah.ayah_count)
            .contains(&r.ayah)
            .then(|| surah.first_ayah + r.ayah - 1)
    }

    /// Unmodified Tanzil text of verse `r`.
    pub fn text(&self, r: VerseRef) -> Option<&'a str> {
        self.global_index(r).map(|i| self.text_at(i))
    }

    /// Verse `r` with its juz, hizb quarter, page and sajda.
    pub fn ayah(&self, r: VerseRef) -> Option<AyahMeta<'a>> {
        let i = self.global_index(r)?;
        Some(self.ayah_at(r, i))
    }

    /// All verses of surah `number` in order.
    pub fn ayahs(&self, number: u16) -> impl Iterator<Item = AyahMeta<'a>> + '_ {
        let surah = self.surah(number);
        let (first, count) = surah.map_or((0, 0), |s| (s.first_ayah, s.ayah_count));
        (1..=count).map(move |a| self.ayah_at(VerseRef::new(number, a), first + a - 1))
    }

    /// Surah `number` with all its verses, ready to send to the UI.
    pub fn surah_text(&self, number: u16) -> Option<SurahText> {
        let surah = self.surah(number)?;
        Some(SurahText {
            surah: surah.to_surah(),
            bismillah: surah.bismillah.map(str::to_owned),
            ayahs: self.ayahs(number).map(|a| a.to_ayah()).collect(),
        })
    }

    /// The whole text and its markers, ready to send to the UI once at start-up.
    pub fn quran_text(&self) -> QuranText<'a> {
        let indices = |starts: &'a [u8]| (0..starts.len() / 2).map(|i| read_u16(starts, i));
        QuranText {
            surahs: self
                .surahs()
                .map(|s| SurahEntry {
                    surah: s.to_surah(),
                    bismillah: s.bismillah,
                })
                .collect(),
            verses: self.verses().collect(),
            juz_starts: indices(self.juz).collect(),
            quarter_starts: indices(self.quarters).collect(),
            page_starts: indices(self.pages).collect(),
            sajdas: self
                .sajdas
                .chunks_exact(SAJDA_RECORD_LEN)
                .map(|r| Sajda {
                    index: read_u16(r, 0),
                    kind: sajda_kind(r[2]),
                })
                .collect(),
        }
    }

    /// Unmodified Tanzil text of every verse, in Quran order.
    pub fn verses(&self) -> impl ExactSizeIterator<Item = &'a str> + '_ {
        (0..AYAH_COUNT).map(|i| self.text_at(i))
    }

    /// The verse at global index `i` (1:1 is 0).
    pub fn verse_ref_at(&self, i: u16) -> Option<VerseRef> {
        if i >= AYAH_COUNT {
            return None;
        }
        // Validated: surahs are contiguous from verse 0, so the last one starting at or
        // before `i` contains it.
        let (mut lo, mut hi) = (0, usize::from(SURAH_COUNT));
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if read_u16(&self.surahs[mid * SURAH_RECORD_LEN..], 0) <= i {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let first = read_u16(&self.surahs[lo * SURAH_RECORD_LEN..], 0);
        Some(VerseRef::new(lo as u16 + 1, i - first + 1))
    }

    /// How many juz (30), hizbs (60) or pages (604) the data has.
    pub fn part_count(&self, part: Part) -> u16 {
        // Validated: juz and quarter counts fit in u8, page counts in u16.
        match part {
            Part::Juz => (self.juz.len() / 2) as u16,
            Part::Hizb => (self.quarters.len() / 2 / 4) as u16,
            Part::Page => (self.pages.len() / 2) as u16,
        }
    }

    /// First verse of juz, hizb or page `number` (1-based).
    pub fn part_start(&self, part: Part, number: u16) -> Option<VerseRef> {
        if !(1..=self.part_count(part)).contains(&number) {
            return None;
        }
        let i = usize::from(number - 1);
        let start = match part {
            Part::Juz => read_u16(self.juz, i),
            Part::Hizb => read_u16(self.quarters, 4 * i),
            Part::Page => read_u16(self.pages, i),
        };
        self.verse_ref_at(start)
    }

    fn ayah_at(&self, verse_ref: VerseRef, i: u16) -> AyahMeta<'a> {
        // Validated: every list starts at verse 0, so the count is at least 1 and fits.
        let ordinal = |starts: &[u8]| count_starts_le(starts, i);
        AyahMeta {
            verse_ref,
            text: self.text_at(i),
            juz: ordinal(self.juz) as u8,
            hizb_quarter: ordinal(self.quarters) as u8,
            page: ordinal(self.pages) as u16,
            sajda: self.sajda_at(i),
        }
    }

    fn text_at(&self, i: u16) -> &'a str {
        let i = usize::from(i);
        let start = read_u32(self.ayah_offsets, i) as usize;
        let end = read_u32(self.ayah_offsets, i + 1) as usize;
        &self.text[start..end]
    }

    fn string(&self, i: u16) -> &'a str {
        let i = usize::from(i);
        let start = read_u32(self.string_offsets, i) as usize;
        let end = read_u32(self.string_offsets, i + 1) as usize;
        &self.strings[start..end]
    }

    fn sajda_at(&self, i: u16) -> Option<SajdaKind> {
        let n = self.sajdas.len() / SAJDA_RECORD_LEN;
        let (mut lo, mut hi) = (0, n);
        while lo < hi {
            let mid = (lo + hi) / 2;
            let r = &self.sajdas[mid * SAJDA_RECORD_LEN..(mid + 1) * SAJDA_RECORD_LEN];
            match read_u16(r, 0).cmp(&i) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Some(sajda_kind(r[2])),
            }
        }
        None
    }
}

/// One line of the canonical text listing: `surah|ayah|text\n`, with the Basmala of a
/// surah listed as ayah 0. [`crate::TEXT_SHA256`] is the SHA-256 of all lines in Quran
/// order; `cargo xtask verify` computes it from the Tanzil source, `cargo test` from the
/// data file.
pub fn canonical_line(surah: u16, ayah: u16, text: &str) -> String {
    format!("{surah}|{ayah}|{text}\n")
}

impl Quran<'_> {
    /// All lines of the canonical text listing (see [`canonical_line`]).
    pub fn canonical_lines(&self) -> impl Iterator<Item = String> + '_ {
        self.surahs().flat_map(move |s| {
            let bismillah = s.bismillah.map(|b| canonical_line(s.number, 0, b));
            bismillah.into_iter().chain(
                self.ayahs(s.number)
                    .map(move |a| canonical_line(a.verse_ref.surah, a.verse_ref.ayah, a.text)),
            )
        })
    }
}

/// Input for [`encode`], as parsed from the Tanzil sources by `cargo xtask build`.
#[derive(Debug, Clone)]
pub struct Source<'s> {
    pub notice: &'s str,
    pub text_sha256: [u8; 32],
    pub metadata_sha256: [u8; 32],
    pub surahs: Vec<SurahSource<'s>>,
    pub juz: Vec<VerseRef>,
    pub hizb_quarters: Vec<VerseRef>,
    pub pages: Vec<VerseRef>,
    pub sajdas: Vec<(VerseRef, SajdaKind)>,
}

#[derive(Debug, Clone)]
pub struct SurahSource<'s> {
    pub name_arabic: &'s str,
    pub name_latin: &'s str,
    pub name_translated: &'s str,
    pub revelation: Revelation,
    pub revelation_order: u16,
    pub rukus: u8,
    pub bismillah: Option<&'s str>,
    pub ayahs: Vec<&'s str>,
}

/// Why a [`Source`] cannot be encoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    UnknownVerse(VerseRef),
    TooLarge(&'static str),
    Invalid(FormatError),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVerse(r) => write!(f, "metadata refers to unknown verse {r}"),
            Self::TooLarge(what) => write!(f, "{what} does not fit the format"),
            Self::Invalid(e) => write!(f, "encoded data does not validate: {e}"),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Writes the data file. Deterministic: the same source always gives the same bytes.
/// The result is validated with [`Quran::from_bytes`] before it is returned.
pub fn encode(src: &Source<'_>) -> Result<Vec<u8>, EncodeError> {
    let mut strings = StringTable::default();
    let mut surahs = Vec::with_capacity(src.surahs.len() * SURAH_RECORD_LEN);
    let mut ayah_offsets = Vec::new();
    let mut text = String::new();
    let mut first_ayah = 0u16;
    let mut surah_starts = Vec::with_capacity(src.surahs.len());

    for s in &src.surahs {
        let count = u16::try_from(s.ayahs.len()).map_err(|_| EncodeError::TooLarge("surah"))?;
        surah_starts.push((first_ayah, count));
        put_u16(&mut surahs, first_ayah);
        put_u16(&mut surahs, count);
        put_u16(&mut surahs, s.revelation_order);
        surahs.push(match s.revelation {
            Revelation::Meccan => 0,
            Revelation::Medinan => 1,
        });
        surahs.push(s.rukus);
        put_u16(&mut surahs, strings.insert(s.name_arabic)?);
        put_u16(&mut surahs, strings.insert(s.name_latin)?);
        put_u16(&mut surahs, strings.insert(s.name_translated)?);
        let bismillah = s.bismillah.map(|b| strings.insert(b)).transpose()?;
        put_u16(&mut surahs, bismillah.unwrap_or(NO_STRING));
        for ayah in &s.ayahs {
            put_u32(&mut ayah_offsets, offset(text.len())?);
            text.push_str(ayah);
        }
        first_ayah = first_ayah
            .checked_add(count)
            .ok_or(EncodeError::TooLarge("verse count"))?;
    }
    put_u32(&mut ayah_offsets, offset(text.len())?);

    let index = |r: VerseRef| -> Result<u16, EncodeError> {
        let (first, count) = usize::from(r.surah)
            .checked_sub(1)
            .and_then(|i| surah_starts.get(i))
            .copied()
            .ok_or(EncodeError::UnknownVerse(r))?;
        if r.ayah == 0 || r.ayah > count {
            return Err(EncodeError::UnknownVerse(r));
        }
        Ok(first + r.ayah - 1)
    };
    let starts = |refs: &[VerseRef]| -> Result<Vec<u8>, EncodeError> {
        let mut out = Vec::with_capacity(refs.len() * 2);
        for &r in refs {
            put_u16(&mut out, index(r)?);
        }
        Ok(out)
    };
    let mut sajdas = Vec::with_capacity(src.sajdas.len() * SAJDA_RECORD_LEN);
    for &(r, kind) in &src.sajdas {
        put_u16(&mut sajdas, index(r)?);
        sajdas.push(match kind {
            SajdaKind::Recommended => 0,
            SajdaKind::Obligatory => 1,
        });
        sajdas.push(0);
    }

    let mut sources = Vec::with_capacity(64);
    sources.extend_from_slice(&src.text_sha256);
    sources.extend_from_slice(&src.metadata_sha256);
    let (string_offsets, string_bytes) = strings.finish()?;

    let sections: [([u8; 4], Vec<u8>); 11] = [
        (NOTICE, src.notice.as_bytes().to_vec()),
        (SOURCES, sources),
        (STRING_OFFSETS, string_offsets),
        (STRINGS, string_bytes),
        (SURAHS, surahs),
        (AYAH_OFFSETS, ayah_offsets),
        (TEXT, text.into_bytes()),
        (JUZ, starts(&src.juz)?),
        (QUARTERS, starts(&src.hizb_quarters)?),
        (PAGES, starts(&src.pages)?),
        (SAJDAS, sajdas),
    ];

    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    put_u32(&mut out, VERSION);
    put_u32(&mut out, sections.len() as u32);
    let mut position = HEADER_LEN + sections.len() * TABLE_ENTRY_LEN;
    for (tag, body) in &sections {
        position = position.next_multiple_of(4);
        out.extend_from_slice(tag);
        put_u32(&mut out, offset(position)?);
        put_u32(&mut out, offset(body.len())?);
        position += body.len();
    }
    for (_, body) in &sections {
        out.resize(out.len().next_multiple_of(4), 0);
        out.extend_from_slice(body);
    }

    Quran::from_bytes(&out).map_err(EncodeError::Invalid)?;
    Ok(out)
}

#[derive(Default)]
struct StringTable {
    bytes: String,
    offsets: Vec<u32>,
    index: HashMap<String, u16>,
}

impl StringTable {
    fn insert(&mut self, s: &str) -> Result<u16, EncodeError> {
        if let Some(&i) = self.index.get(s) {
            return Ok(i);
        }
        let i = u16::try_from(self.offsets.len())
            .ok()
            .filter(|&i| i != NO_STRING)
            .ok_or(EncodeError::TooLarge("string table"))?;
        self.offsets.push(offset(self.bytes.len())?);
        self.bytes.push_str(s);
        self.index.insert(s.to_owned(), i);
        Ok(i)
    }

    fn finish(self) -> Result<(Vec<u8>, Vec<u8>), EncodeError> {
        let mut offsets = Vec::with_capacity((self.offsets.len() + 1) * 4);
        for o in &self.offsets {
            put_u32(&mut offsets, *o);
        }
        put_u32(&mut offsets, offset(self.bytes.len())?);
        Ok((offsets, self.bytes.into_bytes()))
    }
}

struct Sections<'a> {
    bytes: &'a [u8],
    table: &'a [u8],
}

impl<'a> Sections<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, FormatError> {
        if bytes.len() < HEADER_LEN || bytes[..8] != MAGIC {
            return Err(FormatError::BadMagic);
        }
        let version = read_u32(&bytes[8..12], 0);
        if version != VERSION {
            return Err(FormatError::UnsupportedVersion(version));
        }
        let count = read_u32(&bytes[12..16], 0) as usize;
        let table = count
            .checked_mul(TABLE_ENTRY_LEN)
            .and_then(|len| bytes.get(HEADER_LEN..HEADER_LEN.checked_add(len)?))
            .ok_or(FormatError::Malformed("section table is truncated"))?;
        Ok(Self { bytes, table })
    }

    fn get(&self, tag: [u8; 4], name: &'static str) -> Result<&'a [u8], FormatError> {
        let entry = self
            .table
            .chunks_exact(TABLE_ENTRY_LEN)
            .find(|e| e[..4] == tag)
            .ok_or(FormatError::MissingSection(name))?;
        let start = read_u32(&entry[4..8], 0) as usize;
        let len = read_u32(&entry[8..12], 0) as usize;
        start
            .checked_add(len)
            .and_then(|end| self.bytes.get(start..end))
            .ok_or(FormatError::Malformed("section out of bounds"))
    }
}

fn utf8<'a>(bytes: &'a [u8], what: &'static str) -> Result<&'a str, FormatError> {
    std::str::from_utf8(bytes).map_err(|_| FormatError::Malformed(what))
}

/// `offsets` must be u32 × (n + 1), non-decreasing, starting at 0, ending at `text.len()`,
/// and every offset a char boundary — then slicing `text` with them never panics.
fn validate_offsets(offsets: &[u8], text: &str, what: &'static str) -> Result<(), FormatError> {
    let n = offsets.len() / 4;
    let ok = offsets.len().is_multiple_of(4)
        && n >= 1
        && read_u32(offsets, 0) == 0
        && read_u32(offsets, n - 1) as usize == text.len()
        && (0..n).all(|i| text.is_char_boundary(read_u32(offsets, i) as usize))
        && (1..n).all(|i| read_u32(offsets, i - 1) <= read_u32(offsets, i));
    if ok {
        Ok(())
    } else {
        Err(FormatError::Malformed(what))
    }
}

/// Start lists must hold 1..=`max` entries, begin at verse 0, increase strictly and stay in
/// range.
fn validate_starts(starts: &[u8], max: usize, what: &'static str) -> Result<(), FormatError> {
    let n = starts.len() / 2;
    let ok = starts.len().is_multiple_of(2)
        && (1..=max).contains(&n)
        && read_u16(starts, 0) == 0
        && read_u16(starts, n - 1) < AYAH_COUNT
        && (1..n).all(|i| read_u16(starts, i - 1) < read_u16(starts, i));
    if ok {
        Ok(())
    } else {
        Err(FormatError::Malformed(what))
    }
}

/// Number of starts `<= i`, i.e. the 1-based ordinal of the part containing verse `i`.
fn count_starts_le(starts: &[u8], i: u16) -> usize {
    let (mut lo, mut hi) = (0, starts.len() / 2);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if read_u16(starts, mid) <= i {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

fn sajda_kind(byte: u8) -> SajdaKind {
    if byte == 0 {
        SajdaKind::Recommended
    } else {
        SajdaKind::Obligatory
    }
}

fn read_u16(bytes: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]])
}

fn read_u32(bytes: &[u8], i: usize) -> u32 {
    let b = &bytes[4 * i..4 * i + 4];
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn offset(n: usize) -> Result<u32, EncodeError> {
    u32::try_from(n).map_err(|_| EncodeError::TooLarge("data file"))
}
