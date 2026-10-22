//! Translation files: one per translation, in the container of `data/quran.bin` (see
//! `format.rs`) with their own magic and sections. Bundled translations are built by
//! `cargo xtask build` into `data/translations/` and embedded at compile time; user imports
//! (`import.rs`) are converted to the same format and stored in the user's app data folder.
//! Keeping them apart from `data/quran.bin` is DECISIONS.md D-018.
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! header   magic "QURANTRN" · version u32 · section count u32
//! table    per section: tag [u8; 4] · offset u32 · length u32   (offsets from file start)
//! INFO     field offsets into INFS, u32 × (7 + 1): id, language, name, translator,
//!          source, license, notice
//! INFS     the field texts (UTF-8)
//! SRCS     SHA-256 of the source file the verses were taken from (32 bytes)
//! AYAO     verse offsets into TEXT, u32 × (6,236 + 1)
//! TEXT     verse texts in Quran order, concatenated (UTF-8, exactly as in the source)
//! ```

use serde::{Deserialize, Serialize};

use crate::AYAH_COUNT;
use crate::format::{
    EncodeError, FormatError, Sections, put_u32, read_u32, utf8, validate_offsets, write_sections,
};

pub const MAGIC: [u8; 8] = *b"QURANTRN";
pub const VERSION: u32 = 1;

const INFO: [u8; 4] = *b"INFO";
const INFO_STRINGS: [u8; 4] = *b"INFS";
const SOURCE: [u8; 4] = *b"SRCS";
const AYAH_OFFSETS: [u8; 4] = *b"AYAO";
const TEXT: [u8; 4] = *b"TEXT";
const FIELDS: usize = 7;

/// Describes a translation; returned by the `translations` Tauri command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationInfo {
    /// Unique among installed translations; imports start with `user.`.
    pub id: String,
    /// BCP 47 language tag such as `de` or `en`; `und` when unknown.
    pub language: String,
    /// Title shown in the settings, e.g. "Pickthall (1930)".
    pub name: String,
    pub translator: String,
    /// Where the text was taken from: a URL for bundled translations, the file name for
    /// imports.
    pub source: String,
    /// Legal status in one line, e.g. "Public domain".
    pub license: String,
    /// Full notice (provenance, terms, attribution), shown on the credits page.
    pub notice: String,
    /// Shipped with the app; false for the user's own imports. Not stored in the file.
    #[serde(default)]
    pub bundled: bool,
}

/// A translation, read in place from a translation file.
#[derive(Debug, Clone, Copy)]
pub struct Translation<'a> {
    info_offsets: &'a [u8],
    info: &'a str,
    source: &'a [u8],
    ayah_offsets: &'a [u8],
    text: &'a str,
}

impl<'a> Translation<'a> {
    /// Validates `bytes` (UTF-8, offsets, 6,236 verses) and returns a view into them.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, FormatError> {
        let sections = Sections::parse(bytes, MAGIC, VERSION)?;
        let t = Self {
            info_offsets: sections.get(INFO, "INFO")?,
            info: utf8(sections.get(INFO_STRINGS, "INFS")?, "info is not UTF-8")?,
            source: sections.get(SOURCE, "SRCS")?,
            ayah_offsets: sections.get(AYAH_OFFSETS, "AYAO")?,
            text: utf8(sections.get(TEXT, "TEXT")?, "text is not UTF-8")?,
        };
        if t.info_offsets.len() != (FIELDS + 1) * 4 {
            return Err(FormatError::Malformed("expected 7 info fields"));
        }
        validate_offsets(t.info_offsets, t.info, "info offsets")?;
        if t.source.len() != 32 {
            return Err(FormatError::Malformed("SRCS must hold one SHA-256 digest"));
        }
        if t.ayah_offsets.len() != (usize::from(AYAH_COUNT) + 1) * 4 {
            return Err(FormatError::Malformed("expected 6,236 verse offsets"));
        }
        validate_offsets(t.ayah_offsets, t.text, "verse offsets")?;
        if t.id().is_empty() {
            return Err(FormatError::Malformed("translation without id"));
        }
        Ok(t)
    }

    fn field(&self, i: usize) -> &'a str {
        let start = read_u32(self.info_offsets, i) as usize;
        let end = read_u32(self.info_offsets, i + 1) as usize;
        &self.info[start..end]
    }

    pub fn id(&self) -> &'a str {
        self.field(0)
    }

    /// The descriptive fields; `bundled` is up to the caller.
    pub fn info(&self, bundled: bool) -> TranslationInfo {
        TranslationInfo {
            id: self.field(0).to_owned(),
            language: self.field(1).to_owned(),
            name: self.field(2).to_owned(),
            translator: self.field(3).to_owned(),
            source: self.field(4).to_owned(),
            license: self.field(5).to_owned(),
            notice: self.field(6).to_owned(),
            bundled,
        }
    }

    /// SHA-256 of the file the verses were taken from.
    pub fn source_sha256(&self) -> &'a [u8] {
        self.source
    }

    /// Verse `i` by global index (1:1 is 0).
    pub fn verse(&self, i: u16) -> Option<&'a str> {
        (i < AYAH_COUNT).then(|| {
            let start = read_u32(self.ayah_offsets, usize::from(i)) as usize;
            let end = read_u32(self.ayah_offsets, usize::from(i) + 1) as usize;
            &self.text[start..end]
        })
    }

    /// All 6,236 verses in Quran order.
    pub fn verses(&self) -> impl ExactSizeIterator<Item = &'a str> + '_ {
        (0..AYAH_COUNT).map(|i| self.verse(i).unwrap_or_default())
    }
}

/// Writes a translation file. Deterministic; validated with [`Translation::from_bytes`]
/// before it is returned, so `verses` must hold exactly 6,236 strings.
pub fn encode<S: AsRef<str>>(
    info: &TranslationInfo,
    source_sha256: [u8; 32],
    verses: &[S],
) -> Result<Vec<u8>, EncodeError> {
    let fields = [
        &info.id,
        &info.language,
        &info.name,
        &info.translator,
        &info.source,
        &info.license,
        &info.notice,
    ];
    let (mut info_offsets, mut info_text) = (Vec::new(), String::new());
    for f in fields {
        put_u32(&mut info_offsets, crate::format::offset(info_text.len())?);
        info_text.push_str(f);
    }
    put_u32(&mut info_offsets, crate::format::offset(info_text.len())?);

    let (mut ayah_offsets, mut text) = (Vec::new(), String::new());
    for v in verses {
        put_u32(&mut ayah_offsets, crate::format::offset(text.len())?);
        text.push_str(v.as_ref());
    }
    put_u32(&mut ayah_offsets, crate::format::offset(text.len())?);

    let sections = [
        (INFO, info_offsets),
        (INFO_STRINGS, info_text.into_bytes()),
        (SOURCE, source_sha256.to_vec()),
        (AYAH_OFFSETS, ayah_offsets),
        (TEXT, text.into_bytes()),
    ];
    let out = write_sections(MAGIC, VERSION, &sections)?;
    Translation::from_bytes(&out).map_err(EncodeError::Invalid)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> TranslationInfo {
        TranslationInfo {
            id: "xx.synthetic".into(),
            language: "en".into(),
            name: "Synthetic".into(),
            translator: "Nobody".into(),
            source: "test".into(),
            license: "none".into(),
            notice: "line one\nline two".into(),
            bundled: false,
        }
    }

    fn synthetic() -> Vec<String> {
        (0..AYAH_COUNT).map(|i| format!("verse {i} ✓")).collect()
    }

    #[test]
    fn round_trips() {
        let verses = synthetic();
        let bytes = encode(&info(), [7; 32], &verses).unwrap();
        let t = Translation::from_bytes(&bytes).unwrap();
        assert_eq!(
            t.info(true),
            TranslationInfo {
                bundled: true,
                ..info()
            }
        );
        assert_eq!(t.source_sha256(), &[7; 32]);
        assert_eq!(t.verses().len(), usize::from(AYAH_COUNT));
        assert!(t.verses().eq(verses.iter().map(String::as_str)));
        assert_eq!(t.verse(AYAH_COUNT - 1), Some("verse 6235 ✓"));
        assert_eq!(t.verse(AYAH_COUNT), None);
        // Deterministic.
        assert_eq!(encode(&info(), [7; 32], &verses).unwrap(), bytes);
    }

    #[test]
    fn needs_one_string_per_verse() {
        let mut verses = synthetic();
        verses.pop();
        assert_eq!(
            encode(&info(), [0; 32], &verses),
            Err(EncodeError::Invalid(FormatError::Malformed(
                "expected 6,236 verse offsets"
            )))
        );
    }

    #[test]
    fn rejects_other_files_and_damage() {
        assert_eq!(
            Translation::from_bytes(b"QURANBIN\x01\0\0\0\0\0\0\0").err(),
            Some(FormatError::BadMagic)
        );
        let mut bytes = encode(&info(), [0; 32], &synthetic()).unwrap();
        let last = bytes.len() - 1;
        bytes[last] = 0xFF; // no longer UTF-8
        assert_eq!(
            Translation::from_bytes(&bytes).err(),
            Some(FormatError::Malformed("text is not UTF-8"))
        );
    }
}
