//! Core of quran-reader: data model, binary format, reference parser, search and local store.
//!
//! Everything here is UI-free and testable with `cargo test`. The desktop app exposes it
//! through Tauri commands; the `quran` binary is a small CLI on top.

use std::fmt;
use std::sync::OnceLock;

pub mod format;
pub mod model;
mod names;
pub mod reference;

pub use format::{AyahMeta, FormatError, Quran, SurahMeta};
pub use model::{
    Ayah, CoreInfo, Hit, Part, QuranText, Revelation, Sajda, SajdaKind, Surah, SurahEntry,
    SurahText, Target, VerseRef,
};
pub use reference::{ParseError, parse_ref};

/// Number of surahs in the Quran.
pub const SURAH_COUNT: u16 = 114;
/// Number of verses in the Quran (Hafs, Kufan count, as in the Tanzil text).
pub const AYAH_COUNT: u16 = 6236;

/// SHA-256 (hex) of the canonical text listing (see [`format::canonical_line`]) of the
/// pinned Tanzil Uthmani text. `cargo xtask verify` checks it against the source and
/// `cargo test` against the embedded data, so a changed verse fails both.
pub const TEXT_SHA256: &str = "97354c3ceb4cc240181b120d27b0f65889f25fb5093fc1a77ffe3a840f68435b";

/// Crate version, shown on the credits page.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `data/quran.bin` as of compile time, or empty if it had not been built yet.
static EMBEDDED: &[u8] = include_bytes!(env!("QURAN_DATA_PATH"));

/// Why the embedded data is unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    /// `data/quran.bin` did not exist when the crate was compiled.
    Missing,
    Format(FormatError),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => write!(f, "no Quran data embedded — run `cargo xtask build`"),
            Self::Format(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for DataError {}

/// The embedded Quran data. Validated on first use (well under a millisecond), then shared.
pub fn quran() -> Result<&'static Quran<'static>, DataError> {
    static QURAN: OnceLock<Result<Quran<'static>, DataError>> = OnceLock::new();
    QURAN
        .get_or_init(|| {
            if EMBEDDED.is_empty() {
                Err(DataError::Missing)
            } else {
                Quran::from_bytes(EMBEDDED).map_err(DataError::Format)
            }
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Summary for the UI.
pub fn info() -> CoreInfo {
    CoreInfo {
        version: VERSION.to_owned(),
        surah_count: SURAH_COUNT,
        ayah_count: AYAH_COUNT,
        data_loaded: quran().is_ok(),
    }
}
