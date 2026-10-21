//! `quran` — command-line front end for quran-core.

use std::process::ExitCode;

const USAGE: &str = "\
usage:
  quran read <ref>      print a verse, e.g. `quran read 2:255`
  quran search <query>  full-text search
  quran info            data and version info";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("read") => read(&args[1..].join(" ")),
        Some("search") => {
            eprintln!("search: not implemented yet");
            ExitCode::FAILURE
        }
        Some("info") => {
            let info = quran_core::info();
            println!(
                "quran-core {}\nsurahs: {}\nverses: {}\ndata loaded: {}",
                info.version, info.surah_count, info.ayah_count, info.data_loaded
            );
            if let Ok(quran) = quran_core::quran() {
                println!("\n{}", quran.notice());
            }
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn read(input: &str) -> ExitCode {
    let quran = match quran_core::quran() {
        Ok(q) => q,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let r = match quran.parse_ref(input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let (Some(ayah), Some(surah)) = (quran.ayah(r), quran.surah(r.surah)) else {
        eprintln!("error: {r} not found");
        return ExitCode::FAILURE;
    };
    println!("{}", ayah.text);
    println!(
        "— {} {} ({}) · juz {} · page {} · Tanzil.net (CC BY 3.0)",
        surah.name_latin, ayah.verse_ref, surah.name_arabic, ayah.juz, ayah.page
    );
    ExitCode::SUCCESS
}
