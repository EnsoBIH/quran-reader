//! Tauri shell. Commands stay thin and delegate to `quran-core`.

use quran_core::CoreInfo;

#[tauri::command]
fn core_info() -> CoreInfo {
    quran_core::info()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![core_info])
        .run(tauri::generate_context!())
        .expect("error while running Quran Reader");
}
