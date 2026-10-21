//! Tauri shell. Commands stay thin and delegate to `quran-core`.

use quran_core::{CoreInfo, Surah, SurahText};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![core_info, surahs, surah])
        .run(tauri::generate_context!())
        .expect("error while running Quran Reader");
}
