mod cli;
mod config;
mod extensions;
mod history;
mod schemas;
mod tui;
mod types;
mod utils;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
