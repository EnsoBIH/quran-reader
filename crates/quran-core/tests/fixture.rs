//! The verses the screenshot tests render (`apps/desktop/e2e/`), taken from the embedded
//! data so the test page shows exactly what the app shows. The file is committed; this test
//! fails when it no longer matches the data, and rewrites it with `UPDATE_FIXTURE=1`:
//!
//! ```sh
//! UPDATE_FIXTURE=1 cargo test -p quran-core --test fixture
//! ```

use std::collections::BTreeSet;

use quran_core::format::Quran;
use quran_core::{Translation, VerseRef};
use serde_json::{Value, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../apps/desktop/e2e/verses.json"
);

/// `(id, title, verses)` of one section of the test page.
type Section = (&'static str, &'static str, &'static [(u16, u16)]);

/// Sections of the test page, each one screenshot: what tends to break in Arabic rendering.
const SECTIONS: &[Section] = &[
    (
        "fatiha",
        "Al-Fatihah, Basmala as verse 1, with the translation below",
        &[(1, 1), (1, 2), (1, 3), (1, 4), (1, 5), (1, 6), (1, 7)],
    ),
    (
        "baqara",
        "Surah header, Basmala line, disjoined letters (2:1), pause marks (2:2)",
        &[(2, 1), (2, 2), (2, 3), (2, 4), (2, 5)],
    ),
    (
        "kursi",
        "A long verse across many lines (2:255)",
        &[(2, 255)],
    ),
    (
        "basmala",
        "Basmala spellings: 95 and 97 with a shadda on the ba', and inside 27:30",
        &[(95, 1), (97, 1), (27, 30)],
    ),
    (
        "muqattaat",
        "Disjoined letters with maddah",
        &[
            (19, 1),
            (20, 1),
            (36, 1),
            (38, 1),
            (42, 1),
            (42, 2),
            (50, 1),
            (68, 1),
        ],
    ),
    (
        "sajda",
        "Verses with a sajda sign",
        &[(7, 206), (32, 15), (96, 19)],
    ),
    ("tawba", "Surah 9: no Basmala", &[(9, 1), (9, 2)]),
    (
        "rare-marks",
        "Rare marks of the Hafs text: 11:41, 12:11, 21:88, 41:44",
        &[(11, 41), (12, 11), (21, 88), (41, 44)],
    ),
    (
        "short-surahs",
        "Short surahs in full",
        &[
            (108, 1),
            (108, 2),
            (108, 3),
            (112, 1),
            (112, 2),
            (112, 3),
            (112, 4),
        ],
    ),
];

/// Surahs whose header and Basmala the page shows above their first verse in a section.
fn shows_header(section: &str, r: VerseRef) -> bool {
    r.ayah == 1 && !matches!(section, "basmala" | "muqattaat" | "sajda" | "rare-marks")
}

fn build() -> Value {
    let quran: &Quran<'_> = quran_core::quran().expect("data/quran.bin");
    let pickthall: &Translation<'_> = quran_core::bundled_translations()
        .expect("bundled translations")
        .iter()
        .find(|t| t.id() == "en.pickthall")
        .expect("Pickthall is bundled");

    let verse = |r: VerseRef, covered: &mut BTreeSet<char>| {
        let text = quran.text(r).unwrap_or_else(|| panic!("{r} exists"));
        covered.extend(text.chars());
        let index = quran.global_index(r).expect("verse index");
        json!({
            "ref": r,
            "text": text,
            "translation": pickthall.verse(index).expect("translated"),
        })
    };

    let mut covered = BTreeSet::new();
    let mut sections: Vec<Value> = Vec::new();
    for (id, title, refs) in SECTIONS {
        let mut rows = Vec::new();
        for &(surah, ayah) in *refs {
            let r = VerseRef::new(surah, ayah);
            let meta = quran.surah(surah).expect("surah exists");
            if shows_header(id, r) {
                rows.push(json!({ "header": meta.to_surah(), "bismillah": meta.bismillah }));
            } else if *id == "basmala" && ayah == 1 {
                rows.push(json!({ "bismillah": meta.bismillah }));
                continue;
            }
            rows.push(verse(r, &mut covered));
        }
        sections.push(json!({ "id": id, "title": title, "rows": rows }));
    }

    // Every character of the text appears somewhere on the page, so the font coverage test
    // sees all of them: for each one not covered yet, the shortest verse that has it.
    let mut rare = Vec::new();
    let all: BTreeSet<char> = quran.verses().flat_map(str::chars).collect();
    for c in all {
        if covered.contains(&c) {
            continue;
        }
        let shortest = (0..quran.verses().len() as u16)
            .filter_map(|i| quran.verse_ref_at(i))
            .filter(|&r| quran.text(r).is_some_and(|t| t.contains(c)))
            .min_by_key(|&r| quran.text(r).map_or(usize::MAX, str::len))
            .expect("the character comes from a verse");
        rare.push(verse(shortest, &mut covered));
    }
    if !rare.is_empty() {
        sections.push(json!({
            "id": "coverage",
            "title": "Every other character used in the text",
            "rows": rare,
        }));
    }

    json!({
        "comment": "Generated from data/quran.bin and data/translations by `UPDATE_FIXTURE=1 cargo test -p quran-core --test fixture`. Do not edit.",
        "sections": sections,
    })
}

#[test]
fn screenshot_fixture_matches_the_data() {
    let expected = format!(
        "{}\n",
        serde_json::to_string_pretty(&build()).expect("serializes")
    );
    if std::env::var_os("UPDATE_FIXTURE").is_some() {
        std::fs::write(FIXTURE, &expected).expect("writes the fixture");
        return;
    }
    let committed = std::fs::read_to_string(FIXTURE).unwrap_or_default();
    assert!(
        committed == expected,
        "apps/desktop/e2e/verses.json is out of date; run \
         `UPDATE_FIXTURE=1 cargo test -p quran-core --test fixture`"
    );
}
