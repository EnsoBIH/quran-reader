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
