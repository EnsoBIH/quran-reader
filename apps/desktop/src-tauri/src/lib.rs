//! Tauri shell. Commands stay thin and delegate to `quran-core`.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::Instant;

use quran_core::plan::Date;
use quran_core::store::Store;
use quran_core::study::{
    self, Bookmark, Highlight, ImportMode, ImportPreview, Note, PlanStatus, StudyData,
};
use quran_core::{
    CoreInfo, Hit, QuranText, SearchIndex, SearchResults, Surah, SurahText, Translation,
    TranslationInfo, VerseRef,
};
use sha2::{Digest, Sha256};
use tauri::Manager;

/// When `run` started, for the start-up measurement in [`text_visible`].
static STARTED: OnceLock<Instant> = OnceLock::new();

/// Installed translations (bundled ones first, then the user's imports) and the search
/// index over the Arabic and all of them. One lock, so the list and the index never
/// disagree about which translations exist.
struct Library {
    /// Where imports are stored: `<app data>/translations/<id>.bin`.
    dir: Option<PathBuf>,
    state: RwLock<LibraryState>,
}

#[derive(Default)]
struct LibraryState {
    translations: Vec<Installed>,
    /// Built on a background thread at start-up (≈3 ms per text), or by the first search.
    index: Option<SearchIndex<'static>>,
}

struct Installed {
    info: TranslationInfo,
    /// Borrowed from the binary for bundled translations, owned for imports.
    verses: Vec<Cow<'static, str>>,
}

impl Installed {
    fn from_file(t: &Translation<'_>, bundled: bool) -> Self {
        Self {
            info: t.info(bundled),
            verses: t.verses().map(|v| Cow::Owned(v.to_owned())).collect(),
        }
    }
}

impl LibraryState {
    /// Adds or replaces a translation, in the index too if it has been built.
    fn insert(&mut self, t: Installed) {
        if let Some(index) = &mut self.index {
            let _ = index.add_owned_text(&t.info.id, owned(&t.verses));
        }
        self.translations.retain(|x| x.info.id != t.info.id);
        self.translations.push(t);
    }

    fn build_index(&mut self) -> Result<&SearchIndex<'static>, String> {
        if self.index.is_none() {
            let quran = quran_core::quran().map_err(|e| e.to_string())?;
            let mut index = SearchIndex::new(*quran);
            for t in &self.translations {
                let result = match all_borrowed(&t.verses) {
                    Some(verses) => index.add_text(&t.info.id, verses),
                    None => index.add_owned_text(&t.info.id, owned(&t.verses)),
                };
                result.map_err(|e| e.to_string())?;
            }
            self.index = Some(index);
        }
        Ok(self.index.as_ref().expect("built above"))
    }
}

fn owned(verses: &[Cow<'static, str>]) -> Vec<String> {
    verses.iter().map(|v| v.clone().into_owned()).collect()
}

fn all_borrowed(verses: &[Cow<'static, str>]) -> Option<Vec<&'static str>> {
    verses
        .iter()
        .map(|v| match v {
            Cow::Borrowed(s) => Some(*s),
            Cow::Owned(_) => None,
        })
        .collect()
}

impl Library {
    /// Bundled translations and the imports found in `dir`. Imports that no longer load
    /// (damaged, or from an incompatible version) are skipped with a message.
    fn load(dir: Option<PathBuf>) -> Self {
        let mut state = LibraryState::default();
        match quran_core::bundled_translations() {
            Ok(bundled) => {
                for t in bundled {
                    state.translations.push(Installed {
                        info: t.info(true),
                        verses: t.verses().map(Cow::Borrowed).collect(),
                    });
                }
            }
            Err(e) => eprintln!("bundled translations: {e}"),
        }
        let mut imported: Vec<Installed> = Vec::new();
        if let Some(entries) = dir.as_ref().and_then(|d| fs::read_dir(d).ok()) {
            for path in entries.filter_map(|e| Some(e.ok()?.path())) {
                if path.extension().is_none_or(|x| x != "bin") {
                    continue;
                }
                let loaded = fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| {
                        let t = Translation::from_bytes(&bytes).map_err(|e| e.to_string())?;
                        Ok(Installed::from_file(&t, false))
                    });
                match loaded {
                    Ok(t) if t.info.id.starts_with("user.") => imported.push(t),
                    Ok(_) => eprintln!("{}: not an import, skipped", path.display()),
                    Err(e) => eprintln!("{}: {e}, skipped", path.display()),
                }
            }
        }
        imported.sort_by(|a, b| a.info.name.cmp(&b.info.name));
        state.translations.extend(imported);
        Self {
            dir,
            state: RwLock::new(state),
        }
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, LibraryState> {
        self.state.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, LibraryState> {
        self.state.write().unwrap_or_else(|e| e.into_inner())
    }

    fn file(&self, id: &str) -> Result<PathBuf, String> {
        let valid = id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        if !valid {
            return Err(format!("invalid translation id {id:?}"));
        }
        let dir = self.dir.as_ref().ok_or("no app data folder")?;
        Ok(dir.join(format!("{id}.bin")))
    }
}

/// The study store (highlights, notes, bookmarks, plan, last position): one SQLite file in
/// the app data folder that never leaves this computer except through the user's export.
struct Study(Mutex<Result<Store<'static>, String>>);

impl Study {
    fn open(path: Option<PathBuf>) -> Self {
        let store = (|| {
            let path = path.ok_or("no app data folder")?;
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            let quran = quran_core::quran().map_err(|e| e.to_string())?;
            Store::open(&path, quran).map_err(|e| format!("{}: {e}", path.display()))
        })();
        if let Err(e) = &store {
            eprintln!("study store: {e}");
        }
        Self(Mutex::new(store))
    }

    fn with<T>(
        &self,
        f: impl FnOnce(&mut Store<'static>) -> Result<T, study::StudyError>,
    ) -> Result<T, String> {
        let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let store = guard.as_mut().map_err(|e| e.clone())?;
        f(store).map_err(|e| e.to_string())
    }
}

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

/// Installed translations: bundled ones first, then the user's imports by name.
#[tauri::command]
fn translations(library: tauri::State<'_, Library>) -> Vec<TranslationInfo> {
    library
        .read()
        .translations
        .iter()
        .map(|t| t.info.clone())
        .collect()
}

/// All 6,236 verses of one translation, in Quran order.
#[tauri::command]
fn translation_text(
    library: tauri::State<'_, Library>,
    id: &str,
) -> Result<Vec<Cow<'static, str>>, String> {
    library
        .read()
        .translations
        .iter()
        .find(|t| t.info.id == id)
        .map(|t| t.verses.clone())
        .ok_or_else(|| format!("translation {id:?} is not installed"))
}

/// Imports a translation file the user picked (Tanzil plain text, `surah|ayah|text` or
/// XML; see docs/translation-import.md). It is validated, stored in the app data folder and
/// never leaves this computer. Replaces an earlier import with the same id.
#[tauri::command]
fn import_translation(
    library: tauri::State<'_, Library>,
    file_name: &str,
    contents: &str,
) -> Result<TranslationInfo, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    let import = quran_core::import::parse(contents, quran).map_err(|e| e.to_string())?;
    let info = import.info(file_name);
    let digest = sha256(contents.as_bytes());
    let bytes = quran_core::translation::encode(&info, digest, &import.verses)
        .map_err(|e| e.to_string())?;
    let path = library.file(&info.id)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    library.write().insert(Installed {
        info: info.clone(),
        verses: import.verses.into_iter().map(Cow::Owned).collect(),
    });
    Ok(info)
}

/// Deletes an imported translation. Bundled ones cannot be removed.
#[tauri::command]
fn remove_translation(library: tauri::State<'_, Library>, id: &str) -> Result<(), String> {
    if !id.starts_with("user.") {
        return Err("only imported translations can be removed".into());
    }
    let path = library.file(id)?;
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("{}: {e}", path.display())),
    }
    let mut state = library.write();
    state.translations.retain(|t| t.info.id != id);
    if let Some(index) = &mut state.index {
        index.remove_text(id);
    }
    Ok(())
}

/// ⌘K palette: ranked matches for a reference, surah name, named verse or part.
#[tauri::command]
fn lookup(query: &str, limit: usize) -> Result<Vec<Hit>, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    quran
        .lookup(query, limit.min(20))
        .map_err(|e| e.to_string())
}

/// ⌘F search over the Arabic and every installed translation: matching verses with their
/// matched ranges in the unmodified texts.
#[tauri::command]
fn search(
    library: tauri::State<'_, Library>,
    query: &str,
    limit: usize,
) -> Result<SearchResults, String> {
    {
        let state = library.read();
        if let Some(index) = &state.index {
            return index
                .search(query, limit.min(1000))
                .map_err(|e| e.to_string());
        }
    }
    let mut state = library.write();
    state
        .build_index()?
        .search(query, limit.min(1000))
        .map_err(|e| e.to_string())
}

/// All highlights, notes, bookmarks and the plan, sorted by verse.
#[tauri::command]
fn study_data(study: tauri::State<'_, Study>) -> Result<StudyData, String> {
    study.with(|s| s.data())
}

/// Sets a verse's highlight colour (1–6), or removes it with `null`.
#[tauri::command]
fn set_highlight(
    study: tauri::State<'_, Study>,
    verse: VerseRef,
    color: Option<u8>,
) -> Result<Option<Highlight>, String> {
    study.with(|s| s.set_highlight(verse, color))
}

/// Sets a verse's note; an empty body removes it.
#[tauri::command]
fn set_note(
    study: tauri::State<'_, Study>,
    verse: VerseRef,
    body: &str,
) -> Result<Option<Note>, String> {
    study.with(|s| s.set_note(verse, body))
}

/// Adds or relabels a bookmark, or removes it.
#[tauri::command]
fn set_bookmark(
    study: tauri::State<'_, Study>,
    verse: VerseRef,
    bookmarked: bool,
    label: Option<&str>,
) -> Result<Option<Bookmark>, String> {
    study.with(|s| {
        if bookmarked {
            s.set_bookmark(verse, label).map(Some)
        } else {
            s.remove_bookmark(verse).map(|()| None)
        }
    })
}

fn date(s: &str) -> Result<Date, study::StudyError> {
    Date::parse(s).map_err(|e| study::StudyError::Invalid(e.to_string()))
}

/// The reading plan with its portions and the plan day of `today` (the UI's local date,
/// `YYYY-MM-DD`), or `null` without a plan.
#[tauri::command]
fn plan_status(study: tauri::State<'_, Study>, today: &str) -> Result<Option<PlanStatus>, String> {
    study.with(|s| {
        let quran = quran_core::quran().map_err(|e| study::StudyError::Invalid(e.to_string()))?;
        s.plan()?
            .map(|p| study::plan_status(quran, &p, date(today)?))
            .transpose()
    })
}

/// Starts a plan of `days` days from `start`, replacing the current one.
#[tauri::command]
fn start_plan(
    study: tauri::State<'_, Study>,
    days: u16,
    start: &str,
    today: &str,
) -> Result<Option<PlanStatus>, String> {
    study.with(|s| s.start_plan(days, date(start)?).map(|_| ()))?;
    plan_status(study, today)
}

#[tauri::command]
fn stop_plan(study: tauri::State<'_, Study>) -> Result<(), String> {
    study.with(|s| s.stop_plan())
}

/// Marks a plan day as read or not.
#[tauri::command]
fn set_plan_day(
    study: tauri::State<'_, Study>,
    day: u16,
    read: bool,
    today: &str,
) -> Result<Option<PlanStatus>, String> {
    study.with(|s| s.set_day_read(day, read).map(|_| ()))?;
    plan_status(study, today)
}

/// The last reading position.
#[tauri::command]
fn position(study: tauri::State<'_, Study>) -> Result<Option<VerseRef>, String> {
    study.with(|s| s.position())
}

#[tauri::command]
fn set_position(study: tauri::State<'_, Study>, verse: VerseRef) -> Result<(), String> {
    study.with(|s| s.set_position(verse))
}

/// Writes the study data to the Downloads folder as `markdown` or `json` and returns the
/// file's path. `exported` is the local date and time shown in the Markdown file.
#[tauri::command]
fn export_study(
    app: tauri::AppHandle,
    study: tauri::State<'_, Study>,
    format: &str,
    exported: &str,
) -> Result<String, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    let data = study.with(|s| s.data())?;
    let (text, ext) = match format {
        "json" => (study::to_json(&data, &quran_core::plan::now_utc()), "json"),
        "markdown" => (study::to_markdown(&data, quran, exported), "md"),
        other => return Err(format!("unknown export format {other:?}")),
    };
    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    let day = exported
        .get(..10)
        .filter(|d| Date::parse(d).is_ok())
        .unwrap_or("export");
    let path = free_path(&dir, &format!("quran-reader-study-{day}"), ext);
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.display().to_string())
}

/// `dir/stem.ext`, or `dir/stem (2).ext` and so on if that exists, so no file is overwritten.
fn free_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut path = dir.join(format!("{stem}.{ext}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}).{ext}"));
        n += 1;
    }
    path
}

/// What importing an export file (JSON or Markdown) would change. Changes nothing.
#[tauri::command]
fn preview_import(
    study: tauri::State<'_, Study>,
    contents: &str,
    mode: ImportMode,
) -> Result<ImportPreview, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    study.with(|s| {
        let file = study::parse_export(contents, quran, &quran_core::plan::now_utc())?;
        s.preview_import(&file, mode)
    })
}

/// Imports an export file, merging it into the study data or replacing it.
#[tauri::command]
fn apply_import(
    study: tauri::State<'_, Study>,
    contents: &str,
    mode: ImportMode,
) -> Result<StudyData, String> {
    let quran = quran_core::quran().map_err(|e| e.to_string())?;
    study.with(|s| {
        let file = study::parse_export(contents, quran, &quran_core::plan::now_utc())?;
        s.import(&file, mode)?;
        s.data()
    })
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

/// SHA-256 of an imported file, kept in its translation file as the source digest.
fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    STARTED.get_or_init(Instant::now);
    tauri::Builder::default()
        .setup(|app| {
            let data = app.path().app_data_dir().ok();
            app.manage(Library::load(data.as_ref().map(|d| d.join("translations"))));
            app.manage(Study::open(data.map(|d| d.join("study.sqlite3"))));
            // Build the search index off the start-up path, so the first ⌘F search is fast.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let library = handle.state::<Library>();
                let _ = library.write().build_index();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core_info,
            surahs,
            surah,
            quran_text,
            translations,
            translation_text,
            import_translation,
            remove_translation,
            lookup,
            search,
            study_data,
            set_highlight,
            set_note,
            set_bookmark,
            plan_status,
            start_plan,
            stop_plan,
            set_plan_day,
            position,
            set_position,
            export_study,
            preview_import,
            apply_import,
            text_visible
        ])
        .run(tauri::generate_context!())
        .expect("error while running Quran Reader");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic import (no translation text), stored the way `import_translation` does.
    fn store(dir: &std::path::Path, word: &str) -> TranslationInfo {
        let quran = quran_core::quran().unwrap();
        let body: Vec<String> = (0..quran_core::AYAH_COUNT)
            .map(|i| format!("synthetic {word} {i}"))
            .collect();
        let file = format!("{}\n\n# Name: Test\n# ID: de.test\n", body.join("\n"));
        let import = quran_core::import::parse(&file, quran).unwrap();
        let info = import.info("de.test.txt");
        let bytes = quran_core::translation::encode(&info, sha256(file.as_bytes()), &import.verses)
            .unwrap();
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(format!("{}.bin", info.id)), bytes).unwrap();
        info
    }

    #[test]
    fn loads_bundled_and_stored_imports_and_searches_them() {
        let dir = std::env::temp_dir().join(format!("quran-reader-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let info = store(&dir, "zebra");
        fs::write(dir.join("broken.bin"), b"not a translation").unwrap();

        let library = Library::load(Some(dir.clone()));
        let ids: Vec<String> = library
            .read()
            .translations
            .iter()
            .map(|t| t.info.id.clone())
            .collect();
        assert_eq!(ids.first().map(String::as_str), Some("en.pickthall"));
        assert_eq!(ids.last(), Some(&info.id));
        assert!(library.read().translations[0].info.bundled);

        let mut state = library.write();
        let hits = state.build_index().unwrap().search("zebra", 5).unwrap();
        assert_eq!(hits.total, usize::from(quran_core::AYAH_COUNT));
        assert_eq!(hits.hits[0].matches[0].text, info.id);

        // Re-import with other text: replaced in the list and the index.
        let verses: Vec<Cow<'static, str>> = (0..quran_core::AYAH_COUNT)
            .map(|i| Cow::Owned(format!("synthetic okapi {i}")))
            .collect();
        state.insert(Installed {
            info: info.clone(),
            verses,
        });
        let index = state.index.as_ref().unwrap();
        assert_eq!(index.search("zebra", 5).unwrap().total, 0);
        assert_eq!(index.search("okapi", 5).unwrap().hits.len(), 5);
        assert_eq!(
            state
                .translations
                .iter()
                .filter(|t| t.info.id == info.id)
                .count(),
            1
        );
        drop(state);
        assert!(library.file("../escape").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
