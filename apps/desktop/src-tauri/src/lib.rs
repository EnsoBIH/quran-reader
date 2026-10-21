//! Tauri shell. Commands stay thin and delegate to `quran-core`.

use std::sync::OnceLock;
use std::time::Instant;

use quran_core::{CoreInfo, Hit, QuranText, Surah, SurahText};

/// When `run` started, for the start-up measurement in [`text_visible`].
static STARTED: OnceLock<Instant> = OnceLock::new();

#[tauri::command]
fn core_info() -> CoreInfo {
    quran_core::info()
}

/// All 114 surahs, for navigation.
#[tauri::command]
fn surahs() -> Result<Vec<Surah>, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    Ok(quran.surahs().map(|s| s.to_surah()).collect())
}

/// One surah with all its verses.
#[tauri::command]
fn surah(number: u16) -> Result<SurahText, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    quran
        .surah_text(number)
        .ok_or_else(|| format!("surah {number} does not exist"))
}

/// The whole text with its markers, loaded once at start-up (≈1.4 MB of JSON, built in
/// under a millisecond; see DECISIONS.md D-012).
#[tauri::command]
fn quran_text() -> Result<QuranText<'static>, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    Ok(quran.quran_text())
}

/// ⌘K palette: ranked matches for a reference, surah name, named verse or part.
#[tauri::command]
fn lookup(query: &str, limit: usize) -> Result<Vec<Hit>, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    quran
        .lookup(query, limit.min(20))
        .map_err(|e| e.to_string())
}

/// Called by the UI once the first verses are painted. Returns milliseconds since start-up
/// and prints them when `QURAN_READER_TIMING` is set (start-up target: < 300 ms).
#[tauri::command]
fn text_visible() -> u64 {
    let ms = STARTED.get().map_or(0, |t| t.elapsed().as_millis() as u64);
    if std::env::var_os("QURAN_READER_TIMING").is_some() {
        eprintln!("text visible after {ms} ms");
    }
    ms
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    STARTED.get_or_init(Instant::now);
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            core_info,
            surahs,
            surah,
            quran_text,
            lookup,
            text_visible
        ])
        .run(tauri::generate_context!())
        .expect("error while running Quran Reader");
}
