//! `cargo bench -p quran-core` — load time of the data file and reference parsing.
//! Targets (README / plan): parse a reference in < 0.1 ms; text on the first frame.

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

fn parse(c: &mut Criterion) {
    quran_core::quran().expect("embedded data");
    c.bench_function("parse_ref(\"2:255\")", |b| {
        b.iter(|| parse_ref(black_box("2:255")))
    });
    c.bench_function("parse_ref(\"2:287\") (out of range)", |b| {
        b.iter(|| parse_ref(black_box("2:287")))
    });
}

criterion_group!(benches, load, parse);
criterion_main!(benches);
