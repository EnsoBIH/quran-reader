//! Embeds `data/quran.bin` when it exists. Without it (e.g. before the first
//! `cargo xtask build`) the crate still compiles and `info().data_loaded` is false.

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/quran.bin");
    println!("cargo::rerun-if-changed={}", data.display());

    let path = if data.is_file() {
        data
    } else {
        let empty = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo"))
            .join("quran.bin");
        fs::write(&empty, []).expect("write empty placeholder");
        empty
    };
    println!("cargo::rustc-env=QURAN_DATA_PATH={}", path.display());
}
