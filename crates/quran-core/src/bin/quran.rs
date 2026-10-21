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
        Some("read") => {
            let input = args[1..].join(" ");
            match quran_core::parse_ref(&input) {
                Ok(r) => {
                    eprintln!("{r}: no text data yet — the data build arrives in Phase 1");
                    ExitCode::FAILURE
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(2)
                }
            }
        }
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
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
