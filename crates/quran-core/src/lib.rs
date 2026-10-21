//! Core of quran-reader: data model, binary format, reference parser, search and local store.
//!
//! Everything here is UI-free and testable with `cargo test`. The desktop app exposes it
//! through Tauri commands; the `quran` binary is a small CLI on top.

pub mod model;
pub mod reference;

pub use model::{CoreInfo, VerseRef};
pub use reference::{ParseError, parse_ref};

/// Number of surahs in the Quran.
pub const SURAH_COUNT: u16 = 114;
/// Number of verses in the Quran (Hafs, Kufan count, as in the Tanzil text).
pub const AYAH_COUNT: u16 = 6236;

/// Crate version, shown on the credits page.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Summary for the UI. `data_loaded` turns true once the Phase 1 data build is wired in.
pub fn info() -> CoreInfo {
    CoreInfo {
        version: VERSION.to_owned(),
        surah_count: SURAH_COUNT,
        ayah_count: AYAH_COUNT,
        data_loaded: false,
    }
}
