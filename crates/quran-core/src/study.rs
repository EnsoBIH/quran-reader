//! Study data: highlights, notes, bookmarks and the reading plan, keyed by verse reference
//! (`surah:ayah`), so they stay valid whichever translation is shown.
//!
//! This module holds the data types, their validation, export and import as JSON (lossless,
//! versioned) and Markdown (human-readable, importable), and the merge of an import into
//! the current data. Storage is in [`crate::store`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::plan::{self, Date, Portion};
use crate::{Quran, VerseRef};

/// Highlight colours, keys 1–6 in the reader (`--color-mark-1..6` in the theme).
pub const COLORS: [&str; 6] = ["yellow", "green", "blue", "pink", "purple", "orange"];
/// `format` of a JSON export.
pub const FORMAT: &str = "quran-reader-study";
/// Version of the export format. Readers accept this and older versions.
pub const FORMAT_VERSION: u32 = 1;
/// Longest note, in bytes.
pub const MAX_NOTE: usize = 100_000;
/// Longest bookmark label, in bytes.
pub const MAX_LABEL: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Highlight {
    #[serde(rename = "ref")]
    pub verse: VerseRef,
    /// 1–6.
    pub color: u8,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(rename = "ref")]
    pub verse: VerseRef,
    /// Plain text, never empty.
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    #[serde(rename = "ref")]
    pub verse: VerseRef,
    /// One line; absent for a bookmark without a label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub created_at: String,
}

/// A khatm plan: the Quran in `days` days from `start`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub days: u16,
    /// `YYYY-MM-DD`, day 1 of the plan.
    pub start: String,
    /// Days marked as read, ascending.
    #[serde(default)]
    pub read: Vec<u16>,
    pub created_at: String,
}

/// Everything the user has added. Lists are sorted by verse, one entry per verse.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyData {
    #[serde(default)]
    pub highlights: Vec<Highlight>,
    #[serde(default)]
    pub notes: Vec<Note>,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub plan: Option<Plan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudyError {
    /// Data that breaks a rule, with a message for the user.
    Invalid(String),
    /// A file that is not a study export.
    Format(String),
    /// An export or database written by a newer version of the app.
    Newer {
        found: u32,
        known: u32,
    },
    Sqlite(String),
}

impl fmt::Display for StudyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(s) | Self::Format(s) => f.write_str(s),
            Self::Newer { found, known } => write!(
                f,
                "made by a newer version of Quran Reader (format {found}, this version reads up to {known}); please update"
            ),
            Self::Sqlite(s) => write!(f, "study database: {s}"),
        }
    }
}

impl std::error::Error for StudyError {}

fn invalid(s: impl Into<String>) -> StudyError {
    StudyError::Invalid(s.into())
}

/// Checks a verse reference against the text.
pub fn check_ref(quran: &Quran<'_>, r: VerseRef) -> Result<(), StudyError> {
    quran
        .check(r)
        .map(|_| ())
        .map_err(|e| invalid(format!("{r}: {e}")))
}

pub fn check_color(color: u8) -> Result<(), StudyError> {
    if (1..=6).contains(&color) {
        Ok(())
    } else {
        Err(invalid(format!("highlight colour {color} is not 1–6")))
    }
}

/// A note body as stored: line endings unified, trailing whitespace dropped. Empty means
/// "no note".
pub fn clean_note(body: &str) -> String {
    body.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim_end()
        .to_owned()
}

/// A bookmark label as stored: one line, trimmed; `None` when empty.
pub fn clean_label(label: Option<&str>) -> Option<String> {
    let one_line = label?.split_whitespace().collect::<Vec<_>>().join(" ");
    (!one_line.is_empty()).then_some(one_line)
}

impl StudyData {
    /// Sorts every list by verse and the plan's read days.
    pub fn normalize(&mut self) {
        self.highlights.sort_by_key(|h| h.verse);
        self.notes.sort_by_key(|n| n.verse);
        self.bookmarks.sort_by_key(|b| b.verse);
        if let Some(p) = &mut self.plan {
            p.read.sort_unstable();
            p.read.dedup();
        }
    }

    /// Checks every rule the store enforces, so an import fails as a whole before anything
    /// is written.
    pub fn validate(&self, quran: &Quran<'_>) -> Result<(), StudyError> {
        fn unique<'a>(
            kind: &str,
            refs: impl Iterator<Item = &'a VerseRef>,
        ) -> Result<(), StudyError> {
            let mut seen = BTreeSet::new();
            for r in refs {
                if !seen.insert(*r) {
                    return Err(invalid(format!("{r} has more than one {kind}")));
                }
            }
            Ok(())
        }
        let stamp = |r: &VerseRef, s: &str| {
            if s.len() >= 10 && Date::parse(&s[..10]).is_ok() {
                Ok(())
            } else {
                Err(invalid(format!("{r}: {s:?} is not a timestamp")))
            }
        };
        for h in &self.highlights {
            check_ref(quran, h.verse)?;
            check_color(h.color).map_err(|e| invalid(format!("{}: {e}", h.verse)))?;
            stamp(&h.verse, &h.updated_at)?;
        }
        unique("highlight", self.highlights.iter().map(|h| &h.verse))?;
        for n in &self.notes {
            check_ref(quran, n.verse)?;
            if n.body.trim().is_empty() {
                return Err(invalid(format!("{}: the note is empty", n.verse)));
            }
            if n.body.len() > MAX_NOTE {
                return Err(invalid(format!("{}: the note is too long", n.verse)));
            }
            stamp(&n.verse, &n.created_at)?;
            stamp(&n.verse, &n.updated_at)?;
        }
        unique("note", self.notes.iter().map(|n| &n.verse))?;
        for b in &self.bookmarks {
            check_ref(quran, b.verse)?;
            if let Some(label) = &b.label
                && (label.contains('\n') || label.len() > MAX_LABEL || label.trim().is_empty())
            {
                return Err(invalid(format!(
                    "{}: a bookmark label is one line of up to {MAX_LABEL} bytes",
                    b.verse
                )));
            }
            stamp(&b.verse, &b.created_at)?;
        }
        unique("bookmark", self.bookmarks.iter().map(|b| &b.verse))?;
        if let Some(p) = &self.plan {
            if !(1..=plan::MAX_DAYS).contains(&p.days) {
                return Err(invalid(plan::PlanError::Days(p.days).to_string()));
            }
            Date::parse(&p.start).map_err(|e| invalid(e.to_string()))?;
            if let Some(d) = p.read.iter().find(|&&d| d == 0 || d > p.days) {
                return Err(invalid(format!(
                    "reading plan: day {d} is not in 1–{}",
                    p.days
                )));
            }
            if p.created_at.len() < 10 || Date::parse(&p.created_at[..10]).is_err() {
                return Err(invalid("reading plan: bad creation timestamp"));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------
// JSON

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportOut<'a> {
    format: &'static str,
    version: u32,
    exported_at: &'a str,
    app_version: &'static str,
    #[serde(flatten)]
    data: &'a StudyData,
}

#[derive(Deserialize)]
struct ExportIn {
    format: String,
    version: u32,
    #[serde(flatten)]
    data: StudyData,
}

/// The lossless, versioned export: `{"format": "quran-reader-study", "version": 1, …}`.
pub fn to_json(data: &StudyData, exported_at: &str) -> String {
    let out = ExportOut {
        format: FORMAT,
        version: FORMAT_VERSION,
        exported_at,
        app_version: crate::VERSION,
        data,
    };
    // Cannot fail: plain structs with string keys.
    serde_json::to_string_pretty(&out).unwrap_or_default() + "\n"
}

/// Reads a JSON export, checking its format and version, and validates the data.
pub fn from_json(text: &str, quran: &Quran<'_>) -> Result<StudyData, StudyError> {
    #[derive(Deserialize)]
    struct Head {
        format: Option<String>,
        version: Option<u32>,
    }
    let head: Head = serde_json::from_str(text)
        .map_err(|e| StudyError::Format(format!("not a study export: {e}")))?;
    if head.format.as_deref() != Some(FORMAT) {
        return Err(StudyError::Format(format!(
            "not a study export (no \"format\": \"{FORMAT}\")"
        )));
    }
    match head.version {
        Some(v) if v > FORMAT_VERSION => {
            return Err(StudyError::Newer {
                found: v,
                known: FORMAT_VERSION,
            });
        }
        Some(_) => {}
        None => return Err(StudyError::Format("the export has no version".into())),
    }
    let file: ExportIn =
        serde_json::from_str(text).map_err(|e| StudyError::Format(format!("export: {e}")))?;
    debug_assert_eq!(file.format, FORMAT);
    debug_assert!(file.version <= FORMAT_VERSION);
    let mut data = file.data;
    data.normalize();
    data.validate(quran)?;
    Ok(data)
}

// ---------------------------------------------------------------------------------------
// Markdown

const MD_TITLE: &str = "# Quran Reader — study notes";
const MD_PLAN: &str = "Reading plan";

/// A human-readable export: one section per verse with the Arabic text, highlight,
/// bookmark and note (as a quote), and the reading plan. It can be imported again; only
/// the timestamps are lost (use JSON for a lossless copy).
pub fn to_markdown(data: &StudyData, quran: &Quran<'_>, exported_at: &str) -> String {
    let mut out = String::new();
    out.push_str(MD_TITLE);
    out.push_str("\n\n");
    out.push_str(&format!(
        "Exported {exported_at} from Quran Reader {}. Import this file in Quran Reader to restore \
         the highlights, notes, bookmarks and reading plan; the JSON export also keeps the dates.\n",
        crate::VERSION
    ));
    if let Some(p) = &data.plan {
        out.push_str(&format!("\n## {MD_PLAN}\n\n"));
        out.push_str(&format!("- Length: {} days\n", p.days));
        out.push_str(&format!("- Start: {}\n", p.start));
        let read: Vec<String> = p.read.iter().map(u16::to_string).collect();
        out.push_str(&format!(
            "- Read: {}\n",
            if read.is_empty() {
                "none".to_owned()
            } else {
                read.join(", ")
            }
        ));
    }
    let mut verses: BTreeSet<VerseRef> = BTreeSet::new();
    verses.extend(data.highlights.iter().map(|h| h.verse));
    verses.extend(data.notes.iter().map(|n| n.verse));
    verses.extend(data.bookmarks.iter().map(|b| b.verse));
    for r in verses {
        let name = quran
            .surah(r.surah)
            .map(|s| s.name_latin.to_owned())
            .unwrap_or_default();
        out.push_str(&format!("\n## {name} {r}\n\n"));
        if let Some(text) = quran.text(r) {
            out.push_str(text);
            out.push_str("\n\n");
        }
        if let Some(h) = data.highlights.iter().find(|h| h.verse == r) {
            let name = COLORS.get(usize::from(h.color) - 1).unwrap_or(&"");
            out.push_str(&format!("- Highlight: {name} ({})\n", h.color));
        }
        if let Some(b) = data.bookmarks.iter().find(|b| b.verse == r) {
            match &b.label {
                Some(label) => out.push_str(&format!("- Bookmark: {label}\n")),
                None => out.push_str("- Bookmark\n"),
            }
        }
        if let Some(n) = data.notes.iter().find(|n| n.verse == r) {
            if !out.ends_with("\n\n") {
                out.push('\n');
            }
            for line in n.body.lines() {
                if line.is_empty() {
                    out.push_str(">\n");
                } else {
                    out.push_str("> ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
    }
    out.push_str(
        "\n---\n\nQuran text: Tanzil.net (CC BY 3.0), shown unmodified. https://tanzil.net\n",
    );
    out
}

/// Reads a Markdown export (see [`to_markdown`]). Lines it does not know (the Arabic text,
/// prose) are skipped; entries get `now` as their timestamps.
pub fn from_markdown(text: &str, quran: &Quran<'_>, now: &str) -> Result<StudyData, StudyError> {
    enum Section {
        None,
        Plan,
        Verse(VerseRef),
    }
    if !text.lines().any(|l| l.trim_end() == MD_TITLE) {
        return Err(StudyError::Format(format!(
            "not a study export (no \"{MD_TITLE}\" line)"
        )));
    }
    let mut data = StudyData::default();
    let mut notes: BTreeMap<VerseRef, Vec<&str>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut section = Section::None;
    let mut plan: (Option<u16>, Option<String>, Vec<u16>) = (None, None, Vec::new());
    let mut has_plan = false;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end();
        let at = |msg: String| invalid(format!("line {}: {msg}", i + 1));
        if let Some(heading) = line.strip_prefix("## ") {
            let heading = heading.trim();
            if heading == MD_PLAN {
                has_plan = true;
                section = Section::Plan;
                continue;
            }
            let token = heading.rsplit(char::is_whitespace).next().unwrap_or("");
            let r = parse_colon_ref(token).ok_or_else(|| {
                at(format!(
                    "{heading:?} does not end with a verse such as 2:255"
                ))
            })?;
            check_ref(quran, r).map_err(|e| at(e.to_string()))?;
            if !seen.insert(r) {
                return Err(at(format!("{r} appears twice")));
            }
            section = Section::Verse(r);
            continue;
        }
        if line.starts_with("# ") || line == "---" {
            section = Section::None;
            continue;
        }
        match section {
            Section::None => {}
            Section::Plan => {
                if let Some(v) = item(line, "Length") {
                    let n = v.trim_end_matches("days").trim_end_matches("day").trim();
                    plan.0 = Some(
                        n.parse()
                            .map_err(|_| at(format!("{v:?} is not a length")))?,
                    );
                } else if let Some(v) = item(line, "Start") {
                    plan.1 = Some(v.to_owned());
                } else if let Some(v) = item(line, "Read")
                    && v != "none"
                {
                    for d in v.split(',') {
                        plan.2.push(
                            d.trim()
                                .parse()
                                .map_err(|_| at(format!("{d:?} is not a day")))?,
                        );
                    }
                }
            }
            Section::Verse(r) => {
                if let Some(rest) = line.strip_prefix('>') {
                    notes
                        .entry(r)
                        .or_default()
                        .push(rest.strip_prefix(' ').unwrap_or(rest));
                } else if let Some(v) = item(line, "Highlight") {
                    let color =
                        parse_color(v).ok_or_else(|| at(format!("{v:?} is not a colour")))?;
                    data.highlights.push(Highlight {
                        verse: r,
                        color,
                        updated_at: now.to_owned(),
                    });
                } else if line == "- Bookmark" || item(line, "Bookmark").is_some() {
                    data.bookmarks.push(Bookmark {
                        verse: r,
                        label: clean_label(item(line, "Bookmark")),
                        created_at: now.to_owned(),
                    });
                }
            }
        }
    }
    for (r, lines) in notes {
        let body = clean_note(&lines.join("\n"));
        if !body.trim().is_empty() {
            data.notes.push(Note {
                verse: r,
                body,
                created_at: now.to_owned(),
                updated_at: now.to_owned(),
            });
        }
    }
    if has_plan {
        let (Some(days), Some(start)) = (plan.0, plan.1) else {
            return Err(invalid("the reading plan needs a length and a start"));
        };
        data.plan = Some(Plan {
            days,
            start,
            read: plan.2,
            created_at: now.to_owned(),
        });
    }
    data.normalize();
    data.validate(quran)?;
    Ok(data)
}

/// `- Name: value` → `value`.
fn item<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let rest = line
        .strip_prefix("- ")?
        .strip_prefix(name)?
        .strip_prefix(':')?;
    Some(rest.trim())
}

fn parse_colon_ref(s: &str) -> Option<VerseRef> {
    let (a, b) = s.split_once(':')?;
    Some(VerseRef::new(a.parse().ok()?, b.parse().ok()?))
}

/// `yellow`, `yellow (1)`, `1`.
fn parse_color(v: &str) -> Option<u8> {
    let v = v.trim();
    if let Some(open) = v.rfind('(') {
        return v[open + 1..]
            .trim_end_matches(')')
            .trim()
            .parse()
            .ok()
            .filter(|c| (1..=6).contains(c));
    }
    if let Ok(n) = v.parse::<u8>() {
        return (1..=6).contains(&n).then_some(n);
    }
    COLORS
        .iter()
        .position(|c| c.eq_ignore_ascii_case(v))
        .map(|i| i as u8 + 1)
}

/// Reads an export in either format: JSON when it starts with `{`, Markdown otherwise.
pub fn parse_export(text: &str, quran: &Quran<'_>, now: &str) -> Result<StudyData, StudyError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.trim_start().starts_with('{') {
        from_json(text, quran)
    } else {
        from_markdown(text, quran, now)
    }
}

// ---------------------------------------------------------------------------------------
// Import: merge or replace, with a preview

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// Keep what is there and add the file's entries; on the same verse the file wins.
    Merge,
    /// Make the study data exactly the file's.
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Change {
    Added,
    Changed,
    Removed,
}

/// One entry an import adds, changes or removes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Changed {
    #[serde(rename = "ref")]
    pub verse: VerseRef,
    pub change: Change,
}

/// What an import does to one kind of entry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Changes {
    pub added: u32,
    pub changed: u32,
    pub removed: u32,
    pub unchanged: u32,
    /// The entries counted in `added`, `changed` and `removed`, in Quran order.
    pub entries: Vec<Changed>,
}

/// What an import would do, shown before it is applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub mode: ImportMode,
    pub highlights: Changes,
    pub notes: Changes,
    pub bookmarks: Changes,
    /// `None` when the plan stays as it is.
    pub plan: Option<Change>,
}

impl ImportPreview {
    pub fn is_empty(&self) -> bool {
        let none = |c: &Changes| c.entries.is_empty();
        none(&self.highlights) && none(&self.notes) && none(&self.bookmarks) && self.plan.is_none()
    }
}

trait Entry: Clone {
    fn verse(&self) -> VerseRef;
    /// Same content, ignoring timestamps.
    fn same(&self, other: &Self) -> bool;
}

impl Entry for Highlight {
    fn verse(&self) -> VerseRef {
        self.verse
    }
    fn same(&self, o: &Self) -> bool {
        self.color == o.color
    }
}

impl Entry for Note {
    fn verse(&self) -> VerseRef {
        self.verse
    }
    fn same(&self, o: &Self) -> bool {
        self.body == o.body
    }
}

impl Entry for Bookmark {
    fn verse(&self) -> VerseRef {
        self.verse
    }
    fn same(&self, o: &Self) -> bool {
        self.label == o.label
    }
}

fn combine<T: Entry>(current: &[T], imported: &[T], mode: ImportMode) -> (Vec<T>, Changes) {
    let mut out: BTreeMap<VerseRef, T> = BTreeMap::new();
    let mut changes = Changes::default();
    let mut entry = |verse, change| changes.entries.push(Changed { verse, change });
    let old: BTreeMap<VerseRef, &T> = current.iter().map(|e| (e.verse(), e)).collect();
    let new: BTreeMap<VerseRef, &T> = imported.iter().map(|e| (e.verse(), e)).collect();
    let refs: BTreeSet<VerseRef> = old.keys().chain(new.keys()).copied().collect();
    for r in refs {
        match (old.get(&r), new.get(&r)) {
            (Some(o), Some(n)) if o.same(n) => {
                // Unchanged: keep the stored entry and its dates.
                out.insert(r, (*o).clone());
            }
            (Some(_), Some(n)) => {
                entry(r, Change::Changed);
                out.insert(r, (*n).clone());
            }
            (None, Some(n)) => {
                entry(r, Change::Added);
                out.insert(r, (*n).clone());
            }
            (Some(o), None) => match mode {
                ImportMode::Merge => {
                    out.insert(r, (*o).clone());
                }
                ImportMode::Replace => entry(r, Change::Removed),
            },
            (None, None) => {}
        }
    }
    for e in &changes.entries {
        match e.change {
            Change::Added => changes.added += 1,
            Change::Changed => changes.changed += 1,
            Change::Removed => changes.removed += 1,
        }
    }
    changes.unchanged = (out.len() as u32).saturating_sub(changes.added + changes.changed);
    (out.into_values().collect(), changes)
}

/// The data after importing `imported` into `current`, and what changed. Unchanged entries
/// keep their stored dates.
pub fn merge(
    current: &StudyData,
    imported: &StudyData,
    mode: ImportMode,
) -> (StudyData, ImportPreview) {
    let (highlights, hc) = combine(&current.highlights, &imported.highlights, mode);
    let (notes, nc) = combine(&current.notes, &imported.notes, mode);
    let (bookmarks, bc) = combine(&current.bookmarks, &imported.bookmarks, mode);
    let plan = match (&current.plan, &imported.plan, mode) {
        (old, None, ImportMode::Merge) => old.clone(),
        (_, None, ImportMode::Replace) => None,
        (Some(old), Some(new), ImportMode::Merge)
            if old.days == new.days && old.start == new.start =>
        {
            let mut p = old.clone();
            p.read.extend(&new.read);
            p.read.sort_unstable();
            p.read.dedup();
            Some(p)
        }
        (Some(old), Some(new), _)
            if old.days == new.days && old.start == new.start && old.read == new.read =>
        {
            Some(old.clone())
        }
        (_, Some(new), _) => Some(new.clone()),
    };
    let plan_change = match (&current.plan, &plan) {
        (None, None) => None,
        (None, Some(_)) => Some(Change::Added),
        (Some(_), None) => Some(Change::Removed),
        (Some(a), Some(b)) => (a != b).then_some(Change::Changed),
    };
    (
        StudyData {
            highlights,
            notes,
            bookmarks,
            plan,
        },
        ImportPreview {
            mode,
            highlights: hc,
            notes: nc,
            bookmarks: bc,
            plan: plan_change,
        },
    )
}

// ---------------------------------------------------------------------------------------
// Plan status

/// The plan with its portions and where `today` falls, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStatus {
    pub plan: Plan,
    pub portions: Vec<Portion>,
    /// Plan day of `today`: below 1 before the start, above `days` after the end.
    pub today: i64,
}

pub fn plan_status(quran: &Quran<'_>, plan: &Plan, today: Date) -> Result<PlanStatus, StudyError> {
    let start = Date::parse(&plan.start).map_err(|e| invalid(e.to_string()))?;
    Ok(PlanStatus {
        plan: plan.clone(),
        portions: plan::portions(quran, plan.days).map_err(|e| invalid(e.to_string()))?,
        today: plan::day_of(start, today),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn quran() -> &'static Quran<'static> {
        crate::quran().expect("data embedded")
    }

    const T: &str = "2026-10-22T10:00:00Z";

    pub(crate) fn sample() -> StudyData {
        let r = VerseRef::new;
        let mut d = StudyData {
            highlights: vec![
                Highlight {
                    verse: r(2, 255),
                    color: 1,
                    updated_at: T.into(),
                },
                Highlight {
                    verse: r(1, 1),
                    color: 6,
                    updated_at: T.into(),
                },
            ],
            notes: vec![Note {
                verse: r(2, 255),
                body: "First line\n\n- not a list item\n> not a quote\n## not a heading".into(),
                created_at: "2026-10-01T08:00:00Z".into(),
                updated_at: T.into(),
            }],
            bookmarks: vec![
                Bookmark {
                    verse: r(18, 1),
                    label: Some("Friday: al-Kahf".into()),
                    created_at: T.into(),
                },
                Bookmark {
                    verse: r(36, 1),
                    label: None,
                    created_at: T.into(),
                },
            ],
            plan: Some(Plan {
                days: 30,
                start: "2026-10-01".into(),
                read: vec![1, 2, 5],
                created_at: T.into(),
            }),
        };
        d.normalize();
        d
    }

    #[test]
    fn json_round_trip_is_lossless() {
        let q = quran();
        let data = sample();
        data.validate(q).unwrap();
        let json = to_json(&data, T);
        assert!(json.contains("\"format\": \"quran-reader-study\""));
        assert!(json.contains("\"version\": 1"));
        assert_eq!(from_json(&json, q).unwrap(), data);
        assert_eq!(parse_export(&json, q, "x").unwrap(), data);
        let empty = StudyData::default();
        assert_eq!(from_json(&to_json(&empty, T), q).unwrap(), empty);
    }

    #[test]
    fn json_is_checked() {
        let q = quran();
        let newer = r#"{"format":"quran-reader-study","version":2}"#;
        assert_eq!(
            from_json(newer, q),
            Err(StudyError::Newer { found: 2, known: 1 })
        );
        assert!(matches!(
            from_json(r#"{"version":1}"#, q),
            Err(StudyError::Format(_))
        ));
        assert!(matches!(from_json("[]", q), Err(StudyError::Format(_))));
        let bad = |body: &str| {
            from_json(
                &format!(r#"{{"format":"quran-reader-study","version":1,{body}}}"#),
                q,
            )
        };
        let e = bad(
            r#""highlights":[{"ref":{"surah":2,"ayah":287},"color":1,"updatedAt":"2026-10-22"}]"#,
        );
        assert!(e.unwrap_err().to_string().contains("2:287"));
        assert!(
            bad(
                r#""highlights":[{"ref":{"surah":2,"ayah":1},"color":7,"updatedAt":"2026-10-22"}]"#
            )
            .is_err()
        );
        assert!(bad(r#""notes":[{"ref":{"surah":1,"ayah":1},"body":" ","createdAt":"2026-10-22","updatedAt":"2026-10-22"}]"#).is_err());
        let twice = r#""bookmarks":[{"ref":{"surah":1,"ayah":1},"createdAt":"2026-10-22"},{"ref":{"surah":1,"ayah":1},"createdAt":"2026-10-22"}]"#;
        assert!(
            bad(twice)
                .unwrap_err()
                .to_string()
                .contains("more than one bookmark")
        );
        assert!(
            bad(r#""plan":{"days":30,"start":"2026-10-01","read":[31],"createdAt":"2026-10-22"}"#)
                .is_err()
        );
        assert!(bad(r#""plan":{"days":0,"start":"2026-10-01","createdAt":"2026-10-22"}"#).is_err());
        assert!(
            bad(r#""highlights":[{"ref":{"surah":1,"ayah":1},"color":1,"updatedAt":"yesterday"}]"#)
                .is_err()
        );
        // Unknown fields from a future minor addition are ignored.
        assert!(bad(r#""extra":true"#).is_ok());
    }

    #[test]
    fn markdown_round_trip_keeps_everything_but_dates() {
        let q = quran();
        let data = sample();
        let md = to_markdown(&data, q, "2026-10-22 10:00 UTC");
        assert!(
            md.contains("## Al-Baqara 2:255") || md.contains(" 2:255\n"),
            "{md}"
        );
        assert!(md.contains("- Highlight: yellow (1)"));
        assert!(md.contains("- Bookmark: Friday: al-Kahf"));
        assert!(md.contains(
            "> First line\n>\n> - not a list item\n> > not a quote\n> ## not a heading\n"
        ));
        assert!(md.contains(q.text(VerseRef::new(2, 255)).unwrap()));
        assert!(md.contains("Tanzil.net"));
        let back = from_markdown(&md, q, T).unwrap();
        assert_eq!(back.highlights, data.highlights);
        assert_eq!(back.bookmarks, data.bookmarks);
        assert_eq!(back.notes.len(), 1);
        assert_eq!(back.notes[0].body, data.notes[0].body);
        let plan = back.plan.unwrap();
        assert_eq!(
            (plan.days, plan.start.as_str(), plan.read),
            (30, "2026-10-01", vec![1, 2, 5])
        );
        assert_eq!(parse_export(&md, q, T).unwrap().notes, back.notes);
    }

    #[test]
    fn markdown_is_checked() {
        let q = quran();
        let head = format!("{MD_TITLE}\n\n");
        assert!(matches!(
            from_markdown("## 2:255\n- Bookmark\n", q, T),
            Err(StudyError::Format(_))
        ));
        let e = from_markdown(&format!("{head}## Al-Baqara 2:300\n"), q, T).unwrap_err();
        assert!(e.to_string().starts_with("line 3:"), "{e}");
        assert!(from_markdown(&format!("{head}## Notes\n"), q, T).is_err());
        assert!(from_markdown(&format!("{head}## 2:1\n- Highlight: teal\n"), q, T).is_err());
        assert!(from_markdown(&format!("{head}## 2:1\n## 2:1\n"), q, T).is_err());
        assert!(
            from_markdown(&format!("{head}## {MD_PLAN}\n- Start: 2026-10-01\n"), q, T).is_err()
        );
        let ok = from_markdown(
            &format!("{head}## x 2:1\n\n- Highlight: 3\n- Bookmark\n"),
            q,
            T,
        )
        .unwrap();
        assert_eq!(ok.highlights[0].color, 3);
        assert_eq!(ok.bookmarks[0].label, None);
        assert_eq!(parse_color("Purple"), Some(5));
        assert_eq!(parse_color("blue (9)"), None);
    }

    #[test]
    fn merge_and_replace() {
        let r = VerseRef::new;
        let current = sample();
        let mut file = StudyData::default();
        // Same content, other date: unchanged, stored date kept.
        file.highlights.push(Highlight {
            verse: r(2, 255),
            color: 1,
            updated_at: "2020-01-01T00:00:00Z".into(),
        });
        // Other colour: changed. New verse: added.
        file.highlights.push(Highlight {
            verse: r(1, 1),
            color: 2,
            updated_at: T.into(),
        });
        file.highlights.push(Highlight {
            verse: r(3, 1),
            color: 4,
            updated_at: T.into(),
        });
        file.plan = Some(Plan {
            days: 30,
            start: "2026-10-01".into(),
            read: vec![3],
            created_at: T.into(),
        });
        file.normalize();

        let (merged, p) = merge(&current, &file, ImportMode::Merge);
        assert_eq!(
            (
                p.highlights.added,
                p.highlights.changed,
                p.highlights.removed,
                p.highlights.unchanged
            ),
            (1, 1, 0, 1)
        );
        assert_eq!(
            p.highlights.entries,
            vec![
                Changed {
                    verse: r(1, 1),
                    change: Change::Changed
                },
                Changed {
                    verse: r(3, 1),
                    change: Change::Added
                },
            ]
        );
        assert_eq!(
            merged
                .highlights
                .iter()
                .find(|h| h.verse == r(2, 255))
                .unwrap()
                .updated_at,
            T
        );
        assert_eq!(merged.notes, current.notes);
        assert_eq!(merged.bookmarks, current.bookmarks);
        assert_eq!(
            p.notes,
            Changes {
                unchanged: 1,
                ..Changes::default()
            }
        );
        // Same plan: read days are combined.
        assert_eq!(merged.plan.as_ref().unwrap().read, vec![1, 2, 3, 5]);
        assert_eq!(p.plan, Some(Change::Changed));

        let (replaced, p) = merge(&current, &file, ImportMode::Replace);
        assert_eq!(replaced.highlights.len(), 3);
        assert!(replaced.notes.is_empty() && replaced.bookmarks.is_empty());
        assert_eq!((p.notes.removed, p.bookmarks.removed), (1, 2));
        assert_eq!(replaced.plan.as_ref().unwrap().read, vec![3]);

        // Importing the current data changes nothing.
        let (same, p) = merge(&current, &current, ImportMode::Replace);
        assert_eq!(same, current);
        assert!(p.is_empty());
        let (_, p) = merge(&current, &StudyData::default(), ImportMode::Merge);
        assert!(p.is_empty());
        let (gone, p) = merge(&current, &StudyData::default(), ImportMode::Replace);
        assert_eq!(gone, StudyData::default());
        assert_eq!(p.plan, Some(Change::Removed));
    }

    #[test]
    fn cleaning() {
        assert_eq!(clean_note("a\r\nb\rc  \n\n"), "a\nb\nc");
        assert_eq!(
            clean_label(Some("  two\n words ")),
            Some("two words".into())
        );
        assert_eq!(clean_label(Some("  ")), None);
        assert_eq!(clean_label(None), None);
    }

    #[test]
    fn plan_status_finds_today() {
        let q = quran();
        let plan = sample().plan.unwrap();
        let s = plan_status(q, &plan, Date::parse("2026-10-22").unwrap()).unwrap();
        assert_eq!(s.today, 22);
        assert_eq!(s.portions.len(), 30);
        assert_eq!(s.portions[21].day, 22);
    }
}
