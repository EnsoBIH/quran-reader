//! `cargo bench -p quran-core` — load time of the data file, reference parsing and the
//! payload of the `quran_text` command.
//! Targets (README / plan): parse a reference in < 0.1 ms; text on the first frame.
//! Run one group with e.g. `cargo bench -p quran-core -- parse_ref`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use quran_core::{Quran, VerseRef, parse_ref};

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

criterion_group!(benches, load, parse, text);
criterion_main!(benches);
