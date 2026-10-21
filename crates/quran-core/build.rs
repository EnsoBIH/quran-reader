//! Embeds `data/quran.bin` when it exists. Without it (e.g. before the first
//! `cargo xtask build`) the crate still compiles and `info().data_loaded` is false.
//! Also embeds every bundled translation in `data/translations/` (DECISIONS.md D-018).

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo"));

    let data = root.join("quran.bin");
    println!("cargo::rerun-if-changed={}", data.display());
    let path = if data.is_file() {
        data
    } else {
        let empty = out.join("quran.bin");
        fs::write(&empty, []).expect("write empty placeholder");
        empty
    };
    println!("cargo::rustc-env=QURAN_DATA_PATH={}", path.display());

    // Sorted, so the order (and the build) is reproducible.
    let dir = root.join("translations");
    println!("cargo::rerun-if-changed={}", dir.display());
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| Some(e.ok()?.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "bin"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut code = String::from("&[\n");
    for f in &files {
        println!("cargo::rerun-if-changed={}", f.display());
        code.push_str(&format!(
            "    include_bytes!({:?}),\n",
            f.display().to_string()
        ));
    }
    code.push(']');
    fs::write(out.join("translations.rs"), code).expect("write translations.rs");
}
