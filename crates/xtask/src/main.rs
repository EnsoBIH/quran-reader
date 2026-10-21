//! `cargo xtask` — reproducible data build.
//!
//! 1. `fetch`  — download every source in `data/sources.toml` into `.cache/`, verify SHA-256.
//! 2. `build`  — generate `data/quran.bin` and `LICENSES/TANZIL.txt` from the cached sources.
//! 3. `verify` — check `data/quran.bin` against the sources: counts, Basmala rules, text
//!    digest, notice, and that a fresh build reproduces the committed file byte for byte.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use quran_core::format::{self, Source, SurahSource};
use quran_core::{AYAH_COUNT, Quran, Revelation, SURAH_COUNT, SajdaKind, VerseRef};
use roxmltree::{Document, Node};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  fetch   download pinned sources and verify SHA-256
  build   generate data/ from cached sources
  verify  check verse counts and checksums";

const TEXT_ID: &str = "tanzil-uthmani";
const METADATA_ID: &str = "tanzil-metadata";
const DATA_FILE: &str = "data/quran.bin";
const NOTICE_FILE: &str = "LICENSES/TANZIL.txt";

const JUZ_COUNT: u8 = 30;
const HIZB_QUARTER_COUNT: u8 = 240;
const PAGE_COUNT: u16 = 604;
const SAJDA_COUNT: usize = 15;

type Result<T, E = String> = std::result::Result<T, E>;

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        Some("fetch") => fetch(),
        Some("build") => build(),
        Some("verify") => verify(),
        Some("-h" | "--help") => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Deserialize)]
struct Manifest {
    source: Vec<SourceEntry>,
}

#[derive(Deserialize)]
struct SourceEntry {
    id: String,
    url: String,
    sha256: String,
}

impl SourceEntry {
    fn cache_path(&self) -> PathBuf {
        root().join(".cache").join(&self.id)
    }

    /// The cached file, if it is there and matches the pinned SHA-256.
    fn read_cached(&self) -> Result<Vec<u8>> {
        let path = self.cache_path();
        let bytes = fs::read(&path)
            .map_err(|e| format!("{}: {e} — run `cargo xtask fetch` first", path.display()))?;
        let actual = sha256_hex(&bytes);
        if actual != self.sha256 {
            return Err(format!(
                "{}: SHA-256 mismatch\n  expected {}\n  actual   {actual}",
                path.display(),
                self.sha256
            ));
        }
        Ok(bytes)
    }
}

fn manifest() -> Result<Manifest> {
    let path = root().join("data/sources.toml");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn source<'m>(manifest: &'m Manifest, id: &str) -> Result<&'m SourceEntry> {
    manifest
        .source
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("data/sources.toml has no source `{id}`"))
}

// --- fetch -------------------------------------------------------------------------------

fn fetch() -> Result<()> {
    let manifest = manifest()?;
    fs::create_dir_all(root().join(".cache")).map_err(|e| format!(".cache: {e}"))?;
    for entry in &manifest.source {
        if entry.read_cached().is_ok() {
            println!("{}: cached, SHA-256 ok", entry.id);
            continue;
        }
        let path = entry.cache_path();
        let part = path.with_extension("part");
        println!("{}: downloading {}", entry.id, entry.url);
        let status = Command::new("curl")
            .args(["--fail", "--silent", "--show-error", "--location"])
            .args(["--proto", "=https", "--retry", "3", "--output"])
            .arg(&part)
            .arg(&entry.url)
            .status()
            .map_err(|e| format!("could not run curl: {e}"))?;
        if !status.success() {
            let _ = fs::remove_file(&part);
            return Err(format!("{}: download failed ({status})", entry.id));
        }
        let bytes = fs::read(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        let actual = sha256_hex(&bytes);
        if actual != entry.sha256 {
            let _ = fs::remove_file(&part);
            return Err(format!(
                "{}: SHA-256 mismatch — the source changed upstream; do not update the pin \
                 without reviewing the new text\n  expected {}\n  actual   {actual}",
                entry.id, entry.sha256
            ));
        }
        fs::rename(&part, &path).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}: SHA-256 ok", entry.id);
    }
    Ok(())
}

// --- build -------------------------------------------------------------------------------

/// Both Tanzil sources, read from the cache and checked against their pins.
struct Sources {
    text: String,
    metadata: String,
    text_sha256: [u8; 32],
    metadata_sha256: [u8; 32],
}

impl Sources {
    fn load() -> Result<Self> {
        let manifest = manifest()?;
        let read = |id| -> Result<(String, [u8; 32])> {
            let bytes = source(&manifest, id)?.read_cached()?;
            let digest = Sha256::digest(&bytes).into();
            let text = String::from_utf8(bytes).map_err(|_| format!("{id}: not UTF-8"))?;
            Ok((text, digest))
        };
        let (text, text_sha256) = read(TEXT_ID)?;
        let (metadata, metadata_sha256) = read(METADATA_ID)?;
        Ok(Self {
            text,
            metadata,
            text_sha256,
            metadata_sha256,
        })
    }
}

/// Parses both sources and encodes the data file. Returns the bytes and the notice.
fn encode(sources: &Sources) -> Result<(Vec<u8>, String)> {
    let text = Document::parse(&sources.text).map_err(|e| format!("{TEXT_ID}: {e}"))?;
    let metadata = Document::parse(&sources.metadata).map_err(|e| format!("{METADATA_ID}: {e}"))?;

    let notice = notice(&text)?;
    let text_surahs = parse_text(&text)?;
    let meta_surahs = elements(&metadata, "sura")?;
    if meta_surahs.len() != text_surahs.len() {
        return Err("metadata and text disagree on the number of surahs".into());
    }

    let mut surahs = Vec::with_capacity(text_surahs.len());
    for (number, (meta, (bismillah, ayahs))) in (1u16..).zip(meta_surahs.iter().zip(text_surahs)) {
        let declared: usize = parse_attr(meta, "ayas")?;
        if declared != ayahs.len() {
            return Err(format!(
                "surah {number}: metadata says {declared} verses, text has {}",
                ayahs.len()
            ));
        }
        surahs.push(SurahSource {
            name_arabic: attr(meta, "name")?,
            name_latin: attr(meta, "tname")?,
            name_translated: attr(meta, "ename")?,
            revelation: match attr(meta, "type")? {
                "Meccan" => Revelation::Meccan,
                "Medinan" => Revelation::Medinan,
                other => return Err(format!("surah {number}: unknown type {other:?}")),
            },
            revelation_order: parse_attr(meta, "order")?,
            rukus: parse_attr(meta, "rukus")?,
            bismillah,
            ayahs,
        });
    }

    let starts = |tag| -> Result<Vec<VerseRef>> {
        elements(&metadata, tag)?.iter().map(verse_ref).collect()
    };
    let sajdas = elements(&metadata, "sajda")?
        .iter()
        .map(|n| {
            let kind = match attr(n, "type")? {
                "recommended" => SajdaKind::Recommended,
                "obligatory" => SajdaKind::Obligatory,
                other => return Err(format!("unknown sajda type {other:?}")),
            };
            Ok((verse_ref(n)?, kind))
        })
        .collect::<Result<Vec<_>>>()?;

    let src = Source {
        notice: &notice,
        text_sha256: sources.text_sha256,
        metadata_sha256: sources.metadata_sha256,
        surahs,
        juz: starts("juz")?,
        hizb_quarters: starts("quarter")?,
        pages: starts("page")?,
        sajdas,
    };
    let bytes = format::encode(&src).map_err(|e| e.to_string())?;
    Ok((bytes, notice))
}

fn build() -> Result<()> {
    let sources = Sources::load()?;
    let (bytes, notice) = encode(&sources)?;
    write(DATA_FILE, &bytes)?;
    write(NOTICE_FILE, format!("{notice}\n").as_bytes())?;
    println!(
        "wrote {DATA_FILE} ({} bytes, SHA-256 {})",
        bytes.len(),
        sha256_hex(&bytes)
    );
    println!("wrote {NOTICE_FILE}");
    Ok(())
}

/// The Tanzil copyright block from the comment at the top of the text file, verbatim
/// except for line endings and the blank lines around it.
fn notice(doc: &Document<'_>) -> Result<String> {
    let comment = doc
        .root()
        .children()
        .find(|n| n.is_comment())
        .and_then(|n| n.text())
        .ok_or("text source has no copyright block")?;
    let lines: Vec<&str> = comment.lines().collect();
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let last = lines.iter().rposition(|l| !l.trim().is_empty());
    let (Some(first), Some(last)) = (first, last) else {
        return Err("text source has an empty copyright block".into());
    };
    let notice = lines[first..=last].join("\n");
    for expected in [
        "Tanzil",
        "Creative Commons Attribution 3.0",
        "CHANGING IT IS NOT ALLOWED",
    ] {
        if !notice.contains(expected) {
            return Err(format!("copyright block does not mention {expected:?}"));
        }
    }
    Ok(notice)
}

/// Per surah: the Basmala (if any) and the verse texts, exactly as in the source.
type TextSurah<'a> = (Option<&'a str>, Vec<&'a str>);

fn parse_text<'a>(doc: &'a Document<'_>) -> Result<Vec<TextSurah<'a>>> {
    let surahs = doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("sura"));
    let mut out = Vec::new();
    for (number, sura) in (1usize..).zip(surahs) {
        expect_index(&sura, number)?;
        let mut bismillah = None;
        let mut ayahs = Vec::new();
        for (a, aya) in (1usize..).zip(sura.children().filter(|n| n.has_tag_name("aya"))) {
            expect_index(&aya, a)?;
            if let Some(b) = aya.attribute("bismillah") {
                if a != 1 {
                    return Err(format!("{number}:{a}: Basmala outside verse 1"));
                }
                bismillah = Some(b);
            }
            ayahs.push(attr(&aya, "text")?);
        }
        out.push((bismillah, ayahs));
    }
    Ok(out)
}

/// All `tag` elements in document order; their `index` attributes must run 1, 2, 3, …
fn elements<'a, 'i>(doc: &'a Document<'i>, tag: &str) -> Result<Vec<Node<'a, 'i>>> {
    let nodes: Vec<_> = doc.descendants().filter(|n| n.has_tag_name(tag)).collect();
    for (i, n) in (1usize..).zip(&nodes) {
        expect_index(n, i)?;
    }
    Ok(nodes)
}

fn expect_index(node: &Node<'_, '_>, expected: usize) -> Result<()> {
    let index: usize = parse_attr(node, "index")?;
    if index == expected {
        Ok(())
    } else {
        Err(format!(
            "<{} index=\"{index}\"> where {expected} was expected",
            node.tag_name().name()
        ))
    }
}

fn attr<'a>(node: &Node<'a, '_>, name: &str) -> Result<&'a str> {
    node.attribute(name)
        .ok_or_else(|| format!("<{}> without `{name}`", node.tag_name().name()))
}

fn parse_attr<T: std::str::FromStr>(node: &Node<'_, '_>, name: &str) -> Result<T> {
    let value = attr(node, name)?;
    value.parse().map_err(|_| {
        format!(
            "<{}> has invalid `{name}`: {value:?}",
            node.tag_name().name()
        )
    })
}

fn verse_ref(node: &Node<'_, '_>) -> Result<VerseRef> {
    Ok(VerseRef::new(
        parse_attr(node, "sura")?,
        parse_attr(node, "aya")?,
    ))
}

// --- verify ------------------------------------------------------------------------------

fn verify() -> Result<()> {
    let path = root().join(DATA_FILE);
    let committed = fs::read(&path).map_err(|e| {
        format!(
            "{}: {e} — run `cargo xtask fetch && cargo xtask build`",
            path.display()
        )
    })?;
    let quran = Quran::from_bytes(&committed).map_err(|e| format!("{DATA_FILE}: {e}"))?;
    check_structure(&quran)?;
    println!(
        "{DATA_FILE}: {SURAH_COUNT} surahs, {AYAH_COUNT} verses, {JUZ_COUNT} juz, \
              {HIZB_QUARTER_COUNT} hizb quarters, {PAGE_COUNT} pages, {SAJDA_COUNT} sajdas"
    );

    let mut hasher = Sha256::new();
    for line in quran.canonical_lines() {
        hasher.update(line.as_bytes());
    }
    expect_digest(
        "text digest of data file",
        &hasher.finalize(),
        quran_core::TEXT_SHA256,
    )?;

    let manifest = manifest()?;
    expect_digest(
        "text source pinned in data file",
        quran.text_source_sha256(),
        &source(&manifest, TEXT_ID)?.sha256,
    )?;
    expect_digest(
        "metadata source pinned in data file",
        quran.metadata_source_sha256(),
        &source(&manifest, METADATA_ID)?.sha256,
    )?;

    let notice_path = root().join(NOTICE_FILE);
    let notice_file =
        fs::read_to_string(&notice_path).map_err(|e| format!("{NOTICE_FILE}: {e}"))?;
    if notice_file != format!("{}\n", quran.notice()) {
        return Err(format!(
            "{NOTICE_FILE} differs from the Tanzil copyright block"
        ));
    }

    // Against the pinned sources: digest computed straight from the XML, then a fresh build.
    let sources = Sources::load()?;
    let doc = Document::parse(&sources.text).map_err(|e| format!("{TEXT_ID}: {e}"))?;
    let mut hasher = Sha256::new();
    for (surah, (bismillah, ayahs)) in (1u16..).zip(parse_text(&doc)?) {
        if let Some(b) = bismillah {
            hasher.update(format::canonical_line(surah, 0, b).as_bytes());
        }
        for (ayah, text) in (1u16..).zip(ayahs) {
            hasher.update(format::canonical_line(surah, ayah, text).as_bytes());
        }
    }
    expect_digest(
        "text digest of Tanzil source",
        &hasher.finalize(),
        quran_core::TEXT_SHA256,
    )?;

    let (rebuilt, _) = encode(&sources)?;
    if rebuilt != committed {
        return Err(format!(
            "{DATA_FILE} is not what `cargo xtask build` produces from the pinned sources \
             (committed {}, rebuilt {})",
            sha256_hex(&committed),
            sha256_hex(&rebuilt)
        ));
    }
    println!("{DATA_FILE}: matches the pinned sources byte for byte");
    println!("verify ok");
    Ok(())
}

fn check_structure(quran: &Quran<'_>) -> Result<()> {
    let ayahs: Vec<_> = quran.surahs().flat_map(|s| quran.ayahs(s.number)).collect();
    if quran.surahs().count() != usize::from(SURAH_COUNT) || ayahs.len() != usize::from(AYAH_COUNT)
    {
        return Err("wrong number of surahs or verses".into());
    }
    let last = ayahs.last().ok_or("no verses")?;
    if (last.juz, last.hizb_quarter, last.page) != (JUZ_COUNT, HIZB_QUARTER_COUNT, PAGE_COUNT) {
        return Err(format!(
            "114:6 is in juz {}, hizb quarter {}, page {}; expected {JUZ_COUNT}, \
             {HIZB_QUARTER_COUNT}, {PAGE_COUNT}",
            last.juz, last.hizb_quarter, last.page
        ));
    }
    let sajdas = ayahs.iter().filter(|a| a.sajda.is_some()).count();
    if sajdas != SAJDA_COUNT {
        return Err(format!("{sajdas} sajda verses, expected {SAJDA_COUNT}"));
    }

    // Basmala: verse 1 of surah 1, absent from surah 9, a separate line everywhere else.
    // Its spelling is not uniform: surahs 95 and 97 carry a shadda on the bā' (بِّسْمِ) in
    // the Madina Mushaf, so only presence is checked here, never equality.
    let fatiha = quran.surah(1).ok_or("no surah 1")?;
    let first_verse = quran.text(VerseRef::new(1, 1)).ok_or("no verse 1:1")?;
    let basmala = quran
        .surah(2)
        .and_then(|s| s.bismillah)
        .ok_or("surah 2 has no Basmala")?;
    if fatiha.bismillah.is_some() || first_verse != basmala {
        return Err("surah 1 must open with the Basmala as verse 1".into());
    }
    for s in quran.surahs() {
        match (s.number, s.bismillah) {
            (1 | 9, None) => {}
            (1 | 9, Some(_)) => return Err(format!("surah {} must not have a Basmala", s.number)),
            (_, Some(_)) => {}
            (n, None) => return Err(format!("surah {n} lacks the Basmala")),
        }
    }
    Ok(())
}

// --- helpers -----------------------------------------------------------------------------

fn write(relative: &str, bytes: &[u8]) -> Result<()> {
    let path = root().join(relative);
    fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn expect_digest(what: &str, actual: &[u8], expected: &str) -> Result<()> {
    let actual = hex(actual);
    if actual == expected {
        println!("{what}: {actual} ok");
        Ok(())
    } else {
        Err(format!("{what}: expected {expected}, got {actual}"))
    }
}
