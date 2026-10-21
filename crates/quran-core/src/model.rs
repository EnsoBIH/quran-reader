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
