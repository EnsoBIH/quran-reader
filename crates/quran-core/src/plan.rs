//! Khatm reading plans: the whole Quran in a number of days, one portion per day.
//!
//! Portions are whole hizb quarters for plans of up to 240 days, so 30 days is one juz a
//! day and 60 days one hizb a day; longer plans (up to 604 days) are cut by Madina Mushaf
//! pages. Units are spread as evenly as possible: day *i* of *n* gets units
//! `⌊(i−1)·U/n⌋ .. ⌊i·U/n⌋`.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{AYAH_COUNT, Part, Quran, VerseRef};

/// Hizb quarters in the Quran (60 hizbs of four quarters).
const QUARTERS: u16 = 240;
/// Longest plan: one Madina Mushaf page a day.
pub const MAX_DAYS: u16 = 604;

/// One day's reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portion {
    /// 1-based.
    pub day: u16,
    /// First verse.
    pub from: VerseRef,
    /// Last verse, inclusive.
    pub to: VerseRef,
    /// Global index of `from` (1:1 is 0).
    pub from_index: u16,
    /// Global index of `to`.
    pub to_index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    Days(u16),
    Date(String),
    /// The embedded data has no part starts (it is always there in a valid file).
    Data,
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Days(n) => write!(f, "a plan has 1 to {MAX_DAYS} days, not {n}"),
            Self::Date(s) => write!(f, "{s:?} is not a date (YYYY-MM-DD)"),
            Self::Data => write!(f, "the Quran data has no part starts"),
        }
    }
}

impl std::error::Error for PlanError {}

/// The portions of a plan of `days` days, in order. They cover every verse exactly once.
pub fn portions(quran: &Quran<'_>, days: u16) -> Result<Vec<Portion>, PlanError> {
    if !(1..=MAX_DAYS).contains(&days) {
        return Err(PlanError::Days(days));
    }
    // Start of each unit as a global verse index.
    let starts: Vec<u16> = if days <= QUARTERS {
        (1..=QUARTERS)
            .map(|q| quran.quarter_start(q))
            .collect::<Option<_>>()
    } else {
        (1..=quran.part_count(Part::Page))
            .map(|p| {
                quran
                    .part_start(Part::Page, p)
                    .and_then(|r| quran.global_index(r))
            })
            .collect::<Option<_>>()
    }
    .ok_or(PlanError::Data)?;
    let units = starts.len();
    let n = usize::from(days);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let first = i * units / n;
        let next = (i + 1) * units / n;
        let from_index = starts[first];
        let to_index = starts.get(next).map_or(AYAH_COUNT - 1, |s| s - 1);
        let at = |i| quran.verse_ref_at(i).ok_or(PlanError::Data);
        out.push(Portion {
            day: i as u16 + 1,
            from: at(from_index)?,
            to: at(to_index)?,
            from_index,
            to_index,
        });
    }
    Ok(out)
}

/// The plan day `today` falls on, for a plan starting on `start` (day 1). Before the start
/// it is 0 or negative, after the last day greater than the plan's length.
pub fn day_of(start: Date, today: Date) -> i64 {
    today.days() - start.days() + 1
}

/// A calendar date (proleptic Gregorian), as the UI's local "today" or a plan's start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    /// Parses `YYYY-MM-DD`.
    pub fn parse(s: &str) -> Result<Self, PlanError> {
        let bad = || PlanError::Date(s.to_owned());
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return Err(bad());
        }
        let num = |r: std::ops::Range<usize>| {
            let part = &s[r];
            if part.bytes().all(|c| c.is_ascii_digit()) {
                part.parse::<u32>().map_err(|_| bad())
            } else {
                Err(bad())
            }
        };
        let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year as i32, month) {
            return Err(bad());
        }
        Ok(Self {
            year: year as i32,
            month: month as u8,
            day: day as u8,
        })
    }

    /// Days since 1970-01-01.
    pub fn days(self) -> i64 {
        // Howard Hinnant's days_from_civil.
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` days after 1970-01-01.
    pub fn from_days(days: i64) -> Self {
        // Howard Hinnant's civil_from_days.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Self { year, month, day }
    }

    /// The date `n` days later (or earlier, for negative `n`).
    pub fn add_days(self, n: i64) -> Self {
        Self::from_days(self.days() + n)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The current UTC time as `YYYY-MM-DDTHH:MM:SSZ`, for timestamps in the study store.
pub fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let date = Date::from_days(secs.div_euclid(86_400));
    let t = secs.rem_euclid(86_400);
    format!("{date}T{:02}:{:02}:{:02}Z", t / 3600, t % 3600 / 60, t % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quran() -> &'static Quran<'static> {
        crate::quran().expect("data embedded")
    }

    fn check_cover(p: &[Portion]) {
        assert_eq!(p[0].from, VerseRef::new(1, 1));
        assert_eq!(p.last().unwrap().to, VerseRef::new(114, 6));
        for w in p.windows(2) {
            assert_eq!(w[0].to_index + 1, w[1].from_index, "{w:?}");
        }
        for x in p {
            assert!(x.from_index <= x.to_index, "{x:?}");
        }
    }

    #[test]
    fn thirty_days_is_one_juz_a_day() {
        let q = quran();
        let p = portions(q, 30).unwrap();
        assert_eq!(p.len(), 30);
        check_cover(&p);
        for (i, x) in p.iter().enumerate() {
            let juz = i as u16 + 1;
            assert_eq!(Some(x.from), q.part_start(Part::Juz, juz));
            assert_eq!(x.day, juz);
        }
        assert_eq!(p[1].from, VerseRef::new(2, 142));
        assert_eq!(p[0].to, VerseRef::new(2, 141));
        assert_eq!(p[29].from, VerseRef::new(78, 1));
    }

    #[test]
    fn sixty_days_is_one_hizb_a_day() {
        let q = quran();
        let p = portions(q, 60).unwrap();
        check_cover(&p);
        for x in &p {
            assert_eq!(Some(x.from), q.part_start(Part::Hizb, x.day));
        }
    }

    #[test]
    fn any_length_covers_everything_once() {
        let q = quran();
        for days in [1, 2, 7, 29, 31, 100, 239, 240, 241, 365, 603, 604] {
            let p = portions(q, days).unwrap();
            assert_eq!(p.len(), usize::from(days));
            check_cover(&p);
        }
        let one = portions(q, 1).unwrap();
        assert_eq!((one[0].from_index, one[0].to_index), (0, AYAH_COUNT - 1));
        // Page plans: each day starts a page.
        let pages = portions(q, 604).unwrap();
        assert_eq!(Some(pages[1].from), q.part_start(Part::Page, 2));
        // Even spread: 7 days of 240 quarters are 34 or 35 quarters each.
        let week = portions(q, 7).unwrap();
        assert_eq!(
            week[1].from,
            q.verse_ref_at(q.quarter_start(35).unwrap()).unwrap()
        );
        assert_eq!(portions(q, 0), Err(PlanError::Days(0)));
        assert_eq!(portions(q, 605), Err(PlanError::Days(605)));
    }

    #[test]
    fn dates() {
        let d = Date::parse("2026-10-22").unwrap();
        assert_eq!(d.to_string(), "2026-10-22");
        assert_eq!(Date::parse("1970-01-01").unwrap().days(), 0);
        assert_eq!(Date::from_days(0).to_string(), "1970-01-01");
        for s in [
            "2024-02-29",
            "2000-02-29",
            "1999-12-31",
            "2026-03-01",
            "0001-01-01",
        ] {
            let d = Date::parse(s).unwrap();
            assert_eq!(Date::from_days(d.days()), d, "{s}");
        }
        for s in [
            "2026-02-29",
            "1900-02-29",
            "2026-13-01",
            "2026-00-10",
            "2026-1-01",
            "26-10-22",
            "2026-10-2x",
            "+026-10-22",
        ] {
            assert!(Date::parse(s).is_err(), "{s}");
        }
        assert_eq!(d.add_days(10).to_string(), "2026-11-01");
        assert_eq!(
            Date::parse("2024-02-28").unwrap().add_days(1).to_string(),
            "2024-02-29"
        );
        let start = Date::parse("2026-10-01").unwrap();
        assert_eq!(day_of(start, start), 1);
        assert_eq!(day_of(start, d), 22);
        assert_eq!(day_of(start, Date::parse("2026-09-30").unwrap()), 0);
        // Across a year and a leap day.
        assert_eq!(
            day_of(
                Date::parse("2027-12-31").unwrap(),
                Date::parse("2028-03-01").unwrap()
            ),
            62
        );
        let now = now_utc();
        assert_eq!(now.len(), 20);
        assert!(Date::parse(&now[..10]).is_ok());
    }
}
