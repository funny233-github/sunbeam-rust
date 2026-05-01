mod cli;
mod config;
mod extensions;
mod history;
mod schemas;
mod tui;
mod types;
mod utils;

/// Entry point for the sunbeam command-line launcher.
///
/// Parses CLI arguments via `clap` and delegates to the appropriate handler.
/// Non-zero exit code is returned on errors.
fn main() {
    if let Err(e) = cli::run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
