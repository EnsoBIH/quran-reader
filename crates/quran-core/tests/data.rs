//! Checks the embedded `data/quran.bin`. `cargo xtask verify` checks the same file against
//! the pinned Tanzil sources; together they make sure only the unmodified text ships.

use quran_core::format::{FormatError, Quran};
use quran_core::{AYAH_COUNT, Part, Revelation, SURAH_COUNT, SajdaKind, VerseRef};
use sha2::{Digest, Sha256};

const AYAT_AL_KURSI_SHA256: &str =
    "b036974542211b4c684147cc80b1943b932229e7d59d8e872035144f4aaaef9c";
// SHA-256 of three Pickthall verses, checked by hand against the source when pinned.
const PICKTHALL_1_1_SHA256: &str =
    "3378da21fe4d9f255e0336c517936ef15453fb07d4cf1d5842eaff0eaa30da0e";
const PICKTHALL_2_255_SHA256: &str =
    "f174338173480fd74890bc2dcc0d605c6f418a9923abb17d4e76bee129a5cd64";
const PICKTHALL_114_6_SHA256: &str =
    "d441541ba23bed57d8679fb1d70dcbe7625584f06e7d365b52e31c2ced70c70d";

fn quran() -> &'static Quran<'static> {
    quran_core::quran().expect("data/quran.bin missing — run `cargo xtask build`")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn data_is_loaded() {
    assert!(quran_core::info().data_loaded);
}

#[test]
fn counts_surahs_and_verses() {
    let q = quran();
    assert_eq!(q.surahs().count(), usize::from(SURAH_COUNT));
    let total: usize = q.surahs().map(|s| q.ayahs(s.number).count()).sum();
    assert_eq!(total, usize::from(AYAH_COUNT));
    assert_eq!(q.surah(2).map(|s| s.ayah_count), Some(286));
    assert_eq!(q.surah(108).map(|s| s.ayah_count), Some(3));
    assert!(q.surah(0).is_none());
    assert!(q.surah(115).is_none());
}

#[test]
fn displayed_text_matches_source_checksum() {
    let mut hasher = Sha256::new();
    for line in quran().canonical_lines() {
        hasher.update(line.as_bytes());
    }
    assert_eq!(hex(&hasher.finalize()), quran_core::TEXT_SHA256);
}

#[test]
fn records_pinned_source_checksums() {
    let pins = include_str!("../../../data/sources.toml");
    let q = quran();
    for digest in [q.text_source_sha256(), q.metadata_source_sha256()] {
        let line = format!("sha256 = \"{}\"", hex(digest));
        assert!(
            pins.contains(&line),
            "{line} not pinned in data/sources.toml"
        );
    }
}

#[test]
fn basmala_rules() {
    let q = quran();
    let fatiha = q.surah(1).unwrap();
    let baqara = q.surah(2).unwrap();
    // Surah 1: the Basmala is verse 1, not a separate line.
    assert_eq!(fatiha.bismillah, None);
    assert_eq!(q.text(VerseRef::new(1, 1)), baqara.bismillah);
    // Surah 9: no Basmala at all.
    let tawba = q.surah(9).unwrap();
    assert_eq!(tawba.bismillah, None);
    assert!(
        !q.text(VerseRef::new(9, 1))
            .unwrap()
            .starts_with(baqara.bismillah.unwrap())
    );
    // Everywhere else it is a separate line and verse 1 does not repeat it.
    for s in q.surahs().filter(|s| s.number != 1 && s.number != 9) {
        let basmala = s
            .bismillah
            .unwrap_or_else(|| panic!("surah {} lacks it", s.number));
        let first = q.text(VerseRef::new(s.number, 1)).unwrap();
        assert!(!first.starts_with(basmala), "surah {} repeats it", s.number);
    }
    // Spelled as in the Madina Mushaf: with shadda on the bā' in surahs 95 and 97.
    assert_ne!(q.surah(95).unwrap().bismillah, baqara.bismillah);
    assert_ne!(q.surah(97).unwrap().bismillah, baqara.bismillah);
}

#[test]
fn verse_text_and_markers() {
    let q = quran();
    let kursi = q.ayah(VerseRef::new(2, 255)).unwrap();
    // SHA-256 of the `text` attribute of 2:255 in the pinned Tanzil XML.
    assert_eq!(hex(&Sha256::digest(kursi.text)), AYAT_AL_KURSI_SHA256);
    assert_eq!((kursi.juz, kursi.page), (3, 42));

    let first = q.ayah(VerseRef::new(1, 1)).unwrap();
    assert_eq!((first.juz, first.hizb_quarter, first.page), (1, 1, 1));
    assert_eq!(q.ayah(VerseRef::new(2, 1)).unwrap().page, 2);
    assert_eq!(q.ayah(VerseRef::new(2, 141)).unwrap().juz, 1);
    assert_eq!(q.ayah(VerseRef::new(2, 142)).unwrap().juz, 2);
    let last = q.ayah(VerseRef::new(114, 6)).unwrap();
    assert_eq!((last.juz, last.hizb_quarter, last.page), (30, 240, 604));

    assert_eq!(
        q.ayah(VerseRef::new(96, 19)).unwrap().sajda,
        Some(SajdaKind::Obligatory)
    );
    assert_eq!(
        q.ayah(VerseRef::new(7, 206)).unwrap().sajda,
        Some(SajdaKind::Recommended)
    );
    assert_eq!(q.ayah(VerseRef::new(7, 205)).unwrap().sajda, None);
    let sajdas = q
        .surahs()
        .flat_map(|s| q.ayahs(s.number))
        .filter(|a| a.sajda.is_some())
        .count();
    assert_eq!(sajdas, 15);

    assert!(q.ayah(VerseRef::new(1, 8)).is_none());
    assert!(q.ayah(VerseRef::new(1, 0)).is_none());
}

#[test]
fn surah_metadata() {
    let q = quran();
    let fatiha = q.surah(1).unwrap();
    assert_eq!(fatiha.name_arabic, "الفاتحة");
    assert_eq!(fatiha.revelation, Revelation::Meccan);
    assert_eq!(q.surah(2).unwrap().revelation, Revelation::Medinan);

    let text = q.surah_text(2).unwrap();
    assert_eq!(text.ayahs.len(), 286);
    assert_eq!(text.ayahs[254].verse_ref, VerseRef::new(2, 255));
    assert!(text.bismillah.is_some());
}

#[test]
fn notice_is_the_tanzil_block() {
    let notice = quran().notice();
    assert!(notice.starts_with("# PLEASE DO NOT REMOVE OR CHANGE THIS COPYRIGHT BLOCK"));
    assert!(notice.contains("CHANGING IT IS NOT ALLOWED"));
    assert_eq!(
        format!("{notice}\n"),
        include_str!("../../../LICENSES/TANZIL.txt")
    );
}

#[test]
fn surah_text_serializes_like_the_ts_types() {
    let json = serde_json::to_value(quran().surah_text(9).unwrap()).unwrap();
    assert_eq!(json["surah"]["nameLatin"], "At-Tawba");
    assert_eq!(json["surah"]["revelation"], "medinan");
    assert_eq!(json["surah"]["ayahCount"], 129);
    assert!(json.get("bismillah").is_none());
    let ayah = &json["ayahs"][0];
    assert_eq!(ayah["ref"], serde_json::json!({ "surah": 9, "ayah": 1 }));
    assert_eq!(ayah["hizbQuarter"], 75);
    assert_eq!(ayah["page"], 187);
    assert!(ayah.get("sajda").is_none());
}

#[test]
fn rejects_corrupt_files_without_panicking() {
    let bytes = include_bytes!("../../../data/quran.bin");
    assert_eq!(Quran::from_bytes(&[]).unwrap_err(), FormatError::BadMagic);
    assert!(Quran::from_bytes(&bytes[..bytes.len() - 1]).is_err());

    let mut wrong_version = bytes.to_vec();
    wrong_version[8] = 9;
    assert_eq!(
        Quran::from_bytes(&wrong_version).unwrap_err(),
        FormatError::UnsupportedVersion(9)
    );

    // Flipping any single byte of the header, table or index sections must either be
    // rejected or still yield a readable file — never a panic.
    let mut copy = bytes.to_vec();
    for i in (0..512).chain((512..copy.len()).step_by(4099)) {
        copy[i] ^= 0xff;
        if let Ok(q) = Quran::from_bytes(&copy) {
            for s in q.surahs() {
                q.ayahs(s.number).for_each(drop);
            }
        }
        copy[i] ^= 0xff;
    }
}

#[test]
fn global_indices_round_trip() {
    let q = quran();
    for s in q.surahs() {
        for a in q.ayahs(s.number) {
            let i = q.global_index(a.verse_ref).unwrap();
            assert_eq!(q.verse_ref_at(i), Some(a.verse_ref));
        }
    }
    assert_eq!(q.verse_ref_at(0), Some(VerseRef::new(1, 1)));
    assert_eq!(q.verse_ref_at(AYAH_COUNT - 1), Some(VerseRef::new(114, 6)));
    assert_eq!(q.verse_ref_at(AYAH_COUNT), None);
}

#[test]
fn part_starts() {
    let q = quran();
    assert_eq!(q.part_count(Part::Juz), 30);
    assert_eq!(q.part_count(Part::Hizb), 60);
    assert_eq!(q.part_count(Part::Page), 604);
    for part in [Part::Juz, Part::Hizb, Part::Page] {
        assert_eq!(q.part_start(part, 1), Some(VerseRef::new(1, 1)));
        assert_eq!(q.part_start(part, 0), None);
        assert_eq!(q.part_start(part, q.part_count(part) + 1), None);
        // Every part starts where the verse markers say it does.
        for n in 1..=q.part_count(part) {
            let r = q.part_start(part, n).unwrap();
            let a = q.ayah(r).unwrap();
            let number = match part {
                Part::Juz => u16::from(a.juz),
                Part::Hizb => u16::from(a.hizb_quarter).div_ceil(4),
                Part::Page => a.page,
            };
            assert_eq!(number, n, "{part} {n} starts at {r}");
        }
    }
}

#[test]
fn whole_text_for_the_reader() {
    let q = quran();
    let t = q.quran_text();
    assert_eq!(t.surahs.len(), usize::from(SURAH_COUNT));
    assert_eq!(t.verses.len(), usize::from(AYAH_COUNT));
    assert_eq!(t.juz_starts.len(), 30);
    assert_eq!(t.quarter_starts.len(), 240);
    assert_eq!(t.page_starts.len(), 604);
    assert_eq!(t.sajdas.len(), 15);
    let kursi = VerseRef::new(2, 255);
    let i = usize::from(q.global_index(kursi).unwrap());
    assert_eq!(Some(t.verses[i]), q.text(kursi));
    // Juz 2 starts at 2:142.
    assert_eq!(q.verse_ref_at(t.juz_starts[1]), Some(VerseRef::new(2, 142)));

    let json = serde_json::to_value(&t).unwrap();
    assert_eq!(json["surahs"][1]["nameLatin"], "Al-Baqara");
    assert_eq!(json["surahs"][1]["ayahCount"], 286);
    assert!(json["surahs"][0].get("bismillah").is_none());
    assert!(json["surahs"][8].get("bismillah").is_none());
    assert!(json["surahs"][1]["bismillah"].is_string());
    assert_eq!(json["pageStarts"][603], 6221);
    assert_eq!(json["sajdas"][0]["kind"], "recommended");
}

#[test]
fn bundled_translations_have_every_verse_and_a_pinned_source() {
    let pins = include_str!("../../../data/sources.toml");
    let translations = quran_core::bundled_translations().expect("bundled translations load");
    assert!(!translations.is_empty(), "data/translations/ is empty");
    for t in translations {
        let info = t.info(true);
        assert_eq!(t.verses().len(), usize::from(AYAH_COUNT), "{}", info.id);
        assert!(t.verses().all(|v| !v.trim().is_empty()), "{}", info.id);
        assert!(
            !info.notice.is_empty() && !info.license.is_empty(),
            "{}",
            info.id
        );
        let line = format!("sha256 = \"{}\"", hex(t.source_sha256()));
        assert!(pins.contains(&line), "{}: {line} not pinned", info.id);
    }
}

#[test]
fn pickthall_is_aligned_to_the_verses() {
    // Digests, not text: no translation text in hand-written tests.
    let t = quran_core::bundled_translations()
        .unwrap()
        .iter()
        .find(|t| t.id() == "en.pickthall")
        .expect("Pickthall is bundled");
    let q = quran();
    let digest = |r: VerseRef| {
        let verse = t.verse(q.global_index(r).unwrap()).unwrap();
        hex(&Sha256::digest(verse.as_bytes()))
    };
    // 1:1 is the Basmala, as in the Arabic; 2:255 is Ayat al-Kursi; 114:6 the last verse.
    assert_eq!(digest(VerseRef::new(1, 1)), PICKTHALL_1_1_SHA256);
    assert_eq!(digest(VerseRef::new(2, 255)), PICKTHALL_2_255_SHA256);
    assert_eq!(digest(VerseRef::new(114, 6)), PICKTHALL_114_6_SHA256);
    assert_eq!(t.verses().filter(|v| v.contains('\n')).count(), 0);
}
