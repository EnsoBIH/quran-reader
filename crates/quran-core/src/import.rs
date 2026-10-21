//! Import of a translation the user has downloaded themselves (e.g. one of Tanzil's
//! translations, which may not be redistributed with the app). Imports stay on the user's
//! machine: the app converts them to a translation file (`translation.rs`) in its data
//! folder and never uploads or bundles them.
//!
//! Accepted formats (Tanzil's download formats; see `docs/translation-import.md`):
//!
//! - **Plain text**: one verse per line, 6,236 lines in Quran order. Lines starting with
//!   `#` are comments, blank lines at the end are ignored.
//! - **Text with verse numbers**: `surah|ayah|text` per line, in any order.
//! - **XML**: `<sura index="…"><aya index="…" text="…"/>…</sura>`.
//!
//! Every verse must be present exactly once, so a file that is off by one line is refused
//! instead of being shown next to the wrong verse. Comments (Tanzil puts its copyright
//! notice there) are kept as the translation's notice; `Name:`, `Translator:`,
//! `Language:` and `ID:` lines in them fill in the description.

use std::fmt;

use crate::translation::TranslationInfo;
use crate::{AYAH_COUNT, Quran, VerseRef};

/// Which of the accepted formats a file was in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    Plain,
    Numbered,
    Xml,
}

/// A parsed, validated import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub format: ImportFormat,
    /// All 6,236 verses in Quran order, exactly as in the file.
    pub verses: Vec<String>,
    /// The comment lines, without their `#` (plain formats) or the XML comment, verbatim.
    pub notice: String,
    pub name: Option<String>,
    pub translator: Option<String>,
    /// Language name as written in the file (`Language: German`).
    pub language: Option<String>,
    /// Tanzil id such as `de.bubenheim`.
    pub tanzil_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    Empty,
    /// Plain format with the wrong number of lines.
    LineCount(usize),
    /// A line or element that cannot be read.
    Unreadable {
        line: usize,
        reason: String,
    },
    UnknownVerse {
        line: usize,
        verse: VerseRef,
    },
    Duplicate {
        line: usize,
        verse: VerseRef,
    },
    Missing {
        verse: VerseRef,
        count: usize,
    },
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "the file has no verses"),
            Self::LineCount(n) => write!(
                f,
                "the file has {n} verse lines, expected {AYAH_COUNT} (one line per verse in \
                 Quran order; lines starting with # are comments)"
            ),
            Self::Unreadable { line, reason } => write!(f, "line {line}: {reason}"),
            Self::UnknownVerse { line, verse } => {
                write!(f, "line {line}: verse {verse} does not exist")
            }
            Self::Duplicate { line, verse } => {
                write!(f, "line {line}: verse {verse} appears a second time")
            }
            Self::Missing { verse, count: 1 } => write!(f, "verse {verse} is missing"),
            Self::Missing { verse, count } => {
                write!(f, "{count} verses are missing, the first is {verse}")
            }
        }
    }
}

impl std::error::Error for ImportError {}

/// Parses and validates a file in one of the accepted formats.
pub fn parse(input: &str, quran: &Quran<'_>) -> Result<Import, ImportError> {
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);
    let (format, entries, comments) = if input.trim_start().starts_with('<') {
        let (entries, comments) = xml(input)?;
        (ImportFormat::Xml, entries, comments)
    } else {
        let comments = input
            .lines()
            .filter_map(|l| l.strip_prefix('#'))
            .collect::<Vec<_>>()
            .join("\n");
        let numbered = input
            .lines()
            .find(|l| !l.starts_with('#') && !l.trim().is_empty())
            .is_some_and(|l| numbered_line(l).is_some());
        if numbered {
            (ImportFormat::Numbered, numbered_lines(input)?, comments)
        } else {
            return finish(ImportFormat::Plain, plain(input)?, comments);
        }
    };
    finish(format, align(quran, entries)?, comments)
}

fn finish(
    format: ImportFormat,
    verses: Vec<String>,
    comments: String,
) -> Result<Import, ImportError> {
    let notice = trim_lines(&comments);
    let field = |key: &str| {
        notice.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            let v = v.trim();
            (k.trim().eq_ignore_ascii_case(key) && !v.is_empty()).then(|| v.to_owned())
        })
    };
    Ok(Import {
        format,
        name: field("Name"),
        translator: field("Translator"),
        language: field("Language"),
        tanzil_id: field("ID"),
        verses,
        notice,
    })
}

/// Drops leading and trailing blank lines and the common indentation of the others.
fn trim_lines(s: &str) -> String {
    let lines: Vec<&str> = s.lines().map(str::trim_end).collect();
    let first = lines.iter().position(|l| !l.is_empty());
    let last = lines.iter().rposition(|l| !l.is_empty());
    let (Some(first), Some(last)) = (first, last) else {
        return String::new();
    };
    let lines = &lines[first..=last];
    let indent = lines
        .iter()
        .filter(|l| !l.is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| l.get(indent..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn plain(input: &str) -> Result<Vec<String>, ImportError> {
    let mut lines: Vec<&str> = input.lines().filter(|l| !l.starts_with('#')).collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    match lines.len() {
        0 => Err(ImportError::Empty),
        n if n == usize::from(AYAH_COUNT) => Ok(lines.into_iter().map(str::to_owned).collect()),
        n => Err(ImportError::LineCount(n)),
    }
}

/// `(line, verse, text)` as found in the file.
type Entry = (usize, VerseRef, String);

fn numbered_line(line: &str) -> Option<(VerseRef, &str)> {
    let mut parts = line.splitn(3, '|');
    let surah = parts.next()?.trim().parse().ok()?;
    let ayah = parts.next()?.trim().parse().ok()?;
    Some((VerseRef::new(surah, ayah), parts.next()?))
}

fn numbered_lines(input: &str) -> Result<Vec<Entry>, ImportError> {
    let mut out = Vec::with_capacity(usize::from(AYAH_COUNT));
    for (i, line) in input.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let (verse, text) = numbered_line(line).ok_or_else(|| ImportError::Unreadable {
            line: i + 1,
            reason: "expected surah|ayah|text".into(),
        })?;
        out.push((i + 1, verse, text.to_owned()));
    }
    Ok(out)
}

/// Puts entries in Quran order; every verse exactly once.
fn align(quran: &Quran<'_>, entries: Vec<Entry>) -> Result<Vec<String>, ImportError> {
    if entries.is_empty() {
        return Err(ImportError::Empty);
    }
    let mut verses: Vec<Option<String>> = vec![None; usize::from(AYAH_COUNT)];
    for (line, verse, text) in entries {
        let i = quran
            .global_index(verse)
            .ok_or(ImportError::UnknownVerse { line, verse })?;
        let slot = &mut verses[usize::from(i)];
        if slot.is_some() {
            return Err(ImportError::Duplicate { line, verse });
        }
        *slot = Some(text);
    }
    let count = verses.iter().filter(|v| v.is_none()).count();
    if let Some(first) = verses.iter().position(Option::is_none) {
        let verse = quran
            .verse_ref_at(first as u16)
            .unwrap_or(VerseRef::new(0, 0));
        return Err(ImportError::Missing { verse, count });
    }
    Ok(verses.into_iter().flatten().collect())
}

/// Tanzil's XML: `sura` elements with an `index`, holding `aya` elements with `index` and
/// `text`. Only what that needs: comments, declarations, elements and attributes with the
/// five named entities and character references. Returns the verses and the comments.
fn xml(input: &str) -> Result<(Vec<Entry>, String), ImportError> {
    let line_at = |pos: usize| input[..pos].matches('\n').count() + 1;
    let bad = |pos: usize, reason: &str| ImportError::Unreadable {
        line: line_at(pos),
        reason: reason.to_owned(),
    };
    let (mut entries, mut comments) = (Vec::new(), Vec::new());
    let mut surah: Option<u16> = None;
    let mut pos = 0;
    while let Some(lt) = input[pos..].find('<').map(|p| pos + p) {
        let rest = &input[lt..];
        let close = |end: &str| {
            rest.find(end)
                .map(|e| lt + e + end.len())
                .ok_or_else(|| bad(lt, "unterminated markup"))
        };
        if let Some(body) = rest.strip_prefix("<!--") {
            let end = body
                .find("-->")
                .ok_or_else(|| bad(lt, "unterminated comment"))?;
            comments.push(&body[..end]);
            pos = lt + 4 + end + 3;
            continue;
        }
        if rest.starts_with("<?") {
            pos = close("?>")?;
            continue;
        }
        if rest.starts_with("<!") || rest.starts_with("</") {
            pos = close(">")?;
            continue;
        }
        let (name, attrs, end) = tag(rest).ok_or_else(|| bad(lt, "malformed tag"))?;
        pos = lt + end;
        let attr = |key: &str| -> Result<Option<String>, ImportError> {
            attrs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| unescape(v).ok_or_else(|| bad(lt, "unknown entity")))
                .transpose()
        };
        let index = |what: &str| -> Result<u16, ImportError> {
            attr("index")?
                .and_then(|v| v.trim().parse().ok())
                .ok_or_else(|| bad(lt, &format!("<{what}> without a valid index")))
        };
        match name {
            "sura" => surah = Some(index("sura")?),
            "aya" => {
                let s = surah.ok_or_else(|| bad(lt, "<aya> outside <sura>"))?;
                let text = attr("text")?.ok_or_else(|| bad(lt, "<aya> without text"))?;
                entries.push((line_at(lt), VerseRef::new(s, index("aya")?), text));
            }
            _ => {}
        }
    }
    Ok((entries, comments.join("\n")))
}

/// Name, raw attributes and byte length of a start tag.
type Tag<'s> = (&'s str, Vec<(&'s str, &'s str)>, usize);

/// The start tag at the beginning of `s`.
fn tag(s: &str) -> Option<Tag<'_>> {
    let body = s.strip_prefix('<')?;
    let name_end = body.find(|c: char| c.is_whitespace() || c == '/' || c == '>')?;
    let name = &body[..name_end];
    let mut attrs = Vec::new();
    let mut rest = &body[name_end..];
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix("/>").or_else(|| rest.strip_prefix('>')) {
            return Some((name, attrs, s.len() - r.len()));
        }
        let eq = rest.find('=')?;
        let key = rest[..eq].trim();
        let value = rest[eq + 1..].trim_start();
        let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let end = value[1..].find(quote)?;
        attrs.push((key, &value[1..1 + end]));
        rest = &value[end + 2..];
    }
}

fn unescape(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let semi = rest[amp..].find(';')? + amp;
        let entity = &rest[amp + 1..semi];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = entity.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    entity.strip_prefix('#')?.parse().ok()?
                };
                char::from_u32(code)?
            }
        };
        out.push(c);
        rest = &rest[semi + 1..];
    }
    out.push_str(rest);
    Some(out)
}

impl Import {
    /// Description for the translation file. `file_name` is where it came from; the id is
    /// derived from the Tanzil id or the file name and always starts with `user.`.
    pub fn info(&self, file_name: &str) -> TranslationInfo {
        let stem = file_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(file_name)
            .trim_end_matches(".txt")
            .trim_end_matches(".xml");
        let base = self.tanzil_id.as_deref().unwrap_or(stem);
        let slug: String = base
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let language = self
            .tanzil_id
            .as_deref()
            .and_then(|id| id.split_once('.'))
            .map(|(code, _)| code.to_ascii_lowercase())
            .filter(|code| code.len() == 2 || code.len() == 3)
            .or_else(|| self.language.as_deref().and_then(language_code))
            .unwrap_or_else(|| "und".to_owned());
        TranslationInfo {
            id: format!("user.{}", slug.trim_matches('-')),
            language,
            name: self.name.clone().unwrap_or_else(|| stem.to_owned()),
            translator: self.translator.clone().unwrap_or_default(),
            source: file_name.to_owned(),
            license: "Imported by you; stays on this computer".to_owned(),
            notice: self.notice.clone(),
            bundled: false,
        }
    }
}

/// BCP 47 code for a language name as Tanzil writes it.
fn language_code(name: &str) -> Option<String> {
    const CODES: &[(&str, &str)] = &[
        ("albanian", "sq"),
        ("amharic", "am"),
        ("arabic", "ar"),
        ("azerbaijani", "az"),
        ("bengali", "bn"),
        ("bosnian", "bs"),
        ("bulgarian", "bg"),
        ("chinese", "zh"),
        ("czech", "cs"),
        ("dutch", "nl"),
        ("english", "en"),
        ("french", "fr"),
        ("german", "de"),
        ("hausa", "ha"),
        ("hindi", "hi"),
        ("indonesian", "id"),
        ("italian", "it"),
        ("japanese", "ja"),
        ("korean", "ko"),
        ("kurdish", "ku"),
        ("malay", "ms"),
        ("norwegian", "no"),
        ("persian", "fa"),
        ("polish", "pl"),
        ("portuguese", "pt"),
        ("romanian", "ro"),
        ("russian", "ru"),
        ("somali", "so"),
        ("spanish", "es"),
        ("swahili", "sw"),
        ("swedish", "sv"),
        ("tamil", "ta"),
        ("thai", "th"),
        ("turkish", "tr"),
        ("urdu", "ur"),
        ("uzbek", "uz"),
    ];
    let name = name.trim().to_ascii_lowercase();
    CODES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| (*c).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic verse strings only: no translation text in fixtures.

    fn quran() -> &'static Quran<'static> {
        crate::quran().expect("data/quran.bin missing — run `cargo xtask build`")
    }

    fn refs() -> Vec<VerseRef> {
        quran()
            .surahs()
            .flat_map(|s| (1..=s.ayah_count).map(move |a| VerseRef::new(s.number, a)))
            .collect()
    }

    fn text(r: VerseRef) -> String {
        format!("synthetic {r} <&> \"q\"")
    }

    const HEADER: &str = "#  Name: Synthetic Test\n#  Translator: Nobody\n#  Language: German\n\
                          #  ID: de.synthetic\n#  Terms: for testing only";

    fn plain_file() -> String {
        let body: Vec<String> = refs().into_iter().map(text).collect();
        format!("{}\n\n{HEADER}\n", body.join("\n"))
    }

    fn numbered_file(refs: &[VerseRef]) -> String {
        refs.iter()
            .map(|&r| format!("{}|{}|{}\n", r.surah, r.ayah, text(r)))
            .collect()
    }

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&#62;")
            .replace('"', "&quot;")
    }

    fn xml_file() -> String {
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\" ?>\n<!--\n{}\n-->\n<quran>\n",
            HEADER.replace('#', "")
        );
        for s in quran().surahs() {
            out.push_str(&format!("<sura index=\"{}\" name=\"x\">\n", s.number));
            for a in 1..=s.ayah_count {
                let t = escape(&text(VerseRef::new(s.number, a)));
                out.push_str(&format!("\t<aya index='{a}' text=\"{t}\" />\n"));
            }
            out.push_str("</sura>\n");
        }
        out + "</quran>\n"
    }

    fn expected() -> Vec<String> {
        refs().into_iter().map(text).collect()
    }

    #[test]
    fn reads_all_three_formats_aligned_to_the_verses() {
        let mut shuffled = refs();
        shuffled.reverse();
        for (input, format) in [
            (plain_file(), ImportFormat::Plain),
            (plain_file().replace('\n', "\r\n"), ImportFormat::Plain),
            (
                format!("\u{FEFF}{HEADER}\n{}", numbered_file(&shuffled)),
                ImportFormat::Numbered,
            ),
            (xml_file(), ImportFormat::Xml),
        ] {
            let import = parse(&input, quran()).unwrap();
            assert_eq!(import.format, format);
            assert_eq!(import.verses, expected(), "{format:?}");
            assert_eq!(import.verses[261], "synthetic 2:255 <&> \"q\"");
            assert_eq!(import.name.as_deref(), Some("Synthetic Test"));
            assert_eq!(import.tanzil_id.as_deref(), Some("de.synthetic"));
            assert!(import.notice.starts_with("Name: Synthetic Test\n"));
            assert!(import.notice.ends_with("Terms: for testing only"));
        }
    }

    #[test]
    fn describes_the_import() {
        let import = parse(&plain_file(), quran()).unwrap();
        let info = import.info("/Users/x/Downloads/de.synthetic.txt");
        assert_eq!(info.id, "user.de.synthetic");
        assert_eq!(info.language, "de");
        assert_eq!(info.name, "Synthetic Test");
        assert_eq!(info.translator, "Nobody");
        assert!(!info.bundled);

        // Without a header: name from the file, language from nothing.
        let bare: Vec<String> = refs().into_iter().map(text).collect();
        let import = parse(&bare.join("\n"), quran()).unwrap();
        let info = import.info("My Translation (2).txt");
        assert_eq!(info.id, "user.my-translation--2");
        assert_eq!(info.name, "My Translation (2)");
        assert_eq!(info.language, "und");
        assert_eq!(language_code(" English "), Some("en".into()));
    }

    #[test]
    fn refuses_misaligned_plain_text() {
        let file = plain_file();
        let one_short = file.replacen("synthetic 1:1 <&> \"q\"\n", "", 1);
        assert_eq!(
            parse(&one_short, quran()),
            Err(ImportError::LineCount(6235))
        );
        let one_long = format!("extra\n{file}");
        assert_eq!(parse(&one_long, quran()), Err(ImportError::LineCount(6237)));
        assert_eq!(
            parse("# only a comment\n\n", quran()),
            Err(ImportError::Empty)
        );
    }

    #[test]
    fn refuses_missing_duplicate_and_unknown_verses() {
        let all = refs();
        let missing: Vec<_> = all.iter().copied().filter(|r| r.surah != 2).collect();
        assert_eq!(
            parse(&numbered_file(&missing), quran()),
            Err(ImportError::Missing {
                verse: VerseRef::new(2, 1),
                count: 286
            })
        );
        let mut twice = all.clone();
        twice.insert(10, VerseRef::new(1, 3));
        assert_eq!(
            parse(&numbered_file(&twice), quran()),
            Err(ImportError::Duplicate {
                line: 11,
                verse: VerseRef::new(1, 3)
            })
        );
        let mut unknown = all.clone();
        unknown.push(VerseRef::new(2, 287));
        let err = parse(&numbered_file(&unknown), quran()).unwrap_err();
        assert_eq!(err.to_string(), "line 6237: verse 2:287 does not exist");
        let garbled = numbered_file(&all).replacen("1|2|", "1-2|", 1);
        assert_eq!(
            parse(&garbled, quran()).unwrap_err().to_string(),
            "line 2: expected surah|ayah|text"
        );
    }

    #[test]
    fn refuses_broken_xml() {
        let file = xml_file();
        let missing = file.replacen("\t<aya index='7'", "\t<ignored index='7'", 1);
        assert_eq!(
            parse(&missing, quran()),
            Err(ImportError::Missing {
                verse: VerseRef::new(1, 7),
                count: 1
            })
        );
        let entity = file.replacen("&amp;", "&nbsp;", 1);
        assert!(matches!(
            parse(&entity, quran()),
            Err(ImportError::Unreadable { .. })
        ));
        let open = file.replacen("<sura index=\"1\" name=\"x\">", "<sura index=\"1\"", 1);
        assert!(parse(&open, quran()).is_err());
        assert_eq!(
            parse("<quran><aya index=\"1\" text=\"x\"/></quran>", quran()).unwrap_err(),
            ImportError::Unreadable {
                line: 1,
                reason: "<aya> outside <sura>".into()
            }
        );
    }
}
