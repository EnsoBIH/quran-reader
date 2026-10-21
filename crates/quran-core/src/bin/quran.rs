//! `quran` — command-line front end for quran-core.

use std::io::IsTerminal;
use std::process::ExitCode;

use quran_core::Target;

const USAGE: &str = "\
usage:
  quran read <ref>      print a verse, e.g. `quran read 2:255` or `quran read kursi`
  quran find <ref>      list what the ⌘K palette matches, e.g. `quran find bakara`
  quran search [--limit N] <query>
                        full-text search, e.g. `quran search رحمن juz:30`
                        words must all occur; \"a phrase\"; -word excludes;
                        surah:2, surah:2-4, surah:baqara, juz:30 limit the search
  quran info            data and version info";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("read") => read(&args[1..].join(" ")),
        Some("find") => find(&args[1..].join(" ")),
        Some("search") => search(&args[1..]),
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

fn find(input: &str) -> ExitCode {
    let quran = match quran_core::quran() {
        Ok(q) => q,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    match quran.lookup(input, 8) {
        Ok(hits) => {
            for hit in hits {
                let r = hit.verse_ref;
                let surah = quran.surah(r.surah).map_or("", |s| s.name_latin);
                let what = match hit.target {
                    Target::Verse => format!("{surah} {r}"),
                    Target::Surah => format!("{surah} ({})", r.surah),
                    Target::Named { name } => format!("{name} — {surah} {r}"),
                    Target::Part { part, number } => format!("{part} {number} — {surah} {r}"),
                };
                println!("{what}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn search(args: &[String]) -> ExitCode {
    let (limit, query) = match args {
        [flag, n, rest @ ..] if flag == "--limit" => match n.parse::<usize>() {
            Ok(n) => (n, rest.join(" ")),
            Err(_) => {
                eprintln!("error: --limit takes a number\n{USAGE}");
                return ExitCode::from(2);
            }
        },
        _ => (20, args.join(" ")),
    };
    let (quran, index) =
        match quran_core::quran().and_then(|q| Ok((q, quran_core::search_index()?))) {
            Ok(both) => both,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        };
    let results = match index.search(&query, limit) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    // Matches are bracketed, or bold on a terminal.
    let (open, close) = if std::io::stdout().is_terminal() {
        ("\x1b[1m", "\x1b[0m")
    } else {
        ("[", "]")
    };
    for hit in &results.hits {
        let (Some(text), Some(surah)) =
            (quran.text(hit.verse_ref), quran.surah(hit.verse_ref.surah))
        else {
            continue;
        };
        let mut marked = String::with_capacity(text.len() + 16);
        let mut at = 0;
        for m in hit
            .matches
            .iter()
            .filter(|m| m.text == quran_core::QURAN_TEXT)
        {
            for range in &m.ranges {
                let bytes = range.to_bytes(text);
                marked.push_str(&text[at..bytes.start]);
                marked.push_str(open);
                marked.push_str(&text[bytes.clone()]);
                marked.push_str(close);
                at = bytes.end;
            }
        }
        marked.push_str(&text[at..]);
        println!("{} {}\n{marked}\n", surah.name_latin, hit.verse_ref);
    }
    let shown = results.hits.len();
    let verses = if results.total == 1 {
        "verse"
    } else {
        "verses"
    };
    if shown < results.total {
        println!(
            "{} {verses}, first {shown} shown (--limit N for more)",
            results.total
        );
    } else {
        println!("{} {verses}", results.total);
    }
    println!("Quran text: Tanzil.net (CC BY 3.0)");
    ExitCode::SUCCESS
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
