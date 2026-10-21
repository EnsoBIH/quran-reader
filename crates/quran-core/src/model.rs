//! Domain types. Keep in sync with `packages/types/src/index.ts`.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A verse reference such as `2:255`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VerseRef {
    pub surah: u16,
    pub ayah: u16,
}

impl VerseRef {
    pub const fn new(surah: u16, ayah: u16) -> Self {
        Self { surah, ayah }
    }
}

impl fmt::Display for VerseRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.surah, self.ayah)
    }
}

/// Returned by the `core_info` Tauri command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreInfo {
    pub version: String,
    pub surah_count: u16,
    pub ayah_count: u16,
    pub data_loaded: bool,
}

/// Where a surah was revealed (Tanzil metadata `type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Revelation {
    Meccan,
    Medinan,
}

/// Kind of a prostration (sajda) verse (Tanzil metadata `type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SajdaKind {
    Recommended,
    Obligatory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Surah {
    pub number: u16,
    pub name_arabic: String,
    pub name_latin: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_translated: Option<String>,
    pub revelation: Revelation,
    pub ayah_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ayah {
    #[serde(rename = "ref")]
    pub verse_ref: VerseRef,
    /// Unmodified Tanzil text. Never normalize this for display.
    pub text: String,
    pub juz: u8,
    pub hizb_quarter: u8,
    pub page: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sajda: Option<SajdaKind>,
}

/// Returned by the `surah` Tauri command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurahText {
    pub surah: Surah,
    /// Basmala shown above verse 1. Absent for surah 1 (it is verse 1 there) and surah 9.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bismillah: Option<String>,
    pub ayahs: Vec<Ayah>,
}

/// A division of the Madina Mushaf that can be jumped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Part {
    /// 30 parts.
    Juz,
    /// 60 parts, each four hizb quarters.
    Hizb,
    /// 604 pages.
    Page,
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Juz => "juz",
            Self::Hizb => "hizb",
            Self::Page => "page",
        })
    }
}

/// What a ⌘K palette entry points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    /// A verse, given by number (`2:255`) or by surah name and number (`baqara 255`).
    Verse,
    /// A surah, given by number (`36`) or name (`yasin`), without a verse.
    Surah,
    /// A verse from the alias table, e.g. Ayat al-Kursi. `name` is its display name.
    Named { name: &'static str },
    /// The start of a juz, hizb or page.
    Part { part: Part, number: u16 },
}

/// One ⌘K palette entry: what was matched and where reading starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Hit {
    pub target: Target,
    #[serde(rename = "ref")]
    pub verse_ref: VerseRef,
}

/// A prostration verse, by global verse index (1:1 is 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Sajda {
    pub index: u16,
    pub kind: SajdaKind,
}

/// A surah with its Basmala, as part of [`QuranText`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SurahEntry<'a> {
    #[serde(flatten)]
    pub surah: Surah,
    /// Basmala shown above verse 1. Absent for surah 1 (it is verse 1 there) and surah 9.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bismillah: Option<&'a str>,
}

/// The whole Quran for continuous reading, returned once by the `quran_text` Tauri command.
/// Verses are addressed by global index (1:1 is 0, 114:6 is 6,235).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuranText<'a> {
    pub surahs: Vec<SurahEntry<'a>>,
    /// Unmodified Tanzil text of every verse, in Quran order.
    pub verses: Vec<&'a str>,
    /// Global index of the first verse of each juz.
    pub juz_starts: Vec<u16>,
    /// Global index of the first verse of each hizb quarter (four per hizb).
    pub quarter_starts: Vec<u16>,
    /// Global index of the first verse of each page.
    pub page_starts: Vec<u16>,
    pub sajdas: Vec<Sajda>,
}

/// Returned by the `search` Tauri command: how many verses match, and the first of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    /// All matching verses, also those beyond the limit.
    pub total: usize,
    /// Matching verses in Quran order, at most the limit.
    pub hits: Vec<SearchHit>,
}

/// A verse that matches a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchHit {
    #[serde(rename = "ref")]
    pub verse_ref: VerseRef,
    /// Global verse index (1:1 is 0).
    pub index: u16,
    /// Per searchable text that matched, where.
    pub matches: Vec<TextMatches>,
}

/// Matched ranges in one searchable text of a verse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextMatches {
    /// Which text: [`crate::search::QURAN_TEXT`] for the Arabic, a translation id later.
    pub text: String,
    /// Sorted, non-overlapping ranges of the unmodified text.
    pub ranges: Vec<TextRange>,
}

/// A range of an unmodified text in UTF-16 code units, i.e. JavaScript string indices
/// (`text.slice(start, end)`). Never splits a letter from its tashkeel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TextRange {
    pub start: u32,
    pub end: u32,
}

impl TextRange {
    /// The same range as byte offsets into `text`, for slicing in Rust.
    pub fn to_bytes(self, text: &str) -> std::ops::Range<usize> {
        let (mut units, mut start) = (0, None);
        for (i, c) in text.char_indices() {
            if units == self.start {
                start.get_or_insert(i);
            }
            if units >= self.end {
                return start.unwrap_or(i)..i;
            }
            units += c.len_utf16() as u32;
        }
        start.unwrap_or(text.len())..text.len()
    }
}
