use std::env::args_os;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::io::stdout;
use std::process;

mod account;
mod ledger;
mod transaction;

use ledger::*;

/// Reads command arguments for csv file path, reads file into
/// a new `Ledger`, and writes that `Ledger` to `stdout`
fn parse_file() -> Result<(), Box<dyn Error>> {
    // Try to read file from arg name
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    // Make a CSV reader
    let mut rdr = csv::ReaderBuilder::new()
        // Support spaces between fields
        .trim(csv::Trim::All)
        .from_reader(file);
    // Compose a Ledger from csv lines
    let ldgr = Ledger::from_reader(&mut rdr);
    // Make a writer for writing Account records into
    let mut wtr = csv::WriterBuilder::new().from_writer(stdout());
    // Write accounts to
    ldgr.write_accounts(&mut wtr)?;

    Ok(())
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = parse_file() {
        println!("{}", err);
        process::exit(1);
    }
}
