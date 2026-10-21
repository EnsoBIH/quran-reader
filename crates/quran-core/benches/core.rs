//! `cargo bench -p quran-core` — load time of the data file, reference parsing, the
//! payload of the `quran_text` command and full-text search.
//! Targets (README / plan): parse a reference in < 0.1 ms; text on the first frame; search
//! the whole Quran in < 5 ms.
//! Run one group with e.g. `cargo bench -p quran-core -- parse_ref`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use quran_core::{AYAH_COUNT, Quran, SearchIndex, VerseRef, parse_ref};

const DATA: &[u8] = include_bytes!("../../../data/quran.bin");

fn load(c: &mut Criterion) {
    c.bench_function("load (validate data/quran.bin)", |b| {
        b.iter(|| Quran::from_bytes(black_box(DATA)).expect("valid data"))
    });

    let quran = Quran::from_bytes(DATA).expect("valid data");
    c.bench_function("surah_text(2) (286 verses, owned)", |b| {
        b.iter(|| quran.surah_text(black_box(2)))
    });
    c.bench_function("ayah(2:255)", |b| {
        b.iter(|| quran.ayah(black_box(VerseRef::new(2, 255))))
    });
}

/// The ⌘K palette examples from the plan, plus the slow paths: a long fuzzy name and a
/// short prefix that matches many surahs.
const QUERIES: &[&str] = &[
    "2:255",
    "2:287",
    "baqara 255",
    "Al-Baqarah 255",
    "بقرة",
    "bakara",
    "Ayat al-Kursi",
    "juz 30",
    "page 604",
    "hizb 10",
    "al-muddaththir",
    "an",
];

fn parse(c: &mut Criterion) {
    quran_core::quran().expect("embedded data");
    for query in QUERIES {
        c.bench_function(&format!("parse_ref({query:?})"), |b| {
            b.iter(|| parse_ref(black_box(query)))
        });
    }
    let quran = quran_core::quran().expect("embedded data");
    c.bench_function("lookup(\"an\", 8) (palette list)", |b| {
        b.iter(|| quran.lookup(black_box("an"), 8))
    });
}

fn text(c: &mut Criterion) {
    let quran = Quran::from_bytes(DATA).expect("valid data");
    c.bench_function("quran_text() (whole text, borrowed)", |b| {
        b.iter(|| quran.quran_text())
    });
    c.bench_function("quran_text() as JSON (quran_text command)", |b| {
        b.iter(|| serde_json::to_vec(&quran.quran_text()).expect("serializes"))
    });
}

/// Arabic searches over the whole text: common and rare words, a single letter (the worst
/// case: almost every verse matches), a phrase, an exclusion and scopes.
const SEARCHES: &[&str] = &[
    "الله",
    "رحمن",
    "زيتون",
    "م",
    "\"ان الله\"",
    "الله -رحمن",
    "الله رحيم غفور",
    "الله surah:2",
    "الله juz:30",
    "xyzzy",
];

fn search(c: &mut Criterion) {
    let quran = Quran::from_bytes(DATA).expect("valid data");
    c.bench_function("search index (build at load)", |b| {
        b.iter(|| SearchIndex::new(black_box(quran)))
    });
    let index = SearchIndex::new(quran);
    for query in SEARCHES {
        c.bench_function(&format!("search({query:?}, 100)"), |b| {
            b.iter(|| index.search(black_box(query), 100))
        });
    }

    // The Latin path (translations from Phase 4) on synthetic text of similar size.
    let words = [
        "lorem",
        "ipsum",
        "dolor",
        "sit",
        "amet",
        "consectetur",
        "adipiscing",
        "elit",
        "sed",
        "do",
        "eiusmod",
        "tempor",
        "incididunt",
        "ut",
        "labore",
        "et",
        "dolore",
        "magna",
    ];
    let synthetic: Vec<String> = (0..usize::from(AYAH_COUNT))
        .map(|i| {
            (0..30)
                .map(|j| words[(i * 7 + j * 13) % words.len()])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    let mut index = SearchIndex::new(quran);
    index
        .add_text("synthetic", synthetic.iter().map(String::as_str).collect())
        .expect("one string per verse");
    for query in ["magna", "\"sed do\"", "dolor -amet", "zzz"] {
        c.bench_function(&format!("search({query:?}, 100) with a Latin text"), |b| {
            b.iter(|| index.search(black_box(query), 100))
        });
    }

    // The Arabic and every bundled translation, as the app searches them (D-017): each query
    // scans every text. "the" matches almost every verse of an English translation.
    let translations = quran_core::bundled_translations().expect("bundled translations");
    let build = || {
        let mut index = SearchIndex::new(quran);
        for t in translations {
            index
                .add_text(t.id(), t.verses().collect())
                .expect("one string per verse");
        }
        index
    };
    c.bench_function("search index with bundled translations (build)", |b| {
        b.iter(build)
    });
    let index = build();
    for query in [
        "mercy",
        "\"lord of the worlds\"",
        "allah -merciful",
        "the",
        "الله",
        "zzz",
    ] {
        c.bench_function(&format!("search({query:?}, 100) with translations"), |b| {
            b.iter(|| index.search(black_box(query), 100))
        });
    }
}

criterion_group!(benches, load, parse, text, search);
criterion_main!(benches);
