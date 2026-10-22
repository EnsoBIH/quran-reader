//! The local study store: one SQLite file in the app data folder (DECISIONS.md D-021).
//!
//! Rows are keyed by verse reference (`surah`, `ayah`), never by a translation, so they
//! stay valid whichever translation is shown. The schema version is SQLite's
//! `user_version`; [`MIGRATIONS`] upgrade older files step by step, each in a transaction,
//! and a file from a newer app is refused rather than changed.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::plan::{Date, now_utc};
use crate::study::{
    self, Bookmark, Highlight, ImportMode, ImportPreview, Note, Plan, StudyData, StudyError,
};
use crate::{Quran, VerseRef};

/// Schema migrations; entry *n* upgrades version *n* to *n + 1*. Never edit one that has
/// shipped — add a new one.
pub const MIGRATIONS: &[&str] = &[
    // 1: highlights, notes, bookmarks, the reading plan, and app state (last position).
    "CREATE TABLE highlights (
        surah INTEGER NOT NULL,
        ayah INTEGER NOT NULL,
        color INTEGER NOT NULL CHECK (color BETWEEN 1 AND 6),
        updated_at TEXT NOT NULL,
        PRIMARY KEY (surah, ayah)
    ) WITHOUT ROWID;
    CREATE TABLE notes (
        surah INTEGER NOT NULL,
        ayah INTEGER NOT NULL,
        body TEXT NOT NULL CHECK (body <> ''),
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (surah, ayah)
    ) WITHOUT ROWID;
    CREATE TABLE bookmarks (
        surah INTEGER NOT NULL,
        ayah INTEGER NOT NULL,
        label TEXT,
        created_at TEXT NOT NULL,
        PRIMARY KEY (surah, ayah)
    ) WITHOUT ROWID;
    CREATE TABLE plan (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        days INTEGER NOT NULL,
        start TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE TABLE plan_read (
        day INTEGER PRIMARY KEY,
        read_at TEXT NOT NULL
    );
    CREATE TABLE state (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    ) WITHOUT ROWID;",
];

/// The schema version this build writes.
pub const SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

impl From<rusqlite::Error> for StudyError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e.to_string())
    }
}

type Result<T> = std::result::Result<T, StudyError>;

pub struct Store<'q> {
    conn: Connection,
    quran: &'q Quran<'q>,
}

impl<'q> Store<'q> {
    /// Opens (or creates) the store at `path` and brings its schema up to date.
    pub fn open(path: &Path, quran: &'q Quran<'q>) -> Result<Self> {
        Self::with(Connection::open(path)?, quran)
    }

    /// A store that lives in memory only, for tests.
    pub fn in_memory(quran: &'q Quran<'q>) -> Result<Self> {
        Self::with(Connection::open_in_memory()?, quran)
    }

    fn with(mut conn: Connection, quran: &'q Quran<'q>) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        migrate(&mut conn, MIGRATIONS)?;
        Ok(Self { conn, quran })
    }

    pub fn schema_version(&self) -> Result<u32> {
        Ok(self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    /// Everything, sorted by verse.
    pub fn data(&self) -> Result<StudyData> {
        read_all(&self.conn)
    }

    /// Sets the highlight of a verse, or removes it with `None`.
    pub fn set_highlight(&self, r: VerseRef, color: Option<u8>) -> Result<Option<Highlight>> {
        study::check_ref(self.quran, r)?;
        let Some(color) = color else {
            self.conn.execute(
                "DELETE FROM highlights WHERE surah = ?1 AND ayah = ?2",
                params![r.surah, r.ayah],
            )?;
            return Ok(None);
        };
        study::check_color(color)?;
        let h = Highlight {
            verse: r,
            color,
            updated_at: now_utc(),
        };
        self.conn.execute(
            "INSERT INTO highlights (surah, ayah, color, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (surah, ayah) DO UPDATE SET color = ?3, updated_at = ?4",
            params![r.surah, r.ayah, h.color, h.updated_at],
        )?;
        Ok(Some(h))
    }

    /// Sets the note of a verse; an empty body removes it.
    pub fn set_note(&self, r: VerseRef, body: &str) -> Result<Option<Note>> {
        study::check_ref(self.quran, r)?;
        let body = study::clean_note(body);
        if body.trim().is_empty() {
            self.conn.execute(
                "DELETE FROM notes WHERE surah = ?1 AND ayah = ?2",
                params![r.surah, r.ayah],
            )?;
            return Ok(None);
        }
        if body.len() > study::MAX_NOTE {
            return Err(StudyError::Invalid("the note is too long".into()));
        }
        let now = now_utc();
        self.conn.execute(
            "INSERT INTO notes (surah, ayah, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT (surah, ayah) DO UPDATE SET body = ?3, updated_at = ?4",
            params![r.surah, r.ayah, body, now],
        )?;
        Ok(self
            .conn
            .query_row(
                "SELECT body, created_at, updated_at FROM notes WHERE surah = ?1 AND ayah = ?2",
                params![r.surah, r.ayah],
                |row| {
                    Ok(Note {
                        verse: r,
                        body: row.get(0)?,
                        created_at: row.get(1)?,
                        updated_at: row.get(2)?,
                    })
                },
            )
            .optional()?)
    }

    /// Adds a bookmark or changes its label (`None`: no label).
    pub fn set_bookmark(&self, r: VerseRef, label: Option<&str>) -> Result<Bookmark> {
        study::check_ref(self.quran, r)?;
        let label = study::clean_label(label);
        if label.as_ref().is_some_and(|l| l.len() > study::MAX_LABEL) {
            return Err(StudyError::Invalid("the label is too long".into()));
        }
        self.conn.execute(
            "INSERT INTO bookmarks (surah, ayah, label, created_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (surah, ayah) DO UPDATE SET label = ?3",
            params![r.surah, r.ayah, label, now_utc()],
        )?;
        Ok(self.conn.query_row(
            "SELECT label, created_at FROM bookmarks WHERE surah = ?1 AND ayah = ?2",
            params![r.surah, r.ayah],
            |row| {
                Ok(Bookmark {
                    verse: r,
                    label: row.get(0)?,
                    created_at: row.get(1)?,
                })
            },
        )?)
    }

    pub fn remove_bookmark(&self, r: VerseRef) -> Result<()> {
        self.conn.execute(
            "DELETE FROM bookmarks WHERE surah = ?1 AND ayah = ?2",
            params![r.surah, r.ayah],
        )?;
        Ok(())
    }

    pub fn plan(&self) -> Result<Option<Plan>> {
        read_plan(&self.conn)
    }

    /// Starts a new plan, replacing the current one and its progress.
    pub fn start_plan(&mut self, days: u16, start: Date) -> Result<Plan> {
        let plan = Plan {
            days,
            start: start.to_string(),
            read: Vec::new(),
            created_at: now_utc(),
        };
        let data = StudyData {
            plan: Some(plan.clone()),
            ..StudyData::default()
        };
        data.validate(self.quran)?;
        let tx = self.conn.transaction()?;
        write_plan(&tx, Some(&plan))?;
        tx.commit()?;
        Ok(plan)
    }

    pub fn stop_plan(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        write_plan(&tx, None)?;
        tx.commit()?;
        Ok(())
    }

    /// Marks a plan day as read or unread.
    pub fn set_day_read(&self, day: u16, read: bool) -> Result<Plan> {
        let plan = self
            .plan()?
            .ok_or_else(|| StudyError::Invalid("there is no reading plan".into()))?;
        if day == 0 || day > plan.days {
            return Err(StudyError::Invalid(format!(
                "day {day} is not in 1–{}",
                plan.days
            )));
        }
        if read {
            self.conn.execute(
                "INSERT OR IGNORE INTO plan_read (day, read_at) VALUES (?1, ?2)",
                params![day, now_utc()],
            )?;
        } else {
            self.conn
                .execute("DELETE FROM plan_read WHERE day = ?1", params![day])?;
        }
        self.plan()?
            .ok_or_else(|| StudyError::Invalid("there is no reading plan".into()))
    }

    /// The last reading position.
    pub fn position(&self) -> Result<Option<VerseRef>> {
        let value: Option<String> = self
            .conn
            .query_row("SELECT value FROM state WHERE key = 'position'", [], |r| {
                r.get(0)
            })
            .optional()?;
        let parsed = value.and_then(|v| {
            let (s, a) = v.split_once(':')?;
            Some(VerseRef::new(s.parse().ok()?, a.parse().ok()?))
        });
        Ok(parsed.filter(|r| self.quran.check(*r).is_ok()))
    }

    pub fn set_position(&self, r: VerseRef) -> Result<()> {
        study::check_ref(self.quran, r)?;
        self.conn.execute(
            "INSERT INTO state (key, value) VALUES ('position', ?1)
             ON CONFLICT (key) DO UPDATE SET value = ?1",
            params![r.to_string()],
        )?;
        Ok(())
    }

    /// What importing `file` would change, without changing anything.
    pub fn preview_import(&self, file: &StudyData, mode: ImportMode) -> Result<ImportPreview> {
        Ok(study::merge(&self.data()?, file, mode).1)
    }

    /// Imports `file` (already validated by [`study::parse_export`]) in one transaction.
    pub fn import(&mut self, file: &StudyData, mode: ImportMode) -> Result<ImportPreview> {
        file.validate(self.quran)?;
        let tx = self.conn.transaction()?;
        let (data, preview) = study::merge(&read_all(&tx)?, file, mode);
        write_all(&tx, &data)?;
        tx.commit()?;
        Ok(preview)
    }
}

/// Applies the migrations `user_version` has not seen yet. Refuses a newer file.
pub(crate) fn migrate(conn: &mut Connection, migrations: &[&str]) -> Result<u32> {
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let known = migrations.len() as u32;
    if version > known {
        return Err(StudyError::Newer {
            found: version,
            known,
        });
    }
    for (i, sql) in migrations.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as u32 + 1)?;
        tx.commit()?;
    }
    Ok(known)
}

fn read_all(conn: &Connection) -> Result<StudyData> {
    let at = |row: &rusqlite::Row<'_>| -> rusqlite::Result<VerseRef> {
        Ok(VerseRef::new(row.get(0)?, row.get(1)?))
    };
    let highlights = conn
        .prepare("SELECT surah, ayah, color, updated_at FROM highlights ORDER BY surah, ayah")?
        .query_map([], |row| {
            Ok(Highlight {
                verse: at(row)?,
                color: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let notes = conn
        .prepare(
            "SELECT surah, ayah, body, created_at, updated_at FROM notes ORDER BY surah, ayah",
        )?
        .query_map([], |row| {
            Ok(Note {
                verse: at(row)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let bookmarks = conn
        .prepare("SELECT surah, ayah, label, created_at FROM bookmarks ORDER BY surah, ayah")?
        .query_map([], |row| {
            Ok(Bookmark {
                verse: at(row)?,
                label: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(StudyData {
        highlights,
        notes,
        bookmarks,
        plan: read_plan(conn)?,
    })
}

fn read_plan(conn: &Connection) -> Result<Option<Plan>> {
    let plan = conn
        .query_row(
            "SELECT days, start, created_at FROM plan WHERE id = 1",
            [],
            |r| {
                Ok(Plan {
                    days: r.get(0)?,
                    start: r.get(1)?,
                    read: Vec::new(),
                    created_at: r.get(2)?,
                })
            },
        )
        .optional()?;
    let Some(mut plan) = plan else {
        return Ok(None);
    };
    plan.read = conn
        .prepare("SELECT day FROM plan_read WHERE day BETWEEN 1 AND ?1 ORDER BY day")?
        .query_map([plan.days], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(plan))
}

fn write_plan(tx: &Transaction<'_>, plan: Option<&Plan>) -> Result<()> {
    tx.execute("DELETE FROM plan_read", [])?;
    tx.execute("DELETE FROM plan", [])?;
    if let Some(p) = plan {
        tx.execute(
            "INSERT INTO plan (id, days, start, created_at) VALUES (1, ?1, ?2, ?3)",
            params![p.days, p.start, p.created_at],
        )?;
        let now = now_utc();
        let mut insert = tx.prepare("INSERT INTO plan_read (day, read_at) VALUES (?1, ?2)")?;
        for day in &p.read {
            insert.execute(params![day, now])?;
        }
    }
    Ok(())
}

/// Replaces all study data (not the app state) with `data`.
fn write_all(tx: &Transaction<'_>, data: &StudyData) -> Result<()> {
    tx.execute_batch("DELETE FROM highlights; DELETE FROM notes; DELETE FROM bookmarks;")?;
    let mut h = tx.prepare(
        "INSERT INTO highlights (surah, ayah, color, updated_at) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for x in &data.highlights {
        h.execute(params![x.verse.surah, x.verse.ayah, x.color, x.updated_at])?;
    }
    let mut n = tx.prepare(
        "INSERT INTO notes (surah, ayah, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for x in &data.notes {
        n.execute(params![
            x.verse.surah,
            x.verse.ayah,
            x.body,
            x.created_at,
            x.updated_at
        ])?;
    }
    let mut b = tx.prepare(
        "INSERT INTO bookmarks (surah, ayah, label, created_at) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for x in &data.bookmarks {
        b.execute(params![x.verse.surah, x.verse.ayah, x.label, x.created_at])?;
    }
    write_plan(tx, data.plan.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::study::tests::{quran, sample};
    use crate::study::{Change, ImportMode};

    /// A fresh database file in the temp folder, removed on drop.
    struct TempDb(std::path::PathBuf);

    impl TempDb {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "quran-reader-store-{name}-{}.sqlite3",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            Self(path)
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn creates_schema_and_keeps_data_across_opens() {
        let db = TempDb::new("reopen");
        let r = VerseRef::new(2, 255);
        {
            let mut s = Store::open(&db.0, quran()).unwrap();
            assert_eq!(s.schema_version().unwrap(), SCHEMA_VERSION);
            assert_eq!(s.data().unwrap(), StudyData::default());
            s.set_highlight(r, Some(3)).unwrap();
            s.set_note(r, "Ayat al-Kursi\r\n").unwrap();
            s.set_bookmark(r, Some("  Kursi ")).unwrap();
            s.set_position(VerseRef::new(18, 10)).unwrap();
            s.start_plan(30, Date::parse("2026-10-01").unwrap())
                .unwrap();
            s.set_day_read(2, true).unwrap();
        }
        let s = Store::open(&db.0, quran()).unwrap();
        let d = s.data().unwrap();
        assert_eq!(d.highlights[0].color, 3);
        assert_eq!(d.notes[0].body, "Ayat al-Kursi");
        assert_eq!(d.bookmarks[0].label.as_deref(), Some("Kursi"));
        assert_eq!(d.plan.unwrap().read, vec![2]);
        assert_eq!(s.position().unwrap(), Some(VerseRef::new(18, 10)));
    }

    #[test]
    fn highlights_notes_bookmarks() {
        let s = Store::in_memory(quran()).unwrap();
        let r = VerseRef::new(1, 2);
        s.set_highlight(r, Some(1)).unwrap();
        s.set_highlight(r, Some(6)).unwrap();
        assert_eq!(s.data().unwrap().highlights.len(), 1);
        assert_eq!(s.data().unwrap().highlights[0].color, 6);
        assert!(s.set_highlight(r, Some(7)).is_err());
        assert!(s.set_highlight(VerseRef::new(1, 8), Some(1)).is_err());
        assert!(s.set_highlight(VerseRef::new(115, 1), Some(1)).is_err());
        s.set_highlight(r, None).unwrap();
        assert!(s.data().unwrap().highlights.is_empty());

        let first = s.set_note(r, "one").unwrap().unwrap();
        let second = s.set_note(r, "two").unwrap().unwrap();
        assert_eq!(second.created_at, first.created_at);
        assert_eq!(second.body, "two");
        assert_eq!(s.set_note(r, "  \n ").unwrap(), None);
        assert!(s.data().unwrap().notes.is_empty());

        let b = s.set_bookmark(r, None).unwrap();
        assert_eq!(b.label, None);
        let b2 = s.set_bookmark(r, Some("label")).unwrap();
        assert_eq!(
            (b2.label.as_deref(), b2.created_at),
            (Some("label"), b.created_at)
        );
        s.remove_bookmark(r).unwrap();
        assert!(s.data().unwrap().bookmarks.is_empty());
    }

    #[test]
    fn plan_and_position() {
        let mut s = Store::in_memory(quran()).unwrap();
        assert_eq!(s.position().unwrap(), None);
        assert!(s.set_position(VerseRef::new(2, 287)).is_err());
        s.set_position(VerseRef::new(2, 286)).unwrap();
        s.set_position(VerseRef::new(3, 1)).unwrap();
        assert_eq!(s.position().unwrap(), Some(VerseRef::new(3, 1)));

        assert!(s.set_day_read(1, true).is_err());
        let start = Date::parse("2026-10-01").unwrap();
        assert!(s.start_plan(0, start).is_err());
        s.start_plan(30, start).unwrap();
        s.set_day_read(3, true).unwrap();
        s.set_day_read(1, true).unwrap();
        s.set_day_read(1, true).unwrap();
        assert_eq!(s.plan().unwrap().unwrap().read, vec![1, 3]);
        assert_eq!(s.set_day_read(3, false).unwrap().read, vec![1]);
        assert!(s.set_day_read(31, true).is_err());
        // A new plan starts from scratch.
        s.start_plan(60, start).unwrap();
        let p = s.plan().unwrap().unwrap();
        assert_eq!((p.days, p.read.len()), (60, 0));
        s.stop_plan().unwrap();
        assert_eq!(s.plan().unwrap(), None);
    }

    #[test]
    fn export_import_round_trip() {
        let q = quran();
        let mut a = Store::in_memory(q).unwrap();
        a.import(&sample(), ImportMode::Replace).unwrap();
        assert_eq!(a.data().unwrap(), sample());

        // JSON: lossless through a second store.
        let json = study::to_json(&a.data().unwrap(), "2026-10-22T10:00:00Z");
        let mut b = Store::in_memory(q).unwrap();
        let file = study::parse_export(&json, q, &now_utc()).unwrap();
        let preview = b.preview_import(&file, ImportMode::Merge).unwrap();
        assert_eq!(preview.highlights.added, 2);
        assert_eq!(preview.plan, Some(Change::Added));
        assert_eq!(
            b.data().unwrap(),
            StudyData::default(),
            "preview changes nothing"
        );
        b.import(&file, ImportMode::Merge).unwrap();
        assert_eq!(b.data().unwrap(), a.data().unwrap());

        // Markdown: same content, so importing it back changes nothing.
        let md = study::to_markdown(&a.data().unwrap(), q, "today");
        let file = study::parse_export(&md, q, &now_utc()).unwrap();
        assert!(
            a.preview_import(&file, ImportMode::Replace)
                .unwrap()
                .is_empty()
        );
        let before = a.data().unwrap();
        a.import(&file, ImportMode::Replace).unwrap();
        assert_eq!(a.data().unwrap(), before);

        // Replace with an empty file clears study data but not the position.
        a.set_position(VerseRef::new(5, 5)).unwrap();
        a.import(&StudyData::default(), ImportMode::Replace)
            .unwrap();
        assert_eq!(a.data().unwrap(), StudyData::default());
        assert_eq!(a.position().unwrap(), Some(VerseRef::new(5, 5)));
    }

    #[test]
    fn migrations_run_once_in_order_and_refuse_newer_files() {
        let db = TempDb::new("migrate");
        let v1 = MIGRATIONS[0];
        let v2 = "ALTER TABLE notes ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;";
        {
            let mut conn = Connection::open(&db.0).unwrap();
            assert_eq!(migrate(&mut conn, &[v1]).unwrap(), 1);
            conn.execute(
                "INSERT INTO notes VALUES (1, 1, 'kept', '2026-10-22', '2026-10-22')",
                [],
            )
            .unwrap();
            // Upgrading keeps existing rows; running again changes nothing.
            assert_eq!(migrate(&mut conn, &[v1, v2]).unwrap(), 2);
            assert_eq!(migrate(&mut conn, &[v1, v2]).unwrap(), 2);
            let (body, pinned): (String, i64) = conn
                .query_row("SELECT body, pinned FROM notes", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .unwrap();
            assert_eq!((body.as_str(), pinned), ("kept", 0));
            // A failing migration is rolled back and leaves the version alone.
            assert!(
                migrate(
                    &mut conn,
                    &[v1, v2, "CREATE TABLE x (a); SELECT nope FROM y;"]
                )
                .is_err()
            );
            let version: u32 = conn
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .unwrap();
            assert_eq!(version, 2);
            assert!(conn.prepare("SELECT * FROM x").is_err());
        }
        // This build knows one migration; the file is at version 2.
        let err = Store::open(&db.0, quran()).err().unwrap();
        assert_eq!(
            err,
            StudyError::Newer {
                found: 2,
                known: SCHEMA_VERSION
            }
        );
    }
}
