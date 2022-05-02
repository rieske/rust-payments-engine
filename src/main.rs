use std::env;
use std::fs::File;
use std::io;
use std::process;

fn main() {
    // TODO: unsure about best practices for Rust error handling. Do some reading.
    let transactions_csv = env::args_os()
        .nth(1)
        .map(File::open)
        .expect("Could not get CSV path")
        .expect("Could not open CSV file");

    if let Err(e) = payments_engine::process_transactions_csv(transactions_csv, &mut io::stdout()) {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}
