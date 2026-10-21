//! `cargo xtask` — reproducible data build.
//!
//! Planned pipeline (Phase 1):
//! 1. `fetch`  — download every source in `data/sources.toml` into `.cache/`, verify SHA-256.
//! 2. `build`  — generate the binary data files in `data/` from the cached sources.
//! 3. `verify` — check 114 surahs, 6,236 verses and the checksums of the generated files.

use std::process::ExitCode;

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  fetch   download pinned sources and verify SHA-256
  build   generate data/ from cached sources
  verify  check verse counts and checksums";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some(cmd @ ("fetch" | "build" | "verify")) => {
            eprintln!("xtask {cmd}: not implemented yet (Phase 1)");
            ExitCode::FAILURE
        }
        Some("-h" | "--help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
